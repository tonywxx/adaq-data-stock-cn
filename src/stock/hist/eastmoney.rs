use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};
use crate::core::json::*;
use crate::stock::hist::HistRow;

const UT: &str = "7eea3edcaed734bea9cbfc24409ed989";
const BASE: &str = "https://push2his.eastmoney.com/api/qt/stock/kline/get";

const PERIOD_MAP: &[(&str, &str)] = &[("daily", "101"), ("weekly", "102"), ("monthly", "103")];

const ADJUST_MAP: &[(&str, &str)] = &[("qfq", "1"), ("hfq", "2"), ("", "0")];

/// Per-symbol historical OHLC from Eastmoney (`stock_zh_a_hist`).
///
/// Uses the Eastmoney `push2his` kline API (static `ut`, no JS signing — ADR-0005).
/// `period` is one of daily/weekly/monthly; `adjust` is "" (none) / qfq / hfq.
pub async fn daily(
    client: &Client,
    symbol: &str,
    period: &str,
    adjust: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<HistRow>> {
    let klt = period_map(period)?;
    let fqt = adjust_map(adjust)?;
    let market_code: &str = if symbol.starts_with('6') { "1" } else { "0" };
    let secid = format!("{market_code}.{symbol}");

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
        .get_json(SOURCE_EASTMONEY, "stock_zh_a_hist", BASE, &params)
        .await?;
    parse_klines(&v)
}

pub(crate) fn parse_klines(resp: &Value) -> Result<Vec<HistRow>> {
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
        if parts.len() < 10 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: format!("kline has {} fields, expected >= 10", parts.len()),
            });
        }
        out.push(HistRow {
            symbol: String::new(),
            date: parts[0].to_string(),
            open: parse_f64_str(parts[1]),
            close: parse_f64_str(parts[2]),
            high: parse_f64_str(parts[3]),
            low: parse_f64_str(parts[4]),
            volume: parse_f64_str(parts[5]),
            amount: parse_f64_str(parts[6]),
            pct_change: parse_f64_str(parts[8]),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_eastmoney_hist_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/stock_zh_a_hist_em.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_klines(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2025-01-02");
        assert_eq!(rows[0].open, Some(10.10));
        assert_eq!(rows[0].close, Some(10.50));
        assert_eq!(rows[0].high, Some(10.80));
        assert_eq!(rows[0].low, Some(9.90));
        assert_eq!(rows[0].volume, Some(123456.0));
        assert_eq!(rows[0].amount, Some(1300000.0));
        assert_eq!(rows[0].pct_change, Some(2.31));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].date, "2025-01-03");
        assert_eq!(rows[1].close, Some(10.20));
    }
}
