//! Cross-market stock data (Hong Kong / US / AH connect) — Rust port of akshare.
//!
//! Mirrors the Eastmoney-backed public functions from akshare's
//! `stock_feature/stock_hist_em.py`:
//!
//! - [`hk::stock_hk_spot_em`] — HK real-time spot quotes (`stock_hk_spot_em`)
//! - [`hk::stock_hk_hist`] — HK daily/weekly/monthly history (`stock_hk_hist`)
//! - [`us::stock_us_spot_em`] — US real-time spot quotes (`stock_us_spot_em`)
//! - [`us::stock_us_hist`] — US daily/weekly/monthly history (`stock_us_hist`)
//!
//! All endpoints use Eastmoney's static-`ut` `clist` / `push2his` kline APIs — no
//! JS signing (ADR-0005). Symbols mirror akshare: HK is the bare 5-digit code
//! (e.g. `"00593"`), US is the `"<market>.<ticker>"` form (e.g. `"105.MSFT"`).
//!
//! NOTE: this module is not wired into `crate::stock` yet. The lead must add
//! `pub mod cross;` to `src/stock/mod.rs`.

pub mod hk;
pub mod us;

pub use hk::{HkHistRow, HkSpotRow, stock_hk_hist, stock_hk_spot_em};
pub use us::{UsHistRow, UsSpotRow, stock_us_hist, stock_us_spot_em};

use serde_json::Value;

use crate::core::client::SOURCE_EASTMONEY;
use crate::core::error::{Error, Result};

/// Eastmoney `push2his` kline endpoint (used by both HK and US history).
pub(crate) const BASE_HIS: &str = "https://push2his.eastmoney.com/api/qt/stock/kline/get";

/// One parsed OHLCV kline point, shared by both HK and US history parsers.
pub(crate) struct Kline {
    pub date: String,
    pub open: Option<f64>,
    pub close: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub volume: Option<f64>,
    pub amount: Option<f64>,
    pub pct_change: Option<f64>,
}

/// Parse an Eastmoney `push2his` kline payload into [`Kline`]s.
///
/// `endpoint` is used for error context. `klines` is the `data.klines` array of
/// comma-separated strings (f51..f61). Malformed entries are skipped.
pub(crate) fn parse_klines(resp: &Value, endpoint: &'static str) -> Result<Vec<Kline>> {
    let klines = resp
        .get("data")
        .and_then(|d| d.get("klines"))
        .and_then(|k| k.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: format!("missing data.klines for {endpoint}"),
        })?;
    let mut out = Vec::with_capacity(klines.len());
    for line in klines {
        let s = match line.as_str() {
            Some(s) => s,
            None => continue, // skip malformed row
        };
        let p: Vec<&str> = s.split(',').collect();
        if p.len() < 7 {
            continue; // skip malformed row
        }
        out.push(Kline {
            date: p[0].to_string(),
            open: parse_f64(p[1]),
            close: parse_f64(p[2]),
            high: parse_f64(p[3]),
            low: parse_f64(p[4]),
            volume: parse_f64(p[5]),
            amount: parse_f64(p[6]),
            pct_change: if p.len() > 8 { parse_f64(p[8]) } else { None },
        });
    }
    Ok(out)
}

/// Map akshare `period` to Eastmoney `klt` (daily/weekly/monthly).
pub(crate) fn period_map(period: &str) -> Result<&'static str> {
    match period {
        "daily" => Ok("101"),
        "weekly" => Ok("102"),
        "monthly" => Ok("103"),
        other => Err(Error::InvalidParam(format!("unknown period: {other}"))),
    }
}

/// Map akshare `adjust` to Eastmoney `fqt` (qfq/hfq/none).
pub(crate) fn adjust_map(adjust: &str) -> Result<&'static str> {
    match adjust {
        "qfq" => Ok("1"),
        "hfq" => Ok("2"),
        "" => Ok("0"),
        other => Err(Error::InvalidParam(format!("unknown adjust: {other}"))),
    }
}

/// Read a string/number field from a clist `diff` item, normalized to `String`.
///
/// Eastmoney returns codes/markets (`f12`, `f13`) sometimes as numbers, so we
/// accept both number and string forms (mirrors akshare's `.astype(str)`).
pub(crate) fn fstr(item: &Value, k: &str) -> String {
    match item.get(k) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

/// Read a numeric field from a clist `diff` item (number or numeric string).
pub(crate) fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    })
}

/// Parse a single kline field string into `f64`, tolerating empties.
pub(crate) fn parse_f64(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        t.parse::<f64>().ok()
    }
}
