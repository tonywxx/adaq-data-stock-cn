//! 百度股市通-热搜股票 (Baidu Finance hot-search stocks).
//!
//! Ports `akshare/stock/stock_hot_search_baidu.py:15`. JSON GET to
//! `finance.pae.baidu.com/selfselect/listsugrecomm`; reads
//! `Result.list.body`. `pxChangeRate` → 涨跌幅, `heat` → 综合热度.
//!
//! | Rust fn | akshare fn | source |
//! |---|---|---|
//! | `stock_hot_search_baidu` | `stock_hot_search_baidu` | `akshare/stock/stock_hot_search_baidu.py:15` |
//!
//! ## DEFERRED
//! None.

use chrono::{Local, Timelike};
use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE: &str = "baidu";
const URL: &str = "https://finance.pae.baidu.com/selfselect/listsugrecomm";

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

#[derive(Debug, Clone, serde::Serialize)]
pub struct HotSearchBaiduRow {
    #[serde(rename = "名称/代码")]
    pub name: Option<String>,
    #[serde(rename = "涨跌幅")]
    pub pct: Option<f64>,
    #[serde(rename = "综合热度")]
    pub heat: Option<f64>,
}

/// Parse `stock_hot_search_baidu` rows from the already-fetched `Value`.
pub(crate) fn parse_hot_search_baidu(resp: &Value) -> Result<Vec<HotSearchBaiduRow>> {
    let body = resp
        .get("Result")
        .and_then(|r| r.get("list"))
        .and_then(|l| l.get("body"))
        .and_then(|b| b.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing Result.list.body".into(),
        })?;
    Ok(body
        .iter()
        .map(|o| HotSearchBaiduRow {
            name: o.get("name").and_then(|v| v.as_str()).map(str::to_string),
            pct: num_of(o.get("pxChangeRate")),
            heat: num_of(o.get("heat")),
        })
        .collect())
}

/// Port of `stock_hot_search_baidu(symbol, date, time)`.
pub async fn stock_hot_search_baidu(
    client: &Client,
    symbol: &str,
    date: &str,
    time: &str,
) -> Result<Vec<HotSearchBaiduRow>> {
    let symbol_map = [
        ("全市场", "all"),
        ("A股", "ab"),
        ("港股", "hk"),
        ("美股", "us"),
    ];
    let market = symbol_map
        .iter()
        .find(|(k, _)| *k == symbol)
        .map(|(_, v)| *v)
        .ok_or_else(|| Error::InvalidParam(format!("unknown symbol {symbol}")))?;
    let hour: u32 = Local::now().hour();
    let hour_str = hour.to_string();
    let params = [
        ("bizType", "wisexmlnew"),
        ("dsp", "iphone"),
        ("product", "search"),
        ("style", "tablelist"),
        ("market", market),
        ("type", time),
        ("day", date),
        ("hour", hour_str.as_str()),
        ("pn", "0"),
        ("rn", "12"),
        ("finClientType", "pc"),
    ];
    let v = client
        .get_json(SOURCE, "stock_hot_search_baidu", URL, &params)
        .await?;
    parse_hot_search_baidu(&v)
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
    fn parses_hot_search_baidu() {
        let rows = parse_hot_search_baidu(&fixture("stock_hot_search_baidu.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name.as_deref(), Some("贵州茅台"));
        assert!(approx(rows[0].pct, 1.23));
        assert!(approx(rows[0].heat, 98765.0));
        assert_eq!(rows[1].name.as_deref(), Some("宁德时代"));
        assert!(approx(rows[1].pct, -0.56));
    }
}
