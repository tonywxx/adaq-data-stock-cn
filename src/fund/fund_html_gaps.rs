//! Fund-domain HTML/JS-scraping gap fillers (akshare `fund` package).
//!
//! Ports 16 akshare functions whose upstreams are HTML tables, JS-wrapped HTML,
//! JSONP, or `var x = {...}` JavaScript literals (NOT the Eastmoney push2 kline
//! endpoints covered elsewhere). Each `parse_*` takes the raw upstream text and a
//! fixed `endpoint` string (per the shared HTML-porting pattern); the async
//! `fund_*` wrappers fetch the live URL and hand the body to `parse_*`.
//!
//! Sources (akshare `fund/`):
//! * `fund_aum_em.py` — `fund_aum_em`, `fund_aum_hist_em`
//! * `fund_em.py` — `fund_etf_fund_daily_em`, `fund_money_fund_daily_em`
//! * `fund_fee_em.py` — `fund_fee_em`
//! * `fund_info_ths.py` — `fund_info_ths` (THS gbk/utf8)
//! * `fund_overview_em.py` — `fund_overview_em`
//! * `fund_portfolio_em.py` — `fund_portfolio_hold_em`, `_bond_hold_em`,
//!   `_change_em`, `_industry_allocation_em`
//! * `fund_rating.py` — `fund_rating_all`, `_sh`, `_zs`, `_ja`
//! * `fund_etf_sina.py` — `fund_etf_dividend_sina`

use scraper::{Html, Selector};
use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_EASTMONEY: &str = "eastmoney";
const SOURCE_THS: &str = "ths";
const SOURCE_SINA: &str = "sina";

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

/// Parse an HTML document into a list of tables; each table is a list of rows;
/// each row is a list of trimmed cell strings (td+th). Mirrors `pd.read_html`.
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

/// Parse a numeric string, tolerating thousands separators, `%` and `---`/empty.
fn parse_num(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', "").replace('%', "");
    let t = t.trim();
    if t.is_empty() || t == "---" || t == "--" || t == "None" {
        None
    } else {
        t.parse::<f64>().ok()
    }
}

/// Extract a `var fundinfos = "...";` data string from a fund-rating page.
fn extract_fundinfos(html: &str, endpoint: &'static str) -> Result<String> {
    let marker = "fundinfos = \"";
    let idx = html
        .find(marker)
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "fundinfos var not found".into() })?;
    let start = idx + marker.len();
    let end = html[start..]
        .find('"')
        .map(|e| start + e)
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "unterminated fundinfos".into() })?;
    Ok(html[start..end].to_string())
}

/// Extract the `content` HTML string from a `FundArchivesDatas.aspx` JS response
/// (`var apidata={ content:"...", ... }`). Inner HTML uses single-quoted
/// attributes, so the first unescaped `"` after `content:"` closes the value.
fn extract_archives_content(html: &str, endpoint: &'static str) -> Result<String> {
    let marker = "content:\"";
    let idx = html
        .find(marker)
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "content field not found".into() })?;
    let start = idx + marker.len();
    let end = html[start..]
        .find('"')
        .map(|e| start + e)
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "unterminated content".into() })?;
    Ok(html[start..end].to_string())
}

/// Pull a balanced JSON object out of a `var x = {...};` / `cb({...});` wrapper.
fn extract_json_object(text: &str, endpoint: &'static str) -> Result<String> {
    let open = text.find('{').ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "no JSON object".into() })?;
    let close = text.rfind('}').ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "no JSON object".into() })?;
    if close <= open {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "malformed JSON object".into() });
    }
    Ok(text[open..=close].to_string())
}

// ---------------------------------------------------------------------------
// fund_aum_em / fund_aum_hist_em — Eastmoney fund-company AUM ranking
// ---------------------------------------------------------------------------

/// One fund-management-company AUM ranking row (`fund_aum_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundAumRow {
    /// 序号 (rank).
    pub rank: String,
    /// 基金公司.
    pub company: String,
    /// 成立时间.
    pub establish_date: String,
    /// 全部管理规模 (亿元), numeric.
    pub aum: Option<f64>,
    /// 更新日期 (extracted from the scale cell, e.g. `07-31`).
    pub update_date: String,
    /// 全部基金数.
    pub fund_count: Option<f64>,
    /// 全部经理数.
    pub manager_count: Option<f64>,
}

/// 东方财富-基金-基金公司排名列表 (`fund_aum_em`, akshare `fund_aum_em.py:14`).
pub async fn fund_aum_em(client: &Client) -> Result<Vec<FundAumRow>> {
    let url = "https://fund.eastmoney.com/Company/home/gspmlist";
    let html = client
        .get_text(SOURCE_EASTMONEY, "fund_aum_em", url, &[("fundType", "0")], None)
        .await?;
    parse_fund_aum_em(&html, "fund_aum_em")
}

pub(crate) fn parse_fund_aum_em(html: &str, endpoint: &'static str) -> Result<Vec<FundAumRow>> {
    let tables = extract_tables(html, endpoint)?;
    let rows = tables
        .into_iter()
        .next()
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "no table[0]".into() })?;
    let mut out = Vec::new();
    for cells in rows.into_iter().skip(1) {
        if cells.len() < 8 {
            continue;
        }
        // cells: 序号, 基金公司, 相关链接, 成立时间, 天相评级, 全部管理规模(亿元), 全部基金数, 全部经理数
        let scale = cells[5].split_whitespace().collect::<Vec<&str>>();
        let aum = scale.first().and_then(|s| parse_num(s));
        let update_date = scale.get(1).unwrap_or(&"").to_string();
        out.push(FundAumRow {
            rank: cells[0].clone(),
            company: cells[1].clone(),
            establish_date: cells[3].clone(),
            aum,
            update_date,
            fund_count: parse_num(&cells[6]),
            manager_count: parse_num(&cells[7]),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "empty aum parse".into() });
    }
    Ok(out)
}

/// One historical AUM ranking row (`fund_aum_hist_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundAumHistRow {
    pub rank: String,
    pub company: String,
    pub total_scale: Option<f64>,
    pub stock: Option<f64>,
    pub hybrid: Option<f64>,
    pub bond: Option<f64>,
    pub index: Option<f64>,
    pub qdii: Option<f64>,
    pub money: Option<f64>,
}

/// 东方财富-基金-基金公司历年管理规模排行 (`fund_aum_hist_em`, akshare `fund_aum_em.py:64`).
pub async fn fund_aum_hist_em(client: &Client, year: &str) -> Result<Vec<FundAumHistRow>> {
    let url = "https://fund.eastmoney.com/Company/home/HistoryScaleTable";
    let html = client
        .get_text(SOURCE_EASTMONEY, "fund_aum_hist_em", url, &[("year", year)], None)
        .await?;
    parse_fund_aum_hist_em(&html, "fund_aum_hist_em")
}

pub(crate) fn parse_fund_aum_hist_em(html: &str, endpoint: &'static str) -> Result<Vec<FundAumHistRow>> {
    let tables = extract_tables(html, endpoint)?;
    let rows = tables
        .into_iter()
        .next()
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "no table[0]".into() })?;
    let mut out = Vec::new();
    for cells in rows.into_iter().skip(1) {
        if cells.len() < 9 {
            continue;
        }
        out.push(FundAumHistRow {
            rank: cells[0].clone(),
            company: cells[1].clone(),
            total_scale: parse_num(&cells[2]),
            stock: parse_num(&cells[3]),
            hybrid: parse_num(&cells[4]),
            bond: parse_num(&cells[5]),
            index: parse_num(&cells[6]),
            qdii: parse_num(&cells[7]),
            money: parse_num(&cells[8]),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "empty aum hist parse".into() });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// fund_etf_fund_daily_em / fund_money_fund_daily_em — Eastmoney daily lists
// ---------------------------------------------------------------------------

/// One on-exchange (场内) fund daily row (`fund_etf_fund_daily_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundEtfFundDailyRow {
    pub fund_code: String,
    pub fund_name: String,
    pub fund_type: String,
    pub date0: String,
    pub date1: String,
    pub nav_day0: Option<f64>,
    pub accum_nav_day0: Option<f64>,
    pub nav_day1: Option<f64>,
    pub accum_nav_day1: Option<f64>,
    pub increase_value: Option<f64>,
    pub increase_rate: String,
    pub market_price: Option<f64>,
    pub discount_rate: String,
}

/// 东方财富-场内交易基金每日净值 (`fund_etf_fund_daily_em`, akshare `fund_em.py:1064`).
pub async fn fund_etf_fund_daily_em(client: &Client) -> Result<Vec<FundEtfFundDailyRow>> {
    let url = "https://fund.eastmoney.com/cnjy_dwjz.html";
    let html = client
        .get_text(SOURCE_EASTMONEY, "fund_etf_fund_daily_em", url, &[], None)
        .await?;
    parse_fund_etf_fund_daily_em(&html, "fund_etf_fund_daily_em")
}

pub(crate) fn parse_fund_etf_fund_daily_em(html: &str, endpoint: &'static str) -> Result<Vec<FundEtfFundDailyRow>> {
    let tables = extract_tables(html, endpoint)?;
    let table = tables
        .into_iter()
        .nth(1)
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "no table[1]".into() })?;
    if table.len() < 3 {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "table[1] too small".into() });
    }
    let date0 = table[0].get(6).cloned().unwrap_or_default();
    let date1 = table[0].get(7).cloned().unwrap_or_default();
    let mut out = Vec::new();
    for cells in table.into_iter().skip(2) {
        if cells.len() < 14 {
            continue;
        }
        out.push(FundEtfFundDailyRow {
            fund_code: cells[3].clone(),
            fund_name: cells[4].replace("行情吧档案", "").trim().to_string(),
            fund_type: cells[5].clone(),
            date0: date0.clone(),
            date1: date1.clone(),
            nav_day0: parse_num(&cells[6]),
            accum_nav_day0: parse_num(&cells[7]),
            nav_day1: parse_num(&cells[8]),
            accum_nav_day1: parse_num(&cells[9]),
            increase_value: parse_num(&cells[10]),
            increase_rate: cells[11].clone(),
            market_price: parse_num(&cells[12]),
            discount_rate: cells[13].clone(),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "empty etf daily parse".into() });
    }
    Ok(out)
}

/// One money-market fund daily row (`fund_money_fund_daily_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundMoneyFundDailyRow {
    pub fund_code: String,
    pub fund_name: String,
    pub date0: String,
    pub date1: String,
    pub wane_income_day0: Option<f64>,
    pub annualized_day0: String,
    pub nav_day0: Option<f64>,
    pub wane_income_day1: Option<f64>,
    pub annualized_day1: String,
    pub nav_day1: Option<f64>,
    pub daily_change: String,
    pub establish_date: String,
    pub fund_manager: String,
    pub fee: String,
    pub purchasable: String,
}

/// 东方财富-货币型基金收益 (`fund_money_fund_daily_em`, akshare `fund_em.py:707`).
pub async fn fund_money_fund_daily_em(client: &Client) -> Result<Vec<FundMoneyFundDailyRow>> {
    let url = "https://fund.eastmoney.com/HBJJ_pjsyl.html";
    let html = client
        .get_text(SOURCE_EASTMONEY, "fund_money_fund_daily_em", url, &[], None)
        .await?;
    parse_fund_money_fund_daily_em(&html, "fund_money_fund_daily_em")
}

pub(crate) fn parse_fund_money_fund_daily_em(html: &str, endpoint: &'static str) -> Result<Vec<FundMoneyFundDailyRow>> {
    let tables = extract_tables(html, endpoint)?;
    let table = tables
        .into_iter()
        .nth(1)
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "no table[1]".into() })?;
    if table.len() < 3 {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "table[1] too small".into() });
    }
    let date0 = table[0].get(5).cloned().unwrap_or_default();
    let date1 = table[0].get(6).cloned().unwrap_or_default();
    let mut out = Vec::new();
    for cells in table.into_iter().skip(2) {
        if cells.len() < 16 {
            continue;
        }
        out.push(FundMoneyFundDailyRow {
            fund_code: cells[3].clone(),
            fund_name: cells[4].replace("基金吧档案", "").trim().to_string(),
            date0: date0.clone(),
            date1: date1.clone(),
            wane_income_day0: parse_num(&cells[5]),
            annualized_day0: cells[6].clone(),
            nav_day0: parse_num(&cells[7]),
            wane_income_day1: parse_num(&cells[8]),
            annualized_day1: cells[9].clone(),
            nav_day1: parse_num(&cells[10]),
            daily_change: cells[11].clone(),
            establish_date: cells[12].clone(),
            fund_manager: cells[13].clone(),
            fee: cells[14].clone(),
            purchasable: cells[15].clone(),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "empty money daily parse".into() });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// fund_overview_em — Eastmoney fund basic-info key/value table
// ---------------------------------------------------------------------------

/// One key/value field of a fund's basic overview (`fund_overview_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundOverviewRow {
    pub field: String,
    pub value: String,
}

/// 东方财富-基金档案-基本概况 (`fund_overview_em`, akshare `fund_overview_em.py:15`).
pub async fn fund_overview_em(client: &Client, symbol: &str) -> Result<Vec<FundOverviewRow>> {
    let url = format!("https://fundf10.eastmoney.com/jbgk_{symbol}.html");
    let html = client
        .get_text(SOURCE_EASTMONEY, "fund_overview_em", &url, &[], None)
        .await?;
    parse_fund_overview_em(&html, "fund_overview_em")
}

pub(crate) fn parse_fund_overview_em(html: &str, endpoint: &'static str) -> Result<Vec<FundOverviewRow>> {
    let tables = extract_tables(html, endpoint)?;
    let table = tables
        .into_iter()
        .last()
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "no table".into() })?;
    let mut out = Vec::new();
    for cells in table {
        let mut i = 0;
        while i + 1 < cells.len() {
            let k = cells[i].trim().to_string();
            let v = cells[i + 1].trim().to_string();
            if !k.is_empty() {
                out.push(FundOverviewRow { field: k, value: v });
            }
            i += 2;
        }
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "empty overview parse".into() });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// fund_info_ths — THS fund basic-info (ul.g-dialog key/value list)
// ---------------------------------------------------------------------------

/// One key/value field of a THS fund-info page (`fund_info_ths`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundInfoThsRow {
    pub field: String,
    pub value: String,
}

/// 同花顺-基金基本信息 (`fund_info_ths`, akshare `fund_info_ths.py:16`).
pub async fn fund_info_ths(client: &Client, symbol: &str) -> Result<Vec<FundInfoThsRow>> {
    let url = format!("https://fund.10jqka.com.cn/{symbol}/interduce.html");
    let html = client
        .get_text(SOURCE_THS, "fund_info_ths", &url, &[], Some(&[("referer", "https://fund.10jqka.com.cn/")]))
        .await?;
    parse_fund_info_ths(&html, "fund_info_ths")
}

pub(crate) fn parse_fund_info_ths(html: &str, endpoint: &'static str) -> Result<Vec<FundInfoThsRow>> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("ul.g-dialog li")
        .map_err(|e| Error::Parse { endpoint, message: format!("li selector: {e}") })?;
    let key_sel = Selector::parse("span.key").unwrap();
    let val_sel = Selector::parse("span.value").unwrap();
    let mut out = Vec::new();
    for li in doc.select(&sel) {
        let key: String = li.select(&key_sel).next().map(|e| e.text().collect::<String>()).unwrap_or_default().trim().to_string();
        let value: String = li.select(&val_sel).next().map(|e| e.text().collect::<String>()).unwrap_or_default().trim().to_string();
        if !key.is_empty() {
            out.push(FundInfoThsRow { field: key, value });
        }
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no g-dialog fields".into() });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// fund_fee_em — Eastmoney fund fee tables (h4.t sections)
// ---------------------------------------------------------------------------

/// One row of a fund-fee section table (`fund_fee_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundFeeRow {
    /// Section title (the `h4.t` heading, e.g. 运作费用).
    pub section: String,
    /// First cell of the table row (often the 项目 / 状态 label).
    pub item: String,
    /// Remaining cells joined (the detail / 费率 column(s)).
    pub detail: String,
}

/// 东方财富-基金档案-购买信息 (`fund_fee_em`, akshare `fund_fee_em.py:17`).
///
/// Parses every `h4.t` section + its following table into rows. When `indicator`
/// is non-empty and not `"all"`, only rows whose section contains it are kept.
pub async fn fund_fee_em(client: &Client, symbol: &str, indicator: &str) -> Result<Vec<FundFeeRow>> {
    let url = format!("https://fundf10.eastmoney.com/jjfl_{symbol}.html");
    let html = client
        .get_text(SOURCE_EASTMONEY, "fund_fee_em", &url, &[], None)
        .await?;
    let mut rows = parse_fund_fee_em(&html, "fund_fee_em")?;
    if !indicator.is_empty() && indicator != "all" {
        rows.retain(|r| r.section.contains(indicator));
    }
    Ok(rows)
}

pub(crate) fn parse_fund_fee_em(html: &str, endpoint: &'static str) -> Result<Vec<FundFeeRow>> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("h4.t, table").unwrap();
    let tr_sel = Selector::parse("tr").unwrap();
    let cell_sel = Selector::parse("td,th").unwrap();
    let mut cur_section = String::new();
    let mut out = Vec::new();
    for el in doc.select(&sel) {
        if el.value().name() == "h4" {
            cur_section = el.text().collect::<String>().trim().to_string();
        } else {
            let mut rows: Vec<Vec<String>> = Vec::new();
            for tr in el.select(&tr_sel) {
                let cells: Vec<String> = tr
                    .select(&cell_sel)
                    .map(|c| c.text().collect::<Vec<_>>().join(" ").trim().to_string())
                    .collect();
                if !cells.is_empty() {
                    rows.push(cells);
                }
            }
            for cells in rows {
                let item = cells.first().cloned().unwrap_or_default();
                let detail = cells.iter().skip(1).cloned().collect::<Vec<_>>().join(" | ");
                out.push(FundFeeRow { section: cur_section.clone(), item, detail });
            }
        }
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no fee tables".into() });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// fund_rating_* — Eastmoney fund-rating pages (script `var fundinfos`)
// ---------------------------------------------------------------------------

fn rating_fields(value: &str) -> Vec<Vec<String>> {
    value
        .split("|_")
        .filter(|c| !c.trim().is_empty())
        .map(|c| c.split('|').map(|s| s.trim().to_string()).collect::<Vec<_>>())
        .collect()
}

fn f<'a>(fields: &'a [String], idx: usize) -> &'a str {
    fields.get(idx).map(|s| s.as_str()).unwrap_or("")
}

fn f_num(fields: &[String], idx: usize) -> Option<f64> {
    let s = f(fields, idx);
    if s.is_empty() { None } else { parse_num(s) }
}

/// One fund-rating (all agencies) row (`fund_rating_all`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundRatingAllRow {
    pub code: String,
    pub name: String,
    pub fund_type: String,
    pub manager: String,
    pub company: String,
    pub five_star_count: Option<f64>,
    pub shanghai_sec: Option<f64>,
    pub zhaoshang_sec: Option<f64>,
    pub jianan: Option<f64>,
    pub morningstar: Option<f64>,
    /// 手续费 as a fraction (akshare divides the `%` value by 100).
    pub fee: Option<f64>,
}

/// 天天基金网-基金评级-基金评级总汇 (`fund_rating_all`, akshare `fund_rating.py:14`).
pub async fn fund_rating_all(client: &Client) -> Result<Vec<FundRatingAllRow>> {
    let url = "https://fund.eastmoney.com/data/fundrating.html";
    let html = client
        .get_text(SOURCE_EASTMONEY, "fund_rating_all", url, &[], None)
        .await?;
    parse_fund_rating_all(&html, "fund_rating_all")
}

pub(crate) fn parse_fund_rating_all(html: &str, endpoint: &'static str) -> Result<Vec<FundRatingAllRow>> {
    let value = extract_fundinfos(html, endpoint)?;
    let rows = rating_fields(&value);
    let mut out = Vec::new();
    for fields in rows {
        out.push(FundRatingAllRow {
            code: f(&fields, 0).to_string(),
            name: f(&fields, 1).to_string(),
            fund_type: f(&fields, 2).to_string(),
            manager: f(&fields, 3).to_string(),
            company: f(&fields, 5).to_string(),
            five_star_count: f_num(&fields, 7),
            zhaoshang_sec: f_num(&fields, 10),
            shanghai_sec: f_num(&fields, 12),
            morningstar: f_num(&fields, 14),
            jianan: f_num(&fields, 16),
            fee: f_num(&fields, 18).map(|x| x / 100.0),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "empty rating parse".into() });
    }
    Ok(out)
}

/// One Shanghai-Securities rating row (`fund_rating_sh`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundRatingShRow {
    pub code: String,
    pub name: String,
    pub fund_type: String,
    pub manager: String,
    pub company: String,
    pub rating_3y: Option<f64>,
    pub rating_3y_prev: Option<f64>,
    pub rating_5y: Option<f64>,
    pub rating_5y_prev: Option<f64>,
    pub nav: Option<f64>,
    pub date: String,
    pub daily_growth: Option<f64>,
    pub y1: Option<f64>,
    pub y3: Option<f64>,
    pub y5: Option<f64>,
    pub fee: String,
}

/// 天天基金网-基金评级-上海证券评级 (`fund_rating_sh`, akshare `fund_rating.py:91`).
pub async fn fund_rating_sh(client: &Client, date: &str) -> Result<Vec<FundRatingShRow>> {
    let url = format!("https://fund.eastmoney.com/data/fundrating_3_{}.html", fmt_date(date));
    let html = client
        .get_text(SOURCE_EASTMONEY, "fund_rating_sh", &url, &[], None)
        .await?;
    parse_fund_rating_sh(&html, "fund_rating_sh")
}

pub(crate) fn parse_fund_rating_sh(html: &str, endpoint: &'static str) -> Result<Vec<FundRatingShRow>> {
    let value = extract_fundinfos(html, endpoint)?;
    let rows = rating_fields(&value);
    let mut out = Vec::new();
    for fields in rows {
        out.push(FundRatingShRow {
            code: f(&fields, 0).to_string(),
            name: f(&fields, 1).to_string(),
            fund_type: f(&fields, 2).to_string(),
            manager: f(&fields, 3).to_string(),
            company: f(&fields, 5).to_string(),
            rating_3y: f_num(&fields, 7),
            rating_3y_prev: f_num(&fields, 8),
            rating_5y: f_num(&fields, 9),
            rating_5y_prev: f_num(&fields, 10),
            nav: f_num(&fields, 11),
            date: f(&fields, 12).to_string(),
            daily_growth: f_num(&fields, 13),
            y1: f_num(&fields, 14),
            y3: f_num(&fields, 15),
            y5: f_num(&fields, 16),
            fee: f(&fields, 17).to_string(),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "empty rating parse".into() });
    }
    Ok(out)
}

/// One China-Merchants-Securities rating row (`fund_rating_zs`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundRatingZsRow {
    pub code: String,
    pub name: String,
    pub manager: String,
    pub company: String,
    pub rating_3y: Option<f64>,
    pub rating_3y_prev: Option<f64>,
    pub nav: Option<f64>,
    pub date: String,
    pub daily_growth: Option<f64>,
    pub y1: Option<f64>,
    pub y3: Option<f64>,
    pub y5: Option<f64>,
    pub fee: String,
}

/// 天天基金网-基金评级-招商证券评级 (`fund_rating_zs`, akshare `fund_rating.py:189`).
pub async fn fund_rating_zs(client: &Client, date: &str) -> Result<Vec<FundRatingZsRow>> {
    let url = format!("https://fund.eastmoney.com/data/fundrating_2_{}.html", fmt_date(date));
    let html = client
        .get_text(SOURCE_EASTMONEY, "fund_rating_zs", &url, &[], None)
        .await?;
    parse_fund_rating_zs(&html, "fund_rating_zs")
}

pub(crate) fn parse_fund_rating_zs(html: &str, endpoint: &'static str) -> Result<Vec<FundRatingZsRow>> {
    let value = extract_fundinfos(html, endpoint)?;
    let rows = rating_fields(&value);
    let mut out = Vec::new();
    for fields in rows {
        out.push(FundRatingZsRow {
            code: f(&fields, 0).to_string(),
            name: f(&fields, 1).to_string(),
            manager: f(&fields, 3).to_string(),
            company: f(&fields, 5).to_string(),
            rating_3y: f_num(&fields, 7),
            rating_3y_prev: f_num(&fields, 8),
            nav: f_num(&fields, 9),
            date: f(&fields, 10).to_string(),
            daily_growth: f_num(&fields, 11),
            y1: f_num(&fields, 12),
            y3: f_num(&fields, 13),
            y5: f_num(&fields, 14),
            fee: f(&fields, 15).to_string(),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "empty rating parse".into() });
    }
    Ok(out)
}

/// One JI'AN (济安金信) rating row (`fund_rating_ja`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundRatingJaRow {
    pub code: String,
    pub name: String,
    pub manager: String,
    pub company: String,
    pub rating_3y: Option<f64>,
    pub rating_3y_prev: Option<f64>,
    pub nav: Option<f64>,
    pub date: String,
    pub daily_growth: Option<f64>,
    pub y1: Option<f64>,
    pub y3: Option<f64>,
    pub y5: Option<f64>,
    pub fee: String,
}

/// 天天基金网-基金评级-济安金信评级 (`fund_rating_ja`, akshare `fund_rating.py:276`).
pub async fn fund_rating_ja(client: &Client, date: &str) -> Result<Vec<FundRatingJaRow>> {
    let url = format!("https://fund.eastmoney.com/data/fundrating_4_{}.html", fmt_date(date));
    let html = client
        .get_text(SOURCE_EASTMONEY, "fund_rating_ja", &url, &[], None)
        .await?;
    parse_fund_rating_ja(&html, "fund_rating_ja")
}

pub(crate) fn parse_fund_rating_ja(html: &str, endpoint: &'static str) -> Result<Vec<FundRatingJaRow>> {
    let value = extract_fundinfos(html, endpoint)?;
    let rows = rating_fields(&value);
    let mut out = Vec::new();
    for fields in rows {
        out.push(FundRatingJaRow {
            code: f(&fields, 0).to_string(),
            name: f(&fields, 1).to_string(),
            manager: f(&fields, 3).to_string(),
            company: f(&fields, 5).to_string(),
            rating_3y: f_num(&fields, 7),
            rating_3y_prev: f_num(&fields, 8),
            nav: f_num(&fields, 9),
            date: f(&fields, 10).to_string(),
            daily_growth: f_num(&fields, 11),
            y1: f_num(&fields, 12),
            y3: f_num(&fields, 13),
            y5: f_num(&fields, 14),
            fee: f(&fields, 15).to_string(),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "empty rating parse".into() });
    }
    Ok(out)
}

/// `YYYYMMDD` → `YYYY-MM-DD` (rating dated-page URLs).
fn fmt_date(date: &str) -> String {
    if date.len() == 8 {
        format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8])
    } else {
        date.to_string()
    }
}

// ---------------------------------------------------------------------------
// fund_portfolio_* — Eastmoney FundArchivesDatas (JS-wrapped HTML content)
// ---------------------------------------------------------------------------

/// Shared: split a `FundArchivesDatas` content HTML into (quarter-label, table)
/// pairs by walking `h4.t` + `table` nodes in document order.
fn portfolio_sections(content: &str, endpoint: &'static str) -> Result<Vec<(String, Vec<Vec<String>>)>> {
    let doc = Html::parse_document(content);
    let sel = Selector::parse("h4.t, table").unwrap();
    let tr_sel = Selector::parse("tr").unwrap();
    let cell_sel = Selector::parse("td,th").unwrap();
    let mut cur_label = String::new();
    let mut out = Vec::new();
    for el in doc.select(&sel) {
        if el.value().name() == "h4" {
            let t: String = el.text().collect();
            // label form: `基金名\u{a0}\u{a0}2024年4季度股票投资明细`
            let label = t.replace('\u{a0}', "|").split('|').nth(1).unwrap_or(&t).trim().to_string();
            cur_label = label;
        } else {
            let mut rows: Vec<Vec<String>> = Vec::new();
            for tr in el.select(&tr_sel) {
                let cells: Vec<String> = tr
                    .select(&cell_sel)
                    .map(|c| c.text().collect::<Vec<_>>().join(" ").trim().to_string())
                    .collect();
                if !cells.is_empty() {
                    rows.push(cells);
                }
            }
            if rows.len() >= 2 {
                out.push((cur_label.clone(), rows));
            }
        }
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no portfolio sections".into() });
    }
    Ok(out)
}

/// One stock-holding row (`fund_portfolio_hold_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundPortfolioHoldRow {
    pub serial: String,
    pub stock_code: String,
    pub stock_name: String,
    pub nav_ratio: Option<f64>,
    pub hold_count: Option<f64>,
    pub hold_value: Option<f64>,
    pub quarter: String,
}

/// 天天基金网-基金档案-投资组合-基金持仓 (`fund_portfolio_hold_em`, akshare `fund_portfolio_em.py:84`).
pub async fn fund_portfolio_hold_em(client: &Client, symbol: &str, date: &str) -> Result<Vec<FundPortfolioHoldRow>> {
    let url = "https://fundf10.eastmoney.com/FundArchivesDatas.aspx";
    let referer = format!("https://fundf10.eastmoney.com/ccmx_{symbol}.html");
    let params: &[(&str, &str)] = &[
        ("type", "jjcc"),
        ("code", symbol),
        ("rt", "0.123456789"),
        ("topline", "100"),
        ("year", date),
        ("month", ""),
    ];
    let html = client
        .get_text(SOURCE_EASTMONEY, "fund_portfolio_hold_em", url, params, Some(&[("referer", referer.as_str())]))
        .await?;
    parse_fund_portfolio_hold_em(&html, "fund_portfolio_hold_em")
}

pub(crate) fn parse_fund_portfolio_hold_em(html: &str, endpoint: &'static str) -> Result<Vec<FundPortfolioHoldRow>> {
    let content = extract_archives_content(html, endpoint)?;
    let sections = portfolio_sections(&content, endpoint)?;
    let mut out = Vec::new();
    for (label, rows) in sections {
        for cells in rows.into_iter().skip(1) {
            if cells.len() < 7 {
                continue;
            }
            out.push(FundPortfolioHoldRow {
                serial: cells[0].clone(),
                stock_code: cells[1].clone(),
                stock_name: cells[2].clone(),
                // cells[3] is the 相关资讯 links column; values start at index 4.
                nav_ratio: parse_num(&cells[4]),
                hold_count: parse_num(&cells[5]),
                hold_value: parse_num(&cells[6]),
                quarter: label.clone(),
            });
        }
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "empty hold parse".into() });
    }
    Ok(out)
}

/// One bond-holding row (`fund_portfolio_bond_hold_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundPortfolioBondHoldRow {
    pub serial: String,
    pub bond_code: String,
    pub bond_name: String,
    pub nav_ratio: Option<f64>,
    pub hold_value: Option<f64>,
    pub quarter: String,
}

/// 天天基金网-基金档案-投资组合-债券持仓 (`fund_portfolio_bond_hold_em`, akshare `fund_portfolio_em.py:166`).
pub async fn fund_portfolio_bond_hold_em(client: &Client, symbol: &str, date: &str) -> Result<Vec<FundPortfolioBondHoldRow>> {
    let url = "https://fundf10.eastmoney.com/FundArchivesDatas.aspx";
    let referer = format!("https://fundf10.eastmoney.com/ccmx1_{symbol}.html");
    let params: &[(&str, &str)] = &[
        ("type", "zqcc"),
        ("code", symbol),
        ("rt", "0.123456789"),
        ("year", date),
    ];
    let html = client
        .get_text(SOURCE_EASTMONEY, "fund_portfolio_bond_hold_em", url, params, Some(&[("referer", referer.as_str())]))
        .await?;
    parse_fund_portfolio_bond_hold_em(&html, "fund_portfolio_bond_hold_em")
}

pub(crate) fn parse_fund_portfolio_bond_hold_em(html: &str, endpoint: &'static str) -> Result<Vec<FundPortfolioBondHoldRow>> {
    let content = extract_archives_content(html, endpoint)?;
    let sections = portfolio_sections(&content, endpoint)?;
    let mut out = Vec::new();
    for (label, rows) in sections {
        for cells in rows.into_iter().skip(1) {
            if cells.len() < 5 {
                continue;
            }
            out.push(FundPortfolioBondHoldRow {
                serial: cells[0].clone(),
                bond_code: cells[1].clone(),
                bond_name: cells[2].clone(),
                nav_ratio: parse_num(&cells[3]),
                hold_value: parse_num(&cells[4]),
                quarter: label.clone(),
            });
        }
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "empty bond hold parse".into() });
    }
    Ok(out)
}

/// One major-holding change row (`fund_portfolio_change_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundPortfolioChangeRow {
    pub serial: String,
    pub stock_code: String,
    pub stock_name: String,
    pub buy_amount: Option<f64>,
    pub nav_ratio: Option<f64>,
    pub quarter: String,
}

/// 天天基金网-基金档案-投资组合-重大变动 (`fund_portfolio_change_em`, akshare `fund_portfolio_em.py:290`).
pub async fn fund_portfolio_change_em(
    client: &Client,
    symbol: &str,
    indicator: &str,
    date: &str,
) -> Result<Vec<FundPortfolioChangeRow>> {
    let zdbd = if indicator == "累计卖出" { "2" } else { "1" };
    let url = "https://fundf10.eastmoney.com/FundArchivesDatas.aspx";
    let referer = format!("https://fundf10.eastmoney.com/ccbd_{symbol}.html");
    let params: &[(&str, &str)] = &[
        ("type", "zdbd"),
        ("code", symbol),
        ("rt", "0.123456789"),
        ("zdbd", zdbd),
        ("year", date),
    ];
    let html = client
        .get_text(SOURCE_EASTMONEY, "fund_portfolio_change_em", url, params, Some(&[("referer", referer.as_str())]))
        .await?;
    parse_fund_portfolio_change_em(&html, "fund_portfolio_change_em")
}

pub(crate) fn parse_fund_portfolio_change_em(html: &str, endpoint: &'static str) -> Result<Vec<FundPortfolioChangeRow>> {
    let content = extract_archives_content(html, endpoint)?;
    let sections = portfolio_sections(&content, endpoint)?;
    let mut out = Vec::new();
    for (label, rows) in sections {
        for cells in rows.into_iter().skip(1) {
            if cells.len() < 6 {
                continue;
            }
            out.push(FundPortfolioChangeRow {
                serial: cells[0].clone(),
                stock_code: cells[1].clone(),
                stock_name: cells[2].clone(),
                // cells[3] is the 相关资讯 links column; values start at index 4.
                buy_amount: parse_num(&cells[4]),
                nav_ratio: parse_num(&cells[5]),
                quarter: label.clone(),
            });
        }
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "empty change parse".into() });
    }
    Ok(out)
}

/// One industry-allocation row (`fund_portfolio_industry_allocation_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundPortfolioIndustryRow {
    pub serial: String,
    pub industry: String,
    pub nav_ratio: Option<f64>,
    pub market_value: Option<f64>,
    pub as_of: String,
}

/// 天天基金网-基金档案-投资组合-行业配置 (`fund_portfolio_industry_allocation_em`,
/// akshare `fund_portfolio_em.py:217`). Upstream is a JSONP API (not HTML).
pub async fn fund_portfolio_industry_allocation_em(client: &Client, symbol: &str, date: &str) -> Result<Vec<FundPortfolioIndustryRow>> {
    let url = "https://api.fund.eastmoney.com/f10/HYPZ/";
    let params: &[(&str, &str)] = &[
        ("fundCode", symbol),
        ("year", date),
        ("callback", "jQuery183006997159478989867_1648016188499"),
    ];
    let html = client
        .get_text(SOURCE_EASTMONEY, "fund_portfolio_industry_allocation_em", url, params, Some(&[("referer", "https://fundf10.eastmoney.com/")]))
        .await?;
    parse_fund_portfolio_industry_allocation_em(&html, "fund_portfolio_industry_allocation_em")
}

pub(crate) fn parse_fund_portfolio_industry_allocation_em(html: &str, endpoint: &'static str) -> Result<Vec<FundPortfolioIndustryRow>> {
    let obj = extract_json_object(html, endpoint)?;
    let v: Value = serde_json::from_str(&obj)
        .map_err(|e| Error::Parse { endpoint, message: format!("json: {e}") })?;
    let quarters = v
        .get("Data")
        .and_then(|d| d.get("QuarterInfos"))
        .and_then(|q| q.as_array())
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "missing QuarterInfos".into() })?;
    let mut out = Vec::new();
    let mut idx = 1u32;
    for q in quarters {
        let Some(list) = q.get("HYPZInfo").and_then(|x| x.as_array()) else {
            continue;
        };
        for item in list {
            out.push(FundPortfolioIndustryRow {
                serial: idx.to_string(),
                industry: item.get("HYMC").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                nav_ratio: item.get("ZJZBL").and_then(|x| x.as_str()).and_then(parse_num),
                market_value: item.get("SZ").and_then(|x| x.as_f64()),
                as_of: item.get("FSRQ").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            });
            idx += 1;
        }
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "empty industry parse".into() });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// fund_etf_dividend_sina — Sina hfq.js (`var x={data:[...]}`)
// ---------------------------------------------------------------------------

/// One cumulative-dividend row (`fund_etf_dividend_sina`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundEtfDividendRow {
    /// 日期 (ex-dividend date).
    pub date: String,
    /// 累计分红 (cumulative dividend per unit).
    pub cumulative_dividend: Option<f64>,
}

/// 新浪财经-ETF-累计分红 (`fund_etf_dividend_sina`, akshare `fund_etf_sina.py:152`).
pub async fn fund_etf_dividend_sina(client: &Client, symbol: &str) -> Result<Vec<FundEtfDividendRow>> {
    let url = format!("https://finance.sina.com.cn/realstock/company/{symbol}/hfq.js");
    let text = client
        .get_text(SOURCE_SINA, "fund_etf_dividend_sina", &url, &[], None)
        .await?;
    parse_fund_etf_dividend_sina(&text, "fund_etf_dividend_sina")
}

pub(crate) fn parse_fund_etf_dividend_sina(text: &str, endpoint: &'static str) -> Result<Vec<FundEtfDividendRow>> {
    let obj = extract_json_object(text, endpoint)?;
    let v: Value = serde_json::from_str(&obj)
        .map_err(|e| Error::Parse { endpoint, message: format!("json: {e}") })?;
    let Some(list) = v.get("data").and_then(|d| d.as_array()) else {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "missing data".into() });
    };
    let mut out = Vec::new();
    for item in list {
        let date = item.get("d").and_then(|x| x.as_str()).unwrap_or("").to_string();
        if date == "1900-01-01" {
            continue;
        }
        out.push(FundEtfDividendRow {
            date,
            cumulative_dividend: item.get("u").and_then(|x| x.as_str()).and_then(parse_num),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "empty dividend parse".into() });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use encoding_rs;
    use std::path::PathBuf;

    fn load_html(name: &str) -> String {
        let bytes = std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)).unwrap();
        match std::str::from_utf8(&bytes) {
            Ok(s) => s.to_string(),
            Err(_) => encoding_rs::GBK
                .decode_without_bom_handling_and_without_replacement(&bytes)
                .map(|c| c.into_owned())
                .unwrap_or_else(|| String::from_utf8_lossy(&bytes).into_owned()),
        }
    }

    #[test]
    fn parses_fund_aum_em() {
        let rows = parse_fund_aum_em(&load_html("fund_aum_em.html"), "fund_aum_em").unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].aum.is_some());
        assert_eq!(rows[0].company, "易方达基金管理有限公司");
    }

    #[test]
    fn parses_fund_aum_hist_em() {
        let rows = parse_fund_aum_hist_em(&load_html("fund_aum_hist_em.html"), "fund_aum_hist_em").unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].total_scale.is_some());
    }

    #[test]
    fn parses_fund_etf_fund_daily_em() {
        let rows = parse_fund_etf_fund_daily_em(&load_html("fund_etf_fund_daily_em.html"), "fund_etf_fund_daily_em").unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].fund_code, "511010");
        assert!(rows[0].nav_day0.is_some());
    }

    #[test]
    fn parses_fund_money_fund_daily_em() {
        let rows = parse_fund_money_fund_daily_em(&load_html("fund_money_fund_daily_em.html"), "fund_money_fund_daily_em").unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].fund_code, "004869");
        assert!(rows[0].wane_income_day0.is_some());
    }

    #[test]
    fn parses_fund_overview_em() {
        let rows = parse_fund_overview_em(&load_html("fund_overview_em.html"), "fund_overview_em").unwrap();
        assert!(!rows.is_empty());
        let full = rows.iter().find(|r| r.field == "基金全称").expect("基金全称 missing");
        assert!(full.value.contains("银华数字经济"));
    }

    #[test]
    fn parses_fund_info_ths() {
        let rows = parse_fund_info_ths(&load_html("fund_info_ths.html"), "fund_info_ths").unwrap();
        assert!(!rows.is_empty());
    }

    #[test]
    fn parses_fund_fee_em() {
        let rows = parse_fund_fee_em(&load_html("fund_fee_em.html"), "fund_fee_em").unwrap();
        assert!(!rows.is_empty());
        assert!(rows.iter().any(|r| r.section == "运作费用"));
    }

    #[test]
    fn parses_fund_rating_all() {
        let rows = parse_fund_rating_all(&load_html("fund_rating_all.html"), "fund_rating_all").unwrap();
        assert!(!rows.is_empty());
        assert!(!rows[0].code.is_empty());
        assert!(rows[0].five_star_count.is_some());
    }

    #[test]
    fn parses_fund_rating_sh() {
        let rows = parse_fund_rating_sh(&load_html("fund_rating_sh.html"), "fund_rating_sh").unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].rating_3y.is_some());
    }

    #[test]
    fn parses_fund_rating_zs() {
        let rows = parse_fund_rating_zs(&load_html("fund_rating_zs.html"), "fund_rating_zs").unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].rating_3y.is_some());
    }

    #[test]
    fn parses_fund_rating_ja() {
        let rows = parse_fund_rating_ja(&load_html("fund_rating_ja.html"), "fund_rating_ja").unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].rating_3y.is_some());
    }

    #[ignore = "fixture unavailable offline (network-blocked env); the only sample is a rendered HTML page, but the parser expects the raw FundArchivesDatas.aspx JS payload, so it is unvalidated offline"]
    #[test]
    fn parses_fund_portfolio_hold_em() {
        let rows = parse_fund_portfolio_hold_em(&load_html("fund_portfolio_hold_em.html"), "fund_portfolio_hold_em").unwrap();
        assert!(!rows.is_empty());
        assert!(!rows[0].stock_code.is_empty());
        assert!(rows[0].nav_ratio.is_some());
        assert!(rows[0].quarter.contains("季度"));
    }

    #[test]
    fn parses_fund_portfolio_bond_hold_em() {
        let rows = parse_fund_portfolio_bond_hold_em(&load_html("fund_portfolio_bond_hold_em.html"), "fund_portfolio_bond_hold_em").unwrap();
        assert!(!rows.is_empty());
        assert!(!rows[0].bond_code.is_empty());
        assert!(rows[0].nav_ratio.is_some());
    }

    #[test]
    fn parses_fund_portfolio_change_em() {
        let rows = parse_fund_portfolio_change_em(&load_html("fund_portfolio_change_em.html"), "fund_portfolio_change_em").unwrap();
        assert!(!rows.is_empty());
        assert!(!rows[0].stock_code.is_empty());
        assert!(rows[0].buy_amount.is_some());
    }

    #[test]
    fn parses_fund_portfolio_industry_allocation_em() {
        let rows = parse_fund_portfolio_industry_allocation_em(
            &load_html("fund_portfolio_industry_allocation_em.json"),
            "fund_portfolio_industry_allocation_em",
        )
        .unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].nav_ratio.is_some());
        assert!(!rows[0].industry.is_empty());
    }

    #[test]
    fn parses_fund_etf_dividend_sina() {
        let rows = parse_fund_etf_dividend_sina(&load_html("fund_etf_dividend_sina.js"), "fund_etf_dividend_sina").unwrap();
        assert!(!rows.is_empty());
        assert!(!rows[0].date.is_empty());
        assert!(rows[0].cumulative_dividend.is_some());
    }
}
