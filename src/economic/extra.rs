//! Extra China macro indicators (akshare `economic/macro_china.py`).
//!
//! Each function hits the Eastmoney `datacenter-web` `reportName` endpoint (the
//! same shape as `china.rs`) and returns the `result.data` rows. Unlike
//! `china.rs`, a few of these endpoints carry an extra `filter`/`token` param
//! or a smaller page size, so the param builders here are tailored per call.
//!
//! Functions intentionally **not** ported here (and why):
//! - `macro_china_social_pay` (社会融资规模, akshare `macro_china_shrzgm`):
//!   source is MOFCOM (`data.mofcom.gov.cn`), a `POST` returning a bare JSON
//!   array — not the Eastmoney datacenter `result.data` envelope. Different
//!   fetch path + TLS adapter required; out of scope for pure Eastmoney JSON.
//! - `macro_china_foreign_reserve` (外汇储备, akshare `macro_china_fx_reserves_yearly`):
//!   source is Jin10 datacenter (`datacenter-api.jin10.com`), needs
//!   `x-app-id`/`x-csrf-token` headers + a paginated loop. Different source.
//! - `macro_china_trade` (进出口, akshare `macro_china_trade_balance`):
//!   Jin10 datacenter, same as above.

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

/// Shared param block for a datacenter-web `reportName` endpoint (akshare style).
fn dc_params(report_name: &'static str, columns: &'static str) -> [(&'static str, &'static str); 11] {
    [
        ("reportName", report_name),
        ("columns", columns),
        ("pageNumber", "1"),
        ("pageSize", "2000"),
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
// macro_china_new_house_price
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChinaNewHousePrice {
    /// Report period (month), e.g. `2024-03`.
    pub date: String,
    /// City name, e.g. `北京`.
    pub city: String,
    /// New commercial residential price index — year-over-year (同比, akshare `FIRST_COMHOUSE_SAME`).
    pub new_house_yoy: Option<f64>,
    /// New commercial residential price index — month-over-month (环比, akshare `FIRST_COMHOUSE_SEQUENTIAL`).
    pub new_house_mom: Option<f64>,
    /// New commercial residential price index — fixed-base (定基, akshare `FIRST_COMHOUSE_BASE`).
    pub new_house_base: Option<f64>,
    /// Second-hand residential price index — year-over-year (同比, akshare `SECOND_HOUSE_SAME`).
    pub second_hand_yoy: Option<f64>,
    /// Second-hand residential price index — month-over-month (环比, akshare `SECOND_HOUSE_SEQUENTIAL`).
    pub second_hand_mom: Option<f64>,
    /// Second-hand residential price index — fixed-base (定基, akshare `SECOND_HOUSE_BASE`).
    pub second_hand_base: Option<f64>,
    pub source: &'static str,
}

/// New commercial & second-hand residential price indices by city
/// (`macro_china_new_house_price`, Eastmoney `RPT_ECONOMY_HOUSE_PRICE`).
///
/// `city_first`/`city_second` select which cities to include (akshare defaults
/// to `北京`/`上海`).
pub async fn macro_china_new_house_price(
    client: &Client,
    city_first: &str,
    city_second: &str,
) -> Result<Vec<ChinaNewHousePrice>> {
    let filter = format!("(CITY in (\"{city_first}\",\"{city_second}\"))");
    let params = [
        ("reportName", "RPT_ECONOMY_HOUSE_PRICE"),
        (
            "columns",
            "REPORT_DATE,CITY,FIRST_COMHOUSE_SAME,FIRST_COMHOUSE_SEQUENTIAL,\
             FIRST_COMHOUSE_BASE,SECOND_HOUSE_SAME,SECOND_HOUSE_SEQUENTIAL,SECOND_HOUSE_BASE,REPORT_DAY",
        ),
        ("filter", filter.as_str()),
        ("pageNumber", "1"),
        ("pageSize", "500"),
        ("sortColumns", "REPORT_DATE,CITY"),
        ("sortTypes", "-1,-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("p", "1"),
        ("pageNo", "1"),
        ("pageNum", "1"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "macro_china_new_house_price", BASE, &params)
        .await?;
    parse_china_new_house_price(&v)
}

pub(crate) fn parse_china_new_house_price(resp: &Value) -> Result<Vec<ChinaNewHousePrice>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(date) = fstr(item, "REPORT_DATE") else {
            continue;
        };
        let city = fstr(item, "CITY").unwrap_or_default();
        out.push(ChinaNewHousePrice {
            date,
            city,
            new_house_yoy: fnum(item, "FIRST_COMHOUSE_SAME"),
            new_house_mom: fnum(item, "FIRST_COMHOUSE_SEQUENTIAL"),
            new_house_base: fnum(item, "FIRST_COMHOUSE_BASE"),
            second_hand_yoy: fnum(item, "SECOND_HOUSE_SAME"),
            second_hand_mom: fnum(item, "SECOND_HOUSE_SEQUENTIAL"),
            second_hand_base: fnum(item, "SECOND_HOUSE_BASE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// macro_china_lpr
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChinaLpr {
    /// Trade date, e.g. `2024-03-20`.
    pub date: String,
    /// 1-year LPR (akshare `LPR1Y`).
    pub lpr_1y: Option<f64>,
    /// 5-year LPR (akshare `LPR5Y`).
    pub lpr_5y: Option<f64>,
    /// 1-year releasable loan rate (akshare `RATE_1`).
    pub rate_1: Option<f64>,
    /// 5-year releasable loan rate (akshare `RATE_2`).
    pub rate_2: Option<f64>,
    pub source: &'static str,
}

/// Loan Prime Rate (LPR) detail (`macro_china_lpr`, Eastmoney `RPTA_WEB_RATE`).
///
/// The endpoint requires the public Eastmoney datacenter token that akshare
/// ships with; the response is paginated server-side but one page (500 rows)
/// is fetched here, matching the other single-page indicators.
pub async fn macro_china_lpr(client: &Client) -> Result<Vec<ChinaLpr>> {
    const TOKEN: &str = "894050c76af8597a853f5b408b759f5d";
    let params = [
        ("reportName", "RPTA_WEB_RATE"),
        ("columns", "ALL"),
        ("sortColumns", "TRADE_DATE"),
        ("sortTypes", "-1"),
        ("token", TOKEN),
        ("pageNumber", "1"),
        ("pageSize", "500"),
        ("p", "1"),
        ("pageNo", "1"),
        ("pageNum", "1"),
        ("source", "WEB"),
        ("client", "WEB"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "macro_china_lpr", BASE, &params)
        .await?;
    parse_china_lpr(&v)
}

pub(crate) fn parse_china_lpr(resp: &Value) -> Result<Vec<ChinaLpr>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(date) = fstr(item, "TRADE_DATE") else {
            continue;
        };
        out.push(ChinaLpr {
            date,
            lpr_1y: fnum(item, "LPR1Y"),
            lpr_5y: fnum(item, "LPR5Y"),
            rate_1: fnum(item, "RATE_1"),
            rate_2: fnum(item, "RATE_2"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// macro_china_enterprise_boom_index
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChinaEnterpriseBoom {
    /// Report period (quarter), e.g. `2024-03`.
    pub date: String,
    /// Enterprise boom index — index value (akshare `BOOM_INDEX`).
    pub boom_index: Option<f64>,
    /// Enterprise boom index — year-over-year (akshare `BOOM_INDEX_SAME`).
    pub boom_index_yoy: Option<f64>,
    /// Enterprise boom index — quarter-over-quarter (akshare `BOOM_INDEX_SEQUENTIAL`).
    pub boom_index_mom: Option<f64>,
    /// Entrepreneur confidence index — index value (akshare `FAITH_INDEX`).
    pub faith_index: Option<f64>,
    /// Entrepreneur confidence index — year-over-year (akshare `FAITH_INDEX_SAME`).
    pub faith_index_yoy: Option<f64>,
    /// Entrepreneur confidence index — quarter-over-quarter (akshare `FAITH_INDEX_SEQUENTIAL`).
    pub faith_index_mom: Option<f64>,
    pub source: &'static str,
}

/// Enterprise boom & entrepreneur confidence indices
/// (`macro_china_enterprise_boom_index`, Eastmoney `RPT_ECONOMY_BOOM_INDEX`).
pub async fn macro_china_enterprise_boom_index(client: &Client) -> Result<Vec<ChinaEnterpriseBoom>> {
    const COLUMNS: &str = "REPORT_DATE,TIME,BOOM_INDEX,FAITH_INDEX,BOOM_INDEX_SAME,\
        BOOM_INDEX_SEQUENTIAL,FAITH_INDEX_SAME,FAITH_INDEX_SEQUENTIAL";
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "macro_china_enterprise_boom_index",
            BASE,
            &dc_params("RPT_ECONOMY_BOOM_INDEX", COLUMNS),
        )
        .await?;
    parse_china_enterprise_boom_index(&v)
}

pub(crate) fn parse_china_enterprise_boom_index(resp: &Value) -> Result<Vec<ChinaEnterpriseBoom>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(date) = fstr(item, "TIME") else {
            continue;
        };
        out.push(ChinaEnterpriseBoom {
            date,
            boom_index: fnum(item, "BOOM_INDEX"),
            boom_index_yoy: fnum(item, "BOOM_INDEX_SAME"),
            boom_index_mom: fnum(item, "BOOM_INDEX_SEQUENTIAL"),
            faith_index: fnum(item, "FAITH_INDEX"),
            faith_index_yoy: fnum(item, "FAITH_INDEX_SAME"),
            faith_index_mom: fnum(item, "FAITH_INDEX_SEQUENTIAL"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// macro_china_national_tax_receipts
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChinaNationalTax {
    /// Report period (quarter), e.g. `2024-03`.
    pub date: String,
    /// Total tax revenue (亿元, akshare `TAX_INCOME`).
    pub tax_income: Option<f64>,
    /// Total tax revenue — year-over-year (akshare `TAX_INCOME_SAME`).
    pub tax_income_yoy: Option<f64>,
    /// Total tax revenue — quarter-over-quarter (akshare `TAX_INCOME_SEQUENTIAL`).
    pub tax_income_mom: Option<f64>,
    pub source: &'static str,
}

/// National tax revenue (`macro_china_national_tax_receipts`, Eastmoney `RPT_ECONOMY_TAX`).
pub async fn macro_china_national_tax_receipts(client: &Client) -> Result<Vec<ChinaNationalTax>> {
    const COLUMNS: &str = "REPORT_DATE,TIME,TAX_INCOME,TAX_INCOME_SAME,TAX_INCOME_SEQUENTIAL";
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "macro_china_national_tax_receipts",
            BASE,
            &dc_params("RPT_ECONOMY_TAX", COLUMNS),
        )
        .await?;
    parse_china_national_tax_receipts(&v)
}

pub(crate) fn parse_china_national_tax_receipts(resp: &Value) -> Result<Vec<ChinaNationalTax>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(date) = fstr(item, "TIME") else {
            continue;
        };
        out.push(ChinaNationalTax {
            date,
            tax_income: fnum(item, "TAX_INCOME"),
            tax_income_yoy: fnum(item, "TAX_INCOME_SAME"),
            tax_income_mom: fnum(item, "TAX_INCOME_SEQUENTIAL"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// macro_china_qyspjg (企业商品价格指数)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChinaGoodsPriceIndex {
    /// Report period (month), e.g. `2024-03`.
    pub date: String,
    /// Total index — index value (akshare `BASE`).
    pub total_index: Option<f64>,
    /// Total index — year-over-year (akshare `BASE_SAME`).
    pub total_index_yoy: Option<f64>,
    /// Total index — month-over-month (akshare `BASE_SEQUENTIAL`).
    pub total_index_mom: Option<f64>,
    /// Farm products index — index value (akshare `FARM_BASE`).
    pub farm_index: Option<f64>,
    /// Farm products index — year-over-year (akshare `FARM_BASE_SAME`).
    pub farm_yoy: Option<f64>,
    /// Farm products index — month-over-month (akshare `FARM_BASE_SEQUENTIAL`).
    pub farm_mom: Option<f64>,
    /// Mineral products index — index value (akshare `MINERAL_BASE`).
    pub mineral_index: Option<f64>,
    /// Mineral products index — year-over-year (akshare `MINERAL_BASE_SAME`).
    pub mineral_yoy: Option<f64>,
    /// Mineral products index — month-over-month (akshare `MINERAL_BASE_SEQUENTIAL`).
    pub mineral_mom: Option<f64>,
    /// Energy products index — index value (akshare `ENERGY_BASE`).
    pub energy_index: Option<f64>,
    /// Energy products index — year-over-year (akshare `ENERGY_BASE_SAME`).
    pub energy_yoy: Option<f64>,
    /// Energy products index — month-over-month (akshare `ENERGY_BASE_SEQUENTIAL`).
    pub energy_mom: Option<f64>,
    pub source: &'static str,
}

/// Corporate goods price index (`macro_china_qyspjg`, Eastmoney `RPT_ECONOMY_GOODS_INDEX`).
pub async fn macro_china_qyspjg(client: &Client) -> Result<Vec<ChinaGoodsPriceIndex>> {
    const COLUMNS: &str = "REPORT_DATE,TIME,BASE,BASE_SAME,BASE_SEQUENTIAL,FARM_BASE,\
        FARM_BASE_SAME,FARM_BASE_SEQUENTIAL,MINERAL_BASE,MINERAL_BASE_SAME,\
        MINERAL_BASE_SEQUENTIAL,ENERGY_BASE,ENERGY_BASE_SAME,ENERGY_BASE_SEQUENTIAL";
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "macro_china_qyspjg",
            BASE,
            &dc_params("RPT_ECONOMY_GOODS_INDEX", COLUMNS),
        )
        .await?;
    parse_china_qyspjg(&v)
}

pub(crate) fn parse_china_qyspjg(resp: &Value) -> Result<Vec<ChinaGoodsPriceIndex>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(date) = fstr(item, "TIME") else {
            continue;
        };
        out.push(ChinaGoodsPriceIndex {
            date,
            total_index: fnum(item, "BASE"),
            total_index_yoy: fnum(item, "BASE_SAME"),
            total_index_mom: fnum(item, "BASE_SEQUENTIAL"),
            farm_index: fnum(item, "FARM_BASE"),
            farm_yoy: fnum(item, "FARM_BASE_SAME"),
            farm_mom: fnum(item, "FARM_BASE_SEQUENTIAL"),
            mineral_index: fnum(item, "MINERAL_BASE"),
            mineral_yoy: fnum(item, "MINERAL_BASE_SAME"),
            mineral_mom: fnum(item, "MINERAL_BASE_SEQUENTIAL"),
            energy_index: fnum(item, "ENERGY_BASE"),
            energy_yoy: fnum(item, "ENERGY_BASE_SAME"),
            energy_mom: fnum(item, "ENERGY_BASE_SEQUENTIAL"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// macro_china_fdi (外商直接投资)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChinaFdi {
    /// Report period (month), e.g. `2024-03`.
    pub date: String,
    /// Actual foreign capital — current month (亿美元, akshare `ACTUAL_FOREIGN`).
    pub actual_foreign: Option<f64>,
    /// Actual foreign capital — year-over-year (akshare `ACTUAL_FOREIGN_SAME`).
    pub actual_foreign_yoy: Option<f64>,
    /// Actual foreign capital — month-over-month (akshare `ACTUAL_FOREIGN_SEQUENTIAL`).
    pub actual_foreign_mom: Option<f64>,
    /// Actual foreign capital — accumulated (akshare `ACTUAL_FOREIGN_ACCUMULATE`).
    pub actual_foreign_accumulate: Option<f64>,
    /// Accumulated — year-over-year (akshare `FOREIGN_ACCUMULATE_SAME`).
    pub accumulate_yoy: Option<f64>,
    pub source: &'static str,
}

/// Foreign direct investment (`macro_china_fdi`, Eastmoney `RPT_ECONOMY_FDI`).
pub async fn macro_china_fdi(client: &Client) -> Result<Vec<ChinaFdi>> {
    const COLUMNS: &str = "REPORT_DATE,TIME,ACTUAL_FOREIGN,ACTUAL_FOREIGN_SAME,\
        ACTUAL_FOREIGN_SEQUENTIAL,ACTUAL_FOREIGN_ACCUMULATE,FOREIGN_ACCUMULATE_SAME";
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "macro_china_fdi",
            BASE,
            &dc_params("RPT_ECONOMY_FDI", COLUMNS),
        )
        .await?;
    parse_china_fdi(&v)
}

pub(crate) fn parse_china_fdi(resp: &Value) -> Result<Vec<ChinaFdi>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(date) = fstr(item, "TIME") else {
            continue;
        };
        out.push(ChinaFdi {
            date,
            actual_foreign: fnum(item, "ACTUAL_FOREIGN"),
            actual_foreign_yoy: fnum(item, "ACTUAL_FOREIGN_SAME"),
            actual_foreign_mom: fnum(item, "ACTUAL_FOREIGN_SEQUENTIAL"),
            actual_foreign_accumulate: fnum(item, "ACTUAL_FOREIGN_ACCUMULATE"),
            accumulate_yoy: fnum(item, "FOREIGN_ACCUMULATE_SAME"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

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
    fn parses_macro_china_new_house_price() {
        let rows = parse_china_new_house_price(&fixture("macro_china_new_house_price.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-03-31");
        assert_eq!(rows[0].city, "北京");
        assert_eq!(rows[0].new_house_yoy, Some(-0.7));
        assert_eq!(rows[0].new_house_mom, Some(0.0));
        assert_eq!(rows[0].new_house_base, Some(105.3));
        assert_eq!(rows[0].second_hand_yoy, Some(-6.4));
        assert_eq!(rows[1].city, "上海");
        assert_eq!(rows[1].second_hand_base, Some(105.1));
    }

    #[test]
    fn parses_macro_china_lpr() {
        let rows = parse_china_lpr(&fixture("macro_china_lpr.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-03-20");
        assert_eq!(rows[0].lpr_1y, Some(3.45));
        assert_eq!(rows[0].lpr_5y, Some(3.95));
        assert_eq!(rows[0].rate_1, Some(3.45));
        assert_eq!(rows[1].lpr_5y, Some(4.2));
    }

    #[test]
    fn parses_macro_china_enterprise_boom_index() {
        let rows = parse_china_enterprise_boom_index(&fixture("macro_china_enterprise_boom_index.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-03");
        assert_eq!(rows[0].boom_index, Some(120.3));
        assert_eq!(rows[0].boom_index_yoy, Some(1.9));
        assert_eq!(rows[0].faith_index, Some(115.8));
        assert_eq!(rows[1].faith_index_mom, Some(-1.0));
    }

    #[test]
    fn parses_macro_china_national_tax_receipts() {
        let rows = parse_china_national_tax_receipts(&fixture("macro_china_national_tax_receipts.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-03");
        assert_eq!(rows[0].tax_income, Some(53823.0));
        assert_eq!(rows[0].tax_income_yoy, Some(-4.0));
        assert_eq!(rows[0].tax_income_mom, Some(0.0));
        assert_eq!(rows[1].tax_income, Some(51799.0));
    }

    #[test]
    fn parses_macro_china_qyspjg() {
        let rows = parse_china_qyspjg(&fixture("macro_china_qyspjg.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-03");
        assert_eq!(rows[0].total_index, Some(100.2));
        assert_eq!(rows[0].total_index_yoy, Some(-1.5));
        assert_eq!(rows[0].farm_index, Some(101.1));
        assert_eq!(rows[0].energy_mom, Some(-0.8));
        assert_eq!(rows[1].mineral_yoy, Some(1.9));
    }

    #[test]
    fn parses_macro_china_fdi() {
        let rows = parse_china_fdi(&fixture("macro_china_fdi.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-03");
        assert_eq!(rows[0].actual_foreign, Some(120.0));
        assert_eq!(rows[0].actual_foreign_yoy, Some(-15.0));
        assert_eq!(rows[0].actual_foreign_accumulate, Some(360.0));
        assert_eq!(rows[0].accumulate_yoy, Some(-10.0));
        assert_eq!(rows[1].actual_foreign_mom, Some(5.0));
    }
}
