//! ChinaMoney (CFETS) bond endpoints. Ports `akshare/bond/bond_china_money.py`.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `bond_china_close_return_map` | `bond_china_money.py:93` | GET `ClsYldCurvCurvGO`, `records` (dict rows) |
//!
//! ## DEFERRED
//! This batch triages every remaining `bond` function; the deferred ones are
//! recorded here for completeness:
//! - `macro_china_bond_public` (`bond_china_money.py:313`) — POST `bnBondEmit`
//!   returns 404 unless preceded by a ChinaMoney session/token bootstrap
//!   (`bond_china_close_return_map()` cookie pre-step); DEFERRED (third-party
//!   session/token, cannot chain cookies with the stateless client).
//! - `bond_corporate_issue_cninfo` (`bond_issue_cninfo.py:222`) — cninfo source;
//!   DEFERRED (cninfo auth / `Accept-Enckey`).
//! - `bond_cov_issue_cninfo` (`bond_issue_cninfo.py:322`) — cninfo source;
//!   DEFERRED (cninfo auth / `Accept-Enckey`).
//! - `bond_cov_stock_issue_cninfo` (`bond_issue_cninfo.py:481`) — cninfo source;
//!   DEFERRED (cninfo auth / `Accept-Enckey`).
//! - `bond_local_government_issue_cninfo` (`bond_issue_cninfo.py:126`) — cninfo;
//!   DEFERRED (cninfo auth / `Accept-Enckey`).
//! - `bond_treasure_issue_cninfo` (`bond_issue_cninfo.py:30`) — cninfo;
//!   DEFERRED (cninfo auth / `Accept-Enckey`).
//! - `bond_zh_hs_daily` (`bond_zh_sina.py:118`) — Sina history decoded via
//!   `py_mini_racer` JS execution; DEFERRED (JS-exec / `wencode`).

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE: &str = "chinamoney";

const CLS_YLD_CURV_URL: &str =
    "https://www.chinamoney.com.cn/ags/ms/cm-u-bk-currency/ClsYldCurvCurvGO";

/// 收盘收益率曲线映射行 (`bond_china_close_return_map`). Column names match
/// akshare's pass-through output (`records` dict keys are not renamed).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondCloseReturnMapRow {
    #[serde(rename = "value")]
    pub value: String,
    #[serde(rename = "cnLabel")]
    pub cn_label: String,
    #[serde(rename = "enLabel")]
    pub en_label: String,
}

pub(crate) fn parse_close_return_map(resp: &Value) -> Result<Vec<BondCloseReturnMapRow>> {
    let records = resp
        .get("records")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing records".into(),
        })?;
    let mut out = Vec::with_capacity(records.len());
    for rec in records {
        let obj = match rec.as_object() {
            Some(o) => o,
            None => continue,
        };
        let get = |k: &str| -> String {
            obj.get(k)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string()
        };
        out.push(BondCloseReturnMapRow {
            value: get("value"),
            cn_label: get("cnLabel"),
            en_label: get("enLabel"),
        });
    }
    Ok(out)
}

/// 收盘收益率曲线历史数据映射 (`bond_china_close_return_map`).
///
/// GETs the ChinaMoney `ClsYldCurvCurvGO` endpoint and returns the `records`
/// array as-is (akshare does not rename these columns).
pub async fn bond_china_close_return_map(client: &Client) -> Result<Vec<BondCloseReturnMapRow>> {
    let hdrs: [(&str, &str); 1] = [("X-Requested-With", "XMLHttpRequest")];
    let v = client
        .get_json_with_headers(
            SOURCE,
            "bond_china_close_return_map",
            CLS_YLD_CURV_URL,
            &[],
            Some(&hdrs),
        )
        .await?;
    parse_close_return_map(&v)
}

const CLS_YLD_CURV_HIS_URL: &str =
    "https://www.chinamoney.com.cn/ags/ms/cm-u-bk-currency/ClsYldCurvHis";

/// 收盘收益率曲线历史数据行 (`bond_china_close_return`). Column order/names
/// match akshare's pass-through output (akshare drops `newDateValue` and keeps
/// 日期, 期限, 到期收益率, 即期收益率, 远期收益率).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondCloseReturnRow {
    #[serde(rename = "日期")]
    pub date: String,
    #[serde(rename = "期限")]
    pub term: Option<f64>,
    #[serde(rename = "到期收益率")]
    pub ytm: Option<f64>,
    #[serde(rename = "即期收益率")]
    pub spot: Option<f64>,
    #[serde(rename = "远期收益率")]
    pub forward: Option<f64>,
}

pub(crate) fn parse_close_return(resp: &Value) -> Result<Vec<BondCloseReturnRow>> {
    let records = resp
        .get("records")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing records".into(),
        })?;
    let mut out = Vec::with_capacity(records.len());
    for rec in records {
        let obj = match rec.as_object() {
            Some(o) => o,
            None => continue,
        };
        let s = |k: &str| -> String {
            obj.get(k)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string()
        };
        let n = |k: &str| -> Option<f64> {
            obj.get(k).and_then(|v| match v {
                Value::Number(n) => n.as_f64(),
                Value::String(s) => s.trim().parse::<f64>().ok(),
                _ => None,
            })
        };
        out.push(BondCloseReturnRow {
            date: s("日期"),
            term: n("期限"),
            ytm: n("到期收益率"),
            spot: n("即期收益率"),
            forward: n("远期收益率"),
        });
    }
    Ok(out)
}

/// 收盘收益率曲线历史数据 (`bond_china_close_return`, bond_china_money.py:127).
///
/// Resolves `symbol` (e.g. `国债`) to its ChinaMoney `bondType` code via
/// [`bond_china_close_return_map`], then GETs `ClsYldCurvHis`.
pub async fn bond_china_close_return(
    client: &Client,
    symbol: &str,
    period: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<BondCloseReturnRow>> {
    let map = bond_china_close_return_map(client).await?;
    let code = map
        .iter()
        .find(|r| r.cn_label == symbol)
        .map(|r| r.value.clone())
        .ok_or_else(|| Error::InvalidParam(format!("unknown bond symbol: {symbol}")))?;
    let hdrs: [(&str, &str); 1] = [("X-Requested-With", "XMLHttpRequest")];
    let params: &[(&str, &str)] = &[
        ("lang", "CN"),
        ("reference", "1,2,3"),
        ("bondType", &code),
        ("startDate", start_date),
        ("endDate", end_date),
        ("termId", period),
        ("pageNum", "1"),
        ("pageSize", "50"),
    ];
    let v = client
        .get_json_with_headers(
            SOURCE,
            "bond_china_close_return",
            CLS_YLD_CURV_HIS_URL,
            params,
            Some(&hdrs),
        )
        .await?;
    parse_close_return(&v)
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
    fn parse_close_return_map_ok() {
        let rows = parse_close_return_map(&fixture("bond_china_close_return_map.json")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].value, "CYCC000");
        assert_eq!(rows[0].cn_label, "国债");
        assert_eq!(rows[0].en_label, "Treasury Bond");
        assert_eq!(rows[2].cn_label, "政策性金融债(国开)");
    }

    #[test]
    fn parse_close_return_ok() {
        let rows = parse_close_return(&fixture("bond_china_close_return.json")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].date, "2023-11-01");
        assert_eq!(rows[0].term, Some(1.0));
        assert_eq!(rows[0].ytm, Some(2.1));
        assert_eq!(rows[0].spot, Some(2.05));
        assert_eq!(rows[0].forward, Some(2.12));
        assert_eq!(rows[2].term, Some(0.5));
    }
}
