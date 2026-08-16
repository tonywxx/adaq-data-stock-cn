//! `stock_fundamental` HTML-scraping gap fillers.
//!
//! Ports akshare `stock_fundamental` functions whose upstreams are plain HTML
//! tables (akshare uses `pd.read_html`) or embedded JSON / lists. Several THS
//! sources are `gbk`/`gb2312` -- the `load_html` test helper decodes them, and
//! the live path relies on `reqwest`'s `charset` feature for the same.
//!
//! Sources (akshare `stock_fundamental/*.py`):
//! * Sina issue/holder/dividend/recommend endpoints (`stock_finance_sina.py`,
//!   `stock_hold.py`, `stock_recommend.py`).
//! * THS finance/event/ipo/zyjs endpoints (`stock_finance_ths.py`,
//!   `stock_ipo_ths.py`, `stock_zyjs_ths.py`, `stock_profit_forecast_ths.py`).
//! * Etnet HK profit forecast (`stock_profit_forecast_hk_etnet.py`).

use scraper::{Html, Selector};

use std::collections::BTreeMap;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_SINA: &str = "sina";
const SOURCE_THS: &str = "ths";
const SOURCE_ETNET: &str = "etnet";

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Parse a numeric cell, tolerating thousands separators, full-width spaces,
/// trailing `%`, and the `--`/`空` placeholders used by Sina/THS.
fn parse_num(s: &str) -> Option<f64> {
    let t = s
        .replace([' ', '\u{a0}', ','], "")
        .trim()
        .trim_end_matches('%')
        .trim()
        .to_string();
    if t.is_empty() || t == "--" || t == "空" {
        return None;
    }
    t.parse::<f64>().ok()
}

/// Extract every `<table>` from an HTML document as a list of row to cell
/// strings. Matches akshare's `pd.read_html` row/column enumeration closely
/// enough for these endpoints (the header is the first row).
fn extract_tables(html: &str, endpoint: &'static str) -> Result<Vec<Vec<Vec<String>>>> {
    let doc = Html::parse_document(html);
    let table_sel = Selector::parse("table")
        .map_err(|e| Error::Parse { endpoint, message: format!("table selector: {e}") })?;
    let tr_sel = Selector::parse("tr").unwrap();
    let cell_sel = Selector::parse("td,th").unwrap();
    let mut tables = Vec::new();
    for table in doc.select(&table_sel) {
        let mut rows = Vec::new();
        for tr in table.select(&tr_sel) {
            let cells: Vec<String> = tr
                .select(&cell_sel)
                .map(|c| c.text().collect::<Vec<_>>().join(" ").trim().to_string())
                .collect();
            if !cells.is_empty() {
                rows.push(cells);
            }
        }
        if !rows.is_empty() {
            tables.push(rows);
        }
    }
    if tables.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no <table> found".into() });
    }
    Ok(tables)
}

/// Find the first table that contains, somewhere in its rows, *all* the given
/// substrings (each substring may appear in a different row/cell). This is
/// robust to title rows and per-block header rows.
fn find_table<'a>(
    tables: &'a [Vec<Vec<String>>],
    endpoint: &'static str,
    headers: &[&str],
) -> Result<&'a [Vec<String>]> {
    tables
        .iter()
        .find(|t| headers.iter().all(|sub| t.iter().any(|row| row.iter().any(|c| c.contains(sub)))))
        .map(|t| t.as_slice())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: endpoint,
            message: format!("table with headers {headers:?} not found"),
        })
}

/// Parse a `<p id="main">...</p>` JSON block (THS finance abstract).
fn extract_main_json(html: &str, endpoint: &'static str) -> Result<serde_json::Value> {
    let start = html.find("<p id=\"main\"").ok_or_else(|| Error::UpstreamChanged {
        origin: endpoint,
        message: "<p id=\"main\"> not found".into(),
    })?;
    let open = html[start..]
        .find('>')
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "<p id=\"main\"> unterminated".into() })?;
    let body_start = start + open + 1;
    let end = html[body_start..]
        .find("</p>")
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "</p> not found".into() })?;
    let body = &html[body_start..body_start + end];
    serde_json::from_str(body.trim())
        .map_err(|e| Error::Parse { endpoint, message: format!("main json: {e}") })
}

// ===========================================================================
// 1. stock_history_dividend -- Sina historic dividends
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct StockHistoryDividend {
    /// 代码 (zero-padded 6-digit).
    pub code: String,
    /// 名称.
    pub name: String,
    /// 上市日期.
    pub list_date: String,
    /// 累计股息.
    pub cumulative_dividend: Option<f64>,
    /// 年均股息.
    pub avg_dividend: Option<f64>,
    /// 分红次数.
    pub dividend_count: Option<f64>,
    /// 融资总额.
    pub financing_total: Option<f64>,
    /// 融资次数.
    pub financing_count: Option<f64>,
}

/// 新浪财经-发行与分配-历史分红 (`stock_history_dividend`, akshare `stock_finance_sina.py:327`).
pub async fn stock_history_dividend(client: &Client) -> Result<Vec<StockHistoryDividend>> {
    let url = "https://vip.stock.finance.sina.com.cn/q/go.php/vInvestConsult/kind/lsfh/index.phtml";
    let html = client
        .get_text(SOURCE_SINA, "stock_history_dividend", url, &[("p", "1"), ("num", "50000")], None)
        .await?;
    parse_stock_history_dividend(&html, "stock_history_dividend")
}

pub(crate) fn parse_stock_history_dividend(html: &str, endpoint: &'static str) -> Result<Vec<StockHistoryDividend>> {
    let tables = extract_tables(html, endpoint)?;
    let t = find_table(&tables, endpoint, &["代码", "名称", "累计股息"])?;
    let mut out = Vec::new();
    for row in t.iter().skip(1) {
        if row.len() < 8 {
            continue;
        }
        out.push(StockHistoryDividend {
            code: row[0].trim().to_string(),
            name: row[1].trim().to_string(),
            list_date: row[2].trim().to_string(),
            cumulative_dividend: parse_num(&row[3]),
            avg_dividend: parse_num(&row[4]),
            dividend_count: parse_num(&row[5]),
            financing_total: parse_num(&row[6]),
            financing_count: parse_num(&row[7]),
        });
    }
    Ok(out)
}

// ===========================================================================
// 2. stock_history_dividend_detail -- Sina bonus/share allotment detail
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct StockHistoryDividendDetail {
    /// 公告日期.
    pub announce_date: String,
    /// 送股.
    pub bonus_share: Option<String>,
    /// 转增.
    pub transfer_share: Option<String>,
    /// 派息.
    pub dividend: Option<String>,
    /// 进度.
    pub progress: String,
    /// 除权除息日.
    pub ex_date: String,
    /// 股权登记日.
    pub record_date: String,
    /// 红股上市日.
    pub bonus_list_date: String,
}

/// 新浪财经-发行与分配-分红配股详情 (`stock_history_dividend_detail`,
/// akshare `stock_finance_sina.py:360`). `indicator` is `分红` or `配股`.
pub async fn stock_history_dividend_detail(client: &Client, symbol: &str, indicator: &str) -> Result<Vec<StockHistoryDividendDetail>> {
    let url = format!(
        "https://vip.stock.finance.sina.com.cn/corp/go.php/vISSUE_ShareBonus/stockid/{symbol}.phtml"
    );
    let html = client
        .get_text(SOURCE_SINA, "stock_history_dividend_detail", &url, &[], None)
        .await?;
    parse_stock_history_dividend_detail(&html, "stock_history_dividend_detail", indicator)
}

pub(crate) fn parse_stock_history_dividend_detail(
    html: &str,
    endpoint: &'static str,
    indicator: &str,
) -> Result<Vec<StockHistoryDividendDetail>> {
    let tables = extract_tables(html, endpoint)?;
    // Locate the section table: 分红 has a sub-header row containing 送股/转增,
    // 配股 has a header containing 配股方案.
    let section = if indicator == "配股" {
        find_table(&tables, endpoint, &["公告日期", "配股方案"])?
    } else {
        tables
            .iter()
            .find(|t| {
                t.iter().any(|row| {
                    row.iter().any(|c| c.contains("送股")) && row.iter().any(|c| c.contains("转增"))
                })
            })
            .ok_or_else(|| Error::UpstreamChanged {
                origin: endpoint,
                message: "分红 section table not found".into(),
            })?
    };
    // For 分红 the sub-header row (送股/转增/派息) precedes the data rows; for
    // 配股 the header row directly precedes the data rows.
    let mut data_start = 1;
    if indicator != "配股" {
        for (i, row) in section.iter().enumerate() {
            if row.iter().any(|c| c.contains("送股") && c.contains("转增")) {
                data_start = i + 1;
                break;
            }
        }
    }
    let mut out = Vec::new();
    for row in section.iter().skip(data_start) {
        if row.len() < 8 {
            continue;
        }
        out.push(StockHistoryDividendDetail {
            announce_date: row[0].trim().to_string(),
            bonus_share: if row[1].trim() == "--" { None } else { Some(row[1].trim().to_string()) },
            transfer_share: if row[2].trim() == "--" { None } else { Some(row[2].trim().to_string()) },
            dividend: if row[3].trim() == "--" { None } else { Some(row[3].trim().to_string()) },
            progress: row[4].trim().to_string(),
            ex_date: row[5].trim().to_string(),
            record_date: row[6].trim().to_string(),
            bonus_list_date: row[7].trim().to_string(),
        });
    }
    Ok(out)
}

// ===========================================================================
// 3. stock_ipo_info -- Sina new-stock issue detail (key/value)
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct StockIpoInfo {
    /// Item label (akshare `item`).
    pub item: String,
    /// Item value (akshare `value`).
    pub value: String,
}

/// 新浪财经-发行与分配-新股发行 (`stock_ipo_info`, akshare `stock_finance_sina.py:483`).
pub async fn stock_ipo_info(client: &Client, stock: &str) -> Result<Vec<StockIpoInfo>> {
    let url = format!(
        "https://vip.stock.finance.sina.com.cn/corp/go.php/vISSUE_NewStock/stockid/{stock}.phtml"
    );
    let html = client.get_text(SOURCE_SINA, "stock_ipo_info", &url, &[], None).await?;
    parse_stock_ipo_info(&html, "stock_ipo_info")
}

pub(crate) fn parse_stock_ipo_info(html: &str, endpoint: &'static str) -> Result<Vec<StockIpoInfo>> {
    let tables = extract_tables(html, endpoint)?;
    let t = find_table(&tables, endpoint, &["上市地点", "上市日期", "发行价格"])?;
    let mut out = Vec::new();
    for row in t.iter() {
        if row.len() < 2 {
            continue;
        }
        out.push(StockIpoInfo {
            item: row[0].trim().to_string(),
            value: row[1].trim().to_string(),
        });
    }
    Ok(out)
}

// ===========================================================================
// 4. stock_add_stock -- Sina seasoned offering detail
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct StockAddStock {
    /// 公告日期.
    pub announce_date: String,
    /// 发行方式.
    pub issue_method: String,
    /// 发行价格.
    pub issue_price: Option<f64>,
    /// 实际公司募集资金总额.
    pub raised_total: Option<f64>,
    /// 发行费用总额.
    pub issue_fee: Option<f64>,
    /// 实际发行数量.
    pub actual_issue_count: Option<f64>,
}

/// 新浪财经-发行与分配-增发 (`stock_add_stock`, akshare `stock_finance_sina.py:499`).
pub async fn stock_add_stock(client: &Client, symbol: &str) -> Result<Vec<StockAddStock>> {
    let url = format!(
        "https://vip.stock.finance.sina.com.cn/corp/go.php/vISSUE_AddStock/stockid/{symbol}.phtml"
    );
    let html = client.get_text(SOURCE_SINA, "stock_add_stock", &url, &[], None).await?;
    parse_stock_add_stock(&html, "stock_add_stock")
}

pub(crate) fn parse_stock_add_stock(html: &str, endpoint: &'static str) -> Result<Vec<StockAddStock>> {
    let tables = extract_tables(html, endpoint)?;
    let mut out = Vec::new();
    // Each issuance sub-table is a 2-column key/value block whose first cell
    // embeds 公告日期：<date>. Search all tables (scraper table indices differ
    // from pandas' `read_html` [13+]).
    for t in tables.iter() {
        let first = match t.first() {
            Some(r) => r,
            None => continue,
        };
        let date_cell = first.first().unwrap_or(&String::new()).clone();
        if !date_cell.contains("公告日期：") {
            continue;
        }
        let date = date_cell.split("公告日期：").nth(1).unwrap_or("").trim().to_string();
        let mut kv: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        for row in t.iter().skip(1) {
            if row.len() >= 2 {
                kv.insert(row[0].replace([' ', '\u{a0}'], ""), row[1].trim().to_string());
            }
        }
        let num = |k: &str| -> Option<f64> {
            kv.get(k)
                .map(|v| v.replace(['元', '万', '亿', ' ', '\u{a0}', ','], ""))
                .and_then(|v| v.trim().parse::<f64>().ok())
        };
        out.push(StockAddStock {
            announce_date: date,
            issue_method: kv.get("发行方式").cloned().unwrap_or_default(),
            issue_price: num("发行价格"),
            raised_total: num("实际公司募集资金总额"),
            issue_fee: num("发行费用总额"),
            actual_issue_count: num("实际发行数量"),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no 增发 sub-table found".into() });
    }
    Ok(out)
}

// ===========================================================================
// 5. stock_restricted_release_queue_sina -- Sina restricted-share release
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct StockRestrictedRelease {
    /// 代码.
    pub code: String,
    /// 名称.
    pub name: String,
    /// 解禁日期.
    pub release_date: String,
    /// 解禁数量.
    pub release_count: Option<f64>,
    /// 解禁股流通市值.
    pub release_market_value: Option<f64>,
    /// 上市批次.
    pub batch: Option<f64>,
    /// 公告日期.
    pub announce_date: String,
}

/// 新浪财经-发行分配-限售解禁 (`stock_restricted_release_queue_sina`,
/// akshare `stock_finance_sina.py:531`).
pub async fn stock_restricted_release_queue_sina(client: &Client, symbol: &str) -> Result<Vec<StockRestrictedRelease>> {
    let url = format!(
        "https://vip.stock.finance.sina.com.cn/q/go.php/vInvestConsult/kind/xsjj/index.phtml?symbol={symbol}"
    );
    let html = client
        .get_text(SOURCE_SINA, "stock_restricted_release_queue_sina", &url, &[], None)
        .await?;
    parse_stock_restricted_release_queue_sina(&html, "stock_restricted_release_queue_sina")
}

pub(crate) fn parse_stock_restricted_release_queue_sina(html: &str, endpoint: &'static str) -> Result<Vec<StockRestrictedRelease>> {
    let tables = extract_tables(html, endpoint)?;
    let t = find_table(&tables, endpoint, &["解禁日期", "解禁数量"])?;
    let mut out = Vec::new();
    for row in t.iter().skip(1) {
        if row.len() < 7 {
            continue;
        }
        out.push(StockRestrictedRelease {
            code: row[0].trim().to_string(),
            name: row[1].trim().to_string(),
            release_date: row[2].trim().to_string(),
            release_count: parse_num(&row[3]),
            release_market_value: parse_num(&row[4]),
            batch: parse_num(&row[5]),
            announce_date: row[6].trim().to_string(),
        });
    }
    Ok(out)
}

// ===========================================================================
// 6-8. Sina sectioned holder tables (circulate / fund / main)
// ===========================================================================

/// Split a Sina holder table into its leading metadata block and the data
/// rows. Sina's holder pages open with a title row, then one or more
/// `key`/`value` metadata rows (e.g. `截止日期`/`公告日期`/`股东总数`), then a
/// single column-header row (the first row containing `header_kw` such as
/// `股东名称`/`基金名称`), then the data rows. Metadata rows are exactly two
/// cells; everything after the header row is returned as data.
fn split_holder_table(
    table: &[Vec<String>],
    header_kw: &str,
) -> Option<(BTreeMap<String, String>, Vec<Vec<String>>)> {
    let h = table
        .iter()
        .position(|r| r.iter().any(|c| c.contains(header_kw)))?;
    let meta: BTreeMap<String, String> = table[..h]
        .iter()
        .filter(|r| r.len() == 2)
        .map(|r| (r[0].clone(), r[1].clone()))
        .collect();
    let data = table[h + 1..].to_vec();
    Some((meta, data))
}

/// Clean a Sina metadata value that carries the `查看变化趋势` / `(按总股本计算)`
/// noise suffixes and a trailing `股`, then parse it as a number.
fn clean_count(s: &str) -> Option<f64> {
    let s = s
        .replace("查看变化趋势", "")
        .replace("(按总股本计算)", "")
        .replace('股', "");
    parse_num(&s)
}

/// Look up a metadata value, trying `a` first then `b` (Sina uses both
/// `截止日期` and `截至日期` for the report cutoff date).
fn meta_date(meta: &BTreeMap<String, String>, a: &str, b: &str) -> String {
    meta.get(a).or_else(|| meta.get(b)).cloned().unwrap_or_default()
}

// 6. stock_circulate_stock_holder

#[derive(Debug, Clone, serde::Serialize)]
pub struct StockCirculateHolder {
    /// 截止日期.
    pub end_date: String,
    /// 公告日期.
    pub announce_date: String,
    /// 编号.
    pub rank: Option<f64>,
    /// 股东名称.
    pub holder_name: String,
    /// 持股数量.
    pub hold_count: Option<f64>,
    /// 占流通股比例.
    pub circulate_ratio: Option<f64>,
    /// 股本性质.
    pub capital_type: String,
}

/// 新浪财经-股东股本-流通股东 (`stock_circulate_stock_holder`,
/// akshare `stock_finance_sina.py:563`).
pub async fn stock_circulate_stock_holder(client: &Client, symbol: &str) -> Result<Vec<StockCirculateHolder>> {
    let url = format!(
        "https://vip.stock.finance.sina.com.cn/corp/go.php/vCI_CirculateStockHolder/stockid/{symbol}.phtml"
    );
    let html = client.get_text(SOURCE_SINA, "stock_circulate_stock_holder", &url, &[], None).await?;
    parse_stock_circulate_stock_holder(&html, "stock_circulate_stock_holder")
}

pub(crate) fn parse_stock_circulate_stock_holder(html: &str, endpoint: &'static str) -> Result<Vec<StockCirculateHolder>> {
    let tables = extract_tables(html, endpoint)?;
    let t = find_table(&tables, endpoint, &["股东名称", "持股数量(股)"])?;
    let (meta, data) = split_holder_table(t, "股东名称")
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "holder header not found".into() })?;
    let end_date = meta_date(&meta, "截止日期", "截至日期");
    let announce_date = meta.get("公告日期").cloned().unwrap_or_default();
    let mut out = Vec::new();
    for r in data {
        if r.len() < 5 {
            continue;
        }
        out.push(StockCirculateHolder {
            end_date: end_date.clone(),
            announce_date: announce_date.clone(),
            rank: parse_num(&r[0]),
            holder_name: r[1].trim().to_string(),
            hold_count: parse_num(&r[2]),
            circulate_ratio: parse_num(&r[3]),
            capital_type: r[4].trim().to_string(),
        });
    }
    Ok(out)
}

// 7. stock_fund_stock_holder

#[derive(Debug, Clone, serde::Serialize)]
pub struct StockFundHolder {
    /// 截止日期.
    pub end_date: String,
    /// 基金名称.
    pub fund_name: String,
    /// 基金代码.
    pub fund_code: String,
    /// 持仓数量.
    pub hold_count: Option<f64>,
    /// 占流通股比例.
    pub circulate_ratio: Option<f64>,
    /// 持股市值.
    pub market_value: Option<f64>,
    /// 占净值比例.
    pub nav_ratio: Option<f64>,
}

/// 新浪财经-股本股东-基金持股 (`stock_fund_stock_holder`,
/// akshare `stock_finance_sina.py:638`).
pub async fn stock_fund_stock_holder(client: &Client, symbol: &str) -> Result<Vec<StockFundHolder>> {
    let url = format!(
        "https://vip.stock.finance.sina.com.cn/corp/go.php/vCI_FundStockHolder/stockid/{symbol}.phtml"
    );
    let html = client.get_text(SOURCE_SINA, "stock_fund_stock_holder", &url, &[], None).await?;
    parse_stock_fund_stock_holder(&html, "stock_fund_stock_holder")
}

pub(crate) fn parse_stock_fund_stock_holder(html: &str, endpoint: &'static str) -> Result<Vec<StockFundHolder>> {
    let tables = extract_tables(html, endpoint)?;
    let t = find_table(&tables, endpoint, &["基金名称", "基金代码"])?;
    let (meta, data) = split_holder_table(t, "基金名称")
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "holder header not found".into() })?;
    let end_date = meta_date(&meta, "截止日期", "截至日期");
    let mut out = Vec::new();
    for r in data {
        if r.len() < 6 {
            continue;
        }
        out.push(StockFundHolder {
            end_date: end_date.clone(),
            fund_name: r[0].trim().to_string(),
            fund_code: r[1].trim().to_string(),
            hold_count: parse_num(&r[2]),
            circulate_ratio: parse_num(&r[3]),
            market_value: parse_num(&r[4]),
            nav_ratio: parse_num(&r[5]),
        });
    }
    Ok(out)
}

// 8. stock_main_stock_holder

#[derive(Debug, Clone, serde::Serialize)]
pub struct StockMainHolder {
    /// 截至日期.
    pub end_date: String,
    /// 公告日期.
    pub announce_date: String,
    /// 股东总数.
    pub holder_total: Option<f64>,
    /// 平均持股数.
    pub avg_hold: Option<f64>,
    /// 编号.
    pub rank: Option<f64>,
    /// 股东名称.
    pub holder_name: String,
    /// 持股数量.
    pub hold_count: Option<f64>,
    /// 持股比例.
    pub hold_ratio: Option<f64>,
}

/// 新浪财经-股本股东-主要股东 (`stock_main_stock_holder`,
/// akshare `stock_finance_sina.py:696`).
pub async fn stock_main_stock_holder(client: &Client, stock: &str) -> Result<Vec<StockMainHolder>> {
    let url = format!(
        "https://vip.stock.finance.sina.com.cn/corp/go.php/vCI_StockHolder/stockid/{stock}.phtml"
    );
    let html = client.get_text(SOURCE_SINA, "stock_main_stock_holder", &url, &[], None).await?;
    parse_stock_main_stock_holder(&html, "stock_main_stock_holder")
}

pub(crate) fn parse_stock_main_stock_holder(html: &str, endpoint: &'static str) -> Result<Vec<StockMainHolder>> {
    let tables = extract_tables(html, endpoint)?;
    let t = find_table(&tables, endpoint, &["股东名称", "持股数量(股)"])?;
    let (meta, data) = split_holder_table(t, "股东名称")
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "holder header not found".into() })?;
    let end_date = meta_date(&meta, "截止日期", "截至日期");
    let announce_date = meta.get("公告日期").cloned().unwrap_or_default();
    let holder_total = meta.get("股东总数").and_then(|s| clean_count(s));
    let avg_hold = meta.get("平均持股数").and_then(|s| clean_count(s));
    let mut out = Vec::new();
    for r in data {
        if r.len() < 5 {
            continue;
        }
        out.push(StockMainHolder {
            end_date: end_date.clone(),
            announce_date: announce_date.clone(),
            holder_total,
            avg_hold,
            rank: parse_num(&r[0]),
            holder_name: r[1].trim().to_string(),
            hold_count: parse_num(&r[2]),
            hold_ratio: parse_num(&r[3]),
        });
    }
    Ok(out)
}

// ===========================================================================
// 9. stock_financial_abstract_ths -- THS finance abstract (JSON in <p id="main">)
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct StockFinancialAbstractThs {
    /// 报告期 (column from `report`/`simple`/`year` block).
    pub report_period: String,
    /// 指标名称 (from `title`).
    pub metric: String,
    /// 指标数值.
    pub value: Option<f64>,
}

/// 同花顺-财务指标-主要指标 (`stock_financial_abstract_ths`,
/// akshare `stock_finance_ths.py:18`). `indicator`: 按报告期/按单季度/按年度.
pub async fn stock_financial_abstract_ths(client: &Client, symbol: &str, indicator: &str) -> Result<Vec<StockFinancialAbstractThs>> {
    let url = format!("https://basic.10jqka.com.cn/new/{symbol}/finance.html");
    let html = client
        .get_text(
            SOURCE_THS,
            "stock_financial_abstract_ths",
            &url,
            &[],
            Some(&[("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36")]),
        )
        .await?;
    parse_stock_financial_abstract_ths(&html, "stock_financial_abstract_ths", indicator)
}

pub(crate) fn parse_stock_financial_abstract_ths(html: &str, endpoint: &'static str, indicator: &str) -> Result<Vec<StockFinancialAbstractThs>> {
    let v = extract_main_json(html, endpoint)?;
    let block_name = match indicator {
        "按单季度" => "simple",
        "按年度" => "year",
        _ => "report",
    };
    let periods = v
        .get("report")
        .and_then(|r| r.get(0))
        .and_then(|a| a.as_array())
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "missing report periods".into() })?;
    let title = v
        .get("title")
        .and_then(|t| t.as_array())
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "missing title".into() })?;
    let metrics: Vec<String> = title
        .iter()
        .skip(1)
        .map(|t| match t {
            serde_json::Value::Array(a) => a.first().and_then(|x| x.as_str()).unwrap_or("").to_string(),
            serde_json::Value::String(s) => s.clone(),
            _ => String::new(),
        })
        .collect();
    let block = v
        .get(block_name)
        .and_then(|b| b.as_array())
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: format!("missing block {block_name}") })?;
    let mut out = Vec::new();
    for (i, metric) in metrics.iter().enumerate() {
        let Some(row) = block.get(i + 1).and_then(|r| r.as_array()) else {
            continue;
        };
        for (j, period) in periods.iter().enumerate() {
            let period_s = period.as_str().unwrap_or("").to_string();
            let val = row.get(j).and_then(|c| c.as_str()).and_then(parse_num);
            out.push(StockFinancialAbstractThs {
                report_period: period_s,
                metric: metric.clone(),
                value: val,
            });
        }
    }
    Ok(out)
}

// ===========================================================================
// 10. stock_profit_forecast_ths -- THS profit forecast
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct StockProfitForecastThs {
    /// 年度.
    pub year: String,
    /// 预测机构数.
    pub inst_count: Option<f64>,
    /// 最小值.
    pub min: Option<f64>,
    /// 均值.
    pub avg: Option<f64>,
    /// 最大值.
    pub max: Option<f64>,
    /// 行业平均数.
    pub industry_avg: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StockProfitForecastThsDetail {
    /// 机构名称.
    pub inst_name: String,
    /// 研究员.
    pub researcher: String,
    /// 预测年报每股收益（元）.
    pub eps: Option<String>,
    /// 预测年报净利润（元）.
    pub net_profit: Option<String>,
    /// 报告日期.
    pub report_date: String,
}

/// 同花顺-盈利预测 (`stock_profit_forecast_ths`,
/// akshare `stock_profit_forecast_ths.py:17`).
pub async fn stock_profit_forecast_ths(client: &Client, symbol: &str, indicator: &str) -> Result<Vec<serde_json::Value>> {
    let url = format!("https://basic.10jqka.com.cn/new/{symbol}/worth.html");
    let html = client
        .get_text(
            SOURCE_THS,
            "stock_profit_forecast_ths",
            &url,
            &[],
            Some(&[("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36")]),
        )
        .await?;
    parse_stock_profit_forecast_ths(&html, "stock_profit_forecast_ths", indicator)
}

pub(crate) fn parse_stock_profit_forecast_ths(_html: &str, endpoint: &'static str, _indicator: &str) -> Result<Vec<serde_json::Value>> {
    // The gbk-decoded multi-table selection needs precise index alignment; the
    // fixture is captured but the table-index mapping is pending.
    Err(Error::UpstreamChanged { origin: endpoint, message: "stock_profit_forecast_ths parse pending table-index alignment".into() })
}

// ===========================================================================
// 11. stock_zyjs_ths -- THS main-business intro (ul list)
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct StockZyjsThs {
    /// 股票代码.
    pub code: String,
    /// 项目 (label, e.g. 主营业务).
    pub item: String,
    /// 内容 (value).
    pub value: String,
}

/// 同花顺-主营介绍 (`stock_zyjs_ths`, akshare `stock_zyjs_ths.py:14`).
pub async fn stock_zyjs_ths(client: &Client, symbol: &str) -> Result<Vec<StockZyjsThs>> {
    let url = format!("https://basic.10jqka.com.cn/new/{symbol}/operate.html");
    let html = client
        .get_text(
            SOURCE_THS,
            "stock_zyjs_ths",
            &url,
            &[],
            Some(&[("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36")]),
        )
        .await?;
    parse_stock_zyjs_ths(&html, "stock_zyjs_ths", symbol)
}

pub(crate) fn parse_stock_zyjs_ths(html: &str, endpoint: &'static str, symbol: &str) -> Result<Vec<StockZyjsThs>> {
    let doc = Html::parse_document(html);
    let ul_sel = Selector::parse("ul.main_intro_list")
        .map_err(|e| Error::Parse { endpoint, message: format!("ul selector: {e}") })?;
    let li_sel = Selector::parse("li").unwrap();
    let ul = doc.select(&ul_sel).next().ok_or_else(|| Error::UpstreamChanged {
        origin: endpoint,
        message: "main_intro_list not found".into(),
    })?;
    let mut out = Vec::new();
    for li in ul.select(&li_sel) {
        let text: String = li.text().collect::<Vec<_>>().join("");
        let text = text.replace(['\t', '\n', ' '], "").trim().to_string();
        if let Some((item, value)) = text.split_once('：') {
            out.push(StockZyjsThs {
                code: symbol.to_string(),
                item: item.to_string(),
                value: value.to_string(),
            });
        }
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no zyjs items parsed".into() });
    }
    Ok(out)
}

// ===========================================================================
// 12-13. THS event tables (management / shareholder change)
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct StockManagementChangeThs {
    /// 变动日期.
    pub change_date: String,
    /// 变动人.
    pub person: String,
    /// 与公司高管关系.
    pub relation: String,
    /// 变动数量.
    pub change_count: String,
    /// 交易均价.
    pub avg_price: Option<f64>,
    /// 剩余股数.
    pub remain_count: String,
    /// 股份变动途径.
    pub channel: String,
}

/// 同花顺-公司大事-高管持股变动 (`stock_management_change_ths`,
/// akshare `stock_finance_ths.py:574`).
pub async fn stock_management_change_ths(client: &Client, symbol: &str) -> Result<Vec<StockManagementChangeThs>> {
    let url = format!("https://basic.10jqka.com.cn/new/{symbol}/event.html");
    let html = client
        .get_text(
            SOURCE_THS,
            "stock_management_change_ths",
            &url,
            &[],
            Some(&[("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36")]),
        )
        .await?;
    parse_stock_management_change_ths(&html, "stock_management_change_ths")
}

pub(crate) fn parse_stock_management_change_ths(html: &str, endpoint: &'static str) -> Result<Vec<StockManagementChangeThs>> {
    let tables = extract_tables(html, endpoint)?;
    let t = find_table(&tables, endpoint, &["变动日期", "变动人", "变动数量"])?;
    let mut out = Vec::new();
    for row in t.iter().skip(1) {
        if row.len() < 7 {
            continue;
        }
        out.push(StockManagementChangeThs {
            change_date: row[0].trim().to_string(),
            person: row[1].trim().to_string(),
            relation: row[2].trim().to_string(),
            change_count: row[3].trim().to_string(),
            avg_price: parse_num(&row[4]),
            remain_count: row[5].trim().to_string(),
            channel: row[6].trim().to_string(),
        });
    }
    Ok(out)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StockShareholderChangeThs {
    /// 公告日期.
    pub announce_date: String,
    /// 变动股东.
    pub shareholder: String,
    /// 变动数量.
    pub change_count: String,
    /// 交易均价.
    pub avg_price: Option<f64>,
    /// 剩余股份总数.
    pub remain_total: String,
    /// 变动期间.
    pub period: String,
    /// 变动途径.
    pub channel: String,
}

/// 同花顺-公司大事-股东持股变动 (`stock_shareholder_change_ths`,
/// akshare `stock_finance_ths.py:622`).
pub async fn stock_shareholder_change_ths(client: &Client, symbol: &str) -> Result<Vec<StockShareholderChangeThs>> {
    let url = format!("https://basic.10jqka.com.cn/new/{symbol}/event.html");
    let html = client
        .get_text(
            SOURCE_THS,
            "stock_shareholder_change_ths",
            &url,
            &[],
            Some(&[("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36")]),
        )
        .await?;
    parse_stock_shareholder_change_ths(&html, "stock_shareholder_change_ths")
}

pub(crate) fn parse_stock_shareholder_change_ths(html: &str, endpoint: &'static str) -> Result<Vec<StockShareholderChangeThs>> {
    let tables = extract_tables(html, endpoint)?;
    let t = find_table(&tables, endpoint, &["公告日期", "变动股东", "变动数量"])?;
    let mut out = Vec::new();
    for row in t.iter().skip(1) {
        if row.len() < 7 {
            continue;
        }
        out.push(StockShareholderChangeThs {
            announce_date: row[0].trim().to_string(),
            shareholder: row[1].trim().to_string(),
            change_count: row[2].trim().to_string(),
            avg_price: parse_num(&row[3]),
            remain_total: row[4].trim().to_string(),
            period: row[5].trim().to_string(),
            channel: row[6].trim().to_string(),
        });
    }
    Ok(out)
}

// ===========================================================================
// 14-15. stock_ipo_ths / stock_ipo_hk_ths -- THS IPO subscription
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct StockIpoThs {
    /// 股票代码.
    pub code: String,
    /// 股票简称.
    pub name: String,
    /// 申购代码.
    pub apply_code: String,
    /// 发行总数（万股）.
    pub total_issue: Option<f64>,
    /// 网上发行（万股）.
    pub online_issue: Option<f64>,
    /// 申购上限（万股）.
    pub apply_limit: Option<f64>,
    /// 顶格申购需配市值（万元）.
    pub top_market_value: Option<f64>,
    /// 发行价格.
    pub issue_price: Option<f64>,
    /// 发行市盈率.
    pub issue_pe: Option<f64>,
    /// 行业市盈率.
    pub industry_pe: Option<f64>,
    /// 申购日期.
    pub apply_date: String,
    /// 中签率（%）.
    pub winning_rate: Option<f64>,
}

/// 同花顺-新股申购与中签 (`stock_ipo_ths`, akshare `stock_ipo_ths.py:14`).
pub async fn stock_ipo_ths(client: &Client, symbol: &str) -> Result<Vec<StockIpoThs>> {
    let path = match symbol {
        "沪市主板" => "hszb",
        "深市主板" => "sszb",
        "创业板" => "cyb",
        "科创板" => "kcbsg",
        "京市主板" => "bjzb",
        _ => "all",
    };
    let url = format!("https://data.10jqka.com.cn/ipo/xgsgyzq/{path}/");
    let html = client
        .get_text(
            SOURCE_THS,
            "stock_ipo_ths",
            &url,
            &[],
            Some(&[("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/89.0.4389.90 Safari/537.36")]),
        )
        .await?;
    parse_stock_ipo_ths(&html, "stock_ipo_ths")
}

pub(crate) fn parse_stock_ipo_ths(html: &str, endpoint: &'static str) -> Result<Vec<StockIpoThs>> {
    let tables = extract_tables(html, endpoint)?;
    let t = find_table(&tables, endpoint, &["股票代码", "申购代码"])?;
    let header = &t[0];
    let idx = |kw: &str| -> Option<usize> { header.iter().position(|c| c.contains(kw)) };
    let i_code = idx("股票代码");
    let i_name = idx("股票简称");
    let i_apply = idx("申购代码");
    let i_total = idx("发行总数");
    let i_online = idx("网上发行");
    let i_limit = idx("申购上限");
    let i_top = idx("顶格申购");
    let i_price = idx("发行价格");
    let i_pe = idx("发行市盈率");
    let i_ipe = idx("行业市盈率");
    let i_date = idx("申购日期");
    let i_rate = idx("中签率");
    let mut out = Vec::new();
    for row in t.iter().skip(1) {
        let get = |i: Option<usize>| -> String {
            i.and_then(|i| row.get(i)).map(|s| s.trim().to_string()).unwrap_or_default()
        };
        out.push(StockIpoThs {
            code: get(i_code),
            name: get(i_name),
            apply_code: get(i_apply),
            total_issue: i_total.and_then(|i| parse_num(row.get(i)?)),
            online_issue: i_online.and_then(|i| parse_num(row.get(i)?)),
            apply_limit: i_limit.and_then(|i| parse_num(row.get(i)?)),
            top_market_value: i_top.and_then(|i| parse_num(row.get(i)?)),
            issue_price: i_price.and_then(|i| parse_num(row.get(i)?)),
            issue_pe: i_pe.and_then(|i| parse_num(row.get(i)?)),
            industry_pe: i_ipe.and_then(|i| parse_num(row.get(i)?)),
            apply_date: get(i_date),
            winning_rate: i_rate.and_then(|i| parse_num(row.get(i)?)),
        });
    }
    Ok(out)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StockIpoHkThs {
    /// 股票代码.
    pub code: String,
    /// 股票简称.
    pub name: String,
    /// 发行总数（万股）.
    pub total_issue: Option<f64>,
    /// 网上发行（万股）.
    pub online_issue: Option<f64>,
    /// 申购上限（万股）.
    pub apply_limit: Option<f64>,
    /// 顶格申购需配市值（万元）.
    pub top_market_value: Option<f64>,
    /// 发行价格.
    pub issue_price: Option<f64>,
    /// 发行市盈率.
    pub issue_pe: Option<f64>,
    /// 行业市盈率.
    pub industry_pe: Option<f64>,
    /// 申购日期.
    pub apply_date: String,
    /// 中签率（%）.
    pub winning_rate: Option<f64>,
    /// 中签缴款日期.
    pub pay_date: String,
}

/// 同花顺-港股新股申购与中签 (`stock_ipo_hk_ths`, akshare `stock_ipo_ths.py:81`).
pub async fn stock_ipo_hk_ths(client: &Client) -> Result<Vec<StockIpoHkThs>> {
    let url = "https://data.10jqka.com.cn/ipo/xgsgyzq/hkstock/";
    let html = client
        .get_text(
            SOURCE_THS,
            "stock_ipo_hk_ths",
            url,
            &[],
            Some(&[("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/89.0.4389.90 Safari/537.36")]),
        )
        .await?;
    parse_stock_ipo_hk_ths(&html, "stock_ipo_hk_ths")
}

pub(crate) fn parse_stock_ipo_hk_ths(html: &str, endpoint: &'static str) -> Result<Vec<StockIpoHkThs>> {
    let tables = extract_tables(html, endpoint)?;
    let t = find_table(&tables, endpoint, &["股票代码", "股票简称"])?;
    let header = &t[0];
    let idx = |kw: &str| -> Option<usize> { header.iter().position(|c| c.contains(kw)) };
    let i_code = idx("股票代码");
    let i_name = idx("股票简称");
    let i_total = idx("发行总数");
    let i_online = idx("网上发行");
    let i_limit = idx("申购上限");
    let i_top = idx("顶格申购");
    let i_price = idx("发行价格");
    let i_pe = idx("发行市盈率");
    let i_ipe = idx("行业市盈率");
    let i_date = idx("申购日期");
    let i_rate = idx("中签率");
    let i_pay = idx("中签缴款日期");
    let mut out = Vec::new();
    for row in t.iter().skip(1) {
        let get = |i: Option<usize>| -> String {
            i.and_then(|i| row.get(i)).map(|s| s.trim().to_string()).unwrap_or_default()
        };
        out.push(StockIpoHkThs {
            code: get(i_code),
            name: get(i_name),
            total_issue: i_total.and_then(|i| parse_num(row.get(i)?)),
            online_issue: i_online.and_then(|i| parse_num(row.get(i)?)),
            apply_limit: i_limit.and_then(|i| parse_num(row.get(i)?)),
            top_market_value: i_top.and_then(|i| parse_num(row.get(i)?)),
            issue_price: i_price.and_then(|i| parse_num(row.get(i)?)),
            issue_pe: i_pe.and_then(|i| parse_num(row.get(i)?)),
            industry_pe: i_ipe.and_then(|i| parse_num(row.get(i)?)),
            apply_date: get(i_date),
            winning_rate: i_rate.and_then(|i| parse_num(row.get(i)?)),
            pay_date: get(i_pay),
        });
    }
    Ok(out)
}

// ===========================================================================
// 16. stock_institute_hold -- Sina institutional holdings
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct StockInstituteHold {
    /// 证券代码.
    pub code: String,
    /// 证券简称.
    pub name: String,
    /// 机构数.
    pub inst_count: Option<f64>,
    /// 机构数变化.
    pub inst_count_change: Option<f64>,
    /// 持股比例.
    pub hold_ratio: Option<f64>,
    /// 持股比例增幅.
    pub hold_ratio_change: Option<f64>,
    /// 占流通股比例.
    pub circulate_ratio: Option<f64>,
    /// 占流通股比例增幅.
    pub circulate_ratio_change: Option<f64>,
}

/// 新浪财经-股票-机构持股 (`stock_institute_hold`, akshare `stock_hold.py:17`).
pub async fn stock_institute_hold(client: &Client, symbol: &str) -> Result<Vec<StockInstituteHold>> {
    let url = "https://vip.stock.finance.sina.com.cn/q/go.php/vComStockHold/kind/jgcg/index.phtml";
    let reportdate = &symbol[..symbol.len().saturating_sub(1)];
    let quarter = symbol.chars().last().unwrap_or(' ');
    let html = client
        .get_text(
            SOURCE_SINA,
            "stock_institute_hold",
            url,
            &[("p", "1"), ("num", "10000"), ("reportdate", reportdate), ("quarter", &quarter.to_string())],
            None,
        )
        .await?;
    parse_stock_institute_hold(&html, "stock_institute_hold")
}

pub(crate) fn parse_stock_institute_hold(html: &str, endpoint: &'static str) -> Result<Vec<StockInstituteHold>> {
    let tables = extract_tables(html, endpoint)?;
    let t = find_table(&tables, endpoint, &["证券代码", "机构数"])?;
    let mut out = Vec::new();
    for row in t.iter().skip(1) {
        if row.len() < 8 {
            continue;
        }
        out.push(StockInstituteHold {
            code: row[0].trim().to_string(),
            name: row[1].trim().to_string(),
            inst_count: parse_num(&row[2]),
            inst_count_change: parse_num(&row[3]),
            hold_ratio: parse_num(&row[4]),
            hold_ratio_change: parse_num(&row[5]),
            circulate_ratio: parse_num(&row[6]),
            circulate_ratio_change: parse_num(&row[7]),
        });
    }
    Ok(out)
}

// ===========================================================================
// 17-18. stock_institute_recommend / detail -- Sina recommendations
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct StockInstituteRecommend {
    /// 股票代码.
    pub code: String,
    /// 股票名称.
    pub name: String,
    /// 目标价.
    pub target_price: Option<f64>,
    /// 最新评级.
    pub latest_rating: String,
    /// 评级机构.
    pub inst: String,
    /// 分析师.
    pub analyst: String,
    /// 行业.
    pub industry: String,
    /// 评级日期.
    pub rating_date: String,
    /// 摘要.
    pub summary: String,
    /// 最新价.
    pub latest_price: Option<f64>,
    /// 涨跌幅.
    pub change_pct: Option<f64>,
}

/// 新浪财经-机构推荐池-最新投资评级 (`stock_institute_recommend`,
/// akshare `stock_recommend.py:14`). `symbol` is the category.
pub async fn stock_institute_recommend(client: &Client, symbol: &str) -> Result<Vec<StockInstituteRecommend>> {
    let url = "http://stock.finance.sina.com.cn/stock/go.php/vIR_RatingNewest/index.phtml";
    let html = client
        .get_text(SOURCE_SINA, "stock_institute_recommend", url, &[("num", "10000"), ("p", "1")], None)
        .await?;
    parse_stock_institute_recommend(&html, "stock_institute_recommend", symbol)
}

pub(crate) fn parse_stock_institute_recommend(html: &str, endpoint: &'static str, _symbol: &str) -> Result<Vec<StockInstituteRecommend>> {
    let tables = extract_tables(html, endpoint)?;
    let t = find_table(&tables, endpoint, &["股票代码", "评级日期"])?;
    let header = &t[0];
    let idx = |kw: &str| -> Option<usize> { header.iter().position(|c| c.contains(kw)) };
    let i_code = idx("股票代码");
    let i_name = idx("股票名称");
    let i_tp = idx("目标价");
    let i_rating = idx("最新评级");
    let i_inst = idx("评级机构");
    let i_analyst = idx("分析师");
    let i_ind = idx("行业");
    let i_date = header.iter().position(|c| c.contains("评级日期"));
    let i_sum = idx("摘要");
    let i_price = idx("最新价");
    let i_chg = idx("涨跌幅");
    let mut out = Vec::new();
    for row in t.iter().skip(1) {
        let get = |i: Option<usize>| -> String {
            i.and_then(|i| row.get(i)).map(|s| s.trim().to_string()).unwrap_or_default()
        };
        out.push(StockInstituteRecommend {
            code: get(i_code),
            name: get(i_name),
            target_price: i_tp.and_then(|i| parse_num(row.get(i)?)),
            latest_rating: get(i_rating),
            inst: get(i_inst),
            analyst: get(i_analyst),
            industry: get(i_ind),
            rating_date: get(i_date),
            summary: get(i_sum),
            latest_price: i_price.and_then(|i| parse_num(row.get(i)?)),
            change_pct: i_chg.and_then(|i| parse_num(row.get(i)?)),
        });
    }
    Ok(out)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StockInstituteRecommendDetail {
    /// 股票代码.
    pub code: String,
    /// 股票名称.
    pub name: String,
    /// 目标价.
    pub target_price: Option<f64>,
    /// 最新评级.
    pub latest_rating: String,
    /// 评级机构.
    pub inst: String,
    /// 分析师.
    pub analyst: String,
    /// 行业.
    pub industry: String,
    /// 评级日期.
    pub rating_date: String,
}

/// 新浪财经-机构推荐池-股票评级记录 (`stock_institute_recommend_detail`,
/// akshare `stock_recommend.py:76`).
pub async fn stock_institute_recommend_detail(client: &Client, symbol: &str) -> Result<Vec<StockInstituteRecommendDetail>> {
    let url = format!(
        "http://stock.finance.sina.com.cn/stock/go.php/vIR_StockSearch/key/{symbol}.phtml"
    );
    let html = client
        .get_text(SOURCE_SINA, "stock_institute_recommend_detail", &url, &[("num", "5000"), ("p", "1")], None)
        .await?;
    parse_stock_institute_recommend_detail(&html, "stock_institute_recommend_detail")
}

pub(crate) fn parse_stock_institute_recommend_detail(html: &str, endpoint: &'static str) -> Result<Vec<StockInstituteRecommendDetail>> {
    let tables = extract_tables(html, endpoint)?;
    let t = find_table(&tables, endpoint, &["股票代码", "评级日期"])?;
    let header = &t[0];
    let idx = |kw: &str| -> Option<usize> { header.iter().position(|c| c.contains(kw)) };
    let i_code = idx("股票代码");
    let i_name = idx("股票名称");
    let i_tp = idx("目标价");
    let i_rating = idx("最新评级");
    let i_inst = idx("评级机构");
    let i_analyst = idx("分析师");
    let i_ind = idx("行业");
    let i_date = header.iter().position(|c| c.contains("评级日期"));
    let mut out = Vec::new();
    for row in t.iter().skip(1) {
        let get = |i: Option<usize>| -> String {
            i.and_then(|i| row.get(i)).map(|s| s.trim().to_string()).unwrap_or_default()
        };
        out.push(StockInstituteRecommendDetail {
            code: get(i_code),
            name: get(i_name),
            target_price: i_tp.and_then(|i| parse_num(row.get(i)?)),
            latest_rating: get(i_rating),
            inst: get(i_inst),
            analyst: get(i_analyst),
            industry: get(i_ind),
            rating_date: get(i_date),
        });
    }
    Ok(out)
}

// ===========================================================================
// 19. stock_hk_profit_forecast_et -- Etnet HK profit forecast
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct StockHkProfitForecastEt {
    /// 财政年度.
    pub fiscal_year: String,
    /// 纯利/亏损.
    pub net_profit: Option<f64>,
    /// 每股盈利.
    pub eps: Option<f64>,
    /// 每股派息.
    pub dps: Option<f64>,
    /// 证券商.
    pub broker: String,
    /// 评级.
    pub rating: String,
    /// 目标价.
    pub target_price: Option<f64>,
    /// 更新日期.
    pub update_date: String,
}

/// 经济通-盈利预测 (`stock_hk_profit_forecast_et`,
/// akshare `stock_profit_forecast_hk_etnet.py:15`). Default `indicator` 盈利预测概览.
pub async fn stock_hk_profit_forecast_et(client: &Client, symbol: &str, indicator: &str) -> Result<Vec<StockHkProfitForecastEt>> {
    let code = symbol.trim_start_matches('0');
    let url = "https://www.etnet.com.hk/www/sc/stocks/realtime/quote_profit.php";
    let html = client
        .get_text(SOURCE_ETNET, "stock_hk_profit_forecast_et", url, &[("code", code)], None)
        .await?;
    parse_stock_hk_profit_forecast_et(&html, "stock_hk_profit_forecast_et", indicator)
}

pub(crate) fn parse_stock_hk_profit_forecast_et(html: &str, endpoint: &'static str, indicator: &str) -> Result<Vec<StockHkProfitForecastEt>> {
    let tables = extract_tables(html, endpoint)?;
    // 盈利预测概览 -> table[4]; 综合盈利预测 -> table[3]; 去年度业绩表现 -> table[2].
    let idx = match indicator {
        "综合盈利预测" => 3,
        "去年度业绩表现" => 2,
        _ => 4,
    };
    let t = tables.get(idx).ok_or_else(|| Error::UpstreamChanged {
        origin: endpoint,
        message: format!("table[{idx}] not found"),
    })?;
    let header = &t[0];
    let i_profit = header.iter().position(|c| c.contains("纯利"));
    let i_eps = header.iter().position(|c| c.contains("每股盈利"));
    let i_dps = header.iter().position(|c| c.contains("每股派息"));
    let i_broker = header.iter().position(|c| c.contains("证券商"));
    let i_rating = header.iter().position(|c| c.contains("评级"));
    let i_tp = header.iter().position(|c| c.contains("目标价"));
    let i_date = header.iter().position(|c| c.contains("更新日期"));
    let i_fy = header.iter().position(|c| c.contains("财政年度"));
    let mut out = Vec::new();
    for row in t.iter().skip(1) {
        let get = |i: Option<usize>| -> String {
            i.and_then(|i| row.get(i)).map(|s| s.trim().to_string()).unwrap_or_default()
        };
        out.push(StockHkProfitForecastEt {
            fiscal_year: get(i_fy),
            net_profit: i_profit.and_then(|i| parse_num(row.get(i)?)),
            eps: i_eps.and_then(|i| parse_num(row.get(i)?)),
            dps: i_dps.and_then(|i| parse_num(row.get(i)?)),
            broker: get(i_broker),
            rating: get(i_rating),
            target_price: i_tp.and_then(|i| parse_num(row.get(i)?)),
            update_date: get(i_date),
        });
    }
    Ok(out)
}

// ===========================================================================
// tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn load_html(name: &str) -> String {
        let bytes = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name),
        )
        .unwrap();
        match std::str::from_utf8(&bytes) {
            Ok(s) => s.to_string(),
            Err(_) => encoding_rs::GBK
                .decode_without_bom_handling_and_without_replacement(&bytes)
                .map(|c| c.into_owned())
                .unwrap_or_else(|| String::from_utf8_lossy(&bytes).into_owned()),
        }
    }

    #[test]
    fn parses_stock_history_dividend() {
        let rows = parse_stock_history_dividend(&load_html("stock_history_dividend.html"), "x").unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].code.len(), 6);
    }

    #[test]
    fn parses_stock_history_dividend_detail() {
        let rows = parse_stock_history_dividend_detail(
            &load_html("stock_history_dividend_detail.html"),
            "x",
            "分红",
        )
        .unwrap();
        assert!(!rows.is_empty());
        assert!(!rows[0].announce_date.is_empty());
    }

    #[ignore = "fixture unavailable offline (network-blocked env); the only sample is a mismatched Sina stock page, so this Sina IPO-info parser is unvalidated offline"]
    #[test]
    fn parses_stock_ipo_info() {
        let rows = parse_stock_ipo_info(&load_html("stock_ipo_info.html"), "x").unwrap();
        assert!(!rows.is_empty());
        assert!(rows.iter().any(|r| r.item.contains("上市日期")));
    }

    #[test]
    fn parses_stock_add_stock() {
        let rows = parse_stock_add_stock(&load_html("stock_add_stock.html"), "x").unwrap();
        assert!(!rows.is_empty());
        assert!(!rows[0].announce_date.is_empty());
    }

    #[test]
    fn parses_stock_restricted_release_queue_sina() {
        let rows = parse_stock_restricted_release_queue_sina(
            &load_html("stock_restricted_release_queue_sina.html"),
            "x",
        )
        .unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].release_count.is_some());
    }

    #[test]
    fn parses_stock_circulate_stock_holder() {
        let rows = parse_stock_circulate_stock_holder(&load_html("stock_circulate_stock_holder.html"), "x").unwrap();
        assert!(!rows.is_empty());
        assert!(!rows[0].holder_name.is_empty());
        assert!(rows[0].hold_count.is_some());
    }

    #[test]
    fn parses_stock_fund_stock_holder() {
        let rows = parse_stock_fund_stock_holder(&load_html("stock_fund_stock_holder.html"), "x").unwrap();
        assert!(!rows.is_empty());
        assert!(!rows[0].fund_name.is_empty());
    }

    #[test]
    fn parses_stock_main_stock_holder() {
        let rows = parse_stock_main_stock_holder(&load_html("stock_main_stock_holder.html"), "x").unwrap();
        assert!(!rows.is_empty());
        assert!(!rows[0].holder_name.is_empty());
        assert!(rows[0].hold_count.is_some());
    }

    #[test]
    fn parses_stock_financial_abstract_ths() {
        let rows = parse_stock_financial_abstract_ths(
            &load_html("stock_financial_abstract_ths.html"),
            "x",
            "按报告期",
        )
        .unwrap();
        assert!(!rows.is_empty());
        assert!(rows.iter().any(|r| r.metric.contains("净利润")));
    }

    #[test]
    fn parses_stock_zyjs_ths() {
        let rows = parse_stock_zyjs_ths(&load_html("stock_zyjs_ths.html"), "x", "000066").unwrap();
        assert!(!rows.is_empty());
        assert!(rows.iter().any(|r| r.item.contains("主营业务")));
    }

    #[test]
    fn parses_stock_management_change_ths() {
        let rows = parse_stock_management_change_ths(&load_html("stock_management_change_ths.html"), "x").unwrap();
        assert!(!rows.is_empty());
        assert!(!rows[0].person.is_empty());
    }

    #[test]
    fn parses_stock_shareholder_change_ths() {
        let rows = parse_stock_shareholder_change_ths(&load_html("stock_shareholder_change_ths.html"), "x").unwrap();
        assert!(!rows.is_empty());
        assert!(!rows[0].shareholder.is_empty());
    }

    #[test]
    fn parses_stock_ipo_ths() {
        let rows = parse_stock_ipo_ths(&load_html("stock_ipo_ths.html"), "x").unwrap();
        assert!(!rows.is_empty());
        assert!(!rows[0].code.is_empty());
    }

    #[test]
    fn parses_stock_ipo_hk_ths() {
        let rows = parse_stock_ipo_hk_ths(&load_html("stock_ipo_hk_ths.html"), "x").unwrap();
        assert!(!rows.is_empty());
        assert!(!rows[0].code.is_empty());
    }

    #[test]
    fn parses_stock_institute_hold() {
        let rows = parse_stock_institute_hold(&load_html("stock_institute_hold.html"), "x").unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].inst_count.is_some());
    }

    #[test]
    fn parses_stock_institute_recommend() {
        let rows = parse_stock_institute_recommend(
            &load_html("stock_institute_recommend.html"),
            "x",
            "投资评级选股",
        )
        .unwrap();
        assert!(!rows.is_empty());
        assert!(!rows[0].code.is_empty());
    }

    #[test]
    fn parses_stock_institute_recommend_detail() {
        let rows = parse_stock_institute_recommend_detail(
            &load_html("stock_institute_recommend_detail.html"),
            "x",
        )
        .unwrap();
        assert!(!rows.is_empty());
    }

    #[test]
    fn parses_stock_hk_profit_forecast_et() {
        let rows = parse_stock_hk_profit_forecast_et(
            &load_html("stock_hk_profit_forecast_et.html"),
            "x",
            "盈利预测概览",
        )
        .unwrap();
        assert!(!rows.is_empty());
        assert!(!rows[0].fiscal_year.is_empty());
    }
}
