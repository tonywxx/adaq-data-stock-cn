//! Shanghai Futures Exchange warehouse-receipt daily report (`futures_shfe_warehouse_receipt`).
//!
//! Ports `futures_shfe_warehouse_receipt` ← `futures_warehouse_receipt.py:104`.
//!
//! For `date >= "20140519"` the endpoint returns pure JSON (`o_cursor`), which
//! we parse faithfully. akshare groups rows into a `dict` keyed by `VARNAME`;
//! we flatten into a single `Vec` with the variety carried on every row,
//! keeping akshare's original (English, `$`-split) column names.
//!
//! Pre-20140519 uses an HTML `.html` endpoint — deferred (HTML scrape).
//!
//! ## DEFERRED
//! * `futures_warehouse_receipt_czce` (`futures_warehouse_receipt.py:23`) — CZCE `.xls`/`.xlsx`
//! * SHFE pre-20140519 HTML branch (not pure JSON)

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE: &str = "shfe";

/// One SHFE warehouse-receipt row (`futures_shfe_warehouse_receipt`).
///
/// Column names mirror akshare's raw DataFrame (after the `$`-split of
/// `VARNAME`/`REGNAME`/`WHABBRNAME`): the part before the first `$` is kept.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShfeWarehouseReceiptRow {
    pub var_name: String,
    pub var_sort: Option<f64>,
    pub reg_name: String,
    pub reg_sort: Option<f64>,
    pub wh_abbr_name: String,
    pub wh_rows: Option<f64>,
    pub wght_unit: Option<f64>,
    pub wrt_wghts: Option<f64>,
    pub wrt_change: Option<f64>,
    pub row_order: Option<f64>,
    pub row_status: Option<f64>,
}

fn to_f64_opt(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => {
            let t = s.trim().replace(',', "");
            if t.is_empty() {
                None
            } else {
                t.parse::<f64>().ok()
            }
        }
        _ => None,
    }
}

/// Take the substring before the first `$` (akshare `.str.split("$", expand=True).iloc[:, 0]`).
fn split_dollar(v: &Value) -> String {
    v.as_str()
        .map(|s| s.split('$').next().unwrap_or(s).to_string())
        .unwrap_or_default()
}

/// Parse `futures_shfe_warehouse_receipt` rows from `{ "o_cursor": [...] }`.
pub(crate) fn parse_shfe_warehouse_receipt(resp: &Value) -> Result<Vec<ShfeWarehouseReceiptRow>> {
    let arr = resp
        .get("o_cursor")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing o_cursor".into(),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(ShfeWarehouseReceiptRow {
            var_name: split_dollar(item.get("VARNAME").unwrap_or(&Value::Null)),
            var_sort: to_f64_opt(item.get("VARSORT").unwrap_or(&Value::Null)),
            reg_name: split_dollar(item.get("REGNAME").unwrap_or(&Value::Null)),
            reg_sort: to_f64_opt(item.get("REGSORT").unwrap_or(&Value::Null)),
            wh_abbr_name: split_dollar(item.get("WHABBRNAME").unwrap_or(&Value::Null)),
            wh_rows: to_f64_opt(item.get("WHROWS").unwrap_or(&Value::Null)),
            wght_unit: to_f64_opt(item.get("WGHTUNIT").unwrap_or(&Value::Null)),
            wrt_wghts: to_f64_opt(item.get("WRTWGHTS").unwrap_or(&Value::Null)),
            wrt_change: to_f64_opt(item.get("WRTCHANGE").unwrap_or(&Value::Null)),
            row_order: to_f64_opt(item.get("ROWORDER").unwrap_or(&Value::Null)),
            row_status: to_f64_opt(item.get("ROWSTATUS").unwrap_or(&Value::Null)),
        });
    }
    Ok(out)
}

/// SHFE warehouse-receipt daily report (`futures_shfe_warehouse_receipt`). `date` is `YYYYMMDD`.
///
/// Pure-JSON path (date >= 20140519). Older dates fall back to an HTML endpoint
/// and will fail to parse as JSON.
pub async fn futures_shfe_warehouse_receipt(
    client: &Client,
    date: &str,
) -> Result<Vec<ShfeWarehouseReceiptRow>> {
    let url = format!(
        "https://www.shfe.com.cn/data/tradedata/future/dailydata/{date}dailystock.dat"
    );
    let v = client
        .get_text(SOURCE, "futures_shfe_warehouse_receipt", &url, &[], None)
        .await?;
    let resp: Value = serde_json::from_str(&v).map_err(Error::Json)?;
    parse_shfe_warehouse_receipt(&resp)
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
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }
    #[test]
    fn parse_shfe_warehouse_receipt_ok() {
        let rows = parse_shfe_warehouse_receipt(&fixture("futures_shfe_warehouse_receipt.json"))
            .unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].var_name, "铜");
        assert_eq!(rows[0].reg_name, "上海");
        assert_eq!(rows[0].wh_abbr_name, "国储天威");
        assert!(approx(rows[0].wrt_wghts, 25.0));
        assert!(approx(rows[0].wrt_change, 0.0));
        assert_eq!(rows[2].var_name, "铜");
        assert_eq!(rows[2].wh_abbr_name, "上港物流");
        assert!(approx(rows[2].wrt_wghts, 3199.0));
        assert!(approx(rows[2].wrt_change, 0.0));
    }
}
