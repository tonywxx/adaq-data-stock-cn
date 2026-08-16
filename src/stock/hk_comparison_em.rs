//! 东方财富-港股-行业对比 (Eastmoney HK industry comparison).
//!
//! Ports three functions from `akshare/stock/stock_hk_comparison_em.py`:
//!
//! | Rust fn | akshare fn | source |
//! |---|---|---|
//! | `stock_hk_growth_comparison_em` | `stock_hk_growth_comparison_em` | `akshare/stock/stock_hk_comparison_em.py:13` |
//! | `stock_hk_valuation_comparison_em` | `stock_hk_valuation_comparison_em` | `akshare/stock/stock_hk_comparison_em.py:61` |
//! | `stock_hk_scale_comparison_em` | `stock_hk_scale_comparison_em` | `akshare/stock/stock_hk_comparison_em.py:118` |
//!
//! All hit `datacenter.eastmoney.com/securities/api/data/v1/get` and read
//! `result.data` (a `null` result yields an empty table).
//!
//! ## DEFERRED
//! None.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

const URL: &str = "https://datacenter.eastmoney.com/securities/api/data/v1/get";

fn str_of(v: Option<&Value>) -> Option<String> {
    match v {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    }
}

fn num_of(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => {
            let t = s.trim().replace(',', "");
            if t.is_empty() {
                None
            } else {
                t.parse().ok()
            }
        }
        _ => None,
    }
}

fn em_data_array(resp: &Value) -> Result<Vec<Value>> {
    match resp.get("result") {
        Some(Value::Null) | None => Ok(Vec::new()),
        Some(result) => match result.get("data") {
            Some(Value::Null) | None => Ok(Vec::new()),
            Some(Value::Array(a)) => Ok(a.clone()),
            _ => Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "result.data not array".into(),
            }),
        },
    }
}

// ---------------------------------------------------------------------------
// 成长性对比 (growth)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct HkGrowthComparisonRow {
    #[serde(rename = "代码")]
    pub code: Option<String>,
    #[serde(rename = "简称")]
    pub name: Option<String>,
    #[serde(rename = "基本每股收益同比增长率")]
    pub eps_yoy: Option<f64>,
    #[serde(rename = "基本每股收益同比增长率排名")]
    pub eps_yoy_rank: Option<f64>,
    #[serde(rename = "营业收入同比增长率")]
    pub operate_income_yoy: Option<f64>,
    #[serde(rename = "营业收入同比增长率排名")]
    pub opincoming_yoy_rank: Option<f64>,
    #[serde(rename = "营业利润率同比增长率")]
    pub operate_profit_yoy: Option<f64>,
    #[serde(rename = "营业利润率同比增长率排名")]
    pub oprofit_yoy_rank: Option<f64>,
    #[serde(rename = "总资产同比增长率")]
    pub total_asset_yoy: Option<f64>,
    #[serde(rename = "总资产同比增长率排名")]
    pub toasset_yoy_rank: Option<f64>,
}

pub(crate) fn parse_hk_growth_comparison(arr: &[Value]) -> Vec<HkGrowthComparisonRow> {
    arr.iter()
        .map(|o| HkGrowthComparisonRow {
            code: str_of(o.get("CORRE_SECURITY_CODE")),
            name: str_of(o.get("CORRE_SECURITY_NAME")),
            eps_yoy: num_of(o.get("EPS_YOY")),
            eps_yoy_rank: num_of(o.get("EPS_YOY_RANK")),
            operate_income_yoy: num_of(o.get("OPERATE_INCOME_YOY")),
            opincoming_yoy_rank: num_of(o.get("OPINCOME_YOY_RANK")),
            operate_profit_yoy: num_of(o.get("OPERATE_PROFIT_YOY")),
            oprofit_yoy_rank: num_of(o.get("OPROFIT_YOY_RANK")),
            total_asset_yoy: num_of(o.get("TOTAL_ASSET_YOY")),
            toasset_yoy_rank: num_of(o.get("TOASSET_YOY_RANK")),
        })
        .collect()
}

/// Port of `stock_hk_growth_comparison_em(symbol)`.
pub async fn stock_hk_growth_comparison_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<HkGrowthComparisonRow>> {
    let filter = format!("(SECUCODE=\"{symbol}.HK\")(CORRE_SECUCODE=\"{symbol}.HK\")");
    let params = [
        ("reportName", "RPT_PCF10_INDUSTRY_HKGROWTH"),
        ("columns", "SECUCODE,SECURITY_CODE,ORG_CODE,REPORT_DATE,TYPE_ID,TYPE_TYPE,TYPE_NAME,TYPE_NAME_EN,CORRE_SECURITY_CODE,CORRE_SECUCODE,CORRE_SECURITY_NAME,EPS_YOY,OPERATE_INCOME_YOY,OPERATE_PROFIT_YOY,TOTAL_ASSET_YOY,EPS_YOY_RANK,OPINCOME_YOY_RANK,OPROFIT_YOY_RANK,TOASSET_YOY_RANK"),
        ("quoteColumns", ""),
        ("filter", filter.as_str()),
        ("pageNumber", "1"),
        ("pageSize", ""),
        ("sortTypes", ""),
        ("sortColumns", ""),
        ("source", "F10"),
        ("client", "PC"),
        ("v", "03313416193688571"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_hk_growth_comparison_em",
            URL,
            &params,
        )
        .await?;
    let arr = em_data_array(&v)?;
    Ok(parse_hk_growth_comparison(&arr))
}

// ---------------------------------------------------------------------------
// 估值对比 (valuation)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct HkValuationComparisonRow {
    #[serde(rename = "代码")]
    pub code: Option<String>,
    #[serde(rename = "简称")]
    pub name: Option<String>,
    #[serde(rename = "市盈率-TTM")]
    pub pe_ttm: Option<f64>,
    #[serde(rename = "市盈率-TTM排名")]
    pub pe_ttm_rank: Option<f64>,
    #[serde(rename = "市盈率-LYR")]
    pub pe_lyr: Option<f64>,
    #[serde(rename = "市盈率-LYR排名")]
    pub pe_lyr_rank: Option<f64>,
    #[serde(rename = "市净率-MRQ")]
    pub pb_mqr: Option<f64>,
    #[serde(rename = "市净率-MRQ排名")]
    pub pb_mqr_rank: Option<f64>,
    #[serde(rename = "市净率-LYR")]
    pub pb_lyr: Option<f64>,
    #[serde(rename = "市净率-LYR排名")]
    pub pb_lyr_rank: Option<f64>,
    #[serde(rename = "市销率-TTM")]
    pub ps_ttm: Option<f64>,
    #[serde(rename = "市销率-TTM排名")]
    pub ps_ttm_rank: Option<f64>,
    #[serde(rename = "市销率-LYR")]
    pub ps_lyr: Option<f64>,
    #[serde(rename = "市销率-LYR排名")]
    pub ps_lyr_rank: Option<f64>,
    #[serde(rename = "市现率-TTM")]
    pub pce_ttm: Option<f64>,
    #[serde(rename = "市现率-TTM排名")]
    pub pce_ttm_rank: Option<f64>,
    #[serde(rename = "市现率-LYR")]
    pub pce_lyr: Option<f64>,
    #[serde(rename = "市现率-LYR排名")]
    pub pce_lyr_rank: Option<f64>,
}

pub(crate) fn parse_hk_valuation_comparison(arr: &[Value]) -> Vec<HkValuationComparisonRow> {
    arr.iter()
        .map(|o| HkValuationComparisonRow {
            code: str_of(o.get("CORRE_SECURITY_CODE")),
            name: str_of(o.get("CORRE_SECURITY_NAME")),
            pe_ttm: num_of(o.get("PE_TTM")),
            pe_ttm_rank: num_of(o.get("PE_TTM_RANK")),
            pe_lyr: num_of(o.get("PE_LYR")),
            pe_lyr_rank: num_of(o.get("PE_LYR_RANK")),
            pb_mqr: num_of(o.get("PB_MQR")),
            pb_mqr_rank: num_of(o.get("PB_MQR_RANK")),
            pb_lyr: num_of(o.get("PB_LYR")),
            pb_lyr_rank: num_of(o.get("PB_LYR_RANK")),
            ps_ttm: num_of(o.get("PS_TTM")),
            ps_ttm_rank: num_of(o.get("PS_TTM_RANK")),
            ps_lyr: num_of(o.get("PS_LYR")),
            ps_lyr_rank: num_of(o.get("PS_LYR_RANK")),
            pce_ttm: num_of(o.get("PCE_TTM")),
            pce_ttm_rank: num_of(o.get("PCE_TTM_RANK")),
            pce_lyr: num_of(o.get("PCE_LYR")),
            pce_lyr_rank: num_of(o.get("PCE_LYR_RANK")),
        })
        .collect()
}

/// Port of `stock_hk_valuation_comparison_em(symbol)`.
pub async fn stock_hk_valuation_comparison_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<HkValuationComparisonRow>> {
    let filter = format!("(SECUCODE=\"{symbol}.HK\")(CORRE_SECUCODE=\"{symbol}.HK\")");
    let params = [
        ("reportName", "RPT_PCF10_INDUSTRY_HKCVALUE"),
        ("columns", "SECUCODE,SECURITY_CODE,ORG_CODE,REPORT_DATE,TYPE_ID,TYPE_TYPE,TYPE_NAME,TYPE_NAME_EN,CORRE_SECURITY_CODE,CORRE_SECUCODE,CORRE_SECURITY_NAME,PE_TTM,PE_LYR,PB_MQR,PB_LYR,PS_TTM,PS_LYR,PCE_TTM,PCE_LYR,PE_TTM_RANK,PE_LYR_RANK,PB_MQR_RANK,PB_LYR_RANK,PS_TTM_RANK,PS_LYR_RANK,PCE_TTM_RANK,PCE_LYR_RANK"),
        ("quoteColumns", ""),
        ("filter", filter.as_str()),
        ("pageNumber", "1"),
        ("pageSize", ""),
        ("sortTypes", ""),
        ("sortColumns", ""),
        ("source", "F10"),
        ("client", "PC"),
        ("v", "03445297742754925"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_hk_valuation_comparison_em",
            URL,
            &params,
        )
        .await?;
    let arr = em_data_array(&v)?;
    Ok(parse_hk_valuation_comparison(&arr))
}

// ---------------------------------------------------------------------------
// 规模对比 (scale)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct HkScaleComparisonRow {
    #[serde(rename = "代码")]
    pub code: Option<String>,
    #[serde(rename = "简称")]
    pub name: Option<String>,
    #[serde(rename = "总市值")]
    pub hksdqmv: Option<f64>,
    #[serde(rename = "总市值排名")]
    pub hksdqmv_rank: Option<f64>,
    #[serde(rename = "流通市值")]
    pub hktotal_market_cap: Option<f64>,
    #[serde(rename = "流通市值排名")]
    pub hktotal_cap_rank: Option<f64>,
    #[serde(rename = "营业总收入")]
    pub operate_income: Option<f64>,
    #[serde(rename = "营业总收入排名")]
    pub operate_income_rank: Option<f64>,
    #[serde(rename = "净利润")]
    pub gross_profit: Option<f64>,
    #[serde(rename = "净利润排名")]
    pub gross_profit_rank: Option<f64>,
}

pub(crate) fn parse_hk_scale_comparison(arr: &[Value]) -> Vec<HkScaleComparisonRow> {
    arr.iter()
        .map(|o| HkScaleComparisonRow {
            code: str_of(o.get("CORRE_SECURITY_CODE")),
            name: str_of(o.get("CORRE_SECURITY_NAME")),
            hksdqmv: num_of(o.get("HKSDQMV")),
            hksdqmv_rank: num_of(o.get("HKSDQMV_RANK")),
            hktotal_market_cap: num_of(o.get("HKTOTAL_MARKET_CAP")),
            hktotal_cap_rank: num_of(o.get("HKTOTAL_CAP_RANK")),
            operate_income: num_of(o.get("OPERATE_INCOME")),
            operate_income_rank: num_of(o.get("OPERATE_INCOME_RANK")),
            gross_profit: num_of(o.get("GROSS_PROFIT")),
            gross_profit_rank: num_of(o.get("GROSS_PROFIT_RANK")),
        })
        .collect()
}

/// Port of `stock_hk_scale_comparison_em(symbol)`.
pub async fn stock_hk_scale_comparison_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<HkScaleComparisonRow>> {
    let filter = format!("(SECUCODE=\"{symbol}.HK\")(CORRE_SECUCODE=\"{symbol}.HK\")");
    let params = [
        ("reportName", "RPT_PCF10_INDUSTRY_SCALE"),
        ("columns", "SECURITY_CODE,SECUCODE,TYPE_ID,TYPE_TYPE,TYPE_NAME,TYPE_NAME_EN,CORRE_SECURITY_CODE,CORRE_SECUCODE,CORRE_SECURITY_NAME,MAXSTDREPORTDATE,HKSDQMV,HKTOTAL_MARKET_CAP,OPERATE_INCOME,GROSS_PROFIT,HKSDQMV_RANK,HKTOTAL_CAP_RANK,OPERATE_INCOME_RANK,GROSS_PROFIT_RANK"),
        ("quoteColumns", ""),
        ("filter", filter.as_str()),
        ("pageNumber", "1"),
        ("pageSize", ""),
        ("sortTypes", ""),
        ("sortColumns", ""),
        ("source", "F10"),
        ("client", "PC"),
        ("v", "07839693368708753"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_hk_scale_comparison_em",
            URL,
            &params,
        )
        .await?;
    let arr = em_data_array(&v)?;
    Ok(parse_hk_scale_comparison(&arr))
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

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    #[test]
    fn parses_hk_growth() {
        let arr = em_data_array(&fixture("stock_hk_growth_comparison_em.json")).unwrap();
        let rows = parse_hk_growth_comparison(&arr);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code.as_deref(), Some("03900.HK"));
        assert!(approx(rows[0].eps_yoy, 8.5));
        assert!(approx(rows[1].operate_income_yoy, 5.5));
    }

    #[test]
    fn parses_hk_valuation() {
        let arr = em_data_array(&fixture("stock_hk_valuation_comparison_em.json")).unwrap();
        let rows = parse_hk_valuation_comparison(&arr);
        assert_eq!(rows.len(), 2);
        assert!(approx(rows[0].pe_ttm, 9.8));
        assert!(approx(rows[1].pb_lyr_rank, 11.0));
    }

    #[test]
    fn parses_hk_scale() {
        let arr = em_data_array(&fixture("stock_hk_scale_comparison_em.json")).unwrap();
        let rows = parse_hk_scale_comparison(&arr);
        assert_eq!(rows.len(), 2);
        assert!(approx(rows[0].hksdqmv, 350_000_000_000.0));
        assert!(approx(rows[1].gross_profit_rank, 2.0));
    }

    #[test]
    fn parses_hk_growth_empty() {
        let arr = em_data_array(&fixture("stock_hk_growth_comparison_em_empty.json")).unwrap();
        assert!(arr.is_empty());
    }
}
