//! 东方财富网-数据中心-特色数据-股权质押 (akshare `akshare/stock_feature/stock_gpzy_em.py`).
//!
//! Every function hits the Eastmoney `datacenter-web` JSON endpoint
//! (`https://datacenter-web.eastmoney.com/api/data/v1/get`) with a plain
//! `requests.get` — no JS signing, token, encryption, cookie or HTML scraping.
//! All seven public functions in the akshare source are ported:
//!
//! | Rust fn                                     | akshare fn                                  | reportName                          | Paged |
//! |---------------------------------------------|---------------------------------------------|-------------------------------------|-------|
//! | `stock_gpzy_profile_em`                     | `stock_gpzy_profile_em`                     | `RPT_CSDC_STATISTICS`               | yes   |
//! | `stock_gpzy_pledge_ratio_em`               | `stock_gpzy_pledge_ratio_em`               | `RPT_CSDC_LIST`                     | yes   |
//! | `stock_gpzy_pledge_ratio_detail_em`        | `stock_gpzy_pledge_ratio_detail_em`        | `RPT_A_APP_ACCUMDETAILS`            | yes   |
//! | `stock_gpzy_individual_pledge_ratio_detail_em` | `stock_gpzy_individual_pledge_ratio_detail_em` | `RPT_A_APP_ACCUMDETAILS` (`SECURITY_CODE` filter) | yes |
//! | `stock_gpzy_distribute_statistics_company_em`  | `stock_gpzy_distribute_statistics_company_em`  | `RPT_GDZY_ZYJG_SUM` (`PFORG_TYPE="证券"`) | no |
//! | `stock_gpzy_distribute_statistics_bank_em` | `stock_gpzy_distribute_statistics_bank_em` | `RPT_GDZY_ZYJG_SUM` (`PFORG_TYPE="银行"`) | no |
//! | `stock_gpzy_industry_data_em`              | `stock_gpzy_industry_data_em`              | `RPT_CSDC_INDUSTRY_STATISTICS`      | no    |
//!
//! ## Field-name fidelity note
//!
//! akshare relabels `columns=ALL` responses with **positional** Chinese column
//! labels (`big_df.columns = [...]`), so the real upstream Eastmoney field keys
//! are not recoverable from the akshare source for `profile`, `pledge_ratio`,
//! `pledge_ratio_detail` and `distribute_statistics`. The field names used in
//! those row structs below are **inferred** from the report name, `sortColumns`
//! and column semantics (mirrors the approach in `gdfx.rs`), and must be
//! verified against a live sample before production use. `industry_data` passes
//! an explicit `columns` list, so those field names ARE the real upstream keys
//! and are ported exactly (`INDUSTRY`/`TRADE_DATE`/`AVERAGE_PLEDGE_RATIO`/
//! `ORG_NUM`/`PLEDGE_TOTAL_NUM`/`TOTAL_PLEDGE_SHARES`/`PLEDGE_TOTAL_MARKETCAP`;
//! `INDUSTRY_CODE` is requested but dropped by akshare and omitted here).
//!
//! ## DEFERRED
//!
//! None. All seven public functions are pure HTTP JSON; the two akshare private
//! helpers (`_get_page_num_gpzy_market_pledge_ratio_detail`,
//! `_stock_gpzy_pledge_ratio_detail_em`) are inlined into the public fns / their
//! shared `parse_*` (pagination follows Eastmoney `result.pages`, equivalent to
//! akshare's `ceil(count/500)`).

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// Eastmoney source bucket, for rate limiting / error context.
const SOURCE_EASTMONEY: &str = "eastmoney";

/// Eastmoney `datacenter-web` data-center endpoint (shared by every fn here).
const BASE: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";

// ---------------------------------------------------------------------------
// Shared helpers
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
fn fmt_date8(date: &str) -> Result<String> {
    check_date8(date, "date")?;
    Ok(format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8]))
}

/// Follow Eastmoney `result.pages` pagination, concatenating every `result.data`
/// page. Used by the fns whose akshare source loops over the total page count.
async fn paged(client: &Client, endpoint: &'static str, params: &[(&str, &str)]) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    let mut pn: u32 = 1;
    loop {
        let mut owned: Vec<(String, String)> = params
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        owned.push(("pageNumber".to_string(), pn.to_string()));
        let borrowed: Vec<(&str, &str)> = owned.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let v = client.get_json(SOURCE_EASTMONEY, endpoint, BASE, &borrowed).await?;
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
// stock_gpzy_profile_em — 股权质押市场概况
// ===========================================================================

/// One equity-pledge market-profile row, port of `stock_gpzy_profile_em`
/// (Eastmoney `RPT_CSDC_STATISTICS`). Field names inferred — see module note.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GpzyProfileRow {
    /// `TRADE_DATE` 交易日期
    pub trade_date: String,
    /// `PLEDGE_RATIO` A股质押总比例 (akshare divides by 100)
    pub pledge_ratio: Option<f64>,
    /// `PLEDGE_COMPANY_NUM` 质押公司数量
    pub pledge_company_num: Option<f64>,
    /// `PLEDGE_NUM` 质押笔数
    pub pledge_num: Option<f64>,
    /// `PLEDGE_TOTAL_SHARES` 质押总股数
    pub pledge_total_shares: Option<f64>,
    /// `PLEDGE_TOTAL_MARKETCAP` 质押总市值
    pub pledge_total_marketcap: Option<f64>,
    /// `CSI300_POINT` 沪深300指数 (inferred)
    pub csi300_index: Option<f64>,
    /// `CHANGE_RATE` 涨跌幅
    pub change_rate: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_gpzy_profile_em()` (akshare `stock_gpzy_em.py:21`).
pub async fn stock_gpzy_profile_em(client: &Client) -> Result<Vec<GpzyProfileRow>> {
    let params = [
        ("sortColumns", "TRADE_DATE"),
        ("sortTypes", "-1"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("reportName", "RPT_CSDC_STATISTICS"),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
    ];
    let items = paged(client, "stock_gpzy_profile_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_gpzy_profile_em(&synthetic)
}

/// Parse a datacenter `result.data` array into [`GpzyProfileRow`]s.
pub(crate) fn parse_stock_gpzy_profile_em(resp: &Value) -> Result<Vec<GpzyProfileRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(GpzyProfileRow {
            trade_date: fstr(item, "TRADE_DATE").unwrap_or_default(),
            // akshare: big_df["A股质押总比例"] = ... / 100
            pledge_ratio: fnum(item, "PLEDGE_RATIO").map(|v| v / 100.0),
            pledge_company_num: fnum(item, "PLEDGE_COMPANY_NUM"),
            pledge_num: fnum(item, "PLEDGE_NUM"),
            pledge_total_shares: fnum(item, "PLEDGE_TOTAL_SHARES"),
            pledge_total_marketcap: fnum(item, "PLEDGE_TOTAL_MARKETCAP"),
            csi300_index: fnum(item, "CSI300_POINT"),
            change_rate: fnum(item, "CHANGE_RATE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_gpzy_pledge_ratio_em — 上市公司质押比例
// ===========================================================================

/// One listed-company pledge-ratio row, port of `stock_gpzy_pledge_ratio_em`
/// (Eastmoney `RPT_CSDC_LIST`). Field names inferred — see module note.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GpzyPledgeRatioRow {
    /// `SECURITY_CODE` 股票代码
    pub code: String,
    /// `SECURITY_NAME_ABBR` 股票简称
    pub name: String,
    /// `TRADE_DATE` 交易日期
    pub trade_date: String,
    /// `INDUSTRY` 所属行业 (inferred)
    pub industry: Option<String>,
    /// `PLEDGE_RATIO` 质押比例
    pub pledge_ratio: Option<f64>,
    /// `PLEDGE_SHARES` 质押股数 (inferred)
    pub pledge_shares: Option<f64>,
    /// `PLEDGE_MARKETCAP` 质押市值 (inferred)
    pub pledge_marketcap: Option<f64>,
    /// `PLEDGE_NUM` 质押笔数
    pub pledge_num: Option<f64>,
    /// `UNRESTRICTED_PLEDGE_SHARES` 无限售股质押数 (inferred)
    pub unrestricted_pledge_shares: Option<f64>,
    /// `RESTRICTED_PLEDGE_SHARES` 限售股质押数 (inferred)
    pub restricted_pledge_shares: Option<f64>,
    /// `YEAR_CHANGE_RATE` 近一年涨跌幅 (inferred)
    pub year_change_rate: Option<f64>,
    /// `INDUSTRY_CODE` 所属行业代码 (inferred)
    pub industry_code: Option<String>,
    pub source: &'static str,
}

/// Port of `stock_gpzy_pledge_ratio_em(date)` (akshare `stock_gpzy_em.py:88`).
///
/// `date` is `YYYYMMDD`; mapped to the `TRADE_DATE='YYYY-MM-DD'` filter.
pub async fn stock_gpzy_pledge_ratio_em(client: &Client, date: &str) -> Result<Vec<GpzyPledgeRatioRow>> {
    let d = fmt_date8(date)?;
    let filter = format!("(TRADE_DATE='{d}')");
    let params = [
        ("sortColumns", "PLEDGE_RATIO"),
        ("sortTypes", "-1"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("reportName", "RPT_CSDC_LIST"),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
    ];
    let items = paged(client, "stock_gpzy_pledge_ratio_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_gpzy_pledge_ratio_em(&synthetic)
}

/// Parse a datacenter `result.data` array into [`GpzyPledgeRatioRow`]s.
pub(crate) fn parse_stock_gpzy_pledge_ratio_em(resp: &Value) -> Result<Vec<GpzyPledgeRatioRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let code = fstr(item, "SECURITY_CODE").unwrap_or_default();
        let name = fstr(item, "SECURITY_NAME_ABBR").unwrap_or_default();
        if code.is_empty() || name.is_empty() {
            continue;
        }
        out.push(GpzyPledgeRatioRow {
            code,
            name,
            trade_date: fstr(item, "TRADE_DATE").unwrap_or_default(),
            industry: fstr(item, "INDUSTRY"),
            pledge_ratio: fnum(item, "PLEDGE_RATIO"),
            pledge_shares: fnum(item, "PLEDGE_SHARES"),
            pledge_marketcap: fnum(item, "PLEDGE_MARKETCAP"),
            pledge_num: fnum(item, "PLEDGE_NUM"),
            unrestricted_pledge_shares: fnum(item, "UNRESTRICTED_PLEDGE_SHARES"),
            restricted_pledge_shares: fnum(item, "RESTRICTED_PLEDGE_SHARES"),
            year_change_rate: fnum(item, "YEAR_CHANGE_RATE"),
            industry_code: fstr(item, "INDUSTRY_CODE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_gpzy_pledge_ratio_detail_em — 重要股东股权质押明细
// ===========================================================================

/// One major-shareholder pledge-detail row, port of
/// `stock_gpzy_pledge_ratio_detail_em` / `stock_gpzy_individual_pledge_ratio_detail_em`
/// (Eastmoney `RPT_A_APP_ACCUMDETAILS`). Field names inferred — see module note.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GpzyPledgeRatioDetailRow {
    /// `SECURITY_CODE` 股票代码
    pub code: String,
    /// `SECURITY_NAME_ABBR` 股票简称
    pub name: String,
    /// `HOLDER_NAME` 股东名称 (inferred)
    pub holder_name: Option<String>,
    /// `PLEDGE_SHARES` 质押股份数量 (inferred)
    pub pledge_shares: Option<f64>,
    /// `PLEDGE_HOLD_RATIO` 占所持股份比例 (inferred)
    pub pledge_hold_ratio: Option<f64>,
    /// `PLEDGE_TOTAL_RATIO` 占总股本比例 (inferred)
    pub pledge_total_ratio: Option<f64>,
    /// `PLEDGE_ORG` 质押机构 (inferred)
    pub pledge_org: Option<String>,
    /// `LATEST_PRICE` 最新价 (inferred)
    pub latest_price: Option<f64>,
    /// `PLEDGE_PRICE` 质押日收盘价 (inferred)
    pub pledge_price: Option<f64>,
    /// `CLOSE_LINE` 预估平仓线 (inferred)
    pub close_line: Option<f64>,
    /// `PLEDGE_START_DATE` 质押开始日期 (inferred)
    pub pledge_start_date: Option<String>,
    /// `PLEDGE_END_DATE` 质押结束日期 (inferred)
    pub pledge_end_date: Option<String>,
    /// `STATUS` 状态 (inferred)
    pub status: Option<String>,
    /// `NOTICE_DATE` 公告日期
    pub notice_date: Option<String>,
    pub source: &'static str,
}

/// Port of `stock_gpzy_pledge_ratio_detail_em()` (akshare `stock_gpzy_em.py:304`,
/// which delegates to the private `_stock_gpzy_pledge_ratio_detail_em`).
pub async fn stock_gpzy_pledge_ratio_detail_em(
    client: &Client,
) -> Result<Vec<GpzyPledgeRatioDetailRow>> {
    let params = [
        ("sortColumns", "NOTICE_DATE"),
        ("sortTypes", "-1"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("reportName", "RPT_A_APP_ACCUMDETAILS"),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
    ];
    let items = paged(client, "stock_gpzy_pledge_ratio_detail_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_gpzy_pledge_ratio_detail_em(&synthetic)
}

/// Port of `stock_gpzy_individual_pledge_ratio_detail_em(symbol)`
/// (akshare `stock_gpzy_em.py:308`); identical to the full detail query but with
/// a `(SECURITY_CODE="symbol")` filter. Shares the parse fn below.
pub async fn stock_gpzy_individual_pledge_ratio_detail_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<GpzyPledgeRatioDetailRow>> {
    let filter = format!("(SECURITY_CODE=\"{symbol}\")");
    let params = [
        ("sortColumns", "NOTICE_DATE"),
        ("sortTypes", "-1"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("reportName", "RPT_A_APP_ACCUMDETAILS"),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
    ];
    let items = paged(client, "stock_gpzy_individual_pledge_ratio_detail_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_gpzy_pledge_ratio_detail_em(&synthetic)
}

/// Parse a datacenter `result.data` array into [`GpzyPledgeRatioDetailRow`]s.
/// Shared by both `stock_gpzy_pledge_ratio_detail_em` and
/// `stock_gpzy_individual_pledge_ratio_detail_em`.
pub(crate) fn parse_stock_gpzy_pledge_ratio_detail_em(
    resp: &Value,
) -> Result<Vec<GpzyPledgeRatioDetailRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let code = fstr(item, "SECURITY_CODE").unwrap_or_default();
        let name = fstr(item, "SECURITY_NAME_ABBR").unwrap_or_default();
        if code.is_empty() || name.is_empty() {
            continue;
        }
        out.push(GpzyPledgeRatioDetailRow {
            code,
            name,
            holder_name: fstr(item, "HOLDER_NAME"),
            pledge_shares: fnum(item, "PLEDGE_SHARES"),
            pledge_hold_ratio: fnum(item, "PLEDGE_HOLD_RATIO"),
            pledge_total_ratio: fnum(item, "PLEDGE_TOTAL_RATIO"),
            pledge_org: fstr(item, "PLEDGE_ORG"),
            latest_price: fnum(item, "LATEST_PRICE"),
            pledge_price: fnum(item, "PLEDGE_PRICE"),
            close_line: fnum(item, "CLOSE_LINE"),
            pledge_start_date: fstr(item, "PLEDGE_START_DATE"),
            pledge_end_date: fstr(item, "PLEDGE_END_DATE"),
            status: fstr(item, "STATUS"),
            notice_date: fstr(item, "NOTICE_DATE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_gpzy_distribute_statistics_company_em — 质押机构分布统计-证券公司
// ===========================================================================

/// One pledge-institution distribution row, port of
/// `stock_gpzy_distribute_statistics_company_em` /
/// `stock_gpzy_distribute_statistics_bank_em` (Eastmoney `RPT_GDZY_ZYJG_SUM`).
/// Field names inferred — see module note.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GpzyDistributeStatisticsRow {
    /// `ORG_NAME` 质押机构 (inferred)
    pub org_name: String,
    /// `PLEDGE_COMPANY_NUM` 质押公司数量
    pub pledge_company_num: Option<f64>,
    /// `PLEDGE_NUM` 质押笔数
    pub pledge_num: Option<f64>,
    /// `PLEDGE_SHARES` 质押数量 (inferred)
    pub pledge_shares: Option<f64>,
    /// `RATIO_BELOW_WARNING` 未达预警线比例 (inferred)
    pub ratio_below_warning: Option<f64>,
    /// `RATIO_WARNING_TO_CLOSE` 达到预警线未达平仓线比例 (inferred)
    pub ratio_warning_to_close: Option<f64>,
    /// `RATIO_ABOVE_CLOSE` 达到平仓线比例 (inferred)
    pub ratio_above_close: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_gpzy_distribute_statistics_company_em()`
/// (akshare `stock_gpzy_em.py:312`, filter `PFORG_TYPE="证券"`).
pub async fn stock_gpzy_distribute_statistics_company_em(
    client: &Client,
) -> Result<Vec<GpzyDistributeStatisticsRow>> {
    let filter = "(PFORG_TYPE=\"证券\")";
    let params = [
        ("sortColumns", "ORG_NUM"),
        ("sortTypes", "-1"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("reportName", "RPT_GDZY_ZYJG_SUM"),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_gpzy_distribute_statistics_company_em",
            BASE,
            &params,
        )
        .await?;
    parse_stock_gpzy_distribute_statistics_company_em(&v)
}

/// Parse a datacenter `result.data` array into [`GpzyDistributeStatisticsRow`]s
/// (证券公司 variant).
pub(crate) fn parse_stock_gpzy_distribute_statistics_company_em(
    resp: &Value,
) -> Result<Vec<GpzyDistributeStatisticsRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let org_name = fstr(item, "ORG_NAME").unwrap_or_default();
        if org_name.is_empty() {
            continue;
        }
        out.push(GpzyDistributeStatisticsRow {
            org_name,
            pledge_company_num: fnum(item, "PLEDGE_COMPANY_NUM"),
            pledge_num: fnum(item, "PLEDGE_NUM"),
            pledge_shares: fnum(item, "PLEDGE_SHARES"),
            ratio_below_warning: fnum(item, "RATIO_BELOW_WARNING"),
            ratio_warning_to_close: fnum(item, "RATIO_WARNING_TO_CLOSE"),
            ratio_above_close: fnum(item, "RATIO_ABOVE_CLOSE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_gpzy_distribute_statistics_bank_em — 质押机构分布统计-银行
// ===========================================================================

/// Port of `stock_gpzy_distribute_statistics_bank_em()`
/// (akshare `stock_gpzy_em.py:381`, filter `PFORG_TYPE="银行"`).
pub async fn stock_gpzy_distribute_statistics_bank_em(
    client: &Client,
) -> Result<Vec<GpzyDistributeStatisticsRow>> {
    let filter = "(PFORG_TYPE=\"银行\")";
    let params = [
        ("sortColumns", "ORG_NUM"),
        ("sortTypes", "-1"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("reportName", "RPT_GDZY_ZYJG_SUM"),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_gpzy_distribute_statistics_bank_em",
            BASE,
            &params,
        )
        .await?;
    parse_stock_gpzy_distribute_statistics_bank_em(&v)
}

/// Parse a datacenter `result.data` array into [`GpzyDistributeStatisticsRow`]s
/// (银行 variant).
pub(crate) fn parse_stock_gpzy_distribute_statistics_bank_em(
    resp: &Value,
) -> Result<Vec<GpzyDistributeStatisticsRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let org_name = fstr(item, "ORG_NAME").unwrap_or_default();
        if org_name.is_empty() {
            continue;
        }
        out.push(GpzyDistributeStatisticsRow {
            org_name,
            pledge_company_num: fnum(item, "PLEDGE_COMPANY_NUM"),
            pledge_num: fnum(item, "PLEDGE_NUM"),
            pledge_shares: fnum(item, "PLEDGE_SHARES"),
            ratio_below_warning: fnum(item, "RATIO_BELOW_WARNING"),
            ratio_warning_to_close: fnum(item, "RATIO_WARNING_TO_CLOSE"),
            ratio_above_close: fnum(item, "RATIO_ABOVE_CLOSE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_gpzy_industry_data_em — 上市公司质押比例-行业数据
// ===========================================================================

/// One industry pledge-statistic row, port of `stock_gpzy_industry_data_em`
/// (Eastmoney `RPT_CSDC_INDUSTRY_STATISTICS`). Field names are the **real**
/// upstream keys (akshare requests an explicit `columns` list).
#[derive(Debug, Clone, serde::Serialize)]
pub struct GpzyIndustryDataRow {
    /// `INDUSTRY` 行业
    pub industry: String,
    /// `TRADE_DATE` 统计时间
    pub trade_date: String,
    /// `AVERAGE_PLEDGE_RATIO` 平均质押比例
    pub average_pledge_ratio: Option<f64>,
    /// `ORG_NUM` 公司家数
    pub company_num: Option<f64>,
    /// `PLEDGE_TOTAL_NUM` 质押总笔数
    pub pledge_total_num: Option<f64>,
    /// `TOTAL_PLEDGE_SHARES` 质押总股本
    pub total_pledge_shares: Option<f64>,
    /// `PLEDGE_TOTAL_MARKETCAP` 最新质押市值
    pub pledge_total_marketcap: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_gpzy_industry_data_em()` (akshare `stock_gpzy_em.py:450`).
pub async fn stock_gpzy_industry_data_em(client: &Client) -> Result<Vec<GpzyIndustryDataRow>> {
    let params = [
        ("sortColumns", "AVERAGE_PLEDGE_RATIO"),
        ("sortTypes", "-1"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("reportName", "RPT_CSDC_INDUSTRY_STATISTICS"),
        (
            "columns",
            "INDUSTRY_CODE,INDUSTRY,TRADE_DATE,AVERAGE_PLEDGE_RATIO,ORG_NUM,\
PLEDGE_TOTAL_NUM,TOTAL_PLEDGE_SHARES,PLEDGE_TOTAL_MARKETCAP",
        ),
        ("source", "WEB"),
        ("client", "WEB"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_gpzy_industry_data_em",
            BASE,
            &params,
        )
        .await?;
    parse_stock_gpzy_industry_data_em(&v)
}

/// Parse a datacenter `result.data` array into [`GpzyIndustryDataRow`]s.
pub(crate) fn parse_stock_gpzy_industry_data_em(resp: &Value) -> Result<Vec<GpzyIndustryDataRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let industry = fstr(item, "INDUSTRY").unwrap_or_default();
        let trade_date = fstr(item, "TRADE_DATE").unwrap_or_default();
        if industry.is_empty() || trade_date.is_empty() {
            continue;
        }
        out.push(GpzyIndustryDataRow {
            industry,
            trade_date,
            average_pledge_ratio: fnum(item, "AVERAGE_PLEDGE_RATIO"),
            company_num: fnum(item, "ORG_NUM"),
            pledge_total_num: fnum(item, "PLEDGE_TOTAL_NUM"),
            total_pledge_shares: fnum(item, "TOTAL_PLEDGE_SHARES"),
            pledge_total_marketcap: fnum(item, "PLEDGE_TOTAL_MARKETCAP"),
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
    fn parses_stock_gpzy_profile_em() {
        let rows = parse_stock_gpzy_profile_em(&fixture("stock_gpzy_profile_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].trade_date, "2024-09-06");
        // akshare divides A股质押总比例 by 100
        assert_eq!(rows[0].pledge_ratio, Some(12.34 / 100.0));
        assert_eq!(rows[0].pledge_company_num, Some(2500.0));
        assert_eq!(rows[0].pledge_num, Some(5000.0));
        assert_eq!(rows[0].pledge_total_shares, Some(6_000_000_000.0));
        assert_eq!(rows[0].pledge_total_marketcap, Some(50_000_000_000.0));
        assert_eq!(rows[0].csi300_index, Some(3200.5));
        assert_eq!(rows[0].change_rate, Some(-1.23));
        // None case
        assert_eq!(rows[1].csi300_index, None);
        assert_eq!(rows[1].pledge_ratio, Some(12.5 / 100.0));
        assert_eq!(rows[1].source, "eastmoney");
    }

    #[test]
    fn parses_stock_gpzy_pledge_ratio_em() {
        let rows =
            parse_stock_gpzy_pledge_ratio_em(&fixture("stock_gpzy_pledge_ratio_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "600519");
        assert_eq!(rows[0].name, "贵州茅台");
        assert_eq!(rows[0].trade_date, "2024-09-06");
        assert_eq!(rows[0].industry, Some("白酒".to_string()));
        assert_eq!(rows[0].pledge_ratio, Some(1.23));
        assert_eq!(rows[0].pledge_shares, Some(1_234_567.0));
        assert_eq!(rows[0].pledge_marketcap, Some(2_000_000_000.0));
        assert_eq!(rows[0].pledge_num, Some(3.0));
        assert_eq!(rows[0].unrestricted_pledge_shares, Some(1_000_000.0));
        assert_eq!(rows[0].restricted_pledge_shares, Some(234_567.0));
        assert_eq!(rows[0].year_change_rate, Some(-5.6));
        assert_eq!(rows[0].industry_code, Some("BK0001".to_string()));
        // None case
        assert_eq!(rows[1].pledge_shares, None);
        assert_eq!(rows[1].industry_code, Some("BK0002".to_string()));
    }

    #[test]
    fn parses_stock_gpzy_pledge_ratio_detail_em() {
        let rows = parse_stock_gpzy_pledge_ratio_detail_em(
            &fixture("stock_gpzy_pledge_ratio_detail_em.json"),
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "603132");
        assert_eq!(rows[0].name, "某某股份");
        assert_eq!(rows[0].holder_name, Some("张三".to_string()));
        assert_eq!(rows[0].pledge_shares, Some(1_000_000.0));
        assert_eq!(rows[0].pledge_hold_ratio, Some(12.3));
        assert_eq!(rows[0].pledge_total_ratio, Some(1.5));
        assert_eq!(rows[0].pledge_org, Some("中信证券".to_string()));
        assert_eq!(rows[0].latest_price, Some(15.6));
        assert_eq!(rows[0].pledge_price, Some(16.0));
        assert_eq!(rows[0].close_line, Some(12.0));
        assert_eq!(rows[0].pledge_start_date, Some("2024-01-01".to_string()));
        assert_eq!(rows[0].pledge_end_date, Some("2025-01-01".to_string()));
        assert_eq!(rows[0].status, Some("履约中".to_string()));
        assert_eq!(rows[0].notice_date, Some("2024-01-02".to_string()));
        // None cases
        assert_eq!(rows[1].pledge_shares, None);
        assert_eq!(rows[1].latest_price, None);
    }

    #[test]
    fn parses_stock_gpzy_individual_pledge_ratio_detail_em() {
        // Individual query shares the detail parser; it only adds a
        // (SECURITY_CODE="...") filter, which is applied at request time.
        let rows = parse_stock_gpzy_pledge_ratio_detail_em(
            &fixture("stock_gpzy_pledge_ratio_detail_em.json"),
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "603132");
        assert_eq!(rows[0].holder_name, Some("张三".to_string()));
        assert_eq!(rows[1].status, Some("已解押".to_string()));
    }

    #[test]
    fn parses_stock_gpzy_distribute_statistics_company_em() {
        let rows = parse_stock_gpzy_distribute_statistics_company_em(
            &fixture("stock_gpzy_distribute_statistics_company_em.json"),
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].org_name, "中信证券");
        assert_eq!(rows[0].pledge_company_num, Some(120.0));
        assert_eq!(rows[0].pledge_num, Some(300.0));
        assert_eq!(rows[0].pledge_shares, Some(50_000_000.0));
        assert_eq!(rows[0].ratio_below_warning, Some(60.0));
        assert_eq!(rows[0].ratio_warning_to_close, Some(30.0));
        assert_eq!(rows[0].ratio_above_close, Some(10.0));
        // None case
        assert_eq!(rows[1].pledge_shares, None);
        assert_eq!(rows[1].org_name, "华泰证券");
    }

    #[test]
    fn parses_stock_gpzy_distribute_statistics_bank_em() {
        let rows = parse_stock_gpzy_distribute_statistics_bank_em(
            &fixture("stock_gpzy_distribute_statistics_bank_em.json"),
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].org_name, "中国银行");
        assert_eq!(rows[0].pledge_company_num, Some(200.0));
        assert_eq!(rows[0].pledge_num, Some(600.0));
        assert_eq!(rows[0].pledge_shares, Some(90_000_000.0));
        assert_eq!(rows[0].ratio_below_warning, Some(70.0));
        assert_eq!(rows[0].ratio_warning_to_close, Some(25.0));
        assert_eq!(rows[0].ratio_above_close, Some(5.0));
        // None case
        assert_eq!(rows[1].ratio_below_warning, None);
        assert_eq!(rows[1].org_name, "工商银行");
    }

    #[test]
    fn parses_stock_gpzy_industry_data_em() {
        let rows =
            parse_stock_gpzy_industry_data_em(&fixture("stock_gpzy_industry_data_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].industry, "房地产");
        assert_eq!(rows[0].trade_date, "2024-09-06");
        assert_eq!(rows[0].average_pledge_ratio, Some(15.6));
        assert_eq!(rows[0].company_num, Some(100.0));
        assert_eq!(rows[0].pledge_total_num, Some(500.0));
        assert_eq!(rows[0].total_pledge_shares, Some(3_000_000_000.0));
        assert_eq!(rows[0].pledge_total_marketcap, Some(25_000_000_000.0));
        // None case
        assert_eq!(rows[1].pledge_total_marketcap, None);
        assert_eq!(rows[1].industry, "医药生物");
    }
}
