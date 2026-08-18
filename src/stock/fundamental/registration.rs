//! 东方财富网-数据中心-新股数据 (akshare `stock_fundamental` registration/IPO module).
//!
//! Ports the pure-HTTP Eastmoney `datacenter-web` JSON endpoints that back the
//! akshare `stock_fundamental` registration / IPO functions. Every function hits
//! `https://datacenter-web.eastmoney.com/api/data/v1/get` with a `reportName` /
//! `columns` / `filter` query and reads the `result.data` envelope, paginating
//! via `result.pages`. No JS signing, token, encryption, cookie or HTML scraping.
//!
//! | Rust fn                        | akshare fn                     | reportName                  | filter                                          |
//! |--------------------------------|--------------------------------|-----------------------------|-------------------------------------------------|
//! | `stock_register_all_em`        | `stock_register_all_em`       | `RPT_IPO_INFOALLNEW`        | (no filter)                                     |
//! | `stock_register_kcb_em`        | `stock_register_kcb`          | `RPT_IPO_INFOALLNEW`        | `(PREDICT_LISTING_MARKET="科创板")`             |
//! | `stock_register_cyb_em`        | `stock_register_cyb`          | `RPT_IPO_INFOALLNEW`        | `(PREDICT_LISTING_MARKET="创业板")`             |
//! | `stock_register_bj_em`         | `stock_register_bj`           | `RPT_IPO_INFOALLNEW`        | `(PREDICT_LISTING_MARKET="北交所")`             |
//! | `stock_register_sh_em`         | `stock_register_sh`           | `RPT_IPO_INFOALLNEW`        | `(PREDICT_LISTING_MARKET="沪主板")`             |
//! | `stock_register_sz_em`         | `stock_register_sz`           | `RPT_IPO_INFOALLNEW`        | `(PREDICT_LISTING_MARKET="深主板")`             |
//! | `stock_register_db_em`         | `stock_register_db`           | `RPT_KCB_IPO`               | `(ORG_TYPE_CODE="03")`                          |
//! | `stock_ipo_declare_em`         | `stock_ipo_declare_em`        | `RPT_IPO_DECORGNEWEST`      | (no filter)                                     |
//! | `stock_ipo_review_em`          | `stock_ipo_review_em`         | `RPT_IPO_REVIEW`            | (no filter)                                     |
//! | `stock_ipo_tutor_em`           | `stock_ipo_tutor_em`          | `RPT_IPO_TUTRECORD`         | (no filter)                                     |
//! | `stock_profit_forecast_em`     | `stock_profit_forecast_em`    | `RPT_WEB_RESPREDICT`        | `(INDUSTRY_BOARD="{symbol}")` when symbol != "" |
//!
//! akshare source line refs:
//! `akshare/stock_fundamental/stock_register_em.py:16,89,163,237,311,385,459`,
//! `stock_ipo_declare.py:16`, `stock_ipo_review.py:18`, `stock_ipo_tutor.py:18`,
//! `stock_profit_forecast_em.py:15`.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

use crate::core::json::*;

/// Eastmoney source bucket, for rate limiting / error context.
const SOURCE_EASTMONEY: &str = "eastmoney";

/// Eastmoney `datacenter-web` data-center endpoint (shared by every fn here).
const BASE: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";

// ---------------------------------------------------------------------------
// Shared helpers (mirrors `lhb.rs` / `gdfx.rs` convention)
// ---------------------------------------------------------------------------

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

/// Follow Eastmoney `result.pages` pagination, concatenating every `result.data`
/// page. Used by the fns whose akshare source loops over `total_page_num`.
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
// stock_register_* — 注册制审核 (RPT_IPO_INFOALLNEW) + 达标企业 (RPT_KCB_IPO)
// ===========================================================================

/// One IPO registration-review row, port of the `stock_register_*` family
/// (Eastmoney `RPT_IPO_INFOALLNEW`).
///
/// `序号` is the 1-based row index akshare synthesizes; `招股说明书` is rebuilt
/// from `INFO_CODE` as `https://pdf.dfcfw.com/pdf/H2_{INFO_CODE}_1.pdf`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RptIpoInfoallnewRow {
    /// 序号 (synthesized 1-based row index)
    pub seq: usize,
    /// 企业名称 (`DECLARE_ORG`)
    pub declare_org: Option<String>,
    /// 最新状态 (`STATE`)
    pub state: Option<String>,
    /// 注册地 (`REG_ADDRESS`)
    pub reg_address: Option<String>,
    /// 行业 (`CSRC_INDUSTRY`)
    pub csrc_industry: Option<String>,
    /// 保荐机构 (`RECOMMEND_ORG`)
    pub recommend_org: Option<String>,
    /// 律师事务所 (`LAW_FIRM`)
    pub law_firm: Option<String>,
    /// 会计师事务所 (`ACCOUNT_FIRM`)
    pub account_firm: Option<String>,
    /// 更新日期 (`UPDATE_DATE`)
    pub update_date: Option<String>,
    /// 受理日期 (`ACCEPT_DATE`)
    pub accept_date: Option<String>,
    /// 拟上市地点 (`PREDICT_LISTING_MARKET`)
    pub predict_listing_market: Option<String>,
    /// 招股说明书 (derived from `INFO_CODE`: `https://pdf.dfcfw.com/pdf/H2_{INFO_CODE}_1.pdf`)
    pub prospectus: Option<String>,
}

/// Map a registration market key to its Eastmoney `filter` (or `None` for "all").
fn register_market_filter(market: &str) -> Option<&'static str> {
    match market {
        "all" => None,
        "kcb" => Some(r#"(PREDICT_LISTING_MARKET="科创板")"#),
        "cyb" => Some(r#"(PREDICT_LISTING_MARKET="创业板")"#),
        "bj" => Some(r#"(PREDICT_LISTING_MARKET="北交所")"#),
        "sh" => Some(r#"(PREDICT_LISTING_MARKET="沪主板")"#),
        "sz" => Some(r#"(PREDICT_LISTING_MARKET="深主板")"#),
        _ => None,
    }
}

/// Shared fetcher for `RPT_IPO_INFOALLNEW` (the six `stock_register_*` variants).
async fn fetch_register_em(
    client: &Client,
    filter: Option<&'static str>,
    fn_name: &'static str,
) -> Result<Vec<RptIpoInfoallnewRow>> {
    let mut params: Vec<(&str, &str)> = vec![
        ("sortColumns", "UPDATE_DATE,ORG_CODE"),
        ("sortTypes", "-1,-1"),
        ("pageSize", "500"),
        ("reportName", "RPT_IPO_INFOALLNEW"),
        (
            "columns",
            "SECURITY_CODE,STATE,REG_ADDRESS,INFO_CODE,CSRC_INDUSTRY,ACCEPT_DATE,DECLARE_ORG,\
PREDICT_LISTING_MARKET,LAW_FIRM,ACCOUNT_FIRM,ORG_CODE,UPDATE_DATE,RECOMMEND_ORG,IS_REGISTRATION",
        ),
        ("source", "WEB"),
        ("client", "WEB"),
    ];
    if let Some(f) = filter {
        params.push(("filter", f));
    }
    let items = paged(client, fn_name, &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_register_em(&synthetic)
}

/// Port of `stock_register_all_em()` (akshare `stock_register_em.py:16`).
///
/// Returns IPO registration-review rows across all markets.
pub async fn stock_register_all_em(client: &Client) -> Result<Vec<RptIpoInfoallnewRow>> {
    fetch_register_em(
        client,
        register_market_filter("all"),
        "stock_register_all_em",
    )
    .await
}

/// Port of `stock_register_kcb()` (akshare `stock_register_em.py:89`).
///
/// SciTech (STAR) board only — filter `PREDICT_LISTING_MARKET="科创板"`.
pub async fn stock_register_kcb_em(client: &Client) -> Result<Vec<RptIpoInfoallnewRow>> {
    fetch_register_em(
        client,
        register_market_filter("kcb"),
        "stock_register_kcb_em",
    )
    .await
}

/// Port of `stock_register_cyb()` (akshare `stock_register_em.py:163`).
///
/// ChiNext board only — filter `PREDICT_LISTING_MARKET="创业板"`.
pub async fn stock_register_cyb_em(client: &Client) -> Result<Vec<RptIpoInfoallnewRow>> {
    fetch_register_em(
        client,
        register_market_filter("cyb"),
        "stock_register_cyb_em",
    )
    .await
}

/// Port of `stock_register_bj()` (akshare `stock_register_em.py:237`).
///
/// Beijing Stock Exchange only — filter `PREDICT_LISTING_MARKET="北交所"`.
pub async fn stock_register_bj_em(client: &Client) -> Result<Vec<RptIpoInfoallnewRow>> {
    fetch_register_em(client, register_market_filter("bj"), "stock_register_bj_em").await
}

/// Port of `stock_register_sh()` (akshare `stock_register_em.py:311`).
///
/// Shanghai main board only — filter `PREDICT_LISTING_MARKET="沪主板"`.
pub async fn stock_register_sh_em(client: &Client) -> Result<Vec<RptIpoInfoallnewRow>> {
    fetch_register_em(client, register_market_filter("sh"), "stock_register_sh_em").await
}

/// Port of `stock_register_sz()` (akshare `stock_register_em.py:385`).
///
/// Shenzhen main board only — filter `PREDICT_LISTING_MARKET="深主板"`.
pub async fn stock_register_sz_em(client: &Client) -> Result<Vec<RptIpoInfoallnewRow>> {
    fetch_register_em(client, register_market_filter("sz"), "stock_register_sz_em").await
}

/// Parse a datacenter `result.data` array into [`RptIpoInfoallnewRow`]s.
pub(crate) fn parse_stock_register_em(resp: &Value) -> Result<Vec<RptIpoInfoallnewRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let prospectus =
            opt_str(item, "INFO_CODE").map(|c| format!("https://pdf.dfcfw.com/pdf/H2_{}_1.pdf", c));
        out.push(RptIpoInfoallnewRow {
            seq: i + 1,
            declare_org: opt_str(item, "DECLARE_ORG"),
            state: opt_str(item, "STATE"),
            reg_address: opt_str(item, "REG_ADDRESS"),
            csrc_industry: opt_str(item, "CSRC_INDUSTRY"),
            recommend_org: opt_str(item, "RECOMMEND_ORG"),
            law_firm: opt_str(item, "LAW_FIRM"),
            account_firm: opt_str(item, "ACCOUNT_FIRM"),
            update_date: opt_str(item, "UPDATE_DATE"),
            accept_date: opt_str(item, "ACCEPT_DATE"),
            predict_listing_market: opt_str(item, "PREDICT_LISTING_MARKET"),
            prospectus,
        });
    }
    Ok(out)
}

/// One "达标企业" (qualifying enterprise) row, port of `stock_register_db`
/// (Eastmoney `RPT_KCB_IPO`, filter `ORG_TYPE_CODE="03"`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct RptKcbIpoRow {
    /// 序号 (synthesized 1-based row index)
    pub seq: usize,
    /// 企业名称 (`ORG_NAME`)
    pub org_name: Option<String>,
}

/// Port of `stock_register_db()` (akshare `stock_register_em.py:459`).
///
/// Qualifying enterprises (`ORG_TYPE_CODE="03"`), columns `KCB_LB`.
pub async fn stock_register_db_em(client: &Client) -> Result<Vec<RptKcbIpoRow>> {
    let params = [
        ("sortColumns", "NOTICE_DATE,SECURITY_CODE"),
        ("sortTypes", "-1,-1"),
        ("pageSize", "500"),
        ("reportName", "RPT_KCB_IPO"),
        ("columns", "KCB_LB"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", r#"(ORG_TYPE_CODE="03")"#),
    ];
    let items = paged(client, "stock_register_db_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_register_db_em(&synthetic)
}

/// Parse a datacenter `result.data` array into [`RptKcbIpoRow`]s.
pub(crate) fn parse_stock_register_db_em(resp: &Value) -> Result<Vec<RptKcbIpoRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        out.push(RptKcbIpoRow {
            seq: i + 1,
            org_name: opt_str(item, "ORG_NAME"),
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_ipo_declare_em — 首发申报企业信息 (RPT_IPO_DECORGNEWEST)
// ===========================================================================

/// One IPO declaration (首发申报企业) row, port of `stock_ipo_declare_em`
/// (Eastmoney `RPT_IPO_DECORGNEWEST`). `招股说明书` is rebuilt from `INFO_CODE`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RptIpoDecorgnewestRow {
    /// 序号 (synthesized 1-based row index)
    pub seq: usize,
    /// 企业名称 (`DECLARE_ORG`)
    pub declare_org: Option<String>,
    /// 最新状态 (`STATE`)
    pub state: Option<String>,
    /// 注册地 (`REG_ADDRESS`)
    pub reg_address: Option<String>,
    /// 保荐机构 (`RECOMMEND_ORG`)
    pub recommend_org: Option<String>,
    /// 律师事务所 (`LAW_FIRM`)
    pub law_firm: Option<String>,
    /// 会计师事务所 (`ACCOUNT_FIRM`)
    pub account_firm: Option<String>,
    /// 拟上市地点 (`PREDICT_LISTING_MARKET`)
    pub predict_listing_market: Option<String>,
    /// 更新日期 (`END_DATE`)
    pub update_date: Option<String>,
    /// 招股说明书 (derived from `INFO_CODE`: `https://pdf.dfcfw.com/pdf/H2_{INFO_CODE}_1.pdf`)
    pub prospectus: Option<String>,
}

/// Port of `stock_ipo_declare_em()` (akshare `stock_ipo_declare.py:16`).
pub async fn stock_ipo_declare_em(client: &Client) -> Result<Vec<RptIpoDecorgnewestRow>> {
    let params = [
        ("sortColumns", "END_DATE,SECURITY_CODE"),
        ("sortTypes", "-1,-1"),
        ("pageSize", "500"),
        ("reportName", "RPT_IPO_DECORGNEWEST"),
        (
            "columns",
            "DECLARE_ORG,STATE,REG_ADDRESS,RECOMMEND_ORG,LAW_FIRM,ACCOUNT_FIRM,IS_SUBMIT,\
PREDICT_LISTING_MARKET,END_DATE,INFO_CODE,SECURITY_CODE,ORG_CODE,IS_REGISTER,STATE_CODE,\
DERIVE_SECURITY_CODE,ORG_CODE_OLD,IS_STATE",
        ),
        ("source", "WEB"),
        ("client", "WEB"),
    ];
    let items = paged(client, "stock_ipo_declare_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_ipo_declare_em(&synthetic)
}

/// Parse a datacenter `result.data` array into [`RptIpoDecorgnewestRow`]s.
pub(crate) fn parse_stock_ipo_declare_em(resp: &Value) -> Result<Vec<RptIpoDecorgnewestRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let prospectus =
            opt_str(item, "INFO_CODE").map(|c| format!("https://pdf.dfcfw.com/pdf/H2_{}_1.pdf", c));
        out.push(RptIpoDecorgnewestRow {
            seq: i + 1,
            declare_org: opt_str(item, "DECLARE_ORG"),
            state: opt_str(item, "STATE"),
            reg_address: opt_str(item, "REG_ADDRESS"),
            recommend_org: opt_str(item, "RECOMMEND_ORG"),
            law_firm: opt_str(item, "LAW_FIRM"),
            account_firm: opt_str(item, "ACCOUNT_FIRM"),
            predict_listing_market: opt_str(item, "PREDICT_LISTING_MARKET"),
            update_date: opt_str(item, "END_DATE"),
            prospectus,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_ipo_review_em — 新股上会信息 (RPT_IPO_REVIEW)
// ===========================================================================

/// One IPO review (上会) row, port of `stock_ipo_review_em`
/// (Eastmoney `RPT_IPO_REVIEW`, `columns=ALL`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct RptIpoReviewRow {
    /// 序号 (synthesized 1-based row index)
    pub seq: usize,
    /// 企业名称 (`ORG_NAME`)
    pub org_name: Option<String>,
    /// 股票简称 (`SECURITY_NAME_ABBR`)
    pub security_name_abbr: Option<String>,
    /// 股票代码 (`SECURITY_CODE`)
    pub security_code: Option<String>,
    /// 上市板块 (`TRADE_MARKET`)
    pub trade_market: Option<String>,
    /// 上会日期 (`REVIEW_DATE`)
    pub review_date: Option<String>,
    /// 审核状态 (`REVIEW_STATE`)
    pub review_state: Option<String>,
    /// 发审委委员 (`REVIEW_MEMBER`)
    pub review_member: Option<String>,
    /// 主承销商 (`LEAD_UNDERWRITER`)
    pub lead_underwriter: Option<String>,
    /// 发行数量(股) (`ISSUE_NUM`)
    pub issue_num: Option<f64>,
    /// 拟融资额(元) (`FINANCE_AMT_UPPER`)
    pub finance_amt_upper: Option<f64>,
    /// 公告日期 (`NOTICE_DATE`)
    pub notice_date: Option<String>,
    /// 上市日期 (`LISTING_DATE`)
    pub listing_date: Option<String>,
}

/// Port of `stock_ipo_review_em()` (akshare `stock_ipo_review.py:18`).
pub async fn stock_ipo_review_em(client: &Client) -> Result<Vec<RptIpoReviewRow>> {
    let params = [
        ("sortColumns", "REVIEW_DATE,ORG_CODE"),
        ("sortTypes", "-1,-1"),
        ("pageSize", "500"),
        ("reportName", "RPT_IPO_REVIEW"),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
    ];
    let items = paged(client, "stock_ipo_review_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_ipo_review_em(&synthetic)
}

/// Parse a datacenter `result.data` array into [`RptIpoReviewRow`]s.
pub(crate) fn parse_stock_ipo_review_em(resp: &Value) -> Result<Vec<RptIpoReviewRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        out.push(RptIpoReviewRow {
            seq: i + 1,
            org_name: opt_str(item, "ORG_NAME"),
            security_name_abbr: opt_str(item, "SECURITY_NAME_ABBR"),
            security_code: opt_str(item, "SECURITY_CODE"),
            trade_market: opt_str(item, "TRADE_MARKET"),
            review_date: opt_str(item, "REVIEW_DATE"),
            review_state: opt_str(item, "REVIEW_STATE"),
            review_member: opt_str(item, "REVIEW_MEMBER"),
            lead_underwriter: opt_str(item, "LEAD_UNDERWRITER"),
            issue_num: opt_f64(item, "ISSUE_NUM"),
            finance_amt_upper: opt_f64(item, "FINANCE_AMT_UPPER"),
            notice_date: opt_str(item, "NOTICE_DATE"),
            listing_date: opt_str(item, "LISTING_DATE"),
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_ipo_tutor_em — IPO 辅导备案信息 (RPT_IPO_TUTRECORD)
// ===========================================================================

/// One IPO tutoring (辅导备案) row, port of `stock_ipo_tutor_em`
/// (Eastmoney `RPT_IPO_TUTRECORD`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct RptIpoTutrecordRow {
    /// 序号 (synthesized 1-based row index)
    pub seq: usize,
    /// 企业名称 (`TUTOR_OBJECT`)
    pub tutor_object: Option<String>,
    /// 辅导机构 (`TUTOR_ORG`)
    pub tutor_org: Option<String>,
    /// 辅导状态 (`TUTOR_PROCESS_STATE`)
    pub tutor_process_state: Option<String>,
    /// 报告类型 (`REPORT_TYPE`)
    pub report_type: Option<String>,
    /// 派出机构 (`DISPATCH_ORG`)
    pub dispatch_org: Option<String>,
    /// 报告标题 (`REPORT_TITLE`)
    pub report_title: Option<String>,
    /// 备案日期 (`RECORD_DATE`)
    pub record_date: Option<String>,
}

/// Port of `stock_ipo_tutor_em()` (akshare `stock_ipo_tutor.py:18`).
pub async fn stock_ipo_tutor_em(client: &Client) -> Result<Vec<RptIpoTutrecordRow>> {
    let params = [
        ("sortColumns", "RECORD_DATE,TUTOR_OBJECT"),
        ("sortTypes", "-1,-1"),
        ("pageSize", "500"),
        ("reportName", "RPT_IPO_TUTRECORD"),
        (
            "columns",
            "TUTOR_OBJECT,ORG_CODE,TUTOR_ORG_CODE,TUTOR_ORG,TUTOR_PROCESS_STATE,REPORT_TYPE,\
DISPATCH_ORG,REPORT_TITLE,RECORD_DATE",
        ),
        ("source", "WEB"),
        ("client", "WEB"),
    ];
    let items = paged(client, "stock_ipo_tutor_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_ipo_tutor_em(&synthetic)
}

/// Parse a datacenter `result.data` array into [`RptIpoTutrecordRow`]s.
pub(crate) fn parse_stock_ipo_tutor_em(resp: &Value) -> Result<Vec<RptIpoTutrecordRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        out.push(RptIpoTutrecordRow {
            seq: i + 1,
            tutor_object: opt_str(item, "TUTOR_OBJECT"),
            tutor_org: opt_str(item, "TUTOR_ORG"),
            tutor_process_state: opt_str(item, "TUTOR_PROCESS_STATE"),
            report_type: opt_str(item, "REPORT_TYPE"),
            dispatch_org: opt_str(item, "DISPATCH_ORG"),
            report_title: opt_str(item, "REPORT_TITLE"),
            record_date: opt_str(item, "RECORD_DATE"),
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_profit_forecast_em — 盈利预测 (RPT_WEB_RESPREDICT)
// ===========================================================================

/// One earnings-forecast (盈利预测) row, port of `stock_profit_forecast_em`
/// (Eastmoney `RPT_WEB_RESPREDICT`, `columns=WEB_RESPREDICT`).
///
/// Eastmoney returns each row under a single composite `WEB_RESPREDICT` column
/// whose value is a JSON string describing the per-stock forecast; we parse that
/// inner object and lift the relevant fields. `symbol` selects an industry board
/// via the `INDUSTRY_BOARD` filter (empty `symbol` => all boards).
#[derive(Debug, Clone, serde::Serialize)]
pub struct RptWebRespredictRow {
    /// 序号 (synthesized 1-based row index)
    pub seq: usize,
    /// 研报数 (`RATING_ORG_NUM`)
    pub rating_org_num: Option<f64>,
    /// 代码 (`SECURITY_CODE`)
    pub security_code: Option<String>,
    /// 名称 (`SECURITY_NAME_ABBR`)
    pub security_name_abbr: Option<String>,
    /// 机构投资评级(近六个月)-买入 (`NUM_BUY`)
    pub num_buy: Option<f64>,
    /// 机构投资评级(近六个月)-增持 (`NUM_ADD`)
    pub num_add: Option<f64>,
    /// 机构投资评级(近六个月)-中性 (`NUM_NEUTRAL`)
    pub num_neutral: Option<f64>,
    /// 机构投资评级(近六个月)-减持 (`NUM_REDUCE`)
    pub num_reduce: Option<f64>,
    /// 机构投资评级(近六个月)-卖出 (`NUM_SALE`)
    pub num_sale: Option<f64>,
    /// 预测每股收益 year 1 (`PREDICT_EPS1`)
    pub predict_eps1: Option<f64>,
    /// 预测每股收益 year 2 (`PREDICT_EPS2`)
    pub predict_eps2: Option<f64>,
    /// 预测每股收益 year 3 (`PREDICT_EPS3`)
    pub predict_eps3: Option<f64>,
    /// 预测每股收益 year 4 (`PREDICT_EPS4`)
    pub predict_eps4: Option<f64>,
}

/// Port of `stock_profit_forecast_em(symbol)` (akshare `stock_profit_forecast_em.py:15`).
///
/// `symbol` is an industry-board name (e.g. `"船舶制造"`); `""` fetches all boards.
pub async fn stock_profit_forecast_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<RptWebRespredictRow>> {
    let filter = if symbol.is_empty() {
        None
    } else {
        Some(format!(r#"(INDUSTRY_BOARD="{}")"#, symbol))
    };
    let mut params: Vec<(&str, &str)> = vec![
        ("reportName", "RPT_WEB_RESPREDICT"),
        ("columns", "WEB_RESPREDICT"),
        ("pageNumber", "1"),
        ("pageSize", "500"),
        ("sortTypes", "-1"),
        ("sortColumns", "RATING_ORG_NUM"),
        ("p", "1"),
        ("pageNo", "1"),
        ("pageNum", "1"),
    ];
    if let Some(f) = &filter {
        params.push(("filter", f.as_str()));
    }
    let items = paged(client, "stock_profit_forecast_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_profit_forecast_em(&synthetic)
}

/// Parse a datacenter `result.data` array into [`RptWebRespredictRow`]s.
pub(crate) fn parse_stock_profit_forecast_em(resp: &Value) -> Result<Vec<RptWebRespredictRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        // `WEB_RESPREDICT` is a composite column: either an embedded JSON string
        // or (some responses) an inline object. Normalize to a `Value`.
        let inner: Value = match item.get("WEB_RESPREDICT") {
            Some(Value::String(s)) => serde_json::from_str::<Value>(s).map_err(Error::Json)?,
            Some(v @ Value::Object(_)) => v.clone(),
            _ => continue,
        };
        out.push(RptWebRespredictRow {
            seq: i + 1,
            rating_org_num: opt_f64(&inner, "RATING_ORG_NUM"),
            security_code: opt_str(&inner, "SECURITY_CODE"),
            security_name_abbr: opt_str(&inner, "SECURITY_NAME_ABBR"),
            num_buy: opt_f64(&inner, "NUM_BUY"),
            num_add: opt_f64(&inner, "NUM_ADD"),
            num_neutral: opt_f64(&inner, "NUM_NEUTRAL"),
            num_reduce: opt_f64(&inner, "NUM_REDUCE"),
            num_sale: opt_f64(&inner, "NUM_SALE"),
            predict_eps1: opt_f64(&inner, "PREDICT_EPS1"),
            predict_eps2: opt_f64(&inner, "PREDICT_EPS2"),
            predict_eps3: opt_f64(&inner, "PREDICT_EPS3"),
            predict_eps4: opt_f64(&inner, "PREDICT_EPS4"),
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

    // --- registration market filters (pure, maps to the 6 stock_register_* fns) ---

    #[test]
    fn register_filter_all() {
        assert_eq!(register_market_filter("all"), None);
    }

    #[test]
    fn register_filter_kcb() {
        assert_eq!(
            register_market_filter("kcb"),
            Some(r#"(PREDICT_LISTING_MARKET="科创板")"#)
        );
    }

    #[test]
    fn register_filter_cyb() {
        assert_eq!(
            register_market_filter("cyb"),
            Some(r#"(PREDICT_LISTING_MARKET="创业板")"#)
        );
    }

    #[test]
    fn register_filter_bj() {
        assert_eq!(
            register_market_filter("bj"),
            Some(r#"(PREDICT_LISTING_MARKET="北交所")"#)
        );
    }

    #[test]
    fn register_filter_sh() {
        assert_eq!(
            register_market_filter("sh"),
            Some(r#"(PREDICT_LISTING_MARKET="沪主板")"#)
        );
    }

    #[test]
    fn register_filter_sz() {
        assert_eq!(
            register_market_filter("sz"),
            Some(r#"(PREDICT_LISTING_MARKET="深主板")"#)
        );
    }

    // --- parse helpers (one per distinct datacenter report) ---

    #[test]
    fn parses_stock_register_em() {
        let rows = parse_stock_register_em(&fixture("stock_register_all_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].seq, 1);
        assert_eq!(
            rows[0].declare_org,
            Some("中芯国际集成电路制造有限公司".to_string())
        );
        assert_eq!(rows[0].state, Some("已问询".to_string()));
        assert_eq!(rows[0].reg_address, Some("上海市".to_string()));
        assert_eq!(rows[0].csrc_industry, Some("计算机".to_string()));
        assert_eq!(rows[0].recommend_org, Some("中信证券".to_string()));
        assert_eq!(rows[0].law_firm, Some("上海市方达律师事务所".to_string()));
        assert_eq!(
            rows[0].account_firm,
            Some("安永华明会计师事务所".to_string())
        );
        assert_eq!(rows[0].update_date, Some("2023-05-10".to_string()));
        assert_eq!(rows[0].accept_date, Some("2023-03-01".to_string()));
        assert_eq!(rows[0].predict_listing_market, Some("科创板".to_string()));
        assert_eq!(
            rows[0].prospectus,
            Some("https://pdf.dfcfw.com/pdf/H2_1234567890_1.pdf".to_string())
        );
        // None cases (row 2: missing INFO_CODE, ACCEPT_DATE, LAW_FIRM)
        assert_eq!(rows[1].accept_date, None);
        assert_eq!(rows[1].law_firm, None);
        assert_eq!(rows[1].prospectus, None);
    }

    #[test]
    fn parses_stock_register_db_em() {
        let rows = parse_stock_register_db_em(&fixture("stock_register_db_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].seq, 1);
        assert_eq!(rows[0].org_name, Some("北京达标企业有限公司".to_string()));
        assert_eq!(rows[1].org_name, None);
    }

    #[test]
    fn parses_stock_ipo_declare_em() {
        let rows = parse_stock_ipo_declare_em(&fixture("stock_ipo_declare_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].seq, 1);
        assert_eq!(
            rows[0].declare_org,
            Some("贵州茅台酒股份有限公司".to_string())
        );
        assert_eq!(rows[0].state, Some("辅导验收".to_string()));
        assert_eq!(rows[0].predict_listing_market, Some("沪主板".to_string()));
        assert_eq!(rows[0].update_date, Some("2023-07-01".to_string()));
        assert_eq!(
            rows[0].prospectus,
            Some("https://pdf.dfcfw.com/pdf/H2_9876543210_1.pdf".to_string())
        );
        // None cases (row 2: missing LAW_FIRM, END_DATE, INFO_CODE)
        assert_eq!(rows[1].law_firm, None);
        assert_eq!(rows[1].update_date, None);
        assert_eq!(rows[1].prospectus, None);
    }

    #[test]
    fn parses_stock_ipo_review_em() {
        let rows = parse_stock_ipo_review_em(&fixture("stock_ipo_review_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].seq, 1);
        assert_eq!(
            rows[0].org_name,
            Some("常州某智能制造股份有限公司".to_string())
        );
        assert_eq!(rows[0].security_code, Some("601000".to_string()));
        assert_eq!(rows[0].review_state, Some("通过".to_string()));
        assert_eq!(rows[0].issue_num, Some(50_000_000.0));
        assert_eq!(rows[0].finance_amt_upper, Some(2_500_000_000.0));
        assert_eq!(rows[0].listing_date, Some("2023-09-20".to_string()));
        // None cases (row 2: missing REVIEW_MEMBER, ISSUE_NUM, LISTING_DATE)
        assert_eq!(rows[1].review_member, None);
        assert_eq!(rows[1].issue_num, None);
        assert_eq!(rows[1].listing_date, None);
    }

    #[test]
    fn parses_stock_ipo_tutor_em() {
        let rows = parse_stock_ipo_tutor_em(&fixture("stock_ipo_tutor_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].seq, 1);
        assert_eq!(
            rows[0].tutor_object,
            Some("江苏某生物医药股份有限公司".to_string())
        );
        assert_eq!(rows[0].tutor_org, Some("中信证券".to_string()));
        assert_eq!(rows[0].tutor_process_state, Some("辅导备案".to_string()));
        assert_eq!(rows[0].dispatch_org, Some("江苏证监局".to_string()));
        // None cases (row 2: missing REPORT_TYPE, RECORD_DATE)
        assert_eq!(rows[1].report_type, None);
        assert_eq!(rows[1].record_date, None);
    }

    #[test]
    fn parses_stock_profit_forecast_em() {
        let rows =
            parse_stock_profit_forecast_em(&fixture("stock_profit_forecast_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].seq, 1);
        assert_eq!(rows[0].security_code, Some("600519".to_string()));
        assert_eq!(rows[0].security_name_abbr, Some("贵州茅台".to_string()));
        assert_eq!(rows[0].rating_org_num, Some(35.0));
        assert_eq!(rows[0].num_buy, Some(20.0));
        assert_eq!(rows[0].num_add, Some(10.0));
        assert_eq!(rows[0].num_neutral, Some(3.0));
        assert_eq!(rows[0].num_reduce, Some(1.0));
        assert_eq!(rows[0].num_sale, Some(1.0));
        assert_eq!(rows[0].predict_eps1, Some(59.8));
        assert_eq!(rows[0].predict_eps2, Some(67.2));
        assert_eq!(rows[0].predict_eps3, Some(74.1));
        assert_eq!(rows[0].predict_eps4, Some(81.5));
        // None cases (row 2: missing NUM_BUY, NUM_SALE, PREDICT_EPS2)
        assert_eq!(rows[1].num_buy, None);
        assert_eq!(rows[1].num_sale, None);
        assert_eq!(rows[1].predict_eps2, None);
    }
}
