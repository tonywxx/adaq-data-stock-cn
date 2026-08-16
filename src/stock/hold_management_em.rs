//! 东方财富-高管持股-人员增减 (Eastmoney executive shareholding — by person).
//!
//! Ports `akshare/stock/stock_hold_control_em.py:111`
//! (`stock_hold_management_person_em`). Single JSON GET to
//! `datacenter-web.eastmoney.com/api/data/v1/get`
//! (reportName `RPT_EXECUTIVE_HOLD_DETAILS`) filtered by code + person name;
//! reads `result.data`.
//!
//! | Rust fn | akshare fn | source |
//! |---|---|---|
//! | `stock_hold_management_person_em` | `stock_hold_management_person_em` | `akshare/stock/stock_hold_control_em.py:111` |
//!
//! ## DEFERRED
//! None.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

const URL: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";

fn str_of(v: Option<&Value>) -> Option<String> {
    match v {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
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

fn emg_data_array(resp: &Value) -> Result<Vec<Value>> {
    resp.get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .cloned()
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data".into(),
        })
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HoldManagementPersonRow {
    #[serde(rename = "日期")]
    pub change_date: Option<String>,
    #[serde(rename = "代码")]
    pub security_code: Option<String>,
    #[serde(rename = "名称")]
    pub security_name: Option<String>,
    #[serde(rename = "变动人")]
    pub person_name: Option<String>,
    #[serde(rename = "变动股数")]
    pub change_shares: Option<f64>,
    #[serde(rename = "成交均价")]
    pub average_price: Option<f64>,
    #[serde(rename = "变动金额")]
    pub change_amount: Option<f64>,
    #[serde(rename = "变动原因")]
    pub change_reason: Option<String>,
    #[serde(rename = "变动比例")]
    pub change_ratio: Option<f64>,
    #[serde(rename = "变动后持股数")]
    pub change_after_holdnum: Option<f64>,
    #[serde(rename = "持股种类")]
    pub hold_type: Option<String>,
    #[serde(rename = "董监高人员姓名")]
    pub dse_person_name: Option<String>,
    #[serde(rename = "职务")]
    pub position_name: Option<String>,
    #[serde(rename = "变动人与董监高的关系")]
    pub person_dse_relation: Option<String>,
    #[serde(rename = "开始时持有")]
    pub begin_hold_num: Option<f64>,
    #[serde(rename = "结束后持有")]
    pub end_hold_num: Option<f64>,
}

pub(crate) fn parse_hold_management_person(arr: &[Value]) -> Vec<HoldManagementPersonRow> {
    arr.iter()
        .map(|o| HoldManagementPersonRow {
            change_date: str_of(o.get("CHANGE_DATE")),
            security_code: str_of(o.get("SECURITY_CODE")),
            security_name: str_of(o.get("SECURITY_NAME")),
            person_name: str_of(o.get("PERSON_NAME")),
            change_shares: num_of(o.get("CHANGE_SHARES")),
            average_price: num_of(o.get("AVERAGE_PRICE")),
            change_amount: num_of(o.get("CHANGE_AMOUNT")),
            change_reason: str_of(o.get("CHANGE_REASON")),
            change_ratio: num_of(o.get("CHANGE_RATIO")),
            change_after_holdnum: num_of(o.get("CHANGE_AFTER_HOLDNUM")),
            hold_type: str_of(o.get("HOLD_TYPE")),
            dse_person_name: str_of(o.get("DSE_PERSON_NAME")),
            position_name: str_of(o.get("POSITION_NAME")),
            person_dse_relation: str_of(o.get("PERSON_DSE_RELATION")),
            begin_hold_num: num_of(o.get("BEGIN_HOLD_NUM")),
            end_hold_num: num_of(o.get("END_HOLD_NUM")),
        })
        .collect()
}

/// Port of `stock_hold_management_person_em(symbol, name)`.
pub async fn stock_hold_management_person_em(
    client: &Client,
    symbol: &str,
    name: &str,
) -> Result<Vec<HoldManagementPersonRow>> {
    let filter = format!("(SECURITY_CODE=\"{symbol}\")(PERSON_NAME=\"{name}\")");
    let params = [
        ("reportName", "RPT_EXECUTIVE_HOLD_DETAILS"),
        ("columns", "ALL"),
        ("quoteColumns", ""),
        ("filter", filter.as_str()),
        ("pageNumber", "1"),
        ("pageSize", "5000"),
        ("sortTypes", "-1,1,1"),
        ("sortColumns", "CHANGE_DATE,SECURITY_CODE,PERSON_NAME"),
        ("source", "WEB"),
        ("client", "WEB"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_hold_management_person_em",
            URL,
            &params,
        )
        .await?;
    let arr = emg_data_array(&v)?;
    Ok(parse_hold_management_person(&arr))
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
    fn parses_hold_management_person() {
        let arr = emg_data_array(&fixture("stock_hold_management_person_em.json")).unwrap();
        let rows = parse_hold_management_person(&arr);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].security_code.as_deref(), Some("001308"));
        assert_eq!(rows[0].person_name.as_deref(), Some("吴远"));
        assert!(approx(rows[0].change_shares, 10000.0));
        assert!(approx(rows[0].average_price, 15.2));
        assert_eq!(rows[1].position_name.as_deref(), Some("高管"));
        assert!(approx(rows[1].end_hold_num, 5000.0));
    }
}
