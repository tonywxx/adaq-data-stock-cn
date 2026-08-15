//! Additional macro indicators (akshare `economic/macro_china.py` &
//! `economic/macro_usa.py`), Eastmoney `datacenter-web` `reportName` endpoints.
//!
//! Six indicators are ported here, all hitting the same Eastmoney data-center
//! endpoint and returning `result.data` rows:
//! - `macro_china_pmi`              — `RPT_ECONOMY_PMI`
//! - `macro_china_gdzctz`           — `RPT_ECONOMY_ASSET_INVEST` (城镇固定资产投资)
//! - `macro_china_gyzjz`            — `RPT_ECONOMY_INDUS_GROW` (工业增加值增长)
//! - `macro_china_consumer_goods_retail` — `RPT_ECONOMY_TOTAL_RETAIL` (社会消费品零售总额)
//! - `macro_usa_cpi_yoy`            — `RPT_ECONOMICVALUE_USA` (filter `EMG00000733`)
//! - `macro_usa_phs`                — `RPT_ECONOMICVALUE_USA` (filter `EMG00342249`)
//!
//! Functions intentionally **not** ported here (and why):
//! - `macro_china_urban_unemployment`: source is `data.stats.gov.cn`, a `POST`
//!   with header-impersonation (`curl_requests` + `impersonate="chrome"`) — not
//!   the Eastmoney datacenter `result.data` envelope. Different fetch path.
//! - All other `macro_usa_*` functions (`macro_usa_unemployment_rate`,
//!   `macro_usa_pmi`, `macro_usa_gdp_monthly`, ...): these route through the
//!   Jin10 datacenter (`datacenter-api.jin10.com`) via `__macro_usa_base_func`,
//!   which needs `x-app-id`/`x-csrf-token` headers. Different source — skipped.
//!   Only `macro_usa_cpi_yoy` and `macro_usa_phs` use the Eastmoney
//!   `RPT_ECONOMICVALUE_USA` table (selected by an `INDICATOR_ID` filter).
//! - `macro_china_shrzgm` / `macro_china_fx_reserves_yearly` (already noted in
//!   `extra.rs`): MOFCOM / Jin10 sources — skipped.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

const BASE: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";

/// Extract `result.data` (the row array) from a datacenter-web response.
fn data_array(resp: &Value) -> Result<&Vec<Value>> {
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
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    })
}

// ---------------------------------------------------------------------------
// macro_china_pmi
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChinaPmi {
    /// Report period (month), e.g. `2024-03` (akshare `TIME` / 月份).
    pub date: String,
    /// Manufacturing PMI index (akshare `MAKE_INDEX` / 制造业-指数).
    pub manufacturing_index: Option<f64>,
    /// Manufacturing PMI year-over-year (akshare `MAKE_SAME` / 制造业-同比增长).
    pub manufacturing_yoy: Option<f64>,
    /// Non-manufacturing PMI index (akshare `NMAKE_INDEX` / 非制造业-指数).
    pub non_manufacturing_index: Option<f64>,
    /// Non-manufacturing PMI year-over-year (akshare `NMAKE_SAME` / 非制造业-同比增长).
    pub non_manufacturing_yoy: Option<f64>,
    pub source: &'static str,
}

/// China PMI (`macro_china_pmi`, Eastmoney `RPT_ECONOMY_PMI`).
pub async fn macro_china_pmi(client: &Client) -> Result<Vec<ChinaPmi>> {
    const COLUMNS: &str = "REPORT_DATE,TIME,MAKE_INDEX,MAKE_SAME,NMAKE_INDEX,NMAKE_SAME";
    let params = [
        ("reportName", "RPT_ECONOMY_PMI"),
        ("columns", COLUMNS),
        ("pageNumber", "1"),
        ("pageSize", "2000"),
        ("sortColumns", "REPORT_DATE"),
        ("sortTypes", "-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("p", "1"),
        ("pageNo", "1"),
        ("pageNum", "1"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "macro_china_pmi", BASE, &params)
        .await?;
    parse_china_pmi(&v)
}

pub(crate) fn parse_china_pmi(resp: &Value) -> Result<Vec<ChinaPmi>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(date) = fstr(item, "TIME") else {
            continue;
        };
        out.push(ChinaPmi {
            date,
            manufacturing_index: fnum(item, "MAKE_INDEX"),
            manufacturing_yoy: fnum(item, "MAKE_SAME"),
            non_manufacturing_index: fnum(item, "NMAKE_INDEX"),
            non_manufacturing_yoy: fnum(item, "NMAKE_SAME"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// macro_china_gdzctz (城镇固定资产投资)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChinaFixedAssetInvest {
    /// Report period (month), e.g. `2024-03` (akshare `TIME` / 月份).
    pub date: String,
    /// Current-month fixed asset investment (亿元, akshare `BASE` / 当月).
    pub current: Option<f64>,
    /// Year-over-year (%, akshare `BASE_SAME` / 同比增长).
    pub yoy: Option<f64>,
    /// Month-over-month (%, akshare `BASE_SEQUENTIAL` / 环比增长).
    pub mom: Option<f64>,
    /// Accumulated since year start (亿元, akshare `BASE_ACCUMULATE` / 自年初累计).
    pub accumulate: Option<f64>,
    pub source: &'static str,
}

/// China urban fixed asset investment (`macro_china_gdzctz`, Eastmoney `RPT_ECONOMY_ASSET_INVEST`).
pub async fn macro_china_gdzctz(client: &Client) -> Result<Vec<ChinaFixedAssetInvest>> {
    const COLUMNS: &str = "REPORT_DATE,TIME,BASE,BASE_SAME,BASE_SEQUENTIAL,BASE_ACCUMULATE";
    let params = [
        ("reportName", "RPT_ECONOMY_ASSET_INVEST"),
        ("columns", COLUMNS),
        ("pageNumber", "1"),
        ("pageSize", "2000"),
        ("sortColumns", "REPORT_DATE"),
        ("sortTypes", "-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("p", "1"),
        ("pageNo", "1"),
        ("pageNum", "1"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "macro_china_gdzctz", BASE, &params)
        .await?;
    parse_china_gdzctz(&v)
}

pub(crate) fn parse_china_gdzctz(resp: &Value) -> Result<Vec<ChinaFixedAssetInvest>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(date) = fstr(item, "TIME") else {
            continue;
        };
        out.push(ChinaFixedAssetInvest {
            date,
            current: fnum(item, "BASE"),
            yoy: fnum(item, "BASE_SAME"),
            mom: fnum(item, "BASE_SEQUENTIAL"),
            accumulate: fnum(item, "BASE_ACCUMULATE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// macro_china_gyzjz (工业增加值增长)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChinaIndustrialAddedValue {
    /// Report period (month), e.g. `2024-03` (akshare `TIME` / 月份).
    pub date: String,
    /// Year-over-year growth (%, akshare `BASE_SAME` / 同比增长).
    pub yoy: Option<f64>,
    /// Accumulated year-over-year growth (%, akshare `BASE_ACCUMULATE` / 累计增长).
    pub accumulate_yoy: Option<f64>,
    /// Publish date, e.g. `2024-03-31` (akshare `REPORT_DATE` / 发布时间).
    pub report_date: Option<String>,
    pub source: &'static str,
}

/// China industrial added value growth (`macro_china_gyzjz`, Eastmoney `RPT_ECONOMY_INDUS_GROW`).
pub async fn macro_china_gyzjz(client: &Client) -> Result<Vec<ChinaIndustrialAddedValue>> {
    const COLUMNS: &str = "REPORT_DATE,TIME,BASE_SAME,BASE_ACCUMULATE";
    let params = [
        ("reportName", "RPT_ECONOMY_INDUS_GROW"),
        ("columns", COLUMNS),
        ("pageNumber", "1"),
        ("pageSize", "2000"),
        ("sortColumns", "REPORT_DATE"),
        ("sortTypes", "-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("p", "1"),
        ("pageNo", "1"),
        ("pageNum", "1"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "macro_china_gyzjz", BASE, &params)
        .await?;
    parse_china_gyzjz(&v)
}

pub(crate) fn parse_china_gyzjz(resp: &Value) -> Result<Vec<ChinaIndustrialAddedValue>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(date) = fstr(item, "TIME") else {
            continue;
        };
        out.push(ChinaIndustrialAddedValue {
            date,
            yoy: fnum(item, "BASE_SAME"),
            accumulate_yoy: fnum(item, "BASE_ACCUMULATE"),
            report_date: fstr(item, "REPORT_DATE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// macro_china_consumer_goods_retail (社会消费品零售总额)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChinaRetailSales {
    /// Report period (month), e.g. `2024-03` (akshare `TIME` / 月份).
    pub date: String,
    /// Current-month total retail sales (亿元, akshare `RETAIL_TOTAL` / 当月).
    pub current: Option<f64>,
    /// Year-over-year (%, akshare `RETAIL_TOTAL_SAME` / 同比增长).
    pub yoy: Option<f64>,
    /// Month-over-month (%, akshare `RETAIL_TOTAL_SEQUENTIAL` / 环比增长).
    pub mom: Option<f64>,
    /// Accumulated total (亿元, akshare `RETAIL_TOTAL_ACCUMULATE` / 累计).
    pub accumulate: Option<f64>,
    /// Accumulated year-over-year (%, akshare `RETAIL_ACCUMULATE_SAME` / 累计-同比增长).
    pub accumulate_yoy: Option<f64>,
    pub source: &'static str,
}

/// China total retail sales of consumer goods (`macro_china_consumer_goods_retail`,
/// Eastmoney `RPT_ECONOMY_TOTAL_RETAIL`).
pub async fn macro_china_consumer_goods_retail(
    client: &Client,
) -> Result<Vec<ChinaRetailSales>> {
    const COLUMNS: &str = "REPORT_DATE,TIME,RETAIL_TOTAL,RETAIL_TOTAL_SAME,\
        RETAIL_TOTAL_SEQUENTIAL,RETAIL_TOTAL_ACCUMULATE,RETAIL_ACCUMULATE_SAME";
    let params = [
        ("reportName", "RPT_ECONOMY_TOTAL_RETAIL"),
        ("columns", COLUMNS),
        ("pageNumber", "1"),
        ("pageSize", "1000"),
        ("sortColumns", "REPORT_DATE"),
        ("sortTypes", "-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("p", "1"),
        ("pageNo", "1"),
        ("pageNum", "1"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "macro_china_consumer_goods_retail",
            BASE,
            &params,
        )
        .await?;
    parse_china_consumer_goods_retail(&v)
}

pub(crate) fn parse_china_consumer_goods_retail(resp: &Value) -> Result<Vec<ChinaRetailSales>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(date) = fstr(item, "TIME") else {
            continue;
        };
        out.push(ChinaRetailSales {
            date,
            current: fnum(item, "RETAIL_TOTAL"),
            yoy: fnum(item, "RETAIL_TOTAL_SAME"),
            mom: fnum(item, "RETAIL_TOTAL_SEQUENTIAL"),
            accumulate: fnum(item, "RETAIL_TOTAL_ACCUMULATE"),
            accumulate_yoy: fnum(item, "RETAIL_ACCUMULATE_SAME"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// macro_usa_cpi_yoy / macro_usa_phs (RPT_ECONOMICVALUE_USA)
// ---------------------------------------------------------------------------
//
// Both USA indicators come from the same Eastmoney `RPT_ECONOMICVALUE_USA`
// table, distinguished only by the `INDICATOR_ID` filter. The table returns
// `REPORT_DATE` (period), `PUBLISH_DATE`, `VALUE` (current), `PRE_VALUE`
// (previous) when `columns=ALL`.

#[derive(Debug, Clone, serde::Serialize)]
pub struct UsaCpi {
    /// Report period (month), e.g. `2024-03-01` (Eastmoney `REPORT_DATE` / 时间).
    pub date: String,
    /// Publish date, e.g. `2024-04-10` (Eastmoney `PUBLISH_DATE` / 发布日期).
    pub publish_date: Option<String>,
    /// Current CPI YoY (%, Eastmoney `VALUE` / 现值).
    pub value: Option<f64>,
    /// Previous CPI YoY (%, Eastmoney `PRE_VALUE` / 前值).
    pub pre_value: Option<f64>,
    pub source: &'static str,
}

/// US CPI year-over-year (`macro_usa_cpi_yoy`, Eastmoney `RPT_ECONOMICVALUE_USA`,
/// indicator `EMG00000733`).
pub async fn macro_usa_cpi_yoy(client: &Client) -> Result<Vec<UsaCpi>> {
    let params = [
        ("reportName", "RPT_ECONOMICVALUE_USA"),
        ("columns", "ALL"),
        ("filter", r#"(INDICATOR_ID="EMG00000733")"#),
        ("sortColumns", "REPORT_DATE"),
        ("sortTypes", "-1"),
        ("source", "WEB"),
        ("client", "WEB"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "macro_usa_cpi_yoy", BASE, &params)
        .await?;
    parse_usa_cpi_yoy(&v)
}

pub(crate) fn parse_usa_cpi_yoy(resp: &Value) -> Result<Vec<UsaCpi>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(date) = fstr(item, "REPORT_DATE") else {
            continue;
        };
        out.push(UsaCpi {
            date,
            publish_date: fstr(item, "PUBLISH_DATE"),
            value: fnum(item, "VALUE"),
            pre_value: fnum(item, "PRE_VALUE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct UsaPhs {
    /// Report period (month), e.g. `2024-03-01` (Eastmoney `REPORT_DATE` / 时间).
    pub date: String,
    /// Publish date, e.g. `2024-04-29` (Eastmoney `PUBLISH_DATE` / 发布日期).
    pub publish_date: Option<String>,
    /// Pending home sales MoM (%, Eastmoney `VALUE` / 现值).
    pub value: Option<f64>,
    /// Previous pending home sales MoM (%, Eastmoney `PRE_VALUE` / 前值).
    pub pre_value: Option<f64>,
    pub source: &'static str,
}

/// US pending home sales MoM (`macro_usa_phs`, Eastmoney `RPT_ECONOMICVALUE_USA`,
/// indicator `EMG00342249`).
pub async fn macro_usa_phs(client: &Client) -> Result<Vec<UsaPhs>> {
    let params = [
        ("reportName", "RPT_ECONOMICVALUE_USA"),
        ("columns", "ALL"),
        ("filter", r#"(INDICATOR_ID="EMG00342249")"#),
        ("pageNumber", "1"),
        ("pageSize", "2000"),
        ("sortColumns", "REPORT_DATE"),
        ("sortTypes", "-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("p", "1"),
        ("pageNo", "1"),
        ("pageNum", "1"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "macro_usa_phs", BASE, &params)
        .await?;
    parse_usa_phs(&v)
}

pub(crate) fn parse_usa_phs(resp: &Value) -> Result<Vec<UsaPhs>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(date) = fstr(item, "REPORT_DATE") else {
            continue;
        };
        out.push(UsaPhs {
            date,
            publish_date: fstr(item, "PUBLISH_DATE"),
            value: fnum(item, "VALUE"),
            pre_value: fnum(item, "PRE_VALUE"),
            source: SOURCE_EASTMONEY,
        });
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
        let txt = std::fs::read_to_string(p).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    #[test]
    fn parses_macro_china_pmi() {
        let rows = parse_china_pmi(&fixture("macro_china_pmi.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-03");
        assert_eq!(rows[0].manufacturing_index, Some(50.8));
        assert_eq!(rows[0].manufacturing_yoy, None);
        assert_eq!(rows[0].non_manufacturing_index, Some(53.0));
        assert_eq!(rows[0].non_manufacturing_yoy, Some(1.2));
        assert_eq!(rows[1].date, "2024-02");
        assert_eq!(rows[1].manufacturing_yoy, Some(0.5));
    }

    #[test]
    fn parses_macro_china_gdzctz() {
        let rows = parse_china_gdzctz(&fixture("macro_china_gdzctz.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-03");
        assert_eq!(rows[0].current, Some(50000.0));
        assert_eq!(rows[0].yoy, Some(4.5));
        assert_eq!(rows[0].mom, None);
        assert_eq!(rows[0].accumulate, Some(100000.0));
        assert_eq!(rows[1].mom, Some(1.0));
    }

    #[test]
    fn parses_macro_china_gyzjz() {
        let rows = parse_china_gyzjz(&fixture("macro_china_gyzjz.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-03");
        assert_eq!(rows[0].yoy, Some(4.5));
        assert_eq!(rows[0].accumulate_yoy, Some(6.1));
        assert_eq!(rows[0].report_date, Some("2024-03-31".to_string()));
        assert_eq!(rows[1].accumulate_yoy, Some(7.0));
    }

    #[test]
    fn parses_macro_china_consumer_goods_retail() {
        let rows =
            parse_china_consumer_goods_retail(&fixture("macro_china_consumer_goods_retail.json"))
                .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-03");
        assert_eq!(rows[0].current, Some(39010.0));
        assert_eq!(rows[0].yoy, Some(3.1));
        assert_eq!(rows[0].mom, Some(0.5));
        assert_eq!(rows[0].accumulate, Some(120327.0));
        assert_eq!(rows[0].accumulate_yoy, Some(4.7));
        assert_eq!(rows[1].mom, None);
    }

    #[test]
    fn parses_macro_usa_cpi_yoy() {
        let rows = parse_usa_cpi_yoy(&fixture("macro_usa_cpi_yoy.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-03-01");
        assert_eq!(rows[0].publish_date, Some("2024-04-10".to_string()));
        assert_eq!(rows[0].value, Some(3.5));
        assert_eq!(rows[0].pre_value, Some(3.2));
        assert_eq!(rows[1].value, Some(3.2));
    }

    #[test]
    fn parses_macro_usa_phs() {
        let rows = parse_usa_phs(&fixture("macro_usa_phs.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-03-01");
        assert_eq!(rows[0].publish_date, Some("2024-04-29".to_string()));
        assert_eq!(rows[0].value, Some(-0.2));
        assert_eq!(rows[0].pre_value, Some(1.0));
        assert_eq!(rows[1].value, Some(1.0));
    }
}
