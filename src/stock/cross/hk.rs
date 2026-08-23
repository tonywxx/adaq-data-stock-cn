use serde_json::Value;

use super::{BASE_HIS, adjust_map, fnum, fstr, parse_klines, period_map};
use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

/// Static, well-known Eastmoney `ut` token — no JS signing required (ADR-0005).
const UT: &str = "bd1d9ddb04089700cf9c27f6f7426281";
/// Field list mirrors akshare `stock_hk_spot_em`.
const FIELDS: &str = "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f12,f13,f14,f15,f16,f17,f18,f20,f21,f23,f24,f25,f22,f11,f62,f128,f136,f115,f152";
/// HKEX boards (m:128).
const FS: &str = "m:128 t:3,m:128 t:4,m:128 t:1,m:128 t:2";
const PAGE_SIZE: u32 = 100;

/// HK real-time spot quote (`stock_hk_spot_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct HkSpotRow {
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
}

/// HK daily/weekly/monthly OHLCV bar (`stock_hk_hist`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct HkHistRow {
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

/// HK real-time spot quotes from Eastmoney (`stock_hk_spot_em`).
///
/// Eastmoney paginates `clist/get`; we walk pages until `total` is covered.
pub async fn stock_hk_spot_em(client: &Client) -> Result<Vec<HkSpotRow>> {
    let mut out = Vec::new();
    let mut pn: u32 = 1;
    loop {
        let pn_s = pn.to_string();
        let pz_s = PAGE_SIZE.to_string();
        let params = [
            ("pn", pn_s.as_str()),
            ("pz", pz_s.as_str()),
            ("po", "1"),
            ("np", "1"),
            ("ut", UT),
            ("fltt", "2"),
            ("invt", "2"),
            ("fid", "f12"),
            ("fs", FS),
            ("fields", FIELDS),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, "stock_hk_spot_em", &crate::core::eastmoney_push::push2_url("/api/qt/clist/get").await, &params)
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
        out.extend(parse(&v)?);
        let total = v
            .get("data")
            .and_then(|d| d.get("total"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        if (pn as u64) * PAGE_SIZE as u64 >= total {
            break;
        }
        pn += 1;
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    }
    Ok(out)
}

/// HK daily/weekly/monthly history from Eastmoney (`stock_hk_hist`).
///
/// `symbol` is the bare HK code (e.g. `"00593"`). `secid` is built as `116.{symbol}`
/// to match akshare. `start_date`/`end_date` are accepted for akshare API parity
/// but not applied server-side (Eastmoney returns the full series).
pub async fn stock_hk_hist(
    client: &Client,
    symbol: &str,
    period: &str,
    start_date: &str,
    end_date: &str,
    adjust: &str,
) -> Result<Vec<HkHistRow>> {
    let klt = period_map(period)?;
    let fqt = adjust_map(adjust)?;
    let secid = format!("116.{symbol}");
    let params = [
        ("secid", secid.as_str()),
        ("fields1", "f1,f2,f3,f4,f5,f6"),
        ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61"),
        ("klt", klt),
        ("fqt", fqt),
        ("end", "20500000"),
        ("lmt", "1000000"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_hk_hist", BASE_HIS, &params)
        .await?;
    let mut rows = parse_hist(&v)?;
    for r in &mut rows {
        r.symbol = symbol.to_string();
    }
    let _ = (start_date, end_date); // accepted for akshare API parity
    Ok(rows)
}

pub(crate) fn parse(resp: &Value) -> Result<Vec<HkSpotRow>> {
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
        out.push(HkSpotRow {
            code: fstr(item, "f12"),
            name: fstr(item, "f14"),
            price: fnum(item, "f2"),
            pct_change: fnum(item, "f3"),
            open: fnum(item, "f17"),
            high: fnum(item, "f15"),
            low: fnum(item, "f16"),
            pre_close: fnum(item, "f18"),
            volume: fnum(item, "f5"),
            amount: fnum(item, "f6"),
        });
    }
    Ok(out)
}

pub(crate) fn parse_hist(resp: &Value) -> Result<Vec<HkHistRow>> {
    let klines = parse_klines(resp, "stock_hk_hist")?;
    Ok(klines
        .into_iter()
        .map(|k| HkHistRow {
            symbol: String::new(),
            date: k.date,
            open: k.open,
            close: k.close,
            high: k.high,
            low: k.low,
            volume: k.volume,
            amount: k.amount,
            pct_change: k.pct_change,
            source: SOURCE_EASTMONEY,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_hk_spot_fixture() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/stock_hk_spot_em.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "00593");
        assert_eq!(rows[0].name, "XX集团");
        assert_eq!(rows[0].price, Some(123.40));
        assert_eq!(rows[0].pct_change, Some(2.31));
        assert_eq!(rows[0].open, Some(121.00));
        assert_eq!(rows[0].high, Some(125.00));
        assert_eq!(rows[0].low, Some(120.00));
        assert_eq!(rows[0].pre_close, Some(120.50));
        assert_eq!(rows[0].volume, Some(1234567.0));
        assert_eq!(rows[0].amount, Some(150_000_000.0));
        assert_eq!(rows[1].code, "00700");
    }

    #[test]
    fn parses_hk_hist_fixture() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/stock_hk_hist.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_hist(&v).unwrap();
        assert_eq!(rows.len(), 2);
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
