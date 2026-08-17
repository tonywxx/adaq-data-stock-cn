//! Carbon-emissions trading endpoints ported from `akshare/energy/energy_carbon.py`.
//!
//! `energy_carbon_hb` (湖北碳排放权交易中心) is implemented end-to-end with a
//! real captured fixture. The other four (`bj`, `sz`, `eu`, `gz`) require
//! upstreams that are unreachable from the build sandbox (see `## Blocked`
//! below); their parsers are written to match akshare's logic but their tests
//! are `#[ignore]`d because no live fixture could be captured.
//!
//! * [`energy_carbon_bj`] — `energy_carbon.py:76`
//! * [`energy_carbon_eu`] — `energy_carbon.py:166`
//! * [`energy_carbon_gz`] — `energy_carbon.py:242`
//! * [`energy_carbon_hb`] — `energy_carbon.py:198`
//! * [`energy_carbon_sz`] — `energy_carbon.py:134`

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// Parse a numeric cell into `f64`, tolerating thousands separators and the
/// `(单位)` suffix some pages append to 成交额/成交额.
fn as_f64(s: &str) -> Option<f64> {
    let mut t = s.trim().to_string();
    // Drop a parenthesised unit, e.g. "1,234.5(吨)".
    if let Some(p) = t.find('(') {
        t.truncate(p);
    }
    t = t.replace(',', "").replace('（', "").replace('）', "");
    t.parse::<f64>().ok()
}

/// Extract every `<table>` as a list of rows-of-cells (mirrors `pd.read_html`).
fn extract_tables(html: &str, endpoint: &'static str) -> Result<Vec<Vec<Vec<String>>>> {
    crate::core::html::tables(html, endpoint)
}

// ---------------------------------------------------------------------------
// energy_carbon_hb — JSON array embedded in a <script> (湖北)
// ---------------------------------------------------------------------------

/// One daily Hubei carbon spot observation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EnergyCarbonHb {
    /// Trade date `YYYY-MM-DD` (akshare `日期` ← `riqi`).
    pub date: String,
    /// Settlement price (akshare `成交价` ← `cjj`).
    pub price: Option<f64>,
    /// Volume (akshare `成交量` ← `cjl`).
    pub volume: Option<f64>,
    /// Latest price (akshare `最新` ← `zx`).
    pub latest: Option<f64>,
    /// Change (akshare `涨跌` ← `zd`).
    pub change: Option<f64>,
}

/// 湖北碳排放权交易中心-现货交易数据-配额-每日概况 (`energy_carbon_hb`, akshare `energy_carbon.py:198`).
pub async fn energy_carbon_hb(client: &Client) -> Result<Vec<EnergyCarbonHb>> {
    let url = "https://www.hbets.cn/";
    let html = client
        .get_text("hbets", "energy_carbon_hb", url, &[], None)
        .await?;
    parse_energy_carbon_hb(&html, "energy_carbon_hb")
}

/// Extract the `cjj = '[...]'` JSON array embedded in the Hubei page `<script>`
/// and parse it. Mirrors akshare's `demjson.decode` of the same blob.
pub(crate) fn parse_energy_carbon_hb(html: &str, endpoint: &'static str) -> Result<Vec<EnergyCarbonHb>> {
    let marker = "cjj = '";
    let start = html
        .find(marker)
        .map(|i| i + marker.len())
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "cjj array not found".into() })?;
    let end = html[start..]
        .find("]'")
        .map(|e| start + e + 1)
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "cjj array unterminated".into() })?;
    let arr_text = &html[start..end];
    let arr: Vec<Value> = serde_json::from_str(arr_text)
        .map_err(|e| Error::Parse { endpoint, message: format!("json: {e}") })?;
    let f = |v: &Value, k: &str| -> Option<f64> {
        v.get(k).and_then(|x| x.as_str()).and_then(as_f64)
    };
    let mut out = Vec::with_capacity(arr.len());
    for obj in &arr {
        out.push(EnergyCarbonHb {
            date: obj.get("riqi").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            price: f(obj, "cjj"),
            volume: f(obj, "cjl"),
            latest: f(obj, "zx"),
            change: f(obj, "zd"),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "empty cjj array".into() });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// energy_carbon_bj — multi-page crawl of bjets.com.cn (北京)
// ---------------------------------------------------------------------------

/// One Beijing carbon-exchange session (single page of the crawl).
#[derive(Debug, Clone, serde::Serialize)]
pub struct EnergyCarbonBj {
    /// Date (akshare `日期`).
    pub date: String,
    /// Volume (akshare `成交量`).
    pub volume: Option<f64>,
    /// Average price (akshare `成交均价`).
    pub avg_price: Option<f64>,
    /// Turnover (akshare `成交额`, unit stripped).
    pub amount: Option<f64>,
}

/// 北京市碳排放权电子交易平台-北京市碳排放权公开交易行情 (`energy_carbon_bj`, akshare `energy_carbon.py:76`).
///
/// NOTE: the upstream paginates every page; this parses one page's `table[0]`.
pub async fn energy_carbon_bj(client: &Client) -> Result<Vec<EnergyCarbonBj>> {
    let url = "https://www.bjets.com.cn/article/jyxx/";
    let html = client
        .get_text("bjets", "energy_carbon_bj", url, &[], None)
        .await?;
    parse_energy_carbon_bj(&html, "energy_carbon_bj")
}

pub(crate) fn parse_energy_carbon_bj(html: &str, endpoint: &'static str) -> Result<Vec<EnergyCarbonBj>> {
    let tables = extract_tables(html, endpoint)?;
    let rows = tables
        .into_iter()
        .next()
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "missing table".into() })?;
    let mut out = Vec::new();
    for cells in rows.into_iter().skip(1) {
        if cells.len() < 4 {
            continue;
        }
        out.push(EnergyCarbonBj {
            date: cells[0].clone(),
            volume: as_f64(&cells[1]),
            avg_price: as_f64(&cells[2]),
            amount: as_f64(&cells[3]),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no rows".into() });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// energy_carbon_sz / energy_carbon_eu — cerx.cn daily quotes (深圳国内/国际)
// ---------------------------------------------------------------------------

/// One daily carbon quote (深圳 国内/国际 carbon info).
#[derive(Debug, Clone, serde::Serialize)]
pub struct EnergyCarbonQuote {
    /// Trade date (akshare `交易日期`).
    pub date: String,
    /// Open (akshare `开盘价`).
    pub open: Option<f64>,
    /// High (akshare `最高价`).
    pub high: Option<f64>,
    /// Low (akshare `最低价`).
    pub low: Option<f64>,
    /// Average price (akshare `成交均价`).
    pub avg_price: Option<f64>,
    /// Close (akshare `收盘价`).
    pub close: Option<f64>,
    /// Volume (akshare `成交量`).
    pub volume: Option<f64>,
    /// Turnover (akshare `成交额`).
    pub amount: Option<f64>,
}

/// 深圳碳排放交易所-国内碳情 (`energy_carbon_sz`, akshare `energy_carbon.py:134`).
pub async fn energy_carbon_sz(client: &Client) -> Result<Vec<EnergyCarbonQuote>> {
    let url = "http://www.cerx.cn/dailynewsCN/index.htm";
    let html = client
        .get_text("cerx", "energy_carbon_sz", url, &[], None)
        .await?;
    parse_energy_carbon_quote(&html, "energy_carbon_sz")
}

/// 深圳碳排放交易所-国际碳情 (`energy_carbon_eu`, akshare `energy_carbon.py:166`).
pub async fn energy_carbon_eu(client: &Client) -> Result<Vec<EnergyCarbonQuote>> {
    let url = "http://www.cerx.cn/dailynewsOuter/index.htm";
    let html = client
        .get_text("cerx", "energy_carbon_eu", url, &[], None)
        .await?;
    parse_energy_carbon_quote(&html, "energy_carbon_eu")
}

pub(crate) fn parse_energy_carbon_quote(html: &str, endpoint: &'static str) -> Result<Vec<EnergyCarbonQuote>> {
    let tables = extract_tables(html, endpoint)?;
    let rows = tables
        .into_iter()
        .next()
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "missing table".into() })?;
    let mut out = Vec::new();
    for cells in rows.into_iter().skip(1) {
        if cells.len() < 8 {
            continue;
        }
        out.push(EnergyCarbonQuote {
            date: cells[0].clone(),
            open: as_f64(&cells[1]),
            high: as_f64(&cells[2]),
            low: as_f64(&cells[3]),
            avg_price: as_f64(&cells[4]),
            close: as_f64(&cells[5]),
            volume: as_f64(&cells[6]),
            amount: as_f64(&cells[7]),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no rows".into() });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// energy_carbon_gz — cnemission.com market history (广州)
// ---------------------------------------------------------------------------

/// One Guangzhou carbon market-history row.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EnergyCarbonGz {
    /// Date `YYYYMMDD` (akshare `日期`).
    pub date: String,
    /// Variety (akshare `品种`).
    pub variety: String,
    /// Open (akshare `开盘价`).
    pub open: Option<f64>,
    /// Close (akshare `收盘价`).
    pub close: Option<f64>,
    /// High (akshare `最高价`).
    pub high: Option<f64>,
    /// Low (akshare `最低价`).
    pub low: Option<f64>,
    /// Change (akshare `涨跌`).
    pub change: Option<f64>,
    /// Change percent (akshare `涨跌幅`, `%` stripped).
    pub change_pct: Option<f64>,
    /// Volume (akshare `成交数量`).
    pub volume: Option<f64>,
    /// Turnover (akshare `成交金额`).
    pub amount: Option<f64>,
}

/// 广州碳排放权交易中心-行情信息 (`energy_carbon_gz`, akshare `energy_carbon.py:242`).
pub async fn energy_carbon_gz(client: &Client) -> Result<Vec<EnergyCarbonGz>> {
    let url = "http://ets.cnemission.com/carbon/portalIndex/markethistory";
    let params: &[(&str, &str)] = &[
        ("Top", "1"),
        ("beginTime", "2010-01-01"),
        ("endTime", "2030-09-12"),
    ];
    let html = client
        .get_text("cnemission", "energy_carbon_gz", url, params, None)
        .await?;
    parse_energy_carbon_gz(&html, "energy_carbon_gz")
}

pub(crate) fn parse_energy_carbon_gz(html: &str, endpoint: &'static str) -> Result<Vec<EnergyCarbonGz>> {
    let tables = extract_tables(html, endpoint)?;
    // akshare uses `pd.read_html(..., header=0)[1]` (the 2nd table).
    if tables.len() < 2 {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: format!("expected >=2 tables, found {}", tables.len()),
        });
    }
    let rows = &tables[1];
    let pct = |s: &str| -> Option<f64> { s.trim().trim_end_matches('%').parse::<f64>().ok() };
    let mut out = Vec::new();
    for cells in rows.iter().skip(1) {
        if cells.len() < 10 {
            continue;
        }
        out.push(EnergyCarbonGz {
            date: cells[0].clone(),
            variety: cells[1].clone(),
            open: as_f64(&cells[2]),
            close: as_f64(&cells[3]),
            high: as_f64(&cells[4]),
            low: as_f64(&cells[5]),
            change: as_f64(&cells[6]),
            change_pct: pct(&cells[7]),
            volume: as_f64(&cells[8]),
            amount: as_f64(&cells[9]),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no rows".into() });
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
    fn parses_energy_carbon_hb() {
        let rows = parse_energy_carbon_hb(&load_html("energy_carbon_hb.html"), "energy_carbon_hb").unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].date, "2014-04-02");
        assert!((rows[0].price.unwrap() - 21.0).abs() < 1e-9);
        assert!((rows[0].volume.unwrap() - 510020.0).abs() < 1e-6);
    }

    // The following four upstreams are unreachable from the build sandbox
    // (bjets.com.cn → HTTP 521, cerx.cn → DNS unresolved, cnemission.com →
    // HTTP 418). No live fixture could be captured, so their parse tests are
    // ignored rather than failing. The parsers still compile and follow
    // akshare's column layout.
    #[test]
    #[ignore = "upstream bjets.com.cn unreachable (HTTP 521) from build sandbox"]
    fn parses_energy_carbon_bj() {
        let _ = parse_energy_carbon_bj(&load_html("energy_carbon_bj.html"), "energy_carbon_bj");
    }

    #[test]
    #[ignore = "upstream cerx.cn unreachable (no DNS) from build sandbox"]
    fn parses_energy_carbon_sz() {
        let _ = parse_energy_carbon_quote(&load_html("energy_carbon_sz.html"), "energy_carbon_sz");
    }

    #[test]
    #[ignore = "upstream cerx.cn unreachable (no DNS) from build sandbox"]
    fn parses_energy_carbon_eu() {
        let _ = parse_energy_carbon_quote(&load_html("energy_carbon_eu.html"), "energy_carbon_eu");
    }

    #[test]
    #[ignore = "upstream cnemission.com unreachable (HTTP 418) from build sandbox"]
    fn parses_energy_carbon_gz() {
        let _ = parse_energy_carbon_gz(&load_html("energy_carbon_gz.html"), "energy_carbon_gz");
    }
}
