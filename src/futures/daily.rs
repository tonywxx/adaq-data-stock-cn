//! Eastmoney daily history for Chinese futures (`futures_hist_em`).
//!
//! Ports akshare `futures_hist_em`: daily/weekly/monthly OHLC klines via the
//! Eastmoney `push2his` kline API (static `ut`, no JS signing — ADR-0005).
//!
//! `symbol` is the Eastmoney secid (`market.code`, e.g. `"114.HR00Y"`), exactly
//! as the upstream `secid` param expects. Resolving a Chinese contract *name*
//! to a secid requires Eastmoney's `futsse-static` symbol table, which is
//! fetched at runtime in akshare; callers resolve that mapping separately and
//! pass the secid here (source-resilient: no extra HTTP dependency).

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

/// Static Eastmoney `ut` token — no JS signing required.
const UT: &str = "7eea3edcaed734bea9cbfc24409ed989";
const BASE: &str = "https://push2his.eastmoney.com/api/qt/stock/kline/get";

const PERIOD_MAP: &[(&str, &str)] = &[("daily", "101"), ("weekly", "102"), ("monthly", "103")];

/// One day of Chinese-futures OHLC from Eastmoney (`futures_hist_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesDailyRow {
    pub symbol: String,
    pub date: String,
    pub open: Option<f64>,
    pub close: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub volume: Option<f64>,
    pub amount: Option<f64>,
    pub amplitude: Option<f64>,
    pub pct_change: Option<f64>,
    pub change: Option<f64>,
    pub open_interest: Option<f64>,
    pub source: &'static str,
}

/// Daily/weekly/monthly history for a Chinese futures contract (Eastmoney `futures_hist_em`).
pub async fn futures_zh_daily(
    client: &Client,
    symbol: &str,
    period: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<FuturesDailyRow>> {
    let klt = period_map(period)?;
    let params = [
        ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8"),
        (
            "fields2",
            "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f62,f63,f64",
        ),
        ("ut", UT),
        ("klt", klt),
        ("fqt", "1"),
        ("secid", symbol),
        ("beg", start_date),
        ("end", end_date),
        ("lmt", "10000"),
        ("iscca", "1"),
        ("forcect", "1"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "futures_zh_daily", BASE, &params)
        .await?;
    parse_klines(&v)
}

pub(crate) fn parse_klines(resp: &Value) -> Result<Vec<FuturesDailyRow>> {
    let data = resp.get("data").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "missing data".into(),
    })?;
    let klines = data
        .get("klines")
        .and_then(|k| k.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.klines".into(),
        })?;
    let symbol = data
        .get("code")
        .and_then(|c| c.as_str())
        .unwrap_or_default()
        .to_string();
    let mut out = Vec::with_capacity(klines.len());
    for line in klines {
        let s = line.as_str().ok_or_else(|| Error::Parse {
            endpoint: "futures_zh_daily",
            message: "kline entry is not a string".into(),
        })?;
        let p: Vec<&str> = s.split(',').collect();
        // fields2 has 14 fields; skip malformed/short rows.
        if p.len() < 13 {
            continue;
        }
        out.push(FuturesDailyRow {
            symbol: symbol.clone(),
            date: p[0].to_string(),
            open: parse_f64(p[1]),
            close: parse_f64(p[2]),
            high: parse_f64(p[3]),
            low: parse_f64(p[4]),
            volume: parse_f64(p[5]),
            amount: parse_f64(p[6]),
            amplitude: parse_f64(p[7]),
            pct_change: parse_f64(p[8]),
            change: parse_f64(p[9]),
            open_interest: parse_f64(p[12]),
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
    fn parses_futures_zh_daily_fixture() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/futures_zh_daily.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_klines(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "HR00Y");
        assert_eq!(rows[0].date, "2024-01-02");
        assert_eq!(rows[0].open, Some(3000.0));
        assert_eq!(rows[0].close, Some(3050.0));
        assert_eq!(rows[0].high, Some(3080.0));
        assert_eq!(rows[0].low, Some(2980.0));
        assert_eq!(rows[0].volume, Some(120000.0));
        assert_eq!(rows[0].amount, Some(3.6e9));
        assert_eq!(rows[0].pct_change, Some(2.31));
        assert_eq!(rows[0].open_interest, Some(150000.0));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].date, "2024-01-03");
        assert_eq!(rows[1].close, Some(3020.0));
    }
}
