use std::collections::HashMap;

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

const REITS_UT: &str = "f057cbcbce2a86e2866ab8877db1d059";

// ---------------------------------------------------------------------------
// reits_hist_min_em — 东方财富-沪深REITs-历史分钟行情
// https://quote.eastmoney.com/sh508097.html
// ---------------------------------------------------------------------------

/// 沪深REITs历史分钟行情行 (`reits_hist_min_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReitsHistMinRow {
    pub symbol: String,
    pub time: String,
    pub price: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub volume: Option<f64>,
    pub amount: Option<f64>,
    pub pre_close: Option<f64>,
    pub source: &'static str,
}

/// 沪深REITs历史分钟行情 from Eastmoney (`reits_hist_min_em`).
///
/// `symbol` is a REITs code (e.g. `508097`). The market id is resolved via the
/// Eastmoney clist endpoint (ported from `__reits_code_market_map`).
pub async fn reits_hist_min_em(client: &Client, symbol: &str) -> Result<Vec<ReitsHistMinRow>> {
    let market_map = reits_code_market_map(client).await?;
    let market = market_map
        .get(symbol)
        .copied()
        .ok_or_else(|| Error::InvalidParam(format!("unknown reits symbol: {symbol}")))?;
    let secid = format!("{market}.{symbol}");
    let params = [
        ("secid", secid.as_str()),
        ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f11,f12,f13,f14,f17"),
        (
            "fields2",
            "f51,f53,f54,f55,f56,f57,f58",
        ),
        ("iscr", "0"),
        ("iscca", "0"),
        ("ut", REITS_UT),
        ("ndays", "5"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "reits_hist_min_em", &crate::core::eastmoney_push::push2_url("/api/qt/stock/trends2/get").await, &params)
        .await?;
    parse_reits_hist_min_trends(&v, symbol)
}

pub(crate) async fn reits_code_market_map(client: &Client) -> Result<HashMap<String, u32>> {
    let params = [
        ("pn", "1"),
        ("pz", "100"),
        ("po", "1"),
        ("np", "1"),
        ("ut", "bd1d9ddb04089700cf9c27f6f7426281"),
        ("fltt", "2"),
        ("invt", "2"),
        ("fid", "f3"),
        ("fs", "m:1 t:9 e:97,m:0 t:10 e:97"),
        ("fields", "f12,f13"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "reits_hist_min_em", &crate::core::eastmoney_push::push2_url("/api/qt/clist/get").await, &params)
        .await?;
    parse_reits_code_market_map(&v)
}

pub(crate) fn parse_reits_code_market_map(resp: &Value) -> Result<HashMap<String, u32>> {
    let diff = resp
        .get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff".into(),
        })?;
    let mut map = HashMap::with_capacity(diff.len());
    for item in diff {
        let code = item.get("f12").and_then(|c| c.as_str()).unwrap_or_default().to_string();
        let market = item.get("f13").and_then(|m| m.as_u64()).unwrap_or(0) as u32;
        if !code.is_empty() {
            map.insert(code, market);
        }
    }
    Ok(map)
}

pub(crate) fn parse_reits_hist_min_trends(resp: &Value, symbol: &str) -> Result<Vec<ReitsHistMinRow>> {
    let trends = resp
        .get("data")
        .and_then(|d| d.get("trends"))
        .and_then(|t| t.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.trends".into(),
        })?;
    let mut out = Vec::with_capacity(trends.len());
    for t in trends {
        let s = t.as_str().ok_or_else(|| Error::Parse {
            endpoint: "reits_hist_min_em",
            message: "trend entry is not a string".into(),
        })?;
        let p: Vec<&str> = s.split(',').collect();
        out.push(ReitsHistMinRow {
            symbol: symbol.to_string(),
            time: p.first().map(|x| x.to_string()).unwrap_or_default(),
            price: p.get(1).and_then(|x| x.parse::<f64>().ok()),
            high: p.get(2).and_then(|x| x.parse::<f64>().ok()),
            low: p.get(3).and_then(|x| x.parse::<f64>().ok()),
            volume: p.get(4).and_then(|x| x.parse::<f64>().ok()),
            amount: p.get(5).and_then(|x| x.parse::<f64>().ok()),
            pre_close: p.get(6).and_then(|x| x.parse::<f64>().ok()),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/{name}.json"));
        let txt = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    #[test]
    fn parses_reits_hist_min_trends() {
        let v = fixture("reits_hist_min_em");
        let rows = parse_reits_hist_min_trends(&v, "508097").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "508097");
        assert_eq!(rows[0].time, "2024-01-02 09:30");
        assert_eq!(rows[0].price, Some(3.85));
        assert_eq!(rows[0].high, Some(3.90));
        assert_eq!(rows[0].pre_close, Some(3.80));
        assert_eq!(rows[1].amount, Some(1500000.0));
    }

    #[test]
    fn parses_reits_code_market_map() {
        let v = fixture("reits_code_market_map");
        let map = parse_reits_code_market_map(&v).unwrap();
        assert_eq!(map.get("508097"), Some(&1));
        assert_eq!(map.get("180101"), Some(&0));
        assert_eq!(map.len(), 2);
    }
}
