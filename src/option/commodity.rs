//! Commodity options daily history & implied volatility (akshare `option_commodity.py`).
//!
//! Rust port of akshare's commodity-option functions for the four Chinese futures
//! exchanges. Mapping of each Rust fn to its akshare source line:
//!
//! | Rust fn | akshare fn | source line | status |
//! | --- | --- | --- | --- |
//! | `option_hist_dce` | `option_hist_dce` | `option_commodity.py:32` | DEFERRED |
//! | `option_hist_czce` | `option_hist_czce` | `option_commodity.py:187` | DEFERRED |
//! | `option_hist_shfe` | `option_hist_shfe` | `option_commodity.py:365` | implemented |
//! | `option_vol_shfe` | `option_vol_shfe` | `option_commodity.py:445` | implemented |
//! | `option_hist_gfex` | `option_hist_gfex` | `option_commodity.py:504` | implemented |
//! | `option_vol_gfex` | `option_vol_gfex` | `option_commodity.py:593` | implemented |
//!
//! ## DEFERRED
//!
//! - `option_hist_dce` — DCE option daily history is fetched with
//!   `requests.post(url, json=payload)`, i.e. a **JSON-body POST**. The shared
//!   `Client` only exposes GET (`get_json`/`get_text`) and form-encoded POST
//!   (`post_form_json`); there is no JSON-body POST wrapper and `client.rs` must
//!   not be edited, so a faithful replication is not feasible.
//! - `option_hist_czce` — CZCE option daily history returns a pipe-`|`-delimited
//!   `OptionDataDaily.txt` text page; akshare scrapes it with
//!   `pd.read_table(sep="|")`. This is HTML/text table scraping, not a pure JSON
//!   endpoint, so it cannot be ported with `get_json`/`get_text` + a trivial parser.

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use serde_json::Value;

const SOURCE_SHFE: &str = "shfe";
const SOURCE_GFEX: &str = "gfex";

/// Normalize a trade date to `YYYYMMDD` digits (akshare `convert_date`).
fn normalize_trade_date(trade_date: &str) -> Result<String> {
    let digits: String = trade_date.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 8 {
        return Err(Error::InvalidParam(format!(
            "trade_date must be YYYYMMDD, got {trade_date}"
        )));
    }
    Ok(digits)
}

/// Get a string field, tolerating missing/null values.
fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Get a numeric field, tolerating comma thousands-separators and string-encoded numbers.
fn fnum(item: &Value, k: &str) -> Option<f64> {
    match item.get(k) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.replace(',', "").parse::<f64>().ok(),
        _ => None,
    }
}

/// DEFERRED: see module `## DEFERRED` section. DCE uses a JSON-body POST that the
/// shared `Client` does not support.
pub fn option_hist_dce(_client: &Client, _symbol: &str, _trade_date: &str) -> Result<Vec<Value>> {
    Err(Error::UpstreamChanged {
        origin: SOURCE_DCE,
        message: "DEFERRED: DCE option history requires a JSON-body POST not supported by Client"
            .to_string(),
    })
}

/// DEFERRED: see module `## DEFERRED` section. CZCE returns pipe-delimited text
/// that akshare scrapes with `pd.read_table(sep="|")`.
pub fn option_hist_czce(_client: &Client, _symbol: &str, _trade_date: &str) -> Result<Vec<Value>> {
    Err(Error::UpstreamChanged {
        origin: SOURCE_CZCE,
        message: "DEFERRED: CZCE option history returns pipe-delimited text, not pure JSON"
            .to_string(),
    })
}

const SOURCE_DCE: &str = "dce";
const SOURCE_CZCE: &str = "czce";

// ---------------------------------------------------------------------------
// SHFE option daily history (option_commodity.py:365)
// ---------------------------------------------------------------------------

/// A single SHFE option contract daily row (`option_hist_shfe`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShfeOptionHistRow {
    pub instrument_id: String,
    pub open_price: Option<f64>,
    pub highest_price: Option<f64>,
    pub lowest_price: Option<f64>,
    pub close_price: Option<f64>,
    pub pre_settlement_price: Option<f64>,
    pub settlement_price: Option<f64>,
    pub zd1_chg: Option<f64>,
    pub zd2_chg: Option<f64>,
    pub volume: Option<f64>,
    pub open_interest: Option<f64>,
    pub open_interest_chg: Option<f64>,
    pub turnover: Option<f64>,
    pub delta: Option<f64>,
    pub exec_volume: Option<f64>,
}

/// Pure parser for SHFE option daily history (`o_curinstrument` array).
///
/// Mirrors akshare: drops `小计`/`合计`/blank `INSTRUMENTID` rows and keeps rows
/// whose `PRODUCTNAME` (stripped) equals `symbol`.
pub fn parse_shfe_option_hist(json: &Value, symbol: &str) -> Vec<ShfeOptionHistRow> {
    let Some(arr) = json.get("o_curinstrument").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|row| {
            let id = fstr(row, "INSTRUMENTID")?;
            if id.is_empty() || id == "小计" || id == "合计" {
                return None;
            }
            let product = fstr(row, "PRODUCTNAME")?.trim().to_string();
            if product != symbol {
                return None;
            }
            Some(ShfeOptionHistRow {
                instrument_id: id,
                open_price: fnum(row, "OPENPRICE"),
                highest_price: fnum(row, "HIGHESTPRICE"),
                lowest_price: fnum(row, "LOWESTPRICE"),
                close_price: fnum(row, "CLOSEPRICE"),
                pre_settlement_price: fnum(row, "PRESETTLEMENTPRICE"),
                settlement_price: fnum(row, "SETTLEMENTPRICE"),
                zd1_chg: fnum(row, "ZD1_CHG"),
                zd2_chg: fnum(row, "ZD2_CHG"),
                volume: fnum(row, "VOLUME"),
                open_interest: fnum(row, "OPENINTEREST"),
                open_interest_chg: fnum(row, "OPENINTERESTCHG"),
                turnover: fnum(row, "TURNOVER"),
                delta: fnum(row, "DELTA"),
                exec_volume: fnum(row, "EXECVOLUME"),
            })
        })
        .collect()
}

/// SHFE option daily history (akshare `option_hist_shfe`).
///
/// GETs `https://www.shfe.com.cn/data/tradedata/option/dailydata/kx{YYYYMMDD}.dat`
/// (JSON) with the exchange's `User-Agent`; parses the `o_curinstrument` array.
pub async fn option_hist_shfe(
    client: &Client,
    symbol: &str,
    trade_date: &str,
) -> Result<Vec<ShfeOptionHistRow>> {
    let day = normalize_trade_date(trade_date)?;
    let url = format!(
        "https://www.shfe.com.cn/data/tradedata/option/dailydata/kx{day}.dat"
    );
    let headers = &[("User-Agent", "Mozilla/4.0 (compatible; MSIE 5.5; Windows NT)")];
    let json = client
        .get_json_with_headers(SOURCE_SHFE, "option_hist_shfe", &url, &[], Some(headers))
        .await?;
    Ok(parse_shfe_option_hist(&json, symbol))
}

// ---------------------------------------------------------------------------
// SHFE option implied volatility (option_commodity.py:445)
// ---------------------------------------------------------------------------

/// A single SHFE option implied-volatility row (`option_vol_shfe`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShfeOptionVolRow {
    pub instrument_id: String,
    pub volume: Option<f64>,
    pub open_interest: Option<f64>,
    pub open_interest_chg: Option<f64>,
    pub turnover: Option<f64>,
    pub exec_volume: Option<f64>,
    pub sigma: Option<f64>,
}

/// Pure parser for SHFE option implied volatility (`o_cursigma` array).
///
/// Keeps rows whose `PRODUCTNAME` (stripped) equals `symbol`.
pub fn parse_shfe_option_vol(json: &Value, symbol: &str) -> Vec<ShfeOptionVolRow> {
    let Some(arr) = json.get("o_cursigma").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|row| {
            let product = fstr(row, "PRODUCTNAME")?.trim().to_string();
            if product != symbol {
                return None;
            }
            Some(ShfeOptionVolRow {
                instrument_id: fstr(row, "INSTRUMENTID")?,
                volume: fnum(row, "VOLUME"),
                open_interest: fnum(row, "OPENINTEREST"),
                open_interest_chg: fnum(row, "OPENINTERESTCHG"),
                turnover: fnum(row, "TURNOVER"),
                exec_volume: fnum(row, "EXECVOLUME"),
                sigma: fnum(row, "SIGMA"),
            })
        })
        .collect()
}

/// SHFE option implied volatility (akshare `option_vol_shfe`).
///
/// Same endpoint as [`option_hist_shfe`]; parses the `o_cursigma` array.
pub async fn option_vol_shfe(
    client: &Client,
    symbol: &str,
    trade_date: &str,
) -> Result<Vec<ShfeOptionVolRow>> {
    let day = normalize_trade_date(trade_date)?;
    let url = format!(
        "https://www.shfe.com.cn/data/tradedata/option/dailydata/kx{day}.dat"
    );
    let headers = &[("User-Agent", "Mozilla/4.0 (compatible; MSIE 5.5; Windows NT)")];
    let json = client
        .get_json_with_headers(SOURCE_SHFE, "option_vol_shfe", &url, &[], Some(headers))
        .await?;
    Ok(parse_shfe_option_vol(&json, symbol))
}

// ---------------------------------------------------------------------------
// GFEX option daily history (option_commodity.py:504)
// ---------------------------------------------------------------------------

/// A single GFEX option contract daily row (`option_hist_gfex`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct GfexOptionHistRow {
    pub variety: Option<String>,
    pub deliv_month: Option<String>,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: Option<f64>,
    pub last_clear: Option<f64>,
    pub clear_price: Option<f64>,
    pub diff: Option<f64>,
    pub diff1: Option<f64>,
    pub delta: Option<f64>,
    pub volumn: Option<f64>,
    pub open_interest: Option<f64>,
    pub diff_i: Option<f64>,
    pub turnover: Option<f64>,
    pub match_qty_sum: Option<f64>,
    pub implied_volatility: Option<f64>,
}

/// Pure parser for GFEX option daily history (`data` array).
///
/// Keeps rows whose `variety` contains `symbol` (akshare `str.contains`).
pub fn parse_gfex_option_hist(json: &Value, symbol: &str) -> Vec<GfexOptionHistRow> {
    let Some(arr) = json.get("data").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|row| {
            let variety = fstr(row, "variety")?;
            if !variety.contains(symbol) {
                return None;
            }
            Some(GfexOptionHistRow {
                variety: Some(variety),
                deliv_month: fstr(row, "delivMonth"),
                open: fnum(row, "open"),
                high: fnum(row, "high"),
                low: fnum(row, "low"),
                close: fnum(row, "close"),
                last_clear: fnum(row, "lastClear"),
                clear_price: fnum(row, "clearPrice"),
                diff: fnum(row, "diff"),
                diff1: fnum(row, "diff1"),
                delta: fnum(row, "delta"),
                volumn: fnum(row, "volumn"),
                open_interest: fnum(row, "openInterest"),
                diff_i: fnum(row, "diffI"),
                turnover: fnum(row, "turnover"),
                match_qty_sum: fnum(row, "matchQtySum"),
                implied_volatility: fnum(row, "impliedVolatility"),
            })
        })
        .collect()
}

/// GFEX option daily history (akshare `option_hist_gfex`).
///
/// POSTs form `trade_date` + `trade_type=1` to
/// `http://www.gfex.com.cn/u/interfacesWebTiDayQuotes/loadList` and parses `data`.
pub async fn option_hist_gfex(
    client: &Client,
    symbol: &str,
    trade_date: &str,
) -> Result<Vec<GfexOptionHistRow>> {
    let day = normalize_trade_date(trade_date)?;
    let url = "http://www.gfex.com.cn/u/interfacesWebTiDayQuotes/loadList";
    let params = &[("trade_date", day.as_str()), ("trade_type", "1")];
    let headers = &[("Referer", "http://www.gfex.com.cn/gfex/rihq/hqsj_tjsj.shtml")];
    let json = client
        .post_form_json(SOURCE_GFEX, "option_hist_gfex", url, params, Some(headers))
        .await?;
    Ok(parse_gfex_option_hist(&json, symbol))
}

// ---------------------------------------------------------------------------
// GFEX option implied volatility (option_commodity.py:593)
// ---------------------------------------------------------------------------

/// A single GFEX option implied-volatility row (`option_vol_gfex`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct GfexOptionVolRow {
    pub series_id: Option<String>,
    pub his_volatility: Option<f64>,
}

/// Map a GFEX option `symbol` to its variety code (akshare `symbol_code_map`).
fn gfex_vol_symbol_code(symbol: &str) -> Option<&'static str> {
    match symbol {
        "工业硅" => Some("si"),
        "碳酸锂" => Some("lc"),
        "多晶硅" => Some("ps"),
        _ => None,
    }
}

/// Pure parser for GFEX option implied volatility (`data` array).
///
/// Keeps rows whose `seriesId` contains `symbol_code`.
pub fn parse_gfex_option_vol(json: &Value, symbol_code: &str) -> Vec<GfexOptionVolRow> {
    let Some(arr) = json.get("data").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|row| {
            let series = fstr(row, "seriesId")?;
            if !series.contains(symbol_code) {
                return None;
            }
            Some(GfexOptionVolRow {
                series_id: Some(series),
                his_volatility: fnum(row, "hisVolatility"),
            })
        })
        .collect()
}

/// GFEX option implied volatility (akshare `option_vol_gfex`).
///
/// POSTs form `trade_date` to
/// `http://www.gfex.com.cn/u/interfacesWebTiDayQuotes/loadListOptVolatility`
/// and parses `data`, filtering by the variety code for `symbol`.
pub async fn option_vol_gfex(
    client: &Client,
    symbol: &str,
    trade_date: &str,
) -> Result<Vec<GfexOptionVolRow>> {
    let code = gfex_vol_symbol_code(symbol).ok_or_else(|| {
        Error::InvalidParam(format!("unsupported GFEX option symbol: {symbol}"))
    })?;
    let day = normalize_trade_date(trade_date)?;
    let url = "http://www.gfex.com.cn/u/interfacesWebTiDayQuotes/loadListOptVolatility";
    let params = &[("trade_date", day.as_str())];
    let headers = &[("Referer", "http://www.gfex.com.cn/gfex/rihq/hqsj_tjsj.shtml")];
    let json = client
        .post_form_json(SOURCE_GFEX, "option_vol_gfex", url, params, Some(headers))
        .await?;
    Ok(parse_gfex_option_vol(&json, code))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Value {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    #[test]
    fn parse_shfe_option_hist_filters_and_renames() {
        let json = fixture("option_hist_shfe.json");
        let rows = parse_shfe_option_hist(&json, "天胶期权");
        // Two real 天胶期权 rows; 小计 / blank / 黄金期权 rows are dropped.
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].instrument_id, "ru2411C12000");
        assert_eq!(rows[0].close_price, Some(105.0));
        // Comma thousands-separators are stripped on numeric parse.
        assert_eq!(rows[0].volume, Some(1234.0));
        assert_eq!(rows[0].open_interest, Some(5678.0));
        assert_eq!(rows[1].instrument_id, "ru2411P12000");
        assert_eq!(rows[1].delta, Some(-0.3));
    }

    #[test]
    fn parse_shfe_option_vol_filters_by_product() {
        let json = fixture("option_vol_shfe.json");
        let rows = parse_shfe_option_vol(&json, "天胶期权");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].instrument_id, "ru2411");
        assert_eq!(rows[0].sigma, Some(0.25));
        assert_eq!(rows[0].volume, Some(1234.0));
    }

    #[test]
    fn parse_gfex_option_hist_filters_by_variety() {
        let json = fixture("option_hist_gfex.json");
        let rows = parse_gfex_option_hist(&json, "工业硅");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].variety.as_deref(), Some("工业硅期权"));
        assert_eq!(rows[0].deliv_month.as_deref(), Some("si2411"));
        assert_eq!(rows[0].close, Some(8050.0));
        assert_eq!(rows[0].implied_volatility, Some(0.30));
    }

    #[test]
    fn parse_gfex_option_vol_filters_by_code() {
        let json = fixture("option_vol_gfex.json");
        let rows = parse_gfex_option_vol(&json, "si");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].series_id.as_deref(), Some("si2411"));
        assert_eq!(rows[0].his_volatility, Some(0.30));
        assert_eq!(rows[1].series_id.as_deref(), Some("si2412"));
    }

    #[test]
    fn normalize_trade_date_strips_dashes() {
        assert_eq!(normalize_trade_date("2025-10-16").unwrap(), "20251016");
        assert!(normalize_trade_date("2025").is_err());
    }
}
