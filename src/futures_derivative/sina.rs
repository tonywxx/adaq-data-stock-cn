//! Sina futures main-continuous daily data, ported from
//! `akshare/futures_derivative/futures_index_sina.py`.
//!
//! | Rust fn               | akshare source                     | transport / notes              |
//! | --------------------- | ---------------------------------- | ------------------------------ |
//! | `futures_main_sina`   | `futures_index_sina.py:103`        | Sina JSONP (`getDailyKLine`)   |
//!
//! ## DEFERRED
//!
//! - `futures_display_main_sina` (`futures_index_sina.py:89`) — depends on
//!   `match_main_contract` / `zh_subscribe_exchange_symbol`, which fetch a JS
//!   file and parse it with `akshare.utils.demjson` (lenient/non-strict JSON).
//!   Rust's `serde_json` is strict and no lenient parser is available without a
//!   new crate, so this is deferred.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// Sina source identifier.
const SOURCE_SINA: &str = "sina";

/// Sina main-continuous daily-KLine JSONP endpoint.
const SINA_MAIN_URL: &str = "https://stock2.finance.sina.com.cn/futures/api/jsonp.php/var%20_{symbol}{trade_date}=/InnerFuturesNewService.getDailyKLine";

/// akshare hardcodes the callback's date stamp; the payload is the full history.
const CALLBACK_DATE: &str = "20210817";

/// One Sina main-continuous daily bar (`futures_main_sina`).
///
/// akshare columns: 日期, 开盘价, 最高价, 最低价, 收盘价, 成交量, 持仓量, 动态结算价.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SinaMainRow {
    /// Date `YYYY-MM-DD`. akshare `日期`.
    pub date: Option<String>,
    /// Open. akshare `开盘价`.
    pub open: Option<f64>,
    /// High. akshare `最高价`.
    pub high: Option<f64>,
    /// Low. akshare `最低价`.
    pub low: Option<f64>,
    /// Close. akshare `收盘价`.
    pub close: Option<f64>,
    /// Volume. akshare `成交量`.
    pub volume: Option<f64>,
    /// Open interest. akshare `持仓量`.
    pub open_interest: Option<f64>,
    /// Dynamic settlement price. akshare `动态结算价`.
    pub settle: Option<f64>,
}

/// Sina main-continuous daily history (`futures_main_sina`).
///
/// `symbol` is the main-continuous code from `futures_display_main_sina`
/// (e.g. `"V0"`, `"CF0"`). `start_date` / `end_date` are `YYYYMMDD` bounds;
/// pass `None` to keep akshare's full default range. The Sina JSONP payload
/// is `var _SYMBOL...=[ [date, o, h, l, c, v, oi, settle], ... ]`; the wrapper
/// is stripped before parsing.
pub async fn futures_main_sina(
    client: &Client,
    symbol: &str,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<Vec<SinaMainRow>> {
    let url = format!("{SINA_MAIN_URL}?symbol={symbol}&_={CALLBACK_DATE}");
    let text = client
        .get_text(SOURCE_SINA, "futures_main_sina", &url, &[], None)
        .await?;
    let arr = strip_jsonp_array(&text)?;
    let mut rows = parse_sina_main(&arr)?;
    if let (Some(s), Some(e)) = (start_date, end_date) {
        rows.retain(|r| {
            if let Some(d) = &r.date {
                d.as_str() >= s && d.as_str() <= e
            } else {
                false
            }
        });
    }
    Ok(rows)
}

/// Extract the JSON array body from a Sina JSONP response
/// (`var <cb>=[ ... ];`), returning it as a `Value`.
fn strip_jsonp_array(text: &str) -> Result<Value> {
    let body = text.trim();
    let start = body.find('[').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "main KLine response missing '['".into(),
    })?;
    let end = body.rfind(']').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "main KLine response missing ']'".into(),
    })?;
    serde_json::from_str(&body[start..=end]).map_err(Error::Json)
}

/// Parse the Sina main-continuous daily array (array of 8-element rows).
pub(crate) fn parse_sina_main(resp: &Value) -> Result<Vec<SinaMainRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "main KLine payload is not an array".into(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for row in arr {
        let cells = row.as_array().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "main KLine row is not an array".into(),
        })?;
        if cells.len() < 8 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_SINA,
                message: format!("expected 8 columns, got {}", cells.len()),
            });
        }
        out.push(SinaMainRow {
            date: cell_str(&cells[0]),
            open: cell_num(&cells[1]),
            high: cell_num(&cells[2]),
            low: cell_num(&cells[3]),
            close: cell_num(&cells[4]),
            volume: cell_num(&cells[5]),
            open_interest: cell_num(&cells[6]),
            settle: cell_num(&cells[7]),
        });
    }
    Ok(out)
}

/// Extract a string cell from an array element.
fn cell_str(v: &Value) -> Option<String> {
    v.as_str().map(str::to_string)
}

/// Extract a numeric cell, tolerating numeric strings.
fn cell_num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Value {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        // The fixture stores the raw JSONP text.
        let text = std::fs::read_to_string(p).unwrap();
        strip_jsonp_array(&text).unwrap()
    }

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    #[test]
    fn parse_sina_main_ok() {
        let arr = fixture("futures_main_sina.json");
        let rows = parse_sina_main(&arr).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, Some("2024-01-02".into()));
        assert!(approx(rows[0].open, 100.0));
        assert!(approx(rows[0].high, 102.5));
        assert!(approx(rows[0].low, 99.0));
        assert!(approx(rows[0].close, 101.2));
        assert!(approx(rows[0].volume, 12345.0));
        assert!(approx(rows[0].open_interest, 54321.0));
        assert!(approx(rows[0].settle, 101.0));
        assert_eq!(rows[1].date, Some("2024-01-03".into()));
        assert!(approx(rows[1].close, 102.0));
    }
}
