//! Eastmoney warehouse-receipt (inventory) data for futures (`futures_inventory_em`).
//!
//! Ports akshare `futures_inventory_em`: two `datacenter-web` report calls. The
//! first resolves the product's `TRADE_CODE` from `RPT_FUTU_POSITIONCODE`, the
//! second fetches `RPT_FUTU_STOCKDATA` (on-warrant inventory + daily change).

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

const BASE: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";

/// One day of warehouse-receipt inventory for a futures product (`futures_inventory_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesInventoryRow {
    pub symbol: String,
    pub date: String,
    pub inventory: Option<f64>,
    pub change: Option<f64>,
    pub source: &'static str,
}

/// Warehouse-receipt inventory for a futures product (Eastmoney `futures_inventory_em`).
///
/// `symbol` is a Chinese product type (e.g. `"豆一"`) or a product code (e.g. `"a"`/`"A"`).
pub async fn futures_inventory(client: &Client, symbol: &str) -> Result<Vec<FuturesInventoryRow>> {
    // Step 1: resolve TRADE_TYPE/TRADE_CODE -> product id.
    let pos_params = [
        ("reportName", "RPT_FUTU_POSITIONCODE"),
        ("columns", "TRADE_MARKET_CODE,TRADE_CODE,TRADE_TYPE"),
        ("filter", "(IS_MAINCODE=\"1\")"),
        ("pageNumber", "1"),
        ("pageSize", "500"),
        ("source", "WEB"),
        ("client", "WEB"),
    ];
    let pos = client
        .get_json(
            SOURCE_EASTMONEY,
            "futures_inventory",
            BASE,
            &pos_params,
        )
        .await?;
    let pos_data = pos
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data in RPT_FUTU_POSITIONCODE".into(),
        })?;

    let mut type_to_code: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut code_to_code: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for item in pos_data {
        let tcode = item.get("TRADE_CODE").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        if let Some(ttype) = item.get("TRADE_TYPE").and_then(|v| v.as_str()) {
            type_to_code.insert(ttype.to_string(), tcode.clone());
        }
        if !tcode.is_empty() {
            code_to_code.insert(tcode.to_lowercase(), tcode);
        }
    }

    let product_id = if let Some(code) = type_to_code.get(symbol) {
        code.clone()
    } else if let Some(code) = code_to_code.get(&symbol.to_lowercase()) {
        code.clone()
    } else {
        return Err(Error::InvalidParam(format!(
            "unknown futures inventory symbol: {symbol}"
        )));
    };

    // Step 2: fetch on-warrant stock data for the product.
    let filter = format!("(SECURITY_CODE=\"{product_id}\")(TRADE_DATE>='2020-10-28')");
    let stock_params = [
        ("reportName", "RPT_FUTU_STOCKDATA"),
        ("columns", "SECURITY_CODE,TRADE_DATE,ON_WARRANT_NUM,ADDCHANGE"),
        ("filter", &filter),
        ("pageNumber", "1"),
        ("pageSize", "500"),
        ("sortTypes", "-1"),
        ("sortColumns", "TRADE_DATE"),
        ("source", "WEB"),
        ("client", "WEB"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "futures_inventory", BASE, &stock_params)
        .await?;
    parse_inventory(&v)
}

pub(crate) fn parse_inventory(resp: &Value) -> Result<Vec<FuturesInventoryRow>> {
    let data = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data in RPT_FUTU_STOCKDATA".into(),
        })?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let obj = match item.as_object() {
            Some(o) => o,
            None => continue,
        };
        if !obj.contains_key("TRADE_DATE") || !obj.contains_key("ON_WARRANT_NUM") {
            continue;
        }
        out.push(FuturesInventoryRow {
            symbol: fstr(item, "SECURITY_CODE"),
            date: fstr(item, "TRADE_DATE"),
            inventory: fnum(item, "ON_WARRANT_NUM"),
            change: fnum(item, "ADDCHANGE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

fn fstr(item: &Value, k: &str) -> String {
    item.get(k)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_futures_inventory_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/futures_inventory.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_inventory(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "A");
        assert_eq!(rows[0].date, "2024-01-02");
        assert_eq!(rows[0].inventory, Some(12345.0));
        assert_eq!(rows[0].change, Some(-100.0));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].date, "2024-01-03");
        assert_eq!(rows[1].inventory, Some(12200.0));
    }
}
