//! Selected `akshare/stock_feature/*` indicators that hit **pure HTTP** JSON
//! endpoints — Eastmoney `datacenter-web` / `dataapi` / `push2ex`, and the Futu
//! quote API. No JS-signing, token/session, HTML-scrape or Excel download.
//!
//! | Rust fn | akshare source | endpoint |
//! |---|---|---|
//! | `stock_analyst_rank_em` | `stock_analyst_em.py:15` | `data.eastmoney.com/dataapi/invest/list` (`RPT_ANALYST_INDEX_RANK`) |
//! | `stock_analyst_detail_em` | `stock_analyst_em.py:105` | `datacenter.eastmoney.com/special/api/data/v1/get` (`RPT_RESEARCHER_*`) |
//! | `stock_comment_detail_zlkp_jgcyd_em` | `stock_comment_em.py:120` | `datacenter-web` `RPT_DMSK_TS_STOCKEVALUATE` |
//! | `stock_concept_cons_futu` | `stock_concept_futu.py:103` | `futunn.com/quote-api/.../get-plate-stock` (JSON path) |
//! | `stock_dxsyl_em` | `stock_dxsyl_em.py:18` | `datacenter-web` `RPTA_APP_IPOAPPLY` |
//! | `stock_fhps_detail_em` | `stock_fhps_em.py:141` | `datacenter-web` `RPT_SHAREBONUS_DET` |
//! | `stock_fhps_em` | `stock_fhps_em.py:15` | `datacenter-web` `RPT_SHAREBONUS_DET` |
//! | `stock_changes_em` | `stock_pankou_em.py:13` | `push2ex.eastmoney.com/getAllStockChanges` |
//! | `stock_board_change_em` | `stock_pankou_em.py:83` | `push2ex.eastmoney.com/getAllBKChanges` |
//!
//! ## DEFERRED (not ported — see reasons)
//!
//! * `get_cookie_csrf` (`stock_a_indicator.py:20`) — **token/session helper**:
//!   fetches a page and extracts an `_csrf` meta token + cookies for legulegu.
//! * `get_token_lg` (`stock_a_indicator.py:40`) — **token/session helper**:
//!   MD5 date token for legulegu.
//! * `stock_a_all_pb` (`stock_all_pb.py:15`) — **legulegu token + cookie-csrf**.
//! * `stock_a_congestion_lg` (`stock_congestion_lg.py:15`) — **legulegu token + cookie-csrf**.
//! * `stock_a_gxl_lg` (`stock_gxl_lg.py:15`) — **legulegu token + cookie-csrf**.
//! * `stock_a_ttm_lyr` (`stock_a_pe_and_pb.py`, `RPT_*_PE`) — **legulegu token +
//!   cookie-csrf + `py_mini_racer` JS signing**.
//! * `stock_buffett_index_lg` (`stock_buffett_index_lg.py:15`) — **legulegu token + cookie-csrf**.
//! * `stock_ebs_lg` (`stock_ebs_lg.py:15`) — **legulegu token + cookie-csrf**.
//! * `stock_board_concept_index_ths` / `_info_ths` / `_name_ths` / `_summary_ths`
//!   (`stock_board_concept_ths.py`) — **`py_mini_racer` JS signing (`ths.js`)**
//!   + HTML scrape.
//! * `stock_board_industry_index_ths` / `_info_ths` / `_name_ths` / `_summary_ths`
//!   (`stock_board_industry_ths.py`) — **`py_mini_racer` JS signing (`ths.js`)**
//!   + HTML scrape.
//! * `stock_classify_board` (`stock_classify_sina.py:17`) — **returns a nested
//!   dict** and parses embedded `<font>` HTML inside the JSON via BeautifulSoup.
//! * `stock_classify_sina` (`stock_classify_sina.py:48`) — depends on
//!   `stock_classify_board` (embedded-HTML board dict) + multi-page Sina JSON.
//! * `stock_cyq_em` (`stock_cyq_em.py:16`) — **`py_mini_racer` JS engine** runs the
//!   `CYQCalculator` to compute the chip distribution per row.
//! * `stock_fhps_detail_ths` (`stock_fhps_ths.py:15`) — **HTML table scrape**
//!   (`pd.read_html`) + THS.
//! * `stock_fund_flow_big_deal` (`stock_fund_flow.py:349`) — **`py_mini_racer`
//!   JS signing (`ths.js`)** + HTML table scrape.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};
use crate::core::json::*;

const SOURCE_FUTU: &str = "futu";

const BASE: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";
const SPECIAL: &str = "https://datacenter.eastmoney.com/special/api/data/v1/get";
const DATA_API: &str = "https://data.eastmoney.com/dataapi/invest/list";
const PUSH2EX_CHANGES: &str = "https://push2ex.eastmoney.com/getAllStockChanges";
const PUSH2EX_BK: &str = "https://push2ex.eastmoney.com/getAllBKChanges";
const FUTU_PLATE: &str = "https://www.futunn.com/quote-api/quote-v2/get-plate-stock";

/// Static, well-known Eastmoney `ut` token used by the `push2ex` pankou
/// endpoints (a literal constant in akshare, not JS-signed).
const UT: &str = "7eea3edcaed734bea9cbfc24409ed989";

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------


/// Extract `result.data` (the row array) from an Eastmoney datacenter / dataapi
/// response.
fn em_data_array(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data".into(),
        })
}

/// Fetch every page of an Eastmoney `result.data` / `result.pages` endpoint and
/// return the concatenated row array. `params` must NOT include `pageNumber`
/// (it is injected here for pagination, bounded to 100 pages).
async fn emdc_fetch_all(
    client: &Client,
    fn_name: &'static str,
    url: &str,
    params: &[(&str, &str)],
) -> Result<Vec<Value>> {
    let first = client.get_json(SOURCE_EASTMONEY, fn_name, url, params).await?;
    let pages = first
        .get("result")
        .and_then(|r| r.get("pages"))
        .and_then(|p| p.as_i64())
        .unwrap_or(1)
        .max(1);
    let pages = (pages as usize).min(100);
    let mut out = em_data_array(&first)?.clone();
    for page in 2..=pages {
        let page_buf = page.to_string();
        let mut p: Vec<(&str, &str)> = params.to_vec();
        p.push(("pageNumber", page_buf.as_str()));
        let v = client.get_json(SOURCE_EASTMONEY, fn_name, url, &p).await?;
        if let Ok(d) = em_data_array(&v) {
            out.extend_from_slice(d);
        }
    }
    Ok(out)
}

/// Extract `data.allstock` (the row array) from a `push2ex` getAllStockChanges
/// response.
fn p2ex_allstock(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("data")
        .and_then(|d| d.get("allstock"))
        .and_then(|x| x.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.allstock".into(),
        })
}

/// Extract `data.allbk` (the row array) from a `push2ex` getAllBKChanges response.
fn p2ex_allbk(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("data")
        .and_then(|d| d.get("allbk"))
        .and_then(|x| x.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.allbk".into(),
        })
}

/// Extract `data.list` (the row array) from a Futu quote-api response.
fn futu_list(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("data")
        .and_then(|d| d.get("list"))
        .and_then(|x| x.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_FUTU,
            message: "missing data.list".into(),
        })
}

// ---------------------------------------------------------------------------
// stock_analyst_rank_em  (stock_analyst_em.py:15)
// ---------------------------------------------------------------------------

/// A row of the Eastmoney analyst index ranking board.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AnalystRankRow {
    /// 序号 (synthesized 1-based row index)
    pub seq: usize,
    /// 分析师ID (Eastmoney `ANALYST_CODE`)
    pub analyst_id: String,
    /// 分析师名称 (Eastmoney `ANALYST_NAME`)
    pub analyst_name: String,
    /// 分析师单位 (Eastmoney `ORG_NAME`)
    pub org_name: String,
    /// 年度 (Eastmoney `YEAR`)
    pub year: Option<String>,
    /// 年度指数 (Eastmoney `INDEX_VALUE`)
    pub index_value: Option<f64>,
    /// <year>年收益率 (Eastmoney `YEAR_YIELD`)
    pub year_yield: Option<f64>,
    /// 3个月收益率 (Eastmoney `YIELD_3`)
    pub yield_3m: Option<f64>,
    /// 6个月收益率 (Eastmoney `YIELD_6`)
    pub yield_6m: Option<f64>,
    /// 12个月收益率 (Eastmoney `YIELD_12`)
    pub yield_12m: Option<f64>,
    /// 成分股个数 (Eastmoney `SECURITY_COUNT`)
    pub security_count: Option<f64>,
    /// <year>最新个股评级-股票名称 (Eastmoney `SECURITY_NAME_ABBR`)
    pub security_name: Option<String>,
    /// <year>最新个股评级-股票代码 (Eastmoney `SECURITY_CODE`)
    pub security_code: Option<String>,
    /// 行业代码 (Eastmoney `INDUSTRY_CODE`)
    pub industry_code: Option<String>,
    /// 行业 (Eastmoney `INDUSTRY_NAME`)
    pub industry_name: Option<String>,
    /// 更新日期 (Eastmoney `TRADE_DATE`)
    pub update_date: Option<String>,
}

/// Parse `stock_analyst_rank_em` rows from a `result.data` array.
pub(crate) fn parse_analyst_rank(items: &[Value]) -> Result<Vec<AnalystRankRow>> {
    let mut out = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let Some(analyst_id) = opt_str(item, "ANALYST_CODE") else {
            continue;
        };
        let Some(analyst_name) = opt_str(item, "ANALYST_NAME") else {
            continue;
        };
        out.push(AnalystRankRow {
            seq: i + 1,
            analyst_id,
            analyst_name,
            org_name: opt_str(item, "ORG_NAME").unwrap_or_default(),
            year: opt_str(item, "YEAR"),
            index_value: opt_f64(item, "INDEX_VALUE"),
            year_yield: opt_f64(item, "YEAR_YIELD"),
            yield_3m: opt_f64(item, "YIELD_3"),
            yield_6m: opt_f64(item, "YIELD_6"),
            yield_12m: opt_f64(item, "YIELD_12"),
            security_count: opt_f64(item, "SECURITY_COUNT"),
            security_name: opt_str(item, "SECURITY_NAME_ABBR"),
            security_code: opt_str(item, "SECURITY_CODE"),
            industry_code: opt_str(item, "INDUSTRY_CODE"),
            industry_name: opt_str(item, "INDUSTRY_NAME"),
            update_date: opt_str(item, "TRADE_DATE"),
        });
    }
    Ok(out)
}

/// 东方财富-分析师指数-排名 (Eastmoney `RPT_ANALYST_INDEX_RANK`, dataapi).
/// `year` ∈ {2015..now}; default `year="2024"`.
pub async fn stock_analyst_rank_em(client: &Client, year: &str) -> Result<Vec<AnalystRankRow>> {
    let filter = format!(r#"(YEAR="{year}")"#);
    let params = [
        ("sortColumns", "YEAR_YIELD"),
        ("sortTypes", "-1"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("reportName", "RPT_ANALYST_INDEX_RANK"),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
        ("distinct", "ANALYST_CODE"),
        ("limit", "top100"),
    ];
    let items = emdc_fetch_all(client, "stock_analyst_rank_em", DATA_API, &params).await?;
    parse_analyst_rank(&items)
}

// ---------------------------------------------------------------------------
// stock_analyst_detail_em  (stock_analyst_em.py:105)
// ---------------------------------------------------------------------------

/// A row of an Eastmoney analyst detail view. The three `indicator` variants
/// populate different subsets of the (all-`Option`) fields below.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AnalystDetailRow {
    /// 序号 (synthesized 1-based row index)
    pub seq: usize,
    // 最新跟踪成分股 (RPT_RESEARCHER_NTCSTOCK)
    /// 股票代码 (Eastmoney `SECURITY_CODE`)
    pub stock_code: Option<String>,
    /// 股票名称 (Eastmoney `SECURITY_NAME_ABBR`)
    pub stock_name: Option<String>,
    /// 调入日期 (Eastmoney `INTO_DATE`)
    pub into_date: Option<String>,
    /// 最新评级日期 (Eastmoney `CHANGE_DATE`)
    pub rating_date: Option<String>,
    /// 当前评级名称 (Eastmoney `RATING_NAME`)
    pub rating_name: Option<String>,
    /// 成交价格(前复权) (Eastmoney `DEAL_PRICE`)
    pub deal_price: Option<f64>,
    /// 最新价格 (Eastmoney `CLOSE_PRICE`)
    pub close_price: Option<f64>,
    /// 阶段涨跌幅 (Eastmoney `CHANGE_RATIO`)
    pub change_ratio: Option<f64>,
    // 历史跟踪成分股 (RPT_RESEARCHER_HISTORYSTOCK)
    /// 调出日期 (Eastmoney `OUT_DATE`)
    pub out_date: Option<String>,
    /// 调入时评级名称 (Eastmoney `INTO_RATING_NAME`)
    pub into_rating_name: Option<String>,
    /// 调出原因 (Eastmoney `OUT_REASON`)
    pub out_reason: Option<String>,
    /// 累计涨跌幅 (Eastmoney `CUMULATIVE_CHANGE`)
    pub cumulative_change: Option<f64>,
    // 历史指数 (RPT_RESEARCHER_DETAILS)
    /// 交易日 (Eastmoney `TRADE_DATE`)
    pub trade_date: Option<String>,
    /// 指数 (Eastmoney `INDEX_HVALUE`)
    pub index_value: Option<f64>,
}

/// Parse `stock_analyst_detail_em` rows from a `result.data` array. The
/// `indicator` selects which column layout applies.
pub(crate) fn parse_analyst_detail(items: &[Value], indicator: &str) -> Result<Vec<AnalystDetailRow>> {
    let mut out = Vec::with_capacity(items.len());
    match indicator {
        "最新跟踪成分股" => {
            for (i, item) in items.iter().enumerate() {
                out.push(AnalystDetailRow {
                    seq: i + 1,
                    stock_code: opt_str(item, "SECURITY_CODE"),
                    stock_name: opt_str(item, "SECURITY_NAME_ABBR"),
                    into_date: opt_str(item, "INTO_DATE"),
                    rating_date: opt_str(item, "CHANGE_DATE"),
                    rating_name: opt_str(item, "RATING_NAME"),
                    deal_price: opt_f64(item, "DEAL_PRICE"),
                    close_price: opt_f64(item, "CLOSE_PRICE"),
                    change_ratio: opt_f64(item, "CHANGE_RATIO"),
                    out_date: None,
                    into_rating_name: None,
                    out_reason: None,
                    cumulative_change: None,
                    trade_date: None,
                    index_value: None,
                });
            }
        }
        "历史跟踪成分股" => {
            for (i, item) in items.iter().enumerate() {
                out.push(AnalystDetailRow {
                    seq: i + 1,
                    stock_code: opt_str(item, "SECURITY_CODE"),
                    stock_name: opt_str(item, "SECURITY_NAME_ABBR"),
                    into_date: opt_str(item, "INTO_DATE"),
                    rating_date: None,
                    rating_name: None,
                    deal_price: None,
                    close_price: None,
                    change_ratio: None,
                    out_date: opt_str(item, "OUT_DATE"),
                    into_rating_name: opt_str(item, "INTO_RATING_NAME"),
                    out_reason: opt_str(item, "OUT_REASON"),
                    cumulative_change: opt_f64(item, "CUMULATIVE_CHANGE"),
                    trade_date: None,
                    index_value: None,
                });
            }
        }
        "历史指数" => {
            for (i, item) in items.iter().enumerate() {
                out.push(AnalystDetailRow {
                    seq: i + 1,
                    stock_code: None,
                    stock_name: None,
                    into_date: None,
                    rating_date: None,
                    rating_name: None,
                    deal_price: None,
                    close_price: None,
                    change_ratio: None,
                    out_date: None,
                    into_rating_name: None,
                    out_reason: None,
                    cumulative_change: None,
                    trade_date: opt_str(item, "TRADE_DATE"),
                    index_value: opt_f64(item, "INDEX_HVALUE"),
                });
            }
        }
        other => {
            return Err(Error::InvalidParam(format!(
                "unknown analyst_detail indicator: {other}"
            )))
        }
    }
    Ok(out)
}

/// 东方财富-分析师详情 (Eastmoney `RPT_RESEARCHER_*`, datacenter special api).
/// `indicator` ∈ {最新跟踪成分股, 历史跟踪成分股, 历史指数}; default 最新跟踪成分股.
pub async fn stock_analyst_detail_em(
    client: &Client,
    analyst_id: &str,
    indicator: &str,
) -> Result<Vec<AnalystDetailRow>> {
    let (report_name, sort_columns) = match indicator {
        "最新跟踪成分股" => ("RPT_RESEARCHER_NTCSTOCK", "CHANGE_DATE"),
        "历史跟踪成分股" => ("RPT_RESEARCHER_HISTORYSTOCK", "CHANGE_DATE"),
        "历史指数" => ("RPT_RESEARCHER_DETAILS", "TRADE_DATE"),
        _ => {
            return Err(Error::InvalidParam(format!(
                "unknown analyst_detail indicator: {indicator}"
            )))
        }
    };
    let filter = format!(r#"(ANALYST_CODE="{analyst_id}")"#);
    let params = [
        ("reportName", report_name),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("sortColumns", sort_columns),
        ("sortTypes", "-1"),
        ("pageNumber", "1"),
        ("pageSize", "1000"),
        ("filter", filter.as_str()),
    ];
    let items = emdc_fetch_all(client, "stock_analyst_detail_em", SPECIAL, &params).await?;
    parse_analyst_detail(&items, indicator)
}

// ---------------------------------------------------------------------------
// stock_comment_detail_zlkp_jgcyd_em  (stock_comment_em.py:120)
// ---------------------------------------------------------------------------

/// A row of the 千股千评 机构参与度 (per-trade-date) history.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CommentJgcydRow {
    /// 交易日 (Eastmoney `TRADE_DATE`)
    pub trade_date: Option<String>,
    /// 机构参与度 (Eastmoney `ORG_PARTICIPATE` × 100, matching akshare)
    pub org_participate: Option<f64>,
}

/// Parse `stock_comment_detail_zlkp_jgcyd_em` rows from a `result.data` array.
pub(crate) fn parse_comment_jgcyd(items: &[Value]) -> Result<Vec<CommentJgcydRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let org = opt_f64(item, "ORG_PARTICIPATE").map(|x| x * 100.0);
        out.push(CommentJgcydRow {
            trade_date: opt_str(item, "TRADE_DATE"),
            org_participate: org,
        });
    }
    Ok(out)
}

/// 东方财富-千股千评-主力控盘-机构参与度 (Eastmoney `RPT_DMSK_TS_STOCKEVALUATE`).
pub async fn stock_comment_detail_zlkp_jgcyd_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<CommentJgcydRow>> {
    let filter = format!(r#"(SECURITY_CODE="{symbol}")"#);
    let params = [
        ("reportName", "RPT_DMSK_TS_STOCKEVALUATE"),
        ("filter", filter.as_str()),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("sortColumns", "TRADE_DATE"),
        ("sortTypes", "-1"),
        ("pageSize", "1000"),
        ("pageNumber", "1"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_comment_detail_zlkp_jgcyd_em", BASE, &params)
        .await?;
    let items = em_data_array(&v)?;
    parse_comment_jgcyd(items)
}

// ---------------------------------------------------------------------------
// stock_concept_cons_futu  (stock_concept_futu.py:103)
// ---------------------------------------------------------------------------

/// A constituent stock of a Futu concept plate (JSON API path).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConceptConsFutuRow {
    /// 代码 (Futu `stockCode`)
    pub code: String,
    /// 股票名称 (Futu `name`)
    pub name: String,
    /// 最新价 (Futu `price`, string → f64)
    pub price: Option<f64>,
    /// 涨跌额 (Futu `change`, string → f64)
    pub change: Option<f64>,
    /// 涨跌幅 (Futu `changeRatio`, e.g. "+14.54%")
    pub change_ratio: Option<String>,
    /// 成交量 (Futu `tradeVolumn`, e.g. "5.95M")
    pub volume: Option<String>,
    /// 成交额 (Futu `tradeTrunover`, e.g. "65.85M")
    pub turnover: Option<String>,
}

/// Parse `stock_concept_cons_futu` rows from a `data.list` array.
pub(crate) fn parse_concept_cons_futu(items: &[Value]) -> Result<Vec<ConceptConsFutuRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(code) = opt_str(item, "stockCode") else {
            continue;
        };
        let Some(name) = opt_str(item, "name") else {
            continue;
        };
        out.push(ConceptConsFutuRow {
            code,
            name,
            price: opt_f64(item, "price"),
            change: opt_f64(item, "change"),
            change_ratio: opt_str(item, "changeRatio"),
            volume: opt_str(item, "tradeVolumn"),
            turnover: opt_str(item, "tradeTrunover"),
        });
    }
    Ok(out)
}

/// 富途牛牛-概念板块-成分股 (Futu `get-plate-stock` JSON API).
///
/// Only the pure-HTTP JSON path is implemented: the default `symbol="特朗普概念股"`
/// maps to plate `10102960`. The other akshare symbols (巴菲特持仓, 佩洛西持仓)
/// are served by an HTML-scraped endpoint and return `InvalidParam`.
pub async fn stock_concept_cons_futu(client: &Client, symbol: &str) -> Result<Vec<ConceptConsFutuRow>> {
    let plate_id = match symbol {
        "特朗普概念股" => "10102960",
        _ => {
            return Err(Error::InvalidParam(format!(
                "stock_concept_cons_futu only supports '特朗普概念股' (HTML-scrape path for '{symbol}' is deferred)"
            )))
        }
    };
    let params = [
        ("marketType", "2"),
        ("plateId", plate_id),
        ("page", "0"),
        ("pageSize", "30"),
    ];
    let headers = [("Quote-Token", "7f74cd2a5e")];
    let v = client
        .get_json_with_headers(
            SOURCE_FUTU,
            "stock_concept_cons_futu",
            FUTU_PLATE,
            &params,
            Some(&headers),
        )
        .await?;
    let items = futu_list(&v)?;
    parse_concept_cons_futu(items)
}

// ---------------------------------------------------------------------------
// stock_dxsyl_em  (stock_dxsyl_em.py:18)
// ---------------------------------------------------------------------------

/// A row of the Eastmoney 打新收益率 board.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DxsylRow {
    /// 股票代码 (Eastmoney `SECURITY_CODE`)
    pub security_code: String,
    /// 股票简称 (Eastmoney `f14`)
    pub security_name: Option<String>,
    /// 发行价 (Eastmoney `ISSUE_PRICE`)
    pub issue_price: Option<f64>,
    /// 最新价 (Eastmoney `LATELY_PRICE`)
    pub lately_price: Option<f64>,
    /// 网上-发行中签率 (Eastmoney `ONLINE_ISSUE_LWR`)
    pub online_issue_lwr: Option<f64>,
    /// 网上-有效申购股数 (Eastmoney `ONLINE_VA_SHARES`)
    pub online_va_shares: Option<f64>,
    /// 网上-有效申购户数 (Eastmoney `ONLINE_VA_NUM`)
    pub online_va_num: Option<f64>,
    /// 网上-超额认购倍数 (Eastmoney `ONLINE_ES_MULTIPLE`)
    pub online_es_multiple: Option<f64>,
    /// 网下-配售中签率 (Eastmoney `OFFLINE_VAP_RATIO`)
    pub offline_vap_ratio: Option<f64>,
    /// 网下-有效申购股数 (Eastmoney `OFFLINE_VATS`)
    pub offline_vats: Option<f64>,
    /// 网下-有效申购户数 (Eastmoney `OFFLINE_VAP_OBJECT`)
    pub offline_vap_object: Option<f64>,
    /// 网下-配售认购倍数 (Eastmoney `OFFLINE_VAS_MULTIPLE`)
    pub offline_vas_multiple: Option<f64>,
    /// 总发行数量 (Eastmoney `ISSUE_NUM`)
    pub issue_num: Option<f64>,
    /// 开盘溢价 (Eastmoney `LD_OPEN_PREMIUM`)
    pub ld_open_premium: Option<f64>,
    /// 首日涨幅 (Eastmoney `LD_CLOSE_CHANGE`)
    pub ld_close_change: Option<f64>,
    /// 上市日期 (Eastmoney `LISTING_DATE`)
    pub listing_date: Option<String>,
}

/// Parse `stock_dxsyl_em` rows from a `result.data` array.
pub(crate) fn parse_dxsyl(items: &[Value]) -> Result<Vec<DxsylRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(security_code) = opt_str(item, "SECURITY_CODE") else {
            continue;
        };
        out.push(DxsylRow {
            security_code,
            security_name: opt_str(item, "f14"),
            issue_price: opt_f64(item, "ISSUE_PRICE"),
            lately_price: opt_f64(item, "LATELY_PRICE"),
            online_issue_lwr: opt_f64(item, "ONLINE_ISSUE_LWR"),
            online_va_shares: opt_f64(item, "ONLINE_VA_SHARES"),
            online_va_num: opt_f64(item, "ONLINE_VA_NUM"),
            online_es_multiple: opt_f64(item, "ONLINE_ES_MULTIPLE"),
            offline_vap_ratio: opt_f64(item, "OFFLINE_VAP_RATIO"),
            offline_vats: opt_f64(item, "OFFLINE_VATS"),
            offline_vap_object: opt_f64(item, "OFFLINE_VAP_OBJECT"),
            offline_vas_multiple: opt_f64(item, "OFFLINE_VAS_MULTIPLE"),
            issue_num: opt_f64(item, "ISSUE_NUM"),
            ld_open_premium: opt_f64(item, "LD_OPEN_PREMIUM"),
            ld_close_change: opt_f64(item, "LD_CLOSE_CHANGE"),
            listing_date: opt_str(item, "LISTING_DATE"),
        });
    }
    Ok(out)
}

/// 东方财富-打新收益率 (Eastmoney `RPTA_APP_IPOAPPLY`, datacenter-web).
pub async fn stock_dxsyl_em(client: &Client) -> Result<Vec<DxsylRow>> {
    let filter = r#"((APPLY_DATE>'2010-01-01')(|@APPLY_DATE="NULL"))((LISTING_DATE>'2010-01-01')(|@LISTING_DATE="NULL"))(TRADE_MARKET_CODE!="069001017")"#;
    let params = [
        ("sortColumns", "LISTING_DATE,SECURITY_CODE"),
        ("sortTypes", "-1,-1"),
        ("pageSize", "5000"),
        ("pageNumber", "1"),
        ("reportName", "RPTA_APP_IPOAPPLY"),
        ("quoteColumns", "f2~01~SECURITY_CODE,f14~01~SECURITY_CODE"),
        ("quoteType", "0"),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter),
    ];
    let items = emdc_fetch_all(client, "stock_dxsyl_em", BASE, &params).await?;
    parse_dxsyl(&items)
}

// ---------------------------------------------------------------------------
// stock_fhps_em  (stock_fhps_em.py:15)
// ---------------------------------------------------------------------------

/// A 分红送配 row (Eastmoney `RPT_SHAREBONUS_DET`, by report date).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FhpsRow {
    /// 股票代码 (Eastmoney `SECURITY_CODE`)
    pub security_code: String,
    /// 股票简称 (Eastmoney `SECURITY_NAME_ABBR`)
    pub security_name: Option<String>,
    /// 送转股份-送转总比例 (Eastmoney `BONUS_IT_RATIO`)
    pub bonus_it_ratio: Option<f64>,
    /// 送转股份-送股比例 (Eastmoney `BONUS_RATIO`)
    pub bonus_ratio: Option<f64>,
    /// 送转股份-转股比例 (Eastmoney `IT_RATIO`)
    pub it_ratio: Option<f64>,
    /// 现金分红-现金分红比例 (Eastmoney `PRETAX_BONUS_RMB`)
    pub pretax_bonus_rmb: Option<f64>,
    /// 现金分红-股息率 (Eastmoney `DIVIDENT_RATIO`)
    pub dividend_ratio: Option<f64>,
    /// 每股收益 (Eastmoney `BASIC_EPS`)
    pub basic_eps: Option<f64>,
    /// 每股净资产 (Eastmoney `BVPS`)
    pub bvps: Option<f64>,
    /// 每股公积金 (Eastmoney `PER_CAPITAL_RESERVE`)
    pub per_capital_reserve: Option<f64>,
    /// 每股未分配利润 (Eastmoney `PER_UNASSIGN_PROFIT`)
    pub per_unassign_profit: Option<f64>,
    /// 净利润同比增长 (Eastmoney `PNP_YOY_RATIO`)
    pub pnp_yoy_ratio: Option<f64>,
    /// 总股本 (Eastmoney `TOTAL_SHARES`)
    pub total_shares: Option<f64>,
    /// 预案公告日 (Eastmoney `PLAN_NOTICE_DATE`)
    pub plan_notice_date: Option<String>,
    /// 股权登记日 (Eastmoney `EQUITY_RECORD_DATE`)
    pub equity_record_date: Option<String>,
    /// 除权除息日 (Eastmoney `EX_DIVIDEND_DATE`)
    pub ex_dividend_date: Option<String>,
    /// 方案进度 (Eastmoney `ASSIGN_PROGRESS`)
    pub assign_progress: Option<String>,
    /// 最新公告日期 (Eastmoney `NOTICE_DATE`)
    pub notice_date: Option<String>,
}

/// Parse `stock_fhps_em` rows from a `result.data` array.
pub(crate) fn parse_fhps(items: &[Value]) -> Result<Vec<FhpsRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(security_code) = opt_str(item, "SECURITY_CODE") else {
            continue;
        };
        out.push(FhpsRow {
            security_code,
            security_name: opt_str(item, "SECURITY_NAME_ABBR"),
            bonus_it_ratio: opt_f64(item, "BONUS_IT_RATIO"),
            bonus_ratio: opt_f64(item, "BONUS_RATIO"),
            it_ratio: opt_f64(item, "IT_RATIO"),
            pretax_bonus_rmb: opt_f64(item, "PRETAX_BONUS_RMB"),
            dividend_ratio: opt_f64(item, "DIVIDENT_RATIO"),
            basic_eps: opt_f64(item, "BASIC_EPS"),
            bvps: opt_f64(item, "BVPS"),
            per_capital_reserve: opt_f64(item, "PER_CAPITAL_RESERVE"),
            per_unassign_profit: opt_f64(item, "PER_UNASSIGN_PROFIT"),
            pnp_yoy_ratio: opt_f64(item, "PNP_YOY_RATIO"),
            total_shares: opt_f64(item, "TOTAL_SHARES"),
            plan_notice_date: opt_str(item, "PLAN_NOTICE_DATE"),
            equity_record_date: opt_str(item, "EQUITY_RECORD_DATE"),
            ex_dividend_date: opt_str(item, "EX_DIVIDEND_DATE"),
            assign_progress: opt_str(item, "ASSIGN_PROGRESS"),
            notice_date: opt_str(item, "NOTICE_DATE"),
        });
    }
    Ok(out)
}

/// 东方财富-分红送配 (Eastmoney `RPT_SHAREBONUS_DET`, by report date).
/// `date` is `YYYYMMDD`; default `date="20231231"`.
pub async fn stock_fhps_em(client: &Client, date: &str) -> Result<Vec<FhpsRow>> {
    let (y, m, d) = (
        &date[..4],
        &date[4..6],
        &date[6..8],
    );
    let filter = format!("(REPORT_DATE='{y}-{m}-{d}')");
    let params = [
        ("sortColumns", "PLAN_NOTICE_DATE"),
        ("sortTypes", "-1"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("reportName", "RPT_SHAREBONUS_DET"),
        ("columns", "ALL"),
        ("quoteColumns", ""),
        ("js", r#"{"data":(x),"pages":(tp)}"#),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
    ];
    let items = emdc_fetch_all(client, "stock_fhps_em", BASE, &params).await?;
    parse_fhps(&items)
}

// ---------------------------------------------------------------------------
// stock_fhps_detail_em  (stock_fhps_em.py:141)
// ---------------------------------------------------------------------------

/// A 分红送配 detail row (Eastmoney `RPT_SHAREBONUS_DET`, by security code).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FhpsDetailRow {
    /// 报告期 (Eastmoney `REPORT_DATE`)
    pub report_date: Option<String>,
    /// 业绩披露日期 (Eastmoney `PUBLISH_DATE`)
    pub publish_date: Option<String>,
    /// 送转股份-送转总比例 (Eastmoney `BONUS_IT_RATIO`)
    pub bonus_it_ratio: Option<f64>,
    /// 送转股份-送股比例 (Eastmoney `BONUS_RATIO`)
    pub bonus_ratio: Option<f64>,
    /// 送转股份-转股比例 (Eastmoney `IT_RATIO`)
    pub it_ratio: Option<f64>,
    /// 现金分红-现金分红比例 (Eastmoney `PRETAX_BONUS_RMB`)
    pub pretax_bonus_rmb: Option<f64>,
    /// 现金分红-现金分红比例描述 (Eastmoney `IMPL_PLAN_PROFILE`)
    pub impl_plan_profile: Option<String>,
    /// 现金分红-股息率 (Eastmoney `DIVIDENT_RATIO`)
    pub dividend_ratio: Option<f64>,
    /// 每股收益 (Eastmoney `BASIC_EPS`)
    pub basic_eps: Option<f64>,
    /// 每股净资产 (Eastmoney `BVPS`)
    pub bvps: Option<f64>,
    /// 每股公积金 (Eastmoney `PER_CAPITAL_RESERVE`)
    pub per_capital_reserve: Option<f64>,
    /// 每股未分配利润 (Eastmoney `PER_UNASSIGN_PROFIT`)
    pub per_unassign_profit: Option<f64>,
    /// 净利润同比增长 (Eastmoney `PNP_YOY_RATIO`)
    pub pnp_yoy_ratio: Option<f64>,
    /// 总股本 (Eastmoney `TOTAL_SHARES`)
    pub total_shares: Option<f64>,
    /// 预案公告日 (Eastmoney `PLAN_NOTICE_DATE`)
    pub plan_notice_date: Option<String>,
    /// 股权登记日 (Eastmoney `EQUITY_RECORD_DATE`)
    pub equity_record_date: Option<String>,
    /// 除权除息日 (Eastmoney `EX_DIVIDEND_DATE`)
    pub ex_dividend_date: Option<String>,
    /// 方案进度 (Eastmoney `ASSIGN_PROGRESS`)
    pub assign_progress: Option<String>,
    /// 最新公告日期 (Eastmoney `NOTICE_DATE`)
    pub notice_date: Option<String>,
}

/// Parse `stock_fhps_detail_em` rows from a `result.data` array.
pub(crate) fn parse_fhps_detail(items: &[Value]) -> Result<Vec<FhpsDetailRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(FhpsDetailRow {
            report_date: opt_str(item, "REPORT_DATE"),
            publish_date: opt_str(item, "PUBLISH_DATE"),
            bonus_it_ratio: opt_f64(item, "BONUS_IT_RATIO"),
            bonus_ratio: opt_f64(item, "BONUS_RATIO"),
            it_ratio: opt_f64(item, "IT_RATIO"),
            pretax_bonus_rmb: opt_f64(item, "PRETAX_BONUS_RMB"),
            impl_plan_profile: opt_str(item, "IMPL_PLAN_PROFILE"),
            dividend_ratio: opt_f64(item, "DIVIDENT_RATIO"),
            basic_eps: opt_f64(item, "BASIC_EPS"),
            bvps: opt_f64(item, "BVPS"),
            per_capital_reserve: opt_f64(item, "PER_CAPITAL_RESERVE"),
            per_unassign_profit: opt_f64(item, "PER_UNASSIGN_PROFIT"),
            pnp_yoy_ratio: opt_f64(item, "PNP_YOY_RATIO"),
            total_shares: opt_f64(item, "TOTAL_SHARES"),
            plan_notice_date: opt_str(item, "PLAN_NOTICE_DATE"),
            equity_record_date: opt_str(item, "EQUITY_RECORD_DATE"),
            ex_dividend_date: opt_str(item, "EX_DIVIDEND_DATE"),
            assign_progress: opt_str(item, "ASSIGN_PROGRESS"),
            notice_date: opt_str(item, "NOTICE_DATE"),
        });
    }
    Ok(out)
}

/// 东方财富-分红送配详情 (Eastmoney `RPT_SHAREBONUS_DET`, by security code).
pub async fn stock_fhps_detail_em(client: &Client, symbol: &str) -> Result<Vec<FhpsDetailRow>> {
    let filter = format!(r#"(SECURITY_CODE="{symbol}")"#);
    let params = [
        ("sortColumns", "REPORT_DATE"),
        ("sortTypes", "-1"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("reportName", "RPT_SHAREBONUS_DET"),
        ("columns", "ALL"),
        ("quoteColumns", ""),
        ("js", r#"{"data":(x),"pages":(tp)}"#),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
    ];
    let items = emdc_fetch_all(client, "stock_fhps_detail_em", BASE, &params).await?;
    parse_fhps_detail(&items)
}

// ---------------------------------------------------------------------------
// stock_changes_em  (stock_pankou_em.py:13)
// ---------------------------------------------------------------------------

/// Map a `push2ex` change-type code to its akshare name.
fn changes_board_name(t: i64) -> Option<&'static str> {
    let map: &[(&str, i64)] = &[
        ("火箭发射", 8201),
        ("快速反弹", 8202),
        ("大笔买入", 8193),
        ("封涨停板", 4),
        ("打开跌停板", 32),
        ("有大买盘", 64),
        ("竞价上涨", 8207),
        ("高开5日线", 8209),
        ("向上缺口", 8211),
        ("60日新高", 8213),
        ("60日大幅上涨", 8215),
        ("加速下跌", 8204),
        ("高台跳水", 8203),
        ("大笔卖出", 8194),
        ("封跌停板", 8),
        ("打开涨停板", 16),
        ("有大卖盘", 128),
        ("竞价下跌", 8208),
        ("低开5日线", 8210),
        ("向下缺口", 8212),
        ("60日新低", 8214),
        ("60日大幅下跌", 8216),
    ];
    map.iter().find(|(_, code)| *code == t).map(|(name, _)| *name)
}

/// A 盘口异动 row (Eastmoney `push2ex` getAllStockChanges).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChangesRow {
    /// 时间 (Eastmoney `tm`, HHMMSS integer)
    pub time: Option<f64>,
    /// 代码 (Eastmoney `c`)
    pub code: String,
    /// 名称 (Eastmoney `n`)
    pub name: String,
    /// 板块 (Eastmoney `t`, mapped to the akshare change-type name)
    pub board: Option<String>,
    /// 相关信息 (Eastmoney `i`, e.g. "68500,24.49,0.036,1677565.00")
    pub info: Option<String>,
}

/// Parse `stock_changes_em` rows from a `data.allstock` array.
pub(crate) fn parse_changes(items: &[Value]) -> Result<Vec<ChangesRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(code) = opt_str(item, "c") else {
            continue;
        };
        let Some(name) = opt_str(item, "n") else {
            continue;
        };
        let board = item
            .get("t")
            .and_then(|v| v.as_i64())
            .and_then(changes_board_name)
            .map(str::to_string);
        out.push(ChangesRow {
            time: opt_f64(item, "tm"),
            code,
            name,
            board,
            info: opt_str(item, "i"),
        });
    }
    Ok(out)
}

/// 东方财富-盘口异动 (Eastmoney `push2ex` getAllStockChanges). `symbol` ∈ the
/// 22 akshare change types; default `symbol="大笔买入"`.
pub async fn stock_changes_em(client: &Client, symbol: &str) -> Result<Vec<ChangesRow>> {
    let symbol_map: &[(&str, &str)] = &[
        ("火箭发射", "8201"),
        ("快速反弹", "8202"),
        ("大笔买入", "8193"),
        ("封涨停板", "4"),
        ("打开跌停板", "32"),
        ("有大买盘", "64"),
        ("竞价上涨", "8207"),
        ("高开5日线", "8209"),
        ("向上缺口", "8211"),
        ("60日新高", "8213"),
        ("60日大幅上涨", "8215"),
        ("加速下跌", "8204"),
        ("高台跳水", "8203"),
        ("大笔卖出", "8194"),
        ("封跌停板", "8"),
        ("打开涨停板", "16"),
        ("有大卖盘", "128"),
        ("竞价下跌", "8208"),
        ("低开5日线", "8210"),
        ("向下缺口", "8212"),
        ("60日新低", "8214"),
        ("60日大幅下跌", "8216"),
    ];
    let type_code = symbol_map
        .iter()
        .find(|(k, _)| *k == symbol)
        .map(|(_, v)| *v)
        .ok_or_else(|| Error::InvalidParam(format!("unknown stock_changes_em symbol: {symbol}")))?;
    let params = [
        ("type", type_code),
        ("pageindex", "0"),
        ("pagesize", "5000"),
        ("ut", UT),
        ("dpt", "wzchanges"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_changes_em", PUSH2EX_CHANGES, &params)
        .await?;
    let items = p2ex_allstock(&v)?;
    parse_changes(items)
}

// ---------------------------------------------------------------------------
// stock_board_change_em  (stock_pankou_em.py:83)
// ---------------------------------------------------------------------------

/// A 当日板块异动 row (Eastmoney `push2ex` getAllBKChanges).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BoardChangeRow {
    /// 板块名称 (Eastmoney `n`)
    pub board_name: String,
    /// 涨跌幅 (Eastmoney `u`)
    pub change_pct: Option<f64>,
    /// 主力净流入 (Eastmoney `zjl`)
    pub main_net_in: Option<f64>,
    /// 板块异动总次数 (Eastmoney `ct`)
    pub change_count: Option<f64>,
    /// 板块异动最频繁个股-股票代码 (Eastmoney `ms.c`)
    pub top_stock_code: Option<String>,
    /// 板块异动最频繁个股-股票名称 (Eastmoney `ms.n`)
    pub top_stock_name: Option<String>,
    /// 板块异动最频繁个股-买卖方向 (Eastmoney `ms.m`: 0=大笔买入, 1=大笔卖出)
    pub top_stock_dir: Option<String>,
    /// 板块具体异动类型列表及出现次数 (Eastmoney `ydl`, raw JSON array)
    pub detail_types: Option<Value>,
}

/// Parse `stock_board_change_em` rows from a `data.allbk` array.
pub(crate) fn parse_board_change(items: &[Value]) -> Result<Vec<BoardChangeRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(board_name) = opt_str(item, "n") else {
            continue;
        };
        let (top_code, top_name, top_dir) = match item.get("ms") {
            Some(ms) => (
                opt_str(ms, "c"),
                opt_str(ms, "n"),
                ms.get("m")
                    .and_then(|v| v.as_i64())
                    .map(|m| if m == 0 { "大笔买入" } else { "大笔卖出" }.to_string()),
            ),
            None => (None, None, None),
        };
        out.push(BoardChangeRow {
            board_name,
            change_pct: opt_f64(item, "u"),
            main_net_in: opt_f64(item, "zjl"),
            change_count: opt_f64(item, "ct"),
            top_stock_code: top_code,
            top_stock_name: top_name,
            top_stock_dir: top_dir,
            detail_types: item.get("ydl").cloned(),
        });
    }
    Ok(out)
}

/// 东方财富-当日板块异动详情 (Eastmoney `push2ex` getAllBKChanges).
pub async fn stock_board_change_em(client: &Client) -> Result<Vec<BoardChangeRow>> {
    let params = [
        ("ut", UT),
        ("dpt", "wzchanges"),
        ("pageindex", "0"),
        ("pagesize", "5000"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_board_change_em", PUSH2EX_BK, &params)
        .await?;
    let items = p2ex_allbk(&v)?;
    parse_board_change(items)
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

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

    /// Extract `result.data` from a datacenter/dataapi fixture.
    fn em_data(name: &str) -> Vec<Value> {
        em_data_array(&fixture(name)).unwrap().clone()
    }

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    // ---- stock_analyst_rank_em ----

    #[test]
    fn parse_analyst_rank_ok() {
        let rows = parse_analyst_rank(&em_data("stock_analyst_rank_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].seq, 1);
        assert_eq!(rows[0].analyst_id, "11000213851");
        assert_eq!(rows[0].analyst_name, "任志强");
        assert_eq!(rows[0].org_name, "华福证券");
        assert_eq!(rows[0].year, Some("2024".to_string()));
        assert!(approx(rows[0].index_value, 6424.01));
        assert!(approx(rows[0].year_yield, 135.17));
        assert!(approx(rows[0].yield_3m, 57.0));
        assert!(approx(rows[0].yield_12m, 135.17));
        assert_eq!(rows[0].security_name, Some("寒武纪".to_string()));
        assert_eq!(rows[0].security_code, Some("688256".to_string()));
        assert_eq!(rows[0].industry_name, Some("电子".to_string()));
        assert_eq!(rows[1].analyst_name, "王伟");
    }

    // ---- stock_analyst_detail_em ----

    #[test]
    fn parse_analyst_detail_ntcstock_ok() {
        let rows = parse_analyst_detail(&em_data("stock_analyst_detail_em.json"), "最新跟踪成分股")
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].seq, 1);
        assert_eq!(rows[0].stock_code, Some("600519".to_string()));
        assert_eq!(rows[0].stock_name, Some("贵州茅台".to_string()));
        assert_eq!(rows[0].into_date, Some("2024-01-10 00:00:00".to_string()));
        assert_eq!(rows[0].rating_date, Some("2024-03-01 00:00:00".to_string()));
        assert_eq!(rows[0].rating_name, Some("买入".to_string()));
        assert!(approx(rows[0].deal_price, 1685.0));
        assert!(approx(rows[0].close_price, 1720.5));
        assert!(approx(rows[0].change_ratio, 2.1));
        assert_eq!(rows[1].stock_code, Some("000001".to_string()));
    }

    #[test]
    fn parse_analyst_detail_history_index_ok() {
        let rows = parse_analyst_detail(&em_data("stock_analyst_detail_em.json"), "历史指数").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].trade_date, Some("2024-03-01 00:00:00".to_string()));
        assert!(approx(rows[0].index_value, 6424.01));
        assert_eq!(rows[1].index_value, Some(6300.0));
    }

    #[test]
    fn parse_analyst_detail_bad_indicator() {
        let rows = parse_analyst_detail(&em_data("stock_analyst_detail_em.json"), "nope");
        assert!(rows.is_err());
    }

    // ---- stock_comment_detail_zlkp_jgcyd_em ----

    #[test]
    fn parse_comment_jgcyd_ok() {
        let rows = parse_comment_jgcyd(&em_data("stock_comment_detail_zlkp_jgcyd_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].trade_date, Some("2026-08-14 00:00:00".to_string()));
        // ORG_PARTICIPATE 0.3928764 × 100 = 39.28764
        assert!(approx(rows[0].org_participate, 39.28764));
        assert!(approx(rows[1].org_participate, 12.345));
    }

    // ---- stock_concept_cons_futu ----

    #[test]
    fn parse_concept_cons_futu_ok() {
        let v = fixture("stock_concept_cons_futu.json");
        let items = futu_list(&v).unwrap();
        let rows = parse_concept_cons_futu(items).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "NMAX");
        assert_eq!(rows[0].name, "Newsmax");
        assert!(approx(rows[0].price, 10.87));
        assert!(approx(rows[0].change, 1.38));
        assert_eq!(rows[0].change_ratio, Some("+14.54%".to_string()));
        assert_eq!(rows[0].volume, Some("5.95M".to_string()));
        assert_eq!(rows[0].turnover, Some("65.85M".to_string()));
        assert_eq!(rows[1].code, "DJT");
    }

    // ---- stock_dxsyl_em ----

    #[test]
    fn parse_dxsyl_ok() {
        let rows = parse_dxsyl(&em_data("stock_dxsyl_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].security_code, "600519");
        assert_eq!(rows[0].security_name, Some("贵州茅台".to_string()));
        assert!(approx(rows[0].issue_price, 1234.0));
        assert!(approx(rows[0].online_issue_lwr, 0.034));
        assert!(approx(rows[0].issue_num, 100000000.0));
        assert_eq!(rows[1].security_code, "300750");
        assert!(approx(rows[1].ld_close_change, 45.6));
    }

    // ---- stock_fhps_em ----

    #[test]
    fn parse_fhps_ok() {
        let rows = parse_fhps(&em_data("stock_fhps_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].security_code, "600323");
        assert_eq!(rows[0].security_name, Some("瀚蓝环境".to_string()));
        assert!(approx(rows[0].pretax_bonus_rmb, 4.8));
        assert!(approx(rows[0].dividend_ratio, 0.02194787));
        assert!(approx(rows[0].basic_eps, 1.75));
        assert!(approx(rows[0].total_shares, 815347146.0));
        assert_eq!(rows[0].assign_progress, Some("实施分配".to_string()));
        assert_eq!(rows[1].security_code, "600000");
    }

    // ---- stock_fhps_detail_em ----

    #[test]
    fn parse_fhps_detail_ok() {
        let rows = parse_fhps_detail(&em_data("stock_fhps_detail_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].report_date, Some("2023-12-31 00:00:00".to_string()));
        assert_eq!(rows[0].publish_date, Some("2024-04-12 00:00:00".to_string()));
        assert!(approx(rows[0].bonus_it_ratio, 0.5));
        assert!(approx(rows[0].pretax_bonus_rmb, 4.8));
        assert_eq!(rows[0].impl_plan_profile, Some("10派4.80元".to_string()));
        assert!(approx(rows[0].dividend_ratio, 0.02194787));
        assert_eq!(rows[1].report_date, Some("2022-12-31 00:00:00".to_string()));
    }

    // ---- stock_changes_em ----

    #[test]
    fn parse_changes_ok() {
        let v = fixture("stock_changes_em.json");
        let items = p2ex_allstock(&v).unwrap();
        let rows = parse_changes(items).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "301584");
        assert_eq!(rows[0].name, "建发致新");
        assert!(approx(rows[0].time, 145546.0));
        // type 8193 → 大笔买入
        assert_eq!(rows[0].board, Some("大笔买入".to_string()));
        assert_eq!(rows[0].info, Some("68500,24.49000,0.035956,1677565.00".to_string()));
        // type 8194 → 大笔卖出
        assert_eq!(rows[1].board, Some("大笔卖出".to_string()));
    }

    // ---- stock_board_change_em ----

    #[test]
    fn parse_board_change_ok() {
        let v = fixture("stock_board_change_em.json");
        let items = p2ex_allbk(&v).unwrap();
        let rows = parse_board_change(items).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].board_name, "融资融券");
        assert!(approx(rows[0].change_pct, -0.14));
        assert!(approx(rows[0].main_net_in, -1259995.9552));
        assert!(approx(rows[0].change_count, 7857.0));
        assert_eq!(rows[0].top_stock_code, Some("300862".to_string()));
        assert_eq!(rows[0].top_stock_name, Some("蓝盾光电".to_string()));
        // ms.m = 0 → 大笔买入
        assert_eq!(rows[0].top_stock_dir, Some("大笔买入".to_string()));
        assert!(rows[0].detail_types.is_some());
        assert_eq!(rows[1].board_name, "半导体");
    }
}
