use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};
use crate::fund::{fnum, fstr, parse_f64};

const UT: &str = "bd1d9ddb04089700cf9c27f6f7426281";
const SPOT_URL: &str = "https://push2delay.eastmoney.com/api/qt/clist/get";
/// ETF boards: b:MK0021 (沪股通ETF), b:MK0022, b:MK0023, b:MK0024, b:MK0827 (跨境ETF).
const SPOT_FS: &str = "b:MK0021,b:MK0022,b:MK0023,b:MK0024,b:MK0827";
const SPOT_FIELDS: &str = "f1,f2,f3,f4,f5,f6,f12,f13,f14,f15,f16,f17,f18";
const SPOT_PAGE_SIZE: u32 = 1000;

const HIST_URL: &str = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
const HIST_UT: &str = "7eea3edcaed734bea9cbfc24409ed989";
const HIST_PERIOD_MAP: &[(&str, &str)] = &[("daily", "101"), ("weekly", "102"), ("monthly", "103")];
const HIST_ADJUST_MAP: &[(&str, &str)] = &[("", "0"), ("qfq", "1"), ("hfq", "2")];

/// Canonical ETF real-time spot quote (akshare `fund_etf_spot_em`), Eastmoney.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EtfSpotRow {
    pub code: String,
    pub name: String,
    pub price: Option<f64>,
    pub pct_change: Option<f64>,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub pre_close: Option<f64>,
    pub volume: Option<f64>,
    pub amount: Option<f64>,
    pub source: &'static str,
}

/// Real-time ETF spot quotes from Eastmoney (`fund_etf_spot_em`).
///
/// Replicates akshare's `push2delay` `clist/get` request (static `ut`, no JS
/// signing). Eastmoney paginates; we walk pages until `data.total` is covered.
pub async fn fund_etf_spot_em(client: &Client) -> Result<Vec<EtfSpotRow>> {
    let mut out = Vec::new();
    let mut pn: u32 = 1;
    loop {
        let pn_s = pn.to_string();
        let pz_s = SPOT_PAGE_SIZE.to_string();
        let params = [
            ("pn", pn_s.as_str()),
            ("pz", pz_s.as_str()),
            ("po", "1"),
            ("np", "1"),
            ("ut", UT),
            ("fltt", "2"),
            ("invt", "2"),
            ("wbp2u", "|0|0|0|web"),
            ("fid", "f12"),
            ("fs", SPOT_FS),
            ("fields", SPOT_FIELDS),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, "fund_etf_spot_em", SPOT_URL, &params)
            .await?;
        let diff = v
            .get("data")
            .and_then(|d| d.get("diff"))
            .and_then(|d| d.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing data.diff".into(),
            })?;
        if diff.is_empty() {
            break;
        }
        out.extend(parse_spot(&v)?);
        let total = v
            .get("data")
            .and_then(|d| d.get("total"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        if (pn as u64) * SPOT_PAGE_SIZE as u64 >= total {
            break;
        }
        pn += 1;
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    }
    Ok(out)
}

pub(crate) fn parse_spot(resp: &Value) -> Result<Vec<EtfSpotRow>> {
    let diff = resp
        .get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff".into(),
        })?;
    let mut out = Vec::with_capacity(diff.len());
    for item in diff {
        let code = fstr(item, "f12");
        if code.is_empty() {
            continue; // skip malformed rows
        }
        out.push(EtfSpotRow {
            code,
            name: fstr(item, "f14"),
            price: fnum(item, "f2"),
            pct_change: fnum(item, "f3"),
            open: fnum(item, "f17"),
            high: fnum(item, "f15"),
            low: fnum(item, "f16"),
            pre_close: fnum(item, "f18"),
            volume: fnum(item, "f5"),
            amount: fnum(item, "f6"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

/// Canonical per-symbol ETF daily history (akshare `fund_etf_hist_em`), Eastmoney.
///
/// HistRow-like: `symbol` + OHLC + volume/amount + pct_change + source.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EtfHistRow {
    pub symbol: String,
    pub date: String,
    pub open: Option<f64>,
    pub close: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub volume: Option<f64>,
    pub amount: Option<f64>,
    pub pct_change: Option<f64>,
    pub source: &'static str,
}

/// ETF daily/weekly/monthly history from Eastmoney (`fund_etf_hist_em`).
///
/// Uses the Eastmoney `push2his` kline API (static `ut`, no JS signing — ADR-0005).
/// `period` is one of daily/weekly/monthly; `adjust` is "" (none) / qfq / hfq.
pub async fn fund_etf_hist_em(
    client: &Client,
    symbol: &str,
    period: &str,
    adjust: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<EtfHistRow>> {
    let klt = period_map(period)?;
    let fqt = adjust_map(adjust)?;
    let market: &str = if symbol.starts_with('5') || symbol.starts_with('6') {
        "1"
    } else {
        "0"
    };
    let secid = format!("{market}.{symbol}");
    let params = [
        ("fields1", "f1,f2,f3,f4,f5,f6"),
        (
            "fields2",
            "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f116",
        ),
        ("ut", HIST_UT),
        ("klt", klt),
        ("fqt", fqt),
        ("secid", &secid),
        ("beg", start_date),
        ("end", end_date),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "fund_etf_hist_em", HIST_URL, &params)
        .await?;
    parse_hist(&v, symbol)
}

pub(crate) fn parse_hist(resp: &Value, symbol: &str) -> Result<Vec<EtfHistRow>> {
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
        let s = match line.as_str() {
            Some(s) => s,
            None => continue, // skip malformed rows
        };
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() < 10 {
            continue; // skip malformed rows
        }
        out.push(EtfHistRow {
            symbol: symbol.to_string(),
            date: parts[0].to_string(),
            open: parse_f64(parts[1]),
            close: parse_f64(parts[2]),
            high: parse_f64(parts[3]),
            low: parse_f64(parts[4]),
            volume: parse_f64(parts[5]),
            amount: parse_f64(parts[6]),
            pct_change: parse_f64(parts[8]),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

fn period_map(period: &str) -> Result<&'static str> {
    HIST_PERIOD_MAP
        .iter()
        .find(|(k, _)| *k == period)
        .map(|(_, v)| *v)
        .ok_or_else(|| Error::InvalidParam(format!("unknown period: {period}")))
}

fn adjust_map(adjust: &str) -> Result<&'static str> {
    HIST_ADJUST_MAP
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
    fn parses_etf_spot_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/fund_etf_spot_em.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_spot(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "510300");
        assert_eq!(rows[0].name, "沪深300ETF");
        assert_eq!(rows[0].price, Some(3.95));
        assert_eq!(rows[0].pct_change, Some(1.28));
        assert_eq!(rows[0].open, Some(3.92));
        assert_eq!(rows[0].high, Some(3.98));
        assert_eq!(rows[0].low, Some(3.90));
        assert_eq!(rows[0].pre_close, Some(3.90));
        assert_eq!(rows[0].volume, Some(12_345_678.0));
        assert_eq!(rows[0].amount, Some(48_765_432.0));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].code, "159915");
        assert_eq!(rows[1].name, "创业板ETF");
        assert_eq!(rows[1].pct_change, Some(-0.46));
    }

    #[test]
    fn parses_etf_hist_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/fund_etf_hist_em.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_hist(&v, "510300").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "510300");
        assert_eq!(rows[0].date, "2025-01-02");
        assert_eq!(rows[0].open, Some(10.10));
        assert_eq!(rows[0].close, Some(10.50));
        assert_eq!(rows[0].high, Some(10.80));
        assert_eq!(rows[0].low, Some(9.90));
        assert_eq!(rows[0].volume, Some(123456.0));
        assert_eq!(rows[0].amount, Some(1_300_000.0));
        assert_eq!(rows[0].pct_change, Some(2.31));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].date, "2025-01-03");
        assert_eq!(rows[1].close, Some(10.20));
    }
}
