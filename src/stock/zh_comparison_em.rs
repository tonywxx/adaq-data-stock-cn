//! 东方财富-同行比较-成长性 / 杜邦分析 (Eastmoney A-share peer comparison).
//!
//! Ports two functions from `akshare/stock/stock_zh_comparison_em.py`:
//!
//! | Rust fn | akshare fn | source |
//! |---|---|---|
//! | `stock_zh_growth_comparison_em` | `stock_zh_growth_comparison_em` | `akshare/stock/stock_zh_comparison_em.py:13` |
//! | `stock_zh_dupont_comparison_em` | `stock_zh_dupont_comparison_em` | `akshare/stock/stock_zh_comparison_em.py:162` |
//!
//! Both hit `datacenter.eastmoney.com/securities/api/data/v1/get` and read
//! `result.data` (a `null` result yields an empty table). `symbol` is an
//! `SZ`/`SH`-prefixed code (e.g. `SZ000895`), matching akshare.
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
// 成长性比较 (growth)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ZhGrowthComparisonRow {
    #[serde(rename = "代码")]
    pub code: Option<String>,
    #[serde(rename = "简称")]
    pub name: Option<String>,
    #[serde(rename = "基本每股收益增长率-3年复合")]
    pub mgsy_3y: Option<f64>,
    #[serde(rename = "基本每股收益增长率-24A")]
    pub mgsytb: Option<f64>,
    #[serde(rename = "基本每股收益增长率-TTM")]
    pub mgsyttm: Option<f64>,
    #[serde(rename = "基本每股收益增长率-25E")]
    pub mgsy_1e: Option<f64>,
    #[serde(rename = "基本每股收益增长率-26E")]
    pub mgsy_2e: Option<f64>,
    #[serde(rename = "基本每股收益增长率-27E")]
    pub mgsy_3e: Option<f64>,
    #[serde(rename = "营业收入增长率-3年复合")]
    pub yysr_3y: Option<f64>,
    #[serde(rename = "营业收入增长率-24A")]
    pub yysrtb: Option<f64>,
    #[serde(rename = "营业收入增长率-TTM")]
    pub yysrttm: Option<f64>,
    #[serde(rename = "营业收入增长率-25E")]
    pub yysr_1e: Option<f64>,
    #[serde(rename = "营业收入增长率-26E")]
    pub yysr_2e: Option<f64>,
    #[serde(rename = "营业收入增长率-27E")]
    pub yysr_3e: Option<f64>,
    #[serde(rename = "净利润增长率-3年复合")]
    pub jlr_3y: Option<f64>,
    #[serde(rename = "净利润增长率-24A")]
    pub jlrtb: Option<f64>,
    #[serde(rename = "净利润增长率-TTM")]
    pub jlrttm: Option<f64>,
    #[serde(rename = "净利润增长率-25E")]
    pub jlr_1e: Option<f64>,
    #[serde(rename = "净利润增长率-26E")]
    pub jlr_2e: Option<f64>,
    #[serde(rename = "净利润增长率-27E")]
    pub jlr_3e: Option<f64>,
    #[serde(rename = "基本每股收益增长率-3年复合排名")]
    pub paiming: Option<f64>,
}

pub(crate) fn parse_zh_growth_comparison(arr: &[Value]) -> Vec<ZhGrowthComparisonRow> {
    arr.iter()
        .map(|o| ZhGrowthComparisonRow {
            code: str_of(o.get("CORRE_SECURITY_CODE")),
            name: str_of(o.get("CORRE_SECURITY_NAME")),
            mgsy_3y: num_of(o.get("MGSY_3Y")),
            mgsytb: num_of(o.get("MGSYTB")),
            mgsyttm: num_of(o.get("MGSYTTM")),
            mgsy_1e: num_of(o.get("MGSY_1E")),
            mgsy_2e: num_of(o.get("MGSY_2E")),
            mgsy_3e: num_of(o.get("MGSY_3E")),
            yysr_3y: num_of(o.get("YYSR_3Y")),
            yysrtb: num_of(o.get("YYSRTB")),
            yysrttm: num_of(o.get("YYSRTTM")),
            yysr_1e: num_of(o.get("YYSR_1E")),
            yysr_2e: num_of(o.get("YYSR_2E")),
            yysr_3e: num_of(o.get("YYSR_3E")),
            jlr_3y: num_of(o.get("JLR_3Y")),
            jlrtb: num_of(o.get("JLRTB")),
            jlrttm: num_of(o.get("JLRTTM")),
            jlr_1e: num_of(o.get("JLR_1E")),
            jlr_2e: num_of(o.get("JLR_2E")),
            jlr_3e: num_of(o.get("JLR_3E")),
            paiming: num_of(o.get("PAIMING")),
        })
        .collect()
}

/// Port of `stock_zh_growth_comparison_em(symbol)`.
pub async fn stock_zh_growth_comparison_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<ZhGrowthComparisonRow>> {
    let filter = format!("(SECUCODE=\"{}{}\")", &symbol[2..], &symbol[..2]);
    let params = [
        ("reportName", "RPT_PCF10_INDUSTRY_GROWTH"),
        ("columns", "ALL"),
        ("quoteColumns", ""),
        ("filter", filter.as_str()),
        ("pageNumber", ""),
        ("pageSize", ""),
        ("sortTypes", "1"),
        ("sortColumns", "PAIMING"),
        ("source", "HSF10"),
        ("client", "PC"),
        ("v", "02747607708067783"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_zh_growth_comparison_em",
            URL,
            &params,
        )
        .await?;
    let arr = em_data_array(&v)?;
    Ok(parse_zh_growth_comparison(&arr))
}

// ---------------------------------------------------------------------------
// 杜邦分析比较 (dupont)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ZhDupontComparisonRow {
    #[serde(rename = "代码")]
    pub code: Option<String>,
    #[serde(rename = "简称")]
    pub name: Option<String>,
    #[serde(rename = "ROE-3年平均")]
    pub roe_avg: Option<f64>,
    #[serde(rename = "ROE-22A")]
    pub roepj_l3: Option<f64>,
    #[serde(rename = "ROE-23A")]
    pub roepj_l2: Option<f64>,
    #[serde(rename = "ROE-24A")]
    pub roepj_l1: Option<f64>,
    #[serde(rename = "净利率-3年平均")]
    pub xsjll_avg: Option<f64>,
    #[serde(rename = "净利率-22A")]
    pub xsjll_l3: Option<f64>,
    #[serde(rename = "净利率-23A")]
    pub xsjll_l2: Option<f64>,
    #[serde(rename = "净利率-24A")]
    pub xsjll_l1: Option<f64>,
    #[serde(rename = "总资产周转率-3年平均")]
    pub toazzl_avg: Option<f64>,
    #[serde(rename = "总资产周转率-22A")]
    pub toazzl_l3: Option<f64>,
    #[serde(rename = "总资产周转率-23A")]
    pub toazzl_l2: Option<f64>,
    #[serde(rename = "总资产周转率-24A")]
    pub toazzl_l1: Option<f64>,
    #[serde(rename = "权益乘数-3年平均")]
    pub qycs_avg: Option<f64>,
    #[serde(rename = "权益乘数-22A")]
    pub qycs_l3: Option<f64>,
    #[serde(rename = "权益乘数-23A")]
    pub qycs_l2: Option<f64>,
    #[serde(rename = "权益乘数-24A")]
    pub qycs_l1: Option<f64>,
    #[serde(rename = "ROE-3年平均排名")]
    pub paiming: Option<f64>,
}

pub(crate) fn parse_zh_dupont_comparison(arr: &[Value]) -> Vec<ZhDupontComparisonRow> {
    arr.iter()
        .map(|o| ZhDupontComparisonRow {
            code: str_of(o.get("CORRE_SECURITY_CODE")),
            name: str_of(o.get("CORRE_SECURITY_NAME")),
            roe_avg: num_of(o.get("ROE_AVG")),
            roepj_l3: num_of(o.get("ROEPJ_L3")),
            roepj_l2: num_of(o.get("ROEPJ_L2")),
            roepj_l1: num_of(o.get("ROEPJ_L1")),
            xsjll_avg: num_of(o.get("XSJLL_AVG")),
            xsjll_l3: num_of(o.get("XSJLL_L3")),
            xsjll_l2: num_of(o.get("XSJLL_L2")),
            xsjll_l1: num_of(o.get("XSJLL_L1")),
            toazzl_avg: num_of(o.get("TOAZZL_AVG")),
            toazzl_l3: num_of(o.get("TOAZZL_L3")),
            toazzl_l2: num_of(o.get("TOAZZL_L2")),
            toazzl_l1: num_of(o.get("TOAZZL_L1")),
            qycs_avg: num_of(o.get("QYCS_AVG")),
            qycs_l3: num_of(o.get("QYCS_L3")),
            qycs_l2: num_of(o.get("QYCS_L2")),
            qycs_l1: num_of(o.get("QYCS_L1")),
            paiming: num_of(o.get("PAIMING")),
        })
        .collect()
}

/// Port of `stock_zh_dupont_comparison_em(symbol)`.
pub async fn stock_zh_dupont_comparison_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<ZhDupontComparisonRow>> {
    let filter = format!("(SECUCODE=\"{}{}\")", &symbol[2..], &symbol[..2]);
    let params = [
        ("reportName", "RPT_PCF10_INDUSTRY_DBFX"),
        ("columns", "ALL"),
        ("quoteColumns", ""),
        ("filter", filter.as_str()),
        ("pageNumber", ""),
        ("pageSize", ""),
        ("sortTypes", "1"),
        ("sortColumns", "PAIMING"),
        ("source", "HSF10"),
        ("client", "PC"),
        ("v", "05086361194054821"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_zh_dupont_comparison_em",
            URL,
            &params,
        )
        .await?;
    let arr = em_data_array(&v)?;
    Ok(parse_zh_dupont_comparison(&arr))
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
    fn parses_zh_growth() {
        let arr = em_data_array(&fixture("stock_zh_growth_comparison_em.json")).unwrap();
        let rows = parse_zh_growth_comparison(&arr);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code.as_deref(), Some("000895.SZ"));
        assert!(approx(rows[0].mgsy_3y, 8.5));
        assert!(approx(rows[1].jlr_3e, 8.3));
    }

    #[test]
    fn parses_zh_dupont() {
        let arr = em_data_array(&fixture("stock_zh_dupont_comparison_em.json")).unwrap();
        let rows = parse_zh_dupont_comparison(&arr);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code.as_deref(), Some("000895.SZ"));
        assert!(approx(rows[0].roe_avg, 26.5));
        assert!(approx(rows[1].qycs_l1, 1.85));
    }
}
