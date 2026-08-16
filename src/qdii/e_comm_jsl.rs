//! 集思录 (jisilu) T+0 QDII 欧美市场-商品 listings. Ports
//! `akshare/qdii/qdii_jsl.py::qdii_e_comm_jsl` (line 88).
//!
//! Hits `https://www.jisilu.cn/data/qdii/qdii_list/E` (the same URL as
//! `qdii_e_index_jsl`); jisilu returns the JSON `rows` directly, so a plain
//! GET with a `___jsl` cache-buster + `rp` page-size is enough. Akshare's
//! `qdii_e_comm_jsl` renames exactly the same cell columns as `qdii_e_index_jsl`,
//! so the row shape matches `QdiiEIndexRow` (defined separately here to keep
//! this leaf module self-contained / non-editable sibling untouched).
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `qdii_e_comm_jsl` | `qdii_jsl.py:88` | 欧美市场-商品 (`qdii_list/E`) |
//!
//! ## DEFERRED
//! None.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE: &str = "jisilu";
const URL_E: &str = "https://www.jisilu.cn/data/qdii/qdii_list/E";
const RP: &str = "22";
// akshare hardcodes this `___jsl` cache-buster token; jisilu accepts it as-is.
const JSL_E: &str = "LST___t=1728207798534";

/// Read an optional string cell, treating `-`/`""` as absent (N/A).
fn cell_str(cell: &Value, key: &str) -> Option<String> {
    match cell.get(key) {
        Some(Value::String(s)) if !s.is_empty() && s != "-" => Some(s.clone()),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    }
}

/// Read an optional numeric cell, leniently handling strings, `%` suffixes, and
/// `-`/`""` (N/A). `amount` arrives as a JSON number; the rest as strings.
fn cell_num(cell: &Value, key: &str) -> Option<f64> {
    match cell.get(key) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() || t == "-" {
                return None;
            }
            let cleaned: String = t.chars().filter(|c| *c != '%').collect();
            cleaned.parse::<f64>().ok()
        }
        _ => None,
    }
}

/// Index the `rows` array of a jisilu response.
fn qdii_rows(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("rows")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing rows".into(),
        })
}

/// 集思录-T+0 QDII-欧美市场-商品 (`qdii_list/E`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct QdiiECommRow {
    pub fund_id: Option<String>,
    pub fund_nm: Option<String>,
    pub price: Option<f64>,
    pub increase_rt: Option<f64>,
    pub volume: Option<f64>,
    pub amount: Option<f64>,
    pub amount_incr: Option<f64>,
    pub fund_nav: Option<f64>,
    pub nav_dt: Option<String>,
    pub estimate_value: Option<f64>,
    pub last_est_dt: Option<String>,
    pub discount_rt: Option<f64>,
    pub index_nm: Option<String>,
    pub ref_increase_rt: Option<f64>,
    pub apply_fee: Option<String>,
    pub redeem_fee: Option<String>,
    pub mt_fee: Option<f64>,
    pub issuer_nm: Option<String>,
}

/// Parse `qdii_e_comm_jsl` rows from a `qdii_list/E` response.
pub(crate) fn parse_qdii_e_comm(resp: &Value) -> Result<Vec<QdiiECommRow>> {
    let rows = qdii_rows(resp)?;
    let mut out = Vec::with_capacity(rows.len());
    for item in rows {
        let Some(cell) = item.get("cell") else {
            continue;
        };
        out.push(QdiiECommRow {
            fund_id: cell_str(cell, "fund_id"),
            fund_nm: cell_str(cell, "fund_nm"),
            price: cell_num(cell, "price"),
            increase_rt: cell_num(cell, "increase_rt"),
            volume: cell_num(cell, "volume"),
            amount: cell_num(cell, "amount"),
            amount_incr: cell_num(cell, "amount_incr"),
            fund_nav: cell_num(cell, "fund_nav"),
            nav_dt: cell_str(cell, "nav_dt"),
            estimate_value: cell_num(cell, "estimate_value"),
            last_est_dt: cell_str(cell, "last_est_dt"),
            discount_rt: cell_num(cell, "discount_rt"),
            index_nm: cell_str(cell, "index_nm"),
            ref_increase_rt: cell_num(cell, "ref_increase_rt"),
            apply_fee: cell_str(cell, "apply_fee"),
            redeem_fee: cell_str(cell, "redeem_fee"),
            mt_fee: cell_num(cell, "mt_fee"),
            issuer_nm: cell_str(cell, "issuer_nm"),
        });
    }
    Ok(out)
}

/// 集思录-T+0 QDII-欧美市场-商品 (`qdii_list/E`).
pub async fn qdii_e_comm_jsl(client: &Client) -> Result<Vec<QdiiECommRow>> {
    let v = client
        .get_json(
            SOURCE,
            "qdii_e_comm_jsl",
            URL_E,
            &[("___jsl", JSL_E), ("rp", RP)],
        )
        .await?;
    parse_qdii_e_comm(&v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Value {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-9,
            None => false,
        }
    }

    #[test]
    fn parse_qdii_e_comm_ok() {
        let rows = parse_qdii_e_comm(&fixture("qdii_e_comm_jsl.json")).unwrap();
        assert_eq!(rows.len(), 3);
        let r = &rows[0];
        assert_eq!(r.fund_id.as_deref(), Some("518800"));
        assert_eq!(r.fund_nm.as_deref(), Some("黄金ETF"));
        assert!(approx(r.price, 3.600));
        assert!(approx(r.increase_rt, 0.50));
        assert!(approx(r.volume, 12345.67));
        assert!(approx(r.amount, 50000.0));
        assert_eq!(r.nav_dt.as_deref(), Some("2026-08-13"));
        assert!(approx(r.fund_nav, 3.580));
        assert!(approx(r.estimate_value, 3.610));
        assert_eq!(r.last_est_dt.as_deref(), Some("2026-08-14"));
        assert!(approx(r.discount_rt, 0.28));
        assert_eq!(r.index_nm.as_deref(), Some("黄金现货"));
        assert!(approx(r.ref_increase_rt, 0.45));
        assert!(approx(r.mt_fee, 0.50));
        assert_eq!(r.issuer_nm.as_deref(), Some("国泰基金"));
        assert_eq!(r.apply_fee.as_deref(), Some("0.50%"));
        assert_eq!(r.redeem_fee.as_deref(), Some("0.50%"));

        // row with '-' cells maps to None
        let none_row = &rows[2];
        assert_eq!(none_row.fund_id.as_deref(), Some("513100"));
        assert!(none_row.estimate_value.is_none());
        assert!(none_row.discount_rt.is_none());
        assert!(none_row.apply_fee.is_none());
        assert!(none_row.redeem_fee.is_none());
    }
}
