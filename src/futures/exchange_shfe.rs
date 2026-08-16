//! Shanghai Futures Exchange (SHFE) daily / delivery / to-spot data.
//!
//! Ports three SHFE helpers from akshare:
//! - `futures_to_spot_shfe`  ← `futures_to_spot.py:14`   (`ExchangeDelivery{YYYYMM}.dat`, JSON)
//! - `futures_delivery_shfe` ← `futures_to_spot.py:269`  (`{YYYYMMDD}monthvarietystatistics.dat`, JSON)
//! - `get_shfe_daily`        ← `futures_daily_bar.py:453` (`kx{YYYYMMDD}.dat`, JSON `o_curinstrument`)
//!
//! All three return JSON (no JS signing, no HTML scrape, no Excel/ZIP).
//!
//! ## DEFERRED
//! None in this file.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE: &str = "shfe";

/// One SHFE to-spot (期转现) row (`futures_to_spot_shfe`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToSpotShfeRow {
    pub date: String,
    pub symbol: String,
    pub delivery_volume: Option<f64>,
    pub to_spot_volume: Option<f64>,
}

/// One SHFE delivery-statistics row (`futures_delivery_shfe`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct DeliveryShfeRow {
    pub variety: String,
    pub delivery_month: Option<f64>,
    pub delivery_ratio: Option<f64>,
    pub delivery_ytd: Option<f64>,
    pub delivery_ytd_yoy: Option<f64>,
}

/// One SHFE daily trading row (`get_shfe_daily`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShfeDailyRow {
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

/// Parse `futures_to_spot_shfe` rows from `{ "ExchangeDelivery": [...] }`.
pub(crate) fn parse_to_spot_shfe(resp: &Value) -> Result<Vec<ToSpotShfeRow>> {
    let arr = resp
        .get("ExchangeDelivery")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing ExchangeDelivery".into(),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let row = item.as_array().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "row not an array".into(),
        })?;
        // layout: ["_", 日期, 交割量, "_", 期转现量, 合约, "_", "_"]
        let get = |i: usize| row.get(i).and_then(|v| v.as_str()).unwrap_or_default().to_string();
        out.push(ToSpotShfeRow {
            date: get(1),
            symbol: get(5),
            delivery_volume: to_f64_opt(row.get(2).unwrap_or(&Value::Null)),
            to_spot_volume: to_f64_opt(row.get(4).unwrap_or(&Value::Null)),
        });
    }
    Ok(out)
}

/// SHFE to-spot (期转现) data (`futures_to_spot_shfe`). `date` is `YYYYMM`.
pub async fn futures_to_spot_shfe(client: &Client, date: &str) -> Result<Vec<ToSpotShfeRow>> {
    let url = format!("https://tsite.shfe.com.cn/data/instrument/ExchangeDelivery{date}.dat");
    let v = client
        .get_text(SOURCE, "futures_to_spot_shfe", &url, &[], None)
        .await?;
    let resp: Value = serde_json::from_str(&v).map_err(Error::Json)?;
    parse_to_spot_shfe(&resp)
}

/// Parse `futures_delivery_shfe` rows from `{ "o_curdelivery": [...] }`.
pub(crate) fn parse_delivery_shfe(resp: &Value) -> Result<Vec<DeliveryShfeRow>> {
    let arr = resp
        .get("o_curdelivery")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing o_curdelivery".into(),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let row = item.as_array().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "row not an array".into(),
        })?;
        // layout: [品种, 品种代码, "_", 交割量-本月, 交割量-比重, 交割量-本年累计, 交割量-累计同比]
        let get = |i: usize| row.get(i).and_then(|v| v.as_str()).unwrap_or_default().to_string();
        out.push(DeliveryShfeRow {
            variety: get(0),
            delivery_month: to_f64_opt(row.get(3).unwrap_or(&Value::Null)),
            delivery_ratio: to_f64_opt(row.get(4).unwrap_or(&Value::Null)),
            delivery_ytd: to_f64_opt(row.get(5).unwrap_or(&Value::Null)),
            delivery_ytd_yoy: to_f64_opt(row.get(6).unwrap_or(&Value::Null)),
        });
    }
    Ok(out)
}

/// SHFE monthly delivery-statistics table (`futures_delivery_shfe`). `date` is `YYYYMMDD`.
pub async fn futures_delivery_shfe(client: &Client, date: &str) -> Result<Vec<DeliveryShfeRow>> {
    let url = format!("https://tsite.shfe.com.cn/data/dailydata/{date}monthvarietystatistics.dat");
    let v = client
        .get_text(SOURCE, "futures_delivery_shfe", &url, &[], None)
        .await?;
    let resp: Value = serde_json::from_str(&v).map_err(Error::Json)?;
    parse_delivery_shfe(&resp)
}

/// Parse `get_shfe_daily` rows from `{ "o_curinstrument": [...] }`.
pub(crate) fn parse_shfe_daily(resp: &Value) -> Result<Vec<ShfeDailyRow>> {
    let arr = resp
        .get("o_curinstrument")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing o_curinstrument".into(),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let delivery_month = item
            .get("DELIVERYMONTH")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if delivery_month.is_empty() || delivery_month == "小计" || delivery_month == "合计" {
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
        if symbol.contains("efp") {
            continue;
        }
        let vol = item.get("VOLUME").and_then(|v| v.as_str());
        let volume = match vol {
            Some("") => Some(0.0),
            _ => to_f64_opt(item.get("VOLUME").unwrap_or(&Value::Null)),
        };
        let turn = item.get("TURNOVER").and_then(|v| v.as_str());
        let turnover = match turn {
            Some("") => Some(0.0),
            _ => to_f64_opt(item.get("TURNOVER").unwrap_or(&Value::Null)),
        };
        out.push(ShfeDailyRow {
            symbol,
            date: String::new(), // filled in by caller
            open: to_f64_opt(item.get("OPENPRICE").unwrap_or(&Value::Null)),
            high: to_f64_opt(item.get("HIGHESTPRICE").unwrap_or(&Value::Null)),
            low: to_f64_opt(item.get("LOWESTPRICE").unwrap_or(&Value::Null)),
            close: to_f64_opt(item.get("CLOSEPRICE").unwrap_or(&Value::Null)),
            volume,
            open_interest: to_f64_opt(item.get("OPENINTEREST").unwrap_or(&Value::Null)),
            turnover,
            settle: to_f64_opt(item.get("SETTLEMENTPRICE").unwrap_or(&Value::Null)),
            pre_settle: to_f64_opt(item.get("PRESETTLEMENTPRICE").unwrap_or(&Value::Null)),
            variety,
        });
    }
    Ok(out)
}

/// SHFE daily trading data (`get_shfe_daily`). `date` is `YYYYMMDD`.
pub async fn get_shfe_daily(client: &Client, date: &str) -> Result<Vec<ShfeDailyRow>> {
    let url = format!("https://www.shfe.com.cn/data/tradedata/future/dailydata/kx{date}.dat");
    let v = client
        .get_text(SOURCE, "get_shfe_daily", &url, &[], None)
        .await?;
    let resp: Value = serde_json::from_str(&v).map_err(Error::Json)?;
    let mut rows = parse_shfe_daily(&resp)?;
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
    fn parse_to_spot_shfe_ok() {
        let rows = parse_to_spot_shfe(&fixture("futures_to_spot_shfe.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "20231201");
        assert_eq!(rows[0].symbol, "cu2312");
        assert!(approx(rows[0].delivery_volume, 100.0));
        assert!(approx(rows[0].to_spot_volume, 50.0));
    }
    #[test]
    fn parse_delivery_shfe_ok() {
        let rows = parse_delivery_shfe(&fixture("futures_delivery_shfe.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].variety, "铜");
        assert!(approx(rows[0].delivery_month, 100.0));
        assert!(approx(rows[0].delivery_ratio, 5.2));
    }
    #[test]
    fn parse_shfe_daily_ok() {
        let mut rows = parse_shfe_daily(&fixture("get_shfe_daily.json")).unwrap();
        // 小计/efp rows filtered out -> expect 2
        assert_eq!(rows.len(), 2);
        rows[0].date = "20220415".into();
        assert_eq!(rows[0].symbol, "CU2204");
        assert_eq!(rows[0].variety, "CU");
        assert!(approx(rows[0].open, 70000.0));
        assert!(approx(rows[0].volume, 123.0));
        assert!(approx(rows[0].turnover, 8600.0));
        assert!(!rows[1].symbol.contains("efp"));
    }
}
