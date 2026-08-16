//! Additional financial-statement endpoints ported from akshare.
//!
//! This module ports two upstream akshare sources:
//!
//! - `akshare/stock_fundamental/stock_finance_ths.py` — 同花顺 (10jqka) financial
//!   indicators. The legacy `*_ths` endpoints return a JSON document whose
//!   `flashData` field is itself a JSON *string*; the `_new_ths` endpoints hit
//!   the `basicapi` REST endpoint and return a nested `data.data[]` list.
//! - `akshare/stock_fundamental/stock_finance_hk_em.py` and
//!   `akshare/stock_fundamental/stock_finance_us_em.py` — Eastmoney datacenter
//!   (`datacenter.eastmoney.com/securities/api/data/v1/get`) Hong Kong and US
//!   financial statements + main indicators.
//!
//! | Rust fn                                    | akshare fn                              | source   | akshare file:line                                   |
//! |--------------------------------------------|-----------------------------------------|----------|-----------------------------------------------------|
//! | `stock_financial_debt_ths`                 | `stock_financial_debt_ths`             | ths      | `akshare/stock_fundamental/stock_finance_ths.py:58` |
//! | `stock_financial_benefit_ths`              | `stock_financial_benefit_ths`          | ths      | `akshare/stock_fundamental/stock_finance_ths.py:92` |
//! | `stock_financial_cash_ths`                 | `stock_financial_cash_ths`             | ths      | `akshare/stock_fundamental/stock_finance_ths.py:130` |
//! | `stock_financial_abstract_new_ths`         | `stock_financial_abstract_new_ths`     | ths      | `akshare/stock_fundamental/stock_finance_ths.py:194` |
//! | `stock_financial_debt_new_ths`             | `stock_financial_debt_new_ths`         | ths      | `akshare/stock_fundamental/stock_finance_ths.py:291` |
//! | `stock_financial_benefit_new_ths`          | `stock_financial_benefit_new_ths`      | ths      | `akshare/stock_fundamental/stock_finance_ths.py:380` |
//! | `stock_financial_cash_new_ths`             | `stock_financial_cash_new_ths`         | ths      | `akshare/stock_fundamental/stock_finance_ths.py:477` |
//! | `stock_financial_hk_report_em`             | `stock_financial_hk_report_em`         | eastmoney| `akshare/stock_fundamental/stock_finance_hk_em.py:13` |
//! | `stock_financial_hk_analysis_indicator_em` | `stock_financial_hk_analysis_indicator_em` | eastmoney| `akshare/stock_fundamental/stock_finance_hk_em.py:108` |
//! | `stock_financial_us_report_em`             | `stock_financial_us_report_em`         | eastmoney| `akshare/stock_fundamental/stock_finance_us_em.py:110` |
//! | `stock_financial_us_analysis_indicator_em` | `stock_financial_us_analysis_indicator_em` | eastmoney| `akshare/stock_fundamental/stock_finance_us_em.py:158` |
//!
//! ## DEFERRED (HTML scrape — not pure HTTP, do NOT port)
//!
//! - `stock_financial_abstract_ths` (`stock_finance_ths.py:18`) — scrapes
//!   `#main` out of `finance.html` (a `.phtml` page).
//! - `stock_management_change_ths` (`stock_finance_ths.py:574`) — scrapes an
//!   HTML `<table>` out of `event.html` (gb2312).
//! - `stock_shareholder_change_ths` (`stock_finance_ths.py:622`) — scrapes an
//!   HTML `<table>` out of `event.html` (gb2312).

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

/// 10jqka (同花顺) source bucket, for rate limiting / error context.
const SOURCE_THS: &str = "ths";

/// 10jqka `basicapi` financial data endpoint (used by the `*_new_ths` fns).
const THS_APP_API: &str = "https://basic.10jqka.com.cn/basicapi/finance/index/v1/app_data/";

/// Eastmoney datacenter `v1/get` endpoint (HK / US statements + indicators).
const EM_DATACENTER: &str = "https://datacenter.eastmoney.com/securities/api/data/v1/get";

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Read a field's display string: numbers become their text form, strings are
/// cloned, everything else becomes empty.
fn fstr(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

/// `fstr` for an optional field reference (missing → empty string).
fn fstr_of(v: Option<&Value>) -> String {
    v.map(fstr).unwrap_or_default()
}

/// Read a numeric field that may be a JSON number or a plain numeric string.
fn fnum_opt(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// `fnum_opt` for an optional field reference.
fn fnum_of(v: Option<&Value>) -> Option<f64> {
    v.and_then(fnum_opt)
}

/// Read a string field as `Option<String>` (missing / null → `None`).
fn str_of(v: Option<&Value>) -> Option<String> {
    match v {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

/// Extract `result.data` (the row array) from a datacenter response.
fn em_data<'a>(resp: &'a Value, endpoint: &'static str) -> Result<&'a [Value]> {
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
// 10jqka — legacy `*_ths` statements (flashData JSON string)
// ---------------------------------------------------------------------------

/// One normalized financial line-item row for the legacy 10jqka `*_ths` fns.
///
/// The upstream JSON is pivoted: columns are report periods and rows are
/// financial line items, so we emit one row per (item, date) cell.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FinanceThsRow {
    /// 指标项 (financial line-item name, e.g. 货币资金).
    pub item: String,
    /// 报告期 (report period date, e.g. 2024-03-31).
    pub date: Option<String>,
    /// 数值 (cell value).
    pub value: Option<f64>,
    /// Data source (`ths`).
    pub source: &'static str,
}

/// Parse a legacy `*_ths` `flashData` document into [`FinanceThsRow`]s.
///
/// `key` selects the period block: `"report"` (按报告期), `"year"` (按年度) or
/// `"simple"` (按单季度). Mirrors akshare's pivot of the `title` / `<key>`
/// arrays inside `json.loads(flashData)`.
pub(crate) fn parse_ths(resp: &Value, key: &str) -> Result<Vec<FinanceThsRow>> {
    let flash = resp
        .get("flashData")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_THS,
            message: "missing flashData string".into(),
        })?;
    let inner: Value = serde_json::from_str(flash).map_err(Error::Json)?;
    let block =
        inner
            .get(key)
            .and_then(|v| v.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_THS,
                message: format!("missing {key} block in flashData"),
            })?;
    if block.is_empty() {
        return Ok(Vec::new());
    }
    let columns = block[0].as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_THS,
        message: "report header is not an array".into(),
    })?;
    // `columns[0]` is the axis label ("报告期"); the rest are report dates.
    let dates: Vec<String> = columns.iter().skip(1).map(fstr).collect();

    // `title` carries the line-item names; `title[0]` is an axis label.
    let titles: Vec<String> = inner
        .get("title")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().skip(1).map(title_name).collect())
        .unwrap_or_default();

    let mut out = Vec::new();
    for (i, row) in block.iter().enumerate().skip(1) {
        let vals = match row.as_array() {
            Some(v) => v,
            None => continue,
        };
        let item = titles
            .get(i - 1)
            .cloned()
            .unwrap_or_else(|| fstr_of(vals.first()));
        for (j, date) in dates.iter().enumerate() {
            let value = vals.get(j).and_then(fnum_opt);
            out.push(FinanceThsRow {
                item: item.clone(),
                date: Some(date.clone()),
                value,
                source: SOURCE_THS,
            });
        }
    }
    Ok(out)
}

/// Extract a line-item name from a `title` entry (may be a `[name, ...]` list).
fn title_name(v: &Value) -> String {
    match v {
        Value::Array(a) => a.first().map(fstr).unwrap_or_default(),
        _ => fstr(v),
    }
}

/// Pick the `flashData` block key for the legacy debt statement.
fn debt_key(indicator: &str) -> &'static str {
    match indicator {
        "按报告期" => "report",
        _ => "year",
    }
}

/// Pick the `flashData` block key for the legacy benefit / cash statements.
fn benefit_key(indicator: &str) -> &'static str {
    match indicator {
        "按报告期" => "report",
        "按单季度" => "simple",
        _ => "year",
    }
}

/// Port of `stock_financial_debt_ths(symbol, indicator)` — 同花顺 资产负债表.
pub async fn stock_financial_debt_ths(
    client: &Client,
    symbol: &str,
    indicator: &str,
) -> Result<Vec<FinanceThsRow>> {
    let url = format!("https://basic.10jqka.com.cn/api/stock/finance/{symbol}_debt.json");
    let v = client
        .get_json(SOURCE_THS, "stock_financial_debt_ths", &url, &[])
        .await?;
    parse_ths(&v, debt_key(indicator))
}

/// Port of `stock_financial_benefit_ths(symbol, indicator)` — 同花顺 利润表.
pub async fn stock_financial_benefit_ths(
    client: &Client,
    symbol: &str,
    indicator: &str,
) -> Result<Vec<FinanceThsRow>> {
    let url = format!("https://basic.10jqka.com.cn/api/stock/finance/{symbol}_benefit.json");
    let v = client
        .get_json(SOURCE_THS, "stock_financial_benefit_ths", &url, &[])
        .await?;
    parse_ths(&v, benefit_key(indicator))
}

/// Port of `stock_financial_cash_ths(symbol, indicator)` — 同花顺 现金流量表.
pub async fn stock_financial_cash_ths(
    client: &Client,
    symbol: &str,
    indicator: &str,
) -> Result<Vec<FinanceThsRow>> {
    let url = format!("https://basic.10jqka.com.cn/api/stock/finance/{symbol}_cash.json");
    let v = client
        .get_json(SOURCE_THS, "stock_financial_cash_ths", &url, &[])
        .await?;
    parse_ths(&v, benefit_key(indicator))
}

// ---------------------------------------------------------------------------
// 10jqka — `*_new_ths` statements (basicapi nested data.data[])
// ---------------------------------------------------------------------------

/// One normalized financial line-item row for the 10jqka `*_new_ths` fns.
///
/// The upstream nests each metric under `index_list[metric_name]`, which may be
/// a dict of named sub-fields or a bare scalar, so we emit one row per
/// (metric, field) cell.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FinanceThsNewRow {
    /// 报告期日期 (`date`).
    pub report_date: String,
    /// 报告名称 (`report_name`).
    pub report_name: String,
    /// 报告期 (`report`).
    pub report_period: String,
    /// 季度名称 (`quarter_name`).
    pub quarter_name: String,
    /// 指标名称 (metric name within `index_list`).
    pub metric_name: String,
    /// 指标字段名 (sub-field key when `index_list[metric]` is a dict).
    pub field: Option<String>,
    /// 指标值.
    pub value: Option<f64>,
    /// Data source (`ths`).
    pub source: &'static str,
}

/// Resolve a 6-digit A-share code to the 10jqka `market` id (SZ=33, SH=17, BJ=151).
fn market_code(symbol: &str) -> &'static str {
    if ["000", "001", "002", "003", "300"]
        .iter()
        .any(|p| symbol.starts_with(*p))
    {
        "33"
    } else if ["600", "601", "603", "605", "688"]
        .iter()
        .any(|p| symbol.starts_with(*p))
    {
        "17"
    } else if symbol.starts_with("920") {
        "151"
    } else {
        "0"
    }
}

/// Map an indicator to the 10jqka `period` query param for the `*_new_ths` fns.
fn ths_new_period(indicator: &str) -> &'static str {
    match indicator {
        "按报告期" => "0",
        "一季度" => "1",
        "二季度" => "2",
        "三季度" => "3",
        _ => "4", // 四季度 / 按年度
    }
}

/// Parse a `*_new_ths` `basicapi` response into [`FinanceThsNewRow`]s.
pub(crate) fn parse_ths_new(resp: &Value) -> Result<Vec<FinanceThsNewRow>> {
    let reports = resp
        .get("data")
        .and_then(|d| d.get("data"))
        .and_then(|a| a.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_THS,
            message: "missing data.data array".into(),
        })?;
    let mut out = Vec::new();
    for report in reports {
        let report_date = fstr_of(report.get("date"));
        let report_name = fstr_of(report.get("report_name"));
        let report_period = fstr_of(report.get("report"));
        let quarter_name = fstr_of(report.get("quarter_name"));
        let index_list = match report.get("index_list").and_then(|v| v.as_object()) {
            Some(m) => m,
            None => continue,
        };
        for (metric_name, metric_values) in index_list {
            if let Some(obj) = metric_values.as_object() {
                for (field, val) in obj {
                    out.push(FinanceThsNewRow {
                        report_date: report_date.clone(),
                        report_name: report_name.clone(),
                        report_period: report_period.clone(),
                        quarter_name: quarter_name.clone(),
                        metric_name: metric_name.clone(),
                        field: Some(field.clone()),
                        value: fnum_opt(val),
                        source: SOURCE_THS,
                    });
                }
            } else {
                out.push(FinanceThsNewRow {
                    report_date: report_date.clone(),
                    report_name: report_name.clone(),
                    report_period: report_period.clone(),
                    quarter_name: quarter_name.clone(),
                    metric_name: metric_name.clone(),
                    field: None,
                    value: fnum_opt(metric_values),
                    source: SOURCE_THS,
                });
            }
        }
    }
    Ok(out)
}

/// Port of `stock_financial_abstract_new_ths(symbol, indicator)` — 同花顺 重要指标.
pub async fn stock_financial_abstract_new_ths(
    client: &Client,
    symbol: &str,
    indicator: &str,
) -> Result<Vec<FinanceThsNewRow>> {
    let params = [
        ("code", symbol),
        ("id", "client_stock_importance"),
        ("market", market_code(symbol)),
        ("type", "stock"),
        ("page", "1"),
        ("size", "50"),
        ("period", ths_new_period(indicator)),
    ];
    let v = client
        .get_json(
            SOURCE_THS,
            "stock_financial_abstract_new_ths",
            THS_APP_API,
            &params,
        )
        .await?;
    parse_ths_new(&v)
}

/// Port of `stock_financial_debt_new_ths(symbol, indicator)` — 同花顺 资产负债表.
pub async fn stock_financial_debt_new_ths(
    client: &Client,
    symbol: &str,
    indicator: &str,
) -> Result<Vec<FinanceThsNewRow>> {
    let params = [
        ("code", symbol),
        ("id", "client_stock_debt"),
        ("market", market_code(symbol)),
        ("type", "stock"),
        ("page", "1"),
        ("size", "50"),
        ("period", ths_new_period(indicator)),
    ];
    let v = client
        .get_json(
            SOURCE_THS,
            "stock_financial_debt_new_ths",
            THS_APP_API,
            &params,
        )
        .await?;
    parse_ths_new(&v)
}

/// Port of `stock_financial_benefit_new_ths(symbol, indicator)` — 同花顺 利润表.
pub async fn stock_financial_benefit_new_ths(
    client: &Client,
    symbol: &str,
    indicator: &str,
) -> Result<Vec<FinanceThsNewRow>> {
    let params = [
        ("code", symbol),
        ("id", "client_stock_benefit"),
        ("market", market_code(symbol)),
        ("type", "stock"),
        ("page", "1"),
        ("size", "50"),
        ("period", ths_new_period(indicator)),
    ];
    let v = client
        .get_json(
            SOURCE_THS,
            "stock_financial_benefit_new_ths",
            THS_APP_API,
            &params,
        )
        .await?;
    parse_ths_new(&v)
}

/// Port of `stock_financial_cash_new_ths(symbol, indicator)` — 同花顺 现金流量表.
pub async fn stock_financial_cash_new_ths(
    client: &Client,
    symbol: &str,
    indicator: &str,
) -> Result<Vec<FinanceThsNewRow>> {
    let params = [
        ("code", symbol),
        ("id", "client_stock_cash"),
        ("market", market_code(symbol)),
        ("type", "stock"),
        ("page", "1"),
        ("size", "50"),
        ("period", ths_new_period(indicator)),
    ];
    let v = client
        .get_json(
            SOURCE_THS,
            "stock_financial_cash_new_ths",
            THS_APP_API,
            &params,
        )
        .await?;
    parse_ths_new(&v)
}

// ---------------------------------------------------------------------------
// Eastmoney — HK financial statements (`stock_financial_hk_report_em`)
// ---------------------------------------------------------------------------

/// One HK financial-statement row (资产负债表 / 利润表 / 现金流量表).
///
/// The three HK report types share the same column shape; `start_date` is
/// present on the income / cashflow reports and `std_report_date` on the
/// balance-sheet report.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FinanceHkReportRow {
    /// `SECUCODE` (e.g. 00700.HK).
    pub secucode: String,
    /// `SECURITY_CODE` (e.g. 00700).
    pub security_code: String,
    /// `SECURITY_NAME_ABBR`.
    pub security_name: String,
    /// `ORG_CODE`.
    pub org_code: String,
    /// `REPORT_DATE`.
    pub report_date: String,
    /// `DATE_TYPE_CODE`.
    pub date_type_code: String,
    /// `FISCAL_YEAR`.
    pub fiscal_year: String,
    /// `STD_ITEM_CODE` (standard line-item code).
    pub std_item_code: String,
    /// `STD_ITEM_NAME` (standard line-item name).
    pub std_item_name: String,
    /// `AMOUNT` (line-item value).
    pub amount: Option<f64>,
    /// `START_DATE` (income / cashflow reports only).
    pub start_date: Option<String>,
    /// `STD_REPORT_DATE` (balance-sheet report only).
    pub std_report_date: Option<String>,
    /// Data source (`eastmoney`).
    pub source: &'static str,
}

/// Parse a HK `result.data` array into [`FinanceHkReportRow`]s.
pub(crate) fn parse_stock_financial_hk_report_em(resp: &Value) -> Result<Vec<FinanceHkReportRow>> {
    let data = em_data(resp, "stock_financial_hk_report_em")?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(FinanceHkReportRow {
            secucode: fstr_of(item.get("SECUCODE")),
            security_code: fstr_of(item.get("SECURITY_CODE")),
            security_name: fstr_of(item.get("SECURITY_NAME_ABBR")),
            org_code: fstr_of(item.get("ORG_CODE")),
            report_date: fstr_of(item.get("REPORT_DATE")),
            date_type_code: fstr_of(item.get("DATE_TYPE_CODE")),
            fiscal_year: fstr_of(item.get("FISCAL_YEAR")),
            std_item_code: fstr_of(item.get("STD_ITEM_CODE")),
            std_item_name: fstr_of(item.get("STD_ITEM_NAME")),
            amount: fnum_of(item.get("AMOUNT")),
            start_date: str_of(item.get("START_DATE")),
            std_report_date: str_of(item.get("STD_REPORT_DATE")),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

/// Port of `stock_financial_hk_report_em(stock, symbol, indicator)`.
///
/// `symbol` ∈ {"资产负债表", "利润表", "现金流量表"}; `indicator` ∈ {"年度", "报告期"}.
/// Performs the two-step Eastmoney dance: first the summary report list, then
/// the per-statement rows filtered to the selected report dates.
pub async fn stock_financial_hk_report_em(
    client: &Client,
    stock: &str,
    symbol: &str,
    indicator: &str,
) -> Result<Vec<FinanceHkReportRow>> {
    let summary_params = [
        ("reportName", "RPT_CUSTOM_HKSK_APPFN_CASHFLOW_SUMMARY"),
        (
            "columns",
            "SECUCODE,SECURITY_CODE,SECURITY_NAME_ABBR,START_DATE,REPORT_DATE,FISCAL_YEAR,\
CURRENCY,ACCOUNT_STANDARD,REPORT_TYPE",
        ),
        ("quoteColumns", ""),
        ("filter", &format!(r#"(SECUCODE="{stock}.HK")"#)),
        ("source", "F10"),
        ("client", "PC"),
        ("v", "02092616586970355"),
    ];
    let summary = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_financial_hk_report_em",
            EM_DATACENTER,
            &summary_params,
        )
        .await?;
    let report_list = summary
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.get(0))
        .and_then(|a| a.get("REPORT_LIST"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data[0].REPORT_LIST".into(),
        })?;
    let year_list: Vec<String> = report_list
        .iter()
        .filter(|item| {
            indicator != "年度" || item.get("REPORT_TYPE").and_then(|v| v.as_str()) == Some("年报")
        })
        .filter_map(|item| {
            item.get("REPORT_DATE")
                .and_then(|v| v.as_str())
                .map(|s| s.split(' ').next().unwrap_or(s).to_string())
        })
        .collect();
    if year_list.is_empty() {
        return Ok(Vec::new());
    }
    let (report_name, columns) = match symbol {
        "资产负债表" => (
            "RPT_HKF10_FN_BALANCE_PC",
            "SECUCODE,SECURITY_CODE,SECURITY_NAME_ABBR,ORG_CODE,REPORT_DATE,DATE_TYPE_CODE,\
FISCAL_YEAR,STD_ITEM_CODE,STD_ITEM_NAME,AMOUNT,STD_REPORT_DATE",
        ),
        "利润表" => (
            "RPT_HKF10_FN_INCOME_PC",
            "SECUCODE,SECURITY_CODE,SECURITY_NAME_ABBR,ORG_CODE,REPORT_DATE,DATE_TYPE_CODE,\
FISCAL_YEAR,START_DATE,STD_ITEM_CODE,STD_ITEM_NAME,AMOUNT",
        ),
        "现金流量表" => (
            "RPT_HKF10_FN_CASHFLOW_PC",
            "SECUCODE,SECURITY_CODE,SECURITY_NAME_ABBR,ORG_CODE,REPORT_DATE,DATE_TYPE_CODE,\
FISCAL_YEAR,START_DATE,STD_ITEM_CODE,STD_ITEM_NAME,AMOUNT",
        ),
        _ => return Ok(Vec::new()),
    };
    let joined = year_list
        .iter()
        .map(|y| format!("'{y}'"))
        .collect::<Vec<_>>()
        .join(",");
    let filter = format!(r#"(SECUCODE="{stock}.HK")(REPORT_DATE in ({joined}))"#);
    let params = [
        ("reportName", report_name),
        ("columns", columns),
        ("quoteColumns", ""),
        ("filter", filter.as_str()),
        ("pageNumber", "1"),
        ("pageSize", ""),
        ("sortTypes", "-1,1"),
        ("sortColumns", "REPORT_DATE,STD_ITEM_CODE"),
        ("source", "F10"),
        ("client", "PC"),
        ("v", "01975982096513973"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_financial_hk_report_em",
            EM_DATACENTER,
            &params,
        )
        .await?;
    parse_stock_financial_hk_report_em(&v)
}

// ---------------------------------------------------------------------------
// Eastmoney — HK main indicators (`stock_financial_hk_analysis_indicator_em`)
// ---------------------------------------------------------------------------

/// One HK main-financial-indicator row.
///
/// `columns=HKF10_FN_MAININDICATOR` is a column group alias expanded by
/// Eastmoney; we capture the header fields plus the common main indicators.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FinanceHkIndicatorRow {
    /// `SECUCODE`.
    pub secucode: String,
    /// `SECURITY_CODE`.
    pub security_code: String,
    /// `SECURITY_NAME_ABBR`.
    pub security_name: String,
    /// `REPORT_DATE`.
    pub report_date: String,
    /// `STD_REPORT_DATE`.
    pub std_report_date: Option<String>,
    /// 营业收入.
    pub total_income: Option<f64>,
    /// 净利润.
    pub net_profit: Option<f64>,
    /// 毛利率.
    pub gross_margin: Option<f64>,
    /// 净利率.
    pub net_margin: Option<f64>,
    /// 净资产收益率.
    pub roe: Option<f64>,
    /// 总资产收益率.
    pub roa: Option<f64>,
    /// 资产负债率.
    pub debt_ratio: Option<f64>,
    /// 每股收益.
    pub eps: Option<f64>,
    /// Data source (`eastmoney`).
    pub source: &'static str,
}

/// Parse a HK `result.data` array into [`FinanceHkIndicatorRow`]s.
pub(crate) fn parse_stock_financial_hk_analysis_indicator_em(
    resp: &Value,
) -> Result<Vec<FinanceHkIndicatorRow>> {
    let data = em_data(resp, "stock_financial_hk_analysis_indicator_em")?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(FinanceHkIndicatorRow {
            secucode: fstr_of(item.get("SECUCODE")),
            security_code: fstr_of(item.get("SECURITY_CODE")),
            security_name: fstr_of(item.get("SECURITY_NAME_ABBR")),
            report_date: fstr_of(item.get("REPORT_DATE")),
            std_report_date: str_of(item.get("STD_REPORT_DATE")),
            total_income: fnum_of(item.get("TOTAL_INCOME")),
            net_profit: fnum_of(item.get("NET_PROFIT")),
            gross_margin: fnum_of(item.get("GROSS_MARGIN")),
            net_margin: fnum_of(item.get("NET_MARGIN")),
            roe: fnum_of(item.get("ROE")),
            roa: fnum_of(item.get("ROA")),
            debt_ratio: fnum_of(item.get("DEBT_RATIO")),
            eps: fnum_of(item.get("EPS")),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

/// Port of `stock_financial_hk_analysis_indicator_em(symbol, indicator)`.
///
/// `indicator` ∈ {"年度", "报告期"}; maps to a `DATE_TYPE_CODE="001"` filter for annual.
pub async fn stock_financial_hk_analysis_indicator_em(
    client: &Client,
    symbol: &str,
    indicator: &str,
) -> Result<Vec<FinanceHkIndicatorRow>> {
    let filter = if indicator == "年度" {
        format!(r#"(SECUCODE="{symbol}.HK")(DATE_TYPE_CODE="001")"#)
    } else {
        format!(r#"(SECUCODE="{symbol}.HK")"#)
    };
    let params = [
        ("reportName", "RPT_HKF10_FN_MAININDICATOR"),
        ("columns", "HKF10_FN_MAININDICATOR"),
        ("quoteColumns", ""),
        ("pageNumber", "1"),
        ("pageSize", "9"),
        ("sortTypes", "-1"),
        ("sortColumns", "STD_REPORT_DATE"),
        ("source", "F10"),
        ("client", "PC"),
        ("v", "01975982096513973"),
        ("filter", filter.as_str()),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_financial_hk_analysis_indicator_em",
            EM_DATACENTER,
            &params,
        )
        .await?;
    parse_stock_financial_hk_analysis_indicator_em(&v)
}

// ---------------------------------------------------------------------------
// Eastmoney — US financial statements (`stock_financial_us_report_em`)
// ---------------------------------------------------------------------------

/// One US financial-statement row (资产负债表 / 综合损益表 / 现金流量表).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FinanceUsReportRow {
    /// `SECUCODE` (e.g. TSLA.O).
    pub secucode: String,
    /// `SECURITY_CODE` (e.g. TSLA).
    pub security_code: String,
    /// `SECURITY_NAME_ABBR`.
    pub security_name: String,
    /// `REPORT_DATE`.
    pub report_date: String,
    /// `REPORT_TYPE`.
    pub report_type: String,
    /// `REPORT` (period tag, e.g. FY2023 / 2023Q1).
    pub report: String,
    /// `STD_ITEM_CODE`.
    pub std_item_code: String,
    /// `ITEM_NAME`.
    pub item_name: String,
    /// `AMOUNT` (line-item value).
    pub amount: Option<f64>,
    /// Data source (`eastmoney`).
    pub source: &'static str,
}

/// Parse a US `result.data` array into [`FinanceUsReportRow`]s.
pub(crate) fn parse_stock_financial_us_report_em(resp: &Value) -> Result<Vec<FinanceUsReportRow>> {
    let data = em_data(resp, "stock_financial_us_report_em")?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(FinanceUsReportRow {
            secucode: fstr_of(item.get("SECUCODE")),
            security_code: fstr_of(item.get("SECURITY_CODE")),
            security_name: fstr_of(item.get("SECURITY_NAME_ABBR")),
            report_date: fstr_of(item.get("REPORT_DATE")),
            report_type: fstr_of(item.get("REPORT_TYPE")),
            report: fstr_of(item.get("REPORT")),
            std_item_code: fstr_of(item.get("STD_ITEM_CODE")),
            item_name: fstr_of(item.get("ITEM_NAME")),
            amount: fnum_of(item.get("AMOUNT")),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

/// Resolve an Eastmoney US `SECURITY_CODE` to its `SECUCODE` (e.g. TSLA → TSLA.O).
async fn us_secucode(client: &Client, symbol: &str) -> Result<String> {
    let params = [
        ("reportName", "RPT_USF10_INFO_ORGPROFILE"),
        (
            "columns",
            "SECUCODE,SECURITY_CODE,ORG_CODE,SECURITY_INNER_CODE,ORG_NAME,ORG_EN_ABBR,\
BELONG_INDUSTRY,FOUND_DATE,CHAIRMAN,REG_PLACE,ADDRESS,EMP_NUM,ORG_TEL,ORG_FAX,\
ORG_EMAIL,ORG_WEB,ORG_PROFILE",
        ),
        ("quoteColumns", ""),
        ("filter", &format!(r#"(SECURITY_CODE="{symbol}")"#)),
        ("pageNumber", "1"),
        ("pageSize", "200"),
        ("sortTypes", ""),
        ("sortColumns", ""),
        ("source", "SECURITIES"),
        ("client", "PC"),
        ("v", "04406064331266868"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_financial_us_report_em",
            EM_DATACENTER,
            &params,
        )
        .await?;
    let data = em_data(&v, "stock_financial_us_report_em")?;
    data.first()
        .and_then(|f| f.get("SECUCODE"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing SECUCODE in org profile".into(),
        })
}

/// Report-name (Eastmoney `reportName`) for a US statement type.
fn us_report_name(symbol: &str) -> Result<&'static str> {
    match symbol {
        "资产负债表" => Ok("RPT_USF10_FN_BALANCE"),
        "综合损益表" => Ok("RPT_USF10_FN_INCOME"),
        "现金流量表" => Ok("RPT_USSK_FN_CASHFLOW"),
        other => Err(Error::InvalidParam(format!(
            "stock_financial_us_report_em: unknown symbol {other:?}"
        ))),
    }
}

/// Build the `(REPORT in ("FY2023",...))` filter for the selected indicator.
async fn us_report_dates(
    client: &Client,
    secucode: &str,
    symbol: &str,
    indicator: &str,
) -> Result<String> {
    let report_name = us_report_name(symbol)?;
    let params = [
        ("reportName", report_name),
        (
            "columns",
            "SECUCODE,SECURITY_CODE,SECURITY_NAME_ABBR,REPORT,REPORT_DATE,FISCAL_YEAR,\
CURRENCY,ACCOUNT_STANDARD,REPORT_TYPE,DATE_TYPE_CODE",
        ),
        ("quoteColumns", ""),
        ("filter", &format!(r#"(SECUCODE="{secucode}")"#)),
        ("pageNumber", ""),
        ("pageSize", ""),
        ("sortTypes", "-1"),
        ("sortColumns", "REPORT_DATE"),
        ("source", "SECURITIES"),
        ("client", "PC"),
        ("v", "09583551779242467"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_financial_us_report_em",
            EM_DATACENTER,
            &params,
        )
        .await?;
    let data = em_data(&v, "stock_financial_us_report_em")?;
    let mut reports: Vec<String> = data
        .iter()
        .filter_map(|item| {
            item.get("REPORT")
                .and_then(|s| s.as_str())
                .map(|s| s.trim().to_string())
        })
        .collect();
    reports.sort();
    reports.dedup();
    let keep: Vec<String> = match indicator {
        "年报" => reports.into_iter().filter(|r| r.contains("FY")).collect(),
        "单季报" => reports
            .into_iter()
            .filter(|r| ["Q1", "Q2", "Q3", "Q4"].iter().any(|&q| r.contains(q)))
            .collect(),
        "累计季报" => reports
            .into_iter()
            .filter(|r| r.contains("Q6") || r.contains("Q9"))
            .collect(),
        other => {
            return Err(Error::InvalidParam(format!(
                "stock_financial_us_report_em: unknown indicator {other:?}"
            )));
        }
    };
    let mut sorted: Vec<String> = keep;
    sorted.sort_by(|a, b| {
        let ka = a.split('/').next().unwrap_or(a).trim();
        let kb = b.split('/').next().unwrap_or(b).trim();
        kb.cmp(ka)
    });
    let tuple = sorted
        .iter()
        .map(|r| format!("\"{r}\""))
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!("({tuple})"))
}

/// Port of `stock_financial_us_report_em(stock, symbol, indicator)`.
///
/// `symbol` ∈ {"资产负债表", "综合损益表", "现金流量表"};
/// `indicator` ∈ {"年报", "单季报", "累计季报"}.
pub async fn stock_financial_us_report_em(
    client: &Client,
    stock: &str,
    symbol: &str,
    indicator: &str,
) -> Result<Vec<FinanceUsReportRow>> {
    let secucode = us_secucode(client, stock).await?;
    let date_str = us_report_dates(client, &secucode, symbol, indicator).await?;
    let report_name = us_report_name(symbol)?;
    let filter = format!(r#"(SECUCODE="{secucode}")(REPORT in {date_str})"#);
    let params = [
        ("reportName", report_name),
        (
            "columns",
            "SECUCODE,SECURITY_CODE,SECURITY_NAME_ABBR,REPORT_DATE,REPORT_TYPE,REPORT,\
STD_ITEM_CODE,AMOUNT,ITEM_NAME",
        ),
        ("quoteColumns", ""),
        ("filter", filter.as_str()),
        ("pageNumber", ""),
        ("pageSize", ""),
        ("sortTypes", "1,-1"),
        ("sortColumns", "STD_ITEM_CODE,REPORT_DATE"),
        ("source", "SECURITIES"),
        ("client", "PC"),
        ("v", "09583551779242467"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_financial_us_report_em",
            EM_DATACENTER,
            &params,
        )
        .await?;
    parse_stock_financial_us_report_em(&v)
}

// ---------------------------------------------------------------------------
// Eastmoney — US main indicators (`stock_financial_us_analysis_indicator_em`)
// ---------------------------------------------------------------------------

/// One US main-financial-indicator row (explicit-column `IMAININDICATOR` shape,
/// used for symbols containing `_`, e.g. BRK_A / BRK_B).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FinanceUsIndicatorRow {
    /// `ORG_CODE`.
    pub org_code: Option<String>,
    /// `SECURITY_CODE`.
    pub security_code: String,
    /// `SECUCODE`.
    pub secucode: String,
    /// `SECURITY_NAME_ABBR`.
    pub security_name: String,
    /// `SECURITY_INNER_CODE`.
    pub security_inner_code: Option<String>,
    /// `STD_REPORT_DATE`.
    pub std_report_date: Option<String>,
    /// `REPORT_DATE`.
    pub report_date: String,
    /// `DATE_TYPE`.
    pub date_type: Option<String>,
    /// `DATE_TYPE_CODE`.
    pub date_type_code: Option<String>,
    /// `REPORT_TYPE`.
    pub report_type: Option<String>,
    /// `REPORT_DATA_TYPE`.
    pub report_data_type: Option<String>,
    /// `FISCAL_YEAR`.
    pub fiscal_year: Option<String>,
    /// `START_DATE`.
    pub start_date: Option<String>,
    /// `NOTICE_DATE`.
    pub notice_date: Option<String>,
    /// `ACCOUNT_STANDARD`.
    pub account_standard: Option<String>,
    /// `ACCOUNT_STANDARD_NAME`.
    pub account_standard_name: Option<String>,
    /// `CURRENCY`.
    pub currency: Option<String>,
    /// `CURRENCY_NAME`.
    pub currency_name: Option<String>,
    /// `ORGTYPE`.
    pub orgtype: Option<String>,
    /// `TOTAL_INCOME`.
    pub total_income: Option<f64>,
    /// `TOTAL_INCOME_YOY`.
    pub total_income_yoy: Option<f64>,
    /// `PREMIUM_INCOME`.
    pub premium_income: Option<f64>,
    /// `PREMIUM_INCOME_YOY`.
    pub premium_income_yoy: Option<f64>,
    /// `PARENT_HOLDER_NETPROFIT`.
    pub parent_holder_netprofit: Option<f64>,
    /// `PARENT_HOLDER_NETPROFIT_YOY`.
    pub parent_holder_netprofit_yoy: Option<f64>,
    /// `BASIC_EPS_CS`.
    pub basic_eps_cs: Option<f64>,
    /// `BASIC_EPS_CS_YOY`.
    pub basic_eps_cs_yoy: Option<f64>,
    /// `DILUTED_EPS_CS`.
    pub diluted_eps_cs: Option<f64>,
    /// `PAYOUT_RATIO`.
    pub payout_ratio: Option<f64>,
    /// `CAPITIAL_RATIO`.
    pub capitical_ratio: Option<f64>,
    /// `ROE`.
    pub roe: Option<f64>,
    /// `ROE_YOY`.
    pub roe_yoy: Option<f64>,
    /// `ROA`.
    pub roa: Option<f64>,
    /// `ROA_YOY`.
    pub roa_yoy: Option<f64>,
    /// `DEBT_RATIO`.
    pub debt_ratio: Option<f64>,
    /// `DEBT_RATIO_YOY`.
    pub debt_ratio_yoy: Option<f64>,
    /// `EQUITY_RATIO`.
    pub equity_ratio: Option<f64>,
    /// Data source (`eastmoney`).
    pub source: &'static str,
}

/// Parse a US `result.data` array into [`FinanceUsIndicatorRow`]s.
pub(crate) fn parse_stock_financial_us_analysis_indicator_em(
    resp: &Value,
) -> Result<Vec<FinanceUsIndicatorRow>> {
    let data = em_data(resp, "stock_financial_us_analysis_indicator_em")?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(FinanceUsIndicatorRow {
            org_code: str_of(item.get("ORG_CODE")),
            security_code: fstr_of(item.get("SECURITY_CODE")),
            secucode: fstr_of(item.get("SECUCODE")),
            security_name: fstr_of(item.get("SECURITY_NAME_ABBR")),
            security_inner_code: str_of(item.get("SECURITY_INNER_CODE")),
            std_report_date: str_of(item.get("STD_REPORT_DATE")),
            report_date: fstr_of(item.get("REPORT_DATE")),
            date_type: str_of(item.get("DATE_TYPE")),
            date_type_code: str_of(item.get("DATE_TYPE_CODE")),
            report_type: str_of(item.get("REPORT_TYPE")),
            report_data_type: str_of(item.get("REPORT_DATA_TYPE")),
            fiscal_year: str_of(item.get("FISCAL_YEAR")),
            start_date: str_of(item.get("START_DATE")),
            notice_date: str_of(item.get("NOTICE_DATE")),
            account_standard: str_of(item.get("ACCOUNT_STANDARD")),
            account_standard_name: str_of(item.get("ACCOUNT_STANDARD_NAME")),
            currency: str_of(item.get("CURRENCY")),
            currency_name: str_of(item.get("CURRENCY_NAME")),
            orgtype: str_of(item.get("ORGTYPE")),
            total_income: fnum_of(item.get("TOTAL_INCOME")),
            total_income_yoy: fnum_of(item.get("TOTAL_INCOME_YOY")),
            premium_income: fnum_of(item.get("PREMIUM_INCOME")),
            premium_income_yoy: fnum_of(item.get("PREMIUM_INCOME_YOY")),
            parent_holder_netprofit: fnum_of(item.get("PARENT_HOLDER_NETPROFIT")),
            parent_holder_netprofit_yoy: fnum_of(item.get("PARENT_HOLDER_NETPROFIT_YOY")),
            basic_eps_cs: fnum_of(item.get("BASIC_EPS_CS")),
            basic_eps_cs_yoy: fnum_of(item.get("BASIC_EPS_CS_YOY")),
            diluted_eps_cs: fnum_of(item.get("DILUTED_EPS_CS")),
            payout_ratio: fnum_of(item.get("PAYOUT_RATIO")),
            capitical_ratio: fnum_of(item.get("CAPITIAL_RATIO")),
            roe: fnum_of(item.get("ROE")),
            roe_yoy: fnum_of(item.get("ROE_YOY")),
            roa: fnum_of(item.get("ROA")),
            roa_yoy: fnum_of(item.get("ROA_YOY")),
            debt_ratio: fnum_of(item.get("DEBT_RATIO")),
            debt_ratio_yoy: fnum_of(item.get("DEBT_RATIO_YOY")),
            equity_ratio: fnum_of(item.get("EQUITY_RATIO")),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

/// Port of `stock_financial_us_analysis_indicator_em(symbol, indicator)`.
///
/// `indicator` ∈ {"年报", "单季报", "累计季报"}. The report name switches to the
/// explicit-column `IMAININDICATOR` for symbols that contain `_` (e.g. BRK_A).
pub async fn stock_financial_us_analysis_indicator_em(
    client: &Client,
    symbol: &str,
    indicator: &str,
) -> Result<Vec<FinanceUsIndicatorRow>> {
    let secucode = us_secucode(client, symbol).await?;
    let (report_name, columns) = if secucode.contains('_') {
        (
            "RPT_USF10_FN_IMAININDICATOR",
            "ORG_CODE,SECURITY_CODE,SECUCODE,SECURITY_NAME_ABBR,SECURITY_INNER_CODE,\
STD_REPORT_DATE,REPORT_DATE,DATE_TYPE,DATE_TYPE_CODE,REPORT_TYPE,REPORT_DATA_TYPE,\
FISCAL_YEAR,START_DATE,NOTICE_DATE,ACCOUNT_STANDARD,ACCOUNT_STANDARD_NAME,CURRENCY,\
CURRENCY_NAME,ORGTYPE,TOTAL_INCOME,TOTAL_INCOME_YOY,PREMIUM_INCOME,PREMIUM_INCOME_YOY,\
PARENT_HOLDER_NETPROFIT,PARENT_HOLDER_NETPROFIT_YOY,BASIC_EPS_CS,BASIC_EPS_CS_YOY,\
DILUTED_EPS_CS,PAYOUT_RATIO,CAPITIAL_RATIO,ROE,ROE_YOY,ROA,ROA_YOY,DEBT_RATIO,\
DEBT_RATIO_YOY,EQUITY_RATIO",
        )
    } else {
        ("RPT_USF10_FN_GMAININDICATOR", "USF10_FN_GMAININDICATOR")
    };
    let filter = match indicator {
        "年报" => format!(r#"(SECUCODE="{secucode}")(DATE_TYPE_CODE="001")"#),
        "单季报" => {
            format!(r#"(SECUCODE="{secucode}")(DATE_TYPE_CODE in ("003","006","007","008"))"#)
        }
        "累计季报" => format!(r#"(SECUCODE="{secucode}")(DATE_TYPE_CODE in ("002","004"))"#),
        other => {
            return Err(Error::InvalidParam(format!(
                "stock_financial_us_analysis_indicator_em: unknown indicator {other:?}"
            )));
        }
    };
    let params = [
        ("reportName", report_name),
        ("columns", columns),
        ("quoteColumns", ""),
        ("pageNumber", ""),
        ("pageSize", ""),
        ("sortTypes", "-1"),
        ("sortColumns", "REPORT_DATE"),
        ("source", "SECURITIES"),
        ("client", "PC"),
        ("filter", filter.as_str()),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_financial_us_analysis_indicator_em",
            EM_DATACENTER,
            &params,
        )
        .await?;
    parse_stock_financial_us_analysis_indicator_em(&v)
}

// ===========================================================================
// Tests — offline, against fixtures in tests/fixtures/
// ===========================================================================

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
    fn parses_stock_financial_debt_ths() {
        let rows = parse_ths(&fixture("stock_financial_debt_ths.json"), "report").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].item, "货币资金");
        assert_eq!(rows[0].date, Some("2024-03-31".to_string()));
        assert_eq!(rows[0].value, Some(100.5));
        assert_eq!(rows[0].source, "ths");
        // second item has a null amount in the fixture
        assert_eq!(rows[1].item, "应收账款");
        assert_eq!(rows[1].value, None);
    }

    #[test]
    fn parses_stock_financial_benefit_ths() {
        let rows = parse_ths(&fixture("stock_financial_benefit_ths.json"), "report").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].item, "营业收入");
        assert_eq!(rows[0].value, Some(2000.0));
        assert_eq!(rows[1].item, "营业利润");
        assert_eq!(rows[1].value, None);
    }

    #[test]
    fn parses_stock_financial_cash_ths() {
        let rows = parse_ths(&fixture("stock_financial_cash_ths.json"), "report").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].item, "经营活动现金流");
        assert_eq!(rows[0].value, Some(300.0));
        assert_eq!(rows[1].value, None);
    }

    /// Find a `*_new` row by metric name (object key order is not stable).
    fn find_new<'a>(rows: &'a [FinanceThsNewRow], metric: &str) -> &'a FinanceThsNewRow {
        rows.iter()
            .find(|r| r.metric_name == metric)
            .unwrap_or_else(|| panic!("metric {metric:?} not found"))
    }

    #[test]
    fn parses_stock_financial_abstract_new_ths() {
        let rows = parse_ths_new(&fixture("stock_financial_abstract_new_ths.json")).unwrap();
        assert_eq!(rows.len(), 2);
        // order of index_list keys is not guaranteed; look up by metric name
        let profit = find_new(&rows, "盈利能力");
        assert_eq!(profit.report_date, "2024-03-31");
        assert_eq!(profit.field, Some("ROE".to_string()));
        assert_eq!(profit.value, None);
        let growth = find_new(&rows, "成长能力");
        assert_eq!(growth.field, None);
        assert_eq!(growth.value, Some(8.3));
    }

    #[test]
    fn parses_stock_financial_debt_new_ths() {
        let rows = parse_ths_new(&fixture("stock_financial_debt_new_ths.json")).unwrap();
        assert_eq!(rows.len(), 2);
        let solvency = find_new(&rows, "偿债能力");
        assert_eq!(solvency.field, Some("资产负债率".to_string()));
        assert_eq!(solvency.value, Some(45.0));
        assert_eq!(find_new(&rows, "运营能力").value, None);
    }

    #[test]
    fn parses_stock_financial_benefit_new_ths() {
        let rows = parse_ths_new(&fixture("stock_financial_benefit_new_ths.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(find_new(&rows, "盈利能力").value, Some(12.5));
        assert_eq!(find_new(&rows, "成长能力").value, None);
    }

    #[test]
    fn parses_stock_financial_cash_new_ths() {
        let rows = parse_ths_new(&fixture("stock_financial_cash_new_ths.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(find_new(&rows, "现金流").value, Some(99.0));
        assert_eq!(find_new(&rows, "投资").value, None);
    }

    #[test]
    fn parses_stock_financial_hk_report_em() {
        let rows =
            parse_stock_financial_hk_report_em(&fixture("stock_financial_hk_report_em.json"))
                .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].secucode, "00700.HK");
        assert_eq!(rows[0].std_item_name, "货币资金");
        assert_eq!(rows[0].amount, Some(123.0));
        assert_eq!(rows[0].std_report_date, Some("2023-12-31".to_string()));
        assert_eq!(rows[1].amount, None);
    }

    #[test]
    fn parses_stock_financial_hk_analysis_indicator_em() {
        let rows = parse_stock_financial_hk_analysis_indicator_em(&fixture(
            "stock_financial_hk_analysis_indicator_em.json",
        ))
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].secucode, "00700.HK");
        assert_eq!(rows[0].total_income, Some(5000.0));
        assert_eq!(rows[0].roe, Some(15.0));
        assert_eq!(rows[1].roe, None);
    }

    #[test]
    fn parses_stock_financial_us_report_em() {
        let rows =
            parse_stock_financial_us_report_em(&fixture("stock_financial_us_report_em.json"))
                .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].secucode, "TSLA.O");
        assert_eq!(rows[0].item_name, "Total Assets");
        assert_eq!(rows[0].amount, Some(70000.0));
        assert_eq!(rows[1].amount, None);
    }

    #[test]
    fn parses_stock_financial_us_analysis_indicator_em() {
        let rows = parse_stock_financial_us_analysis_indicator_em(&fixture(
            "stock_financial_us_analysis_indicator_em.json",
        ))
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].secucode, "BRK_A");
        assert_eq!(rows[0].total_income, Some(350000.0));
        assert_eq!(rows[0].roe, Some(10.0));
        assert_eq!(rows[1].roe, None);
    }
}
