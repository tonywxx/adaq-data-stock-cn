//! 东方财富网-数据中心-特色数据-商誉 (akshare `akshare/stock_feature/stock_sy_em.py`).
//!
//! Ported public functions (all pure Eastmoney datacenter JSON, no JS/token/signature):
//!
//! | Rust fn                  | akshare fn                 | reportName                      | Paged | akshare line |
//! |--------------------------|----------------------------|---------------------------------|-------|--------------|
//! | `stock_sy_profile_em`    | `stock_sy_profile_em`      | `RPT_GOODWILL_MARKETSTATISTICS` | no    | `stock_sy_em.py:19`    |
//! | `stock_sy_yq_em`         | `stock_sy_yq_em`           | `RPT_GOODWILL_STOCKPREDICT`     | yes   | `stock_sy_em.py:84`    |
//! | `stock_sy_jz_em`         | `stock_sy_jz_em`           | `RPT_GOODWILL_STOCKDETAILS`     | yes   | `stock_sy_em.py:193`   |
//! | `stock_sy_em`            | `stock_sy_em`              | `RPT_GOODWILL_STOCKDETAILS`     | yes   | `stock_sy_em.py:294`   |
//! | `stock_sy_hy_em`         | `stock_sy_hy_em`           | `RPT_GOODWILL_INDUSTATISTICS`   | yes   | `stock_sy_em.py:386`   |
//!
//! ## Field-name fidelity note
//!
//! `stock_sy_profile_em` uses akshare's **positional** column relabel
//! (`data_df.columns = [...]`), so the real upstream Eastmoney field names are
//! not recoverable from the akshare source. Its row struct field ids are
//! **inferred** from the column semantics and must be verified against a live
//! sample before production use. The other four functions use akshare's
//! `.rename(columns={...})` mapping, so their field ids ARE the real upstream
//! keys and are ported exactly.
//!
//! ## DEFERRED
//!
//! None. All five public functions are pure HTTP requests to the Eastmoney
//! `datacenter-web` JSON API; none require JS execution, tokens, signatures,
//! cookies, `execjs`/`MiniRacer` or HTML/Excel scraping.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// Eastmoney source bucket, for rate limiting / error context.
const SOURCE_EASTMONEY: &str = "eastmoney";

/// Eastmoney `datacenter-web` data-center endpoint (shared by every fn here).
const BASE: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";

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

/// Format an `YYYYMMDD` date as `YYYY-MM-DD` (Eastmoney filter style).
fn fmt_date8(date: &str) -> String {
    format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8])
}

/// Follow Eastmoney `result.pages` pagination, concatenating every `result.data`
/// page. Used by the fns whose akshare source loops over `total_page`.
async fn paged(
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
        let borrowed: Vec<(&str, &str)> = owned
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
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

// ===========================================================================
// stock_sy_profile_em — 商誉-A股商誉市场概况
// ===========================================================================

/// One A-share goodwill market-statistics row, port of `stock_sy_profile_em`
/// (Eastmoney `RPT_GOODWILL_MARKETSTATISTICS`).
///
/// Field ids are **inferred** (akshare relabels positionally — see module note).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SyProfileRow {
    /// 报告期 (inferred `REPORT_DATE`)
    pub report_date: String,
    /// 商誉 (inferred `GOODWILL`)
    pub goodwill: Option<f64>,
    /// 商誉减值 (inferred `IMPAIRMENT`)
    pub impairment: Option<f64>,
    /// 净资产 (inferred `NET_ASSET`)
    pub net_asset: Option<f64>,
    /// 商誉占净资产比例 (inferred `GOODWILL_NETASSET_RATIO`)
    pub goodwill_netasset_ratio: Option<f64>,
    /// 商誉减值占净资产比例 (inferred `IMPAIRMENT_NETASSET_RATIO`)
    pub impairment_netasset_ratio: Option<f64>,
    /// 净利润规模 (inferred `NET_PROFIT`)
    pub net_profit: Option<f64>,
    /// 商誉减值占净利润比例 (inferred `IMPAIRMENT_NETPROFIT_RATIO`)
    pub impairment_netprofit_ratio: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_sy_profile_em()` — A股商誉市场概况 (no date parameter).
pub async fn stock_sy_profile_em(client: &Client) -> Result<Vec<SyProfileRow>> {
    let filter = r#"((GOODWILL_STATE="1")( | IMPAIRMENT_STATE="1"))(TRADE_BOARD="all")"#;
    let params = [
        ("sortColumns", "REPORT_DATE"),
        ("sortTypes", "-1"),
        ("pageSize", "5000"),
        ("pageNumber", "1"),
        ("reportName", "RPT_GOODWILL_MARKETSTATISTICS"),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_sy_profile_em", BASE, &params)
        .await?;
    parse_stock_sy_profile_em(&v)
}

/// Parse a datacenter `result.data` array into [`SyProfileRow`]s.
pub(crate) fn parse_stock_sy_profile_em(resp: &Value) -> Result<Vec<SyProfileRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(SyProfileRow {
            report_date: fstr(item, "REPORT_DATE").unwrap_or_default(),
            goodwill: fnum(item, "GOODWILL"),
            impairment: fnum(item, "IMPAIRMENT"),
            net_asset: fnum(item, "NET_ASSET"),
            goodwill_netasset_ratio: fnum(item, "GOODWILL_NETASSET_RATIO"),
            impairment_netasset_ratio: fnum(item, "IMPAIRMENT_NETASSET_RATIO"),
            net_profit: fnum(item, "NET_PROFIT"),
            impairment_netprofit_ratio: fnum(item, "IMPAIRMENT_NETPROFIT_RATIO"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_sy_yq_em — 商誉减值预期明细
// ===========================================================================

/// One goodwill-impairment prediction row, port of `stock_sy_yq_em`
/// (Eastmoney `RPT_GOODWILL_STOCKPREDICT`). Field ids are the real upstream keys.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SyYqRow {
    /// `SECURITY_CODE` 股票代码
    pub code: String,
    /// `SECURITY_NAME_ABBR` 股票简称
    pub name: String,
    /// `PERFORM_CHANGE_EXPLAIN` 业绩变动原因
    pub perform_change_explain: Option<String>,
    /// `NEWEST_REPORT_DATE` 最新商誉报告期
    pub newest_report_date: Option<String>,
    /// `NEWEST_GOODWILL` 最新一期商誉
    pub newest_goodwill: Option<f64>,
    /// `PE_GOODWILL` 上年商誉
    pub pe_goodwill: Option<f64>,
    /// `PREDICT_NETPROFIT_LOWER` 预计净利润-下限
    pub predict_netprofit_lower: Option<f64>,
    /// `PREDICT_NETPROFIT_UPPER` 预计净利润-上限
    pub predict_netprofit_upper: Option<f64>,
    /// `PERFORM_CHANGE_LOWER` 业绩变动幅度-下限
    pub perform_change_lower: Option<f64>,
    /// `PERFORM_CHANGE_UPPER` 业绩变动幅度-上限
    pub perform_change_upper: Option<f64>,
    /// `PE_SAMEREPORT_NETPROFIT` 上年度同期净利润
    pub pe_samereport_netprofit: Option<f64>,
    /// `NOTICE_DATE` 公告日期
    pub notice_date: Option<String>,
    /// `TRADE_MARKET` 交易市场 (raw code, e.g. `shzb`/`kcb`/`szzb`/`cyb`)
    pub trade_market: Option<String>,
    pub source: &'static str,
}

/// Port of `stock_sy_yq_em(date)` — 商誉减值预期明细.
///
/// `date` is `YYYYMMDD` (akshare default `20240630`).
pub async fn stock_sy_yq_em(client: &Client, date: &str) -> Result<Vec<SyYqRow>> {
    check_date8(date, "stock_sy_yq_em date")?;
    let d = fmt_date8(date);
    let filter = format!("(REPORT_DATE='{d}')");
    let params = [
        ("sortColumns", "NOTICE_DATE,SECURITY_CODE"),
        ("sortTypes", "-1,-1"),
        ("pageSize", "5000"),
        ("reportName", "RPT_GOODWILL_STOCKPREDICT"),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
    ];
    let items = paged(client, "stock_sy_yq_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_sy_yq_em(&synthetic)
}

/// Parse a datacenter `result.data` array into [`SyYqRow`]s.
pub(crate) fn parse_stock_sy_yq_em(resp: &Value) -> Result<Vec<SyYqRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let code = fstr(item, "SECURITY_CODE").unwrap_or_default();
        let name = fstr(item, "SECURITY_NAME_ABBR").unwrap_or_default();
        if code.is_empty() || name.is_empty() {
            continue;
        }
        out.push(SyYqRow {
            code,
            name,
            perform_change_explain: fstr(item, "PERFORM_CHANGE_EXPLAIN"),
            newest_report_date: fstr(item, "NEWEST_REPORT_DATE"),
            newest_goodwill: fnum(item, "NEWEST_GOODWILL"),
            pe_goodwill: fnum(item, "PE_GOODWILL"),
            predict_netprofit_lower: fnum(item, "PREDICT_NETPROFIT_LOWER"),
            predict_netprofit_upper: fnum(item, "PREDICT_NETPROFIT_UPPER"),
            perform_change_lower: fnum(item, "PERFORM_CHANGE_LOWER"),
            perform_change_upper: fnum(item, "PERFORM_CHANGE_UPPER"),
            pe_samereport_netprofit: fnum(item, "PE_SAMEREPORT_NETPROFIT"),
            notice_date: fstr(item, "NOTICE_DATE"),
            trade_market: fstr(item, "TRADE_MARKET"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_sy_jz_em — 个股商誉减值明细
// ===========================================================================

/// One per-stock goodwill-impairment detail row, port of `stock_sy_jz_em`
/// (Eastmoney `RPT_GOODWILL_STOCKDETAILS`). Field ids are the real upstream keys.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SyJzRow {
    /// `SECURITY_CODE` 股票代码
    pub code: String,
    /// `SECURITY_NAME_ABBR` 股票简称
    pub name: String,
    /// `GOODWILL` 商誉
    pub goodwill: Option<f64>,
    /// `GOODWILL_CHANGE` 商誉减值
    pub goodwill_change: Option<f64>,
    /// `SUMSHEQUITY_RATIO` 商誉占净资产比例
    pub sumshequity_ratio: Option<f64>,
    /// `SE_CHANGE_RATIO` 商誉减值占净资产比例
    pub se_change_ratio: Option<f64>,
    /// `PARENTNETPROFIT` 净利润
    pub parentnetprofit: Option<f64>,
    /// `PNP_CHANGE_RATIO` 商誉减值占净利润比例
    pub pnp_change_ratio: Option<f64>,
    /// `NOTICE_DATE` 公告日期
    pub notice_date: Option<String>,
    /// `TRADE_BOARD` 交易市场 (raw code, e.g. `shzb`/`kcb`/`szzb`/`cyb`)
    pub trade_board: Option<String>,
    pub source: &'static str,
}

/// Port of `stock_sy_jz_em(date)` — 个股商誉减值明细.
///
/// `date` is `YYYYMMDD` (akshare default `20240630`).
pub async fn stock_sy_jz_em(client: &Client, date: &str) -> Result<Vec<SyJzRow>> {
    check_date8(date, "stock_sy_jz_em date")?;
    let d = fmt_date8(date);
    let filter = format!("(REPORT_DATE='{d}')");
    let params = [
        ("sortColumns", "GOODWILL_CHANGE"),
        ("sortTypes", "-1"),
        ("pageSize", "5000"),
        ("reportName", "RPT_GOODWILL_STOCKDETAILS"),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
    ];
    let items = paged(client, "stock_sy_jz_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_sy_jz_em(&synthetic)
}

/// Parse a datacenter `result.data` array into [`SyJzRow`]s.
pub(crate) fn parse_stock_sy_jz_em(resp: &Value) -> Result<Vec<SyJzRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let code = fstr(item, "SECURITY_CODE").unwrap_or_default();
        let name = fstr(item, "SECURITY_NAME_ABBR").unwrap_or_default();
        if code.is_empty() || name.is_empty() {
            continue;
        }
        out.push(SyJzRow {
            code,
            name,
            goodwill: fnum(item, "GOODWILL"),
            goodwill_change: fnum(item, "GOODWILL_CHANGE"),
            sumshequity_ratio: fnum(item, "SUMSHEQUITY_RATIO"),
            se_change_ratio: fnum(item, "SE_CHANGE_RATIO"),
            parentnetprofit: fnum(item, "PARENTNETPROFIT"),
            pnp_change_ratio: fnum(item, "PNP_CHANGE_RATIO"),
            notice_date: fstr(item, "NOTICE_DATE"),
            trade_board: fstr(item, "TRADE_BOARD"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_sy_em — 个股商誉明细
// ===========================================================================

/// One per-stock goodwill detail row, port of `stock_sy_em`
/// (Eastmoney `RPT_GOODWILL_STOCKDETAILS`). Field ids are the real upstream keys.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SyEmRow {
    /// `SECURITY_CODE` 股票代码
    pub code: String,
    /// `SECURITY_NAME_ABBR` 股票简称
    pub name: String,
    /// `GOODWILL` 商誉
    pub goodwill: Option<f64>,
    /// `SUMSHEQUITY_RATIO` 商誉占净资产比例
    pub sumshequity_ratio: Option<f64>,
    /// `PARENTNETPROFIT` 净利润
    pub parentnetprofit: Option<f64>,
    /// `PNP_YOY_RATIO` 净利润同比
    pub pnp_yoy_ratio: Option<f64>,
    /// `GOODWILL_PRE` 上年商誉
    pub goodwill_pre: Option<f64>,
    /// `NOTICE_DATE` 公告日期
    pub notice_date: Option<String>,
    /// `TRADE_BOARD` 交易市场 (raw code, e.g. `shzb`/`kcb`/`szzb`/`cyb`)
    pub trade_board: Option<String>,
    pub source: &'static str,
}

/// Port of `stock_sy_em(date)` — 个股商誉明细.
///
/// `date` is `YYYYMMDD` (akshare default `20231231`).
pub async fn stock_sy_em(client: &Client, date: &str) -> Result<Vec<SyEmRow>> {
    check_date8(date, "stock_sy_em date")?;
    let d = fmt_date8(date);
    let filter = format!("(REPORT_DATE='{d}')");
    let params = [
        ("sortColumns", "NOTICE_DATE,SECURITY_CODE"),
        ("sortTypes", "-1,-1"),
        ("pageSize", "5000"),
        ("reportName", "RPT_GOODWILL_STOCKDETAILS"),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
    ];
    let items = paged(client, "stock_sy_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_sy_em(&synthetic)
}

/// Parse a datacenter `result.data` array into [`SyEmRow`]s.
pub(crate) fn parse_stock_sy_em(resp: &Value) -> Result<Vec<SyEmRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let code = fstr(item, "SECURITY_CODE").unwrap_or_default();
        let name = fstr(item, "SECURITY_NAME_ABBR").unwrap_or_default();
        if code.is_empty() || name.is_empty() {
            continue;
        }
        out.push(SyEmRow {
            code,
            name,
            goodwill: fnum(item, "GOODWILL"),
            sumshequity_ratio: fnum(item, "SUMSHEQUITY_RATIO"),
            parentnetprofit: fnum(item, "PARENTNETPROFIT"),
            pnp_yoy_ratio: fnum(item, "PNP_YOY_RATIO"),
            goodwill_pre: fnum(item, "GOODWILL_PRE"),
            notice_date: fstr(item, "NOTICE_DATE"),
            trade_board: fstr(item, "TRADE_BOARD"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_sy_hy_em — 行业商誉
// ===========================================================================

/// One industry goodwill row, port of `stock_sy_hy_em`
/// (Eastmoney `RPT_GOODWILL_INDUSTATISTICS`). Field ids are the real upstream keys.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SyHyRow {
    /// `INDUSTRY_NAME` 行业名称
    pub industry_name: String,
    /// `ORG_NUM` 公司家数
    pub org_num: Option<f64>,
    /// `GOODWILL` 商誉规模
    pub goodwill: Option<f64>,
    /// `SUMSHEQUITY` 净资产
    pub sumshequity: Option<f64>,
    /// `SUMSHEQUITY_RATIO` 商誉规模占净资产规模比例
    pub sumshequity_ratio: Option<f64>,
    /// `PARENTNETPROFIT` 净利润规模
    pub parentnetprofit: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_sy_hy_em(date)` — 行业商誉.
///
/// `date` is `YYYYMMDD` (akshare default `20240930`).
pub async fn stock_sy_hy_em(client: &Client, date: &str) -> Result<Vec<SyHyRow>> {
    check_date8(date, "stock_sy_hy_em date")?;
    let d = fmt_date8(date);
    let filter = format!("(REPORT_DATE='{d}')");
    let params = [
        ("sortColumns", "SUMSHEQUITY_RATIO"),
        ("sortTypes", "-1"),
        ("pageSize", "5000"),
        ("reportName", "RPT_GOODWILL_INDUSTATISTICS"),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
    ];
    let items = paged(client, "stock_sy_hy_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_sy_hy_em(&synthetic)
}

/// Parse a datacenter `result.data` array into [`SyHyRow`]s.
pub(crate) fn parse_stock_sy_hy_em(resp: &Value) -> Result<Vec<SyHyRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let industry_name = fstr(item, "INDUSTRY_NAME").unwrap_or_default();
        if industry_name.is_empty() {
            continue;
        }
        out.push(SyHyRow {
            industry_name,
            org_num: fnum(item, "ORG_NUM"),
            goodwill: fnum(item, "GOODWILL"),
            sumshequity: fnum(item, "SUMSHEQUITY"),
            sumshequity_ratio: fnum(item, "SUMSHEQUITY_RATIO"),
            parentnetprofit: fnum(item, "PARENTNETPROFIT"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
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
    fn parses_stock_sy_profile_em() {
        let rows = parse_stock_sy_profile_em(&fixture("stock_sy_profile_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].report_date, "2024-06-30");
        assert_eq!(rows[0].goodwill, Some(16000.0));
        assert_eq!(rows[0].impairment, Some(100.0));
        assert_eq!(rows[0].net_asset, Some(300000.0));
        assert_eq!(rows[0].goodwill_netasset_ratio, Some(5.33));
        assert_eq!(rows[0].impairment_netasset_ratio, Some(0.03));
        assert_eq!(rows[0].net_profit, Some(45000.0));
        assert_eq!(rows[0].impairment_netprofit_ratio, Some(0.22));
        // None case: the 2023-12-31 row omits impairment / impairment ratios.
        assert_eq!(rows[1].report_date, "2023-12-31");
        assert_eq!(rows[1].goodwill, Some(15800.0));
        assert_eq!(rows[1].impairment, None);
        assert_eq!(rows[1].impairment_netasset_ratio, None);
        assert_eq!(rows[1].impairment_netprofit_ratio, None);
        assert_eq!(rows[0].source, "eastmoney");
    }

    #[test]
    fn parses_stock_sy_yq_em() {
        let rows = parse_stock_sy_yq_em(&fixture("stock_sy_yq_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "000725");
        assert_eq!(rows[0].name, "京东方A");
        assert_eq!(
            rows[0].perform_change_explain,
            Some("预计净利润亏损".to_string())
        );
        assert_eq!(rows[0].newest_report_date, Some("2024-03-31".to_string()));
        assert_eq!(rows[0].newest_goodwill, Some(500.0));
        assert_eq!(rows[0].pe_goodwill, Some(520.0));
        assert_eq!(rows[0].predict_netprofit_lower, Some(-50000.0));
        assert_eq!(rows[0].predict_netprofit_upper, Some(-30000.0));
        assert_eq!(rows[0].perform_change_lower, Some(-80.0));
        assert_eq!(rows[0].perform_change_upper, Some(-50.0));
        assert_eq!(rows[0].pe_samereport_netprofit, Some(100000.0));
        assert_eq!(rows[0].notice_date, Some("2024-04-20".to_string()));
        assert_eq!(rows[0].trade_market, Some("szzb".to_string()));
        // None case: 爱尔眼科 row omits perform_change_explain / newest_goodwill.
        assert_eq!(rows[1].code, "300015");
        assert_eq!(rows[1].perform_change_explain, None);
        assert_eq!(rows[1].newest_goodwill, None);
        assert_eq!(rows[1].trade_market, Some("cyb".to_string()));
    }

    #[test]
    fn parses_stock_sy_jz_em() {
        let rows = parse_stock_sy_jz_em(&fixture("stock_sy_jz_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "000333");
        assert_eq!(rows[0].name, "美的集团");
        assert_eq!(rows[0].goodwill, Some(200.0));
        assert_eq!(rows[0].goodwill_change, Some(10.0));
        assert_eq!(rows[0].sumshequity_ratio, Some(3.5));
        assert_eq!(rows[0].se_change_ratio, Some(0.2));
        assert_eq!(rows[0].parentnetprofit, Some(300000.0));
        assert_eq!(rows[0].pnp_change_ratio, Some(0.03));
        assert_eq!(rows[0].notice_date, Some("2024-04-30".to_string()));
        assert_eq!(rows[0].trade_board, Some("szzb".to_string()));
        // None case: 伊利股份 row omits goodwill_change / se_change_ratio / pnp_change_ratio.
        assert_eq!(rows[1].code, "600887");
        assert_eq!(rows[1].goodwill_change, None);
        assert_eq!(rows[1].se_change_ratio, None);
        assert_eq!(rows[1].pnp_change_ratio, None);
    }

    #[test]
    fn parses_stock_sy_em() {
        let rows = parse_stock_sy_em(&fixture("stock_sy_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "000651");
        assert_eq!(rows[0].name, "格力电器");
        assert_eq!(rows[0].goodwill, Some(800.0));
        assert_eq!(rows[0].sumshequity_ratio, Some(12.3));
        assert_eq!(rows[0].parentnetprofit, Some(250000.0));
        assert_eq!(rows[0].pnp_yoy_ratio, Some(-5.0));
        assert_eq!(rows[0].goodwill_pre, Some(820.0));
        assert_eq!(rows[0].notice_date, Some("2024-04-28".to_string()));
        assert_eq!(rows[0].trade_board, Some("szzb".to_string()));
        // None case: 中国平安 row omits pnp_yoy_ratio.
        assert_eq!(rows[1].code, "601318");
        assert_eq!(rows[1].pnp_yoy_ratio, None);
        assert_eq!(rows[1].goodwill_pre, Some(60.0));
        assert_eq!(rows[1].trade_board, Some("shzb".to_string()));
    }

    #[test]
    fn parses_stock_sy_hy_em() {
        let rows = parse_stock_sy_hy_em(&fixture("stock_sy_hy_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].industry_name, "传媒");
        assert_eq!(rows[0].org_num, Some(120.0));
        assert_eq!(rows[0].goodwill, Some(500.0));
        assert_eq!(rows[0].sumshequity, Some(8000.0));
        assert_eq!(rows[0].sumshequity_ratio, Some(6.25));
        assert_eq!(rows[0].parentnetprofit, Some(600.0));
        // None case: 计算机 row omits org_num / sumshequity_ratio.
        assert_eq!(rows[1].industry_name, "计算机");
        assert_eq!(rows[1].org_num, None);
        assert_eq!(rows[1].sumshequity_ratio, None);
        assert_eq!(rows[1].goodwill, Some(800.0));
    }
}
