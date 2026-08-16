//! Foreign-country macro "core" fetchers (akshare `macro_uk.py`,
//! `macro_china_hk.py`, `macro_japan.py`, `macro_swiss.py`, `macro_germany.py`).
//!
//! Each `macro_*_core` is the generic Eastmoney `RPT_ECONOMICVALUE_*` fetcher: it
//! takes an `INDICATOR_ID` (`symbol`) and returns the full time series for that
//! indicator. The specific akshare functions in `macro_intl.rs` (e.g.
//! `macro_uk_unemployment_rate`) are thin wrappers that call the matching core
//! with a hard-coded `symbol`; these `*_core` functions expose the symbol
//! parameter directly, mirroring the Python API.

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

/// One observation of a `RPT_ECONOMICVALUE_*` indicator (akshare columns
/// `时间`/`前值`/`现值`/`发布日期`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Row {
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

/// Shared parser for every `macro_*_core` response.
pub(crate) fn parse_core(resp: &Value) -> Result<Vec<Row>> {
    let data = emg_data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(date) = fstr(item, "REPORT_DATE_CH").or_else(|| fstr(item, "REPORT_DATE")) else {
            continue;
        };
        out.push(Row {
            date,
            value: fnum(item, "VALUE"),
            pre_value: fnum(item, "PRE_VALUE"),
            publish_date: fstr(item, "PUBLISH_DATE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

/// Fetch a single `RPT_ECONOMICVALUE_*` indicator filtered by `INDICATOR_ID`.
async fn core_fetch(
    client: &Client,
    endpoint: &'static str,
    report_name: &'static str,
    symbol: &str,
) -> Result<Vec<Row>> {
    let filter = format!("(INDICATOR_ID=\"{symbol}\")");
    let params = [
        ("reportName", report_name),
        ("columns", "ALL"),
        ("filter", filter.as_str()),
        ("pageNumber", "1"),
        ("pageSize", "5000"),
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
    parse_core(&v)
}

/// UK macro indicator (`macro_uk_core`, Eastmoney `RPT_ECONOMICVALUE_BRITAIN`).
///
/// `symbol` is the Eastmoney `INDICATOR_ID` (akshare default `EMG00010348`,
/// unemployment rate). Returns the full time series for that indicator.
pub async fn macro_uk_core(client: &Client, symbol: &str) -> Result<Vec<Row>> {
    core_fetch(client, "macro_uk_core", "RPT_ECONOMICVALUE_BRITAIN", symbol).await
}

/// Hong Kong macro indicator (`macro_china_hk_core`, Eastmoney `RPT_ECONOMICVALUE_HK`).
///
/// `symbol` is the Eastmoney `INDICATOR_ID` (akshare default `EMG00341602`).
pub async fn macro_china_hk_core(client: &Client, symbol: &str) -> Result<Vec<Row>> {
    core_fetch(client, "macro_china_hk_core", "RPT_ECONOMICVALUE_HK", symbol).await
}

/// Japan macro indicator (`macro_japan_core`, Eastmoney `RPT_ECONOMICVALUE_JPAN`).
///
/// `symbol` is the Eastmoney `INDICATOR_ID` (akshare default `EMG00341602`).
pub async fn macro_japan_core(client: &Client, symbol: &str) -> Result<Vec<Row>> {
    core_fetch(client, "macro_japan_core", "RPT_ECONOMICVALUE_JPAN", symbol).await
}

/// Switzerland macro indicator (`macro_swiss_core`, Eastmoney `RPT_ECONOMICVALUE_CH`).
///
/// `symbol` is the Eastmoney `INDICATOR_ID` (akshare default `EMG00341602`).
pub async fn macro_swiss_core(client: &Client, symbol: &str) -> Result<Vec<Row>> {
    core_fetch(client, "macro_swiss_core", "RPT_ECONOMICVALUE_CH", symbol).await
}

/// Germany macro indicator (`macro_germany_core`, Eastmoney `RPT_ECONOMICVALUE_GER`).
///
/// `symbol` is the Eastmoney `INDICATOR_ID` (akshare default `EMG00179154`, IFO).
pub async fn macro_germany_core(client: &Client, symbol: &str) -> Result<Vec<Row>> {
    core_fetch(client, "macro_germany_core", "RPT_ECONOMICVALUE_GER", symbol).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let txt = std::fs::read_to_string(p).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    #[test]
    fn parses_macro_uk_core() {
        let rows = parse_core(&fixture("macro_uk_core.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-03");
        assert_eq!(rows[0].value, Some(4.2));
        assert_eq!(rows[0].pre_value, Some(4.3));
        assert_eq!(rows[0].publish_date.as_deref(), Some("2024-04-16T00:00:00"));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].date, "2024-02");
    }

    #[test]
    fn parses_macro_china_hk_core() {
        let rows = parse_core(&fixture("macro_china_hk_core.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-03");
        assert_eq!(rows[0].value, Some(48.5));
        assert_eq!(rows[1].pre_value, Some(49.2));
    }

    #[test]
    fn parses_macro_japan_core() {
        let rows = parse_core(&fixture("macro_japan_core.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-03");
        assert_eq!(rows[0].value, Some(2.7));
        assert_eq!(rows[1].value, Some(2.8));
    }

    #[test]
    fn parses_macro_swiss_core() {
        let rows = parse_core(&fixture("macro_swiss_core.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-03");
        assert_eq!(rows[0].value, Some(48.5));
        assert_eq!(rows[1].publish_date.as_deref(), Some("2024-03-01T00:00:00"));
    }

    #[test]
    fn parses_macro_germany_core() {
        let rows = parse_core(&fixture("macro_germany_core.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-03");
        assert_eq!(rows[0].value, Some(87.4));
        assert_eq!(rows[1].value, Some(85.5));
    }
}
