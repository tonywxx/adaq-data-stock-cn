//! Guangzhou Futures Exchange (GFEX) warehouse receipts & daily trading data.
//!
//! Ports two GFEX helpers from akshare:
//! - `futures_gfex_warehouse_receipt` ← `futures_warehouse_receipt.py:159` (POST `data`)
//! - `get_gfex_daily`                ← `futures_daily_bar.py:199`        (POST `data`)
//!
//! Both POST a form body and read a JSON response (no JS signing / HTML /
//! Excel).
//!
//! ## DEFERRED
//! None in this file.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE: &str = "gfex";

/// One GFEX warehouse-receipt row (`futures_gfex_warehouse_receipt`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct GfexReceiptRow {
    pub symbol: String,
    pub variety: String,
    pub warehouse: String,
    pub last_receipt: Option<f64>,
    pub curr_receipt: Option<f64>,
    pub diff: Option<f64>,
}

/// One GFEX daily trading row (`get_gfex_daily`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct GfexDailyRow {
    pub symbol: String,
    pub date: String,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: Option<f64>,
    pub volume: Option<f64>,
    pub open_interest: Option<f64>,
    pub turnover: Option<f64>,
    pub settle: Option<f64>,
    pub pre_settle: Option<f64>,
    pub variety: String,
}

fn to_f64_opt(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => {
            let t = s.trim().replace(',', "");
            if t.is_empty() || t == "\r" {
                None
            } else {
                t.parse::<f64>().ok()
            }
        }
        _ => None,
    }
}

/// Parse `futures_gfex_warehouse_receipt` rows from `{ "data": [...] }`.
pub(crate) fn parse_gfex_receipt(resp: &Value) -> Result<Vec<GfexReceiptRow>> {
    let arr = resp
        .get("data")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing data".into(),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(GfexReceiptRow {
            symbol: item
                .get("varietyOrder")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            variety: item
                .get("variety")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            warehouse: item
                .get("whAbbr")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            last_receipt: to_f64_opt(item.get("lastWbillQty").unwrap_or(&Value::Null)),
            curr_receipt: to_f64_opt(item.get("wbillQty").unwrap_or(&Value::Null)),
            diff: to_f64_opt(item.get("regWbillQty").unwrap_or(&Value::Null)),
        });
    }
    Ok(out)
}

/// GFEX warehouse-receipt daily report (`futures_gfex_warehouse_receipt`).
pub async fn futures_gfex_warehouse_receipt(
    client: &Client,
    date: &str,
) -> Result<Vec<GfexReceiptRow>> {
    let url = "http://www.gfex.com.cn/u/interfacesWebTdWbillWeeklyQuotes/loadList";
    let body = [("gen_date", date)];
    let v = client
        .post_form_json(SOURCE, "futures_gfex_warehouse_receipt", url, &body, None)
        .await?;
    parse_gfex_receipt(&v)
}

/// Parse `get_gfex_daily` rows from `{ "data": [...] }`.
pub(crate) fn parse_gfex_daily(resp: &Value) -> Result<Vec<GfexDailyRow>> {
    let arr = resp
        .get("data")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing data".into(),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let variety_name = item
            .get("variety")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if variety_name.contains("小计") || variety_name.contains("总计") {
            continue;
        }
        let variety_order = item
            .get("varietyOrder")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_uppercase();
        let deliv_month = item
            .get("delivMonth")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        out.push(GfexDailyRow {
            symbol: format!("{variety_order}{deliv_month}"),
            date: String::new(),
            open: to_f64_opt(item.get("open").unwrap_or(&Value::Null)),
            high: to_f64_opt(item.get("high").unwrap_or(&Value::Null)),
            low: to_f64_opt(item.get("low").unwrap_or(&Value::Null)),
            close: to_f64_opt(item.get("close").unwrap_or(&Value::Null)),
            volume: to_f64_opt(item.get("volumn").unwrap_or(&Value::Null)),
            open_interest: to_f64_opt(item.get("openInterest").unwrap_or(&Value::Null)),
            turnover: to_f64_opt(item.get("turnover").unwrap_or(&Value::Null)),
            settle: to_f64_opt(item.get("clearPrice").unwrap_or(&Value::Null)),
            pre_settle: to_f64_opt(item.get("lastClear").unwrap_or(&Value::Null)),
            variety: variety_order,
        });
    }
    Ok(out)
}

/// GFEX daily trading data (`get_gfex_daily`). `date` is `YYYY-MM-DD` or `YYYYMMDD`.
pub async fn get_gfex_daily(client: &Client, date: &str) -> Result<Vec<GfexDailyRow>> {
    let url = "http://www.gfex.com.cn/u/interfacesWebTiDayQuotes/loadList";
    let body = [("trade_date", date), ("trade_type", "0")];
    let v = client
        .post_form_json(SOURCE, "get_gfex_daily", url, &body, None)
        .await?;
    let mut rows = parse_gfex_daily(&v)?;
    for r in &mut rows {
        r.date = date.to_string();
    }
    Ok(rows)
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
    fn parse_gfex_receipt_ok() {
        let rows = parse_gfex_receipt(&fixture("futures_gfex_warehouse_receipt.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "si");
        assert_eq!(rows[0].variety, "工业硅");
        assert!(approx(rows[0].last_receipt, 5000.0));
        assert!(approx(rows[0].curr_receipt, 5200.0));
    }
    #[test]
    fn parse_gfex_daily_ok() {
        let mut rows = parse_gfex_daily(&fixture("get_gfex_daily.json")).unwrap();
        assert_eq!(rows.len(), 2);
        rows[0].date = "20221223".into();
        assert_eq!(rows[0].symbol, "SI2301");
        assert_eq!(rows[0].variety, "SI");
        assert!(approx(rows[0].open, 18000.0));
        assert!(approx(rows[0].settle, 18100.0));
    }
}
