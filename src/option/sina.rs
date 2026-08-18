//! Sina option-market endpoints (akshare `option_finance_sina.py`).
//!
//! Ports of akshare Sina option functions that are pure HTTP (JSON/JSONP,
//! no HTML scraping, JS signing, tokens or cookies).
//!
//! Only the CFFEX index-option **daily** dayline is ported here: it is a pure
//! JSONP endpoint and is not already covered by `src/option/extra.rs`.
//!
//! Deferred (see report): the CFFEX list functions scrape HTML via
//! `BeautifulSoup` (not pure HTTP); the CFFEX spot functions are already
//! ported as `option_cffex_spot_sina` in `extra.rs`; the `option_sina.py`
//! functions have no source file in akshare.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use crate::core::json::*;

/// Sina source identifier (defined locally; mirrors `core::client::SOURCE_SINA`).
const SOURCE_SINA: &str = "sina";

// ---------------------------------------------------------------------------
// Sina: CFFEX index-option daily dayline (akshare `option_cffex_*_daily_sina`)
// ---------------------------------------------------------------------------

/// A single daily OHLCV bar for a CFFEX index-option contract (Sina).
///
/// Mirrors akshare `option_cffex_sz50_daily_sina` / `option_cffex_hs300_daily_sina`
/// / `option_cffex_zz1000_daily_sina`, which share one upstream endpoint
/// (`FutureOptionAllService.getOptionDayline`) differing only by the contract
/// `symbol` (e.g. `"ho2303P2350"`, `"io2202P4350"`, `"mo2208P6200"`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CffexOptionDailyRow {
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
    pub source: &'static str,
}

/// Daily OHLCV history for a CFFEX index-option contract from Sina's
/// `FutureOptionAllService.getOptionDayline`
/// (akshare `option_cffex_*_daily_sina`, `option_finance_sina.py:296`).
///
/// `symbol` is the full contract code, e.g. `"ho2303P2350"` (上证50),
/// `"io2202P4350"` (沪深300) or `"mo2208P6200"` (中证1000). The upstream
/// response is JSONP-wrapped (`var <callback>=[...]`), so the wrapper is
/// stripped before parsing.
pub async fn option_cffex_daily(client: &Client, symbol: &str) -> Result<Vec<CffexOptionDailyRow>> {
    let url = format!(
        "https://stock.finance.sina.com.cn/futures/api/jsonp.php/var%20_{symbol}=/FutureOptionAllService.getOptionDayline"
    );
    let params = [("symbol", symbol)];
    let text = client
        .get_text(SOURCE_SINA, "option_cffex_daily", &url, &params, None)
        .await?;
    let v = dayline_to_value(&text)?;
    parse_cffex_daily(&v)
}

/// Extract the JSON array from a Sina JSONP dayline response
/// (`var <callback>=[...];`), returning it as a `Value`.
fn dayline_to_value(text: &str) -> Result<Value> {
    let body = text.trim();
    let start = body.find('[').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "dayline response missing '['".into(),
    })?;
    let end = body.rfind(']').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "dayline response missing ']'".into(),
    })?;
    serde_json::from_str(&body[start..=end]).map_err(Error::Json)
}

pub(crate) fn parse_cffex_daily(resp: &Value) -> Result<Vec<CffexOptionDailyRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "dayline payload is not an array".into(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(CffexOptionDailyRow {
            date: opt_str(item, "date"),
            open: opt_f64(item, "open"),
            high: opt_f64(item, "high"),
            low: opt_f64(item, "low"),
            close: opt_f64(item, "close"),
            volume: opt_f64(item, "volume"),
            source: SOURCE_SINA,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------


/// Extract an `i64` from a field (numeric or numeric string). Currently unused
/// by the ported parsers; retained for parity with sibling modules.
#[allow(dead_code)]
fn inum(v: Option<&Value>) -> Option<i64> {
    match v {
        Some(Value::Number(n)) => n.as_i64(),
        Some(Value::String(s)) => s.trim().parse::<i64>().ok(),
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

    #[test]
    fn parses_option_cffex_daily_fixture() {
        let v = fixture("option_cffex_daily.json");
        let rows = parse_cffex_daily(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, Some("2024-03-01".to_string()));
        assert_eq!(rows[0].open, Some(0.0500));
        assert_eq!(rows[0].high, Some(0.0520));
        assert_eq!(rows[0].low, Some(0.0490));
        assert_eq!(rows[0].close, Some(0.0510));
        assert_eq!(rows[0].volume, Some(12_345.0));
        assert_eq!(rows[1].date, Some("2024-03-04".to_string()));
        assert_eq!(rows[1].close, Some(0.0540));
        assert_eq!(rows[1].volume, Some(23_456.0));
        assert_eq!(rows[0].source, "sina");
    }
}
