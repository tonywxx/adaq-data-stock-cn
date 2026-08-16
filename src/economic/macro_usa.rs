//! US macro indicators from akshare `economic/macro_usa.py`.
//!
//! akshare's `macro_usa.py` is dominated by the Jin10 datacenter. ~40 public
//! functions (`macro_usa_gdp_monthly`, `macro_usa_unemployment_rate`,
//! `macro_usa_pmi`, `macro_usa_ppi`, `macro_usa_non_farm`, ...) route through
//! the private `__macro_usa_base_func`, which issues a `GET` to
//! `datacenter-api.jin10.com/reports/list_v2`. That endpoint accepts the fixed
//! public `x-app-id: rU6QIu7JHe2gOUeR` header (no real token / JS execution
//! required), so all ~40 are fully portable — see `macro_usa_list!` below.
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

use chrono::{Duration, NaiveDate, Utc};
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
            total_count: total
                .and_then(|a| a.get(i))
                .and_then(|c| c.get(0))
                .and_then(as_f64),
            total_change: total
                .and_then(|a| a.get(i))
                .and_then(|c| c.get(1))
                .and_then(as_f64),
            oil_count: oil
                .and_then(|a| a.get(i))
                .and_then(|c| c.get(0))
                .and_then(as_f64),
            oil_change: oil
                .and_then(|a| a.get(i))
                .and_then(|c| c.get(1))
                .and_then(as_f64),
            mixed_count: mixed
                .and_then(|a| a.get(i))
                .and_then(|c| c.get(0))
                .and_then(as_f64),
            mixed_change: mixed
                .and_then(|a| a.get(i))
                .and_then(|c| c.get(1))
                .and_then(as_f64),
            gas_count: gas
                .and_then(|a| a.get(i))
                .and_then(|c| c.get(0))
                .and_then(as_f64),
            gas_change: gas
                .and_then(|a| a.get(i))
                .and_then(|c| c.get(1))
                .and_then(as_f64),
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
    let lower48 = values
        .get("美国本土48州原油产量")
        .and_then(|v| v.as_array());
    let alaska = values
        .get("美国阿拉斯加州原油产量")
        .and_then(|v| v.as_array());
    let n = domestic.map(|a| a.len()).unwrap_or(0);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(CrudeInner {
            date: i.to_string(),
            domestic_total_output: domestic
                .and_then(|a| a.get(i))
                .and_then(|c| c.get(0))
                .and_then(as_f64),
            domestic_total_change: domestic
                .and_then(|a| a.get(i))
                .and_then(|c| c.get(1))
                .and_then(as_f64),
            lower48_output: lower48
                .and_then(|a| a.get(i))
                .and_then(|c| c.get(0))
                .and_then(as_f64),
            lower48_change: lower48
                .and_then(|a| a.get(i))
                .and_then(|c| c.get(1))
                .and_then(as_f64),
            alaska_output: alaska
                .and_then(|a| a.get(i))
                .and_then(|c| c.get(0))
                .and_then(as_f64),
            alaska_change: alaska
                .and_then(|a| a.get(i))
                .and_then(|c| c.get(1))
                .and_then(as_f64),
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
    let keys =
        resp.get("keys")
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
        .get_json(
            SOURCE_JIN10,
            "macro_usa_cftc_nc_holding",
            &url,
            &[("_", "1")],
        )
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
        .get_json(
            SOURCE_JIN10,
            "macro_usa_cftc_c_holding",
            &url,
            &[("_", "1")],
        )
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

pub(crate) fn parse_macro_usa_cftc_merchant_currency_holding(
    resp: &Value,
) -> Result<Vec<CftcHolding>> {
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

pub(crate) fn parse_macro_usa_cftc_merchant_goods_holding(
    resp: &Value,
) -> Result<Vec<CftcHolding>> {
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
            let pz = rec
                .get(0)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let tc = rec
                .get(1)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
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
// macro_usa_* — Jin10 datacenter `reports/list_v2` indicators (~40 functions)
// ---------------------------------------------------------------------------
//
// These route through akshare's private `__macro_usa_base_func`. The endpoint
// `datacenter-api.jin10.com/reports/list_v2` accepts the fixed public
// `x-app-id: rU6QIu7JHe2gOUeR` header (no auth / JS needed), so all ~40 are
// portable. Each public function differs only by `attr_id`; every one shares a
// single output shape (`商品`, `日期`, `今值`, `预测值`, `前值`). The endpoint
// paginates via a descending `max_date`.

/// Jin10 datacenter list_v2 endpoint (US economic indicators).
const JIN10_LIST_V2: &str = "https://datacenter-api.jin10.com/reports/list_v2";

/// Fixed public app id required by the Jin10 datacenter API (no auth needed).
const JIN10_APP_ID: &str = "rU6QIu7JHe2gOUeR";

/// Headers required by the Jin10 datacenter `list_v2` endpoint.
const JIN10_LIST_HEADERS: &[(&str, &str)] = &[
    (
        "user-agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36",
    ),
    ("x-app-id", JIN10_APP_ID),
    ("x-csrf-token", "x-csrf-token"),
    ("x-version", "1.0.0"),
];

/// One US macroeconomic indicator observation from Jin10 `reports/list_v2`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MacroUsaIndicatorRow {
    /// Indicator label (akshare `商品`, e.g. `美国CPI月率`).
    pub commodity: String,
    /// Report date (`YYYY-MM-DD`).
    pub date: Option<String>,
    /// 今值 — actual value.
    pub value: Option<f64>,
    /// 预测值 — market forecast.
    pub forecast: Option<f64>,
    /// 前值 — previous value.
    pub previous: Option<f64>,
}

/// Parse a `data.values` array (`[[date, 今值, 预测值, 前值], ...]`) for one
/// indicator into [`MacroUsaIndicatorRow`]s.
pub(crate) fn parse_jin10_list_values(values: &[Value], symbol: &str) -> Vec<MacroUsaIndicatorRow> {
    let mut out = Vec::with_capacity(values.len());
    for row in values {
        let arr = match row.as_array() {
            Some(a) => a,
            None => continue,
        };
        let date = arr.first().and_then(|v| v.as_str()).map(|s| s.to_string());
        if date.as_deref().map_or(true, |d| d.is_empty()) {
            continue;
        }
        out.push(MacroUsaIndicatorRow {
            commodity: symbol.to_string(),
            date,
            value: arr.get(1).and_then(as_f64),
            forecast: arr.get(2).and_then(as_f64),
            previous: arr.get(3).and_then(as_f64),
        });
    }
    out
}

/// Subtract one day from a `YYYY-MM-DD` date string.
fn prev_day(date: &str) -> Result<String> {
    let d = NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|e| Error::Parse {
        endpoint: "macro_usa",
        message: format!("invalid date {date}: {e}"),
    })?;
    Ok((d - Duration::days(1)).format("%Y-%m-%d").to_string())
}

/// Fetch + paginate all rows for one Jin10 `list_v2` indicator.
async fn fetch_jin10_list(
    client: &Client,
    symbol: &'static str,
    attr_id: &'static str,
) -> Result<Vec<MacroUsaIndicatorRow>> {
    let mut max_date = String::new();
    let mut all: Vec<MacroUsaIndicatorRow> = Vec::new();
    for _ in 0..200 {
        let ts = Utc::now().timestamp_millis().to_string();
        let params: &[(&str, &str)] = &[
            ("max_date", max_date.as_str()),
            ("category", "ec"),
            ("attr_id", attr_id),
            ("_", ts.as_str()),
        ];
        let resp = client
            .get_json_with_headers(SOURCE_JIN10, symbol, JIN10_LIST_V2, params, Some(JIN10_LIST_HEADERS))
            .await?;
        let values = match resp
            .get("data")
            .and_then(|d| d.get("values"))
            .and_then(|v| v.as_array())
        {
            Some(v) => v,
            None => break,
        };
        if values.is_empty() {
            break;
        }
        let rows = parse_jin10_list_values(values, symbol);
        let last_date = rows.last().and_then(|r| r.date.clone());
        all.extend(rows);
        match last_date {
            Some(d) => max_date = prev_day(&d)?,
            None => break,
        }
    }
    all.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(all)
}

macro_rules! macro_usa_list {
    ($($name:ident => ($symbol:literal, $attr:literal)),* $(,)?) => {
        $(
            #[doc = concat!("美国 ", $symbol, " — Jin10 `reports/list_v2` (akshare `macro_usa.py`).")]
            pub async fn $name(client: &Client) -> Result<Vec<MacroUsaIndicatorRow>> {
                fetch_jin10_list(client, $symbol, $attr).await
            }
        )*
    };
}

macro_usa_list! {
    macro_usa_gdp_monthly => ("美国国内生产总值(GDP)", "53"),
    macro_usa_cpi_monthly => ("美国CPI月率", "9"),
    macro_usa_core_cpi_monthly => ("美国核心CPI月率", "6"),
    macro_usa_personal_spending => ("美国个人支出月率", "35"),
    macro_usa_retail_sales => ("美国零售销售月率", "39"),
    macro_usa_import_price => ("美国进口物价指数", "18"),
    macro_usa_export_price => ("美国出口价格指数", "79"),
    macro_usa_lmci => ("美联储劳动力市场状况指数", "93"),
    macro_usa_unemployment_rate => ("美国失业率", "47"),
    macro_usa_job_cuts => ("美国挑战者企业裁员人数", "78"),
    macro_usa_non_farm => ("美国非农就业人数", "33"),
    macro_usa_adp_employment => ("美国ADP就业人数", "1"),
    macro_usa_core_pce_price => ("美国核心PCE物价指数年率", "80"),
    macro_usa_real_consumer_spending => ("美国实际个人消费支出季率初值", "81"),
    macro_usa_trade_balance => ("美国贸易帐报告", "42"),
    macro_usa_current_account => ("美国经常账报告", "12"),
    macro_usa_ppi => ("美国生产者物价指数", "37"),
    macro_usa_core_ppi => ("美国核心生产者物价指数", "7"),
    macro_usa_api_crude_stock => ("美国API原油库存", "69"),
    macro_usa_pmi => ("美国Markit制造业PMI报告", "74"),
    macro_usa_ism_pmi => ("美国ISM制造业PMI报告", "28"),
    macro_usa_industrial_production => ("美国工业产出月率报告", "20"),
    macro_usa_durable_goods_orders => ("美国耐用品订单月率报告", "13"),
    macro_usa_factory_orders => ("美国工厂订单月率报告", "16"),
    macro_usa_services_pmi => ("美国Markit服务业PMI初值报告", "89"),
    macro_usa_business_inventories => ("美国商业库存月率报告", "4"),
    macro_usa_ism_non_pmi => ("美国ISM非制造业PMI报告", "29"),
    macro_usa_nahb_house_market_index => ("美国NAHB房产市场指数报告", "31"),
    macro_usa_house_starts => ("美国新屋开工总数年化报告", "17"),
    macro_usa_new_home_sales => ("美国新屋销售总数年化报告", "32"),
    macro_usa_building_permits => ("美国营建许可总数报告", "3"),
    macro_usa_exist_home_sales => ("美国成屋销售总数年化报告", "15"),
    macro_usa_house_price_index => ("美国FHFA房价指数月率报告", "51"),
    macro_usa_spcs20 => ("美国S&P/CS20座大城市房价指数年率", "52"),
    macro_usa_pending_home_sales => ("美国成屋签约销售指数月率报告", "34"),
    macro_usa_cb_consumer_confidence => ("美国谘商会消费者信心指数", "5"),
    macro_usa_nfib_small_business => ("美国NFIB小型企业信心指数报告", "63"),
    macro_usa_michigan_consumer_sentiment => ("美国密歇根大学消费者信心指数初值报告", "50"),
    macro_usa_eia_crude_rate => ("美国EIA原油库存", "10"),
    macro_usa_initial_jobless => ("美国初请失业金人数", "44"),
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
        let rows =
            parse_macro_usa_cftc_nc_holding(&fixture("macro_usa_cftc_nc_holding.json")).unwrap();
        assert_eq!(rows.len(), 12);
        assert_eq!(rows[0].symbol, "欧元");
        assert_eq!(rows[0].metric, "净持仓");
        assert_eq!(rows[0].value, Some(100.0));
        assert_eq!(rows[2].metric, "空头持仓");
        assert_eq!(rows[2].value, Some(300.0));
    }

    #[test]
    fn parses_macro_usa_cftc_c_holding() {
        let rows =
            parse_macro_usa_cftc_c_holding(&fixture("macro_usa_cftc_c_holding.json")).unwrap();
        assert_eq!(rows.len(), 12);
        assert_eq!(rows[0].symbol, "欧元");
        assert_eq!(rows[0].value, Some(100.0));
        assert_eq!(rows[11].symbol, "英镑");
        assert_eq!(rows[11].value, Some(75.0));
    }

    #[test]
    fn parses_macro_usa_cftc_merchant_currency_holding() {
        let rows = parse_macro_usa_cftc_merchant_currency_holding(&fixture(
            "macro_usa_cftc_merchant_currency_holding.json",
        ))
        .unwrap();
        assert_eq!(rows.len(), 12);
        assert_eq!(rows[0].symbol, "欧元");
        assert_eq!(rows[6].date, "0");
        assert_eq!(rows[6].value, Some(50.0));
    }

    #[test]
    fn parses_macro_usa_cftc_merchant_goods_holding() {
        let rows = parse_macro_usa_cftc_merchant_goods_holding(&fixture(
            "macro_usa_cftc_merchant_goods_holding.json",
        ))
        .unwrap();
        assert_eq!(rows.len(), 12);
        assert_eq!(rows[0].symbol, "欧元");
        assert_eq!(rows[0].metric, "净持仓");
        assert_eq!(rows[11].value, Some(75.0));
    }

    #[test]
    fn parses_macro_usa_cme_merchant_goods_holding() {
        let rows = parse_macro_usa_cme_merchant_goods_holding(&fixture(
            "macro_usa_cme_merchant_goods_holding.json",
        ))
        .unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].date, "2024-01-01");
        assert_eq!(rows[0].variety, "黄金-GC");
        assert_eq!(rows[0].volume, Some(1000.0));
        assert_eq!(rows[2].variety, "黄金-GC");
        assert_eq!(rows[2].volume, Some(1100.0));
    }

    #[test]
    fn parses_all_macro_usa_list_v2() {
        let cases: &[(&str, &str)] = &[
            ("macro_usa_gdp_monthly", "美国国内生产总值(GDP)"),
            ("macro_usa_cpi_monthly", "美国CPI月率"),
            ("macro_usa_core_cpi_monthly", "美国核心CPI月率"),
            ("macro_usa_personal_spending", "美国个人支出月率"),
            ("macro_usa_retail_sales", "美国零售销售月率"),
            ("macro_usa_import_price", "美国进口物价指数"),
            ("macro_usa_export_price", "美国出口价格指数"),
            ("macro_usa_lmci", "美联储劳动力市场状况指数"),
            ("macro_usa_unemployment_rate", "美国失业率"),
            ("macro_usa_job_cuts", "美国挑战者企业裁员人数"),
            ("macro_usa_non_farm", "美国非农就业人数"),
            ("macro_usa_adp_employment", "美国ADP就业人数"),
            ("macro_usa_core_pce_price", "美国核心PCE物价指数年率"),
            ("macro_usa_real_consumer_spending", "美国实际个人消费支出季率初值"),
            ("macro_usa_trade_balance", "美国贸易帐报告"),
            ("macro_usa_current_account", "美国经常账报告"),
            ("macro_usa_ppi", "美国生产者物价指数"),
            ("macro_usa_core_ppi", "美国核心生产者物价指数"),
            ("macro_usa_api_crude_stock", "美国API原油库存"),
            ("macro_usa_pmi", "美国Markit制造业PMI报告"),
            ("macro_usa_ism_pmi", "美国ISM制造业PMI报告"),
            ("macro_usa_industrial_production", "美国工业产出月率报告"),
            ("macro_usa_durable_goods_orders", "美国耐用品订单月率报告"),
            ("macro_usa_factory_orders", "美国工厂订单月率报告"),
            ("macro_usa_services_pmi", "美国Markit服务业PMI初值报告"),
            ("macro_usa_business_inventories", "美国商业库存月率报告"),
            ("macro_usa_ism_non_pmi", "美国ISM非制造业PMI报告"),
            ("macro_usa_nahb_house_market_index", "美国NAHB房产市场指数报告"),
            ("macro_usa_house_starts", "美国新屋开工总数年化报告"),
            ("macro_usa_new_home_sales", "美国新屋销售总数年化报告"),
            ("macro_usa_building_permits", "美国营建许可总数报告"),
            ("macro_usa_exist_home_sales", "美国成屋销售总数年化报告"),
            ("macro_usa_house_price_index", "美国FHFA房价指数月率报告"),
            ("macro_usa_spcs20", "美国S&P/CS20座大城市房价指数年率"),
            ("macro_usa_pending_home_sales", "美国成屋签约销售指数月率报告"),
            ("macro_usa_cb_consumer_confidence", "美国谘商会消费者信心指数"),
            ("macro_usa_nfib_small_business", "美国NFIB小型企业信心指数报告"),
            ("macro_usa_michigan_consumer_sentiment", "美国密歇根大学消费者信心指数初值报告"),
            ("macro_usa_eia_crude_rate", "美国EIA原油库存"),
            ("macro_usa_initial_jobless", "美国初请失业金人数"),
        ];
        for (name, symbol) in cases {
            let v = fixture(&format!("{name}.json"));
            let values = v
                .get("data")
                .and_then(|d| d.get("values"))
                .and_then(|x| x.as_array())
                .expect("data.values");
            let rows = parse_jin10_list_values(values, symbol);
            assert!(!rows.is_empty(), "empty parse for {name}");
            assert_eq!(rows.last().unwrap().commodity, *symbol, "symbol mismatch {name}");
            for r in &rows {
                if let Some(d) = &r.date {
                    let parts: Vec<&str> = d.split('-').collect();
                    assert_eq!(parts.len(), 3, "bad date {d} in {name}");
                    assert!(
                        parts[0].len() == 4 && parts[1].len() == 2 && parts[2].len() == 2,
                        "bad date {d} in {name}"
                    );
                }
            }
        }
    }
}
