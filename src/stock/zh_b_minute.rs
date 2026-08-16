//! 新浪-B股-分钟行情 (Sina B-share minute K-line, non-adjusted).
//!
//! Ports `akshare/stock/stock_zh_b_sina.py:281` (`stock_zh_b_minute`).
//! The Sina endpoint returns JSONP (`...=(...)`); this port unwraps the
//! padding and parses the inner JSON array of OHLCV bars. Only the
//! non-adjusted (`adjust=""`) path is implemented — the qfq/hfq paths require
//! a second daily-quote fetch + merge that is out of scope here.
//!
//! | Rust fn | akshare fn | source |
//! |---|---|---|
//! | `stock_zh_b_minute` | `stock_zh_b_minute` | `akshare/stock/stock_zh_b_sina.py:281` |
//!
//! ## DEFERRED
//! None (qfq/hfq adjust paths omitted — see doc above).

use serde_json::Value;

use crate::core::client::{Client, SOURCE_SINA};
use crate::core::error::{Error, Result};

const URL: &str =
    "https://quotes.sina.cn/cn/api/jsonp_v2.php/=/CN_MarketDataService.getKLineData";

fn num_of(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => {
            let t = s.trim().replace(',', "");
            if t.is_empty() {
                None
            } else {
                t.parse().ok()
            }
        }
        _ => None,
    }
}

/// Parse the inner JSONP array (already unwrapped) into OHLCV minute bars.
pub(crate) fn parse_zh_b_minute(arr: &Value) -> Result<Vec<ZhBMinuteRow>> {
    let arr = arr.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "expected JSON array".into(),
    })?;
    if arr.is_empty() {
        return Ok(Vec::new());
    }
    Ok(arr
        .iter()
        .map(|o| ZhBMinuteRow {
            day: o.get("day").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            open: num_of(o.get("open")),
            high: num_of(o.get("high")),
            low: num_of(o.get("low")),
            close: num_of(o.get("close")),
            volume: num_of(o.get("volume")),
        })
        .collect())
}

/// Strip the Sina JSONP padding (`prefix=(` ... `);`) and parse the inner array.
fn unwrap_jsonp(text: &str) -> Result<Value> {
    let start = text.find("=(").map(|i| i + 2).unwrap_or(0);
    let end = text.rfind(");").unwrap_or(text.len());
    let inner = &text[start..end];
    serde_json::from_str(inner).map_err(Error::Json)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ZhBMinuteRow {
    /// 时间 (`day`, e.g. `2024-01-02 09:31:00`).
    pub day: String,
    /// 开盘 (`open`).
    pub open: Option<f64>,
    /// 最高 (`high`).
    pub high: Option<f64>,
    /// 最低 (`low`).
    pub low: Option<f64>,
    /// 收盘 (`close`).
    pub close: Option<f64>,
    /// 成交量 (`volume`).
    pub volume: Option<f64>,
}

/// Port of `stock_zh_b_minute(symbol, period, adjust="")` (non-adjusted only).
pub async fn stock_zh_b_minute(
    client: &Client,
    symbol: &str,
    period: &str,
) -> Result<Vec<ZhBMinuteRow>> {
    let params = [
        ("symbol", symbol),
        ("scale", period),
        ("datalen", "1970"),
    ];
    let text = client
        .get_text(SOURCE_SINA, "stock_zh_b_minute", URL, &params, None)
        .await?;
    let arr = unwrap_jsonp(&text)?;
    parse_zh_b_minute(&arr)
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

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    #[test]
    fn parses_zh_b_minute() {
        let rows = parse_zh_b_minute(&fixture("stock_zh_b_minute.json")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].day, "2024-01-02 09:31:00");
        assert!(approx(rows[0].open, 0.782));
        assert!(approx(rows[0].close, 0.785));
        assert!(approx(rows[0].volume, 123456.0));
        assert_eq!(rows[2].day, "2024-01-02 09:33:00");
    }

    #[test]
    fn unwraps_jsonp_padding() {
        let text = "var x=([{\"day\":\"2024-01-02 09:31:00\",\"open\":\"0.78\"}]);";
        let arr = unwrap_jsonp(text).unwrap();
        let rows = parse_zh_b_minute(&arr).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].day, "2024-01-02 09:31:00");
    }
}
