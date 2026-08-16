//! Bond-market HTML-table endpoints ported from `akshare/bond/*`.
//!
//! These mirror akshare functions that fetch an HTML page and scrape a
//! `<table>` with `pd.read_html`:
//!
//! * [`bond_cb_profile_sina`] — `bond/bond_cb_sina.py:15` (Sina convertible-bond detail).
//! * [`bond_cb_summary_sina`] — `bond/bond_cb_sina.py:31` (Sina convertible-bond summary).
//! * [`bond_cb_adj_logs_jsl`] — `bond/bond_convert.py:297` (Jisilu conversion-price adjust log).
//! * [`bond_china_yield`] — `bond/bond_china.py:142` (ChinaBond yield curve).
//!
//! Sina pages are `gbk`-encoded; the [`load_html`] test helper decodes them.

use scraper::{Html, Selector};

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// Parse a numeric cell into `f64`, tolerating string-encoded numbers.
#[allow(dead_code)]
fn as_f64(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', "");
    t.parse::<f64>().ok()
}

/// Extract every `<table>` from an HTML document as a list of rows, each row a
/// list of cell strings (text content of every `<td>`/`<th>`). Mirrors
/// `pd.read_html`, which returns one frame per `<table>`.
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

// ---------------------------------------------------------------------------
// Sina convertible-bond detail (`bond_cb_profile_sina`)
// ---------------------------------------------------------------------------

/// One key/value attribute of a convertible bond (Sina detail page).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondCbProfileSina {
    /// Attribute name (akshare `item`).
    pub item: String,
    /// Attribute value (akshare `value`).
    pub value: String,
}

/// 新浪财经-债券-可转债-详情资料 (`bond_cb_profile_sina`, akshare `bond_cb_sina.py:15`).
pub async fn bond_cb_profile_sina(client: &Client, symbol: &str) -> Result<Vec<BondCbProfileSina>> {
    let url = format!("https://money.finance.sina.com.cn/bond/info/{symbol}.html");
    let html = client
        .get_text("sina", "bond_cb_profile_sina", &url, &[], None)
        .await?;
    parse_bond_cb_profile_sina(&html, "bond_cb_profile_sina")
}

pub(crate) fn parse_bond_cb_profile_sina(html: &str, endpoint: &'static str) -> Result<Vec<BondCbProfileSina>> {
    let tables = extract_tables(html, endpoint)?;
    // `pd.read_html` treats the first row as the header; akshare then relabels
    // the columns `item`/`value`, so data starts at row 1.
    let rows = tables
        .into_iter()
        .next()
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "missing table".into() })?;
    let mut out = Vec::new();
    for cells in rows.into_iter().skip(1) {
        if cells.len() < 2 {
            continue;
        }
        out.push(BondCbProfileSina {
            item: cells[0].clone(),
            value: cells[1].clone(),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no profile rows".into() });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Sina convertible-bond summary (`bond_cb_summary_sina`)
// ---------------------------------------------------------------------------

/// One key/value attribute of a convertible bond (Sina summary page). The
/// source `<table>` is 6 columns wide; akshare slices it into three
/// `(item, value)` pairs and concatenates them.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondCbSummarySina {
    /// Attribute name (akshare `item`).
    pub item: String,
    /// Attribute value (akshare `value`).
    pub value: String,
}

/// 新浪财经-债券-可转债-债券概况 (`bond_cb_summary_sina`, akshare `bond_cb_sina.py:31`).
pub async fn bond_cb_summary_sina(client: &Client, symbol: &str) -> Result<Vec<BondCbSummarySina>> {
    let url = format!("https://money.finance.sina.com.cn/bond/quotes/{symbol}.html");
    let html = client
        .get_text("sina", "bond_cb_summary_sina", &url, &[], None)
        .await?;
    parse_bond_cb_summary_sina(&html, "bond_cb_summary_sina")
}

pub(crate) fn parse_bond_cb_summary_sina(html: &str, endpoint: &'static str) -> Result<Vec<BondCbSummarySina>> {
    let tables = extract_tables(html, endpoint)?;
    // akshare uses `pd.read_html(...)[10]` (the 11th table). Nested tables shift
    // the index vs pandas, so select by the known 6-column header (`时间`) instead.
    let rows = tables
        .iter()
        .find(|r| r.first().map_or(false, |c| c.len() == 6 && c[0] == "时间"))
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "summary table not found".into() })?;
    let mut out = Vec::new();
    for cells in rows.iter().skip(1) {
        if cells.len() < 6 {
            continue;
        }
        // Three (item, value) pairs: cols [0,1], [2,3], [4,5].
        for pair in cells.chunks(2).take(3) {
            out.push(BondCbSummarySina {
                item: pair[0].clone(),
                value: pair[1].clone(),
            });
        }
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no summary rows".into() });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Jisilu conversion-price adjust log (`bond_cb_adj_logs_jsl`)
// ---------------------------------------------------------------------------

/// One conversion-price adjustment record (Jisilu `adj_logs`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondCbAdjLogJsl {
    /// Bond name (akshare `转债名称`).
    pub bond_name: String,
    /// Shareholders' meeting date (akshare `股东大会日`).
    pub meeting_date: String,
    /// Conversion price before the adjustment (akshare `下修前转股价`).
    pub price_before: Option<f64>,
    /// Conversion price after the adjustment (akshare `下修后转股价`).
    pub price_after: Option<f64>,
    /// Effective date of the new conversion price (akshare `新转股价生效日期`).
    pub effective_date: String,
    /// Price floor for the adjustment (akshare `下修底价`).
    pub floor_price: Option<f64>,
}

/// 集思录-可转债转股价-调整记录 (`bond_cb_adj_logs_jsl`, akshare `bond_convert.py:297`).
pub async fn bond_cb_adj_logs_jsl(client: &Client, symbol: &str) -> Result<Vec<BondCbAdjLogJsl>> {
    let url = format!("https://www.jisilu.cn/data/cbnew/adj_logs/?bond_id={symbol}");
    let html = client
        .get_text("jisilu", "bond_cb_adj_logs_jsl", &url, &[], None)
        .await?;
    parse_bond_cb_adj_logs_jsl(&html, "bond_cb_adj_logs_jsl")
}

pub(crate) fn parse_bond_cb_adj_logs_jsl(html: &str, endpoint: &'static str) -> Result<Vec<BondCbAdjLogJsl>> {
    // Upstream returns plain text ('暂无数据') or a JSON error when there are no
    // adjustment records — akshare returns an empty DataFrame in that case.
    if !html.contains("</table>") {
        return Ok(Vec::new());
    }
    let tables = extract_tables(html, endpoint)?;
    let rows = tables
        .into_iter()
        .next()
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "missing table".into() })?;
    let mut out = Vec::new();
    for cells in rows.into_iter().skip(1) {
        if cells.len() < 6 {
            continue;
        }
        out.push(BondCbAdjLogJsl {
            bond_name: cells[0].clone(),
            meeting_date: cells[1].clone(),
            price_before: as_f64(&cells[2]),
            price_after: as_f64(&cells[3]),
            effective_date: cells[4].clone(),
            floor_price: as_f64(&cells[5]),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// ChinaBond yield curve (`bond_china_yield`)
// ---------------------------------------------------------------------------

/// One yield-curve observation (ChinaBond `historyQuery`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondChinaYield {
    /// Curve name (akshare `曲线名称`).
    pub curve_name: String,
    /// Observation date `YYYY-MM-DD` (akshare `日期`).
    pub date: String,
    /// 3-month yield (akshare `3月`).
    pub m3: Option<f64>,
    /// 6-month yield (akshare `6月`).
    pub m6: Option<f64>,
    /// 1-year yield (akshare `1年`).
    pub y1: Option<f64>,
    /// 3-year yield (akshare `3年`).
    pub y3: Option<f64>,
    /// 5-year yield (akshare `5年`).
    pub y5: Option<f64>,
    /// 7-year yield (akshare `7年`).
    pub y7: Option<f64>,
    /// 10-year yield (akshare `10年`).
    pub y10: Option<f64>,
    /// 30-year yield (akshare `30年`).
    pub y30: Option<f64>,
}

/// 中国债券信息网-国债及其他债券收益率曲线 (`bond_china_yield`, akshare `bond_china.py:142`).
pub async fn bond_china_yield(
    client: &Client,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<BondChinaYield>> {
    let fmt = |d: &str| -> String { format!("{}-{}-{}", &d[0..4], &d[4..6], &d[6..8]) };
    let url = "https://yield.chinabond.com.cn/cbweb-pbc-web/pbc/historyQuery";
    let params: &[(&str, &str)] = &[
        ("startDate", &fmt(start_date)),
        ("endDate", &fmt(end_date)),
        ("gjqx", "0"),
        ("qxId", "ycqx"),
        ("locale", "cn_ZH"),
    ];
    let html = client
        .get_text("chinabond", "bond_china_yield", url, params, None)
        .await?;
    parse_bond_china_yield(&html, "bond_china_yield")
}

pub(crate) fn parse_bond_china_yield(html: &str, endpoint: &'static str) -> Result<Vec<BondChinaYield>> {
    let tables = extract_tables(html, endpoint)?;
    // akshare uses `pd.read_html(..., header=0)[1]` (the 2nd table).
    if tables.len() < 2 {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: format!("expected >=2 tables, found {}", tables.len()),
        });
    }
    let rows = &tables[1];
    let mut out = Vec::new();
    for cells in rows.iter().skip(1) {
        if cells.len() < 10 {
            continue;
        }
        out.push(BondChinaYield {
            curve_name: cells[0].clone(),
            date: cells[1].clone(),
            m3: as_f64(&cells[2]),
            m6: as_f64(&cells[3]),
            y1: as_f64(&cells[4]),
            y3: as_f64(&cells[5]),
            y5: as_f64(&cells[6]),
            y7: as_f64(&cells[7]),
            y10: as_f64(&cells[8]),
            y30: as_f64(&cells[9]),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no yield rows".into() });
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

    fn load_html(name: &str) -> String {
        let bytes = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(name),
        )
        .unwrap();
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

    #[test]
    fn parses_bond_cb_profile_sina() {
        let rows = parse_bond_cb_profile_sina(&load_html("bond_cb_profile_sina.html"), "bond_cb_profile_sina").unwrap();
        assert!(!rows.is_empty());
        // The bond code appears as a value row.
        assert!(rows.iter().any(|r| r.value == "sz128039"));
    }

    #[ignore = "fixture unavailable offline (network-blocked env); the only sample is a mismatched Sina CB page, so this parser is unvalidated offline"]
    #[test]
    fn parses_bond_cb_summary_sina() {
        let rows = parse_bond_cb_summary_sina(&load_html("bond_cb_summary_sina.html"), "bond_cb_summary_sina").unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows.len() % 3, 0);
        // The first data row is the fundraising-total item.
        assert!(rows.iter().any(|r| r.item.contains("募集资金总额")));
    }

    #[test]
    fn parses_bond_cb_adj_logs_jsl() {
        let rows = parse_bond_cb_adj_logs_jsl(&load_html("bond_cb_adj_logs_jsl.html"), "bond_cb_adj_logs_jsl").unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].bond_name, "洪涛转债");
        assert_eq!(rows[0].meeting_date, "2021-02-23");
        assert!((rows[0].price_before.unwrap() - 3.1).abs() < 1e-9);
        assert!((rows[0].price_after.unwrap() - 2.32).abs() < 1e-9);
    }

    #[test]
    fn parses_bond_china_yield() {
        let rows = parse_bond_china_yield(&load_html("bond_china_yield.html"), "bond_china_yield").unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].date, "2021-01-22");
        assert!((rows[0].y10.unwrap() - 3.1185).abs() < 1e-9);
    }
}
