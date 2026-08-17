//! `index` gap fillers — HTML / embedded-JSON / Excel endpoints.
//!
//! Ports seven akshare `index` functions:
//!
//! * [`drewry_wci_index`] — akshare `index/index_drewry.py:17` (infogram.com
//!   embedded `window.infographicData` JSON).
//! * [`index_stock_info`] — akshare `index/index_cons.py:70` (joinquant
//!   `pd.read_html`; **geo-blocked** from non-CN IPs — see report).
//! * [`sw_index_first_info`] / [`sw_index_second_info`] / [`sw_index_third_info`]
//!   — akshare `index/index_sw.py:38/96/158` (legulegu.com Shenwan level 1/2/3).
//! * [`sw_index_third_cons`] — akshare `index/index_sw.py:220` (legulegu.com
//!   Shenwan level-3 constituents table).
//! * [`index_detail_hist_adjust_cni`] — akshare `index/index_cni.py:191`
//!   (**calamine** Excel; the live CNI download returns "文件不存在" for every
//!   symbol — see report).

use scraper::{Html, Selector};
use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_INFOGRAM: &str = "infogram";
const SOURCE_JOINQUANT: &str = "joinquant";
const SOURCE_LEGULEGU: &str = "legulegu";
const SOURCE_CNI: &str = "cnindex";

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

/// Extract every `<table>` from an HTML document as `table → rows → cells`.
fn extract_tables(html: &str, endpoint: &'static str) -> Result<Vec<Vec<Vec<String>>>> {
    crate::core::html::tables(html, endpoint)
}

/// Get cell `i` of a row, trimmed (empty when missing).
fn cell(row: &[String], i: usize) -> String {
    row.get(i).cloned().unwrap_or_default().trim().to_string()
}

/// Parse a cell to `Option<f64>`, tolerating `%`, `,`, `—`, `-`, empty.
fn f64opt(s: &str) -> Option<f64> {
    let t = s.trim().trim_end_matches('%').trim();
    if t.is_empty() || t == "-" || t == "—" {
        None
    } else {
        t.replace(',', "").parse::<f64>().ok()
    }
}

// ---------------------------------------------------------------------------
// Drewry World Container Index (infogram embedded JSON)
// ---------------------------------------------------------------------------

/// One Drewry WCI observation (date, index value).
#[derive(Debug, Clone, serde::Serialize)]
pub struct DrewryWciRow {
    /// Observation date (akshare `date`).
    pub date: String,
    /// WCI value (akshare `wci`).
    pub wci: Option<f64>,
}

/// Extract the `window.infographicData = {...}` JSON object from an infogram page.
fn extract_infographic_json(html: &str) -> Option<String> {
    let marker = "window.infographicData=";
    let start = html.find(marker)? + marker.len();
    let rest = &html[start..];
    let end = rest.find("</script>")?;
    let chunk = &rest[..end];
    let last = chunk.rfind('}')?;
    Some(chunk[..last + 1].to_string())
}

/// Parse the Drewry WCI infogram JSON for one route (`symbol`).
pub(crate) fn parse_drewry_wci(
    html: &str,
    endpoint: &'static str,
    symbol: &str,
) -> Result<Vec<DrewryWciRow>> {
    let json = extract_infographic_json(html).ok_or_else(|| Error::UpstreamChanged {
        origin: endpoint,
        message: "window.infographicData script not found".into(),
    })?;
    let v: Value = serde_json::from_str(&json)
        .map_err(|e| Error::Parse { endpoint, message: format!("infographic json: {e}") })?;
    let symbol_idx: usize = match symbol {
        "composite" => 0,
        "shanghai-rotterdam" => 1,
        "rotterdam-shanghai" => 2,
        "shanghai-los angeles" => 3,
        "los angeles-shanghai" => 4,
        "shanghai-genoa" => 5,
        "new york-rotterdam" => 6,
        "rotterdam-new york" => 7,
        _ => {
            return Err(Error::Parse {
                endpoint,
                message: format!("unknown drewry symbol {symbol}"),
            })
        }
    };
    let uuid = "7a55585f-3fb3-44e6-9b54-beea1cd20b4d";
    let series = v["elements"]["content"]["content"]["entities"][uuid]["data"]
        .get(symbol_idx)
        .and_then(|s| s.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: endpoint,
            message: format!("missing series for {symbol}"),
        })?;
    if series.len() < 2 {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: "series has no data points".into(),
        });
    }
    let mut out = Vec::with_capacity(series.len() - 1);
    for point in &series[1..] {
        let arr = match point.as_array() {
            Some(a) if a.len() >= 2 => a,
            _ => continue,
        };
        let date = arr[0].get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let wci = arr[1].get("value").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok());
        if date.is_empty() {
            continue;
        }
        out.push(DrewryWciRow { date, wci });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: "no WCI observations parsed".into(),
        });
    }
    Ok(out)
}

/// Drewry 集装箱指数
/// (`drewry_wci_index`, akshare `index/index_drewry.py:17`).
pub async fn drewry_wci_index(client: &Client, symbol: &str) -> Result<Vec<DrewryWciRow>> {
    let url = "https://infogram.com/world-container-index-1h17493095xl4zj";
    let html = client
        .get_text(SOURCE_INFOGRAM, "drewry_wci_index", url, &[], None)
        .await?;
    parse_drewry_wci(&html, "drewry_wci_index", symbol)
}

// ---------------------------------------------------------------------------
// Joinquant index list (geo-blocked) — `pd.read_html`
// ---------------------------------------------------------------------------

/// One index listing row from joinquant.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexStockInfoRow {
    pub index_code: String,
    pub display_name: String,
    pub publish_date: String,
}

/// Parse the joinquant `indexData` table (`pd.read_html` table 0).
pub(crate) fn parse_index_stock_info(html: &str, endpoint: &'static str) -> Result<Vec<IndexStockInfoRow>> {
    let tables = extract_tables(html, endpoint)?;
    let table = tables
        .first()
        .ok_or_else(|| Error::UpstreamChanged {
            origin: endpoint,
            message: "no index table found".into(),
        })?;
    if table.len() < 2 {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: "index table empty".into(),
        });
    }
    let mut out = Vec::with_capacity(table.len() - 1);
    for row in &table[1..] {
        let code = cell(row, 0).split('.').next().unwrap_or("").to_string();
        out.push(IndexStockInfoRow {
            index_code: code,
            display_name: cell(row, 1),
            publish_date: cell(row, 2),
        });
    }
    Ok(out)
}

/// 聚宽-指数数据-指数列表
/// (`index_stock_info`, akshare `index/index_cons.py:70`).
///
/// NOTE: joinquant geo-blocks non-CN IPs (`当前地区暂不支持访问`), so this
/// cannot be exercised from the build host; the parse logic is implemented.
pub async fn index_stock_info(client: &Client) -> Result<Vec<IndexStockInfoRow>> {
    let url = "https://www.joinquant.com/data/dict/indexData";
    let html = client
        .get_text(SOURCE_JOINQUANT, "index_stock_info", url, &[], None)
        .await?;
    parse_index_stock_info(&html, "index_stock_info")
}

// ---------------------------------------------------------------------------
// Shenwan (legulegu) industry levels
// ---------------------------------------------------------------------------

/// One Shenwan industry classification row (levels 1/2/3 share this shape).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SwLevelRow {
    /// Industry code (akshare `行业代码`).
    pub code: String,
    /// Industry name (akshare `行业名称`).
    pub name: String,
    /// Parent industry name (akshare `上级行业`, levels 2/3 only).
    pub parent: Option<String>,
    /// Constituent count (akshare `成份个数`).
    pub count: Option<i64>,
    /// Static P/E (akshare `静态市盈率`).
    pub pe_static: Option<f64>,
    /// TTM P/E (akshare `TTM(滚动)市盈率`).
    pub pe_ttm: Option<f64>,
    /// P/B (akshare `市净率`).
    pub pb: Option<f64>,
    /// Static dividend yield (akshare `静态股息率`).
    pub dividend_yield: Option<f64>,
}

/// Parse one Shenwan level container (`#level1Items` / `#level2Items` / `#level3Items`).
fn parse_sw_level(
    html: &str,
    endpoint: &'static str,
    level_id: &str,
    with_parent: bool,
) -> Result<Vec<SwLevelRow>> {
    let doc = Html::parse_document(html);
    let container_sel = Selector::parse(level_id)
        .map_err(|e| Error::Parse { endpoint, message: format!("{level_id} selector: {e}") })?;
    let container = doc
        .select(&container_sel)
        .next()
        .ok_or_else(|| Error::UpstreamChanged {
            origin: endpoint,
            message: format!("{level_id} container not found"),
        })?;
    let item_sel = Selector::parse(".lg-industries-item").unwrap();
    let code_sel = Selector::parse(".lg-industries-item-chinese-title").unwrap();
    let num_sel = Selector::parse(".lg-industries-item-number").unwrap();
    let parent_sel = Selector::parse("span.parent-industry-name").unwrap();
    let val_sel = Selector::parse(".lg-sw-industries-item-value .value").unwrap();

    let mut out = Vec::new();
    for item in container.select(&item_sel) {
        let code = item
            .select(&code_sel)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        if code.is_empty() {
            continue;
        }
        let num_div = item.select(&num_sel).next();
        let full_text = num_div
            .map(|e| e.text().collect::<String>())
            .unwrap_or_default();
        // name = text before first '('; count = first parenthesized content.
        let name = full_text.split('(').next().unwrap_or("").trim().to_string();
        let count = full_text
            .split('(')
            .nth(1)
            .and_then(|s| s.split(')').next())
            .and_then(|s| s.trim().parse::<i64>().ok());
        let parent = if with_parent {
            item.select(&parent_sel)
                .next()
                .map(|e| {
                    let t = e.text().collect::<String>().trim().to_string();
                    t.trim_start_matches('[').trim_end_matches(']').to_string()
                })
                .filter(|t| !t.is_empty())
        } else {
            None
        };
        let values: Vec<String> = item
            .select(&val_sel)
            .map(|e| e.text().collect::<String>().trim().to_string())
            .collect();
        out.push(SwLevelRow {
            code,
            name,
            parent,
            count,
            pe_static: values.get(0).and_then(|s| s.parse::<f64>().ok()),
            pe_ttm: values.get(1).and_then(|s| s.parse::<f64>().ok()),
            pb: values.get(2).and_then(|s| s.parse::<f64>().ok()),
            dividend_yield: values.get(3).and_then(|s| s.parse::<f64>().ok()),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: format!("{level_id} has no industry items"),
        });
    }
    Ok(out)
}

/// 乐咕乐股-申万一级-分类
/// (`sw_index_first_info`, akshare `index/index_sw.py:38`).
pub async fn sw_index_first_info(client: &Client) -> Result<Vec<SwLevelRow>> {
    let html = client
        .get_text(
            SOURCE_LEGULEGU,
            "sw_index_first_info",
            "https://legulegu.com/stockdata/sw-industry-overview",
            &[],
            None,
        )
        .await?;
    parse_sw_level(&html, "sw_index_first_info", "#level1Items", false)
}

/// 乐咕乐股-申万二级-分类
/// (`sw_index_second_info`, akshare `index/index_sw.py:96`).
pub async fn sw_index_second_info(client: &Client) -> Result<Vec<SwLevelRow>> {
    let html = client
        .get_text(
            SOURCE_LEGULEGU,
            "sw_index_second_info",
            "https://legulegu.com/stockdata/sw-industry-overview",
            &[],
            None,
        )
        .await?;
    parse_sw_level(&html, "sw_index_second_info", "#level2Items", true)
}

/// 乐咕乐股-申万三级-分类
/// (`sw_index_third_info`, akshare `index/index_sw.py:158`).
pub async fn sw_index_third_info(client: &Client) -> Result<Vec<SwLevelRow>> {
    let html = client
        .get_text(
            SOURCE_LEGULEGU,
            "sw_index_third_info",
            "https://legulegu.com/stockdata/sw-industry-overview",
            &[],
            None,
        )
        .await?;
    parse_sw_level(&html, "sw_index_third_info", "#level3Items", true)
}

// ---------------------------------------------------------------------------
// Shenwan level-3 constituents (legulegu table)
// ---------------------------------------------------------------------------

/// One Shenwan level-3 constituent row.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SwThirdConsRow {
    pub seq: Option<i64>,
    pub code: String,
    pub name: String,
    pub include_date: String,
    pub level1: String,
    pub concept: String,
    pub price: Option<f64>,
    pub pe: Option<f64>,
    pub pe_ttm: Option<f64>,
    pub pb: Option<f64>,
    pub dividend_yield: Option<f64>,
    pub roe: Option<f64>,
    pub market_cap: Option<f64>,
    pub chg_1d: Option<f64>,
    pub chg_5d: Option<f64>,
    pub chg_ytd: Option<f64>,
    pub net_profit_growth: Option<f64>,
    pub revenue_growth: Option<f64>,
}

/// Parse the legulegu `index-composition` table (header JSON-LD noise stripped).
pub(crate) fn parse_sw_index_third_cons(html: &str, endpoint: &'static str) -> Result<Vec<SwThirdConsRow>> {
    let tables = extract_tables(html, endpoint)?;
    let table = tables
        .first()
        .ok_or_else(|| Error::UpstreamChanged {
            origin: endpoint,
            message: "no constituents table found".into(),
        })?;
    if table.len() < 2 {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: "constituents table empty".into(),
        });
    }
    let mut out = Vec::with_capacity(table.len() - 1);
    for row in &table[1..] {
        out.push(SwThirdConsRow {
            seq: cell(row, 0).parse::<i64>().ok(),
            code: cell(row, 1),
            name: cell(row, 2),
            include_date: cell(row, 3),
            level1: cell(row, 4),
            concept: cell(row, 5),
            price: f64opt(&cell(row, 6)),
            pe: f64opt(&cell(row, 7)),
            pe_ttm: f64opt(&cell(row, 8)),
            pb: f64opt(&cell(row, 9)),
            dividend_yield: f64opt(&cell(row, 10)),
            roe: f64opt(&cell(row, 11)),
            market_cap: f64opt(&cell(row, 12)),
            chg_1d: f64opt(&cell(row, 13)),
            chg_5d: f64opt(&cell(row, 14)),
            chg_ytd: f64opt(&cell(row, 15)),
            net_profit_growth: f64opt(&cell(row, 16)),
            revenue_growth: f64opt(&cell(row, 17)),
        });
    }
    Ok(out)
}

/// 乐咕乐股-申万三级-行业成份
/// (`sw_index_third_cons`, akshare `index/index_sw.py:220`).
pub async fn sw_index_third_cons(client: &Client, symbol: &str) -> Result<Vec<SwThirdConsRow>> {
    let url = format!("https://legulegu.com/stockdata/index-composition?industryCode={symbol}");
    let html = client
        .get_text(SOURCE_LEGULEGU, "sw_index_third_cons", &url, &[], None)
        .await?;
    parse_sw_index_third_cons(&html, "sw_index_third_cons")
}

// ---------------------------------------------------------------------------
// CNI index historical adjustment (Excel via calamine) — blocked live
// ---------------------------------------------------------------------------

/// One CNI historical-adjustment sample row.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CniAdjustRow {
    /// 样本代码 (zero-padded to 6 digits by akshare).
    pub sample_code: String,
    /// Remaining raw cells from the xlsx (naive without a live fixture).
    pub extra: Vec<String>,
}

/// Parse a CNI adjustment `.xlsx` from raw bytes using `calamine`.
pub(crate) fn parse_index_detail_hist_adjust_cni(
    bytes: &[u8],
    endpoint: &'static str,
) -> Result<Vec<CniAdjustRow>> {
    use calamine::Reader;
    use std::io::Cursor;

    let mut workbook = calamine::open_workbook_auto_from_rs(Cursor::new(bytes))
        .map_err(|e| Error::Parse { endpoint, message: format!("xlsx: {e}") })?;
    let sheet = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| Error::UpstreamChanged {
            origin: endpoint,
            message: "xlsx has no sheets".into(),
        })?;
    let range = workbook
        .worksheet_range(&sheet)
        .map_err(|e| Error::Parse { endpoint, message: format!("sheet: {e}") })?;
    let mut out = Vec::new();
    for (i, row) in range.rows().enumerate() {
        if i == 0 {
            continue; // header
        }
        let cells: Vec<String> = row.iter().map(|c| c.to_string()).collect();
        if cells.is_empty() || cells.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        let sample_code = cells
            .first()
            .map(|c| c.trim().to_string())
            .unwrap_or_default();
        out.push(CniAdjustRow {
            sample_code,
            extra: cells,
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: "xlsx produced no rows".into(),
        });
    }
    Ok(out)
}

/// 国证指数-样本详情-历史调样
/// (`index_detail_hist_adjust_cni`, akshare `index/index_cni.py:191`).
///
/// NOTE: the live CNI endpoint returns "样本历史调样文件不存在！" for every
/// symbol (no real `.xlsx` obtainable from the build host), so this is
/// implemented but not exercised; see report.
pub async fn index_detail_hist_adjust_cni(
    client: &Client,
    symbol: &str,
) -> Result<Vec<CniAdjustRow>> {
    let url = "http://www.cnindex.com.cn/sample-detail/download-adjustment";
    let bytes = client
        .get_text(
            SOURCE_CNI,
            "index_detail_hist_adjust_cni",
            url,
            &[("indexcode", symbol)],
            None,
        )
        .await?;
    parse_index_detail_hist_adjust_cni(bytes.as_bytes(), "index_detail_hist_adjust_cni")
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn load(name: &str) -> String {
        let bytes = std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name))
            .unwrap_or_else(|e| panic!("fixture {name}: {e}"));
        match std::str::from_utf8(&bytes) {
            Ok(s) => s.to_string(),
            Err(_) => encoding_rs::GBK
                .decode_without_bom_handling_and_without_replacement(&bytes)
                .unwrap_or_else(|| String::from_utf8_lossy(&bytes))
                .into_owned(),
        }
    }

    #[test]
    fn parses_drewry_wci_index() {
        let rows = parse_drewry_wci(&load("index_drewry_wci.html"), "drewry_wci_index", "composite").unwrap();
        assert!(!rows.is_empty());
        assert!(rows.iter().any(|r| r.date.contains("Mar") || r.date.contains("-")));
        assert!(rows.iter().any(|r| r.wci.is_some()));
    }

    #[test]
    fn parses_sw_index_first_info() {
        let rows = parse_sw_level(&load("index_sw_overview.html"), "sw_index_first_info", "#level1Items", false).unwrap();
        assert!(!rows.is_empty());
        let first = &rows[0];
        assert!(first.code.ends_with(".SI"));
        assert!(first.count.is_some());
        assert!(first.pe_static.is_some());
    }

    #[test]
    fn parses_sw_index_second_info() {
        let rows = parse_sw_level(&load("index_sw_overview.html"), "sw_index_second_info", "#level2Items", true).unwrap();
        assert!(!rows.is_empty());
        assert!(rows.iter().all(|r| r.parent.is_some()));
        assert!(rows[0].count.is_some());
    }

    #[test]
    fn parses_sw_index_third_info() {
        let rows = parse_sw_level(&load("index_sw_overview.html"), "sw_index_third_info", "#level3Items", true).unwrap();
        assert!(!rows.is_empty());
        assert!(rows.iter().all(|r| r.parent.is_some()));
    }

    #[test]
    fn parses_sw_index_third_cons() {
        let rows = parse_sw_index_third_cons(&load("index_sw_third_cons.html"), "sw_index_third_cons").unwrap();
        assert!(rows.len() > 50, "expected many constituents, got {}", rows.len());
        assert_eq!(rows[0].code, "600419.SH");
        assert_eq!(rows[0].name, "天润乳业");
        assert!(rows[0].pe.is_some());
    }

    // index_stock_info: joinquant is geo-blocked (returns "当前地区暂不支持访问").
    // parse_index_stock_info is implemented; no live fixture obtainable here.
    #[test]
    #[ignore = "joinquant geo-blocked from build host"]
    fn parses_index_stock_info() {
        let _ = parse_index_stock_info(&load("index_stock_info.html"), "index_stock_info");
    }

    // index_detail_hist_adjust_cni: CNI download returns "文件不存在" for all symbols.
    #[test]
    #[ignore = "CNI adjustment xlsx unavailable (文件不存在)"]
    fn parses_index_detail_hist_adjust_cni() {
        let bytes = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join("index_detail_hist_adjust_cni.xlsx"),
        )
        .unwrap();
        let _ = parse_index_detail_hist_adjust_cni(&bytes, "index_detail_hist_adjust_cni");
    }
}
