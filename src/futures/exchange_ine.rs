//! Shanghai International Energy Exchange (INE) daily trading data.
//!
//! Ports `get_ine_daily` ← `futures_daily_bar.py:275`.
//!
//! JSON endpoint (`kx{YYYYMMDD}.dat`, `o_curinstrument`); no JS signing,
//! HTML scrape, or Excel/ZIP.
//!
//! ## DEFERRED
//! None in this file.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE: &str = "ine";

/// One INE daily trading row (`get_ine_daily`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct IneDailyRow {
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

/// Parse `get_ine_daily` rows from `{ "o_curinstrument": [...] }`.
pub(crate) fn parse_ine_daily(resp: &Value) -> Result<Vec<IneDailyRow>> {
    let arr = resp
        .get("o_curinstrument")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing o_curinstrument".into(),
        })?;
    // akshare drops the last row (iloc[:-1, :]).
    let n = arr.len().saturating_sub(1);
    let mut out = Vec::with_capacity(n);
    for item in &arr[..n] {
        let delivery_month = item
            .get("DELIVERYMONTH")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if delivery_month == "小计" {
            continue;
        }
        let product_name = item
            .get("PRODUCTNAME")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if product_name.contains("总计") {
            continue;
        }
        let product = item
            .get("PRODUCTGROUPID")
            .and_then(|v| v.as_str())
            .or_else(|| item.get("PRODUCTID").and_then(|v| v.as_str()))
            .unwrap_or_default()
            .trim()
            .to_uppercase();
        let variety = product.split('_').next().unwrap_or("").to_string();
        let symbol = format!("{variety}{delivery_month}");
        if symbol == "总计" || symbol.contains("efp") {
            continue;
        }
        let turnover = item
            .get("TURNOVER")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|_| to_f64_opt(item.get("TURNOVER").unwrap_or(&Value::Null)))
            .unwrap_or(Some(0.0));
        out.push(IneDailyRow {
            symbol,
            date: String::new(),
            open: to_f64_opt(item.get("OPENPRICE").unwrap_or(&Value::Null)),
            high: to_f64_opt(item.get("HIGHESTPRICE").unwrap_or(&Value::Null)),
            low: to_f64_opt(item.get("LOWESTPRICE").unwrap_or(&Value::Null)),
            close: to_f64_opt(item.get("CLOSEPRICE").unwrap_or(&Value::Null)),
            volume: to_f64_opt(item.get("VOLUME").unwrap_or(&Value::Null)),
            open_interest: to_f64_opt(item.get("OPENINTEREST").unwrap_or(&Value::Null)),
            turnover,
            settle: to_f64_opt(item.get("SETTLEMENTPRICE").unwrap_or(&Value::Null)),
            pre_settle: to_f64_opt(item.get("PRESETTLEMENTPRICE").unwrap_or(&Value::Null)),
            variety,
        });
    }
    Ok(out)
}

/// INE daily trading data (`get_ine_daily`). `date` is `YYYYMMDD`.
pub async fn get_ine_daily(client: &Client, date: &str) -> Result<Vec<IneDailyRow>> {
    let url = format!("https://www.ine.cn/data/tradedata/future/dailydata/kx{date}.dat");
    let v = client
        .get_text(SOURCE, "get_ine_daily", &url, &[], None)
        .await?;
    let resp: Value = serde_json::from_str(&v).map_err(Error::Json)?;
    let mut rows = parse_ine_daily(&resp)?;
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
    fn parse_ine_daily_ok() {
        let mut rows = parse_ine_daily(&fixture("get_ine_daily.json")).unwrap();
        assert_eq!(rows.len(), 2);
        rows[0].date = "20241129".into();
        assert_eq!(rows[0].symbol, "SC2412");
        assert_eq!(rows[0].variety, "SC");
        assert!(approx(rows[0].open, 560.0));
        assert!(approx(rows[0].settle, 565.0));
        assert!(approx(rows[0].turnover, 0.0));
    }
}
