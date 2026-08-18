//! Caixin (ccxe) PMI trend endpoints ported from `akshare/index/index_cx.py`.
//!
//! `index_pmi_com_cx` / `index_pmi_man_cx` / `index_pmi_ser_cx` each hit the
//! ccxe JSON API (`/api/index/pro/cxIndexTrendInfo`) with `type` = `com` /
//! `man` / `ser`. Pure-HTTP JSON — no JS / token / signature. They share the
//! same three-field shape as the other ccxe series (`日期`, a per-series value
//! column, and `变化值`), so we reuse the [`CxTrendRow`] row from `cx.rs`.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use crate::core::json::*;
use crate::index::cx::CxTrendRow;

const SOURCE_CCXE: &str = "ccxe";
const CCXE_TREND_URL: &str = "https://yun.ccxe.com.cn/api/index/pro/cxIndexTrendInfo";

// ---------------------------------------------------------------------------
// parse core
// ---------------------------------------------------------------------------

/// Parse a ccxe `cxIndexTrendInfo` PMI response into [`CxTrendRow`]s.
///
/// `value_key` is the per-series value column (`综合PMI` / `制造业PMI` /
/// `服务业PMI`); the change column is always `变化值` for the PMI series.
pub(crate) fn parse_cx_pmi(resp: &Value, value_key: &str) -> Result<Vec<CxTrendRow>> {
    let arr = match resp.get("data") {
        Some(Value::Array(a)) => a,
        Some(_) => {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_CCXE,
                message: "data is not an array".into(),
            });
        }
        None => return Ok(Vec::new()),
    };
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(CxTrendRow {
            date: match item.get("日期") {
                Some(Value::Number(n)) => n.as_i64(),
                Some(Value::String(s)) => s.trim().parse::<i64>().ok(),
                _ => None,
            },
            value: opt_f64(item, value_key),
            change: opt_f64(item, "变化值"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// public functions (akshare name -> ccxe `type`)
// ---------------------------------------------------------------------------

/// 财新数据-指数报告-财新中国 PMI-综合 PMI (akshare `index_pmi_com_cx`).
pub async fn index_pmi_com_cx(client: &Client) -> Result<Vec<CxTrendRow>> {
    let params = [("type", "com")];
    let v = client
        .get_json(SOURCE_CCXE, "index_pmi_com_cx", CCXE_TREND_URL, &params)
        .await?;
    parse_cx_pmi(&v, "综合PMI")
}

/// 财新数据-指数报告-财新中国 PMI-制造业 PMI (akshare `index_pmi_man_cx`).
pub async fn index_pmi_man_cx(client: &Client) -> Result<Vec<CxTrendRow>> {
    let params = [("type", "man")];
    let v = client
        .get_json(SOURCE_CCXE, "index_pmi_man_cx", CCXE_TREND_URL, &params)
        .await?;
    parse_cx_pmi(&v, "制造业PMI")
}

/// 财新数据-指数报告-财新中国 PMI-服务业 PMI (akshare `index_pmi_ser_cx`).
pub async fn index_pmi_ser_cx(client: &Client) -> Result<Vec<CxTrendRow>> {
    let params = [("type", "ser")];
    let v = client
        .get_json(SOURCE_CCXE, "index_pmi_ser_cx", CCXE_TREND_URL, &params)
        .await?;
    parse_cx_pmi(&v, "服务业PMI")
}

// ---------------------------------------------------------------------------
// private helpers
// ---------------------------------------------------------------------------


// ---------------------------------------------------------------------------
// offline parse tests
// ---------------------------------------------------------------------------

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
    fn test_parse_index_pmi_com_cx() {
        let rows = parse_cx_pmi(&fixture("index_pmi_com_cx.json"), "综合PMI").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, Some(1704153600000));
        assert_eq!(rows[0].value, Some(50.8));
        assert_eq!(rows[0].change, Some(0.3));
        assert_eq!(rows[1].value, Some(50.5));
        assert_eq!(rows[1].change, Some(0.1));
    }

    #[test]
    fn test_parse_index_pmi_man_cx() {
        let rows = parse_cx_pmi(&fixture("index_pmi_man_cx.json"), "制造业PMI").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, Some(1704153600000));
        assert_eq!(rows[0].value, Some(50.9));
        assert_eq!(rows[0].change, Some(0.2));
    }

    #[test]
    fn test_parse_index_pmi_ser_cx() {
        let rows = parse_cx_pmi(&fixture("index_pmi_ser_cx.json"), "服务业PMI").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, Some(1704153600000));
        assert_eq!(rows[0].value, Some(52.1));
        assert_eq!(rows[0].change, Some(0.6));
    }

    #[test]
    fn test_parse_cx_pmi_empty() {
        let v = serde_json::json!({"data": []});
        let rows = parse_cx_pmi(&v, "综合PMI").unwrap();
        assert!(rows.is_empty());
    }
}
