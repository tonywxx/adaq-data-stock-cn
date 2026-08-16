//! `stock_feature` HTML-scraping gap fillers.
//!
//! Ports akshare `stock_feature` functions whose upstreams return HTML tables
//! or HTML list pages (some THS sources are `gbk`-encoded; the `load_html`
//! test helper decodes them). Each function follows the established pattern:
//! a public `async fn` that performs the network fetch and a `pub(crate)`
//! `parse_*` that turns the captured body into rows.
//!
//! Sources / akshare references:
//! * [`stock_board_concept_info_ths`] — `stock_board_concept_ths.py:91`
//! * [`stock_board_industry_info_ths`] — `stock_board_industry_ths.py:88`
//! * [`stock_classify_board`] — `stock_classify_sina.py:17`
//! * [`stock_fhps_detail_ths`] — `stock_fhps_ths.py:15`
//! * [`stock_lh_yyb_most`] / [`stock_lh_yyb_capital`] / [`stock_lh_yyb_control`] — `stock_lh_yybpm.py:19/42/65`
//! * [`stock_lhb_detail_daily_sina`] — `stock_lhb_sina.py:18`
//! * [`stock_lhb_ggtj_sina`] / [`stock_lhb_yytj_sina`] / [`stock_lhb_jgzz_sina`] / [`stock_lhb_jgmx_sina`] — `stock_lhb_sina.py:91/128/166/208`
//! * [`stock_market_activity_legu`] — `stock_market_legu.py:18`
//! * [`stock_sns_sseinfo`] — `stock_sns_sseinfo.py:56`

use scraper::{Html, Selector};
use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_THS: &str = "ths";
const SOURCE_SINA: &str = "sina";
const SOURCE_LEGU: &str = "legu";
const SOURCE_SSEINFO: &str = "sseinfo";

// ---------------------------------------------------------------------------
// small text helpers
// ---------------------------------------------------------------------------

/// Parse a numeric cell, tolerating thousands separators and `--`/empty.
fn as_opt_f64(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', "");
    if t.is_empty() || t == "--" {
        return None;
    }
    t.parse::<f64>().ok()
}

/// Left-pad an all-digit code with zeros up to 6 chars (akshare `zfill(6)`).
fn zfill6(code: &str) -> String {
    let c = code.trim();
    if !c.is_empty() && c.chars().all(|ch| ch.is_ascii_digit()) && c.len() < 6 {
        format!("{c:0>6}")
    } else {
        c.to_string()
    }
}

/// Strip a single layer of inline HTML tags (e.g. `<font ...>沪港通</font>`).
fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

/// Split a label like `活跃度44.59%` into `(label, value)` at the first digit.
fn split_label_value(s: &str) -> (String, String) {
    let s = s.trim();
    match s.find(|c: char| c.is_ascii_digit()) {
        Some(p) => (s[..p].trim().to_string(), s[p..].trim().to_string()),
        None => (s.to_string(), String::new()),
    }
}

// ---------------------------------------------------------------------------
// generic HTML-table extraction
// ---------------------------------------------------------------------------

/// Return every `<table>` in the document as `table -> row -> cell` strings.
fn extract_tables(
    html: &str,
    endpoint: &'static str,
    table_sel: &str,
) -> Result<Vec<Vec<Vec<String>>>> {
    let doc = Html::parse_document(html);
    let table_sel = Selector::parse(table_sel)
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
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: "no <table> found".into(),
        });
    }
    Ok(tables)
}

/// Extract the data rows (everything after the header row identified by
/// `header_token` in its first cell) of the first `<table>` on the page.
fn data_rows(
    html: &str,
    endpoint: &'static str,
    header_token: &str,
) -> Result<Vec<Vec<String>>> {
    let tables = extract_tables(html, endpoint, "table")?;
    let rows = &tables[0];
    let hidx = rows
        .iter()
        .position(|r| r.first().map_or(false, |c| c.trim().contains(header_token)))
        .ok_or_else(|| Error::UpstreamChanged {
            origin: endpoint,
            message: format!("header '{header_token}' not found"),
        })?;
    Ok(rows[hidx + 1..].to_vec())
}

/// THS pagination: read `N` from `<span class="page_info">1/N</span>`.
fn parse_ths_total_pages(html: &str, endpoint: &'static str) -> Result<usize> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("span.page_info")
        .map_err(|e| Error::Parse { endpoint, message: format!("page_info selector: {e}") })?;
    let txt = doc
        .select(&sel)
        .next()
        .map(|e| e.text().collect::<String>())
        .unwrap_or_default();
    let total: usize = txt
        .split('/')
        .nth(1)
        .and_then(|s| s.trim().parse::<usize>().ok())
        .unwrap_or(1);
    Ok(total)
}

/// Sina pagination: return the second-to-last `class="page"` link's number.
fn parse_sina_last_page(html: &str, _endpoint: &'static str) -> usize {
    let doc = Html::parse_document(html);
    let sel = Selector::parse(".page").unwrap();
    let pages: Vec<usize> = doc
        .select(&sel)
        .filter_map(|e| e.text().collect::<String>().trim().parse::<usize>().ok())
        .collect();
    if pages.len() >= 2 {
        pages[pages.len() - 2]
    } else {
        1
    }
}

// ===========================================================================
// THS board info (concept / industry) — `.board-infos` dt/dd list
// ===========================================================================

/// One board-info key/value pair (akshare columns `项目`, `值`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ThsBoardInfoRow {
    /// Metric name (akshare `项目`).
    pub item: String,
    /// Metric value (akshare `值`).
    pub value: String,
}

/// Shared parser for THS `.board-infos` (`<dt>` names, `<dd>` values).
fn parse_ths_board_info(html: &str, endpoint: &'static str) -> Result<Vec<ThsBoardInfoRow>> {
    let doc = Html::parse_document(html);
    let dt_sel = Selector::parse(".board-infos dt")
        .map_err(|e| Error::Parse { endpoint, message: format!("dt selector: {e}") })?;
    let dd_sel = Selector::parse(".board-infos dd")
        .map_err(|e| Error::Parse { endpoint, message: format!("dd selector: {e}") })?;
    let names: Vec<String> = doc
        .select(&dt_sel)
        .map(|e| e.text().collect::<String>().trim().to_string())
        .collect();
    let values: Vec<String> = doc
        .select(&dd_sel)
        .map(|e| e.text().collect::<String>().trim().replace('\n', "/"))
        .collect();
    if names.is_empty() {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: "no .board-infos found".into(),
        });
    }
    let n = names.len().min(values.len());
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(ThsBoardInfoRow {
            item: names[i].clone(),
            value: values[i].clone(),
        });
    }
    Ok(out)
}

/// 同花顺-板块-概念板块-板块简介 (`stock_board_concept_info_ths`, akshare `stock_board_concept_ths.py:91`).
pub async fn stock_board_concept_info_ths(
    client: &Client,
    symbol: &str,
) -> Result<Vec<ThsBoardInfoRow>> {
    let endpoint = "stock_board_concept_info_ths";
    let boards = crate::stock_feature::sf_gaps::stock_board_concept_name_ths(client).await?;
    let code = boards
        .iter()
        .find(|b| b.name == symbol)
        .map(|b| b.code.clone())
        .ok_or_else(|| Error::NotFound {
            endpoint,
            message: format!("concept board '{symbol}' not found"),
        })?;
    let url = format!("https://q.10jqka.com.cn/gn/detail/code/{code}/");
    let html = client.get_text(SOURCE_THS, endpoint, &url, &[], None).await?;
    parse_ths_board_info(&html, endpoint)
}

/// 同花顺-板块-行业板块-板块简介 (`stock_board_industry_info_ths`, akshare `stock_board_industry_ths.py:88`).
pub async fn stock_board_industry_info_ths(
    client: &Client,
    symbol: &str,
) -> Result<Vec<ThsBoardInfoRow>> {
    let endpoint = "stock_board_industry_info_ths";
    let boards = crate::stock_feature::sf_gaps::stock_board_industry_name_ths(client).await?;
    let code = boards
        .iter()
        .find(|b| b.name == symbol)
        .map(|b| b.code.clone())
        .ok_or_else(|| Error::NotFound {
            endpoint,
            message: format!("industry board '{symbol}' not found"),
        })?;
    let url = format!("https://q.10jqka.com.cn/thshy/detail/code/{code}/");
    let html = client.get_text(SOURCE_THS, endpoint, &url, &[], None).await?;
    parse_ths_board_info(&html, endpoint)
}

// ===========================================================================
// Sina classification board — `getHQNodes` JSON tree
// ===========================================================================

/// One leaf node of a Sina classification tree (akshare `name`, `code`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockClassifyBoardRow {
    /// Top-level class name this leaf belongs to (akshare dict key).
    pub class_name: String,
    /// Leaf display name (akshare `name`).
    pub name: String,
    /// Leaf node code (akshare `code`, a Sina node id such as `new_blhy`).
    pub code: String,
}

/// Recursively collect leaf nodes `[name, _, code, ...]` from a Sina tree node.
fn collect_sina_leaves(v: &Value, out: &mut Vec<(String, String)>) {
    let Some(arr) = v.as_array() else {
        return;
    };
    if arr.len() >= 3 {
        if let (Some(name), Some(code)) = (
            arr.get(0).and_then(|x| x.as_str()),
            arr.get(2).and_then(|x| x.as_str()),
        ) {
            let is_container = arr.get(1).map_or(false, |x| x.is_array());
            if !is_container && !code.is_empty() {
                out.push((strip_html(name), code.to_string()));
                return;
            }
        }
    }
    for el in arr {
        if el.is_array() {
            collect_sina_leaves(el, out);
        }
    }
}

/// Parse a `getHQNodes` JSON response into all leaf nodes, tagged by class name.
pub(crate) fn parse_stock_classify_board(
    resp: &Value,
    endpoint: &'static str,
) -> Result<Vec<StockClassifyBoardRow>> {
    let classes = resp
        .as_array()
        .and_then(|a| a.get(1))
        .and_then(|v| v.get(0))
        .and_then(|v| v.get(1))
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: endpoint,
            message: "unexpected getHQNodes shape".into(),
        })?;
    let mut out = Vec::new();
    for c in classes {
        let cname = c
            .get(0)
            .and_then(|v| v.as_str())
            .map(strip_html)
            .unwrap_or_default();
        let mut leaves = Vec::new();
        collect_sina_leaves(c, &mut leaves);
        for (name, code) in leaves {
            out.push(StockClassifyBoardRow {
                class_name: cname.clone(),
                name,
                code,
            });
        }
    }
    Ok(out)
}

/// 新浪财经-股票分类 (`stock_classify_board`, akshare `stock_classify_sina.py:17`).
pub async fn stock_classify_board(client: &Client) -> Result<Vec<StockClassifyBoardRow>> {
    let endpoint = "stock_classify_board";
    let url = "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodes";
    let v = client.get_json(SOURCE_SINA, endpoint, url, &[]).await?;
    parse_stock_classify_board(&v, endpoint)
}

// ===========================================================================
// THS dividend detail (分红情况) — gbk `bonus.html` table
// ===========================================================================

/// One dividend/bonus record (akshare `stock_fhps_detail_ths`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FhpsDetailRow {
    /// 报告期
    pub report_period: String,
    /// 董事会日期
    pub board_date: String,
    /// 股东大会预案公告日期
    pub meeting_date: String,
    /// 实施公告日
    pub implement_date: String,
    /// 分红方案说明
    pub plan_desc: String,
    /// A股股权登记日
    pub a_reg_date: String,
    /// A股除权除息日
    pub a_ex_date: String,
    /// 分红总额
    pub total_amount: String,
    /// 方案进度
    pub progress: String,
    /// 股利支付率
    pub payout_ratio: String,
    /// 税前分红率
    pub pre_tax_rate: String,
}

/// Parse the THS bonus table (`table` first row is the header).
pub(crate) fn parse_fhps_detail(html: &str, endpoint: &'static str) -> Result<Vec<FhpsDetailRow>> {
    let rows = data_rows(html, endpoint, "报告期")?;
    let mut out = Vec::new();
    for r in &rows {
        if r.len() < 11 {
            continue;
        }
        out.push(FhpsDetailRow {
            report_period: r[0].clone(),
            board_date: r[1].clone(),
            meeting_date: r[2].clone(),
            implement_date: r[3].clone(),
            plan_desc: r[4].clone(),
            a_reg_date: r[5].clone(),
            a_ex_date: r[6].clone(),
            total_amount: r[7].clone(),
            progress: r[8].clone(),
            payout_ratio: r[9].clone(),
            pre_tax_rate: r[10].clone(),
        });
    }
    Ok(out)
}

/// 同花顺-分红情况 (`stock_fhps_detail_ths`, akshare `stock_fhps_ths.py:15`).
pub async fn stock_fhps_detail_ths(client: &Client, symbol: &str) -> Result<Vec<FhpsDetailRow>> {
    let endpoint = "stock_fhps_detail_ths";
    let url = format!("https://basic.10jqka.com.cn/new/{symbol}/bonus.html");
    let html = client.get_text(SOURCE_THS, endpoint, &url, &[], None).await?;
    parse_fhps_detail(&html, endpoint)
}

// ===========================================================================
// THS 营业部排名 (yyb) — gbk paginated table
// ===========================================================================

/// 上榜次数最多 (`stock_lh_yyb_most`, akshare `stock_lh_yybpm.py:19`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct YybMostRow {
    /// 序号
    pub rank: String,
    /// 营业部名称
    pub branch: String,
    /// 上榜次数
    pub count: Option<f64>,
    /// 合计动用资金
    pub total_funds: String,
    /// 年内上榜次数
    pub year_count: Option<f64>,
    /// 年内买入股票只数
    pub year_stocks: Option<f64>,
    /// 年内3日跟买成功率
    pub success_rate: String,
}

/// 资金实力最强 (`stock_lh_yyb_capital`, akshare `stock_lh_yybpm.py:42`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct YybCapitalRow {
    /// 序号
    pub rank: String,
    /// 营业部名称
    pub branch: String,
    /// 今日最高操作
    pub max_ops: Option<f64>,
    /// 今日最高金额
    pub max_amount: String,
    /// 今日最高买入金额
    pub max_buy: String,
    /// 累计参与金额
    pub cum_amount: String,
    /// 累计买入金额
    pub cum_buy: String,
}

/// 抱团操作实力 (`stock_lh_yyb_control`, akshare `stock_lh_yybpm.py:65`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct YybControlRow {
    /// 序号
    pub rank: String,
    /// 营业部名称
    pub branch: String,
    /// 携手营业部家数
    pub partners: Option<f64>,
    /// 年内最佳携手对象
    pub best_partner: String,
    /// 年内最佳携手股票数
    pub best_stocks: Option<f64>,
    /// 年内最佳携手成功率
    pub success_rate: String,
}

fn parse_yyb_most(html: &str, endpoint: &'static str) -> Result<Vec<YybMostRow>> {
    let rows = data_rows(html, endpoint, "序号")?;
    let mut out = Vec::new();
    for r in &rows {
        if r.len() < 7 {
            continue;
        }
        out.push(YybMostRow {
            rank: r[0].clone(),
            branch: r[1].clone(),
            count: as_opt_f64(&r[2]),
            total_funds: r[3].clone(),
            year_count: as_opt_f64(&r[4]),
            year_stocks: as_opt_f64(&r[5]),
            success_rate: r[6].clone(),
        });
    }
    Ok(out)
}

fn parse_yyb_capital(html: &str, endpoint: &'static str) -> Result<Vec<YybCapitalRow>> {
    let rows = data_rows(html, endpoint, "序号")?;
    let mut out = Vec::new();
    for r in &rows {
        if r.len() < 7 {
            continue;
        }
        out.push(YybCapitalRow {
            rank: r[0].clone(),
            branch: r[1].clone(),
            max_ops: as_opt_f64(&r[2]),
            max_amount: r[3].clone(),
            max_buy: r[4].clone(),
            cum_amount: r[5].clone(),
            cum_buy: r[6].clone(),
        });
    }
    Ok(out)
}

fn parse_yyb_control(html: &str, endpoint: &'static str) -> Result<Vec<YybControlRow>> {
    let rows = data_rows(html, endpoint, "序号")?;
    let mut out = Vec::new();
    for r in &rows {
        if r.len() < 6 {
            continue;
        }
        out.push(YybControlRow {
            rank: r[0].clone(),
            branch: r[1].clone(),
            partners: as_opt_f64(&r[2]),
            best_partner: r[3].clone(),
            best_stocks: as_opt_f64(&r[4]),
            success_rate: r[5].clone(),
        });
    }
    Ok(out)
}

/// Shared THS yyb pagination driver: fetch page 1, learn the page count, then
/// fetch the remaining pages and extend `parse` output.
async fn yyb_paginate<T, F>(
    client: &Client,
    endpoint: &'static str,
    base: &str,
    parse: F,
) -> Result<Vec<T>>
where
    F: Fn(&str, &'static str) -> Result<Vec<T>>,
{
    let html1 = client
        .get_text(SOURCE_THS, endpoint, &format!("{base}/1/"), &[], None)
        .await?;
    let total = parse_ths_total_pages(&html1, endpoint)?;
    let mut out = parse(&html1, endpoint)?;
    for p in 2..=total {
        let html = client
            .get_text(SOURCE_THS, endpoint, &format!("{base}/{p}/"), &[], None)
            .await?;
        out.extend(parse(&html, endpoint)?);
    }
    Ok(out)
}

/// 同花顺-数据中心-营业部排名-上榜次数最多.
pub async fn stock_lh_yyb_most(client: &Client) -> Result<Vec<YybMostRow>> {
    yyb_paginate(
        client,
        "stock_lh_yyb_most",
        "https://data.10jqka.com.cn/ifmarket/lhbyyb/type/1/tab/sbcs/field/sbcs/sort/desc/page",
        parse_yyb_most,
    )
    .await
}

/// 同花顺-数据中心-营业部排名-资金实力最强.
pub async fn stock_lh_yyb_capital(client: &Client) -> Result<Vec<YybCapitalRow>> {
    yyb_paginate(
        client,
        "stock_lh_yyb_capital",
        "https://data.10jqka.com.cn/ifmarket/lhbyyb/type/1/tab/zjsl/field/zgczje/sort/desc/page",
        parse_yyb_capital,
    )
    .await
}

/// 同花顺-数据中心-营业部排名-抱团操作实力.
pub async fn stock_lh_yyb_control(client: &Client) -> Result<Vec<YybControlRow>> {
    yyb_paginate(
        client,
        "stock_lh_yyb_control",
        "https://data.10jqka.com.cn/ifmarket/lhbyyb/type/1/tab/btcz/field/xsjs/sort/desc/page",
        parse_yyb_control,
    )
    .await
}

// ===========================================================================
// Sina 龙虎榜 (lhb) — gbk paginated tables
// ===========================================================================

/// 龙虎榜-每日详情 (`stock_lhb_detail_daily_sina`, akshare `stock_lhb_sina.py:18`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct LhbDailyRow {
    /// 序号
    pub rank: Option<f64>,
    /// 股票代码
    pub code: String,
    /// 股票名称
    pub name: String,
    /// 收盘价
    pub close: Option<f64>,
    /// 对应值
    pub value: Option<f64>,
    /// 成交量
    pub volume: Option<f64>,
    /// 成交额
    pub amount: Option<f64>,
    /// 指标 (the board reason, e.g. 涨幅偏离值)
    pub indicator: String,
}

/// 龙虎榜-个股上榜统计 (`stock_lhb_ggtj_sina`, akshare `stock_lhb_sina.py:91`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct LhbGgtjRow {
    /// 股票代码
    pub code: String,
    /// 股票名称
    pub name: String,
    /// 上榜次数
    pub count: Option<f64>,
    /// 累积购买额
    pub buy: Option<f64>,
    /// 累积卖出额
    pub sell: Option<f64>,
    /// 净额
    pub net: Option<f64>,
    /// 买入席位数
    pub buy_seats: Option<f64>,
    /// 卖出席位数
    pub sell_seats: Option<f64>,
}

/// 龙虎榜-营业部上榜统计 (`stock_lhb_yytj_sina`, akshare `stock_lhb_sina.py:128`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct LhbYytjRow {
    /// 营业部名称
    pub branch: String,
    /// 上榜次数
    pub count: Option<f64>,
    /// 累积购买额
    pub buy: Option<f64>,
    /// 买入席位数
    pub buy_seats: Option<f64>,
    /// 累积卖出额
    pub sell: Option<f64>,
    /// 卖出席位数
    pub sell_seats: Option<f64>,
    /// 买入前三股票
    pub top_stocks: String,
}

/// 龙虎榜-机构席位追踪 (`stock_lhb_jgzz_sina`, akshare `stock_lhb_sina.py:166`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct LhbJgzzRow {
    /// 股票代码
    pub code: String,
    /// 股票名称
    pub name: String,
    /// 累积买入额
    pub buy: Option<f64>,
    /// 买入次数
    pub buy_count: Option<f64>,
    /// 累积卖出额
    pub sell: Option<f64>,
    /// 卖出次数
    pub sell_count: Option<f64>,
    /// 净额
    pub net: Option<f64>,
}

/// 龙虎榜-机构席位成交明细 (`stock_lhb_jgmx_sina`, akshare `stock_lhb_sina.py:208`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct LhbJgmxRow {
    /// 股票代码
    pub code: String,
    /// 股票名称
    pub name: String,
    /// 交易日期
    pub date: String,
    /// 机构席位买入额
    pub inst_buy: Option<f64>,
    /// 机构席位卖出额
    pub inst_sell: Option<f64>,
    /// 类型
    pub kind: String,
}

/// Parse the Sina lhb detail page: several `table.list_table`, each with an
/// indicator row, a header row, then data rows.
fn parse_lhb_detail_daily(html: &str, endpoint: &'static str) -> Result<Vec<LhbDailyRow>> {
    let doc = Html::parse_document(html);
    let tbl_sel = Selector::parse("table.list_table")
        .map_err(|e| Error::Parse { endpoint, message: format!("list_table selector: {e}") })?;
    let tr_sel = Selector::parse("tr").unwrap();
    let cell_sel = Selector::parse("td,th").unwrap();
    let mut out = Vec::new();
    for table in doc.select(&tbl_sel) {
        let rows: Vec<Vec<String>> = table
            .select(&tr_sel)
            .map(|tr| {
                tr.select(&cell_sel)
                    .map(|c| c.text().collect::<Vec<_>>().join(" ").trim().to_string())
                    .collect()
            })
            .collect();
        if rows.len() < 2 {
            continue;
        }
        let indicator = rows[0].first().cloned().unwrap_or_default();
        // rows[1] is the header; data starts at rows[2].
        for r in &rows[2..] {
            if r.len() < 7 {
                continue;
            }
            out.push(LhbDailyRow {
                rank: as_opt_f64(&r[0]),
                code: zfill6(&r[1]),
                name: r[2].clone(),
                close: as_opt_f64(&r[3]),
                value: as_opt_f64(&r[4]),
                volume: as_opt_f64(&r[5]),
                amount: as_opt_f64(&r[6]),
                indicator: indicator.clone(),
            });
        }
    }
    Ok(out)
}

fn parse_lhb_ggtj(html: &str, endpoint: &'static str) -> Result<Vec<LhbGgtjRow>> {
    let rows = data_rows(html, endpoint, "股票代码")?;
    let mut out = Vec::new();
    for r in &rows {
        if r.len() < 8 {
            continue;
        }
        out.push(LhbGgtjRow {
            code: zfill6(&r[0]),
            name: r[1].clone(),
            count: as_opt_f64(&r[2]),
            buy: as_opt_f64(&r[3]),
            sell: as_opt_f64(&r[4]),
            net: as_opt_f64(&r[5]),
            buy_seats: as_opt_f64(&r[6]),
            sell_seats: as_opt_f64(&r[7]),
        });
    }
    Ok(out)
}

fn parse_lhb_yytj(html: &str, endpoint: &'static str) -> Result<Vec<LhbYytjRow>> {
    let rows = data_rows(html, endpoint, "营业部名称")?;
    let mut out = Vec::new();
    for r in &rows {
        if r.len() < 7 {
            continue;
        }
        out.push(LhbYytjRow {
            branch: r[0].clone(),
            count: as_opt_f64(&r[1]),
            buy: as_opt_f64(&r[2]),
            buy_seats: as_opt_f64(&r[3]),
            sell: as_opt_f64(&r[4]),
            sell_seats: as_opt_f64(&r[5]),
            top_stocks: r[6].clone(),
        });
    }
    Ok(out)
}

fn parse_lhb_jgzz(html: &str, endpoint: &'static str) -> Result<Vec<LhbJgzzRow>> {
    // Raw columns: 股票代码, 股票名称, 当前价, 涨跌幅, 累积买入额, 买入次数,
    // 累积卖出额, 卖出次数, 净额 — drop the 当前价/涨跌幅 pair (indices 2,3).
    let rows = data_rows(html, endpoint, "股票代码")?;
    let mut out = Vec::new();
    for r in &rows {
        if r.len() < 9 {
            continue;
        }
        out.push(LhbJgzzRow {
            code: zfill6(&r[0]),
            name: r[1].clone(),
            buy: as_opt_f64(&r[4]),
            buy_count: as_opt_f64(&r[5]),
            sell: as_opt_f64(&r[6]),
            sell_count: as_opt_f64(&r[7]),
            net: as_opt_f64(&r[8]),
        });
    }
    Ok(out)
}

fn parse_lhb_jgmx(html: &str, endpoint: &'static str) -> Result<Vec<LhbJgmxRow>> {
    let rows = data_rows(html, endpoint, "股票代码")?;
    let mut out = Vec::new();
    for r in &rows {
        if r.len() < 6 {
            continue;
        }
        out.push(LhbJgmxRow {
            code: zfill6(&r[0]),
            name: r[1].clone(),
            date: r[2].clone(),
            inst_buy: as_opt_f64(&r[3]),
            inst_sell: as_opt_f64(&r[4]),
            kind: r[5].clone(),
        });
    }
    Ok(out)
}

/// 龙虎榜-每日详情.
pub async fn stock_lhb_detail_daily_sina(client: &Client, date: &str) -> Result<Vec<LhbDailyRow>> {
    let endpoint = "stock_lhb_detail_daily_sina";
    let url = "https://vip.stock.finance.sina.com.cn/q/go.php/vInvestConsult/kind/lhb/index.phtml";
    let tradedate = format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..]);
    let html = client
        .get_text(SOURCE_SINA, endpoint, url, &[("tradedate", &tradedate)], None)
        .await?;
    parse_lhb_detail_daily(&html, endpoint)
}

/// Shared Sina lhb pagination driver (params include `last` + `p`).
async fn sina_lhb_paginate<T, F>(
    client: &Client,
    endpoint: &'static str,
    url: &str,
    last: &str,
    parse: F,
) -> Result<Vec<T>>
where
    F: Fn(&str, &'static str) -> Result<Vec<T>>,
{
    let html1 = client
        .get_text(SOURCE_SINA, endpoint, url, &[("last", last), ("p", "1")], None)
        .await?;
    let last_page = parse_sina_last_page(&html1, endpoint);
    let mut out = parse(&html1, endpoint)?;
    for p in 2..=last_page {
        let html = client
            .get_text(
                SOURCE_SINA,
                endpoint,
                url,
                &[("last", last), ("p", p.to_string().as_str())],
                None,
            )
            .await?;
        out.extend(parse(&html, endpoint)?);
    }
    Ok(out)
}

/// Sina lhb pagination driver without a `last` param (jgmx).
async fn sina_lhb_paginate_nolast<T, F>(
    client: &Client,
    endpoint: &'static str,
    url: &str,
    parse: F,
) -> Result<Vec<T>>
where
    F: Fn(&str, &'static str) -> Result<Vec<T>>,
{
    let html1 = client
        .get_text(SOURCE_SINA, endpoint, url, &[("p", "1")], None)
        .await?;
    let last_page = parse_sina_last_page(&html1, endpoint);
    let mut out = parse(&html1, endpoint)?;
    for p in 2..=last_page {
        let html = client
            .get_text(
                SOURCE_SINA,
                endpoint,
                url,
                &[("p", p.to_string().as_str())],
                None,
            )
            .await?;
        out.extend(parse(&html, endpoint)?);
    }
    Ok(out)
}

/// 龙虎榜-个股上榜统计.
pub async fn stock_lhb_ggtj_sina(client: &Client, symbol: &str) -> Result<Vec<LhbGgtjRow>> {
    sina_lhb_paginate(
        client,
        "stock_lhb_ggtj_sina",
        "https://vip.stock.finance.sina.com.cn/q/go.php/vLHBData/kind/ggtj/index.phtml",
        symbol,
        parse_lhb_ggtj,
    )
    .await
}

/// 龙虎榜-营业部上榜统计.
pub async fn stock_lhb_yytj_sina(client: &Client, symbol: &str) -> Result<Vec<LhbYytjRow>> {
    sina_lhb_paginate(
        client,
        "stock_lhb_yytj_sina",
        "https://vip.stock.finance.sina.com.cn/q/go.php/vLHBData/kind/yytj/index.phtml",
        symbol,
        parse_lhb_yytj,
    )
    .await
}

/// 龙虎榜-机构席位追踪.
pub async fn stock_lhb_jgzz_sina(client: &Client, symbol: &str) -> Result<Vec<LhbJgzzRow>> {
    sina_lhb_paginate(
        client,
        "stock_lhb_jgzz_sina",
        "https://vip.stock.finance.sina.com.cn/q/go.php/vLHBData/kind/jgzz/index.phtml",
        symbol,
        parse_lhb_jgzz,
    )
    .await
}

/// 龙虎榜-机构席位成交明细.
pub async fn stock_lhb_jgmx_sina(client: &Client) -> Result<Vec<LhbJgmxRow>> {
    sina_lhb_paginate_nolast(
        client,
        "stock_lhb_jgmx_sina",
        "https://vip.stock.finance.sina.com.cn/q/go.php/vLHBData/kind/jgmx/index.phtml",
        parse_lhb_jgmx,
    )
    .await
}

// ===========================================================================
// Legu 赚钱效应 — table + metric divs (item/value pairs)
// ===========================================================================

/// One 赚钱效应 metric (akshare output is a two-column `item`/`value` frame).
#[derive(Debug, Clone, serde::Serialize)]
pub struct LeguActivityRow {
    /// Metric name (akshare `item`).
    pub item: String,
    /// Metric value (akshare `value`).
    pub value: String,
}

fn parse_market_activity_legu(html: &str, endpoint: &'static str) -> Result<Vec<LeguActivityRow>> {
    let mut out = Vec::new();
    let tables = extract_tables(html, endpoint, "table")?;
    for r in &tables[0] {
        let mut i = 0;
        while i + 1 < r.len() {
            out.push(LeguActivityRow {
                item: r[i].clone(),
                value: r[i + 1].clone(),
            });
            i += 2;
        }
    }
    out.retain(|r| !(r.item.is_empty() && r.value.is_empty()));

    let doc = Html::parse_document(html);
    // `活跃度44.59%` style metric.
    if let Ok(sel) = Selector::parse("div.metric-activity") {
        if let Some(e) = doc.select(&sel).next() {
            let txt = e.text().collect::<String>();
            let (item, value) = split_label_value(&txt);
            if !item.is_empty() {
                out.push(LeguActivityRow { item, value });
            }
        }
    }
    // `统计日期` meta.
    if let Ok(sel) = Selector::parse("div.market-activity-meta") {
        if let Some(e) = doc.select(&sel).next() {
            let value = e.text().collect::<String>().trim().to_string();
            if !value.is_empty() {
                out.push(LeguActivityRow {
                    item: "统计日期".to_string(),
                    value,
                });
            }
        }
    }
    Ok(out)
}

/// 乐咕乐股网-赚钱效应分析 (`stock_market_activity_legu`, akshare `stock_market_legu.py:18`).
pub async fn stock_market_activity_legu(client: &Client) -> Result<Vec<LeguActivityRow>> {
    let endpoint = "stock_market_activity_legu";
    let url = "https://legulegu.com/stockdata/market-activity";
    let html = client.get_text(SOURCE_LEGU, endpoint, url, &[], None).await?;
    parse_market_activity_legu(&html, endpoint)
}

// ===========================================================================
// SSE e互动 提问与回答 — AJAX feed HTML
// ===========================================================================

/// One Q&A pair from 上证e互动 (akshare columns 股票代码/公司简称/问题/回答/...).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SseInfoRow {
    /// 股票代码
    pub code: String,
    /// 公司简称
    pub company: String,
    /// 问题
    pub question: String,
    /// 回答
    pub answer: String,
    /// 问题时间
    pub q_time: String,
    /// 回答时间
    pub a_time: String,
    /// 问题来源
    pub q_source: String,
    /// 回答来源
    pub a_source: String,
    /// 用户名
    pub author: String,
}

/// Split a question's leading `:公司简称(代码)` prefix from the question body.
fn split_q(s: &str) -> (String, String, String) {
    let trimmed = s.trim();
    let s = trimmed
        .strip_prefix(':')
        .or_else(|| trimmed.strip_prefix('\u{FF1A}'))
        .unwrap_or(trimmed)
        .trim();
    if let Some(idx) = s.find('(') {
        let company = s[..idx].trim().to_string();
        let rest = &s[idx + 1..];
        if let Some(end) = rest.find(')') {
            let code = rest[..end].trim().to_string();
            let question = rest[end + 1..].trim().to_string();
            return (company, code, question);
        }
    }
    (s.to_string(), String::new(), String::new())
}

/// Split the `m_feed_from` text into `(date, source)` at `来自`.
fn split_from(s: &str) -> (String, String) {
    let s = s.trim();
    if let Some(idx) = s.find("来自") {
        (s[..idx].trim().to_string(), s[idx..].trim().to_string())
    } else {
        (s.to_string(), String::new())
    }
}

fn parse_sseinfo(html: &str, _endpoint: &'static str) -> Result<Vec<SseInfoRow>> {
    let doc = Html::parse_document(html);
    let txt_sel = Selector::parse("div.m_feed_txt").unwrap();
    let from_sel = Selector::parse("div.m_feed_from").unwrap();
    let face_sel = Selector::parse("a[rel=face]").unwrap();
    let txt: Vec<String> = doc
        .select(&txt_sel)
        .map(|e| e.text().collect::<String>())
        .collect();
    let from: Vec<String> = doc
        .select(&from_sel)
        .map(|e| e.text().collect::<String>())
        .collect();
    let faces: Vec<String> = doc
        .select(&face_sel)
        .filter_map(|e| e.value().attr("title").map(|t| t.to_string()))
        .collect();
    let pairs = txt.len() / 2;
    let mut out = Vec::with_capacity(pairs);
    for i in 0..pairs {
        let q = txt.get(2 * i).cloned().unwrap_or_default();
        let a = txt.get(2 * i + 1).cloned().unwrap_or_default();
        let (company, code, question) = split_q(&q);
        let (q_time, q_src) = split_from(from.get(2 * i).cloned().unwrap_or_default().as_str());
        let (a_time, a_src) = split_from(from.get(2 * i + 1).cloned().unwrap_or_default().as_str());
        let author = faces.get(i).cloned().unwrap_or_default();
        out.push(SseInfoRow {
            code,
            company,
            question,
            answer: a,
            q_time,
            a_time,
            q_source: q_src,
            a_source: a_src,
            author,
        });
    }
    Ok(out)
}

/// 上证e互动-提问与回答 (`stock_sns_sseinfo`, akshare `stock_sns_sseinfo.py:56`).
///
/// Mirrors akshare: first build a code→uid map by paging `allcompany.do`
/// (72 pages), then page the `userfeeds.do` feed for the resolved `uid`.
pub async fn stock_sns_sseinfo(client: &Client, symbol: &str) -> Result<Vec<SseInfoRow>> {
    let endpoint = "stock_sns_sseinfo";
    let mut code_uid: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for page in 1..=72u32 {
        let v = client
            .post_form_json(
                SOURCE_SSEINFO,
                "stock_sns_sseinfo_uid",
                "https://sns.sseinfo.com/allcompany.do",
                &[
                    ("code", "0"),
                    ("order", "2"),
                    ("areaId", "0"),
                    ("page", page.to_string().as_str()),
                ],
                None,
            )
            .await?;
        if let Some(content) = v.get("content").and_then(|c| c.as_str()) {
            let doc = Html::parse_document(content);
            let a_sel = Selector::parse("a[rel=tag]").unwrap();
            let img_sel = Selector::parse("img").unwrap();
            for a in doc.select(&a_sel) {
                let Some(uid) = a.value().attr("uid") else {
                    continue;
                };
                if let Some(src) = a.select(&img_sel).next().and_then(|i| i.value().attr("src")) {
                    if let Some(code) = src.rsplit('/').next().and_then(|s| s.split('.').next()) {
                        code_uid.insert(code.to_string(), uid.to_string());
                    }
                }
            }
        }
    }
    let uid = code_uid.get(symbol).ok_or_else(|| Error::NotFound {
        endpoint,
        message: format!("stock code {symbol} not found in sseinfo uid map"),
    })?;

    let mut page: u32 = 1;
    let mut out = Vec::new();
    loop {
        let html = client
            .get_text(
                SOURCE_SSEINFO,
                endpoint,
                "https://sns.sseinfo.com/ajax/userfeeds.do",
                &[
                    ("typeCode", "company"),
                    ("type", "11"),
                    ("pageSize", "100"),
                    ("uid", uid),
                    ("page", page.to_string().as_str()),
                ],
                None,
            )
            .await?;
        let rows = parse_sseinfo(&html, endpoint)?;
        if rows.is_empty() {
            break;
        }
        out.extend(rows);
        page += 1;
        if page > 200 {
            break;
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    /// Load an HTML fixture as text, decoding gbk when the file is not UTF-8.
    fn load_html(name: &str) -> String {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        match std::str::from_utf8(&bytes) {
            Ok(s) => s.to_string(),
            Err(_) => match encoding_rs::GBK
                .decode_without_bom_handling_and_without_replacement(&bytes)
            {
                Some(cow) => cow.into_owned(),
                None => String::from_utf8_lossy(&bytes).into_owned(),
            },
        }
    }

    /// Load a JSON fixture as `Value`.
    fn load_json(name: &str) -> Value {
        let p = fixture_path(name);
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    #[test]
    fn parses_stock_board_concept_info_ths() {
        let rows = parse_ths_board_info(
            &load_html("stock_board_concept_info_ths.html"),
            "stock_board_concept_info_ths",
        )
        .unwrap();
        assert_eq!(rows.len(), 10);
        assert!(rows.iter().any(|r| r.item == "今开"));
    }

    #[test]
    fn parses_stock_board_industry_info_ths() {
        let rows = parse_ths_board_info(
            &load_html("stock_board_industry_info_ths.html"),
            "stock_board_industry_info_ths",
        )
        .unwrap();
        assert_eq!(rows.len(), 10);
        assert!(rows.iter().any(|r| r.item == "板块涨幅"));
    }

    #[test]
    fn parses_stock_classify_board() {
        let rows =
            parse_stock_classify_board(&load_json("stock_classify_board.json"), "stock_classify_board")
                .unwrap();
        assert!(rows.len() > 100, "expected many leaves, got {}", rows.len());
        let first = &rows[0];
        assert_eq!(first.class_name, "新浪行业");
        assert_eq!(first.code, "new_blhy");
    }

    #[test]
    fn parses_stock_fhps_detail_ths() {
        let rows = parse_fhps_detail(&load_html("stock_fhps_detail_ths.html"), "stock_fhps_detail_ths")
            .unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].report_period, "2026三季报");
    }

    #[test]
    fn parses_stock_lh_yyb_most() {
        let rows =
            parse_yyb_most(&load_html("stock_lh_yyb_most.html"), "stock_lh_yyb_most").unwrap();
        assert!(!rows.is_empty());
        assert!(!rows[0].branch.is_empty());
    }

    #[test]
    fn parses_stock_lh_yyb_capital() {
        let rows = parse_yyb_capital(&load_html("stock_lh_yyb_capital.html"), "stock_lh_yyb_capital")
            .unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].max_amount.contains("亿"));
    }

    #[test]
    fn parses_stock_lh_yyb_control() {
        let rows = parse_yyb_control(&load_html("stock_lh_yyb_control.html"), "stock_lh_yyb_control")
            .unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].success_rate.contains("%"));
    }

    #[test]
    fn parses_stock_lhb_detail_daily_sina() {
        let rows = parse_lhb_detail_daily(
            &load_html("stock_lhb_detail_daily_sina.html"),
            "stock_lhb_detail_daily_sina",
        )
        .unwrap();
        assert!(!rows.is_empty());
        assert!(!rows[0].indicator.is_empty());
        assert!(rows[0].code.len() <= 6);
    }

    #[test]
    fn parses_stock_lhb_ggtj_sina() {
        let rows =
            parse_lhb_ggtj(&load_html("stock_lhb_ggtj_sina.html"), "stock_lhb_ggtj_sina").unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].buy.is_some());
    }

    #[test]
    fn parses_stock_lhb_yytj_sina() {
        let rows =
            parse_lhb_yytj(&load_html("stock_lhb_yytj_sina.html"), "stock_lhb_yytj_sina").unwrap();
        assert!(!rows.is_empty());
    }

    #[test]
    fn parses_stock_lhb_jgzz_sina() {
        let rows =
            parse_lhb_jgzz(&load_html("stock_lhb_jgzz_sina.html"), "stock_lhb_jgzz_sina").unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].buy.is_some());
    }

    #[test]
    fn parses_stock_lhb_jgmx_sina() {
        let rows =
            parse_lhb_jgmx(&load_html("stock_lhb_jgmx_sina.html"), "stock_lhb_jgmx_sina").unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].date.starts_with("202"));
    }

    #[test]
    fn parses_stock_market_activity_legu() {
        let rows = parse_market_activity_legu(
            &load_html("stock_market_activity_legu.html"),
            "stock_market_activity_legu",
        )
        .unwrap();
        assert!(!rows.is_empty());
        assert!(rows.iter().any(|r| r.item == "统计日期"));
    }

    #[test]
    fn parses_stock_sns_sseinfo() {
        let rows =
            parse_sseinfo(&load_html("stock_sns_sseinfo.html"), "stock_sns_sseinfo").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].code, "600000");
        assert_eq!(rows[0].company, "浦发银行");
        assert!(!rows[0].question.is_empty());
        assert!(!rows[0].answer.is_empty());
    }
}
