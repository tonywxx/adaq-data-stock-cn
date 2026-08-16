//! 东方财富-港股-公司资料 / 最新指标 / 分红派息 (Eastmoney HK F10).
//!
//! Ports three functions from `akshare/stock/stock_profile_em.py`:
//!
//! | Rust fn | akshare fn | source |
//! |---|---|---|
//! | `stock_hk_company_profile_em` | `stock_hk_company_profile_em` | `akshare/stock/stock_profile_em.py:79` |
//! | `stock_hk_financial_indicator_em` | `stock_hk_financial_indicator_em` | `akshare/stock/stock_profile_em.py:153` |
//! | `stock_hk_dividend_payout_em` | `stock_hk_dividend_payout_em` | `akshare/stock/stock_profile_em.py:237` |
//!
//! All hit `datacenter.eastmoney.com/securities/api/data/v1/get` (reportName
//! based) and read `result.data`. `stock_hk_dividend_payout_em` tolerates a
//! `null` result (returns empty).
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

/// Read `result.data` as an array (tolerates a `null` result → empty).
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
// 公司资料 (company profile)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct HkCompanyProfileRow {
    #[serde(rename = "公司名称")]
    pub org_name: Option<String>,
    #[serde(rename = "英文名称")]
    pub org_en_abbr: Option<String>,
    #[serde(rename = "注册地")]
    pub reg_place: Option<String>,
    #[serde(rename = "注册地址")]
    pub reg_address: Option<String>,
    #[serde(rename = "公司成立日期")]
    pub found_date: Option<String>,
    #[serde(rename = "所属行业")]
    pub belong_industry: Option<String>,
    #[serde(rename = "董事长")]
    pub chairman: Option<String>,
    #[serde(rename = "公司秘书")]
    pub secretary: Option<String>,
    #[serde(rename = "员工人数")]
    pub emp_num: Option<f64>,
    #[serde(rename = "办公地址")]
    pub address: Option<String>,
    #[serde(rename = "公司网址")]
    pub org_web: Option<String>,
    #[serde(rename = "E-MAIL")]
    pub org_email: Option<String>,
    #[serde(rename = "年结日")]
    pub year_settle_day: Option<String>,
    #[serde(rename = "联系电话")]
    pub org_tel: Option<String>,
    #[serde(rename = "核数师")]
    pub account_firm: Option<String>,
    #[serde(rename = "传真")]
    pub org_fax: Option<String>,
    #[serde(rename = "公司介绍")]
    pub org_profile: Option<String>,
}

pub(crate) fn parse_hk_company_profile(arr: &[Value]) -> Vec<HkCompanyProfileRow> {
    arr.iter()
        .map(|o| HkCompanyProfileRow {
            org_name: str_of(o.get("ORG_NAME")),
            org_en_abbr: str_of(o.get("ORG_EN_ABBR")),
            reg_place: str_of(o.get("REG_PLACE")),
            reg_address: str_of(o.get("REG_ADDRESS")),
            found_date: str_of(o.get("FOUND_DATE")),
            belong_industry: str_of(o.get("BELONG_INDUSTRY")),
            chairman: str_of(o.get("CHAIRMAN")),
            secretary: str_of(o.get("SECRETARY")),
            emp_num: num_of(o.get("EMP_NUM")),
            address: str_of(o.get("ADDRESS")),
            org_web: str_of(o.get("ORG_WEB")),
            org_email: str_of(o.get("ORG_EMAIL")),
            year_settle_day: str_of(o.get("YEAR_SETTLE_DAY")),
            org_tel: str_of(o.get("ORG_TEL")),
            account_firm: str_of(o.get("ACCOUNT_FIRM")),
            org_fax: str_of(o.get("ORG_FAX")),
            org_profile: str_of(o.get("ORG_PROFILE")),
        })
        .collect()
}

/// Port of `stock_hk_company_profile_em(symbol)`.
pub async fn stock_hk_company_profile_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<HkCompanyProfileRow>> {
    let filter = format!("(SECUCODE=\"{symbol}.HK\")");
    let params = [
        ("reportName", "RPT_HKF10_INFO_ORGPROFILE"),
        ("columns", "SECUCODE,SECURITY_CODE,ORG_NAME,ORG_EN_ABBR,BELONG_INDUSTRY,FOUND_DATE,CHAIRMAN,SECRETARY,ACCOUNT_FIRM,REG_ADDRESS,ADDRESS,YEAR_SETTLE_DAY,EMP_NUM,ORG_TEL,ORG_FAX,ORG_EMAIL,ORG_WEB,ORG_PROFILE,REG_PLACE"),
        ("quoteColumns", ""),
        ("filter", filter.as_str()),
        ("pageNumber", "1"),
        ("pageSize", "200"),
        ("sortTypes", ""),
        ("sortColumns", ""),
        ("source", "F10"),
        ("client", "PC"),
        ("v", "04748497219912483"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_hk_company_profile_em",
            URL,
            &params,
        )
        .await?;
    let arr = em_data_array(&v)?;
    Ok(parse_hk_company_profile(&arr))
}

// ---------------------------------------------------------------------------
// 最新指标 (financial indicator)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct HkFinancialIndicatorRow {
    #[serde(rename = "基本每股收益(元)")]
    pub basic_eps: Option<f64>,
    #[serde(rename = "每股净资产(元)")]
    pub bps: Option<f64>,
    #[serde(rename = "法定股本(股)")]
    pub common_acs: Option<f64>,
    #[serde(rename = "每手股")]
    pub per_shares: Option<f64>,
    #[serde(rename = "每股股息TTM(港元)")]
    pub dividend_ttm: Option<f64>,
    #[serde(rename = "派息比率(%)")]
    pub divi_ratio: Option<f64>,
    #[serde(rename = "已发行股本(股)")]
    pub issued_common_shares: Option<f64>,
    #[serde(rename = "已发行股本-H股(股)")]
    pub hk_common_shares: Option<f64>,
    #[serde(rename = "每股经营现金流(元)")]
    pub per_netcash_operate: Option<f64>,
    #[serde(rename = "股息率TTM(%)")]
    pub dividend_rate: Option<f64>,
    #[serde(rename = "总市值(港元)")]
    pub total_market_cap: Option<f64>,
    #[serde(rename = "港股市值(港元)")]
    pub hksk_market_cap: Option<f64>,
    #[serde(rename = "营业总收入")]
    pub operate_income: Option<f64>,
    #[serde(rename = "营业总收入滚动环比增长(%)")]
    pub operate_income_qoq: Option<f64>,
    #[serde(rename = "销售净利率(%)")]
    pub net_profit_ratio: Option<f64>,
    #[serde(rename = "净利润")]
    pub holder_profit: Option<f64>,
    #[serde(rename = "净利润滚动环比增长(%)")]
    pub holder_profit_qoq: Option<f64>,
    #[serde(rename = "股东权益回报率(%)")]
    pub roe_avg: Option<f64>,
    #[serde(rename = "市盈率")]
    pub pe_ttm: Option<f64>,
    #[serde(rename = "市净率")]
    pub pb_ttm: Option<f64>,
    #[serde(rename = "总资产回报率(%)")]
    pub roa: Option<f64>,
}

pub(crate) fn parse_hk_financial_indicator(arr: &[Value]) -> Vec<HkFinancialIndicatorRow> {
    arr.iter()
        .map(|o| HkFinancialIndicatorRow {
            basic_eps: num_of(o.get("BASIC_EPS")),
            bps: num_of(o.get("BPS")),
            common_acs: num_of(o.get("COMMON_ACS")),
            per_shares: num_of(o.get("PER_SHARES")),
            dividend_ttm: num_of(o.get("DIVIDEND_TTM")),
            divi_ratio: num_of(o.get("DIVI_RATIO")),
            issued_common_shares: num_of(o.get("ISSUED_COMMON_SHARES")),
            hk_common_shares: num_of(o.get("HK_COMMON_SHARES")),
            per_netcash_operate: num_of(o.get("PER_NETCASH_OPERATE")),
            dividend_rate: num_of(o.get("DIVIDEND_RATE")),
            total_market_cap: num_of(o.get("TOTAL_MARKET_CAP")),
            hksk_market_cap: num_of(o.get("HKSK_MARKET_CAP")),
            operate_income: num_of(o.get("OPERATE_INCOME")),
            operate_income_qoq: num_of(o.get("OPERATE_INCOME_QOQ")),
            net_profit_ratio: num_of(o.get("NET_PROFIT_RATIO")),
            holder_profit: num_of(o.get("HOLDER_PROFIT")),
            holder_profit_qoq: num_of(o.get("HOLDER_PROFIT_QOQ")),
            roe_avg: num_of(o.get("ROE_AVG")),
            pe_ttm: num_of(o.get("PE_TTM")),
            pb_ttm: num_of(o.get("PB_TTM")),
            roa: num_of(o.get("ROA")),
        })
        .collect()
}

/// Port of `stock_hk_financial_indicator_em(symbol)`.
pub async fn stock_hk_financial_indicator_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<HkFinancialIndicatorRow>> {
    let filter = format!("(SECUCODE=\"{symbol}.HK\")");
    let params = [
        ("reportName", "RPT_CUSTOM_HKF10_FN_MAININDICATORMAX"),
        ("columns", "ORG_CODE,SECUCODE,SECURITY_CODE,SECURITY_NAME_ABBR,SECURITY_INNER_CODE,REPORT_DATE,BASIC_EPS,PER_NETCASH_OPERATE,BPS,BPS_NEDILUTED,COMMON_ACS,PER_SHARES,ISSUED_COMMON_SHARES,HK_COMMON_SHARES,TOTAL_MARKET_CAP,HKSK_MARKET_CAP,OPERATE_INCOME,OPERATE_INCOME_SQ,OPERATE_INCOME_QOQ,OPERATE_INCOME_QOQ_SQ,HOLDER_PROFIT,HOLDER_PROFIT_SQ,HOLDER_PROFIT_QOQ,HOLDER_PROFIT_QOQ_SQ,PE_TTM,PE_TTM_SQ,PB_TTM,PB_TTM_SQ,NET_PROFIT_RATIO,NET_PROFIT_RATIO_SQ,ROE_AVG,ROE_AVG_SQ,ROA,ROA_SQ,DIVIDEND_TTM,DIVIDEND_LFY,DIVI_RATIO,DIVIDEND_RATE,IS_CNY_CODE"),
        ("quoteColumns", ""),
        ("filter", filter.as_str()),
        ("pageNumber", "1"),
        ("pageSize", "200"),
        ("sortTypes", "-1"),
        ("sortColumns", "REPORT_DATE"),
        ("source", "F10"),
        ("client", "PC"),
        ("v", "07945646099062258"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_hk_financial_indicator_em",
            URL,
            &params,
        )
        .await?;
    let arr = em_data_array(&v)?;
    Ok(parse_hk_financial_indicator(&arr))
}

// ---------------------------------------------------------------------------
// 分红派息 (dividend payout)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct HkDividendPayoutRow {
    #[serde(rename = "最新公告日期")]
    pub update_date: Option<String>,
    #[serde(rename = "财政年度")]
    pub year: Option<String>,
    #[serde(rename = "分红方案")]
    pub plan_explain: Option<String>,
    #[serde(rename = "分配类型")]
    pub report_type: Option<String>,
    #[serde(rename = "除净日")]
    pub ex_dividend_date: Option<String>,
    #[serde(rename = "截至过户日")]
    pub transfer_end_date: Option<String>,
    #[serde(rename = "发放日")]
    pub dividend_date: Option<String>,
}

pub(crate) fn parse_hk_dividend_payout(arr: &[Value]) -> Vec<HkDividendPayoutRow> {
    arr.iter()
        .map(|o| HkDividendPayoutRow {
            update_date: str_of(o.get("UPDATE_DATE")),
            year: str_of(o.get("YEAR")),
            plan_explain: str_of(o.get("PLAN_EXPLAIN")),
            report_type: str_of(o.get("REPORT_TYPE")),
            ex_dividend_date: str_of(o.get("EX_DIVIDEND_DATE")),
            transfer_end_date: str_of(o.get("TRANSFER_END_DATE")),
            dividend_date: str_of(o.get("DIVIDEND_DATE")),
        })
        .collect()
}

/// Port of `stock_hk_dividend_payout_em(symbol)`.
pub async fn stock_hk_dividend_payout_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<HkDividendPayoutRow>> {
    let filter = format!("(SECURITY_CODE=\"{symbol}\")(IS_BFP=\"0\")");
    let params = [
        ("reportName", "RPT_HKF10_MAIN_DIVBASIC"),
        ("columns", "SECURITY_CODE,UPDATE_DATE,REPORT_TYPE,EX_DIVIDEND_DATE,DIVIDEND_DATE,TRANSFER_END_DATE,YEAR,PLAN_EXPLAIN,IS_BFP"),
        ("quoteColumns", ""),
        ("filter", filter.as_str()),
        ("pageNumber", "1"),
        ("pageSize", "200"),
        ("sortTypes", "-1,-1"),
        ("sortColumns", "NOTICE_DATE,EX_DIVIDEND_DATE"),
        ("source", "F10"),
        ("client", "PC"),
        ("v", "035584639294227527"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_hk_dividend_payout_em",
            URL,
            &params,
        )
        .await?;
    let arr = em_data_array(&v)?;
    Ok(parse_hk_dividend_payout(&arr))
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
    fn parses_hk_company_profile() {
        let arr = em_data_array(&fixture("stock_hk_company_profile_em.json")).unwrap();
        let rows = parse_hk_company_profile(&arr);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].org_name.as_deref(), Some("Example Holdings Ltd"));
        assert_eq!(rows[0].emp_num, Some(12345.0));
    }

    #[test]
    fn parses_hk_financial_indicator() {
        let arr = em_data_array(&fixture("stock_hk_financial_indicator_em.json")).unwrap();
        let rows = parse_hk_financial_indicator(&arr);
        assert_eq!(rows.len(), 1);
        assert!(approx(rows[0].basic_eps, 1.23));
        assert!(approx(rows[0].pe_ttm, 9.8));
        assert!(approx(rows[0].pb_ttm, 0.85));
    }

    #[test]
    fn parses_hk_dividend_payout() {
        let arr = em_data_array(&fixture("stock_hk_dividend_payout_em.json")).unwrap();
        let rows = parse_hk_dividend_payout(&arr);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].plan_explain.as_deref(), Some("派息每股0.5港元"));
        assert_eq!(rows[1].report_type.as_deref(), Some("中期"));
    }

    #[test]
    fn parses_hk_dividend_payout_empty() {
        let arr = em_data_array(&fixture("stock_hk_dividend_payout_em_empty.json")).unwrap();
        assert!(arr.is_empty());
    }
}
