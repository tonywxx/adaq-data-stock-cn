//! US macro indicators from akshare `economic/macro_usa.py`.
//!
//! akshare's `macro_usa.py` is dominated by the Jin10 datacenter. Most public
//! functions (`macro_usa_gdp_monthly`, `macro_usa_unemployment_rate`,
//! `macro_usa_pmi`, `macro_usa_ppi`, `macro_usa_non_farm`, ... ~40 of them)
//! route through the private `__macro_usa_base_func`, which issues a
//! `GET` to `datacenter-api.jin10.com/reports/list_v2` with `x-app-id` and
//! `x-csrf-token` headers. Those need a token/signed header, so they are
//! DEFERRED (see the report) and not ported here.
//!
//! Two functions hit the **Eastmoney** datacenter
//! (`RPT_ECONOMICVALUE_USA`): `macro_usa_cpi_yoy` (`EMG00000733`) and
//! `macro_usa_phs` (`EMG00342249`). Those are already ported in
//! `macro2.rs`, so to avoid duplicate definitions they are SKIPPED here.
//!
//! The remaining seven functions are **pure HTTP**: a single `requests.get`
//! to a `cdn.jin10.com` JSON document with no token, no JS, no signature.
//! They are ported below. Note the response envelope is `{"values": ...}`
//! (optionally with a `keys` array for the CFTC reports) — NOT the
//! Eastmoney `result.data` envelope used by `macro2.rs` / `macro_intl.rs`.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// Source bucket for these `cdn.jin10.com` endpoints (no token required).
const SOURCE_JIN10: &str = "jin10";

/// Required by the porting spec; unused here because every function in this
/// file targets the Jin10 CDN rather than the Eastmoney datacenter.
#[allow(dead_code)]
const SOURCE_EASTMONEY: &str = "eastmoney";

/// Base for the Jin10 data-center JSON reports.
const BASE: &str = "https://cdn.jin10.com/data_center/reports";

/// Extract the `values` object from a `cdn.jin10.com` response.
fn jin10_values(resp: &Value) -> Result<&serde_json::Map<String, Value>> {
    resp.get("values")
        .and_then(|v| v.as_object())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_JIN10,
            message: "missing values".into(),
        })
}

/// Read an integer field by object key (mirrors `macro_intl.rs`). Unused in
/// this module (no integer columns), kept for API parity.
#[allow(dead_code)]
fn inum(item: &Value, k: &str) -> Option<i64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    })
}

/// Parse a JSON scalar into `f64`, tolerating string-encoded numbers.
fn as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// macro_usa_rig_count — Baker Hughes rig count (baker.json)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct RigCount {
    /// Row index as reported by akshare (`日期`, 0-based position in the series).
    pub date: String,
    /// Total rig count (akshare `钻井总数_钻井数`).
    pub total_count: Option<f64>,
    /// Total rig count change vs prior week (akshare `钻井总数_变化`).
    pub total_change: Option<f64>,
    /// US oil rig count (akshare `美国石油钻井_钻井数`).
    pub oil_count: Option<f64>,
    /// US oil rig count change (akshare `美国石油钻井_变化`).
    pub oil_change: Option<f64>,
    /// Mixed rig count (akshare `混合钻井_钻井数`).
    pub mixed_count: Option<f64>,
    /// Mixed rig count change (akshare `混合钻井_变化`).
    pub mixed_change: Option<f64>,
    /// US natural-gas rig count (akshare `美国天然气钻井_钻井数`).
    pub gas_count: Option<f64>,
    /// US natural-gas rig count change (akshare `美国天然气钻井_变化`).
    pub gas_change: Option<f64>,
    pub source: &'static str,
}

/// US Baker Hughes rig count (`macro_usa_rig_count`, akshare `macro_usa.py:466`).
pub async fn macro_usa_rig_count(client: &Client) -> Result<Vec<RigCount>> {
    let url = format!("{BASE}/baker.json");
    let v = client
        .get_json(SOURCE_JIN10, "macro_usa_rig_count", &url, &[("_", "1")])
        .await?;
    parse_macro_usa_rig_count(&v)
}

pub(crate) fn parse_macro_usa_rig_count(resp: &Value) -> Result<Vec<RigCount>> {
    let values = jin10_values(resp)?;
    let total = values.get("钻井总数").and_then(|v| v.as_array());
    let oil = values.get("美国石油钻井").and_then(|v| v.as_array());
    let mixed = values.get("混合钻井").and_then(|v| v.as_array());
    let gas = values.get("美国天然气钻井").and_then(|v| v.as_array());
    let n = total.map(|a| a.len()).unwrap_or(0);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(RigCount {
            date: i.to_string(),
            total_count: total.and_then(|a| a.get(i)).and_then(|c| c.get(0)).and_then(as_f64),
            total_change: total.and_then(|a| a.get(i)).and_then(|c| c.get(1)).and_then(as_f64),
            oil_count: oil.and_then(|a| a.get(i)).and_then(|c| c.get(0)).and_then(as_f64),
            oil_change: oil.and_then(|a| a.get(i)).and_then(|c| c.get(1)).and_then(as_f64),
            mixed_count: mixed.and_then(|a| a.get(i)).and_then(|c| c.get(0)).and_then(as_f64),
            mixed_change: mixed.and_then(|a| a.get(i)).and_then(|c| c.get(1)).and_then(as_f64),
            gas_count: gas.and_then(|a| a.get(i)).and_then(|c| c.get(0)).and_then(as_f64),
            gas_change: gas.and_then(|a| a.get(i)).and_then(|c| c.get(1)).and_then(as_f64),
            source: SOURCE_JIN10,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// macro_usa_crude_inner — US crude oil production (usa_oil.json)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct CrudeInner {
    /// Row index as reported by akshare (`日期`, 0-based position in the series).
    pub date: String,
    /// Total domestic crude output (akshare `美国国内原油总量-产量`).
    pub domestic_total_output: Option<f64>,
    /// Total domestic crude output change (akshare `美国国内原油总量-变化`).
    pub domestic_total_change: Option<f64>,
    /// Lower-48 crude output (akshare `美国本土48州原油产量-产量`).
    pub lower48_output: Option<f64>,
    /// Lower-48 crude output change (akshare `美国本土48州原油产量-变化`).
    pub lower48_change: Option<f64>,
    /// Alaska crude output (akshare `美国阿拉斯加州原油产量-产量`).
    pub alaska_output: Option<f64>,
    /// Alaska crude output change (akshare `美国阿拉斯加州原油产量-变化`).
    pub alaska_change: Option<f64>,
    pub source: &'static str,
}

/// US domestic crude oil production (`macro_usa_crude_inner`, akshare `macro_usa.py:961`).
pub async fn macro_usa_crude_inner(client: &Client) -> Result<Vec<CrudeInner>> {
    let url = format!("{BASE}/usa_oil.json");
    let v = client
        .get_json(SOURCE_JIN10, "macro_usa_crude_inner", &url, &[("_", "1")])
        .await?;
    parse_macro_usa_crude_inner(&v)
}

pub(crate) fn parse_macro_usa_crude_inner(resp: &Value) -> Result<Vec<CrudeInner>> {
    let values = jin10_values(resp)?;
    let domestic = values.get("美国国内原油总量").and_then(|v| v.as_array());
    let lower48 = values.get("美国本土48州原油产量").and_then(|v| v.as_array());
    let alaska = values.get("美国阿拉斯加州原油产量").and_then(|v| v.as_array());
    let n = domestic.map(|a| a.len()).unwrap_or(0);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(CrudeInner {
            date: i.to_string(),
            domestic_total_output: domestic.and_then(|a| a.get(i)).and_then(|c| c.get(0)).and_then(as_f64),
            domestic_total_change: domestic.and_then(|a| a.get(i)).and_then(|c| c.get(1)).and_then(as_f64),
            lower48_output: lower48.and_then(|a| a.get(i)).and_then(|c| c.get(0)).and_then(as_f64),
            lower48_change: lower48.and_then(|a| a.get(i)).and_then(|c| c.get(1)).and_then(as_f64),
            alaska_output: alaska.and_then(|a| a.get(i)).and_then(|c| c.get(0)).and_then(as_f64),
            alaska_change: alaska.and_then(|a| a.get(i)).and_then(|c| c.get(1)).and_then(as_f64),
            source: SOURCE_JIN10,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// CFTC reports — long-format (one row per symbol x key x time point)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct CftcHolding {
    /// Row index as reported by akshare (`日期`, 0-based position in the series).
    pub date: String,
    /// Instrument / currency name (akshare index label).
    pub symbol: String,
    /// Metric name from the report's `keys` array (e.g. `净持仓`).
    pub metric: String,
    /// Reported value (akshare column `<symbol>-<key>`).
    pub value: Option<f64>,
    pub source: &'static str,
}

/// Shared parser for the four CFTC reports (`cftc_1..4.json`). Each report's
/// `values` maps a symbol to a list of `[v0, v1, v2]` records; the `keys`
/// array names each position. akshare flattens these into `<symbol>-<key>`
/// columns — we normalise to long format instead.
fn parse_cftc(resp: &Value) -> Result<Vec<CftcHolding>> {
    let values = jin10_values(resp)?;
    let keys = resp
        .get("keys")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_JIN10,
            message: "missing keys".into(),
        })?;
    let mut out = Vec::new();
    for (symbol, arr) in values {
        let Some(records) = arr.as_array() else {
            continue;
        };
        for (t, rec) in records.iter().enumerate() {
            for (k, key) in keys.iter().enumerate() {
                let metric = key
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let value = rec.get(k).and_then(as_f64);
                out.push(CftcHolding {
                    date: t.to_string(),
                    symbol: symbol.clone(),
                    metric,
                    value,
                    source: SOURCE_JIN10,
                });
            }
        }
    }
    Ok(out)
}

/// US CFTC forex non-commercial positions (`macro_usa_cftc_nc_holding`, akshare `macro_usa.py:997`).
pub async fn macro_usa_cftc_nc_holding(client: &Client) -> Result<Vec<CftcHolding>> {
    let url = format!("{BASE}/cftc_4.json");
    let v = client
        .get_json(SOURCE_JIN10, "macro_usa_cftc_nc_holding", &url, &[("_", "1")])
        .await?;
    parse_macro_usa_cftc_nc_holding(&v)
}

pub(crate) fn parse_macro_usa_cftc_nc_holding(resp: &Value) -> Result<Vec<CftcHolding>> {
    parse_cftc(resp)
}

/// US CFTC commodity non-commercial positions (`macro_usa_cftc_c_holding`, akshare `macro_usa.py:1026`).
pub async fn macro_usa_cftc_c_holding(client: &Client) -> Result<Vec<CftcHolding>> {
    let url = format!("{BASE}/cftc_2.json");
    let v = client
        .get_json(SOURCE_JIN10, "macro_usa_cftc_c_holding", &url, &[("_", "1")])
        .await?;
    parse_macro_usa_cftc_c_holding(&v)
}

pub(crate) fn parse_macro_usa_cftc_c_holding(resp: &Value) -> Result<Vec<CftcHolding>> {
    parse_cftc(resp)
}

/// US CFTC forex commercial positions (`macro_usa_cftc_merchant_currency_holding`, akshare `macro_usa.py:1055`).
pub async fn macro_usa_cftc_merchant_currency_holding(client: &Client) -> Result<Vec<CftcHolding>> {
    let url = format!("{BASE}/cftc_3.json");
    let v = client
        .get_json(
            SOURCE_JIN10,
            "macro_usa_cftc_merchant_currency_holding",
            &url,
            &[("_", "1")],
        )
        .await?;
    parse_macro_usa_cftc_merchant_currency_holding(&v)
}

pub(crate) fn parse_macro_usa_cftc_merchant_currency_holding(resp: &Value) -> Result<Vec<CftcHolding>> {
    parse_cftc(resp)
}

/// US CFTC commodity commercial positions (`macro_usa_cftc_merchant_goods_holding`, akshare `macro_usa.py:1084`).
pub async fn macro_usa_cftc_merchant_goods_holding(client: &Client) -> Result<Vec<CftcHolding>> {
    let url = format!("{BASE}/cftc_1.json");
    let v = client
        .get_json(
            SOURCE_JIN10,
            "macro_usa_cftc_merchant_goods_holding",
            &url,
            &[("_", "1")],
        )
        .await?;
    parse_macro_usa_cftc_merchant_goods_holding(&v)
}

pub(crate) fn parse_macro_usa_cftc_merchant_goods_holding(resp: &Value) -> Result<Vec<CftcHolding>> {
    parse_cftc(resp)
}

// ---------------------------------------------------------------------------
// macro_usa_cme_merchant_goods_holding — CME precious metals (cme_3.json)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct CmeHolding {
    /// Report date (akshare `日期`).
    pub date: String,
    /// Variety, i.e. `pz-tc` (akshare `品种`).
    pub variety: String,
    /// Volume (akshare `成交量`).
    pub volume: Option<f64>,
    pub source: &'static str,
}

/// CME precious-metals open interest (`macro_usa_cme_merchant_goods_holding`, akshare `macro_usa.py:1113`).
pub async fn macro_usa_cme_merchant_goods_holding(client: &Client) -> Result<Vec<CmeHolding>> {
    let url = format!("{BASE}/cme_3.json");
    let v = client
        .get_json(
            SOURCE_JIN10,
            "macro_usa_cme_merchant_goods_holding",
            &url,
            &[("_", "1")],
        )
        .await?;
    parse_macro_usa_cme_merchant_goods_holding(&v)
}

pub(crate) fn parse_macro_usa_cme_merchant_goods_holding(resp: &Value) -> Result<Vec<CmeHolding>> {
    let values = jin10_values(resp)?;
    let mut out = Vec::new();
    for (date, records) in values {
        let Some(records) = records.as_array() else {
            continue;
        };
        for rec in records {
            let pz = rec.get(0).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let tc = rec.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let volume = rec.get(5).and_then(as_f64);
            out.push(CmeHolding {
                date: date.clone(),
                variety: format!("{pz}-{tc}"),
                volume,
                source: SOURCE_JIN10,
            });
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// tests
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
    fn parses_macro_usa_rig_count() {
        let rows = parse_macro_usa_rig_count(&fixture("macro_usa_rig_count.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "0");
        assert_eq!(rows[0].total_count, Some(800.0));
        assert_eq!(rows[0].total_change, Some(5.0));
        assert_eq!(rows[0].gas_count, Some(110.0));
    }

    #[test]
    fn parses_macro_usa_crude_inner() {
        let rows = parse_macro_usa_crude_inner(&fixture("macro_usa_crude_inner.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "0");
        assert_eq!(rows[0].domestic_total_output, Some(13000.0));
        assert_eq!(rows[0].lower48_change, Some(2.0));
        assert_eq!(rows[1].alaska_output, Some(400.0));
    }

    #[test]
    fn parses_macro_usa_cftc_nc_holding() {
        let rows = parse_macro_usa_cftc_nc_holding(&fixture("macro_usa_cftc_nc_holding.json")).unwrap();
        assert_eq!(rows.len(), 12);
        assert_eq!(rows[0].symbol, "欧元");
        assert_eq!(rows[0].metric, "净持仓");
        assert_eq!(rows[0].value, Some(100.0));
        assert_eq!(rows[2].metric, "空头持仓");
        assert_eq!(rows[2].value, Some(300.0));
    }

    #[test]
    fn parses_macro_usa_cftc_c_holding() {
        let rows = parse_macro_usa_cftc_c_holding(&fixture("macro_usa_cftc_c_holding.json")).unwrap();
        assert_eq!(rows.len(), 12);
        assert_eq!(rows[0].symbol, "欧元");
        assert_eq!(rows[0].value, Some(100.0));
        assert_eq!(rows[11].symbol, "英镑");
        assert_eq!(rows[11].value, Some(75.0));
    }

    #[test]
    fn parses_macro_usa_cftc_merchant_currency_holding() {
        let rows =
            parse_macro_usa_cftc_merchant_currency_holding(&fixture("macro_usa_cftc_merchant_currency_holding.json"))
                .unwrap();
        assert_eq!(rows.len(), 12);
        assert_eq!(rows[0].symbol, "欧元");
        assert_eq!(rows[6].date, "0");
        assert_eq!(rows[6].value, Some(50.0));
    }

    #[test]
    fn parses_macro_usa_cftc_merchant_goods_holding() {
        let rows =
            parse_macro_usa_cftc_merchant_goods_holding(&fixture("macro_usa_cftc_merchant_goods_holding.json"))
                .unwrap();
        assert_eq!(rows.len(), 12);
        assert_eq!(rows[0].symbol, "欧元");
        assert_eq!(rows[0].metric, "净持仓");
        assert_eq!(rows[11].value, Some(75.0));
    }

    #[test]
    fn parses_macro_usa_cme_merchant_goods_holding() {
        let rows =
            parse_macro_usa_cme_merchant_goods_holding(&fixture("macro_usa_cme_merchant_goods_holding.json"))
                .unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].date, "2024-01-01");
        assert_eq!(rows[0].variety, "黄金-GC");
        assert_eq!(rows[0].volume, Some(1000.0));
        assert_eq!(rows[2].variety, "黄金-GC");
        assert_eq!(rows[2].volume, Some(1100.0));
    }
}
