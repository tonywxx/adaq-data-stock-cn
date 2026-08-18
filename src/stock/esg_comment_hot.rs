//! 东方财富网「千股千评」(stock_comment_em) 与 新浪财经「ESG评级中心」(stock_esg_sina)
//! 数据端口。
//!
//! 本模块归属两个 akshare 源文件:
//!
//! * `akshare/stock_feature/stock_comment_em.py` — 东方财富网-数据中心-特色数据-千股千评
//!   (datacenter-web `RPT_*` JSON 接口, 纯 GET, 无 JS 签名)
//! * `akshare/stock_feature/stock_esg_sina.py` — 新浪财经-ESG评级中心
//!   (`global.finance.sina.com.cn/api/openapi.php/EsgService.*` 纯 JSON 接口, 无 JS 签名)
//!
//! | Rust function | akshare source | 源 | 形态 |
//! |---|---|---|---|
//! | `stock_esg_hz_sina` | `stock_esg_sina.py:267` | sina | JSON GET (分页) |
//! | `stock_esg_msci_sina` | `stock_esg_sina.py:16` | sina | JSON GET (分页) |
//! | `stock_esg_rate_sina` | `stock_esg_sina.py:167` | sina | JSON GET (分页, 嵌套) |
//! | `stock_esg_rft_sina` | `stock_esg_sina.py:103` | sina | JSON GET (单页) |
//! | `stock_esg_zd_sina` | `stock_esg_sina.py:221` | sina | JSON GET (分页) |
//! | `stock_comment_em` | `stock_comment_em.py:19` | eastmoney | datacenter-web GET (分页) |
//! | `stock_comment_detail_scrd_focus_em` | `stock_comment_em.py:188` | eastmoney | datacenter-web GET |
//! | `stock_comment_detail_zhpj_lspf_em` | `stock_comment_em.py:151` | eastmoney | datacenter-web GET |
//! | `stock_comment_detail_scrd_desire_em` | `stock_comment_em.py:226` | eastmoney | datacenter-web JSONP GET |
//!
//! ## DEFERRED
//!
//! * 整组 `stock_hot_rank_em.py` (**6 个函数**) 已 **全部 DEFER**:
//!   `stock_hot_rank_em` (`:13`), `stock_hot_rank_detail_em` (`:67`),
//!   `stock_hot_rank_detail_realtime_em` (`:104`), `stock_hot_keyword_em` (`:127`),
//!   `stock_hot_rank_latest_em` (`:150`), `stock_hot_rank_relate_em` (`:174`).
//!   原因: 这些端点位于 `emappdata.eastmoney.com/stockrank/*`, akshare 用
//!   `requests.post(json=payload)` 发送 **JSON 请求体**。本 crate 的
//!   `Client` 仅暴露 `get_json` / `get_text` / `post_form_json`(表单编码),
//!   **没有「原始 JSON 请求体」的 POST**; 而任务硬性规定「不得触碰 `client.rs`
//!   等任何其他文件」, 故无法在不改 `client.rs` 的前提下忠实实现, 因此 DEFER
//!   (规则: DEFER, don't fake)。

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use crate::core::json::*;

const SOURCE_SINA: &str = "sina";
const SOURCE_EASTMONEY: &str = "eastmoney";

/// Sina ESG openapi base.
const SINA_ESG_BASE: &str = "https://global.finance.sina.com.cn/api/openapi.php";

/// Eastmoney datacenter-web base (千股千评).
const EM_BASE: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";

/// 千股千评 `stock_comment_em` 的静态 `token` (akshare 源码硬编码, 非按请求签名).
const COMMENT_EM_TOKEN: &str = "894050c76af8597a853f5b408b759f5d";

// ===========================================================================
// Shared helpers
// ===========================================================================

/// Extract `result.data` (the row array) from a datacenter-web response.
fn emg_data_array(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data".into(),
        })
}

/// Extract the inner `result.data.data` array from a Sina ESG JSON response.
fn sina_esg_data(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.get("data"))
        .and_then(|a| a.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "missing result.data.data".into(),
        })
}

/// Read a string field (null/other -> None).
/// Unwrap a JSONP envelope `callback(...)` into a JSON `Value`.
fn unwrap_jsonp(text: &str) -> Result<Value> {
    let s = text.trim();
    let start = s.find('(').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "JSONP response missing '('".into(),
    })?;
    let end = s.rfind(')').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "JSONP response missing ')'".into(),
    })?;
    if end <= start {
        return Err(Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "JSONP response has unbalanced parentheses".into(),
        });
    }
    serde_json::from_str(&s[start + 1..end]).map_err(Error::Json)
}

// ===========================================================================
// Sina ESG — 华证指数 (stock_esg_sina.py:267)
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct EsgHzRow {
    pub date: Option<String>,
    pub symbol: String,
    pub market: Option<String>,
    pub name: Option<String>,
    pub esg_score: Option<f64>,
    pub esg_grade: Option<String>,
    pub env_score: Option<f64>,
    pub env_grade: Option<String>,
    pub social_score: Option<f64>,
    pub social_grade: Option<String>,
    pub gov_score: Option<f64>,
    pub gov_grade: Option<String>,
}

/// Parse `stock_esg_hz_sina` rows from a Sina `getHzEsgStocks` `result.data.data` array.
pub(crate) fn parse_esg_hz(items: &[Value]) -> Result<Vec<EsgHzRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(symbol) = opt_str(item, "symbol") else {
            continue;
        };
        out.push(EsgHzRow {
            date: opt_str(item, "date"),
            symbol,
            market: opt_str(item, "market"),
            name: opt_str(item, "name"),
            esg_score: opt_f64(item, "esg_score"),
            esg_grade: opt_str(item, "esg_score_grade"),
            env_score: opt_f64(item, "e_score"),
            env_grade: opt_str(item, "e_score_grade"),
            social_score: opt_f64(item, "s_score"),
            social_grade: opt_str(item, "s_score_grade"),
            gov_score: opt_f64(item, "g_score"),
            gov_grade: opt_str(item, "g_score_grade"),
        });
    }
    Ok(out)
}

/// 新浪财经-ESG评级中心-ESG评级-华证指数 (akshare `stock_esg_sina.py:267`).
pub async fn stock_esg_hz_sina(client: &Client) -> Result<Vec<EsgHzRow>> {
    let data = sina_esg_paged(
        client,
        "stock_esg_hz_sina",
        "EsgService.getHzEsgStocks",
        "100",
    )
    .await?;
    parse_esg_hz(&data)
}

// ===========================================================================
// Sina ESG — MSCI (stock_esg_sina.py:16)
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct EsgMsciRow {
    pub symbol: String,
    pub esg_rating: Option<f64>,
    pub env_score: Option<f64>,
    pub social_score: Option<f64>,
    pub governance_score: Option<f64>,
    pub rating_date: Option<String>,
    pub market: Option<String>,
}

/// Parse `stock_esg_msci_sina` rows from a Sina `getMsciEsgStocks` `result.data.data` array.
pub(crate) fn parse_esg_msci(items: &[Value]) -> Result<Vec<EsgMsciRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(symbol) = opt_str(item, "symbol") else {
            continue;
        };
        out.push(EsgMsciRow {
            symbol,
            esg_rating: opt_f64(item, "esg_rating"),
            env_score: opt_f64(item, "env_score"),
            social_score: opt_f64(item, "social_score"),
            governance_score: opt_f64(item, "governance_score"),
            rating_date: opt_str(item, "quarter_date"),
            market: opt_str(item, "market"),
        });
    }
    Ok(out)
}

/// 新浪财经-ESG评级中心-ESG评级-MSCI (akshare `stock_esg_sina.py:16`).
pub async fn stock_esg_msci_sina(client: &Client) -> Result<Vec<EsgMsciRow>> {
    let data = sina_esg_paged(
        client,
        "stock_esg_msci_sina",
        "EsgService.getMsciEsgStocks",
        "100",
    )
    .await?;
    parse_esg_msci(&data)
}

// ===========================================================================
// Sina ESG — 秩鼎 (stock_esg_sina.py:221)
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct EsgZdRow {
    pub symbol: String,
    pub esg_score: Option<f64>,
    pub env_score: Option<f64>,
    pub social_score: Option<f64>,
    pub governance_score: Option<f64>,
    pub rating_date: Option<String>,
}

/// Parse `stock_esg_zd_sina` rows from a Sina `getZdEsgStocks` `result.data.data` array.
pub(crate) fn parse_esg_zd(items: &[Value]) -> Result<Vec<EsgZdRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(symbol) = opt_str(item, "ticker") else {
            continue;
        };
        out.push(EsgZdRow {
            symbol,
            esg_score: opt_f64(item, "esg_score"),
            env_score: opt_f64(item, "environmental_score"),
            social_score: opt_f64(item, "social_score"),
            governance_score: opt_f64(item, "governance_score"),
            rating_date: opt_str(item, "report_date"),
        });
    }
    Ok(out)
}

/// 新浪财经-ESG评级中心-ESG评级-秩鼎 (akshare `stock_esg_sina.py:221`).
pub async fn stock_esg_zd_sina(client: &Client) -> Result<Vec<EsgZdRow>> {
    let data = sina_esg_paged(
        client,
        "stock_esg_zd_sina",
        "EsgService.getZdEsgStocks",
        "100",
    )
    .await?;
    parse_esg_zd(&data)
}

// ===========================================================================
// Sina ESG — 路孚特 (stock_esg_sina.py:103) — single page (num=20000)
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct EsgRftRow {
    pub symbol: String,
    pub esg_score: Option<f64>,
    pub esg_score_date: Option<String>,
    pub env_score: Option<f64>,
    pub env_score_date: Option<String>,
    pub social_score: Option<f64>,
    pub social_score_date: Option<String>,
    pub governance_score: Option<f64>,
    pub governance_score_date: Option<String>,
    pub dispute_score: Option<f64>,
    pub dispute_score_date: Option<String>,
    pub industry: Option<String>,
    pub exchange: Option<String>,
}

/// Parse `stock_esg_rft_sina` rows from a Sina `getRftEsgStocks` `result.data.data` array.
pub(crate) fn parse_esg_rft(items: &[Value]) -> Result<Vec<EsgRftRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(symbol) = opt_str(item, "symbol") else {
            continue;
        };
        out.push(EsgRftRow {
            symbol,
            esg_score: opt_f64(item, "esg_score"),
            esg_score_date: opt_str(item, "esg_score_date"),
            env_score: opt_f64(item, "env_score"),
            env_score_date: opt_str(item, "env_score_date"),
            social_score: opt_f64(item, "social_score"),
            social_score_date: opt_str(item, "social_score_date"),
            governance_score: opt_f64(item, "governance_score"),
            governance_score_date: opt_str(item, "governance_score_date"),
            dispute_score: opt_f64(item, "zy_score"),
            dispute_score_date: opt_str(item, "zy_score_date"),
            industry: opt_str(item, "industry"),
            exchange: opt_str(item, "exchange"),
        });
    }
    Ok(out)
}

/// 新浪财经-ESG评级中心-ESG评级-路孚特 (akshare `stock_esg_sina.py:103`).
pub async fn stock_esg_rft_sina(client: &Client) -> Result<Vec<EsgRftRow>> {
    let url = format!("{SINA_ESG_BASE}/EsgService.getRftEsgStocks");
    let headers = [("Referer", "https://finance.sina.com.cn/")];
    let v = client
        .get_json_with_headers(
            SOURCE_SINA,
            "stock_esg_rft_sina",
            &url,
            &[("num", "20000")],
            Some(&headers),
        )
        .await?;
    let data = sina_esg_data(&v)?.clone();
    parse_esg_rft(&data)
}

// ===========================================================================
// Sina ESG — ESG评级数据 (stock_esg_sina.py:167) — nested stocks[].esg_info
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct EsgRateRow {
    pub symbol: String,
    pub market: Option<String>,
    pub agency_name: Option<String>,
    /// 评级 (akshare 列 `评级`, 来自 `esg_score`, 通常为字母等级).
    pub rating: Option<String>,
    /// 评级季度 (akshare 列 `评级季度`, 来自 `esg_dt`).
    pub rating_quarter: Option<String>,
    pub remark: Option<String>,
}

/// Expand a Sina `getEsgStocks` `stocks` array into one row per `esg_info`
/// entry, tagging each with its parent `symbol`/`market`. Pure (no I/O).
pub(crate) fn expand_esg_rate(stocks: &[Value]) -> Vec<Value> {
    let mut out = Vec::new();
    for st in stocks {
        let symbol = st.get("symbol").cloned().unwrap_or(Value::Null);
        let market = st.get("market").cloned().unwrap_or(Value::Null);
        if let Some(infos) = st.get("esg_info").and_then(|v| v.as_array()) {
            for info in infos {
                let mut obj = info.clone();
                if let Value::Object(ref mut m) = obj {
                    m.insert("symbol".to_string(), symbol.clone());
                    m.insert("market".to_string(), market.clone());
                }
                out.push(obj);
            }
        }
    }
    out
}

/// Parse `stock_esg_rate_sina` rows from the already-expanded array
/// (`expand_esg_rate` output: `{symbol, market, agency_name, esg_score, esg_dt, remark}`).
pub(crate) fn parse_esg_rate(items: &[Value]) -> Result<Vec<EsgRateRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(symbol) = opt_str(item, "symbol") else {
            continue;
        };
        out.push(EsgRateRow {
            symbol,
            market: opt_str(item, "market"),
            agency_name: opt_str(item, "agency_name"),
            rating: opt_str(item, "esg_score"),
            rating_quarter: opt_str(item, "esg_dt"),
            remark: opt_str(item, "remark"),
        });
    }
    Ok(out)
}

/// 新浪财经-ESG评级中心-ESG评级-ESG评级数据 (akshare `stock_esg_sina.py:167`).
pub async fn stock_esg_rate_sina(client: &Client) -> Result<Vec<EsgRateRow>> {
    let url = format!("{SINA_ESG_BASE}/EsgService.getEsgStocks");
    let headers = [("Referer", "https://finance.sina.com.cn/")];
    let v1 = client
        .get_json_with_headers(
            SOURCE_SINA,
            "stock_esg_rate_sina",
            &url,
            &[("page", "1"), ("num", "200")],
            Some(&headers),
        )
        .await?;
    let total = v1
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.get("info"))
        .and_then(|i| i.get("total"))
        .and_then(|t| t.as_i64())
        .unwrap_or(0);
    let page_size: i64 = 200;
    let pages = ((total + page_size - 1) / page_size).max(1);
    let mut stocks: Vec<Value> = v1
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.get("info"))
        .and_then(|i| i.get("stocks"))
        .and_then(|a| a.as_array())
        .map(|a| a.to_vec())
        .unwrap_or_default();
    for p in 2..=pages {
        let p_s = p.to_string();
        let v = client
            .get_json_with_headers(
                SOURCE_SINA,
                "stock_esg_rate_sina",
                &url,
                &[("page", p_s.as_str()), ("num", "200")],
                Some(&headers),
            )
            .await?;
        if let Some(arr) = v
            .get("result")
            .and_then(|r| r.get("data"))
            .and_then(|d| d.get("info"))
            .and_then(|i| i.get("stocks"))
            .and_then(|a| a.as_array())
        {
            stocks.extend(arr.iter().cloned());
        }
    }
    let expanded = expand_esg_rate(&stocks);
    parse_esg_rate(&expanded)
}

// ===========================================================================
// Shared Sina ESG paginated fetch (getHzEsgStocks / getMsciEsgStocks / getZdEsgStocks)
// ===========================================================================

/// Fetch a paginated Sina ESG endpoint whose response nests rows under
/// `result.data.data` and total count under `result.data.total`; returns the
/// concatenated inner row array across all pages.
async fn sina_esg_paged(
    client: &Client,
    fn_name: &'static str,
    service: &str,
    page_size: &str,
) -> Result<Vec<Value>> {
    let url = format!("{SINA_ESG_BASE}/{service}");
    let headers = [("Referer", "https://finance.sina.com.cn/")];
    let v1 = client
        .get_json_with_headers(
            SOURCE_SINA,
            fn_name,
            &url,
            &[("p", "1"), ("num", page_size)],
            Some(&headers),
        )
        .await?;
    let total = v1
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.get("total"))
        .and_then(|t| t.as_i64())
        .unwrap_or(0);
    let ps: i64 = page_size.parse().unwrap_or(100);
    let pages = ((total + ps - 1) / ps).max(1);
    let mut all: Vec<Value> = sina_esg_data(&v1)?.to_vec();
    for p in 2..=pages {
        let p_s = p.to_string();
        let v = client
            .get_json_with_headers(
                SOURCE_SINA,
                fn_name,
                &url,
                &[("p", p_s.as_str()), ("num", page_size)],
                Some(&headers),
            )
            .await?;
        all.extend(sina_esg_data(&v)?.iter().cloned());
    }
    Ok(all)
}

// ===========================================================================
// Eastmoney 千股千评 — stock_comment_em (stock_comment_em.py:19)
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct CommentRow {
    pub index: usize,
    pub symbol: String,
    pub name: Option<String>,
    pub close_price: Option<f64>,
    pub change_rate: Option<f64>,
    pub turnover_rate: Option<f64>,
    pub main_cost: Option<f64>,
    pub pe_dynamic: Option<f64>,
    pub org_participate: Option<f64>,
    pub total_score: Option<f64>,
    pub rise: Option<f64>,
    pub current_rank: Option<f64>,
    pub attention_index: Option<f64>,
    pub trade_date: Option<String>,
}

/// Parse `stock_comment_em` rows from a datacenter-web `result.data` array.
///
/// Field keys mirror akshare's positional column mapping against Eastmoney's
/// `RPT_DMSK_TS_STOCKNEW` (columns=ALL) response. `index` is the 1-based row
/// ordinal akshare assigns after `reset_index`.
pub(crate) fn parse_comment_em(items: &[Value]) -> Result<Vec<CommentRow>> {
    let mut out = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let Some(symbol) = opt_str(item, "SECURITY_CODE") else {
            continue;
        };
        out.push(CommentRow {
            index: i + 1,
            symbol,
            name: opt_str(item, "SECURITY_NAME_ABBR"),
            close_price: opt_f64(item, "CLOSE_PRICE"),
            change_rate: opt_f64(item, "CHANGE_RATE"),
            turnover_rate: opt_f64(item, "TURNOVERRATE"),
            main_cost: opt_f64(item, "MAIN_COST"),
            pe_dynamic: opt_f64(item, "PE_DYNAMIC"),
            org_participate: opt_f64(item, "ORG_PARTICIPATE"),
            total_score: opt_f64(item, "TOTAL_SCORE"),
            rise: opt_f64(item, "RISE"),
            current_rank: opt_f64(item, "CURRENT_RANK"),
            attention_index: opt_f64(item, "ATTENTION_INDEX"),
            trade_date: opt_str(item, "TRADE_DATE"),
        });
    }
    Ok(out)
}

/// 东方财富网-数据中心-特色数据-千股千评 (akshare `stock_comment_em.py:19`).
pub async fn stock_comment_em(client: &Client) -> Result<Vec<CommentRow>> {
    let quote_columns = "f2~01~SECURITY_CODE~CLOSE_PRICE,f8~01~SECURITY_CODE~TURNOVERRATE,f3~01~SECURITY_CODE~CHANGE_RATE,f9~01~SECURITY_CODE~PE_DYNAMIC";
    let v1 = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_comment_em",
            EM_BASE,
            &[
                ("sortColumns", "SECURITY_CODE"),
                ("sortTypes", "1"),
                ("pageSize", "500"),
                ("pageNumber", "1"),
                ("reportName", "RPT_DMSK_TS_STOCKNEW"),
                ("quoteColumns", quote_columns),
                ("columns", "ALL"),
                ("filter", ""),
                ("token", COMMENT_EM_TOKEN),
            ],
        )
        .await?;
    let pages = v1
        .get("result")
        .and_then(|r| r.get("pages"))
        .and_then(|p| p.as_i64())
        .unwrap_or(1)
        .max(1);
    let mut all: Vec<Value> = emg_data_array(&v1)?.to_vec();
    for p in 2..=pages {
        let p_s = p.to_string();
        let v = client
            .get_json(
                SOURCE_EASTMONEY,
                "stock_comment_em",
                EM_BASE,
                &[
                    ("sortColumns", "SECURITY_CODE"),
                    ("sortTypes", "1"),
                    ("pageSize", "500"),
                    ("pageNumber", p_s.as_str()),
                    ("reportName", "RPT_DMSK_TS_STOCKNEW"),
                    ("quoteColumns", quote_columns),
                    ("columns", "ALL"),
                    ("filter", ""),
                    ("token", COMMENT_EM_TOKEN),
                ],
            )
            .await?;
        all.extend(emg_data_array(&v)?.iter().cloned());
    }
    parse_comment_em(&all)
}

// ===========================================================================
// Eastmoney 千股千评 — 市场热度-用户关注指数 (stock_comment_em.py:188)
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct CommentFocusRow {
    pub trade_date: Option<String>,
    pub market_focus: Option<f64>,
}

/// Parse `stock_comment_detail_scrd_focus_em` rows (RPT_STOCK_MARKETFOCUS).
pub(crate) fn parse_scrd_focus(items: &[Value]) -> Result<Vec<CommentFocusRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(CommentFocusRow {
            trade_date: opt_str(item, "TRADE_DATE"),
            market_focus: opt_f64(item, "MARKET_FOCUS"),
        });
    }
    Ok(out)
}

/// 东方财富网-千股千评-市场热度-用户关注指数 (akshare `stock_comment_em.py:188`).
pub async fn stock_comment_detail_scrd_focus_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<CommentFocusRow>> {
    let filter = format!(r#"(SECURITY_CODE="{symbol}")"#);
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_comment_detail_scrd_focus_em",
            EM_BASE,
            &[
                ("filter", filter.as_str()),
                ("columns", "ALL"),
                ("source", "WEB"),
                ("client", "WEB"),
                ("reportName", "RPT_STOCK_MARKETFOCUS"),
                ("sortColumns", "TRADE_DATE"),
                ("sortTypes", "-1"),
                ("pageSize", "30"),
            ],
        )
        .await?;
    parse_scrd_focus(emg_data_array(&v)?)
}

// ===========================================================================
// Eastmoney 千股千评 — 综合评价-历史评分 (stock_comment_em.py:151)
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct CommentLspfRow {
    pub diagnose_date: Option<String>,
    pub total_score: Option<f64>,
}

/// Parse `stock_comment_detail_zhpj_lspf_em` rows (RPT_STOCK_HISTORYMARK).
pub(crate) fn parse_scrd_lspf(items: &[Value]) -> Result<Vec<CommentLspfRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(CommentLspfRow {
            diagnose_date: opt_str(item, "DIAGNOSE_DATE"),
            total_score: opt_f64(item, "TOTAL_SCORE"),
        });
    }
    Ok(out)
}

/// 东方财富网-千股千评-综合评价-历史评分 (akshare `stock_comment_em.py:151`).
pub async fn stock_comment_detail_zhpj_lspf_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<CommentLspfRow>> {
    let filter = format!(r#"(SECURITY_CODE="{symbol}")"#);
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_comment_detail_zhpj_lspf_em",
            EM_BASE,
            &[
                ("filter", filter.as_str()),
                ("columns", "ALL"),
                ("source", "WEB"),
                ("client", "WEB"),
                ("reportName", "RPT_STOCK_HISTORYMARK"),
                ("sortColumns", "DIAGNOSE_DATE"),
                ("sortTypes", "1"),
            ],
        )
        .await?;
    parse_scrd_lspf(emg_data_array(&v)?)
}

// ===========================================================================
// Eastmoney 千股千评 — 市场热度-市场参与意愿 (stock_comment_em.py:226) — JSONP
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct CommentDesireRow {
    pub trade_date: Option<String>,
    pub symbol: Option<String>,
    pub participation_wish: Option<f64>,
    pub participation_wish_5days: Option<f64>,
    pub participation_wish_change: Option<f64>,
    pub participation_wish_5days_change: Option<f64>,
}

/// Parse `stock_comment_detail_scrd_desire_em` rows (RPT_STOCK_PARTICIPATION).
/// Drops `SECURITY_INNER_CODE` (akshare discards it).
pub(crate) fn parse_scrd_desire(items: &[Value]) -> Result<Vec<CommentDesireRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(CommentDesireRow {
            trade_date: opt_str(item, "TRADE_DATE"),
            symbol: opt_str(item, "SECURITY_CODE"),
            participation_wish: opt_f64(item, "PARTICIPATION_WISH"),
            participation_wish_5days: opt_f64(item, "PARTICIPATION_WISH_5DAYS"),
            participation_wish_change: opt_f64(item, "PARTICIPATION_WISH_CHANGE"),
            participation_wish_5days_change: opt_f64(item, "PARTICIPATION_WISH_5DAYSCHANGE"),
        });
    }
    Ok(out)
}

/// 东方财富网-千股千评-市场热度-市场参与意愿 (akshare `stock_comment_em.py:226`).
///
/// Upstream returns a JSONP envelope (`callback(...)`); we strip the wrapper
/// with [`unwrap_jsonp`] then parse the inner datacenter-web payload.
pub async fn stock_comment_detail_scrd_desire_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<CommentDesireRow>> {
    let filter = format!(r#"(SECURITY_CODE="{symbol}")"#);
    let params = [
        ("callback", "jQuery11230899775623921407_0"),
        ("filter", filter.as_str()),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("reportName", "RPT_STOCK_PARTICIPATION"),
        ("sortColumns", "TRADE_DATE"),
        ("sortTypes", "-1"),
        ("pageSize", "30"),
        ("_", "0"),
    ];
    let headers = [
        (
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
        ),
        ("Referer", "https://data.eastmoney.com/"),
        ("Accept", "*/*"),
    ];
    let text = client
        .get_text(
            SOURCE_EASTMONEY,
            "stock_comment_detail_scrd_desire_em",
            EM_BASE,
            &params,
            Some(&headers),
        )
        .await?;
    let v = unwrap_jsonp(&text)?;
    parse_scrd_desire(emg_data_array(&v)?)
}

// ===========================================================================
// Tests (offline parsing only)
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

    /// Extract `result.data.data` from a Sina ESG fixture.
    fn sina_rows(name: &str) -> Vec<Value> {
        sina_esg_data(&fixture(name)).unwrap().clone()
    }

    /// Extract `result.data` from a datacenter-web fixture.
    fn em_rows(name: &str) -> Vec<Value> {
        emg_data_array(&fixture(name)).unwrap().clone()
    }

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    // ---- Sina ESG ----

    #[test]
    fn parse_esg_hz_ok() {
        let rows = parse_esg_hz(&sina_rows("stock_esg_hz_sina.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "600000");
        assert_eq!(rows[0].name.as_deref(), Some("浦发银行"));
        assert!(approx(rows[0].esg_score, 78.5));
        assert_eq!(rows[0].esg_grade.as_deref(), Some("BBB"));
        assert!(approx(rows[1].env_score, 81.0));
    }

    #[test]
    fn parse_esg_msci_ok() {
        let rows = parse_esg_msci(&sina_rows("stock_esg_msci_sina.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "600000");
        assert!(rows[0].esg_rating.is_none()); // esg_rating is a letter grade -> non-numeric
        assert_eq!(rows[0].rating_date.as_deref(), Some("2024-03-31"));
        assert!(approx(rows[1].governance_score, 5.8));
    }

    #[test]
    fn parse_esg_zd_ok() {
        let rows = parse_esg_zd(&sina_rows("stock_esg_zd_sina.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "600000");
        assert!(approx(rows[0].esg_score, 75.0));
        assert!(approx(rows[1].governance_score, 81.0));
        assert_eq!(rows[1].rating_date.as_deref(), Some("2024-03-31"));
    }

    #[test]
    fn parse_esg_rft_ok() {
        let rows = parse_esg_rft(&sina_rows("stock_esg_rft_sina.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "600000");
        assert!(approx(rows[0].esg_score, 62.3));
        assert!(approx(rows[0].dispute_score, 3.2));
        assert_eq!(rows[1].exchange.as_deref(), Some("SZ"));
    }

    #[test]
    fn parse_esg_rate_ok() {
        let v = fixture("stock_esg_rate_sina.json");
        let stocks = v["result"]["data"]["info"]["stocks"].as_array().unwrap();
        let expanded = expand_esg_rate(stocks);
        assert_eq!(expanded.len(), 3);
        let rows = parse_esg_rate(&expanded).unwrap();
        assert_eq!(rows[0].symbol, "600000");
        assert_eq!(rows[0].agency_name.as_deref(), Some("中证指数"));
        assert_eq!(rows[0].rating.as_deref(), Some("BBB"));
        assert_eq!(rows[0].rating_quarter.as_deref(), Some("2024Q1"));
        assert_eq!(rows[2].symbol, "000001");
    }

    // ---- Eastmoney 千股千评 ----

    #[test]
    fn parse_comment_em_ok() {
        let rows = parse_comment_em(&em_rows("stock_comment_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].index, 1);
        assert_eq!(rows[0].symbol, "600000");
        assert_eq!(rows[0].name.as_deref(), Some("浦发银行"));
        assert!(approx(rows[0].close_price, 7.85));
        assert!(approx(rows[0].change_rate, 1.2));
        assert!(approx(rows[0].attention_index, 60.0));
        assert_eq!(rows[1].symbol, "000001");
        assert!(approx(rows[1].current_rank, 90.0));
    }

    #[test]
    fn parse_scrd_focus_ok() {
        let rows = parse_scrd_focus(&em_rows("stock_comment_detail_scrd_focus_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].trade_date.as_deref(), Some("2024-03-31"));
        assert!(approx(rows[0].market_focus, 120.5));
        assert!(approx(rows[1].market_focus, 118.0));
    }

    #[test]
    fn parse_scrd_lspf_ok() {
        let rows = parse_scrd_lspf(&em_rows("stock_comment_detail_zhpj_lspf_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].diagnose_date.as_deref(), Some("2024-03-31"));
        assert!(approx(rows[0].total_score, 80.0));
        assert!(approx(rows[1].total_score, 79.5));
    }

    #[test]
    fn parse_scrd_desire_ok() {
        let rows = parse_scrd_desire(&em_rows("stock_comment_detail_scrd_desire_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol.as_deref(), Some("600000"));
        assert_eq!(rows[0].trade_date.as_deref(), Some("2024-03-31"));
        assert!(approx(rows[0].participation_wish, 55.0));
        assert!(approx(rows[0].participation_wish_5days_change, 1.5));
        assert!(approx(rows[1].participation_wish, 53.0));
    }

    #[test]
    fn unwrap_jsonp_ok() {
        let text = "jQuery1123_123({ \"result\": { \"data\": [ { \"a\": 1 } ] } })";
        let v = unwrap_jsonp(text).unwrap();
        assert_eq!(v["result"]["data"][0]["a"], 1);
        assert!(unwrap_jsonp("not jsonp").is_err());
    }
}
