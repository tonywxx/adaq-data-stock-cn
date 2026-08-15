//! Emerging-market / foreign-country macro indicators.
//!
//! The originally-requested akshare `macro_india.py` / `macro_singapore.py` /
//! `macro_korea.py` / `macro_brazil.py` / `macro_mexico.py` / `macro_turkey.py`
//! modules **do not exist** in the checked-out akshare tree (commit
//! `4771e6e`, 2026-08-13) — those symbols were never published there. This
//! module instead ports the available foreign-country Eastmoney datacenter
//! modules, which follow the exact same `RPT_ECONOMICVALUE_*` + `INDICATOR_ID`
//! pattern already used by `macro2.rs` (`macro_usa_cpi_yoy` /
//! `macro_usa_phs`):
//! - `macro_australia.py`  -> `RPT_ECONOMICVALUE_AUSTRALIA`
//! - `macro_canada.py`     -> `RPT_ECONOMICVALUE_CA`
//! - `macro_uk.py`         -> `RPT_ECONOMICVALUE_BRITAIN`
//! - `macro_swiss.py`      -> `RPT_ECONOMICVALUE_CH`
//! - `macro_japan.py`      -> `RPT_ECONOMICVALUE_JPAN`
//! - `macro_germany.py`    -> `RPT_ECONOMICVALUE_GER`
//! - `macro_china_hk.py`   -> `RPT_ECONOMICVALUE_HK`
//!
//! Every function is PURE HTTP (a single `GET` to the Eastmoney datacenter
//! JSON API, no JS/token/signature/`execjs`/`get_token`/cookie). All endpoints
//! share one response envelope (`result.data`, rows of
//! `REPORT_DATE(_CH)` / `PUBLISH_DATE` / `VALUE` / `PRE_VALUE`), so a single
//! shared [`EmgRow`] struct and [`parse_emg`] parser serve every indicator.
//! The akshare source modules contain only these pure-HTTP functions — there
//! are no deferred (JS/token/HTML/Excel) functions in them.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_EASTMONEY: &str = "eastmoney";
const BASE: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";

/// Extract `result.data` (the row array) from a datacenter-web response.
fn emg_data_array(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data".into(),
        })
}

fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

#[allow(dead_code)]
fn inum(item: &Value, k: &str) -> Option<i64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    })
}

/// A single observation of an Eastmoney (foreign-country) macro indicator.
///
/// Mirrors the four columns every `RPT_ECONOMICVALUE_*` row exposes:
/// `REPORT_DATE_CH`/`REPORT_DATE` (period), `VALUE` (current / 现值),
/// `PRE_VALUE` (previous / 前值) and `PUBLISH_DATE` (发布日期).
#[derive(Debug, Clone, serde::Serialize)]
pub struct EmgRow {
    /// Report period (akshare `时间` / Eastmoney `REPORT_DATE_CH` or `REPORT_DATE`).
    pub date: String,
    /// Current value (akshare `现值` / Eastmoney `VALUE`).
    pub value: Option<f64>,
    /// Previous value (akshare `前值` / Eastmoney `PRE_VALUE`).
    pub pre_value: Option<f64>,
    /// Publish date (akshare `发布日期` / Eastmoney `PUBLISH_DATE`).
    pub publish_date: Option<String>,
    pub source: &'static str,
}

/// Shared parser for every `RPT_ECONOMICVALUE_*` indicator response.
///
/// Period is taken from `REPORT_DATE_CH` when present (UK/Swiss/Japan/Germany/
/// HK tables), falling back to `REPORT_DATE` (Australia/Canada/USA-style tables).
#[allow(dead_code)]
pub(crate) fn parse_emg(resp: &Value) -> Result<Vec<EmgRow>> {
    let data = emg_data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(date) = fstr(item, "REPORT_DATE_CH").or_else(|| fstr(item, "REPORT_DATE")) else {
            continue;
        };
        out.push(EmgRow {
            date,
            value: fnum(item, "VALUE"),
            pre_value: fnum(item, "PRE_VALUE"),
            publish_date: fstr(item, "PUBLISH_DATE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

/// Fetch a single `RPT_ECONOMICVALUE_*` indicator (filtered by `INDICATOR_ID`).
async fn emg_fetch(
    client: &Client,
    endpoint: &'static str,
    report_name: &'static str,
    indicator_id: &'static str,
    page_size: &'static str,
) -> Result<Vec<EmgRow>> {
    let filter = format!(r#"(INDICATOR_ID="{indicator_id}")"#);
    let params = [
        ("reportName", report_name),
        ("columns", "ALL"),
        ("filter", filter.as_str()),
        ("pageNumber", "1"),
        ("pageSize", page_size),
        ("sortColumns", "REPORT_DATE"),
        ("sortTypes", "-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("p", "1"),
        ("pageNo", "1"),
        ("pageNum", "1"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, endpoint, BASE, &params)
        .await?;
    parse_emg(&v)
}

/// Australia retail sales MoM (`macro_australia_retail_rate_monthly`, Eastmoney `RPT_ECONOMICVALUE_AUSTRALIA`, indicator `EMG00152903`).
pub async fn macro_australia_retail_rate_monthly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_australia_retail_rate_monthly", "RPT_ECONOMICVALUE_AUSTRALIA", "EMG00152903", "2000").await
}

/// Parse `macro_australia_retail_rate_monthly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_australia_retail_rate_monthly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Australia trade balance (`macro_australia_trade`, Eastmoney `RPT_ECONOMICVALUE_AUSTRALIA`, indicator `EMG00152793`).
pub async fn macro_australia_trade(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_australia_trade", "RPT_ECONOMICVALUE_AUSTRALIA", "EMG00152793", "2000").await
}

/// Parse `macro_australia_trade` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_australia_trade(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Australia unemployment rate (`macro_australia_unemployment_rate`, Eastmoney `RPT_ECONOMICVALUE_AUSTRALIA`, indicator `EMG00101141`).
pub async fn macro_australia_unemployment_rate(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_australia_unemployment_rate", "RPT_ECONOMICVALUE_AUSTRALIA", "EMG00101141", "2000").await
}

/// Parse `macro_australia_unemployment_rate` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_australia_unemployment_rate(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Australia PPI quarterly (`macro_australia_ppi_quarterly`, Eastmoney `RPT_ECONOMICVALUE_AUSTRALIA`, indicator `EMG00152722`).
pub async fn macro_australia_ppi_quarterly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_australia_ppi_quarterly", "RPT_ECONOMICVALUE_AUSTRALIA", "EMG00152722", "2000").await
}

/// Parse `macro_australia_ppi_quarterly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_australia_ppi_quarterly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Australia CPI quarterly (`macro_australia_cpi_quarterly`, Eastmoney `RPT_ECONOMICVALUE_AUSTRALIA`, indicator `EMG00101104`).
pub async fn macro_australia_cpi_quarterly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_australia_cpi_quarterly", "RPT_ECONOMICVALUE_AUSTRALIA", "EMG00101104", "2000").await
}

/// Parse `macro_australia_cpi_quarterly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_australia_cpi_quarterly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Australia CPI yearly (`macro_australia_cpi_yearly`, Eastmoney `RPT_ECONOMICVALUE_AUSTRALIA`, indicator `EMG00101093`).
pub async fn macro_australia_cpi_yearly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_australia_cpi_yearly", "RPT_ECONOMICVALUE_AUSTRALIA", "EMG00101093", "2000").await
}

/// Parse `macro_australia_cpi_yearly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_australia_cpi_yearly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Australia cash rate target (`macro_australia_bank_rate`, Eastmoney `RPT_ECONOMICVALUE_AUSTRALIA`, indicator `EMG00342255`).
pub async fn macro_australia_bank_rate(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_australia_bank_rate", "RPT_ECONOMICVALUE_AUSTRALIA", "EMG00342255", "2000").await
}

/// Parse `macro_australia_bank_rate` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_australia_bank_rate(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Canada housing starts (`macro_canada_new_house_rate`, Eastmoney `RPT_ECONOMICVALUE_CA`, indicator `EMG00342247`).
pub async fn macro_canada_new_house_rate(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_canada_new_house_rate", "RPT_ECONOMICVALUE_CA", "EMG00342247", "2000").await
}

/// Parse `macro_canada_new_house_rate` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_canada_new_house_rate(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Canada unemployment rate (`macro_canada_unemployment_rate`, Eastmoney `RPT_ECONOMICVALUE_CA`, indicator `EMG00157746`).
pub async fn macro_canada_unemployment_rate(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_canada_unemployment_rate", "RPT_ECONOMICVALUE_CA", "EMG00157746", "2000").await
}

/// Parse `macro_canada_unemployment_rate` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_canada_unemployment_rate(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Canada trade balance (`macro_canada_trade`, Eastmoney `RPT_ECONOMICVALUE_CA`, indicator `EMG00102022`).
pub async fn macro_canada_trade(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_canada_trade", "RPT_ECONOMICVALUE_CA", "EMG00102022", "2000").await
}

/// Parse `macro_canada_trade` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_canada_trade(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Canada retail sales MoM (`macro_canada_retail_rate_monthly`, Eastmoney `RPT_ECONOMICVALUE_CA`, indicator `EMG01337094`).
pub async fn macro_canada_retail_rate_monthly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_canada_retail_rate_monthly", "RPT_ECONOMICVALUE_CA", "EMG01337094", "2000").await
}

/// Parse `macro_canada_retail_rate_monthly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_canada_retail_rate_monthly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Canada overnight rate target (`macro_canada_bank_rate`, Eastmoney `RPT_ECONOMICVALUE_CA`, indicator `EMG00342248`).
pub async fn macro_canada_bank_rate(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_canada_bank_rate", "RPT_ECONOMICVALUE_CA", "EMG00342248", "2000").await
}

/// Parse `macro_canada_bank_rate` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_canada_bank_rate(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Canada core CPI yearly (`macro_canada_core_cpi_yearly`, Eastmoney `RPT_ECONOMICVALUE_CA`, indicator `EMG00102030`).
pub async fn macro_canada_core_cpi_yearly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_canada_core_cpi_yearly", "RPT_ECONOMICVALUE_CA", "EMG00102030", "2000").await
}

/// Parse `macro_canada_core_cpi_yearly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_canada_core_cpi_yearly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Canada core CPI monthly (`macro_canada_core_cpi_monthly`, Eastmoney `RPT_ECONOMICVALUE_CA`, indicator `EMG00102044`).
pub async fn macro_canada_core_cpi_monthly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_canada_core_cpi_monthly", "RPT_ECONOMICVALUE_CA", "EMG00102044", "2000").await
}

/// Parse `macro_canada_core_cpi_monthly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_canada_core_cpi_monthly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Canada CPI yearly (`macro_canada_cpi_yearly`, Eastmoney `RPT_ECONOMICVALUE_CA`, indicator `EMG00102029`).
pub async fn macro_canada_cpi_yearly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_canada_cpi_yearly", "RPT_ECONOMICVALUE_CA", "EMG00102029", "2000").await
}

/// Parse `macro_canada_cpi_yearly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_canada_cpi_yearly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Canada CPI monthly (`macro_canada_cpi_monthly`, Eastmoney `RPT_ECONOMICVALUE_CA`, indicator `EMG00158719`).
pub async fn macro_canada_cpi_monthly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_canada_cpi_monthly", "RPT_ECONOMICVALUE_CA", "EMG00158719", "2000").await
}

/// Parse `macro_canada_cpi_monthly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_canada_cpi_monthly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Canada GDP monthly (`macro_canada_gdp_monthly`, Eastmoney `RPT_ECONOMICVALUE_CA`, indicator `EMG00159259`).
pub async fn macro_canada_gdp_monthly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_canada_gdp_monthly", "RPT_ECONOMICVALUE_CA", "EMG00159259", "2000").await
}

/// Parse `macro_canada_gdp_monthly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_canada_gdp_monthly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// UK Halifax house price MoM (`macro_uk_halifax_monthly`, Eastmoney `RPT_ECONOMICVALUE_BRITAIN`, indicator `EMG00342256`).
pub async fn macro_uk_halifax_monthly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_uk_halifax_monthly", "RPT_ECONOMICVALUE_BRITAIN", "EMG00342256", "5000").await
}

/// Parse `macro_uk_halifax_monthly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_uk_halifax_monthly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// UK Halifax house price YoY (`macro_uk_halifax_yearly`, Eastmoney `RPT_ECONOMICVALUE_BRITAIN`, indicator `EMG00010370`).
pub async fn macro_uk_halifax_yearly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_uk_halifax_yearly", "RPT_ECONOMICVALUE_BRITAIN", "EMG00010370", "5000").await
}

/// Parse `macro_uk_halifax_yearly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_uk_halifax_yearly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// UK trade balance (`macro_uk_trade`, Eastmoney `RPT_ECONOMICVALUE_BRITAIN`, indicator `EMG00158309`).
pub async fn macro_uk_trade(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_uk_trade", "RPT_ECONOMICVALUE_BRITAIN", "EMG00158309", "5000").await
}

/// Parse `macro_uk_trade` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_uk_trade(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// UK bank rate (`macro_uk_bank_rate`, Eastmoney `RPT_ECONOMICVALUE_BRITAIN`, indicator `EMG00342253`).
pub async fn macro_uk_bank_rate(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_uk_bank_rate", "RPT_ECONOMICVALUE_BRITAIN", "EMG00342253", "5000").await
}

/// Parse `macro_uk_bank_rate` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_uk_bank_rate(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// UK core CPI yearly (`macro_uk_core_cpi_yearly`, Eastmoney `RPT_ECONOMICVALUE_BRITAIN`, indicator `EMG00010279`).
pub async fn macro_uk_core_cpi_yearly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_uk_core_cpi_yearly", "RPT_ECONOMICVALUE_BRITAIN", "EMG00010279", "5000").await
}

/// Parse `macro_uk_core_cpi_yearly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_uk_core_cpi_yearly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// UK core CPI monthly (`macro_uk_core_cpi_monthly`, Eastmoney `RPT_ECONOMICVALUE_BRITAIN`, indicator `EMG00010291`).
pub async fn macro_uk_core_cpi_monthly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_uk_core_cpi_monthly", "RPT_ECONOMICVALUE_BRITAIN", "EMG00010291", "5000").await
}

/// Parse `macro_uk_core_cpi_monthly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_uk_core_cpi_monthly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// UK CPI yearly (`macro_uk_cpi_yearly`, Eastmoney `RPT_ECONOMICVALUE_BRITAIN`, indicator `EMG00010267`).
pub async fn macro_uk_cpi_yearly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_uk_cpi_yearly", "RPT_ECONOMICVALUE_BRITAIN", "EMG00010267", "5000").await
}

/// Parse `macro_uk_cpi_yearly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_uk_cpi_yearly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// UK CPI monthly (`macro_uk_cpi_monthly`, Eastmoney `RPT_ECONOMICVALUE_BRITAIN`, indicator `EMG00010291`).
pub async fn macro_uk_cpi_monthly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_uk_cpi_monthly", "RPT_ECONOMICVALUE_BRITAIN", "EMG00010291", "5000").await
}

/// Parse `macro_uk_cpi_monthly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_uk_cpi_monthly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// UK retail sales MoM (`macro_uk_retail_monthly`, Eastmoney `RPT_ECONOMICVALUE_BRITAIN`, indicator `EMG00158298`).
pub async fn macro_uk_retail_monthly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_uk_retail_monthly", "RPT_ECONOMICVALUE_BRITAIN", "EMG00158298", "5000").await
}

/// Parse `macro_uk_retail_monthly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_uk_retail_monthly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// UK retail sales YoY (`macro_uk_retail_yearly`, Eastmoney `RPT_ECONOMICVALUE_BRITAIN`, indicator `EMG00158297`).
pub async fn macro_uk_retail_yearly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_uk_retail_yearly", "RPT_ECONOMICVALUE_BRITAIN", "EMG00158297", "5000").await
}

/// Parse `macro_uk_retail_yearly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_uk_retail_yearly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// UK Rightmove house price YoY (`macro_uk_rightmove_yearly`, Eastmoney `RPT_ECONOMICVALUE_BRITAIN`, indicator `EMG00341608`).
pub async fn macro_uk_rightmove_yearly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_uk_rightmove_yearly", "RPT_ECONOMICVALUE_BRITAIN", "EMG00341608", "5000").await
}

/// Parse `macro_uk_rightmove_yearly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_uk_rightmove_yearly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// UK Rightmove house price MoM (`macro_uk_rightmove_monthly`, Eastmoney `RPT_ECONOMICVALUE_BRITAIN`, indicator `EMG00341607`).
pub async fn macro_uk_rightmove_monthly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_uk_rightmove_monthly", "RPT_ECONOMICVALUE_BRITAIN", "EMG00341607", "5000").await
}

/// Parse `macro_uk_rightmove_monthly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_uk_rightmove_monthly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// UK GDP quarterly (`macro_uk_gdp_quarterly`, Eastmoney `RPT_ECONOMICVALUE_BRITAIN`, indicator `EMG00158277`).
pub async fn macro_uk_gdp_quarterly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_uk_gdp_quarterly", "RPT_ECONOMICVALUE_BRITAIN", "EMG00158277", "5000").await
}

/// Parse `macro_uk_gdp_quarterly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_uk_gdp_quarterly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// UK GDP yearly (`macro_uk_gdp_yearly`, Eastmoney `RPT_ECONOMICVALUE_BRITAIN`, indicator `EMG00158276`).
pub async fn macro_uk_gdp_yearly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_uk_gdp_yearly", "RPT_ECONOMICVALUE_BRITAIN", "EMG00158276", "5000").await
}

/// Parse `macro_uk_gdp_yearly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_uk_gdp_yearly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// UK unemployment rate (`macro_uk_unemployment_rate`, Eastmoney `RPT_ECONOMICVALUE_BRITAIN`, indicator `EMG00010348`).
pub async fn macro_uk_unemployment_rate(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_uk_unemployment_rate", "RPT_ECONOMICVALUE_BRITAIN", "EMG00010348", "5000").await
}

/// Parse `macro_uk_unemployment_rate` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_uk_unemployment_rate(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Switzerland SVME PMI (`macro_swiss_svme`, Eastmoney `RPT_ECONOMICVALUE_CH`, indicator `EMG00341602`).
pub async fn macro_swiss_svme(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_swiss_svme", "RPT_ECONOMICVALUE_CH", "EMG00341602", "5000").await
}

/// Parse `macro_swiss_svme` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_swiss_svme(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Switzerland trade balance (`macro_swiss_trade`, Eastmoney `RPT_ECONOMICVALUE_CH`, indicator `EMG00341603`).
pub async fn macro_swiss_trade(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_swiss_trade", "RPT_ECONOMICVALUE_CH", "EMG00341603", "5000").await
}

/// Parse `macro_swiss_trade` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_swiss_trade(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Switzerland CPI yearly (`macro_swiss_cpi_yearly`, Eastmoney `RPT_ECONOMICVALUE_CH`, indicator `EMG00341604`).
pub async fn macro_swiss_cpi_yearly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_swiss_cpi_yearly", "RPT_ECONOMICVALUE_CH", "EMG00341604", "5000").await
}

/// Parse `macro_swiss_cpi_yearly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_swiss_cpi_yearly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Switzerland GDP quarterly (`macro_swiss_gdp_quarterly`, Eastmoney `RPT_ECONOMICVALUE_CH`, indicator `EMG00341600`).
pub async fn macro_swiss_gdp_quarterly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_swiss_gdp_quarterly", "RPT_ECONOMICVALUE_CH", "EMG00341600", "5000").await
}

/// Parse `macro_swiss_gdp_quarterly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_swiss_gdp_quarterly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Switzerland GDP yearly (`macro_swiss_gbd_yearly`, Eastmoney `RPT_ECONOMICVALUE_CH`, indicator `EMG00341601`).
pub async fn macro_swiss_gbd_yearly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_swiss_gbd_yearly", "RPT_ECONOMICVALUE_CH", "EMG00341601", "5000").await
}

/// Parse `macro_swiss_gbd_yearly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_swiss_gbd_yearly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Switzerland SNB policy rate (`macro_swiss_gbd_bank_rate`, Eastmoney `RPT_ECONOMICVALUE_CH`, indicator `EMG00341606`).
pub async fn macro_swiss_gbd_bank_rate(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_swiss_gbd_bank_rate", "RPT_ECONOMICVALUE_CH", "EMG00341606", "5000").await
}

/// Parse `macro_swiss_gbd_bank_rate` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_swiss_gbd_bank_rate(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Japan policy rate (`macro_japan_bank_rate`, Eastmoney `RPT_ECONOMICVALUE_JPAN`, indicator `EMG00342252`).
pub async fn macro_japan_bank_rate(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_japan_bank_rate", "RPT_ECONOMICVALUE_JPAN", "EMG00342252", "5000").await
}

/// Parse `macro_japan_bank_rate` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_japan_bank_rate(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Japan CPI yearly (`macro_japan_cpi_yearly`, Eastmoney `RPT_ECONOMICVALUE_JPAN`, indicator `EMG00005004`).
pub async fn macro_japan_cpi_yearly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_japan_cpi_yearly", "RPT_ECONOMICVALUE_JPAN", "EMG00005004", "5000").await
}

/// Parse `macro_japan_cpi_yearly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_japan_cpi_yearly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Japan core CPI yearly (`macro_japan_core_cpi_yearly`, Eastmoney `RPT_ECONOMICVALUE_JPAN`, indicator `EMG00158099`).
pub async fn macro_japan_core_cpi_yearly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_japan_core_cpi_yearly", "RPT_ECONOMICVALUE_JPAN", "EMG00158099", "5000").await
}

/// Parse `macro_japan_core_cpi_yearly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_japan_core_cpi_yearly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Japan unemployment rate (`macro_japan_unemployment_rate`, Eastmoney `RPT_ECONOMICVALUE_JPAN`, indicator `EMG00005047`).
pub async fn macro_japan_unemployment_rate(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_japan_unemployment_rate", "RPT_ECONOMICVALUE_JPAN", "EMG00005047", "5000").await
}

/// Parse `macro_japan_unemployment_rate` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_japan_unemployment_rate(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Japan leading index CI (`macro_japan_head_indicator`, Eastmoney `RPT_ECONOMICVALUE_JPAN`, indicator `EMG00005117`).
pub async fn macro_japan_head_indicator(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_japan_head_indicator", "RPT_ECONOMICVALUE_JPAN", "EMG00005117", "5000").await
}

/// Parse `macro_japan_head_indicator` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_japan_head_indicator(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Germany IFO business climate (`macro_germany_ifo`, Eastmoney `RPT_ECONOMICVALUE_GER`, indicator `EMG00179154`).
pub async fn macro_germany_ifo(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_germany_ifo", "RPT_ECONOMICVALUE_GER", "EMG00179154", "5000").await
}

/// Parse `macro_germany_ifo` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_germany_ifo(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Germany CPI monthly (`macro_germany_cpi_monthly`, Eastmoney `RPT_ECONOMICVALUE_GER`, indicator `EMG00009758`).
pub async fn macro_germany_cpi_monthly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_germany_cpi_monthly", "RPT_ECONOMICVALUE_GER", "EMG00009758", "5000").await
}

/// Parse `macro_germany_cpi_monthly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_germany_cpi_monthly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Germany CPI yearly (`macro_germany_cpi_yearly`, Eastmoney `RPT_ECONOMICVALUE_GER`, indicator `EMG00009756`).
pub async fn macro_germany_cpi_yearly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_germany_cpi_yearly", "RPT_ECONOMICVALUE_GER", "EMG00009756", "5000").await
}

/// Parse `macro_germany_cpi_yearly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_germany_cpi_yearly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Germany trade balance (seasonally adjusted) (`macro_germany_trade_adjusted`, Eastmoney `RPT_ECONOMICVALUE_GER`, indicator `EMG00009753`).
pub async fn macro_germany_trade_adjusted(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_germany_trade_adjusted", "RPT_ECONOMICVALUE_GER", "EMG00009753", "5000").await
}

/// Parse `macro_germany_trade_adjusted` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_germany_trade_adjusted(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Germany GDP (`macro_germany_gdp`, Eastmoney `RPT_ECONOMICVALUE_GER`, indicator `EMG00009720`).
pub async fn macro_germany_gdp(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_germany_gdp", "RPT_ECONOMICVALUE_GER", "EMG00009720", "5000").await
}

/// Parse `macro_germany_gdp` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_germany_gdp(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Germany real retail sales MoM (`macro_germany_retail_sale_monthly`, Eastmoney `RPT_ECONOMICVALUE_GER`, indicator `EMG01333186`).
pub async fn macro_germany_retail_sale_monthly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_germany_retail_sale_monthly", "RPT_ECONOMICVALUE_GER", "EMG01333186", "5000").await
}

/// Parse `macro_germany_retail_sale_monthly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_germany_retail_sale_monthly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Germany real retail sales YoY (`macro_germany_retail_sale_yearly`, Eastmoney `RPT_ECONOMICVALUE_GER`, indicator `EMG01333192`).
pub async fn macro_germany_retail_sale_yearly(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_germany_retail_sale_yearly", "RPT_ECONOMICVALUE_GER", "EMG01333192", "5000").await
}

/// Parse `macro_germany_retail_sale_yearly` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_germany_retail_sale_yearly(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Germany ZEW economic sentiment (`macro_germany_zew`, Eastmoney `RPT_ECONOMICVALUE_GER`, indicator `EMG00172577`).
pub async fn macro_germany_zew(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_germany_zew", "RPT_ECONOMICVALUE_GER", "EMG00172577", "5000").await
}

/// Parse `macro_germany_zew` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_germany_zew(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Hong Kong CPI (`macro_china_hk_cpi`, Eastmoney `RPT_ECONOMICVALUE_HK`, indicator `EMG01336996`).
pub async fn macro_china_hk_cpi(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_china_hk_cpi", "RPT_ECONOMICVALUE_HK", "EMG01336996", "5000").await
}

/// Parse `macro_china_hk_cpi` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_china_hk_cpi(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Hong Kong CPI YoY (`macro_china_hk_cpi_ratio`, Eastmoney `RPT_ECONOMICVALUE_HK`, indicator `EMG00059282`).
pub async fn macro_china_hk_cpi_ratio(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_china_hk_cpi_ratio", "RPT_ECONOMICVALUE_HK", "EMG00059282", "5000").await
}

/// Parse `macro_china_hk_cpi_ratio` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_china_hk_cpi_ratio(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Hong Kong unemployment rate (`macro_china_hk_rate_of_unemployment`, Eastmoney `RPT_ECONOMICVALUE_HK`, indicator `EMG00059647`).
pub async fn macro_china_hk_rate_of_unemployment(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_china_hk_rate_of_unemployment", "RPT_ECONOMICVALUE_HK", "EMG00059647", "5000").await
}

/// Parse `macro_china_hk_rate_of_unemployment` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_china_hk_rate_of_unemployment(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Hong Kong GDP (`macro_china_hk_gbp`, Eastmoney `RPT_ECONOMICVALUE_HK`, indicator `EMG01337008`).
pub async fn macro_china_hk_gbp(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_china_hk_gbp", "RPT_ECONOMICVALUE_HK", "EMG01337008", "5000").await
}

/// Parse `macro_china_hk_gbp` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_china_hk_gbp(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Hong Kong GDP YoY (`macro_china_hk_gbp_ratio`, Eastmoney `RPT_ECONOMICVALUE_HK`, indicator `EMG01337009`).
pub async fn macro_china_hk_gbp_ratio(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_china_hk_gbp_ratio", "RPT_ECONOMICVALUE_HK", "EMG01337009", "5000").await
}

/// Parse `macro_china_hk_gbp_ratio` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_china_hk_gbp_ratio(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Hong Kong building transactions volume (`macro_china_hk_building_volume`, Eastmoney `RPT_ECONOMICVALUE_HK`, indicator `EMG00158055`).
pub async fn macro_china_hk_building_volume(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_china_hk_building_volume", "RPT_ECONOMICVALUE_HK", "EMG00158055", "5000").await
}

/// Parse `macro_china_hk_building_volume` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_china_hk_building_volume(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Hong Kong building transactions amount (`macro_china_hk_building_amount`, Eastmoney `RPT_ECONOMICVALUE_HK`, indicator `EMG00158066`).
pub async fn macro_china_hk_building_amount(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_china_hk_building_amount", "RPT_ECONOMICVALUE_HK", "EMG00158066", "5000").await
}

/// Parse `macro_china_hk_building_amount` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_china_hk_building_amount(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Hong Kong merchandise trade balance YoY (`macro_china_hk_trade_diff_ratio`, Eastmoney `RPT_ECONOMICVALUE_HK`, indicator `EMG00157898`).
pub async fn macro_china_hk_trade_diff_ratio(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_china_hk_trade_diff_ratio", "RPT_ECONOMICVALUE_HK", "EMG00157898", "5000").await
}

/// Parse `macro_china_hk_trade_diff_ratio` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_china_hk_trade_diff_ratio(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
}

/// Hong Kong manufacturing PPI YoY (`macro_china_hk_ppi`, Eastmoney `RPT_ECONOMICVALUE_HK`, indicator `EMG00157818`).
pub async fn macro_china_hk_ppi(client: &Client) -> Result<Vec<EmgRow>> {
    emg_fetch(client, "macro_china_hk_ppi", "RPT_ECONOMICVALUE_HK", "EMG00157818", "5000").await
}

/// Parse `macro_china_hk_ppi` response (delegates to the shared [`parse_emg`]).
#[allow(dead_code)]
pub(crate) fn parse_macro_china_hk_ppi(resp: &Value) -> Result<Vec<EmgRow>> {
    parse_emg(resp)
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
    fn parses_macro_australia_retail_rate_monthly() {
        let rows = parse_macro_australia_retail_rate_monthly(&fixture("macro_australia_retail_rate_monthly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_australia_trade() {
        let rows = parse_macro_australia_trade(&fixture("macro_australia_trade.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_australia_unemployment_rate() {
        let rows = parse_macro_australia_unemployment_rate(&fixture("macro_australia_unemployment_rate.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_australia_ppi_quarterly() {
        let rows = parse_macro_australia_ppi_quarterly(&fixture("macro_australia_ppi_quarterly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_australia_cpi_quarterly() {
        let rows = parse_macro_australia_cpi_quarterly(&fixture("macro_australia_cpi_quarterly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_australia_cpi_yearly() {
        let rows = parse_macro_australia_cpi_yearly(&fixture("macro_australia_cpi_yearly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_australia_bank_rate() {
        let rows = parse_macro_australia_bank_rate(&fixture("macro_australia_bank_rate.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_canada_new_house_rate() {
        let rows = parse_macro_canada_new_house_rate(&fixture("macro_canada_new_house_rate.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_canada_unemployment_rate() {
        let rows = parse_macro_canada_unemployment_rate(&fixture("macro_canada_unemployment_rate.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_canada_trade() {
        let rows = parse_macro_canada_trade(&fixture("macro_canada_trade.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_canada_retail_rate_monthly() {
        let rows = parse_macro_canada_retail_rate_monthly(&fixture("macro_canada_retail_rate_monthly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_canada_bank_rate() {
        let rows = parse_macro_canada_bank_rate(&fixture("macro_canada_bank_rate.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_canada_core_cpi_yearly() {
        let rows = parse_macro_canada_core_cpi_yearly(&fixture("macro_canada_core_cpi_yearly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_canada_core_cpi_monthly() {
        let rows = parse_macro_canada_core_cpi_monthly(&fixture("macro_canada_core_cpi_monthly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_canada_cpi_yearly() {
        let rows = parse_macro_canada_cpi_yearly(&fixture("macro_canada_cpi_yearly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_canada_cpi_monthly() {
        let rows = parse_macro_canada_cpi_monthly(&fixture("macro_canada_cpi_monthly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_canada_gdp_monthly() {
        let rows = parse_macro_canada_gdp_monthly(&fixture("macro_canada_gdp_monthly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_uk_halifax_monthly() {
        let rows = parse_macro_uk_halifax_monthly(&fixture("macro_uk_halifax_monthly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_uk_halifax_yearly() {
        let rows = parse_macro_uk_halifax_yearly(&fixture("macro_uk_halifax_yearly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_uk_trade() {
        let rows = parse_macro_uk_trade(&fixture("macro_uk_trade.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_uk_bank_rate() {
        let rows = parse_macro_uk_bank_rate(&fixture("macro_uk_bank_rate.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_uk_core_cpi_yearly() {
        let rows = parse_macro_uk_core_cpi_yearly(&fixture("macro_uk_core_cpi_yearly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_uk_core_cpi_monthly() {
        let rows = parse_macro_uk_core_cpi_monthly(&fixture("macro_uk_core_cpi_monthly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_uk_cpi_yearly() {
        let rows = parse_macro_uk_cpi_yearly(&fixture("macro_uk_cpi_yearly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_uk_cpi_monthly() {
        let rows = parse_macro_uk_cpi_monthly(&fixture("macro_uk_cpi_monthly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_uk_retail_monthly() {
        let rows = parse_macro_uk_retail_monthly(&fixture("macro_uk_retail_monthly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_uk_retail_yearly() {
        let rows = parse_macro_uk_retail_yearly(&fixture("macro_uk_retail_yearly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_uk_rightmove_yearly() {
        let rows = parse_macro_uk_rightmove_yearly(&fixture("macro_uk_rightmove_yearly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_uk_rightmove_monthly() {
        let rows = parse_macro_uk_rightmove_monthly(&fixture("macro_uk_rightmove_monthly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_uk_gdp_quarterly() {
        let rows = parse_macro_uk_gdp_quarterly(&fixture("macro_uk_gdp_quarterly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_uk_gdp_yearly() {
        let rows = parse_macro_uk_gdp_yearly(&fixture("macro_uk_gdp_yearly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_uk_unemployment_rate() {
        let rows = parse_macro_uk_unemployment_rate(&fixture("macro_uk_unemployment_rate.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_swiss_svme() {
        let rows = parse_macro_swiss_svme(&fixture("macro_swiss_svme.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_swiss_trade() {
        let rows = parse_macro_swiss_trade(&fixture("macro_swiss_trade.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_swiss_cpi_yearly() {
        let rows = parse_macro_swiss_cpi_yearly(&fixture("macro_swiss_cpi_yearly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_swiss_gdp_quarterly() {
        let rows = parse_macro_swiss_gdp_quarterly(&fixture("macro_swiss_gdp_quarterly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_swiss_gbd_yearly() {
        let rows = parse_macro_swiss_gbd_yearly(&fixture("macro_swiss_gbd_yearly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_swiss_gbd_bank_rate() {
        let rows = parse_macro_swiss_gbd_bank_rate(&fixture("macro_swiss_gbd_bank_rate.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_japan_bank_rate() {
        let rows = parse_macro_japan_bank_rate(&fixture("macro_japan_bank_rate.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_japan_cpi_yearly() {
        let rows = parse_macro_japan_cpi_yearly(&fixture("macro_japan_cpi_yearly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_japan_core_cpi_yearly() {
        let rows = parse_macro_japan_core_cpi_yearly(&fixture("macro_japan_core_cpi_yearly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_japan_unemployment_rate() {
        let rows = parse_macro_japan_unemployment_rate(&fixture("macro_japan_unemployment_rate.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_japan_head_indicator() {
        let rows = parse_macro_japan_head_indicator(&fixture("macro_japan_head_indicator.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_germany_ifo() {
        let rows = parse_macro_germany_ifo(&fixture("macro_germany_ifo.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_germany_cpi_monthly() {
        let rows = parse_macro_germany_cpi_monthly(&fixture("macro_germany_cpi_monthly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_germany_cpi_yearly() {
        let rows = parse_macro_germany_cpi_yearly(&fixture("macro_germany_cpi_yearly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_germany_trade_adjusted() {
        let rows = parse_macro_germany_trade_adjusted(&fixture("macro_germany_trade_adjusted.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_germany_gdp() {
        let rows = parse_macro_germany_gdp(&fixture("macro_germany_gdp.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_germany_retail_sale_monthly() {
        let rows = parse_macro_germany_retail_sale_monthly(&fixture("macro_germany_retail_sale_monthly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_germany_retail_sale_yearly() {
        let rows = parse_macro_germany_retail_sale_yearly(&fixture("macro_germany_retail_sale_yearly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_germany_zew() {
        let rows = parse_macro_germany_zew(&fixture("macro_germany_zew.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_china_hk_cpi() {
        let rows = parse_macro_china_hk_cpi(&fixture("macro_china_hk_cpi.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_china_hk_cpi_ratio() {
        let rows = parse_macro_china_hk_cpi_ratio(&fixture("macro_china_hk_cpi_ratio.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_china_hk_rate_of_unemployment() {
        let rows = parse_macro_china_hk_rate_of_unemployment(&fixture("macro_china_hk_rate_of_unemployment.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_china_hk_gbp() {
        let rows = parse_macro_china_hk_gbp(&fixture("macro_china_hk_gbp.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_china_hk_gbp_ratio() {
        let rows = parse_macro_china_hk_gbp_ratio(&fixture("macro_china_hk_gbp_ratio.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_china_hk_building_volume() {
        let rows = parse_macro_china_hk_building_volume(&fixture("macro_china_hk_building_volume.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_china_hk_building_amount() {
        let rows = parse_macro_china_hk_building_amount(&fixture("macro_china_hk_building_amount.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_china_hk_trade_diff_ratio() {
        let rows = parse_macro_china_hk_trade_diff_ratio(&fixture("macro_china_hk_trade_diff_ratio.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_china_hk_ppi() {
        let rows = parse_macro_china_hk_ppi(&fixture("macro_china_hk_ppi.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024年03月");
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[1].date, "2024年02月");
        assert_eq!(rows[1].value, Some(3.2));
    }
}
