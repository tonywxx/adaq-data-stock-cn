//! `futures_derivative` HTML-scraping gap fillers.
//!
//! Ports akshare `futures_derivative` functions whose upstreams return HTML
//! tables (Sina futures COT; 生意社 spot-vs-futures). Each function follows the
//! established pattern: a public `async fn` that performs the network fetch and
//! a `pub(crate)` `parse_*` that turns the captured body into rows.
//!
//! Sources / akshare references:
//! * [`futures_hold_pos_sina`] — `futures_derivative/futures_cot_sina.py:15`
//! * [`futures_spot_sys`] — `futures_derivative/futures_spot_sys.py:36`

use scraper::{Html, Selector};

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_SINA: &str = "sina";
const SOURCE_100PPI: &str = "100ppi";

/// Parse a numeric cell, tolerating thousands separators and `--`/empty.
fn as_opt_f64(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', "");
    if t.is_empty() || t == "--" {
        return None;
    }
    t.parse::<f64>().ok()
}

/// Return every `<table>` on the page as `table -> row -> cell` strings (header
/// row included, cells trimmed). Delegates to the shared walker in `core::html`.
fn extract_tables(html: &str, endpoint: &'static str) -> Result<Vec<Vec<Vec<String>>>> {
    crate::core::html::tables(html, endpoint)
}

// ===========================================================================
// Sina futures 成交持仓 (hold position) — `pd.read_html` tables 2/3/4
// ===========================================================================

/// One broker/rank row of Sina futures hold positions.
///
/// `value` is the chosen metric (`成交量` / `多单持仓` / `空单持仓`), selected by
/// the `symbol` argument of [`futures_hold_pos_sina`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesHoldPosRow {
    /// 名次 (rank).
    pub rank: Option<f64>,
    /// 成交量 / 多单持仓 / 空单持仓 (whichever `symbol` was requested).
    pub value: Option<f64>,
    /// 比上交易增减 (change vs previous session).
    pub change: Option<f64>,
}

/// Parse a Sina hold-position table for the requested `metric` (`成交量` /
/// `多单持仓` / `空单持仓`). Tables are selected by header text (robust to the
/// differences between `scraper`'s raw `<table>` enumeration and akshare's
/// `pd.read_html` index). Mirrors akshare `pd.read_html(...)[k].iloc[:-1, :]`:
/// skip the header row, drop a trailing `合计` total row.
pub(crate) fn parse_futures_hold_pos(
    html: &str,
    metric: &str,
    endpoint: &'static str,
) -> Result<Vec<FuturesHoldPosRow>> {
    let tables = extract_tables(html, endpoint)?;
    let chosen = tables
        .iter()
        .find(|rows| {
            // The clean per-metric table has exactly 4 header cells
            // [名次, 会员简称, <metric>, 比上交易增减]; the Sina page also wraps
            // everything in an outer container table whose header row carries
            // the same metric label among many cells, so require the 4-cell shape.
            rows.first()
                .map_or(false, |h| h.len() == 4 && h.get(2).map_or(false, |c| c.trim() == metric))
        })
        .ok_or_else(|| Error::UpstreamChanged {
            origin: endpoint,
            message: format!("no table with header '{metric}'"),
        })?;
    let out: Vec<FuturesHoldPosRow> = chosen
        .iter()
        .skip(1) // header
        .filter(|r| r.len() >= 3 && !r[0].contains("合计"))
        .map(|r| FuturesHoldPosRow {
            rank: as_opt_f64(&r[0]),
            value: as_opt_f64(&r[2]),
            change: as_opt_f64(&r[3]),
        })
        .collect();
    Ok(out)
}

/// 新浪财经-期货-成交持仓 (`futures_hold_pos_sina`, akshare `futures_cot_sina.py:15`).
///
/// `symbol` selects the metric table: `成交量` / `多单持仓` / `空单持仓`.
pub async fn futures_hold_pos_sina(
    client: &Client,
    symbol: &str,
    contract: &str,
    date: &str,
) -> Result<Vec<FuturesHoldPosRow>> {
    let endpoint = "futures_hold_pos_sina";
    let metric = match symbol {
        "成交量" => "成交量",
        "多单持仓" => "多单持仓",
        "空单持仓" => "空单持仓",
        _ => {
            return Err(Error::InvalidParam(format!(
                "{endpoint}: symbol must be one of 成交量/多单持仓/空单持仓"
            )))
        }
    };
    let trade_date = if date.len() == 8 {
        format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..])
    } else {
        date.to_string()
    };
    let url = "https://vip.stock.finance.sina.com.cn/q/view/vFutures_Positions_cjcc.php";
    let html = client
        .get_text(
            SOURCE_SINA,
            endpoint,
            url,
            &[("t_breed", contract), ("t_date", &trade_date)],
            None,
        )
        .await?;
    parse_futures_hold_pos(&html, metric, endpoint)
}

// ===========================================================================
// 生意社 spot-vs-futures (现期图) — name→url dict, then transposed tables
// ===========================================================================

/// One date observation of 生意社 spot-vs-futures metrics.
///
/// Only the columns for the requested `indicator` are populated; the others
/// stay `None`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesSpotSysRow {
    /// 日期.
    pub date: String,
    /// 现货价格 (市场价格).
    pub spot_price: Option<f64>,
    /// 主力合约 (市场价格).
    pub main_contract: Option<f64>,
    /// 最近合约 (市场价格).
    pub near_contract: Option<f64>,
    /// 基差率 (基差率).
    pub basis_rate: Option<f64>,
    /// 主力基差 (主力基差).
    pub main_basis: Option<f64>,
}

/// Parse the 生意社 name→url dictionary from the `div.q8` listing page.
pub(crate) fn parse_sys_dict(html: &str, endpoint: &'static str) -> Result<std::collections::HashMap<String, String>> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("div.q8 li a")
        .map_err(|e| Error::Parse { endpoint, message: format!("q8 selector: {e}") })?;
    let mut map = std::collections::HashMap::new();
    for a in doc.select(&sel) {
        let name = a.text().collect::<String>().trim().to_string();
        if let Some(href) = a.value().attr("href") {
            map.insert(name, href.to_string());
        }
    }
    if map.is_empty() {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: "no 生意社 variety links found".into(),
        });
    }
    Ok(map)
}

/// Parse a transposed 生意社 table (akshare `pd.read_html(..., index_col=0)[k].T`).
///
/// The source table has the metric names in column 0 (`日期` / `现货价格` / ...)
/// and the dates across the header row; this rebuilds one row per date.
pub(crate) fn parse_futures_spot_sys(
    html: &str,
    table_index: usize,
    endpoint: &'static str,
) -> Result<Vec<FuturesSpotSysRow>> {
    let tables = extract_tables(html, endpoint)?;
    let rows = tables.get(table_index).ok_or_else(|| Error::UpstreamChanged {
        origin: endpoint,
        message: format!("table index {table_index} not found"),
    })?;
    if rows.len() < 2 {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: "spot-sys table too small".into(),
        });
    }
    let header = &rows[0]; // dates (col 0 is the label "日期")
    let date_labels: Vec<&str> = header.iter().skip(1).map(|s| s.as_str()).collect();
    let mut out: Vec<FuturesSpotSysRow> = date_labels
        .iter()
        .map(|d| FuturesSpotSysRow {
            date: (*d).to_string(),
            spot_price: None,
            main_contract: None,
            near_contract: None,
            basis_rate: None,
            main_basis: None,
        })
        .collect();
    for row in &rows[1..] {
        let label = row.first().map(|s| s.trim()).unwrap_or_default();
        for (j, cell) in row.iter().skip(1).enumerate() {
            let v = as_opt_f64(cell);
            let target = match label {
                "现货价格" => &mut out[j].spot_price,
                "主力合约" => &mut out[j].main_contract,
                "最近合约" => &mut out[j].near_contract,
                "基差率" => &mut out[j].basis_rate,
                "主力基差" => &mut out[j].main_basis,
                _ => continue,
            };
            *target = v;
        }
    }
    Ok(out)
}

/// 生意社-商品与期货-现期图 (`futures_spot_sys`, akshare `futures_spot_sys.py:36`).
///
/// `indicator` selects the table: `市场价格` → table 1, `基差率` → table 2,
/// `主力基差` → table 3.
pub async fn futures_spot_sys(
    client: &Client,
    symbol: &str,
    indicator: &str,
) -> Result<Vec<FuturesSpotSysRow>> {
    let endpoint = "futures_spot_sys";
    let table_index = match indicator {
        "市场价格" => 1,
        "基差率" => 2,
        "主力基差" => 3,
        _ => {
            return Err(Error::InvalidParam(format!(
                "{endpoint}: indicator must be one of 市场价格/基差率/主力基差"
            )))
        }
    };
    let dict_html = client
        .get_text(
            SOURCE_100PPI,
            endpoint,
            "https://www.100ppi.com/sf/792.html",
            &[],
            None,
        )
        .await?;
    let dict = parse_sys_dict(&dict_html, endpoint)?;
    let href = dict.get(symbol).ok_or_else(|| Error::NotFound {
        endpoint,
        message: format!("variety '{symbol}' not found in 生意社 dictionary"),
    })?;
    let url = format!("https://www.100ppi.com{href}");
    let html = client.get_text(SOURCE_100PPI, endpoint, &url, &[], None).await?;
    parse_futures_spot_sys(&html, table_index, endpoint)
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Load a UTF-8 fixture as text.
    fn load_html(name: &str) -> String {
        let bytes = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(name),
        )
        .unwrap();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// Load a GBK-encoded fixture, decoding to UTF-8 (Sina pages are GBK).
    fn load_gbk(name: &str) -> String {
        let bytes = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(name),
        )
        .unwrap();
        encoding_rs::GBK
            .decode_without_bom_handling_and_without_replacement(&bytes)
            .map(std::borrow::Cow::into_owned)
            .unwrap_or_else(|| String::from_utf8_lossy(&bytes).into_owned())
    }

    #[test]
    fn parses_futures_hold_pos_sina() {
        // Inline synthetic table (UTF-8) with 4 data rows + a 合计 total row.
        let html = r#"<html><body>
<table><tr><th>名次</th><th>会员简称</th><th>成交量</th><th>比上交易增减</th></tr>
<tr><td>1</td><td>中信期货</td><td>12,345</td><td>-123</td></tr>
<tr><td>2</td><td>国泰君安</td><td>10,000</td><td>50</td></tr>
<tr><td>3</td><td>华泰期货</td><td>8,800</td><td>-40</td></tr>
<tr><td>4</td><td>银河期货</td><td>7,200</td><td>12</td></tr>
<tr><td>合计</td><td></td><td>38,345</td><td>-101</td></tr></table></body></html>"#;
        let rows = parse_futures_hold_pos(html, "成交量", "futures_hold_pos_sina").unwrap();
        assert_eq!(rows.len(), 4, "expected 4 data rows (合计 dropped)");
        assert_eq!(rows[0].rank, Some(1.0));
        assert!((rows[0].value.unwrap() - 12345.0).abs() < 1e-9);
        assert!((rows[0].change.unwrap() - (-123.0)).abs() < 1e-9);
        // 多单持仓 table on the same page is independent.
        let html2 = r#"<html><body>
<table><tr><th>名次</th><th>会员简称</th><th>多单持仓</th><th>比上交易增减</th></tr>
<tr><td>1</td><td>中信期货</td><td>20,000</td><td>300</td></tr>
<tr><td>合计</td><td></td><td>56,000</td><td>230</td></tr></table></body></html>"#;
        let rows2 = parse_futures_hold_pos(html2, "多单持仓", "futures_hold_pos_sina").unwrap();
        assert_eq!(rows2.len(), 1);
        assert!((rows2[0].value.unwrap() - 20000.0).abs() < 1e-9);
    }

    #[test]
    fn parses_futures_hold_pos_sina_real_fixture() {
        // The committed GBK fixture has header + 合计 only (no data rows); this
        // proves table selection + 合计-drop without panicking.
        let html = load_gbk("futures_hold_pos_sina.html");
        let rows = parse_futures_hold_pos(&html, "成交量", "futures_hold_pos_sina").unwrap();
        assert!(rows.is_empty(), "committed fixture has no data rows");
        // 空单持仓 table is also selectable.
        let rows3 = parse_futures_hold_pos(&html, "空单持仓", "futures_hold_pos_sina").unwrap();
        assert!(rows3.is_empty());
    }

    #[test]
    fn parses_futures_spot_sys_market() {
        // 市场价格: table index 1 — date x (现货价格, 主力合约, 最近合约).
        let rows = parse_futures_spot_sys(&load_html("futures_spot_sys.html"), 1, "futures_spot_sys")
            .unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].date, "2024-01-02");
        assert!((rows[0].spot_price.unwrap() - 68230.0).abs() < 1e-9);
        assert!((rows[0].main_contract.unwrap() - 68400.0).abs() < 1e-9);
        assert!((rows[0].near_contract.unwrap() - 68100.0).abs() < 1e-9);
    }

    #[test]
    fn parses_futures_spot_sys_basis_rate() {
        let rows = parse_futures_spot_sys(&load_html("futures_spot_sys.html"), 2, "futures_spot_sys")
            .unwrap();
        assert_eq!(rows.len(), 3);
        assert!((rows[0].basis_rate.unwrap() - 1.20).abs() < 1e-9);
        assert!((rows[2].basis_rate.unwrap() - 1.23).abs() < 1e-9);
    }

    #[test]
    fn parses_futures_spot_sys_main_basis() {
        let rows = parse_futures_spot_sys(&load_html("futures_spot_sys.html"), 3, "futures_spot_sys")
            .unwrap();
        assert_eq!(rows.len(), 3);
        assert!((rows[0].main_basis.unwrap() - (-180.0)).abs() < 1e-9);
        assert!((rows[2].main_basis.unwrap() - (-150.0)).abs() < 1e-9);
    }

    #[test]
    fn parses_sys_dict() {
        let dict = parse_sys_dict(&load_html("futures_spot_sys_dict.html"), "futures_spot_sys").unwrap();
        assert_eq!(dict.get("铜").map(String::as_str), Some("/sf/tong.html"));
        assert_eq!(dict.get("铝").map(String::as_str), Some("/sf/lv.html"));
    }
}
