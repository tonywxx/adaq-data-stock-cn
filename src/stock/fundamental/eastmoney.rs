//! Eastmoney financial-statement endpoints (report-based, `datacenter-web` pattern).
//!
//! Ports akshare's Eastmoney financial functions. The upstream akshare
//! `stock_*_sheet_by_report_em` functions scrape an HTML index page to learn a
//! per-company `companyType`, which is a runtime HTML fetch (no JS signing, but
//! still a blocking dependency). We instead use Eastmoney's `datacenter-web`
//! REST API (`datacenter.eastmoney.com/securities/api/data/get`) — the same
//! endpoint the akshare *delisted* variants and `stock_financial_analysis_indicator_em`
//! use. It needs only a `SECUCODE` filter, no HTML scrape, and returns the same
//! statement rows. Symbol is accepted in either `SH600519` or `600519.SH` form.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

use crate::core::json::*;

/// Eastmoney datacenter financial REST endpoint (no JS signing — ADR-0005).
const DATACENTER: &str = "https://datacenter.eastmoney.com/securities/api/data/get";
/// Static client `v` token (matches akshare delisted variants).
const V_TOKEN: &str = "05767841728614413";

/// Normalize a symbol to Eastmoney `SECUCODE` form (`600519.SH`).
///
/// Accepts `SH600519` (akshare `*_em` convention) or `600519.SH` (datacenter
/// convention) and returns the dotted form used by the `SECUCODE` filter.
fn secucode_of(symbol: &str) -> String {
    if symbol.contains('.') {
        return symbol.to_uppercase();
    }
    if symbol.len() >= 2 {
        let (market, code) = symbol.split_at(2);
        return format!("{}.{}", code, market.to_uppercase());
    }
    symbol.to_string()
}

/// Extract the `result.data` array that every datacenter response shares.
fn result_data<'a>(resp: &'a Value, endpoint: &'static str) -> Result<&'a [Value]> {
    resp.get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .map(|a| a.as_slice())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: format!("missing result.data at {endpoint}"),
        })
}

// ---------------------------------------------------------------------------
// Profit / income statement (利润表)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProfitSheetRow {
    #[serde(rename = "SECUCODE")]
    pub secucode: String,
    #[serde(rename = "SECURITY_CODE")]
    pub security_code: String,
    #[serde(rename = "SECURITY_NAME_ABBR")]
    pub security_name: String,
    #[serde(rename = "ORG_CODE")]
    pub org_code: String,
    #[serde(rename = "REPORT_DATE")]
    pub report_date: String,
    #[serde(rename = "REPORT_TYPE")]
    pub report_type: String,
    #[serde(rename = "REPORT_DATE_NAME")]
    pub report_date_name: String,
    #[serde(rename = "TOTAL_OPERATE_INCOME")]
    pub total_operate_income: Option<f64>,
    #[serde(rename = "OPERATE_INCOME")]
    pub operate_income: Option<f64>,
    #[serde(rename = "OPERATE_COST")]
    pub operate_cost: Option<f64>,
    #[serde(rename = "OPERATE_PROFIT")]
    pub operate_profit: Option<f64>,
    #[serde(rename = "SUM_INCOME")]
    pub sum_income: Option<f64>,
    #[serde(rename = "PARENT_NETPROFIT")]
    pub parent_netprofit: Option<f64>,
    #[serde(rename = "BASIC_EPS")]
    pub basic_eps: Option<f64>,
    #[serde(rename = "DILUTE_EPS")]
    pub dilute_eps: Option<f64>,
    #[serde(rename = "WEIGHTAVG_ROE")]
    pub weightavg_roe: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_profit_sheet_by_report_em(symbol)`.
pub async fn stock_profit_sheet_by_report_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<ProfitSheetRow>> {
    let secucode = secucode_of(symbol);
    let filter = format!("(SECUCODE=\"{secucode}\")");
    let params = [
        ("type", "RPT_F10_FINANCE_GINCOME"),
        ("sty", "APP_F10_GINCOME"),
        ("filter", filter.as_str()),
        ("p", "1"),
        ("ps", "200"),
        ("sr", "-1"),
        ("st", "REPORT_DATE"),
        ("source", "HSF10"),
        ("client", "PC"),
        ("v", V_TOKEN),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_profit_sheet_by_report_em",
            DATACENTER,
            &params,
        )
        .await?;
    parse_profit(&v)
}

pub(crate) fn parse_profit(resp: &Value) -> Result<Vec<ProfitSheetRow>> {
    let data = result_data(resp, "stock_profit_sheet_by_report_em")?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        if let Some(row) = profit_row(item) {
            out.push(row);
        }
    }
    Ok(out)
}

fn profit_row(item: &Value) -> Option<ProfitSheetRow> {
    let report_date = opt_str_or(item, "REPORT_DATE", "");
    if report_date.is_empty() {
        return None;
    }
    Some(ProfitSheetRow {
        secucode: opt_str_or(item, "SECUCODE", ""),
        security_code: opt_str_or(item, "SECURITY_CODE", ""),
        security_name: opt_str_or(item, "SECURITY_NAME_ABBR", ""),
        org_code: opt_str_or(item, "ORG_CODE", ""),
        report_date,
        report_type: opt_str_or(item, "REPORT_TYPE", ""),
        report_date_name: opt_str_or(item, "REPORT_DATE_NAME", ""),
        total_operate_income: opt_f64(item, "TOTAL_OPERATE_INCOME"),
        operate_income: opt_f64(item, "OPERATE_INCOME"),
        operate_cost: opt_f64(item, "OPERATE_COST"),
        operate_profit: opt_f64(item, "OPERATE_PROFIT"),
        sum_income: opt_f64(item, "SUM_INCOME"),
        parent_netprofit: opt_f64(item, "PARENT_NETPROFIT"),
        basic_eps: opt_f64(item, "BASIC_EPS"),
        dilute_eps: opt_f64(item, "DILUTE_EPS"),
        weightavg_roe: opt_f64(item, "WEIGHTAVG_ROE"),
        source: SOURCE_EASTMONEY,
    })
}

// ---------------------------------------------------------------------------
// Balance sheet (资产负债表)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct BalanceSheetRow {
    #[serde(rename = "SECUCODE")]
    pub secucode: String,
    #[serde(rename = "SECURITY_CODE")]
    pub security_code: String,
    #[serde(rename = "SECURITY_NAME_ABBR")]
    pub security_name: String,
    #[serde(rename = "ORG_CODE")]
    pub org_code: String,
    #[serde(rename = "REPORT_DATE")]
    pub report_date: String,
    #[serde(rename = "REPORT_TYPE")]
    pub report_type: String,
    #[serde(rename = "REPORT_DATE_NAME")]
    pub report_date_name: String,
    #[serde(rename = "MONETARY_CAP")]
    pub monetary_cap: Option<f64>,
    #[serde(rename = "ACCOUNTS_RECEIVABLE")]
    pub accounts_receivable: Option<f64>,
    #[serde(rename = "INVENTORY")]
    pub inventory: Option<f64>,
    #[serde(rename = "TOTAL_CURRENT_ASSETS")]
    pub total_current_assets: Option<f64>,
    #[serde(rename = "TOTAL_ASSETS")]
    pub total_assets: Option<f64>,
    #[serde(rename = "TOTAL_CURRENT_LIAB")]
    pub total_current_liab: Option<f64>,
    #[serde(rename = "TOTAL_LIAB")]
    pub total_liab: Option<f64>,
    #[serde(rename = "TOTAL_EQUITY")]
    pub total_equity: Option<f64>,
    #[serde(rename = "PARENT_EQUITY")]
    pub parent_equity: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_balance_sheet_by_report_em(symbol)`.
pub async fn stock_balance_sheet_by_report_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<BalanceSheetRow>> {
    let secucode = secucode_of(symbol);
    let filter = format!("(SECUCODE=\"{secucode}\")");
    let params = [
        ("type", "RPT_F10_FINANCE_GBALANCE"),
        ("sty", "F10_FINANCE_GBALANCE"),
        ("filter", filter.as_str()),
        ("p", "1"),
        ("ps", "200"),
        ("sr", "-1"),
        ("st", "REPORT_DATE"),
        ("source", "HSF10"),
        ("client", "PC"),
        ("v", V_TOKEN),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_balance_sheet_by_report_em",
            DATACENTER,
            &params,
        )
        .await?;
    parse_balance(&v)
}

pub(crate) fn parse_balance(resp: &Value) -> Result<Vec<BalanceSheetRow>> {
    let data = result_data(resp, "stock_balance_sheet_by_report_em")?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        if let Some(row) = balance_row(item) {
            out.push(row);
        }
    }
    Ok(out)
}

fn balance_row(item: &Value) -> Option<BalanceSheetRow> {
    let report_date = opt_str_or(item, "REPORT_DATE", "");
    if report_date.is_empty() {
        return None;
    }
    Some(BalanceSheetRow {
        secucode: opt_str_or(item, "SECUCODE", ""),
        security_code: opt_str_or(item, "SECURITY_CODE", ""),
        security_name: opt_str_or(item, "SECURITY_NAME_ABBR", ""),
        org_code: opt_str_or(item, "ORG_CODE", ""),
        report_date,
        report_type: opt_str_or(item, "REPORT_TYPE", ""),
        report_date_name: opt_str_or(item, "REPORT_DATE_NAME", ""),
        monetary_cap: opt_f64(item, "MONETARY_CAP"),
        accounts_receivable: opt_f64(item, "ACCOUNTS_RECEIVABLE"),
        inventory: opt_f64(item, "INVENTORY"),
        total_current_assets: opt_f64(item, "TOTAL_CURRENT_ASSETS"),
        total_assets: opt_f64(item, "TOTAL_ASSETS"),
        total_current_liab: opt_f64(item, "TOTAL_CURRENT_LIAB"),
        total_liab: opt_f64(item, "TOTAL_LIAB"),
        total_equity: opt_f64(item, "TOTAL_EQUITY"),
        parent_equity: opt_f64(item, "PARENT_EQUITY"),
        source: SOURCE_EASTMONEY,
    })
}

// ---------------------------------------------------------------------------
// Cash-flow statement (现金流量表)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct CashFlowSheetRow {
    #[serde(rename = "SECUCODE")]
    pub secucode: String,
    #[serde(rename = "SECURITY_CODE")]
    pub security_code: String,
    #[serde(rename = "SECURITY_NAME_ABBR")]
    pub security_name: String,
    #[serde(rename = "ORG_CODE")]
    pub org_code: String,
    #[serde(rename = "REPORT_DATE")]
    pub report_date: String,
    #[serde(rename = "REPORT_TYPE")]
    pub report_type: String,
    #[serde(rename = "REPORT_DATE_NAME")]
    pub report_date_name: String,
    #[serde(rename = "CASH_RECEIVE_SALE")]
    pub cash_receive_sale: Option<f64>,
    #[serde(rename = "NET_OPERATE_CASH_FLOW")]
    pub net_operate_cash_flow: Option<f64>,
    #[serde(rename = "NET_INVEST_CASH_FLOW")]
    pub net_invest_cash_flow: Option<f64>,
    #[serde(rename = "NET_FINANCE_CASH_FLOW")]
    pub net_finance_cash_flow: Option<f64>,
    #[serde(rename = "CASH_END_PERIOD")]
    pub cash_end_period: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_cash_flow_sheet_by_report_em(symbol)`.
pub async fn stock_cash_flow_sheet_by_report_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<CashFlowSheetRow>> {
    let secucode = secucode_of(symbol);
    let filter = format!("(SECUCODE=\"{secucode}\")");
    let params = [
        ("type", "RPT_F10_FINANCE_GCASHFLOW"),
        ("sty", "APP_F10_GCASHFLOW"),
        ("filter", filter.as_str()),
        ("p", "1"),
        ("ps", "200"),
        ("sr", "-1"),
        ("st", "REPORT_DATE"),
        ("source", "HSF10"),
        ("client", "PC"),
        ("v", V_TOKEN),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_cash_flow_sheet_by_report_em",
            DATACENTER,
            &params,
        )
        .await?;
    parse_cash_flow(&v)
}

pub(crate) fn parse_cash_flow(resp: &Value) -> Result<Vec<CashFlowSheetRow>> {
    let data = result_data(resp, "stock_cash_flow_sheet_by_report_em")?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        if let Some(row) = cash_flow_row(item) {
            out.push(row);
        }
    }
    Ok(out)
}

fn cash_flow_row(item: &Value) -> Option<CashFlowSheetRow> {
    let report_date = opt_str_or(item, "REPORT_DATE", "");
    if report_date.is_empty() {
        return None;
    }
    Some(CashFlowSheetRow {
        secucode: opt_str_or(item, "SECUCODE", ""),
        security_code: opt_str_or(item, "SECURITY_CODE", ""),
        security_name: opt_str_or(item, "SECURITY_NAME_ABBR", ""),
        org_code: opt_str_or(item, "ORG_CODE", ""),
        report_date,
        report_type: opt_str_or(item, "REPORT_TYPE", ""),
        report_date_name: opt_str_or(item, "REPORT_DATE_NAME", ""),
        cash_receive_sale: opt_f64(item, "CASH_RECEIVE_SALE"),
        net_operate_cash_flow: opt_f64(item, "NET_OPERATE_CASH_FLOW"),
        net_invest_cash_flow: opt_f64(item, "NET_INVEST_CASH_FLOW"),
        net_finance_cash_flow: opt_f64(item, "NET_FINANCE_CASH_FLOW"),
        cash_end_period: opt_f64(item, "CASH_END_PERIOD"),
        source: SOURCE_EASTMONEY,
    })
}

// ---------------------------------------------------------------------------
// Main financial indicators (主要指标)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct FinancialIndicatorRow {
    #[serde(rename = "SECUCODE")]
    pub secucode: String,
    #[serde(rename = "SECURITY_CODE")]
    pub security_code: String,
    #[serde(rename = "SECURITY_NAME_ABBR")]
    pub security_name: String,
    #[serde(rename = "ORG_CODE")]
    pub org_code: String,
    #[serde(rename = "REPORT_DATE")]
    pub report_date: String,
    #[serde(rename = "REPORT_TYPE")]
    pub report_type: String,
    #[serde(rename = "REPORT_DATE_NAME")]
    pub report_date_name: String,
    #[serde(rename = "BASIC_EPS")]
    pub basic_eps: Option<f64>,
    #[serde(rename = "WEIGHTAVG_ROE")]
    pub weightavg_roe: Option<f64>,
    #[serde(rename = "GROSS_MARGIN")]
    pub gross_margin: Option<f64>,
    #[serde(rename = "NET_PROFIT_YOY")]
    pub net_profit_yoy: Option<f64>,
    #[serde(rename = "TOTAL_INCOME_YOY")]
    pub total_income_yoy: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_financial_analysis_indicator_em(symbol, indicator)`.
///
/// `indicator` is `"按报告期"` (default) or `"按单季度"`.
pub async fn stock_financial_analysis_indicator_em(
    client: &Client,
    symbol: &str,
    indicator: &str,
) -> Result<Vec<FinancialIndicatorRow>> {
    if indicator == "按单季度" {
        let filter = format!("(SECUCODE=\"{}\")", secucode_of(symbol));
        let params = [
            ("reportName", "RPT_F10_QTR_MAINFINADATA"),
            ("columns", "ALL"),
            ("quoteColumns", ""),
            ("filter", filter.as_str()),
            ("pageNumber", "1"),
            ("pageSize", "200"),
            ("sortTypes", "-1"),
            ("sortColumns", "REPORT_DATE"),
            ("source", "HSF10"),
            ("client", "PC"),
        ];
        let v = client
            .get_json(
                SOURCE_EASTMONEY,
                "stock_financial_analysis_indicator_em",
                "https://datacenter.eastmoney.com/securities/api/data/v1/get",
                &params,
            )
            .await?;
        parse_indicator(&v)
    } else {
        let filter = format!("(SECUCODE=\"{}\")", secucode_of(symbol));
        let params = [
            ("type", "RPT_F10_FINANCE_MAINFINADATA"),
            ("sty", "APP_F10_MAINFINADATA"),
            ("quoteColumns", ""),
            ("filter", filter.as_str()),
            ("p", "1"),
            ("ps", "200"),
            ("sr", "-1"),
            ("st", "REPORT_DATE"),
            ("source", "HSF10"),
            ("client", "PC"),
        ];
        let v = client
            .get_json(
                SOURCE_EASTMONEY,
                "stock_financial_analysis_indicator_em",
                DATACENTER,
                &params,
            )
            .await?;
        parse_indicator(&v)
    }
}

pub(crate) fn parse_indicator(resp: &Value) -> Result<Vec<FinancialIndicatorRow>> {
    let data = result_data(resp, "stock_financial_analysis_indicator_em")?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        if let Some(row) = indicator_row(item) {
            out.push(row);
        }
    }
    Ok(out)
}

fn indicator_row(item: &Value) -> Option<FinancialIndicatorRow> {
    let report_date = opt_str_or(item, "REPORT_DATE", "");
    if report_date.is_empty() {
        return None;
    }
    Some(FinancialIndicatorRow {
        secucode: opt_str_or(item, "SECUCODE", ""),
        security_code: opt_str_or(item, "SECURITY_CODE", ""),
        security_name: opt_str_or(item, "SECURITY_NAME_ABBR", ""),
        org_code: opt_str_or(item, "ORG_CODE", ""),
        report_date,
        report_type: opt_str_or(item, "REPORT_TYPE", ""),
        report_date_name: opt_str_or(item, "REPORT_DATE_NAME", ""),
        basic_eps: opt_f64(item, "BASIC_EPS"),
        weightavg_roe: opt_f64(item, "WEIGHTAVG_ROE"),
        gross_margin: opt_f64(item, "GROSS_MARGIN"),
        net_profit_yoy: opt_f64(item, "NET_PROFIT_YOY"),
        total_income_yoy: opt_f64(item, "TOTAL_INCOME_YOY"),
        source: SOURCE_EASTMONEY,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let txt = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    #[test]
    fn parses_profit_fixture() {
        let v = fixture("stock_profit_sheet_by_report_em.json");
        let rows = parse_profit(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].secucode, "600519.SH");
        assert_eq!(rows[0].report_date, "2024-03-31T00:00:00");
        assert_eq!(rows[0].total_operate_income, Some(46_470_614_350.29));
        assert_eq!(rows[0].parent_netprofit, Some(24_065_276_197.00));
        assert_eq!(rows[0].basic_eps, Some(19.16));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].report_date, "2023-12-31T00:00:00");
    }

    #[test]
    fn parses_balance_fixture() {
        let v = fixture("stock_balance_sheet_by_report_em.json");
        let rows = parse_balance(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].secucode, "600519.SH");
        assert_eq!(rows[0].total_assets, Some(268_396_235_769.37));
        assert_eq!(rows[0].total_liab, Some(49_504_351_109.47));
        assert_eq!(rows[0].parent_equity, Some(218_585_344_534.43));
    }

    #[test]
    fn parses_cash_flow_fixture() {
        let v = fixture("stock_cash_flow_sheet_by_report_em.json");
        let rows = parse_cash_flow(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].secucode, "600519.SH");
        assert_eq!(rows[0].net_operate_cash_flow, Some(9_190_492_745.11));
        assert_eq!(rows[0].cash_end_period, Some(150_639_980_825.28));
    }

    #[test]
    fn parses_indicator_fixture() {
        let v = fixture("stock_financial_analysis_indicator_em.json");
        let rows = parse_indicator(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].secucode, "301389.SZ");
        assert_eq!(rows[0].basic_eps, Some(1.23));
        assert_eq!(rows[0].weightavg_roe, Some(12.34));
    }

    #[test]
    fn skips_malformed_rows() {
        let bad = serde_json::json!({
            "result": { "data": [
                { "SECUCODE": "600519.SH", "REPORT_DATE": "2024-03-31T00:00:00", "TOTAL_OPERATE_INCOME": "46470614350.29" },
                { "SECUCODE": "600519.SH" },
                { "SECURITY_CODE": "600519" }
            ]}
        });
        let rows = parse_profit(&bad).unwrap();
        assert_eq!(rows.len(), 1);
    }
}
