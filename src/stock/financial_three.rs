//! 东方财富-股票-财务分析-三大财务报表 (akshare
//! `stock_feature/stock_three_report_em.py`).
//!
//! Ported public functions. These are Eastmoney `emweb` / `datacenter` AJAX JSON
//! endpoints (pure HTTP, no JS/token/signature). Each returns the financial
//! statement for a single security (`symbol`, with market prefix such as
//! `SH600036` / `SZ000013`), normalized to one row per
//! `(item, report_date)`:
//!
//! | Rust fn                                          | akshare fn                                      | akshare line | endpoint                          |
//! |--------------------------------------------------|-------------------------------------------------|--------------|-----------------------------------|
//! | `stock_balance_sheet_by_yearly_em`               | `stock_balance_sheet_by_yearly_em`              | `:84`        | emweb `zcfzb` (按年度)            |
//! | `stock_profit_sheet_by_yearly_em`               | `stock_profit_sheet_by_yearly_em`              | `:191`       | emweb `lrb`  (按年度)            |
//! | `stock_profit_sheet_by_quarterly_em`            | `stock_profit_sheet_by_quarterly_em`           | `:240`       | emweb `lrb`  (按单季度)          |
//! | `stock_cash_flow_sheet_by_yearly_em`            | `stock_cash_flow_sheet_by_yearly_em`           | `:342`       | emweb `xjllb` (按年度)           |
//! | `stock_cash_flow_sheet_by_quarterly_em`         | `stock_cash_flow_sheet_by_quarterly_em`        | `:393`       | emweb `xjllb` (按单季度)         |
//! | `stock_balance_sheet_by_report_delisted_em`     | `stock_balance_sheet_by_report_delisted_em`    | `:474`       | datacenter `RPT_F10_FINANCE_GBALANCE` |
//! | `stock_profit_sheet_by_report_delisted_em`      | `stock_profit_sheet_by_report_delisted_em`     | `:507`       | datacenter `RPT_F10_FINANCE_GINCOME` |
//! | `stock_cash_flow_sheet_by_report_delisted_em`   | `stock_cash_flow_sheet_by_report_delisted_em`  | `:540`       | datacenter `RPT_F10_FINANCE_GCASHFLOW` |
//!
//! ## Normalized shape
//!
//! The upstream `emweb` `AjaxNew` / datacenter responses embed the line items as
//! *rows* and the reporting periods as *dynamic columns* (one column per
//! `YYYY-MM-DD` date). To make the dynamic date columns queryable we normalize to
//! one row per `(item, report_date)`:
//!
//! ```text
//! ThreeReportRow { item, report_date, value, source }
//! ```
//!
//! `value` is `None` when the upstream cell is missing / `null` / an empty
//! string, so a `None` row is a first-class, lossless representation of a blank
//! cell.
//!
//! ## DEFERRED
//!
//! The private `_stock_balance_sheet_by_report_ctype_em` helper (akshare
//! `stock_three_report_em.py:18`) scrapes a `hidctype` value out of an emweb
//! **HTML** page to learn a security's `companyType`. It is HTML parsing, so it
//! is **deferred** and not ported here. The five non-delisted `emweb` functions
//! below therefore default `companyType` to `"3"` (matching akshare's own
//! yearly-report `except` fallback) and are fully functional for that default;
//! the lead can later wire the HTML helper and pass a resolved `companyType` if
//! broader coverage is needed. The three non-delisted `*_by_report_em`
//! variants (akshare `:35`/`:142`/`:291`) are also out of scope here because
//! they depend on the same deferred HTML helper.
//!
//! ## Field-key fidelity note
//!
//! akshare reads these reports positionally (a fixed Chinese column-label list),
//! so the real upstream field keys are not recoverable from the akshare source.
//! The `REPORT_ITEM` key below is an **inferred** Eastmoney emweb line-item
//! field; date columns are the `YYYY-MM-DD` report-period keys. These must be
//! verified against a live sample before production use (same convention as
//! `financial.rs` / `gdfx.rs`).

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// Eastmoney source bucket, for rate limiting / error context.
const SOURCE_EASTMONEY: &str = "eastmoney";

/// Eastmoney `emweb` `PC_HSF10/NewFinanceAnalysis` base (the non-delisted fns)
/// is `https://emweb.securities.eastmoney.com/PC_HSF10/NewFinanceAnalysis`;
/// the per-endpoint URLs are inlined as literals below (Rust `concat!` requires
/// literal operands).
/// Eastmoney `datacenter` base (the delisted fns).
const DATACENTER_BASE: &str = "https://datacenter.eastmoney.com/securities/api/data/get";

/// Default `companyType` used by the emweb fns (see module DEFERRED note).
const DEFAULT_COMPANY_TYPE: &str = "3";

/// Inferred Eastmoney emweb line-item name field (see module fidelity note).
const ITEM_KEY: &str = "REPORT_ITEM";

/// `zcfzb` (资产负债表) date-list + statement AJAX URLs.
const ZCFZB_DATE_URL: &str =
    "https://emweb.securities.eastmoney.com/PC_HSF10/NewFinanceAnalysis/zcfzbDateAjaxNew";
const ZCFZB_AJAX_URL: &str =
    "https://emweb.securities.eastmoney.com/PC_HSF10/NewFinanceAnalysis/zcfzbAjaxNew";

/// `lrb` (利润表) date-list + statement AJAX URLs.
const LRB_DATE_URL: &str =
    "https://emweb.securities.eastmoney.com/PC_HSF10/NewFinanceAnalysis/lrbDateAjaxNew";
const LRB_AJAX_URL: &str =
    "https://emweb.securities.eastmoney.com/PC_HSF10/NewFinanceAnalysis/lrbAjaxNew";

/// `xjllb` (现金流量表) date-list + statement AJAX URLs.
const XJLLB_DATE_URL: &str =
    "https://emweb.securities.eastmoney.com/PC_HSF10/NewFinanceAnalysis/xjllbDateAjaxNew";
const XJLLB_AJAX_URL: &str =
    "https://emweb.securities.eastmoney.com/PC_HSF10/NewFinanceAnalysis/xjllbAjaxNew";

// ---------------------------------------------------------------------------
// Helpers (mirrors financial.rs / gdfx.rs conventions)
// ---------------------------------------------------------------------------

/// Read a string field, returning `None` when missing/null.
fn fstr(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).map(|s| s.to_string())
}

/// Read a numeric field that may be a JSON number or a plain numeric string.
fn num_or_none(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() || t.eq_ignore_ascii_case("null") {
                None
            } else {
                t.parse::<f64>().ok()
            }
        }
        Value::Null => None,
        _ => None,
    }
}

/// True for `YYYY-MM-DD` date-column keys (the dynamic reporting-period columns).
fn is_date_key(k: &str) -> bool {
    k.len() == 10
        && k.as_bytes()[4] == b'-'
        && k.as_bytes()[7] == b'-'
        && k.bytes().all(|b| b.is_ascii_digit() || b == b'-')
}

/// Extract the `data` array from an emweb `AjaxNew` / date-list response.
fn emweb_data(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "emweb response missing `data`".into(),
        })
}

/// Extract the `result.data` array from a datacenter response.
fn datacenter_data(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("result")
        .and_then(|r| r.get("data"))
        .and_then(Value::as_array)
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "datacenter response missing `result.data`".into(),
        })
}

/// Collect the `REPORT_DATE` strings from an emweb date-list response
/// (`zcfzbDateAjaxNew` / `lrbDateAjaxNew` / `xjllbDateAjaxNew`). Eastmoney may
/// return a full timestamp (`2024-12-31 00:00:00`); keep only the date part.
fn collect_report_dates(resp: &Value) -> Result<Vec<String>> {
    let arr = emweb_data(resp)?;
    let mut out = Vec::new();
    for item in arr {
        if let Some(d) = item.get("REPORT_DATE").and_then(Value::as_str) {
            out.push(d.split(' ').next().unwrap_or(d).to_string());
        }
    }
    Ok(out)
}

/// Collect dated `'YYYY-MM-DD'` literals from a delisted date-list response
/// (`RPT_F10_FINANCE_GINCOME`), ready to drop into a `REPORT_DATE in (...)`
/// filter (mirrors akshare's `"'" + date + "'"` quoting).
fn collect_delisted_dates(resp: &Value) -> Result<Vec<String>> {
    let arr = datacenter_data(resp)?;
    let mut out = Vec::new();
    for item in arr {
        if let Some(d) = item.get("REPORT_DATE").and_then(Value::as_str) {
            let date = d.split(' ').next().unwrap_or(d);
            out.push(format!("'{date}'"));
        }
    }
    Ok(out)
}

/// Normalize a statement `data` array (line items as rows, report periods as
/// `YYYY-MM-DD` columns) into one [`ThreeReportRow`] per `(item, report_date)`.
fn normalize_report_rows(items: &[Value]) -> Vec<ThreeReportRow> {
    let mut out = Vec::new();
    for item in items {
        let name = match fstr(item, ITEM_KEY) {
            Some(n) if !n.is_empty() => n,
            _ => continue,
        };
        if let Some(obj) = item.as_object() {
            for (k, v) in obj {
                if k == ITEM_KEY || !is_date_key(k) {
                    continue;
                }
                out.push(ThreeReportRow {
                    item: name.clone(),
                    report_date: k.clone(),
                    value: num_or_none(v),
                    source: SOURCE_EASTMONEY,
                });
            }
        }
    }
    out
}

/// Fetch an emweb statement: first resolve the reporting-period list, then pull
/// the statement in chunks of 5 dates (Eastmoney's `dates` param caps at 5).
#[allow(clippy::too_many_arguments)]
async fn fetch_emweb_statement(
    client: &Client,
    endpoint: &'static str,
    symbol: &str,
    date_list_url: &str,
    ajax_url: &str,
    date_list_rdt: &str,
    ajax_rdt: &str,
    report_type: &str,
) -> Result<Value> {
    let company_type = DEFAULT_COMPANY_TYPE;
    // 1) reporting-period list.
    let dl = client
        .get_json(
            SOURCE_EASTMONEY,
            endpoint,
            date_list_url,
            &[
                ("companyType", company_type),
                ("reportDateType", date_list_rdt),
                ("code", symbol),
            ],
        )
        .await?;
    let dates = collect_report_dates(&dl)?;
    if dates.is_empty() {
        return Ok(serde_json::json!({ "data": [] }));
    }
    // 2) fetch each 5-date chunk and concatenate the line-item rows.
    let mut all = Vec::new();
    for chunk in dates.chunks(5) {
        let dates_param = chunk.join(",");
        let v = client
            .get_json(
                SOURCE_EASTMONEY,
                endpoint,
                ajax_url,
                &[
                    ("companyType", company_type),
                    ("reportDateType", ajax_rdt),
                    ("reportType", report_type),
                    ("dates", &dates_param),
                    ("code", symbol),
                ],
            )
            .await?;
        match v.get("data").and_then(Value::as_array) {
            Some(arr) if !arr.is_empty() => all.extend(arr.iter().cloned()),
            _ => break,
        }
    }
    Ok(serde_json::json!({ "data": all }))
}

/// Fetch a delisted statement: resolve the reporting-period list via the shared
/// `RPT_F10_FINANCE_GINCOME` endpoint, then pull the statement filtered to those
/// dates.
async fn fetch_delisted_statement(
    client: &Client,
    endpoint: &'static str,
    symbol: &str,
    type_: &str,
    sty: &str,
    v: &str,
) -> Result<Value> {
    if symbol.len() < 2 {
        return Err(Error::InvalidParam(format!(
            "symbol must include a market prefix (e.g. SZ000013), got {symbol:?}"
        )));
    }
    let secucode = format!("{}.{}", &symbol[2..], &symbol[..2]);
    // 1) reporting-period list (shared GINCOME endpoint).
    let dl = client
        .get_json(
            SOURCE_EASTMONEY,
            endpoint,
            DATACENTER_BASE,
            &[
                ("type", "RPT_F10_FINANCE_GINCOME"),
                (
                    "sty",
                    "SECUCODE,SECURITY_CODE,REPORT_DATE,REPORT_TYPE,REPORT_DATE_NAME",
                ),
                ("filter", &format!("(SECUCODE=\"{secucode}\")")),
                ("p", "1"),
                ("ps", "200"),
                ("sr", "-1"),
                ("st", "REPORT_DATE"),
                ("source", "HSF10"),
                ("client", "PC"),
                ("v", "07306678536291241"),
            ],
        )
        .await?;
    let dates = collect_delisted_dates(&dl)?;
    if dates.is_empty() {
        return Ok(serde_json::json!({ "result": { "data": [] } }));
    }
    // 2) statement, filtered to the resolved dates.
    let filter = format!("(SECUCODE=\"{secucode}\")(REPORT_DATE in ({}))", dates.join(","));
    client
        .get_json(
            SOURCE_EASTMONEY,
            endpoint,
            DATACENTER_BASE,
            &[
                ("type", type_),
                ("sty", sty),
                ("filter", &filter),
                ("p", "1"),
                ("ps", "200"),
                ("sr", "-1"),
                ("st", "REPORT_DATE"),
                ("source", "HSF10"),
                ("client", "PC"),
                ("v", v),
            ],
        )
        .await
}

// ---------------------------------------------------------------------------
// Normalized row
// ---------------------------------------------------------------------------

/// One normalized financial-statement cell: a single line item (`item`) at a
/// reporting period (`report_date`) with its numeric `value` (or `None` when
/// missing / `null` / blank).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ThreeReportRow {
    /// Line-item label (e.g. `货币资金` / `营业收入`)
    pub item: String,
    /// Reporting period, `YYYY-MM-DD` (a dynamic upstream date column)
    pub report_date: String,
    /// The line-item value, or `None` when missing / `null` / blank
    pub value: Option<f64>,
    pub source: &'static str,
}

// ===========================================================================
// stock_balance_sheet_by_yearly_em — 资产负债表 (按年度)
// ===========================================================================

/// Port of `stock_balance_sheet_by_yearly_em(symbol="SH600036")`.
///
/// Eastmoney `emweb` `zcfzb` (资产负债表), annual periods (`reportDateType=1`).
/// `companyType` defaults to `"3"` because the upstream HTML `companyType`
/// resolver is deferred (see module DEFERRED note).
pub async fn stock_balance_sheet_by_yearly_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<ThreeReportRow>> {
    let resp = fetch_emweb_statement(
        client,
        "stock_balance_sheet_by_yearly_em",
        symbol,
        ZCFZB_DATE_URL,
        ZCFZB_AJAX_URL,
        "1",
        "1",
        "1",
    )
    .await?;
    parse_stock_balance_sheet_by_yearly_em(&resp)
}

/// Parse an emweb `zcfzb` yearly `data` array into [`ThreeReportRow`]s.
pub(crate) fn parse_stock_balance_sheet_by_yearly_em(resp: &Value) -> Result<Vec<ThreeReportRow>> {
    Ok(normalize_report_rows(emweb_data(resp)?))
}

// ===========================================================================
// stock_profit_sheet_by_yearly_em — 利润表 (按年度)
// ===========================================================================

/// Port of `stock_profit_sheet_by_yearly_em(symbol="SH600519")`.
///
/// Eastmoney `emweb` `lrb` (利润表), annual periods (`reportDateType=1`).
pub async fn stock_profit_sheet_by_yearly_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<ThreeReportRow>> {
    let resp = fetch_emweb_statement(
        client,
        "stock_profit_sheet_by_yearly_em",
        symbol,
        LRB_DATE_URL,
        LRB_AJAX_URL,
        "1",
        "1",
        "1",
    )
    .await?;
    parse_stock_profit_sheet_by_yearly_em(&resp)
}

/// Parse an emweb `lrb` yearly `data` array into [`ThreeReportRow`]s.
pub(crate) fn parse_stock_profit_sheet_by_yearly_em(resp: &Value) -> Result<Vec<ThreeReportRow>> {
    Ok(normalize_report_rows(emweb_data(resp)?))
}

// ===========================================================================
// stock_profit_sheet_by_quarterly_em — 利润表 (按单季度)
// ===========================================================================

/// Port of `stock_profit_sheet_by_quarterly_em(symbol="SH600519")`.
///
/// Eastmoney `emweb` `lrb` (利润表), single-quarter periods (`date-list
/// reportDateType=2`, statement `reportDateType=0`, `reportType=2`).
pub async fn stock_profit_sheet_by_quarterly_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<ThreeReportRow>> {
    let resp = fetch_emweb_statement(
        client,
        "stock_profit_sheet_by_quarterly_em",
        symbol,
        LRB_DATE_URL,
        LRB_AJAX_URL,
        "2",
        "0",
        "2",
    )
    .await?;
    parse_stock_profit_sheet_by_quarterly_em(&resp)
}

/// Parse an emweb `lrb` quarterly `data` array into [`ThreeReportRow`]s.
pub(crate) fn parse_stock_profit_sheet_by_quarterly_em(
    resp: &Value,
) -> Result<Vec<ThreeReportRow>> {
    Ok(normalize_report_rows(emweb_data(resp)?))
}

// ===========================================================================
// stock_cash_flow_sheet_by_yearly_em — 现金流量表 (按年度)
// ===========================================================================

/// Port of `stock_cash_flow_sheet_by_yearly_em(symbol="SH600519")`.
///
/// Eastmoney `emweb` `xjllb` (现金流量表), annual periods (`reportDateType=1`).
pub async fn stock_cash_flow_sheet_by_yearly_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<ThreeReportRow>> {
    let resp = fetch_emweb_statement(
        client,
        "stock_cash_flow_sheet_by_yearly_em",
        symbol,
        XJLLB_DATE_URL,
        XJLLB_AJAX_URL,
        "1",
        "1",
        "1",
    )
    .await?;
    parse_stock_cash_flow_sheet_by_yearly_em(&resp)
}

/// Parse an emweb `xjllb` yearly `data` array into [`ThreeReportRow`]s.
pub(crate) fn parse_stock_cash_flow_sheet_by_yearly_em(
    resp: &Value,
) -> Result<Vec<ThreeReportRow>> {
    Ok(normalize_report_rows(emweb_data(resp)?))
}

// ===========================================================================
// stock_cash_flow_sheet_by_quarterly_em — 现金流量表 (按单季度)
// ===========================================================================

/// Port of `stock_cash_flow_sheet_by_quarterly_em(symbol="SH600519")`.
///
/// Eastmoney `emweb` `xjllb` (现金流量表), single-quarter periods (`date-list
/// reportDateType=2`, statement `reportDateType=0`, `reportType=2`).
pub async fn stock_cash_flow_sheet_by_quarterly_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<ThreeReportRow>> {
    let resp = fetch_emweb_statement(
        client,
        "stock_cash_flow_sheet_by_quarterly_em",
        symbol,
        XJLLB_DATE_URL,
        XJLLB_AJAX_URL,
        "2",
        "0",
        "2",
    )
    .await?;
    parse_stock_cash_flow_sheet_by_quarterly_em(&resp)
}

/// Parse an emweb `xjllb` quarterly `data` array into [`ThreeReportRow`]s.
pub(crate) fn parse_stock_cash_flow_sheet_by_quarterly_em(
    resp: &Value,
) -> Result<Vec<ThreeReportRow>> {
    Ok(normalize_report_rows(emweb_data(resp)?))
}

// ===========================================================================
// stock_balance_sheet_by_report_delisted_em — 资产负债表 (已退市, 按报告期)
// ===========================================================================

/// Port of `stock_balance_sheet_by_report_delisted_em(symbol="SZ000013")`.
///
/// Eastmoney `datacenter` `RPT_F10_FINANCE_GBALANCE` for a delisted security.
pub async fn stock_balance_sheet_by_report_delisted_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<ThreeReportRow>> {
    let resp = fetch_delisted_statement(
        client,
        "stock_balance_sheet_by_report_delisted_em",
        symbol,
        "RPT_F10_FINANCE_GBALANCE",
        "F10_FINANCE_GBALANCE",
        "05767841728614413",
    )
    .await?;
    parse_stock_balance_sheet_by_report_delisted_em(&resp)
}

/// Parse a delisted `zcfzb` `result.data` array into [`ThreeReportRow`]s.
pub(crate) fn parse_stock_balance_sheet_by_report_delisted_em(
    resp: &Value,
) -> Result<Vec<ThreeReportRow>> {
    Ok(normalize_report_rows(datacenter_data(resp)?))
}

// ===========================================================================
// stock_profit_sheet_by_report_delisted_em — 利润表 (已退市, 按报告期)
// ===========================================================================

/// Port of `stock_profit_sheet_by_report_delisted_em(symbol="SZ000013")`.
///
/// Eastmoney `datacenter` `RPT_F10_FINANCE_GINCOME` for a delisted security.
pub async fn stock_profit_sheet_by_report_delisted_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<ThreeReportRow>> {
    let resp = fetch_delisted_statement(
        client,
        "stock_profit_sheet_by_report_delisted_em",
        symbol,
        "RPT_F10_FINANCE_GINCOME",
        "APP_F10_GINCOME",
        "05767841728614413",
    )
    .await?;
    parse_stock_profit_sheet_by_report_delisted_em(&resp)
}

/// Parse a delisted `lrb` `result.data` array into [`ThreeReportRow`]s.
pub(crate) fn parse_stock_profit_sheet_by_report_delisted_em(
    resp: &Value,
) -> Result<Vec<ThreeReportRow>> {
    Ok(normalize_report_rows(datacenter_data(resp)?))
}

// ===========================================================================
// stock_cash_flow_sheet_by_report_delisted_em — 现金流量表 (已退市, 按报告期)
// ===========================================================================

/// Port of `stock_cash_flow_sheet_by_report_delisted_em(symbol="SZ000013")`.
///
/// Eastmoney `datacenter` `RPT_F10_FINANCE_GCASHFLOW` for a delisted security.
pub async fn stock_cash_flow_sheet_by_report_delisted_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<ThreeReportRow>> {
    let resp = fetch_delisted_statement(
        client,
        "stock_cash_flow_sheet_by_report_delisted_em",
        symbol,
        "RPT_F10_FINANCE_GCASHFLOW",
        "APP_F10_GCASHFLOW",
        "05767841728614413",
    )
    .await?;
    parse_stock_cash_flow_sheet_by_report_delisted_em(&resp)
}

/// Parse a delisted `xjllb` `result.data` array into [`ThreeReportRow`]s.
pub(crate) fn parse_stock_cash_flow_sheet_by_report_delisted_em(
    resp: &Value,
) -> Result<Vec<ThreeReportRow>> {
    Ok(normalize_report_rows(datacenter_data(resp)?))
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

    /// Find a single normalized row by item label + report date.
    fn find<'a>(rows: &'a [ThreeReportRow], item: &str, date: &str) -> &'a ThreeReportRow {
        rows.iter()
            .find(|r| r.item == item && r.report_date == date)
            .expect("expected row present")
    }

    #[test]
    fn parses_stock_balance_sheet_by_yearly_em() {
        let rows = parse_stock_balance_sheet_by_yearly_em(&fixture(
            "stock_balance_sheet_by_yearly_em.json",
        ))
        .unwrap();
        // 2 line items × 2 periods
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].source, "eastmoney");
        let m = find(&rows, "货币资金", "2024-12-31");
        assert_eq!(m.value, Some(68265740000.0));
        // None case: 应收账款 is null at 2024-12-31
        assert_eq!(find(&rows, "应收账款", "2024-12-31").value, None);
        assert_eq!(find(&rows, "应收账款", "2023-12-31").value, Some(1234567.0));
    }

    #[test]
    fn parses_stock_profit_sheet_by_yearly_em() {
        let rows = parse_stock_profit_sheet_by_yearly_em(&fixture(
            "stock_profit_sheet_by_yearly_em.json",
        ))
        .unwrap();
        assert_eq!(rows.len(), 4);
        let m = find(&rows, "营业收入", "2024-12-31");
        assert_eq!(m.value, Some(170899000000.0));
        assert_eq!(find(&rows, "净利润", "2024-12-31").value, None);
        assert_eq!(find(&rows, "净利润", "2023-12-31").value, Some(74734000000.0));
    }

    #[test]
    fn parses_stock_profit_sheet_by_quarterly_em() {
        let rows = parse_stock_profit_sheet_by_quarterly_em(&fixture(
            "stock_profit_sheet_by_quarterly_em.json",
        ))
        .unwrap();
        assert_eq!(rows.len(), 4);
        let m = find(&rows, "营业收入", "2024-03-31");
        assert_eq!(m.value, Some(46405000000.0));
        assert_eq!(find(&rows, "净利润", "2024-03-31").value, None);
        assert_eq!(find(&rows, "净利润", "2023-03-31").value, Some(20519000000.0));
    }

    #[test]
    fn parses_stock_cash_flow_sheet_by_yearly_em() {
        let rows = parse_stock_cash_flow_sheet_by_yearly_em(&fixture(
            "stock_cash_flow_sheet_by_yearly_em.json",
        ))
        .unwrap();
        assert_eq!(rows.len(), 4);
        let m = find(&rows, "经营活动现金流量净额", "2024-12-31");
        assert_eq!(m.value, Some(9123700000.0));
        assert_eq!(find(&rows, "投资活动现金流量净额", "2024-12-31").value, None);
        assert_eq!(
            find(&rows, "投资活动现金流量净额", "2023-12-31").value,
            Some(-3344556.0)
        );
    }

    #[test]
    fn parses_stock_cash_flow_sheet_by_quarterly_em() {
        let rows = parse_stock_cash_flow_sheet_by_quarterly_em(&fixture(
            "stock_cash_flow_sheet_by_quarterly_em.json",
        ))
        .unwrap();
        assert_eq!(rows.len(), 4);
        let m = find(&rows, "经营活动现金流量净额", "2024-03-31");
        assert_eq!(m.value, Some(1191200000.0));
        assert_eq!(find(&rows, "投资活动现金流量净额", "2024-03-31").value, None);
        assert_eq!(
            find(&rows, "投资活动现金流量净额", "2023-03-31").value,
            Some(-998870.0)
        );
    }

    #[test]
    fn parses_stock_balance_sheet_by_report_delisted_em() {
        let rows = parse_stock_balance_sheet_by_report_delisted_em(&fixture(
            "stock_balance_sheet_by_report_delisted_em.json",
        ))
        .unwrap();
        assert_eq!(rows.len(), 4);
        let m = find(&rows, "货币资金", "2023-12-31");
        assert_eq!(m.value, Some(5000000.0));
        assert_eq!(find(&rows, "应收账款", "2023-12-31").value, None);
        assert_eq!(find(&rows, "应收账款", "2022-12-31").value, Some(200000.0));
    }

    #[test]
    fn parses_stock_profit_sheet_by_report_delisted_em() {
        let rows = parse_stock_profit_sheet_by_report_delisted_em(&fixture(
            "stock_profit_sheet_by_report_delisted_em.json",
        ))
        .unwrap();
        assert_eq!(rows.len(), 4);
        let m = find(&rows, "营业收入", "2023-12-31");
        assert_eq!(m.value, Some(123456789.0));
        assert_eq!(find(&rows, "净利润", "2023-12-31").value, None);
        assert_eq!(find(&rows, "净利润", "2022-12-31").value, Some(1234567.0));
    }

    #[test]
    fn parses_stock_cash_flow_sheet_by_report_delisted_em() {
        let rows = parse_stock_cash_flow_sheet_by_report_delisted_em(&fixture(
            "stock_cash_flow_sheet_by_report_delisted_em.json",
        ))
        .unwrap();
        assert_eq!(rows.len(), 4);
        let m = find(&rows, "经营活动现金流量净额", "2023-12-31");
        assert_eq!(m.value, Some(987654.0));
        assert_eq!(find(&rows, "投资活动现金流量净额", "2023-12-31").value, None);
        assert_eq!(
            find(&rows, "投资活动现金流量净额", "2022-12-31").value,
            Some(-54321.0)
        );
    }
}
