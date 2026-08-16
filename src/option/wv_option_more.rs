//! CFFEX index-option spot & daily quotes from Sina (`option_finance_sina.py`).
//!
//! Ports of akshare's Sina CFFEX index-option endpoints:
//!
//! | Rust fn | akshare fn | source line | kind |
//! | --- | --- | --- | --- |
//! | `option_cffex_hs300_spot_sina` | `option_cffex_hs300_spot_sina` | `option_finance_sina.py:150` | spot |
//! | `option_cffex_sz50_spot_sina` | `option_cffex_sz50_spot_sina` | `option_finance_sina.py:77` | spot |
//! | `option_cffex_zz1000_spot_sina` | `option_cffex_zz1000_spot_sina` | `option_finance_sina.py:223` | spot |
//! | `option_cffex_hs300_daily_sina` | `option_cffex_hs300_daily_sina` | `option_finance_sina.py:337` | daily |
//! | `option_cffex_sz50_daily_sina` | `option_cffex_sz50_daily_sina` | `option_finance_sina.py:296` | daily |
//! | `option_cffex_zz1000_daily_sina` | `option_cffex_zz1000_daily_sina` | `option_finance_sina.py:378` | daily |
//!
//! ## Spot (`OptionService.getOptionData`)
//!
//! Plain GET to Sina's `openapi.php`. The response is JSON wrapped in some
//! trailing junk, so akshare slices `text[text.find("{") : text.rfind("}")+1]`
//! before `json.loads`; we replicate with [`extract_json_object`]. The payload's
//! `result.data.up` / `result.data.down` are positional row arrays (call /
//! put side), concatenated side-by-side in akshare — we zip them by index into
//! one [`OptionCffexSpotRow`] with 17 columns (call side carries the strike).
//!
//! ## Daily (`FutureOptionAllService.getOptionDayline`)
//!
//! JSONP-wrapped GET (`var <callback>=[...];`). akshare strips the wrapper with
//! `text[text.find("[") : text.rfind("]")+1]` then `eval`; we use
//! [`extract_json_array`] + `serde_json`. Each inner array is positional
//! `[open, high, low, close, volume, date]` (akshare renames the columns to
//! exactly that order). The callback name embeds the current `YYYY_M_D`, as in
//! akshare (`datetime.datetime.now()`).

use chrono::Datelike;
use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// Sina source identifier (mirrors `core::client::SOURCE_SINA`).
const SOURCE: &str = "sina";

const SPOT_URL: &str =
    "https://stock.finance.sina.com.cn/futures/api/openapi.php/OptionService.getOptionData";
const DAYLINE_URL: &str =
    "https://stock.finance.sina.com.cn/futures/api/jsonp.php/var%20";

// ---------------------------------------------------------------------------
// Spot
// ---------------------------------------------------------------------------

/// A single strike row of CFFEX index-option real-time spot quotes (Sina).
///
/// Mirrors akshare's `option_cffex_{hs300,sz50,zz1000}_spot_sina`, which
/// `pd.concat`s the call (`up`) and put (`down`) blocks side-by-side. The
/// 17 columns are: 9 call fields (the 8th is the strike 行权价) + 8 put fields.
/// Numeric columns are `Option<f64>` (akshare `pd.to_numeric`); the contract
/// identifiers (标识) are kept as `Option<String>`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OptionCffexSpotRow {
    // --- call (up) side ---
    /// 看涨合约-买量
    pub call_buy_volume: Option<f64>,
    /// 看涨合约-买价
    pub call_buy_price: Option<f64>,
    /// 看涨合约-最新价
    pub call_last_price: Option<f64>,
    /// 看涨合约-卖价
    pub call_sell_price: Option<f64>,
    /// 看涨合约-卖量
    pub call_sell_volume: Option<f64>,
    /// 看涨合约-持仓量
    pub call_open_interest: Option<f64>,
    /// 看涨合约-涨跌
    pub call_change: Option<f64>,
    /// 行权价
    pub strike: Option<f64>,
    /// 看涨合约-标识
    pub call_symbol: Option<String>,
    // --- put (down) side ---
    /// 看跌合约-买量
    pub put_buy_volume: Option<f64>,
    /// 看跌合约-买价
    pub put_buy_price: Option<f64>,
    /// 看跌合约-最新价
    pub put_last_price: Option<f64>,
    /// 看跌合约-卖价
    pub put_sell_price: Option<f64>,
    /// 看跌合约-卖量
    pub put_sell_volume: Option<f64>,
    /// 看跌合约-持仓量
    pub put_open_interest: Option<f64>,
    /// 看跌合约-涨跌
    pub put_change: Option<f64>,
    /// 看跌合约-标识
    pub put_symbol: Option<String>,
}

/// CFFEX 上证50 index-option spot quotes for `symbol` (default `ho2303`).
pub async fn option_cffex_sz50_spot_sina(
    client: &Client,
    symbol: &str,
) -> Result<Vec<OptionCffexSpotRow>> {
    cffex_spot(client, "ho", symbol).await
}

/// CFFEX 沪深300 index-option spot quotes for `symbol` (default `io2204`).
pub async fn option_cffex_hs300_spot_sina(
    client: &Client,
    symbol: &str,
) -> Result<Vec<OptionCffexSpotRow>> {
    cffex_spot(client, "io", symbol).await
}

/// CFFEX 中证1000 index-option spot quotes for `symbol` (default `mo2208`).
pub async fn option_cffex_zz1000_spot_sina(
    client: &Client,
    symbol: &str,
) -> Result<Vec<OptionCffexSpotRow>> {
    cffex_spot(client, "mo", symbol).await
}

async fn cffex_spot(client: &Client, product: &str, symbol: &str) -> Result<Vec<OptionCffexSpotRow>> {
    let params = [
        ("type", "futures"),
        ("product", product),
        ("exchange", "cffex"),
        ("pinzhong", symbol),
    ];
    let text = client
        .get_text(SOURCE, "option_cffex_spot", SPOT_URL, &params, None)
        .await?;
    let v = extract_json_object(&text)?;
    parse_cffex_spot(&v)
}

/// Parse Sina's `OptionService.getOptionData` payload into spot rows.
pub(crate) fn parse_cffex_spot(resp: &Value) -> Result<Vec<OptionCffexSpotRow>> {
    let data = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "spot response missing result.data".into(),
        })?;
    let up = data
        .get("up")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "spot response missing result.data.up".into(),
        })?;
    let down = data
        .get("down")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "spot response missing result.data.down".into(),
        })?;
    let n = up.len().max(down.len());
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let u = up.get(i).and_then(|v| v.as_array());
        let d = down.get(i).and_then(|v| v.as_array());
        out.push(OptionCffexSpotRow {
            call_buy_volume: u.and_then(|a| arr_num(a, 0)),
            call_buy_price: u.and_then(|a| arr_num(a, 1)),
            call_last_price: u.and_then(|a| arr_num(a, 2)),
            call_sell_price: u.and_then(|a| arr_num(a, 3)),
            call_sell_volume: u.and_then(|a| arr_num(a, 4)),
            call_open_interest: u.and_then(|a| arr_num(a, 5)),
            call_change: u.and_then(|a| arr_num(a, 6)),
            strike: u.and_then(|a| arr_num(a, 7)),
            call_symbol: u.and_then(|a| arr_str(a, 8)),
            put_buy_volume: d.and_then(|a| arr_num(a, 0)),
            put_buy_price: d.and_then(|a| arr_num(a, 1)),
            put_last_price: d.and_then(|a| arr_num(a, 2)),
            put_sell_price: d.and_then(|a| arr_num(a, 3)),
            put_sell_volume: d.and_then(|a| arr_num(a, 4)),
            put_open_interest: d.and_then(|a| arr_num(a, 5)),
            put_change: d.and_then(|a| arr_num(a, 6)),
            put_symbol: d.and_then(|a| arr_str(a, 7)),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Daily
// ---------------------------------------------------------------------------

/// A single daily OHLCV bar for a CFFEX index-option contract (Sina).
///
/// Mirrors akshare's `option_cffex_{hs300,sz50,zz1000}_daily_sina` column order
/// `date, open, high, low, close, volume` (the upstream payload is positional
/// `[open, high, low, close, volume, date]`; akshare renames to this order).
#[derive(Debug, Clone, serde::Serialize)]
pub struct OptionCffexDailyRow {
    /// 日期 (date)
    pub date: Option<String>,
    /// 开盘价 (open)
    pub open: Option<f64>,
    /// 最高价 (high)
    pub high: Option<f64>,
    /// 最低价 (low)
    pub low: Option<f64>,
    /// 收盘价 (close)
    pub close: Option<f64>,
    /// 成交量 (volume)
    pub volume: Option<f64>,
}

/// CFFEX 上证50 index-option daily history for `symbol` (default `ho2303P2350`).
pub async fn option_cffex_sz50_daily_sina(
    client: &Client,
    symbol: &str,
) -> Result<Vec<OptionCffexDailyRow>> {
    cffex_daily(client, symbol).await
}

/// CFFEX 沪深300 index-option daily history for `symbol` (default `io2202P4350`).
pub async fn option_cffex_hs300_daily_sina(
    client: &Client,
    symbol: &str,
) -> Result<Vec<OptionCffexDailyRow>> {
    cffex_daily(client, symbol).await
}

/// CFFEX 中证1000 index-option daily history for `symbol` (default `mo2208P6200`).
pub async fn option_cffex_zz1000_daily_sina(
    client: &Client,
    symbol: &str,
) -> Result<Vec<OptionCffexDailyRow>> {
    cffex_daily(client, symbol).await
}

async fn cffex_daily(client: &Client, symbol: &str) -> Result<Vec<OptionCffexDailyRow>> {
    let now = chrono::Local::now();
    let cb = format!("_{symbol}{}_{}_{}", now.year(), now.month(), now.day());
    let url = format!("{DAYLINE_URL}{cb}=/FutureOptionAllService.getOptionDayline");
    let params = [("symbol", symbol)];
    let text = client
        .get_text(SOURCE, "option_cffex_daily", &url, &params, None)
        .await?;
    let v = extract_json_array(&text)?;
    parse_cffex_daily(&v)
}

/// Parse Sina's `FutureOptionAllService.getOptionDayline` payload (array of
/// positional `[open, high, low, close, volume, date]` rows) into daily rows.
pub(crate) fn parse_cffex_daily(resp: &Value) -> Result<Vec<OptionCffexDailyRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE,
        message: "dayline payload is not an array".into(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let row = item.as_array().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "dayline row is not an array".into(),
        })?;
        out.push(OptionCffexDailyRow {
            date: arr_str(row, 5),
            open: arr_num(row, 0),
            high: arr_num(row, 1),
            low: arr_num(row, 2),
            close: arr_num(row, 3),
            volume: arr_num(row, 4),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Extract the top-level JSON object from a Sina response that may carry
/// trailing junk (akshare: `text[text.find("{") : text.rfind("}")+1]`).
fn extract_json_object(text: &str) -> Result<Value> {
    let s = text.trim();
    let start = s.find('{').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE,
        message: "response missing '{'".into(),
    })?;
    let end = s.rfind('}').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE,
        message: "response missing '}'".into(),
    })?;
    serde_json::from_str(&s[start..=end]).map_err(Error::Json)
}

/// Extract the JSON array from a Sina JSONP response (`var <cb>=[...];`),
/// replicating akshare's `text[text.find("[") : text.rfind("]")+1]`.
fn extract_json_array(text: &str) -> Result<Value> {
    let s = text.trim();
    let start = s.find('[').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE,
        message: "dayline response missing '['".into(),
    })?;
    let end = s.rfind(']').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE,
        message: "dayline response missing ']'".into(),
    })?;
    serde_json::from_str(&s[start..=end]).map_err(Error::Json)
}

/// Numeric element of a positional row array (number or numeric string).
fn arr_num(arr: &[Value], i: usize) -> Option<f64> {
    match arr.get(i) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// String element of a positional row array (string or number rendered).
fn arr_str(arr: &[Value], i: usize) -> Option<String> {
    match arr.get(i) {
        Some(Value::String(s)) => Some(s.to_string()),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    fn fixture_text(name: &str) -> String {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        std::fs::read_to_string(p).unwrap()
    }

    fn approx(a: Option<f64>, b: f64) -> bool {
        a.is_some_and(|x| (x - b).abs() < 1e-6)
    }

    #[test]
    fn parses_cffex_spot_fixture() {
        let v = fixture("option_cffex_wv_spot.json");
        let rows = parse_cffex_spot(&v).unwrap();
        assert_eq!(rows.len(), 2);

        // Row 0 — call side.
        let r = &rows[0];
        assert!(approx(r.call_buy_volume, 120.0));
        assert!(approx(r.call_buy_price, 0.051));
        assert!(approx(r.call_last_price, 0.052));
        assert!(approx(r.call_sell_price, 0.053));
        assert!(approx(r.call_sell_volume, 80.0));
        assert!(approx(r.call_open_interest, 1500.0));
        assert!(approx(r.call_change, 0.002));
        assert!(approx(r.strike, 2350.0));
        assert_eq!(r.call_symbol.as_deref(), Some("IO2204C2350"));

        // Row 0 — put side (no strike column).
        assert!(approx(r.put_buy_volume, 90.0));
        assert!(approx(r.put_buy_price, 0.020));
        assert!(approx(r.put_last_price, 0.021));
        assert!(approx(r.put_sell_price, 0.022));
        assert!(approx(r.put_sell_volume, 60.0));
        assert!(approx(r.put_open_interest, 900.0));
        assert!(approx(r.put_change, 0.001));
        assert_eq!(r.put_symbol.as_deref(), Some("IO2204P2350"));

        // Row 1 strikes/symbols.
        assert!(approx(rows[1].strike, 2400.0));
        assert_eq!(rows[1].call_symbol.as_deref(), Some("IO2204C2400"));
        assert_eq!(rows[1].put_symbol.as_deref(), Some("IO2204P2400"));
    }

    #[test]
    fn parses_cffex_spot_extract_object() {
        // Trailing junk after the JSON object must be tolerated (akshare slice).
        let text = r#"{"result":{"data":{"up":[],"down":[]}}}   // trailing comment"#;
        let v = extract_json_object(text).unwrap();
        let rows = parse_cffex_spot(&v).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn parses_cffex_daily_fixture() {
        let text = fixture_text("option_cffex_wv_daily.jsonp");
        let v = extract_json_array(&text).unwrap();
        let rows = parse_cffex_daily(&v).unwrap();
        assert_eq!(rows.len(), 2);

        assert_eq!(rows[0].date.as_deref(), Some("2024-03-01"));
        assert!(approx(rows[0].open, 0.0500));
        assert!(approx(rows[0].high, 0.0520));
        assert!(approx(rows[0].low, 0.0490));
        assert!(approx(rows[0].close, 0.0510));
        assert!(approx(rows[0].volume, 12345.0));

        assert_eq!(rows[1].date.as_deref(), Some("2024-03-04"));
        assert!(approx(rows[1].close, 0.0540));
        assert!(approx(rows[1].volume, 23456.0));
    }
}
