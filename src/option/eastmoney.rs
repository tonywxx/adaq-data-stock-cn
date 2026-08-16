use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

/// Static, well-known Eastmoney `ut` token — no JS signing required (ADR-0005).
const UT: &str = "7eea3edcaed734bea9cbfc24409ed989";

const PERIOD_MAP: &[(&str, &str)] = &[("daily", "101"), ("weekly", "102"), ("monthly", "103")];

const ADJUST_MAP: &[(&str, &str)] = &[("qfq", "1"), ("hfq", "2"), ("", "0")];

/// A single day of Eastmoney option OHLC history.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OptionDailyRow {
    pub symbol: String,
    pub date: String,
    pub open: Option<f64>,
    pub close: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub volume: Option<f64>,
    pub amount: Option<f64>,
    pub open_interest: Option<f64>,
    pub source: &'static str,
}

/// A single intraday minute bar from Eastmoney's `trends2` API.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OptionMinuteRow {
    pub secid: String,
    pub time: String,
    pub close: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub volume: Option<f64>,
    pub amount: Option<f64>,
    pub source: &'static str,
}

/// Daily option history from Eastmoney's `push2his` kline API (static `ut`,
/// no JS signing — mirrors `stock_zh_a_hist` but for option secids).
///
/// `symbol` is the option code (e.g. `"10003720"`, an SSE ETF option). The
/// Eastmoney `secid` is derived as `"<market>.<code>"`; SSE-style codes
/// (`1`/`5`/`6` prefix) map to market `1`, otherwise `0`.
pub async fn option_daily(
    client: &Client,
    symbol: &str,
    period: &str,
    adjust: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<OptionDailyRow>> {
    let klt = period_map(period)?;
    let fqt = adjust_map(adjust)?;
    let secid = option_secid(symbol);
    let params = [
        ("fields1", "f1,f2,f3,f4,f5,f6"),
        (
            "fields2",
            "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f116",
        ),
        ("ut", UT),
        ("klt", klt),
        ("fqt", fqt),
        ("secid", &secid),
        ("beg", start_date),
        ("end", end_date),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "option_daily",
            "https://push2his.eastmoney.com/api/qt/stock/kline/get",
            &params,
        )
        .await?;
    let mut rows = parse(&v)?;
    for r in &mut rows {
        r.symbol = symbol.to_string();
    }
    Ok(rows)
}

/// Intraday minute bars for an option from Eastmoney's `trends2` API.
///
/// `secid` is the full Eastmoney security id (e.g. `"1.10003720"` for an SSE
/// ETF option, or `"151.MO2404-P-4450"` for a CFFEX futures option). Obtain it
/// from `option_current_em` (Eastmoney option list). The endpoint returns a
/// JSONP envelope, so we strip the callback wrapper before parsing.
pub async fn option_minute(client: &Client, secid: &str) -> Result<Vec<OptionMinuteRow>> {
    let params = [
        ("secid", secid),
        (
            "fields1",
            "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f11,f12,f13,f14,f17",
        ),
        ("fields2", "f51,f53,f54,f55,f56,f57,f58"),
        ("iscr", "0"),
        ("iscca", "0"),
        ("ut", "f057cbcbce2a86e2866ab8877db1d059"),
        ("ndays", "1"),
        ("cb", "quotepushdata1"),
    ];
    let text = client
        .get_text(
            SOURCE_EASTMONEY,
            "option_minute",
            "https://push2.eastmoney.com/api/qt/stock/trends2/get",
            &params,
            None,
        )
        .await?;
    let inner = strip_jsonp(&text).ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "trends2 response is not wrapped JSONP".into(),
    })?;
    let v: Value = serde_json::from_str(inner).map_err(|e| Error::Parse {
        endpoint: "option_minute",
        message: e.to_string(),
    })?;
    parse_minute(&v, secid)
}

pub(crate) fn parse(resp: &Value) -> Result<Vec<OptionDailyRow>> {
    let klines = resp
        .get("data")
        .and_then(|d| d.get("klines"))
        .and_then(|k| k.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.klines".into(),
        })?;
    let mut out = Vec::with_capacity(klines.len());
    for line in klines {
        let s = line.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "kline entry is not a string".into(),
        })?;
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() < 7 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: format!("kline has {} fields, expected >= 7", parts.len()),
            });
        }
        out.push(OptionDailyRow {
            symbol: String::new(),
            date: parts[0].to_string(),
            open: parse_f64(parts[1]),
            close: parse_f64(parts[2]),
            high: parse_f64(parts[3]),
            low: parse_f64(parts[4]),
            volume: parse_f64(parts[5]),
            amount: parse_f64(parts[6]),
            open_interest: parts.get(11).and_then(|v| parse_f64(v)),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

pub(crate) fn parse_minute(resp: &Value, secid: &str) -> Result<Vec<OptionMinuteRow>> {
    let trends = resp
        .get("data")
        .and_then(|d| d.get("trends"))
        .and_then(|t| t.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.trends".into(),
        })?;
    let mut out = Vec::with_capacity(trends.len());
    for line in trends {
        let s = line.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "trend entry is not a string".into(),
        })?;
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() < 6 {
            continue;
        }
        out.push(OptionMinuteRow {
            secid: secid.to_string(),
            time: parts[0].to_string(),
            close: parse_f64(parts[1]),
            high: parse_f64(parts[2]),
            low: parse_f64(parts[3]),
            volume: parse_f64(parts[4]),
            amount: parse_f64(parts[5]),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

/// Extract the inner JSON from a `callback({...})` JSONP envelope.
fn strip_jsonp(text: &str) -> Option<&str> {
    let open = text.find('(')?;
    let close = text.rfind(')')?;
    if close <= open {
        return None;
    }
    Some(text[open + 1..close].trim())
}

fn option_secid(symbol: &str) -> String {
    // Eastmoney secid = "<market>.<code>". SSE-style option codes (prefix 1/5/6)
    // belong to market 1 (Shanghai); everything else defaults to market 0 (Shenzhen).
    let market = if symbol.starts_with('1') || symbol.starts_with('5') || symbol.starts_with('6') {
        "1"
    } else {
        "0"
    };
    format!("{market}.{symbol}")
}

fn period_map(period: &str) -> Result<&'static str> {
    PERIOD_MAP
        .iter()
        .find(|(k, _)| *k == period)
        .map(|(_, v)| *v)
        .ok_or_else(|| Error::InvalidParam(format!("unknown period: {period}")))
}

fn adjust_map(adjust: &str) -> Result<&'static str> {
    ADJUST_MAP
        .iter()
        .find(|(k, _)| *k == adjust)
        .map(|(_, v)| *v)
        .ok_or_else(|| Error::InvalidParam(format!("unknown adjust: {adjust}")))
}

fn parse_f64(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        t.parse::<f64>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_eastmoney_option_daily_fixture() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/option_daily.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2025-01-02");
        assert_eq!(rows[0].open, Some(0.1234));
        assert_eq!(rows[0].close, Some(0.1356));
        assert_eq!(rows[0].high, Some(0.1400));
        assert_eq!(rows[0].low, Some(0.1200));
        assert_eq!(rows[0].volume, Some(54321.0));
        assert_eq!(rows[0].amount, Some(7200000.0));
        assert_eq!(rows[0].open_interest, Some(98765.0));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].date, "2025-01-03");
        assert_eq!(rows[1].close, Some(0.1310));
    }

    #[test]
    fn parses_eastmoney_option_minute_fixture() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/option_minute.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_minute(&v, "1.10003720").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].time, "2025-01-02 09:31");
        assert_eq!(rows[0].close, Some(0.1320));
        assert_eq!(rows[0].high, Some(0.1330));
        assert_eq!(rows[0].low, Some(0.1310));
        assert_eq!(rows[0].volume, Some(120.0));
        assert_eq!(rows[0].amount, Some(15840.0));
        assert_eq!(rows[0].secid, "1.10003720");
        assert_eq!(rows[0].source, "eastmoney");
    }

    #[test]
    fn derives_secid_for_sse_option() {
        assert_eq!(option_secid("10003720"), "1.10003720");
        assert_eq!(option_secid("9000xxxx"), "0.9000xxxx");
    }
}
