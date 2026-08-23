//! 东方财富-风险警示板 (Eastmoney ST / risk-warning board).
//!
//! Ports `akshare/stock/stock_zh_a_special.py:200` (`stock_zh_a_stop_em`).
//! JSON GET to `40.push2.eastmoney.com/api/qt/clist/get`; reads
//! `data.diff`. akshare paginates every page and sorts by 涨跌幅 (f3) desc,
//! then numbers rows 1..n — this port fetches the first page (pz=100) and
//! applies the same f3-desc ordering + 1-based 序号.
//!
//! | Rust fn | akshare fn | source |
//! |---|---|---|
//! | `stock_zh_a_stop_em` | `stock_zh_a_stop_em` | `akshare/stock/stock_zh_a_special.py:200` |
//!
//! ## DEFERRED
//! None.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

fn str_of(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

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
pub struct ZhAStopRow {
    /// 序号 (1-based, assigned after sorting by 涨跌幅 desc).
    pub serial: u32,
    #[serde(rename = "代码")]
    pub code: String,
    #[serde(rename = "名称")]
    pub name: String,
    #[serde(rename = "最新价")]
    pub price: Option<f64>,
    #[serde(rename = "涨跌幅")]
    pub pct: Option<f64>,
    #[serde(rename = "涨跌额")]
    pub change: Option<f64>,
    #[serde(rename = "成交量")]
    pub volume: Option<f64>,
    #[serde(rename = "成交额")]
    pub amount: Option<f64>,
    #[serde(rename = "振幅")]
    pub amplitude: Option<f64>,
    #[serde(rename = "最高")]
    pub high: Option<f64>,
    #[serde(rename = "最低")]
    pub low: Option<f64>,
    #[serde(rename = "今开")]
    pub open: Option<f64>,
    #[serde(rename = "昨收")]
    pub pre_close: Option<f64>,
    #[serde(rename = "量比")]
    pub vol_ratio: Option<f64>,
    #[serde(rename = "换手率")]
    pub turnover: Option<f64>,
    #[serde(rename = "市盈率-动态")]
    pub pe_dynamic: Option<f64>,
    #[serde(rename = "市净率")]
    pub pb: Option<f64>,
}

/// Parse one raw `diff` row object.
fn parse_one(o: &Value, serial: u32) -> ZhAStopRow {
    ZhAStopRow {
        serial,
        code: str_of(o.get("f12")),
        name: str_of(o.get("f14")),
        price: num_of(o.get("f2")),
        pct: num_of(o.get("f3")),
        change: num_of(o.get("f4")),
        volume: num_of(o.get("f5")),
        amount: num_of(o.get("f6")),
        amplitude: num_of(o.get("f7")),
        high: num_of(o.get("f15")),
        low: num_of(o.get("f16")),
        open: num_of(o.get("f17")),
        pre_close: num_of(o.get("f18")),
        vol_ratio: num_of(o.get("f10")),
        turnover: num_of(o.get("f8")),
        pe_dynamic: num_of(o.get("f9")),
        pb: num_of(o.get("f24")),
    }
}

/// Parse `stock_zh_a_stop_em` rows. Mirrors akshare: sort by 涨跌幅 (f3) desc,
/// then number 1..n.
pub(crate) fn parse_zh_a_stop(diff: &[Value]) -> Vec<ZhAStopRow> {
    let mut rows: Vec<ZhAStopRow> = diff.iter().map(|o| parse_one(o, 0)).collect();
    rows.sort_by(|a, b| {
        b.pct.partial_cmp(&a.pct).unwrap_or(std::cmp::Ordering::Equal)
    });
    for (i, r) in rows.iter_mut().enumerate() {
        r.serial = (i + 1) as u32;
    }
    rows
}

/// Port of `stock_zh_a_stop_em()` — Eastmoney ST / risk-warning board.
pub async fn stock_zh_a_stop_em(client: &Client) -> Result<Vec<ZhAStopRow>> {
    let params = [
        ("pn", "1"),
        ("pz", "100"),
        ("po", "1"),
        ("np", "1"),
        ("ut", "bd1d9ddb04089700cf9c27f6f7426281"),
        ("fltt", "2"),
        ("invt", "2"),
        ("fid", "f3"),
        ("fs", "m:0 s:3"),
        ("fields", "f2,f3,f4,f5,f6,f7,f8,f9,f10,f12,f13,f14,f15,f16,f17,f18,f20,f21,f23,f24,f25,f22,f11,f62,f128,f136,f115,f152"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_zh_a_stop_em", &crate::core::eastmoney_push::push2_url("/api/qt/clist/get").await, &params)
        .await?;
    let diff = v
        .get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff".into(),
        })?;
    Ok(parse_zh_a_stop(diff))
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
    fn parses_zh_a_stop() {
        let diff = fixture("stock_zh_a_stop_em.json")
            .get("data")
            .unwrap()
            .get("diff")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        let rows = parse_zh_a_stop(&diff);
        assert_eq!(rows.len(), 3);
        // sorted by 涨跌幅 desc: 5.0, 2.0, -1.0 => serials 1,2,3
        assert_eq!(rows[0].serial, 1);
        assert!(approx(rows[0].pct, 5.0));
        assert_eq!(rows[0].code, "000820");
        assert_eq!(rows[1].serial, 2);
        assert!(approx(rows[1].pct, 2.0));
        assert_eq!(rows[2].serial, 3);
        assert!(approx(rows[2].pct, -1.0));
        // 市净率 = f24
        assert!(approx(rows[0].pb, 1.2));
        assert_eq!(rows[0].name, "神雾节能");
    }
}
