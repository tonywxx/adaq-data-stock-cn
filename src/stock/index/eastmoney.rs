use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};
use crate::core::json::*;
use crate::stock::hist::HistRow;
use crate::stock::index::IndexSpotQuote;

const UT: &str = "bd1d9ddb04089700cf9c27f6f7426281";
const SPOT_URL: &str = "https://48.push2.eastmoney.com/api/qt/clist/get";
const HIST_URL: &str = "https://push2his.eastmoney.com/api/qt/stock/kline/get";

/// Eastmoney category → `fs` filter (akshare `stock_zh_index_spot_em`).
const CATEGORY_FS: &[(&str, &str)] = &[
    ("沪深重要指数", "b:MK0010"),
    ("上证系列指数", "m:1+t:1"),
    ("深证系列指数", "m:0 t:5"),
    ("指数成份", "m:1+s:3,m:0+t:5"),
    ("中证系列指数", "m:2"),
];

/// Real-time index spot from Eastmoney (`stock_zh_index_spot_em`), defaulting to the
/// 上证系列指数 board. Normalizes to [`IndexSpotQuote`].
pub async fn spot(client: &Client) -> Result<Vec<IndexSpotQuote>> {
    spot_category(client, "上证系列指数").await
}

pub async fn spot_category(client: &Client, category: &str) -> Result<Vec<IndexSpotQuote>> {
    let fs = CATEGORY_FS
        .iter()
        .find(|(k, _)| *k == category)
        .map(|(_, v)| *v)
        .ok_or_else(|| Error::InvalidParam(format!("unknown index category: {category}")))?;
    let params = [
        ("pn", "1"),
        ("pz", "100"),
        ("po", "1"),
        ("np", "1"),
        ("ut", UT),
        ("fltt", "2"),
        ("invt", "2"),
        ("wbp2u", "|0|0|0|web"),
        ("fid", "f12"),
        ("fs", fs),
        (
            "fields",
            "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f12,f13,f14,f15,f16,f17,f18,f20,f21,f23,f24,f25,f26,f22,f33,f11,f62,f128,f136,f115,f152",
        ),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_zh_index_spot_em",
            SPOT_URL,
            &params,
        )
        .await?;
    parse_diff(&v)
}

pub(crate) fn parse_diff(resp: &Value) -> Result<Vec<IndexSpotQuote>> {
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
        out.push(IndexSpotQuote {
            code: opt_str_or(item, "f12", ""),
            name: opt_str_or(item, "f14", ""),
            price: opt_f64(item, "f2"),
            pct_change: opt_f64(item, "f3"),
            change: opt_f64(item, "f4"),
            volume: opt_f64(item, "f5"),
            amount: opt_f64(item, "f6"),
            open: opt_f64(item, "f17"),
            high: opt_f64(item, "f15"),
            low: opt_f64(item, "f16"),
            pre_close: opt_f64(item, "f18"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

/// Per-index historical OHLC from Eastmoney (`index_zh_a_hist`). Reuses [`HistRow`].
pub async fn daily(
    client: &Client,
    symbol: &str,
    period: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<HistRow>> {
    let klt = match period {
        "daily" => "101",
        "weekly" => "102",
        "monthly" => "103",
        _ => return Err(Error::InvalidParam(format!("unknown period: {period}"))),
    };
    let market = if symbol.contains("sz") || symbol.contains("bj") {
        "0"
    } else if symbol.contains("sh") {
        "1"
    } else if symbol.contains("csi") {
        "2"
    } else {
        return Err(Error::InvalidParam(format!(
            "cannot infer market for index symbol: {symbol}"
        )));
    };
    let code = symbol
        .replace("sz", "")
        .replace("sh", "")
        .replace("bj", "")
        .replace("csi", "");
    let secid = format!("{market}.{code}");
    let secid_ref = secid.as_str();

    let params = [
        ("secid", secid_ref),
        ("fields1", "f1,f2,f3,f4,f5"),
        ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58"),
        ("klt", klt),
        ("fqt", "0"),
        ("beg", start_date),
        ("end", end_date),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "index_zh_a_hist", HIST_URL, &params)
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
        if parts.len() < 8 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: format!("index kline has {} fields, expected >= 8", parts.len()),
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
            pct_change: None,
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_eastmoney_index_spot_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/stock_zh_index_spot_em.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_diff(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "000001");
        assert_eq!(rows[0].name, "上证指数");
        assert_eq!(rows[0].price, Some(3200.50));
        assert_eq!(rows[0].pct_change, Some(1.20));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].code, "399001");
        assert_eq!(rows[1].name, "深证成指");
    }

    #[test]
    fn parses_eastmoney_index_hist_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/index_zh_a_hist_em.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_klines(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2025-01-02");
        assert_eq!(rows[0].open, Some(3200.0));
        assert_eq!(rows[0].close, Some(3250.0));
        assert_eq!(rows[0].amount, Some(2500000000.0));
        assert_eq!(rows[0].source, "eastmoney");
    }
}
