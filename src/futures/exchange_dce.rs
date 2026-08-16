//! Dalian Commodity Exchange (DCE) warehouse receipts & daily trading data.
//!
//! Ports two DCE helpers from akshare:
//! - `futures_warehouse_receipt_dce` ← `futures_warehouse_receipt.py:61` (POST JSON `data.entityList`)
//! - `get_dce_daily`                ← `futures_daily_bar.py:527`        (POST JSON `data`)
//!
//! Both POST a JSON body and read a JSON response (no JS signing / HTML /
//! Excel). DCE variety names are Chinese; akshare maps them to English codes
//! via `cons.DCE_MAP`, replicated here.
//!
//! ## DEFERRED
//! None in this file.

use serde_json::{json, Value};

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE: &str = "dce";

/// DCE Chinese-variety-name → English code (`cons.DCE_MAP`).
const DCE_MAP: &[(&str, &str)] = &[
    ("大豆", "A"),
    ("豆一", "A"),
    ("豆二", "B"),
    ("豆粕", "M"),
    ("豆油", "Y"),
    ("棕榈油", "P"),
    ("玉米", "C"),
    ("玉米淀粉", "CS"),
    ("鸡蛋", "JD"),
    ("纤维板", "FB"),
    ("胶合板", "BB"),
    ("聚乙烯", "L"),
    ("聚氯乙烯", "V"),
    ("聚丙烯", "PP"),
    ("焦炭", "J"),
    ("焦煤", "JM"),
    ("铁矿石", "I"),
    ("乙二醇", "EG"),
    ("粳米", "RR"),
    ("苯乙烯", "EB"),
    ("液化石油气", "PG"),
    ("生猪", "LH"),
    ("原木", "LG"),
    ("纯苯", "BZ"),
];

fn dce_code(variety: &str) -> String {
    for (cn, code) in DCE_MAP {
        if *cn == variety {
            return code.to_string();
        }
    }
    variety.to_string()
}

/// One DCE warehouse-receipt row (`futures_warehouse_receipt_dce`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct DceReceiptRow {
    pub variety_code: String,
    pub variety_name: String,
    pub warehouse: String,
    pub delivery_point: String,
    pub last_receipt: Option<f64>,
    pub curr_receipt: Option<f64>,
    pub diff: Option<f64>,
}

/// One DCE daily trading row (`get_dce_daily`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct DceDailyRow {
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

/// Parse `futures_warehouse_receipt_dce` rows from `{ "data": { "entityList": [...] } }`.
pub(crate) fn parse_dce_receipt(resp: &Value) -> Result<Vec<DceReceiptRow>> {
    let list = resp
        .get("data")
        .and_then(|d| d.get("entityList"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing data.entityList".into(),
        })?;
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        out.push(DceReceiptRow {
            variety_code: item
                .get("varietyOrder")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            variety_name: item
                .get("variety")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            warehouse: item
                .get("whAbbr")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            delivery_point: item
                .get("deliveryAbbr")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            last_receipt: to_f64_opt(item.get("lastWbillQty").unwrap_or(&Value::Null)),
            curr_receipt: to_f64_opt(item.get("wbillQty").unwrap_or(&Value::Null)),
            diff: to_f64_opt(item.get("diff").unwrap_or(&Value::Null)),
        });
    }
    Ok(out)
}

/// DCE warehouse-receipt daily report (`futures_warehouse_receipt_dce`).
pub async fn futures_warehouse_receipt_dce(client: &Client, date: &str) -> Result<Vec<DceReceiptRow>> {
    let url = "http://www.dce.com.cn/dcereport/publicweb/dailystat/wbillWeeklyQuotes";
    let body = json!({ "tradeDate": date, "varietyId": "all" });
    let v = client.post_json(SOURCE, "futures_warehouse_receipt_dce", url, &body, None).await?;
    parse_dce_receipt(&v)
}

/// Parse `get_dce_daily` rows from `{ "data": [...] }`.
pub(crate) fn parse_dce_daily(resp: &Value) -> Result<Vec<DceDailyRow>> {
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
        let symbol = item
            .get("contractId")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        out.push(DceDailyRow {
            symbol,
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
            variety: dce_code(variety_name),
        });
    }
    Ok(out)
}

/// DCE daily trading data (`get_dce_daily`). `date` is `YYYY-MM-DD` or `YYYYMMDD`.
pub async fn get_dce_daily(client: &Client, date: &str) -> Result<Vec<DceDailyRow>> {
    let url = "http://www.dce.com.cn/dcereport/publicweb/dailystat/dayQuotes";
    let body = json!({
        "contractId": "",
        "lang": "zh",
        "optionSeries": "",
        "statisticsType": "0",
        "tradeDate": date,
        "tradeType": "1",
        "varietyId": "all",
    });
    let v = client.post_json(SOURCE, "get_dce_daily", url, &body, None).await?;
    let mut rows = parse_dce_daily(&v)?;
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
    fn parse_dce_receipt_ok() {
        let rows = parse_dce_receipt(&fixture("futures_warehouse_receipt_dce.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].variety_code, "a");
        assert_eq!(rows[0].variety_name, "大豆");
        assert!(approx(rows[0].last_receipt, 1000.0));
        assert!(approx(rows[0].curr_receipt, 1200.0));
        assert!(approx(rows[0].diff, 200.0));
    }
    #[test]
    fn parse_dce_daily_ok() {
        let mut rows = parse_dce_daily(&fixture("get_dce_daily.json")).unwrap();
        assert_eq!(rows.len(), 2);
        rows[0].date = "20251027".into();
        assert_eq!(rows[0].symbol, "a2511");
        assert_eq!(rows[0].variety, "A");
        assert!(approx(rows[0].open, 3900.0));
        assert!(approx(rows[0].settle, 3920.0));
    }
}
