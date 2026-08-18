//! China macro indicators (akshare `economic/macro_china.py`, Eastmoney `datacenter-web` reportName endpoints).
//!
//! All four ported functions hit the same Eastmoney data-center endpoint with a
//! different `reportName`/`columns` pair and return `result.data` rows.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};
use crate::core::json::*;

const BASE: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";
const PAGE_SIZE: &str = "2000";

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

/// Shared param block for the datacenter-web reportName endpoint (mirrors akshare).
fn report_params(
    report_name: &'static str,
    columns: &'static str,
) -> [(&'static str, &'static str); 11] {
    [
        ("reportName", report_name),
        ("columns", columns),
        ("pageNumber", "1"),
        ("pageSize", PAGE_SIZE),
        ("sortColumns", "REPORT_DATE"),
        ("sortTypes", "-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("p", "1"),
        ("pageNo", "1"),
        ("pageNum", "1"),
    ]
}

// ---------------------------------------------------------------------------
// macro_china_gdp
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChinaGdp {
    /// Report period (quarter), e.g. `2024-03`.
    pub date: String,
    /// GDP total absolute value (亿元).
    pub gdp_total: Option<f64>,
    /// GDP total year-over-year (%).
    pub gdp_total_yoy: Option<f64>,
    /// Primary industry absolute value (亿元).
    pub primary_total: Option<f64>,
    /// Primary industry year-over-year (%).
    pub primary_yoy: Option<f64>,
    /// Secondary industry absolute value (亿元).
    pub secondary_total: Option<f64>,
    /// Secondary industry year-over-year (%).
    pub secondary_yoy: Option<f64>,
    /// Tertiary industry absolute value (亿元).
    pub tertiary_total: Option<f64>,
    /// Tertiary industry year-over-year (%).
    pub tertiary_yoy: Option<f64>,
    pub source: &'static str,
}

/// China GDP (`macro_china_gdp`, Eastmoney `RPT_ECONOMY_GDP`).
pub async fn macro_china_gdp(client: &Client) -> Result<Vec<ChinaGdp>> {
    const COLUMNS: &str = "REPORT_DATE,TIME,DOMESTICL_PRODUCT_BASE,FIRST_PRODUCT_BASE,\
        SECOND_PRODUCT_BASE,THIRD_PRODUCT_BASE,SUM_SAME,FIRST_SAME,SECOND_SAME,THIRD_SAME";
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "macro_china_gdp",
            BASE,
            &report_params("RPT_ECONOMY_GDP", COLUMNS),
        )
        .await?;
    parse_china_gdp(&v)
}

pub(crate) fn parse_china_gdp(resp: &Value) -> Result<Vec<ChinaGdp>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(date) = opt_str(item, "TIME") else {
            continue;
        };
        out.push(ChinaGdp {
            date,
            gdp_total: opt_f64(item, "DOMESTICL_PRODUCT_BASE"),
            gdp_total_yoy: opt_f64(item, "SUM_SAME"),
            primary_total: opt_f64(item, "FIRST_PRODUCT_BASE"),
            primary_yoy: opt_f64(item, "FIRST_SAME"),
            secondary_total: opt_f64(item, "SECOND_PRODUCT_BASE"),
            secondary_yoy: opt_f64(item, "SECOND_SAME"),
            tertiary_total: opt_f64(item, "THIRD_PRODUCT_BASE"),
            tertiary_yoy: opt_f64(item, "THIRD_SAME"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// macro_china_cpi
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChinaCpi {
    /// Report period (month), e.g. `2024-03`.
    pub date: String,
    /// National CPI current-month index.
    pub national_current: Option<f64>,
    /// National CPI year-over-year (%).
    pub national_yoy: Option<f64>,
    /// National CPI month-over-month (%).
    pub national_mom: Option<f64>,
    /// National CPI accumulated.
    pub national_accumulate: Option<f64>,
    /// City CPI current-month index.
    pub city_current: Option<f64>,
    /// City CPI year-over-year (%).
    pub city_yoy: Option<f64>,
    /// City CPI month-over-month (%).
    pub city_mom: Option<f64>,
    /// City CPI accumulated.
    pub city_accumulate: Option<f64>,
    /// Rural CPI current-month index.
    pub rural_current: Option<f64>,
    /// Rural CPI year-over-year (%).
    pub rural_yoy: Option<f64>,
    /// Rural CPI month-over-month (%).
    pub rural_mom: Option<f64>,
    /// Rural CPI accumulated.
    pub rural_accumulate: Option<f64>,
    pub source: &'static str,
}

/// China CPI (`macro_china_cpi`, Eastmoney `RPT_ECONOMY_CPI`).
pub async fn macro_china_cpi(client: &Client) -> Result<Vec<ChinaCpi>> {
    const COLUMNS: &str = "REPORT_DATE,TIME,NATIONAL_BASE,NATIONAL_SAME,NATIONAL_SEQUENTIAL,\
        NATIONAL_ACCUMULATE,CITY_BASE,CITY_SAME,CITY_SEQUENTIAL,CITY_ACCUMULATE,\
        RURAL_BASE,RURAL_SAME,RURAL_SEQUENTIAL,RURAL_ACCUMULATE";
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "macro_china_cpi",
            BASE,
            &report_params("RPT_ECONOMY_CPI", COLUMNS),
        )
        .await?;
    parse_china_cpi(&v)
}

pub(crate) fn parse_china_cpi(resp: &Value) -> Result<Vec<ChinaCpi>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(date) = opt_str(item, "TIME") else {
            continue;
        };
        out.push(ChinaCpi {
            date,
            national_current: opt_f64(item, "NATIONAL_BASE"),
            national_yoy: opt_f64(item, "NATIONAL_SAME"),
            national_mom: opt_f64(item, "NATIONAL_SEQUENTIAL"),
            national_accumulate: opt_f64(item, "NATIONAL_ACCUMULATE"),
            city_current: opt_f64(item, "CITY_BASE"),
            city_yoy: opt_f64(item, "CITY_SAME"),
            city_mom: opt_f64(item, "CITY_SEQUENTIAL"),
            city_accumulate: opt_f64(item, "CITY_ACCUMULATE"),
            rural_current: opt_f64(item, "RURAL_BASE"),
            rural_yoy: opt_f64(item, "RURAL_SAME"),
            rural_mom: opt_f64(item, "RURAL_SEQUENTIAL"),
            rural_accumulate: opt_f64(item, "RURAL_ACCUMULATE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// macro_china_ppi
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChinaPpi {
    /// Report period (month), e.g. `2024-03`.
    pub date: String,
    /// PPI current-month index.
    pub current: Option<f64>,
    /// PPI year-over-year (%).
    pub yoy: Option<f64>,
    /// PPI accumulated.
    pub accumulate: Option<f64>,
    pub source: &'static str,
}

/// China PPI (`macro_china_ppi`, Eastmoney `RPT_ECONOMY_PPI`).
pub async fn macro_china_ppi(client: &Client) -> Result<Vec<ChinaPpi>> {
    const COLUMNS: &str = "REPORT_DATE,TIME,BASE,BASE_SAME,BASE_ACCUMULATE";
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "macro_china_ppi",
            BASE,
            &report_params("RPT_ECONOMY_PPI", COLUMNS),
        )
        .await?;
    parse_china_ppi(&v)
}

pub(crate) fn parse_china_ppi(resp: &Value) -> Result<Vec<ChinaPpi>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(date) = opt_str(item, "TIME") else {
            continue;
        };
        out.push(ChinaPpi {
            date,
            current: opt_f64(item, "BASE"),
            yoy: opt_f64(item, "BASE_SAME"),
            accumulate: opt_f64(item, "BASE_ACCUMULATE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// macro_china_money_supply
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChinaMoneySupply {
    /// Report period (month), e.g. `2024-03`.
    pub date: String,
    /// M2 (货币和准货币) amount (亿元).
    pub m2: Option<f64>,
    /// M2 year-over-year (%).
    pub m2_yoy: Option<f64>,
    /// M2 month-over-month (%).
    pub m2_mom: Option<f64>,
    /// M1 (货币) amount (亿元).
    pub m1: Option<f64>,
    /// M1 year-over-year (%).
    pub m1_yoy: Option<f64>,
    /// M1 month-over-month (%).
    pub m1_mom: Option<f64>,
    /// M0 (流通中的现金) amount (亿元).
    pub m0: Option<f64>,
    /// M0 year-over-year (%).
    pub m0_yoy: Option<f64>,
    /// M0 month-over-month (%).
    pub m0_mom: Option<f64>,
    pub source: &'static str,
}

/// China money supply (`macro_china_money_supply`, Eastmoney `RPT_ECONOMY_CURRENCY_SUPPLY`).
pub async fn macro_china_money_supply(client: &Client) -> Result<Vec<ChinaMoneySupply>> {
    const COLUMNS: &str = "REPORT_DATE,TIME,BASIC_CURRENCY,BASIC_CURRENCY_SAME,BASIC_CURRENCY_SEQUENTIAL,\
        CURRENCY,CURRENCY_SAME,CURRENCY_SEQUENTIAL,FREE_CASH,FREE_CASH_SAME,FREE_CASH_SEQUENTIAL";
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "macro_china_money_supply",
            BASE,
            &report_params("RPT_ECONOMY_CURRENCY_SUPPLY", COLUMNS),
        )
        .await?;
    parse_china_money_supply(&v)
}

pub(crate) fn parse_china_money_supply(resp: &Value) -> Result<Vec<ChinaMoneySupply>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(date) = opt_str(item, "TIME") else {
            continue;
        };
        out.push(ChinaMoneySupply {
            date,
            m2: opt_f64(item, "BASIC_CURRENCY"),
            m2_yoy: opt_f64(item, "BASIC_CURRENCY_SAME"),
            m2_mom: opt_f64(item, "BASIC_CURRENCY_SEQUENTIAL"),
            m1: opt_f64(item, "CURRENCY"),
            m1_yoy: opt_f64(item, "CURRENCY_SAME"),
            m1_mom: opt_f64(item, "CURRENCY_SEQUENTIAL"),
            m0: opt_f64(item, "FREE_CASH"),
            m0_yoy: opt_f64(item, "FREE_CASH_SAME"),
            m0_mom: opt_f64(item, "FREE_CASH_SEQUENTIAL"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// helpers
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
    fn parses_macro_china_gdp() {
        let rows = parse_china_gdp(&fixture("macro_china_gdp.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-03");
        assert_eq!(rows[0].gdp_total, Some(296299.0));
        assert_eq!(rows[0].gdp_total_yoy, Some(5.3));
        assert_eq!(rows[0].tertiary_yoy, Some(5.0));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].date, "2023-12");
    }

    #[test]
    fn parses_macro_china_cpi() {
        let rows = parse_china_cpi(&fixture("macro_china_cpi.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-03");
        assert_eq!(rows[0].national_current, Some(100.7));
        assert_eq!(rows[0].national_yoy, Some(0.1));
        assert_eq!(rows[0].national_mom, Some(-1.0));
        assert_eq!(rows[0].city_yoy, Some(0.2));
        assert_eq!(rows[1].rural_yoy, Some(0.5));
    }

    #[test]
    fn parses_macro_china_ppi() {
        let rows = parse_china_ppi(&fixture("macro_china_ppi.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-03");
        assert_eq!(rows[0].current, Some(100.3));
        assert_eq!(rows[0].yoy, Some(-2.8));
        assert_eq!(rows[0].accumulate, Some(-2.7));
        assert_eq!(rows[1].yoy, Some(-2.5));
    }

    #[test]
    fn parses_macro_china_money_supply() {
        let rows = parse_china_money_supply(&fixture("macro_china_money_supply.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-03");
        assert_eq!(rows[0].m2, Some(3047952.0));
        assert_eq!(rows[0].m2_yoy, Some(8.3));
        assert_eq!(rows[0].m2_mom, Some(1.2));
        assert_eq!(rows[0].m1, Some(685383.0));
        assert_eq!(rows[0].m0_yoy, Some(11.0));
        assert_eq!(rows[1].m1_yoy, Some(1.0));
    }
}
