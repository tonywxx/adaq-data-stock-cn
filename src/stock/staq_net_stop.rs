//! 东方财富-两网及退市 (Eastmoney STAQ/NET & delisted board).
//!
//! Ports `akshare/stock/stock_stop.py:13`. Single JSON GET to
//! `5.push2.eastmoney.com/api/qt/clist/get`; the `data.diff` object maps
//! index → {f12 代码, f14 名称}. akshare builds a (序号, 代码, 名称) table.
//!
//! | Rust fn | akshare fn | source |
//! |---|---|---|
//! | `stock_staq_net_stop` | `stock_staq_net_stop` | eastmoney | `akshare/stock/stock_stop.py:13` |
//!
//! ## DEFERRED
//! None.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

const URL: &str = "https://5.push2.eastmoney.com/api/qt/clist/get";

#[derive(Debug, Clone, serde::Serialize)]
pub struct StaqNetStopRow {
    /// 序号 (1-based row ordinal).
    pub serial: u32,
    /// 代码 (`f12`).
    #[serde(rename = "代码")]
    pub code: String,
    /// 名称 (`f14`).
    #[serde(rename = "名称")]
    pub name: String,
}

fn str_of(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

/// Parse `stock_staq_net_stop` rows from the already-fetched `Value`.
///
/// Eastmoney's `clist/get` returns `data.diff` as an object keyed by the row
/// index (`{"0": {f12, f14}, ...}`). Be lenient and also accept a plain array,
/// so the parser copes with either shape.
pub(crate) fn parse_staq_net_stop(resp: &Value) -> Result<Vec<StaqNetStopRow>> {
    let diff = resp
        .get("data")
        .and_then(|d| d.get("diff"))
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff at stock_staq_net_stop".into(),
        })?;
    let items: Vec<&Value> = match diff {
        Value::Object(o) => o.values().collect(),
        Value::Array(a) => a.iter().collect(),
        _ => {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "data.diff not object/array at stock_staq_net_stop".into(),
            })
        }
    };
    let mut out = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        out.push(StaqNetStopRow {
            serial: (i + 1) as u32,
            code: str_of(item.get("f12")),
            name: str_of(item.get("f14")),
        });
    }
    Ok(out)
}

/// Port of `stock_staq_net_stop()` — Eastmoney STAQ/NET & delisted board.
pub async fn stock_staq_net_stop(client: &Client) -> Result<Vec<StaqNetStopRow>> {
    let params = [
        ("pn", "1"),
        ("pz", "50000"),
        ("po", "1"),
        ("np", "2"),
        ("ut", "bd1d9ddb04089700cf9c27f6f7426281"),
        ("fltt", "2"),
        ("invt", "2"),
        ("fid", "f3"),
        ("fs", "m:0 s:3"),
        ("fields", "f12,f14"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_staq_net_stop", URL, &params)
        .await?;
    parse_staq_net_stop(&v)
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

    #[test]
    fn parses_staq_net_stop() {
        let rows = parse_staq_net_stop(&fixture("stock_staq_net_stop.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].serial, 1);
        assert_eq!(rows[0].code, "400001");
        assert_eq!(rows[0].name, "某某A");
        assert_eq!(rows[1].serial, 2);
        assert_eq!(rows[1].code, "400002");
        assert_eq!(rows[1].name, "某某B");
    }
}
