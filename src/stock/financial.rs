//! 东方财富-数据中心-年报季报-三大财务报表 (akshare
//! `stock_feature/stock_report_em.py`).
//!
//! Ported public functions (all pure Eastmoney `datacenter-web` JSON, no
//! JS/token/signature). Each returns the market-wide financial-statement rows
//! for a single reporting period:
//!
//! | Rust fn                 | akshare fn              | reportName                | akshare line     |
//! |-------------------------|-------------------------|---------------------------|------------------|
//! | `stock_zcfz_em`         | `stock_zcfz_em`         | `RPT_DMSK_FN_BALANCE`     | `stock_report_em.py:20`  |
//! | `stock_zcfz_bj_em`      | `stock_zcfz_bj_em`      | `RPT_DMSK_FN_BALANCE`     | `stock_report_em.py:161` |
//! | `stock_lrb_em`          | `stock_lrb_em`          | `RPT_DMSK_FN_INCOME`      | `stock_report_em.py:302` |
//! | `stock_xjll_em`         | `stock_xjll_em`         | `RPT_DMSK_FN_CASHFLOW`    | `stock_report_em.py:438` |
//!
//! ## Normalized shape
//!
//! The upstream `datacenter-web` response returns **one row per security**, with
//! each financial line item as a *column* (akshare then relabels those columns
//! positionally — see e.g. `big_df.columns = [...]` in the source). To make the
//! dynamic line-item set queryable we normalize to one row per
//! `(security, item, report_date)`:
//!
//! ```text
//! FinancialStatementRow { security_code, security_name, item, report_date, value, source }
//! ```
//!
//! The design's requested `item` / `report_date` / `value` / `source` fields are
//! kept (the leading brief calls for exactly these); `security_code` /
//! `security_name` are added because without them a market-wide dataset collapses
//! to a keyless blob. `report_date` carries the period (the request filters on a
//! single `REPORT_DATE`), so the "dynamic date columns" of a comparative view
//! become ordinary rows instead of new struct fields.
//!
//! ## Field-key fidelity note
//!
//! akshare reads these reports **positionally** (`columns=ALL` then a fixed
//! column-label list), so the real upstream field keys are not recoverable from
//! the akshare source. The `EM_*` keys below are **inferred** Eastmoney
//! `RPT_DMSK_FN_*` column ids, mapped to akshare's Chinese line-item labels;
//! they must be verified against a live sample before production use (same
//! convention as `gdfx.rs`).

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// Eastmoney source bucket, for rate limiting / error context.
const SOURCE_EASTMONEY: &str = "eastmoney";

/// Eastmoney `datacenter-web` data-center endpoint (shared by every fn here).
const BASE: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";

/// akshare's default reporting period (`YYYYMMDD`). Documented here per the
/// porting brief; the public fns take `date: &str` (akshare's default is this).
#[allow(dead_code)]
const DEFAULT_REPORT_DATE: &str = "20240331";

// ---------------------------------------------------------------------------
// Shared helpers (mirrors lhb.rs / gdfx.rs conventions)
// ---------------------------------------------------------------------------

/// Read a string field, returning `None` when missing/null.
fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Read a numeric field that may be a JSON number or a plain numeric string.
fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

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

/// Validate an `YYYYMMDD` date string used as a request parameter.
fn check_date8(date: &str, what: &str) -> Result<()> {
    if date.len() == 8 && date.bytes().all(|b| b.is_ascii_digit()) {
        Ok(())
    } else {
        Err(Error::InvalidParam(format!(
            "{what} must be 8 ASCII digits YYYYMMDD, got {date:?}"
        )))
    }
}

/// Format an `YYYYMMDD` date as `YYYY-MM-DD` (Eastmoney `REPORT_DATE` form).
fn fmt_date8(date: &str) -> Result<String> {
    check_date8(date, "report_date")?;
    Ok(format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8]))
}

/// Follow Eastmoney `result.pages` pagination, concatenating every `result.data`
/// page. Used because the akshare source loops over `data_json["result"]["pages"]`.
async fn fetch_statement(
    client: &Client,
    endpoint: &'static str,
    params: &[(&str, &str)],
) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    let mut pn: u32 = 1;
    loop {
        let mut owned: Vec<(String, String)> = params
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        owned.push(("pageNumber".to_string(), pn.to_string()));
        let borrowed: Vec<(&str, &str)> = owned.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let v = client
            .get_json(SOURCE_EASTMONEY, endpoint, BASE, &borrowed)
            .await?;
        let data = data_array(&v)?;
        if data.is_empty() {
            break;
        }
        out.extend(data.iter().cloned());
        let pages = v
            .get("result")
            .and_then(|r| r.get("pages"))
            .and_then(|p| p.as_u64())
            .unwrap_or(1);
        if pn as u64 >= pages {
            break;
        }
        pn += 1;
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Normalized row
// ---------------------------------------------------------------------------

/// One normalized financial-statement cell: a single line item (`item`) for a
/// single security (`security_code`/`security_name`) at a reporting period
/// (`report_date`), with its numeric `value` (or `None` when missing/null).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FinancialStatementRow {
    /// `SECURITY_CODE` 股票代码
    pub security_code: String,
    /// `SECURITY_NAME_ABBR` 股票简称
    pub security_name: String,
    /// Line-item label (akshare's Chinese column, e.g. `资产-货币资金`)
    pub item: String,
    /// Reporting period, `YYYY-MM-DD` (from `REPORT_DATE`)
    pub report_date: String,
    /// The line-item value, or `None` when missing/null
    pub value: Option<f64>,
    pub source: &'static str,
}

/// `(EM_FIELD_KEY, ITEM_LABEL)` pairs for a statement. Field keys are inferred
/// (see module note); labels mirror akshare's positional column list.
type LineItems = &'static [(&'static str, &'static str)];

/// 资产负债表 line items — akshare `stock_zcfz_em` / `stock_zcfz_bj_em`.
const ZCFZ_ITEMS: LineItems = &[
    ("TOTAL_ASSETS", "资产-总资产"),
    ("MONETARY_FUNDS", "资产-货币资金"),
    ("ACCOUNTS_RECEIVABLE", "资产-应收账款"),
    ("INVENTORY", "资产-存货"),
    ("TOTAL_ASSETS_YOY", "资产-总资产同比"),
    ("ACCOUNTS_PAYABLE", "负债-应付账款"),
    ("ADVANCE_RECEIPTS", "负债-预收账款"),
    ("TOTAL_LIABILITIES", "负债-总负债"),
    ("TOTAL_LIAB_YOY", "负债-总负债同比"),
    ("DEBT_TO_ASSET_RATIO", "资产负债率"),
    ("TOTAL_EQUITY", "股东权益合计"),
];

/// 利润表 line items — akshare `stock_lrb_em`.
const LRB_ITEMS: LineItems = &[
    ("NET_PROFIT", "净利润"),
    ("NET_PROFIT_YOY", "净利润同比"),
    ("TOTAL_OP_REVENUE", "营业总收入"),
    ("TOTAL_OP_REVENUE_YOY", "营业总收入同比"),
    ("OP_COST", "营业总支出-营业支出"),
    ("SELL_EXPENSE", "营业总支出-销售费用"),
    ("ADMIN_EXPENSE", "营业总支出-管理费用"),
    ("FIN_EXPENSE", "营业总支出-财务费用"),
    ("TOTAL_OP_EXPENSE", "营业总支出-营业总支出"),
    ("OP_PROFIT", "营业利润"),
    ("TOTAL_PROFIT", "利润总额"),
];

/// 现金流量表 line items — akshare `stock_xjll_em`.
const XJLL_ITEMS: LineItems = &[
    ("NET_CASHFLOW", "净现金流-净现金流"),
    ("NET_CASHFLOW_YOY", "净现金流-同比增长"),
    ("OP_CASHFLOW_NET", "经营性现金流-现金流量净额"),
    ("OP_CASHFLOW_RATIO", "经营性现金流-净现金流占比"),
    ("INV_CASHFLOW_NET", "投资性现金流-现金流量净额"),
    ("INV_CASHFLOW_RATIO", "投资性现金流-净现金流占比"),
    ("FIN_CASHFLOW_NET", "融资性现金流-现金流量净额"),
    ("FIN_CASHFLOW_RATIO", "融资性现金流-净现金流占比"),
];

/// Explode a `result.data` row array into normalized [`FinancialStatementRow`]s,
/// one row per `(security, item)` of `items`.
fn normalize_rows(rows: &[Value], items: LineItems) -> Result<Vec<FinancialStatementRow>> {
    let mut out = Vec::new();
    for sec in rows {
        let code = fstr(sec, "SECURITY_CODE").unwrap_or_default();
        let name = fstr(sec, "SECURITY_NAME_ABBR").unwrap_or_default();
        if code.is_empty() || name.is_empty() {
            continue;
        }
        let report_date = fstr(sec, "REPORT_DATE").unwrap_or_default();
        for &(key, label) in items {
            out.push(FinancialStatementRow {
                security_code: code.clone(),
                security_name: name.clone(),
                item: label.to_string(),
                report_date: report_date.clone(),
                value: fnum(sec, key),
                source: SOURCE_EASTMONEY,
            });
        }
    }
    Ok(out)
}

// ===========================================================================
// stock_zcfz_em — 资产负债表 (全市场)
// ===========================================================================

/// Port of `stock_zcfz_em(date="20240331")`.
///
/// `date` is `YYYYMMDD`; filters main-board/board types
/// (`SECURITY_TYPE_CODE in ("058001001","058001008")` and
/// `TRADE_MARKET_CODE!="069001017"`).
pub async fn stock_zcfz_em(client: &Client, date: &str) -> Result<Vec<FinancialStatementRow>> {
    let d = fmt_date8(date)?;
    let filter = format!(
        "(SECURITY_TYPE_CODE in (\"058001001\",\"058001008\"))\
(TRADE_MARKET_CODE!=\"069001017\")(REPORT_DATE='{d}')"
    );
    let params = [
        ("reportName", "RPT_DMSK_FN_BALANCE"),
        ("columns", "ALL"),
        ("pageSize", "500"),
        ("sortColumns", "NOTICE_DATE,SECURITY_CODE"),
        ("sortTypes", "-1,-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &filter),
    ];
    let rows = fetch_statement(client, "stock_zcfz_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": rows } });
    parse_stock_zcfz_em(&synthetic)
}

/// Parse a datacenter `result.data` array into balance-sheet [`FinancialStatementRow`]s.
pub(crate) fn parse_stock_zcfz_em(resp: &Value) -> Result<Vec<FinancialStatementRow>> {
    let rows = data_array(resp)?;
    normalize_rows(rows, ZCFZ_ITEMS)
}

// ===========================================================================
// stock_zcfz_bj_em — 资产负债表 (北交所)
// ===========================================================================

/// Port of `stock_zcfz_bj_em(date="20240331")`.
///
/// Identical to [`stock_zcfz_em`] but scoped to the Beijing exchange
/// (`TRADE_MARKET_CODE="069001017"`).
pub async fn stock_zcfz_bj_em(client: &Client, date: &str) -> Result<Vec<FinancialStatementRow>> {
    let d = fmt_date8(date)?;
    let filter = format!("(TRADE_MARKET_CODE=\"069001017\")(REPORT_DATE='{d}')");
    let params = [
        ("reportName", "RPT_DMSK_FN_BALANCE"),
        ("columns", "ALL"),
        ("pageSize", "500"),
        ("sortColumns", "NOTICE_DATE,SECURITY_CODE"),
        ("sortTypes", "-1,-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &filter),
    ];
    let rows = fetch_statement(client, "stock_zcfz_bj_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": rows } });
    parse_stock_zcfz_bj_em(&synthetic)
}

/// Parse a datacenter `result.data` array into Beijing-exchange balance-sheet [`FinancialStatementRow`]s.
pub(crate) fn parse_stock_zcfz_bj_em(resp: &Value) -> Result<Vec<FinancialStatementRow>> {
    let rows = data_array(resp)?;
    normalize_rows(rows, ZCFZ_ITEMS)
}

// ===========================================================================
// stock_lrb_em — 利润表
// ===========================================================================

/// Port of `stock_lrb_em(date="20240331")`.
///
/// `date` is `YYYYMMDD`; same market filter as [`stock_zcfz_em`].
pub async fn stock_lrb_em(client: &Client, date: &str) -> Result<Vec<FinancialStatementRow>> {
    let d = fmt_date8(date)?;
    let filter = format!(
        "(SECURITY_TYPE_CODE in (\"058001001\",\"058001008\"))\
(TRADE_MARKET_CODE!=\"069001017\")(REPORT_DATE='{d}')"
    );
    let params = [
        ("reportName", "RPT_DMSK_FN_INCOME"),
        ("columns", "ALL"),
        ("pageSize", "500"),
        ("sortColumns", "NOTICE_DATE,SECURITY_CODE"),
        ("sortTypes", "-1,-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &filter),
    ];
    let rows = fetch_statement(client, "stock_lrb_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": rows } });
    parse_stock_lrb_em(&synthetic)
}

/// Parse a datacenter `result.data` array into income-statement [`FinancialStatementRow`]s.
pub(crate) fn parse_stock_lrb_em(resp: &Value) -> Result<Vec<FinancialStatementRow>> {
    let rows = data_array(resp)?;
    normalize_rows(rows, LRB_ITEMS)
}

// ===========================================================================
// stock_xjll_em — 现金流量表
// ===========================================================================

/// Port of `stock_xjll_em(date="20240331")`.
///
/// `date` is `YYYYMMDD`; same market filter as [`stock_zcfz_em`].
pub async fn stock_xjll_em(client: &Client, date: &str) -> Result<Vec<FinancialStatementRow>> {
    let d = fmt_date8(date)?;
    let filter = format!(
        "(SECURITY_TYPE_CODE in (\"058001001\",\"058001008\"))\
(TRADE_MARKET_CODE!=\"069001017\")(REPORT_DATE='{d}')"
    );
    let params = [
        ("reportName", "RPT_DMSK_FN_CASHFLOW"),
        ("columns", "ALL"),
        ("pageSize", "500"),
        ("sortColumns", "NOTICE_DATE,SECURITY_CODE"),
        ("sortTypes", "-1,-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &filter),
    ];
    let rows = fetch_statement(client, "stock_xjll_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": rows } });
    parse_stock_xjll_em(&synthetic)
}

/// Parse a datacenter `result.data` array into cash-flow-statement [`FinancialStatementRow`]s.
pub(crate) fn parse_stock_xjll_em(resp: &Value) -> Result<Vec<FinancialStatementRow>> {
    let rows = data_array(resp)?;
    normalize_rows(rows, XJLL_ITEMS)
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

    /// Find a single normalized row by security code + item label.
    fn find<'a>(rows: &'a [FinancialStatementRow], code: &str, item: &str) -> &'a FinancialStatementRow {
        rows.iter()
            .find(|r| r.security_code == code && r.item == item)
            .expect("expected row present")
    }

    #[test]
    fn parses_stock_zcfz_em() {
        let rows = parse_stock_zcfz_em(&fixture("stock_zcfz_em.json")).unwrap();
        // 2 securities × 11 line items
        assert_eq!(rows.len(), 22);
        let m = find(&rows, "600519", "资产-货币资金");
        assert_eq!(m.security_name, "贵州茅台");
        assert_eq!(m.report_date, "2024-03-31");
        assert_eq!(m.value, Some(68265740000.0));
        assert_eq!(m.source, "eastmoney");
        // None case: 负债-应付账款 is null for 贵州茅台
        let ap = find(&rows, "600519", "负债-应付账款");
        assert_eq!(ap.value, None);
        // 平安银行 has a real 应付账款
        assert_eq!(find(&rows, "000001", "负债-应付账款").value, Some(987654.0));
    }

    #[test]
    fn parses_stock_zcfz_bj_em() {
        let rows = parse_stock_zcfz_bj_em(&fixture("stock_zcfz_bj_em.json")).unwrap();
        assert_eq!(rows.len(), 22);
        let m = find(&rows, "920002", "资产-总资产");
        assert_eq!(m.security_name, "万达轴承");
        assert_eq!(m.report_date, "2024-03-31");
        assert_eq!(m.value, Some(1234567890.0));
        // None case: 负债-预收账款 null for one BJ stock
        assert_eq!(find(&rows, "920002", "负债-预收账款").value, None);
        assert_eq!(find(&rows, "920111", "资产负债率").value, Some(33.21));
    }

    #[test]
    fn parses_stock_lrb_em() {
        let rows = parse_stock_lrb_em(&fixture("stock_lrb_em.json")).unwrap();
        // 2 securities × 11 line items
        assert_eq!(rows.len(), 22);
        let m = find(&rows, "600519", "净利润");
        assert_eq!(m.security_name, "贵州茅台");
        assert_eq!(m.report_date, "2024-03-31");
        assert_eq!(m.value, Some(24065100000.0));
        // None case: 营业总支出-财务费用 is null for 贵州茅台
        assert_eq!(find(&rows, "600519", "营业总支出-财务费用").value, None);
        assert_eq!(find(&rows, "000001", "营业总收入").value, Some(43187000000.0));
    }

    #[test]
    fn parses_stock_xjll_em() {
        let rows = parse_stock_xjll_em(&fixture("stock_xjll_em.json")).unwrap();
        // 2 securities × 8 line items
        assert_eq!(rows.len(), 16);
        let m = find(&rows, "600519", "经营性现金流-现金流量净额");
        assert_eq!(m.security_name, "贵州茅台");
        assert_eq!(m.report_date, "2024-03-31");
        assert_eq!(m.value, Some(9123700000.0));
        // None case: 融资性现金流-净现金流占比 null for one stock
        assert_eq!(find(&rows, "600519", "融资性现金流-净现金流占比").value, None);
        assert_eq!(find(&rows, "000001", "净现金流-净现金流").value, Some(-123456789.0));
    }
}
