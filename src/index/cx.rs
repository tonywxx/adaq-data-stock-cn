//! Caixin (ccxe) index trend endpoints ported from `akshare/index/index_cx.py`.
//!
//! Every function in that module hits the same ccxe JSON API
//! (`/api/index/pro/cxIndexTrendInfo`) with a `type` query param (some also send
//! `code` + `month`). They are all pure-HTTP JSON — no JS / token / signature /
//! `execjs` / `MiniRacer` / `get_token` — so the whole surface ports cleanly.
//!
//! All series share the same three-field response shape (`日期`, a per-series
//! value column, and a `变化值`/`变化幅度` change column), so a single shared row
//! struct + parse core back each public function. The `value_key` / `change_key`
//! differ per series and are threaded through `parse_cx_trend`.
//!
//! Already done elsewhere (NOT ported here):
//! - `index_pmi_com_cx` / `index_pmi_man_cx` / `index_pmi_ser_cx`, plus the
//!   `dei` / `ii` / `si` / `fi` series — all served by `index_pmi_cx` in
//!   `src/stock/index/more.rs` via its `category` param. No re-implementation.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_CCXE: &str = "ccxe";
const CCXE_TREND_URL: &str = "https://yun.ccxe.com.cn/api/index/pro/cxIndexTrendInfo";

// ---------------------------------------------------------------------------
// shared row + parse core
// ---------------------------------------------------------------------------

/// One Caixin (ccxe) trend data point from the `cxIndexTrendInfo` endpoint.
///
/// All Caixin index series share the same three-field shape. The `value`
/// column name (akshare `数字经济指数` / `产业指数` / `大宗商品指数` / ...) depends on
/// the series being queried; `change` is either `变化值` or `变化幅度`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CxTrendRow {
    /// 日期 — millisecond epoch (raw from upstream; akshare converts to a date)
    pub date: Option<i64>,
    /// the series value (akshare column name varies by series, e.g. `数字经济指数`)
    pub value: Option<f64>,
    /// 变化值 / 变化幅度 — change vs previous period
    pub change: Option<f64>,
}

/// Parse a ccxe `cxIndexTrendInfo` response into [`CxTrendRow`]s.
///
/// `value_key` is the per-series value column (e.g. `"数字经济指数"`); `change_key`
/// is either `"变化值"` or `"变化幅度"`.
pub(crate) fn parse_cx_trend(
    resp: &Value,
    value_key: &str,
    change_key: &str,
) -> Result<Vec<CxTrendRow>> {
    let data = resp.get("data");
    let arr = match data {
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
            value: fnum(item, value_key),
            change: fnum(item, change_key),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// public functions (akshare name -> ccxe `type`)
// ---------------------------------------------------------------------------

/// 财新数据-指数报告-数字经济指数 (akshare `index_dei_cx`).
pub async fn index_dei_cx(client: &Client) -> Result<Vec<CxTrendRow>> {
    let params = [("type", "dei")];
    let v = client
        .get_json(SOURCE_CCXE, "index_dei_cx", CCXE_TREND_URL, &params)
        .await?;
    parse_index_dei_cx(&v)
}

/// 财新数据-指数报告-产业指数 (akshare `index_ii_cx`).
pub async fn index_ii_cx(client: &Client) -> Result<Vec<CxTrendRow>> {
    let params = [("type", "ii")];
    let v = client
        .get_json(SOURCE_CCXE, "index_ii_cx", CCXE_TREND_URL, &params)
        .await?;
    parse_index_ii_cx(&v)
}

/// 财新数据-指数报告-溢出指数 (akshare `index_si_cx`).
pub async fn index_si_cx(client: &Client) -> Result<Vec<CxTrendRow>> {
    let params = [("type", "si")];
    let v = client
        .get_json(SOURCE_CCXE, "index_si_cx", CCXE_TREND_URL, &params)
        .await?;
    parse_index_si_cx(&v)
}

/// 财新数据-指数报告-融合指数 (akshare `index_fi_cx`).
pub async fn index_fi_cx(client: &Client) -> Result<Vec<CxTrendRow>> {
    let params = [("type", "fi")];
    let v = client
        .get_json(SOURCE_CCXE, "index_fi_cx", CCXE_TREND_URL, &params)
        .await?;
    parse_index_fi_cx(&v)
}

/// 财新数据-指数报告-基础指数 (akshare `index_bi_cx`).
pub async fn index_bi_cx(client: &Client) -> Result<Vec<CxTrendRow>> {
    let params = [("type", "bi")];
    let v = client
        .get_json(SOURCE_CCXE, "index_bi_cx", CCXE_TREND_URL, &params)
        .await?;
    parse_index_bi_cx(&v)
}

/// 财新数据-指数报告-中国新经济指数 (akshare `index_nei_cx`).
pub async fn index_nei_cx(client: &Client) -> Result<Vec<CxTrendRow>> {
    let params = [("type", "nei")];
    let v = client
        .get_json(SOURCE_CCXE, "index_nei_cx", CCXE_TREND_URL, &params)
        .await?;
    parse_index_nei_cx(&v)
}

/// 财新数据-指数报告-劳动力投入指数 (akshare `index_li_cx`).
pub async fn index_li_cx(client: &Client) -> Result<Vec<CxTrendRow>> {
    let params = [("type", "li")];
    let v = client
        .get_json(SOURCE_CCXE, "index_li_cx", CCXE_TREND_URL, &params)
        .await?;
    parse_index_li_cx(&v)
}

/// 财新数据-指数报告-资本投入指数 (akshare `index_ci_cx`).
pub async fn index_ci_cx(client: &Client) -> Result<Vec<CxTrendRow>> {
    let params = [("type", "ci")];
    let v = client
        .get_json(SOURCE_CCXE, "index_ci_cx", CCXE_TREND_URL, &params)
        .await?;
    parse_index_ci_cx(&v)
}

/// 财新数据-指数报告-科技投入指数 (akshare `index_ti_cx`).
pub async fn index_ti_cx(client: &Client) -> Result<Vec<CxTrendRow>> {
    let params = [("type", "ti")];
    let v = client
        .get_json(SOURCE_CCXE, "index_ti_cx", CCXE_TREND_URL, &params)
        .await?;
    parse_index_ti_cx(&v)
}

/// 财新数据-指数报告-新经济行业入职平均工资水平 (akshare `index_neaw_cx`).
pub async fn index_neaw_cx(client: &Client) -> Result<Vec<CxTrendRow>> {
    let params = [("type", "neaw")];
    let v = client
        .get_json(SOURCE_CCXE, "index_neaw_cx", CCXE_TREND_URL, &params)
        .await?;
    parse_index_neaw_cx(&v)
}

/// 财新数据-指数报告-新经济入职工资溢价水平 (akshare `index_awpr_cx`).
pub async fn index_awpr_cx(client: &Client) -> Result<Vec<CxTrendRow>> {
    let params = [("type", "awpr")];
    let v = client
        .get_json(SOURCE_CCXE, "index_awpr_cx", CCXE_TREND_URL, &params)
        .await?;
    parse_index_awpr_cx(&v)
}

/// 财新数据-指数报告-大宗商品指数 (akshare `index_cci_cx`).
pub async fn index_cci_cx(client: &Client) -> Result<Vec<CxTrendRow>> {
    let params = [("type", "cci"), ("code", "1000050"), ("month", "-1")];
    let v = client
        .get_json(SOURCE_CCXE, "index_cci_cx", CCXE_TREND_URL, &params)
        .await?;
    parse_index_cci_cx(&v)
}

/// 财新数据-指数报告-高质量因子 (akshare `index_qli_cx`).
pub async fn index_qli_cx(client: &Client) -> Result<Vec<CxTrendRow>> {
    let params = [("type", "qli"), ("code", "1000050"), ("month", "-1")];
    let v = client
        .get_json(SOURCE_CCXE, "index_qli_cx", CCXE_TREND_URL, &params)
        .await?;
    parse_index_qli_cx(&v)
}

/// 财新数据-指数报告-AI策略指数 (akshare `index_ai_cx`).
pub async fn index_ai_cx(client: &Client) -> Result<Vec<CxTrendRow>> {
    let params = [("type", "ai"), ("code", "1000050"), ("month", "-1")];
    let v = client
        .get_json(SOURCE_CCXE, "index_ai_cx", CCXE_TREND_URL, &params)
        .await?;
    parse_index_ai_cx(&v)
}

/// 财新数据-指数报告-基石经济指数 (akshare `index_bei_cx`).
pub async fn index_bei_cx(client: &Client) -> Result<Vec<CxTrendRow>> {
    let params = [("type", "ind"), ("code", "930927"), ("month", "-1")];
    let v = client
        .get_json(SOURCE_CCXE, "index_bei_cx", CCXE_TREND_URL, &params)
        .await?;
    parse_index_bei_cx(&v)
}

/// 财新数据-指数报告-新动能指数 (akshare `index_neei_cx`).
pub async fn index_neei_cx(client: &Client) -> Result<Vec<CxTrendRow>> {
    let params = [("type", "ind"), ("code", "930928"), ("month", "1")];
    let v = client
        .get_json(SOURCE_CCXE, "index_neei_cx", CCXE_TREND_URL, &params)
        .await?;
    parse_index_neei_cx(&v)
}

// ---------------------------------------------------------------------------
// per-series parse wrappers (thread the correct value_key / change_key)
// ---------------------------------------------------------------------------

pub(crate) fn parse_index_dei_cx(resp: &Value) -> Result<Vec<CxTrendRow>> {
    parse_cx_trend(resp, "数字经济指数", "变化值")
}
pub(crate) fn parse_index_ii_cx(resp: &Value) -> Result<Vec<CxTrendRow>> {
    parse_cx_trend(resp, "产业指数", "变化值")
}
pub(crate) fn parse_index_si_cx(resp: &Value) -> Result<Vec<CxTrendRow>> {
    parse_cx_trend(resp, "溢出指数", "变化值")
}
pub(crate) fn parse_index_fi_cx(resp: &Value) -> Result<Vec<CxTrendRow>> {
    parse_cx_trend(resp, "融合指数", "变化值")
}
pub(crate) fn parse_index_bi_cx(resp: &Value) -> Result<Vec<CxTrendRow>> {
    parse_cx_trend(resp, "基础指数", "变化值")
}
pub(crate) fn parse_index_nei_cx(resp: &Value) -> Result<Vec<CxTrendRow>> {
    parse_cx_trend(resp, "中国新经济指数", "变化值")
}
pub(crate) fn parse_index_li_cx(resp: &Value) -> Result<Vec<CxTrendRow>> {
    parse_cx_trend(resp, "劳动力投入指数", "变化值")
}
pub(crate) fn parse_index_ci_cx(resp: &Value) -> Result<Vec<CxTrendRow>> {
    parse_cx_trend(resp, "资本投入指数", "变化值")
}
pub(crate) fn parse_index_ti_cx(resp: &Value) -> Result<Vec<CxTrendRow>> {
    parse_cx_trend(resp, "科技投入指数", "变化值")
}
pub(crate) fn parse_index_neaw_cx(resp: &Value) -> Result<Vec<CxTrendRow>> {
    parse_cx_trend(resp, "新经济行业入职平均工资水平", "变化值")
}
pub(crate) fn parse_index_awpr_cx(resp: &Value) -> Result<Vec<CxTrendRow>> {
    parse_cx_trend(resp, "新经济入职工资溢价水平", "变化值")
}
pub(crate) fn parse_index_cci_cx(resp: &Value) -> Result<Vec<CxTrendRow>> {
    parse_cx_trend(resp, "大宗商品指数", "变化值")
}
pub(crate) fn parse_index_qli_cx(resp: &Value) -> Result<Vec<CxTrendRow>> {
    parse_cx_trend(resp, "高质量因子指数", "变化幅度")
}
pub(crate) fn parse_index_ai_cx(resp: &Value) -> Result<Vec<CxTrendRow>> {
    parse_cx_trend(resp, "AI策略指数", "变化幅度")
}
pub(crate) fn parse_index_bei_cx(resp: &Value) -> Result<Vec<CxTrendRow>> {
    parse_cx_trend(resp, "基石经济指数", "变化幅度")
}
pub(crate) fn parse_index_neei_cx(resp: &Value) -> Result<Vec<CxTrendRow>> {
    parse_cx_trend(resp, "新动能指数", "变化幅度")
}

// ---------------------------------------------------------------------------
// private helpers (verbatim per task instructions)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

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
    fn test_parse_index_dei_cx() {
        let rows = parse_index_dei_cx(&fixture("index_dei_cx.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, Some(1704153600000));
        assert_eq!(rows[0].value, Some(385.2));
        assert_eq!(rows[0].change, Some(0.3));
    }

    #[test]
    fn test_parse_index_ii_cx() {
        let rows = parse_index_ii_cx(&fixture("index_ii_cx.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value, Some(512.7));
        assert_eq!(rows[0].change, Some(1.2));
    }

    #[test]
    fn test_parse_index_si_cx() {
        let rows = parse_index_si_cx(&fixture("index_si_cx.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value, Some(233.1));
        assert_eq!(rows[0].change, Some(0.5));
    }

    #[test]
    fn test_parse_index_fi_cx() {
        let rows = parse_index_fi_cx(&fixture("index_fi_cx.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value, Some(188.4));
        assert_eq!(rows[0].change, Some(-0.2));
    }

    #[test]
    fn test_parse_index_bi_cx() {
        let rows = parse_index_bi_cx(&fixture("index_bi_cx.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value, Some(300.6));
        assert_eq!(rows[0].change, Some(0.8));
    }

    #[test]
    fn test_parse_index_nei_cx() {
        let rows = parse_index_nei_cx(&fixture("index_nei_cx.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value, Some(29.9));
        assert_eq!(rows[0].change, Some(0.1));
    }

    #[test]
    fn test_parse_index_li_cx() {
        let rows = parse_index_li_cx(&fixture("index_li_cx.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value, Some(25.3));
        assert_eq!(rows[0].change, Some(-0.3));
    }

    #[test]
    fn test_parse_index_ci_cx() {
        let rows = parse_index_ci_cx(&fixture("index_ci_cx.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value, Some(31.7));
        assert_eq!(rows[0].change, Some(0.4));
    }

    #[test]
    fn test_parse_index_ti_cx() {
        let rows = parse_index_ti_cx(&fixture("index_ti_cx.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value, Some(27.8));
        assert_eq!(rows[0].change, Some(0.6));
    }

    #[test]
    fn test_parse_index_neaw_cx() {
        let rows = parse_index_neaw_cx(&fixture("index_neaw_cx.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value, Some(11980.0));
        assert_eq!(rows[0].change, Some(120.0));
    }

    #[test]
    fn test_parse_index_awpr_cx() {
        let rows = parse_index_awpr_cx(&fixture("index_awpr_cx.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value, Some(5.2));
        assert_eq!(rows[0].change, Some(-0.1));
    }

    #[test]
    fn test_parse_index_cci_cx() {
        let rows = parse_index_cci_cx(&fixture("index_cci_cx.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value, Some(1050.3));
        assert_eq!(rows[0].change, Some(12.5));
    }

    #[test]
    fn test_parse_index_qli_cx() {
        let rows = parse_index_qli_cx(&fixture("index_qli_cx.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value, Some(1420.5));
        assert_eq!(rows[0].change, Some(8.3));
    }

    #[test]
    fn test_parse_index_ai_cx() {
        let rows = parse_index_ai_cx(&fixture("index_ai_cx.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value, Some(1601.2));
        assert_eq!(rows[0].change, Some(-5.4));
    }

    #[test]
    fn test_parse_index_bei_cx() {
        let rows = parse_index_bei_cx(&fixture("index_bei_cx.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value, Some(980.7));
        assert_eq!(rows[0].change, Some(3.1));
    }

    #[test]
    fn test_parse_index_neei_cx() {
        let rows = parse_index_neei_cx(&fixture("index_neei_cx.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value, Some(1102.9));
        assert_eq!(rows[0].change, Some(-2.2));
    }
}
