//! Misc `akshare/stock_feature/*` endpoints ported in wave-3 (leaf module).
//!
//! Pure-HTTP endpoints only — Eastmoney `datacenter-web`/`securities` JSON,
//! cninfo POST JSON, Baidu `opendata` JSON, and BSE JSONP text. No JS-signing,
//! token/session, HTML-scrape or Excel download. Each function returns
//! `Vec<Row>` where `Row` is keyed by the akshare Chinese column names.
//!
//! | Rust fn | akshare source | endpoint |
//! |---|---|---|
//! | `stock_fhps_detail_em` | `stock_fhps_em.py:141` | `datacenter-web` `RPT_SHAREBONUS_DET` |
//! | `stock_irm_cninfo` | `stock_irm_cninfo.py:31` | `irm.cninfo.com.cn/newircs/company/question` |
//! | `stock_irm_ans_cninfo` | `stock_irm_cninfo.py:140` | `irm.cninfo.com.cn/newircs/question/getQuestionDetail` |
//! | `stock_jgdy_tj_em` | `stock_jgdy_em.py:16` | `datacenter-web` `RPT_ORG_SURVEYNEW` |
//! | `stock_yjkb_em` | `stock_yjyg_em.py:17` | `datacenter.eastmoney.com/securities` `RPT_FCI_PERFORMANCEE` |
//! | `stock_yjyg_em` | `stock_yjyg_em.py:135` | `datacenter.eastmoney.com/securities` `RPT_PUBLIC_OP_NEWPREDICT` |
//! | `stock_us_valuation_baidu` | `stock_us_valuation_baidu.py:16` | `gushitong.baidu.com/opendata` |
//! | `stock_zh_a_disclosure_report_cninfo` | `stock_disclosure_cninfo.py:129` | `cninfo.com.cn/new/hisAnnouncement/query` |
//! | `stock_zh_a_disclosure_relation_cninfo` | `stock_disclosure_cninfo.py:205` | `cninfo.com.cn/new/hisAnnouncement/query` |
//! | `stock_pg_em` | `stock_zf_pg.py:99` | `datacenter-web` `RPT_IPO_ALLOTMENT` |
//! | `stock_margin_bse` | `stock_margin_bse.py:71` | `bse.cn/rzrqjyyexxController/summaryInfoResult.do` |
//! | `stock_margin_detail_bse` | `stock_margin_bse.py:129` | `bse.cn/rzrqjyyexxController/detailInfoResult.do` |
//! | `stock_margin_underlying_info_bse` | `stock_margin_bse.py:190` | `bse.cn/rzrqbdzqController/infoResult.do` |
//!
//! ## Transport note
//!
//! The shared `Client` exposes `post_form_json` (sends params as the query
//! string) but no x-www-form-urlencoded body POST, and no POST returning raw
//! text (BSE returns JSONP). cninfo/BSE POST calls therefore pass their fields
//! as query params and BSE uses `get_text`. Parsing logic mirrors akshare
//! exactly; live transport would need a `post_form`/`post_text` client method.

use std::collections::HashMap;

use serde_json::{Map, Value};

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};
use crate::core::json::*;

const SOURCE_CNINFO: &str = "cninfo";
const SOURCE_BSE: &str = "bse";
const SOURCE_BAIDU: &str = "baidu";

const EM_DC_WEB: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";
const EM_DC_SEC: &str = "https://datacenter.eastmoney.com/securities/api/data/v1/get";

/// A single record keyed by akshare column names. Newtype over `serde_json::Map`
/// so it serializes directly as a JSON object whose keys are the akshare
/// Chinese column labels. Derives Debug/Clone/Serialize.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Row(pub Map<String, Value>);

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------


/// Read a field that may be a single string or the first element of an array
/// (cninfo irm returns `trade`/`boardType` as single-element arrays).
fn fstr_first(item: &Value, k: &str) -> Option<String> {
    match item.get(k) {
        Some(Value::Array(a)) => a.first().and_then(|v| v.as_str()).map(str::to_string),
        Some(Value::String(s)) => Some(s.to_string()),
        _ => None,
    }
}

/// Format a millisecond epoch timestamp as `YYYY-MM-DD HH:MM:SS` in
/// `Asia/Shanghai` (akshare uses `unit="ms"`, tz `Asia/Shanghai`).
fn fmt_ms_cninfo(v: &Value) -> Option<String> {
    let ms = match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    }?;
    let dt = chrono::DateTime::from_timestamp_millis(ms)?;
    let sh = chrono::FixedOffset::east_opt(8 * 3600)?;
    Some(dt.with_timezone(&sh).format("%Y-%m-%d %H:%M:%S").to_string())
}

fn jstr(o: Option<String>) -> Value {
    match o {
        Some(s) => Value::String(s),
        None => Value::Null,
    }
}

fn jnum(o: Option<f64>) -> Value {
    match o {
        Some(n) => Value::from(n),
        None => Value::Null,
    }
}

/// `YYYYMMDD` -> `YYYY-MM-DD` (Eastmoney/BSE date filter format).
fn fmt_date8(d: &str) -> Result<String> {
    if d.len() == 8 && d.bytes().all(|b| b.is_ascii_digit()) {
        Ok(format!("{}-{}-{}", &d[0..4], &d[4..6], &d[6..8]))
    } else if d.len() == 10 {
        Ok(d.to_string())
    } else {
        Err(Error::InvalidParam(format!("invalid date: {d}")))
    }
}

enum Col {
    Num(&'static str),
    Str(&'static str),
}

/// Build a `Row` from `item` using the (akshare-column, raw-key + transform) specs.
fn build_row(item: &Value, specs: &[(&'static str, Col)]) -> Row {
    let mut m = Map::new();
    for (cn, col) in specs {
        let val = match col {
            Col::Num(r) => jnum(opt_f64(item, r)),
            Col::Str(r) => jstr(opt_str(item, r)),
        };
        m.insert((*cn).to_string(), val);
    }
    Row(m)
}

/// Fetch every page of an Eastmoney `datacenter` report (paginated via
/// `result.pages` / `result.data`).
async fn em_all(
    client: &Client,
    endpoint: &'static str,
    url: &str,
    mut base: Vec<(&str, String)>,
) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    let mut page: u32 = 1;
    loop {
        for (k, v) in base.iter_mut() {
            if *k == "pageNumber" {
                *v = page.to_string();
            }
        }
        let p: Vec<(&str, &str)> = base.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let resp = client.get_json(SOURCE_EASTMONEY, endpoint, url, &p).await?;
        let result = resp.get("result").ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result".into(),
        })?;
        let data = result
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing result.data".into(),
            })?;
        if data.is_empty() {
            break;
        }
        out.extend(data.iter().cloned());
        let pages = result.get("pages").and_then(|x| x.as_u64()).unwrap_or(1);
        if page as u64 >= pages {
            break;
        }
        page += 1;
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// stock_fhps_detail_em
// ---------------------------------------------------------------------------

/// 东方财富网-数据中心-分红送配-分红送配详情 (`stock_fhps_em.py:141`).
pub async fn stock_fhps_detail_em(client: &Client, symbol: &str) -> Result<Vec<Row>> {
    let base: Vec<(&str, String)> = vec![
        ("sortColumns", "REPORT_DATE".into()),
        ("sortTypes", "-1".into()),
        ("pageSize", "500".into()),
        ("pageNumber", "1".into()),
        ("reportName", "RPT_SHAREBONUS_DET".into()),
        ("columns", "ALL".into()),
        ("quoteColumns", "".into()),
        ("source", "WEB".into()),
        ("client", "WEB".into()),
        ("filter", format!(r#"(SECURITY_CODE="{symbol}")"#)),
    ];
    let items = em_all(client, "stock_fhps_detail_em", EM_DC_WEB, base).await?;
    Ok(parse_fhps_details(&items))
}

fn parse_fhps_details(items: &[Value]) -> Vec<Row> {
    items
        .iter()
        .map(|item| {
            build_row(
                item,
                &[
                    ("送转股份-送转总比例", Col::Num("BONUS_IT_RATIO")),
                    ("送转股份-送股比例", Col::Num("BONUS_RATIO")),
                    ("送转股份-转股比例", Col::Num("IT_RATIO")),
                    ("现金分红-现金分红比例", Col::Num("PRETAX_BONUS_RMB")),
                    ("业绩披露日期", Col::Str("PUBLISH_DATE")),
                    ("股权登记日", Col::Str("EQUITY_RECORD_DATE")),
                    ("除权除息日", Col::Str("EX_DIVIDEND_DATE")),
                    ("报告期", Col::Str("REPORT_DATE")),
                    ("方案进度", Col::Str("ASSIGN_PROGRESS")),
                    ("现金分红-现金分红比例描述", Col::Str("IMPL_PLAN_PROFILE")),
                    ("最新公告日期", Col::Str("NOTICE_DATE")),
                    ("每股收益", Col::Num("BASIC_EPS")),
                    ("每股净资产", Col::Num("BVPS")),
                    ("每股公积金", Col::Num("PER_CAPITAL_RESERVE")),
                    ("每股未分配利润", Col::Num("PER_UNASSIGN_PROFIT")),
                    ("净利润同比增长", Col::Num("PNP_YOY_RATIO")),
                    ("总股本", Col::Num("TOTAL_SHARES")),
                    ("预案公告日", Col::Str("PLAN_NOTICE_DATE")),
                    ("现金分红-股息率", Col::Num("DIVIDENT_RATIO")),
                ],
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// stock_jgdy_tj_em
// ---------------------------------------------------------------------------

/// 东方财富网-数据中心-特色数据-机构调研-机构调研统计 (`stock_jgdy_em.py:16`).
pub async fn stock_jgdy_tj_em(client: &Client, date: &str) -> Result<Vec<Row>> {
    let d = fmt_date8(date)?;
    let base: Vec<(&str, String)> = vec![
        ("sortColumns", "NOTICE_DATE,SUM,RECEIVE_START_DATE,SECURITY_CODE".into()),
        ("sortTypes", "-1,-1,-1,1".into()),
        ("pageSize", "500".into()),
        ("pageNumber", "1".into()),
        ("reportName", "RPT_ORG_SURVEYNEW".into()),
        ("columns", "ALL".into()),
        (
            "quoteColumns",
            "f2~01~SECURITY_CODE~CLOSE_PRICE,f3~01~SECURITY_CODE~CHANGE_RATE".into(),
        ),
        ("source", "WEB".into()),
        ("client", "WEB".into()),
        (
            "filter",
            format!("(NUMBERNEW=\"1\")(IS_SOURCE=\"1\")(NOTICE_DATE>'{d}')"),
        ),
    ];
    let items = em_all(client, "stock_jgdy_tj_em", EM_DC_WEB, base).await?;
    Ok(parse_jgdy_tj(&items))
}

fn parse_jgdy_tj(items: &[Value]) -> Vec<Row> {
    items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let mut r = build_row(
                item,
                &[
                    ("代码", Col::Str("SECURITY_CODE")),
                    ("名称", Col::Str("SECURITY_NAME_ABBR")),
                    ("最新价", Col::Num("CLOSE_PRICE")),
                    ("涨跌幅", Col::Num("CHANGE_RATE")),
                    ("接待机构数量", Col::Num("RESERVE_ORG_NUM")),
                    ("接待方式", Col::Str("RECEIVE_WAY")),
                    ("接待人员", Col::Str("RECEPTIONIST")),
                    ("接待地点", Col::Str("RECEIVE_PLACE")),
                    ("接待日期", Col::Str("RECEIVE_START_DATE")),
                    ("公告日期", Col::Str("NOTICE_DATE")),
                ],
            );
            r.0.insert("序号".into(), Value::from((i + 1) as i64));
            r
        })
        .collect()
}

// ---------------------------------------------------------------------------
// stock_yjkb_em / stock_yjyg_em
// ---------------------------------------------------------------------------

/// 东方财富网-数据中心-年报季报-业绩快报 (`stock_yjyg_em.py:17`).
pub async fn stock_yjkb_em(client: &Client, date: &str) -> Result<Vec<Row>> {
    let d = fmt_date8(date)?;
    let base: Vec<(&str, String)> = vec![
        ("sortColumns", "UPDATE_DATE,SECURITY_CODE".into()),
        ("sortTypes", "-1,-1".into()),
        ("pageSize", "500".into()),
        ("pageNumber", "1".into()),
        ("reportName", "RPT_FCI_PERFORMANCEE".into()),
        ("columns", "ALL".into()),
        (
            "filter",
            format!(
                "(SECURITY_TYPE_CODE in (\"058001001\",\"058001008\"))(TRADE_MARKET_CODE!=\"069001017\")(REPORT_DATE='{d}')"
            ),
        ),
    ];
    let items = em_all(client, "stock_yjkb_em", EM_DC_SEC, base).await?;
    Ok(parse_yjkb(&items))
}

fn parse_yjkb(items: &[Value]) -> Vec<Row> {
    items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let mut r = build_row(
                item,
                &[
                    ("股票代码", Col::Str("SECURITY_CODE")),
                    ("股票简称", Col::Str("SECURITY_NAME_ABBR")),
                    ("每股收益", Col::Num("EPS")),
                    ("营业收入-营业收入", Col::Num("OPERATE_INCOME")),
                    ("营业收入-去年同期", Col::Num("OPERATE_INCOME_PREV")),
                    ("营业收入-同比增长", Col::Num("OPERATE_INCOME_YOY")),
                    ("营业收入-季度环比增长", Col::Num("OPERATE_INCOME_QOQ")),
                    ("净利润-净利润", Col::Num("NET_PROFIT")),
                    ("净利润-去年同期", Col::Num("NET_PROFIT_PREV")),
                    ("净利润-同比增长", Col::Num("NET_PROFIT_YOY")),
                    ("净利润-季度环比增长", Col::Num("NET_PROFIT_QOQ")),
                    ("每股净资产", Col::Num("NAPS")),
                    ("净资产收益率", Col::Num("ROE")),
                    ("所处行业", Col::Str("INDUSTRY")),
                    ("公告日期", Col::Str("UPDATE_DATE")),
                ],
            );
            r.0.insert("序号".into(), Value::from((i + 1) as i64));
            r
        })
        .collect()
}

/// 东方财富网-数据中心-年报季报-业绩预告 (`stock_yjyg_em.py:135`).
pub async fn stock_yjyg_em(client: &Client, date: &str) -> Result<Vec<Row>> {
    let d = fmt_date8(date)?;
    let base: Vec<(&str, String)> = vec![
        ("sortColumns", "NOTICE_DATE,SECURITY_CODE".into()),
        ("sortTypes", "-1,-1".into()),
        ("pageSize", "500".into()),
        ("pageNumber", "1".into()),
        ("reportName", "RPT_PUBLIC_OP_NEWPREDICT".into()),
        ("columns", "ALL".into()),
        ("filter", format!("(REPORT_DATE='{d}')")),
    ];
    let items = em_all(client, "stock_yjyg_em", EM_DC_SEC, base).await?;
    Ok(parse_yjyg(&items))
}

fn parse_yjyg(items: &[Value]) -> Vec<Row> {
    items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let mut r = build_row(
                item,
                &[
                    ("股票代码", Col::Str("SECURITY_CODE")),
                    ("股票简称", Col::Str("SECURITY_NAME_ABBR")),
                    ("预测指标", Col::Str("PREDICT_INDEX")),
                    ("业绩变动", Col::Str("PERFORMANCE_CHANGE")),
                    ("预测数值", Col::Num("PREDICT_VALUE")),
                    ("业绩变动幅度", Col::Num("PERFORMANCE_CHANGE_RANGE")),
                    ("业绩变动原因", Col::Str("PERFORMANCE_CHANGE_REASON")),
                    ("预告类型", Col::Str("PREDICT_TYPE")),
                    ("上年同期值", Col::Num("PREV_YEAR_VALUE")),
                    ("公告日期", Col::Str("NOTICE_DATE")),
                ],
            );
            r.0.insert("序号".into(), Value::from((i + 1) as i64));
            r
        })
        .collect()
}

// ---------------------------------------------------------------------------
// stock_pg_em
// ---------------------------------------------------------------------------

/// 东方财富网-数据中心-新股数据-配股 (`stock_zf_pg.py:99`).
pub async fn stock_pg_em(client: &Client) -> Result<Vec<Row>> {
    let base: Vec<(&str, String)> = vec![
        ("sortColumns", "EQUITY_RECORD_DATE".into()),
        ("sortTypes", "-1".into()),
        ("pageSize", "50000".into()),
        ("pageNumber", "1".into()),
        ("reportName", "RPT_IPO_ALLOTMENT".into()),
        ("columns", "ALL".into()),
        ("quoteColumns", "f2~01~SECURITY_CODE~NEW_PRICE".into()),
        ("quoteType", "0".into()),
        ("source", "WEB".into()),
        ("client", "WEB".into()),
    ];
    let items = em_all(client, "stock_pg_em", EM_DC_WEB, base).await?;
    Ok(parse_pg(&items))
}

fn parse_pg(items: &[Value]) -> Vec<Row> {
    items
        .iter()
        .map(|item| {
            build_row(
                item,
                &[
                    ("股票代码", Col::Str("SECURITY_CODE")),
                    ("股票简称", Col::Str("SECURITY_NAME_ABBR")),
                    ("配售代码", Col::Str("ALLOT_CODE")),
                    ("配股数量", Col::Num("ALLOT_NUM")),
                    ("配股比例", Col::Num("ALLOT_RATIO")),
                    ("配股价", Col::Num("ALLOT_PRICE")),
                    ("最新价", Col::Num("NEW_PRICE")),
                    ("配股前总股本", Col::Num("TOTAL_SHARE_BEFORE")),
                    ("配股后总股本", Col::Num("TOTAL_SHARE_AFTER")),
                    ("股权登记日", Col::Str("RECORD_DATE")),
                    ("缴款起始日期", Col::Str("PAY_START_DATE")),
                    ("缴款截止日期", Col::Str("PAY_END_DATE")),
                    ("上市日", Col::Str("LISTING_DATE")),
                ],
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// stock_us_valuation_baidu
// ---------------------------------------------------------------------------

/// 百度股市通-美股-财务报表-估值数据 (`stock_us_valuation_baidu.py:16`).
pub async fn stock_us_valuation_baidu(
    client: &Client,
    symbol: &str,
    indicator: &str,
    period: &str,
) -> Result<Vec<Row>> {
    let params: &[(&str, &str)] = &[
        ("openapi", "1"),
        ("dspName", "iphone"),
        ("tn", "tangram"),
        ("client", "app"),
        ("query", indicator),
        ("code", symbol),
        ("word", ""),
        ("resource_id", "51171"),
        ("market", "us"),
        ("tag", indicator),
        ("chart_select", period),
        ("industry_select", ""),
        ("skip_industry", "1"),
        ("finClientType", "pc"),
    ];
    let v = client
        .get_json(
            SOURCE_BAIDU,
            "stock_us_valuation_baidu",
            "https://gushitong.baidu.com/opendata",
            params,
        )
        .await?;
    Ok(parse_baidu(&v))
}

fn parse_baidu(v: &Value) -> Vec<Row> {
    let body = v
        .get("Result")
        .and_then(|r| r.get(0))
        .and_then(|d| d.get("DisplayData"))
        .and_then(|d| d.get("resultData"))
        .and_then(|d| d.get("tplData"))
        .and_then(|d| d.get("result"))
        .and_then(|d| d.get("chartInfo"))
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("body"))
        .and_then(|b| b.as_array());
    match body {
        Some(arr) => arr
            .iter()
            .map(|item| {
                let mut m = Map::new();
                m.insert("date".into(), jstr(opt_str(item, "date")));
                m.insert("value".into(), jnum(opt_f64(item, "value")));
                Row(m)
            })
            .collect(),
        None => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// stock_irm_cninfo / stock_irm_ans_cninfo
// ---------------------------------------------------------------------------

/// Resolve cninfo `secid` (org id) for a stock code via `queryKeyboardInfo`.
async fn irm_org_id(client: &Client, symbol: &str) -> Result<String> {
    let url = "https://irm.cninfo.com.cn/newircs/index/queryKeyboardInfo";
    let params: &[(&str, &str)] = &[("_t", "1691144074"), ("keyWord", symbol)];
    let v = client
        .post_form_json(SOURCE_CNINFO, "irm_org_id", url, params, None)
        .await?;
    v.get("data")
        .and_then(|d| d.as_array())
        .and_then(|a| a.first())
        .and_then(|x| x.get("secid"))
        .and_then(|s| s.as_str())
        .map(str::to_string)
        .ok_or_else(|| Error::Parse {
            endpoint: "irm_org_id",
            message: "missing secid".into(),
        })
}

/// 互动易-提问 (`stock_irm_cninfo.py:31`).
pub async fn stock_irm_cninfo(client: &Client, symbol: &str) -> Result<Vec<Row>> {
    let org_id = irm_org_id(client, symbol).await?;
    let url = "https://irm.cninfo.com.cn/newircs/company/question";
    let total = {
        let params: &[(&str, &str)] = &[
            ("_t", "1691142650"),
            ("stockcode", symbol),
            ("orgId", org_id.as_str()),
            ("pageSize", "1000"),
            ("pageNum", "1"),
            ("keyWord", ""),
            ("startDay", ""),
            ("endDay", ""),
        ];
        let v = client
            .post_form_json(SOURCE_CNINFO, "stock_irm_cninfo", url, params, None)
            .await?;
        v.get("totalPage")
            .and_then(|x| x.as_u64())
            .unwrap_or(1)
            .min(10)
    };
    let mut out = Vec::new();
    for page in 1..=total {
        let page_s = page.to_string();
        let params: &[(&str, &str)] = &[
            ("_t", "1691142650"),
            ("stockcode", symbol),
            ("orgId", org_id.as_str()),
            ("pageSize", "1000"),
            ("pageNum", page_s.as_str()),
            ("keyWord", ""),
            ("startDay", ""),
            ("endDay", ""),
        ];
        let v = client
            .post_form_json(SOURCE_CNINFO, "stock_irm_cninfo", url, params, None)
            .await?;
        if let Some(rows) = v.get("rows").and_then(|r| r.as_array()) {
            out.extend(parse_irm_rows(rows));
        }
    }
    Ok(out)
}

fn irm_source(item: &Value) -> String {
    match item.get("pubClient").and_then(|v| v.as_str()) {
        Some("2") => "APP".into(),
        Some("5") => "公众号".into(),
        _ => "网站".into(),
    }
}

fn parse_irm_rows(rows: &[Value]) -> Vec<Row> {
    rows.iter()
        .map(|item| {
            let mut m = Map::new();
            m.insert("股票代码".into(), jstr(opt_str(item, "stockCode")));
            m.insert("公司简称".into(), jstr(opt_str(item, "companyShortName")));
            m.insert("行业".into(), jstr(fstr_first(item, "trade")));
            m.insert("行业代码".into(), jstr(fstr_first(item, "boardType")));
            m.insert("问题".into(), jstr(opt_str(item, "mainContent")));
            m.insert("提问者".into(), jstr(opt_str(item, "authorName")));
            m.insert("来源".into(), Value::String(irm_source(item)));
            m.insert(
                "提问时间".into(),
                jstr(fmt_ms_cninfo(item.get("pubDate").unwrap_or(&Value::Null))),
            );
            m.insert(
                "更新时间".into(),
                jstr(fmt_ms_cninfo(item.get("updateDate").unwrap_or(&Value::Null))),
            );
            m.insert("提问者编号".into(), jstr(opt_str(item, "author")));
            m.insert("问题编号".into(), jstr(opt_str(item, "indexId")));
            m.insert("回答ID".into(), jstr(opt_str(item, "attachedId")));
            m.insert("回答内容".into(), jstr(opt_str(item, "attachedContent")));
            m.insert("回答者".into(), jstr(opt_str(item, "attachedAuthor")));
            Row(m)
        })
        .collect()
}

/// 互动易-回答 (`stock_irm_cninfo.py:140`).
pub async fn stock_irm_ans_cninfo(client: &Client, question_id: &str) -> Result<Vec<Row>> {
    let url = "https://irm.cninfo.com.cn/newircs/question/getQuestionDetail";
    let params: &[(&str, &str)] = &[("questionId", question_id), ("_t", "1691146921")];
    let v = client
        .get_json(SOURCE_CNINFO, "stock_irm_ans_cninfo", url, params)
        .await?;
    Ok(parse_irm_ans(&v))
}

fn parse_irm_ans(v: &Value) -> Vec<Row> {
    let Some(d) = v.get("data").and_then(|d| d.as_object()) else {
        return Vec::new();
    };
    if !d.contains_key("replyDate") {
        return Vec::new();
    }
    let item = Value::Object(d.clone());
    let mut m = Map::new();
    m.insert("股票代码".into(), jstr(opt_str(&item, "stockCode")));
    m.insert("公司简称".into(), jstr(opt_str(&item, "shortName")));
    m.insert("问题".into(), jstr(opt_str(&item, "questionContent")));
    m.insert("回答内容".into(), jstr(opt_str(&item, "replyContent")));
    m.insert("提问者".into(), jstr(opt_str(&item, "questioner")));
    m.insert(
        "提问时间".into(),
        jstr(fmt_ms_cninfo(item.get("questionDate").unwrap_or(&Value::Null))),
    );
    m.insert(
        "回答时间".into(),
        jstr(fmt_ms_cninfo(item.get("replyDate").unwrap_or(&Value::Null))),
    );
    vec![Row(m)]
}

// ---------------------------------------------------------------------------
// stock_zh_a_disclosure_*_cninfo
// ---------------------------------------------------------------------------

fn column_map(market: &str) -> Result<&'static str> {
    Ok(match market {
        "沪深京" => "szse",
        "港股" => "hke",
        "三板" => "third",
        "基金" => "fund",
        "债券" => "bond",
        "监管" => "regulator",
        "预披露" => "pre_disclosure",
        _ => return Err(Error::InvalidParam(format!("unknown market: {market}"))),
    })
}

fn category_dict(category: &str) -> Result<&'static str> {
    Ok(match category {
        "年报" => "category_ndbg_szsh",
        "半年报" => "category_bndbg_szsh",
        "一季报" => "category_yjdbg_szsh",
        "三季报" => "category_sjdbg_szsh",
        "业绩预告" => "category_yjygjxz_szsh",
        "权益分派" => "category_qyfpxzcs_szsh",
        "董事会" => "category_dshgg_szsh",
        "监事会" => "category_jshgg_szsh",
        "股东大会" => "category_gddh_szsh",
        "日常经营" => "category_rcjy_szsh",
        "公司治理" => "category_gszl_szsh",
        "中介报告" => "category_zj_szsh",
        "首发" => "category_sf_szsh",
        "增发" => "category_zf_szsh",
        "股权激励" => "category_gqjl_szsh",
        "配股" => "category_pg_szsh",
        "解禁" => "category_jj_szsh",
        "公司债" => "category_gszq_szsh",
        "可转债" => "category_kzzq_szsh",
        "其他融资" => "category_qtrz_szsh",
        "股权变动" => "category_gqbd_szsh",
        "补充更正" => "category_bcgz_szsh",
        "澄清致歉" => "category_cqdq_szsh",
        "风险提示" => "category_fxts_szsh",
        "特别处理和退市" => "category_tbclts_szsh",
        "退市整理期" => "category_tszlq_szsh",
        "" => "",
        _ => return Err(Error::InvalidParam(format!("unknown category: {category}"))),
    })
}

/// Fetch cninfo `code`->`orgId` map (only meaningful for 沪深京/基金).
async fn cninfo_stock_map(client: &Client, market: &str) -> Result<HashMap<String, String>> {
    if market != "沪深京" && market != "基金" {
        return Ok(HashMap::new());
    }
    let url = "http://www.cninfo.com.cn/new/data/szse_stock.json";
    let v = client
        .get_json(SOURCE_CNINFO, "cninfo_stock_map", url, &[])
        .await?;
    let mut map = HashMap::new();
    if let Some(list) = v.get("stockList").and_then(|x| x.as_array()) {
        for it in list {
            if let (Some(c), Some(o)) = (opt_str(it, "code"), opt_str(it, "orgId")) {
                map.insert(c, o);
            }
        }
    }
    Ok(map)
}

#[allow(clippy::too_many_arguments)]
async fn cninfo_disclosure(
    client: &Client,
    symbol: &str,
    market: &str,
    keyword: &str,
    category: &str,
    start_date: &str,
    end_date: &str,
    tab: &str,
) -> Result<Vec<Row>> {
    let smap = cninfo_stock_map(client, market).await?;
    let column = column_map(market)?;
    let stock_item = if symbol.is_empty() {
        String::new()
    } else {
        let org = smap.get(symbol).ok_or_else(|| {
            Error::InvalidParam(format!("symbol {symbol} not found for market {market}"))
        })?;
        format!("{symbol},{org}")
    };
    let category_item = category_dict(category)?;
    let se_date = format!("{}~{}", fmt_date8(start_date)?, fmt_date8(end_date)?);
    let mut payload: Vec<(&str, String)> = vec![
        ("pageNum", "1".to_string()),
        ("pageSize", "30".to_string()),
        ("column", column.to_string()),
        ("tabName", tab.to_string()),
        ("plate", String::new()),
        ("stock", stock_item),
        ("searchkey", keyword.to_string()),
        ("secid", String::new()),
        ("category", category_item.to_string()),
        ("trade", String::new()),
        ("seDate", se_date),
        ("sortName", String::new()),
        ("sortType", String::new()),
        ("isHLtitle", "true".to_string()),
    ];
    let url = "http://www.cninfo.com.cn/new/hisAnnouncement/query";
    let first = post_payload(client, url, &payload).await?;
    let total = first.get("totalAnnouncement").and_then(|x| x.as_i64()).unwrap_or(0);
    let pages = (total + 29) / 30;
    let mut out = Vec::new();
    if let Some(arr) = first.get("announcements").and_then(|a| a.as_array()) {
        out.extend(parse_announcements(arr));
    }
    for page in 2..=pages {
        for (k, v) in payload.iter_mut() {
            if *k == "pageNum" {
                *v = page.to_string();
            }
        }
        let resp = post_payload(client, url, &payload).await?;
        if let Some(arr) = resp.get("announcements").and_then(|a| a.as_array()) {
            out.extend(parse_announcements(arr));
        }
    }
    Ok(out)
}

async fn post_payload(client: &Client, url: &str, payload: &[(&str, String)]) -> Result<Value> {
    let p: Vec<(&str, &str)> = payload.iter().map(|(k, v)| (*k, v.as_str())).collect();
    client
        .post_form_json(SOURCE_CNINFO, "cninfo_disclosure", url, &p, None)
        .await
}

fn parse_announcements(arr: &[Value]) -> Vec<Row> {
    arr.iter()
        .map(|item| {
            let sec_code = opt_str(item, "secCode");
            let ann_id = opt_str(item, "announcementId");
            let org_id = opt_str(item, "orgId");
            let time = fmt_ms_cninfo(item.get("announcementTime").unwrap_or(&Value::Null));
            let link = match (&sec_code, &ann_id, &org_id, &time) {
                (Some(c), Some(a), Some(o), Some(t)) => Some(format!(
                    "http://www.cninfo.com.cn/new/disclosure/detail?stockCode={c}&announcementId={a}&orgId={o}&announcementTime={t}"
                )),
                _ => None,
            };
            let mut m = Map::new();
            m.insert("代码".into(), jstr(sec_code));
            m.insert("简称".into(), jstr(opt_str(item, "secName")));
            m.insert("公告标题".into(), jstr(opt_str(item, "announcementTitle")));
            m.insert("公告时间".into(), jstr(time));
            m.insert("公告链接".into(), jstr(link));
            Row(m)
        })
        .collect()
}

/// 巨潮资讯-首页-公告查询-信息披露公告 (`stock_disclosure_cninfo.py:129`).
pub async fn stock_zh_a_disclosure_report_cninfo(
    client: &Client,
    symbol: &str,
    market: &str,
    keyword: &str,
    category: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<Row>> {
    cninfo_disclosure(
        client, symbol, market, keyword, category, start_date, end_date, "fulltext",
    )
    .await
}

/// 巨潮资讯-首页-数据-预约披露调研 (`stock_disclosure_cninfo.py:205`).
pub async fn stock_zh_a_disclosure_relation_cninfo(
    client: &Client,
    symbol: &str,
    market: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<Row>> {
    cninfo_disclosure(client, symbol, market, "", "", start_date, end_date, "relation").await
}

// ---------------------------------------------------------------------------
// stock_margin_*_bse
// ---------------------------------------------------------------------------

fn bse_headers(referer: &'static str) -> [(&'static str, &'static str); 2] {
    [
        ("Referer", referer),
        (
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36",
        ),
    ]
}

fn bse_norm_date(date: &str) -> Result<String> {
    if date.is_empty() {
        return Ok(String::new());
    }
    if date.len() == 8 && date.bytes().all(|b| b.is_ascii_digit()) {
        Ok(format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8]))
    } else if date.len() == 10 && date.as_bytes()[4] == b'-' && date.as_bytes()[7] == b'-' {
        Ok(date.to_string())
    } else {
        Err(Error::InvalidParam(format!("invalid BSE date: {date}")))
    }
}

/// Strip the JSONP `cb(...)` wrapper and repair single-quoted trailing dates
/// (akshare's `_parse_bse_jsonp` does the same via regex).
fn parse_bse_jsonp(text: &str) -> Result<Value> {
    let s = text.trim();
    let open = s
        .find('(')
        .ok_or_else(|| Error::Parse {
            endpoint: "bse",
            message: "jsonp missing '('".into(),
        })?;
    let close = s.rfind(')').ok_or_else(|| Error::Parse {
        endpoint: "bse",
        message: "jsonp missing ')'".into(),
    })?;
    let inner = &s[open + 1..close];
    let fixed = fix_bse_dates(inner);
    serde_json::from_str(&fixed).map_err(Error::Json)
}

/// Replace `'YYYY-MM-DD'` with `"YYYY-MM-DD"` (BSE emits a single-quoted date
/// at the end of the JSONP payload).
fn fix_bse_dates(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\''
            && i + 11 <= b.len()
            && is_date_digits(&s[i + 1..i + 11])
            && b[i + 11] == b'\''
        {
            out.push('"');
            out.push_str(&s[i + 1..i + 11]);
            out.push('"');
            i += 12;
        } else {
            out.push(b[i] as char);
            i += 1;
        }
    }
    out
}

fn is_date_digits(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b[0..4].iter().all(|c| c.is_ascii_digit())
        && b[5..7].iter().all(|c| c.is_ascii_digit())
        && b[8..10].iter().all(|c| c.is_ascii_digit())
}

fn bse_zfill_code(r: &mut Row) {
    if let Some(Value::String(c)) = r.0.get_mut("证券代码") {
        *c = format!("{c:0>6}");
    }
}

/// 北京证券交易所-融资融券数据-融资融券汇总 (`stock_margin_bse.py:71`).
pub async fn stock_margin_bse(client: &Client, date: &str) -> Result<Vec<Row>> {
    let nd = bse_norm_date(date)?;
    let url = "https://www.bse.cn/rzrqjyyexxController/summaryInfoResult.do";
    let params: &[(&str, &str)] = &[("callback", "cb"), ("transDate", nd.as_str()), ("page", "0")];
    let headers = bse_headers("https://www.bse.cn/disclosure/rzrq_trans_list.html");
    let text = client
        .get_text(SOURCE_BSE, "stock_margin_bse", url, params, Some(&headers))
        .await?;
    let v = parse_bse_jsonp(&text)?;
    Ok(parse_bse_summary(&v))
}

fn parse_bse_summary(v: &Value) -> Vec<Row> {
    let Some(arr) = v.as_array() else {
        return Vec::new();
    };
    let Some(list) = arr.first().and_then(|x| x.as_array()) else {
        return Vec::new();
    };
    if list.is_empty() {
        return Vec::new();
    }
    list.iter()
        .map(|item| {
            build_row(
                item,
                &[
                    ("融资买入额", Col::Num("rzmreRound")),
                    ("融资余额", Col::Num("rzyeRound")),
                    ("融券卖出量", Col::Num("rqmclRound")),
                    ("融券余量", Col::Num("rqylRound")),
                    ("融券余额", Col::Num("rqyeRound")),
                    ("融资融券余额", Col::Num("rzrqyeRound")),
                ],
            )
        })
        .collect()
}

/// 北京证券交易所-融资融券数据-融资融券交易明细 (`stock_margin_bse.py:129`).
///
/// NOTE: the live endpoint expects a POST with form-encoded `transDate`/`page`.
/// The shared client has no POST-text method, so we issue a GET with query
/// params (parsing logic is identical to akshare's `_parse_bse_jsonp`).
pub async fn stock_margin_detail_bse(client: &Client, date: &str) -> Result<Vec<Row>> {
    let nd = bse_norm_date(date)?;
    let url = "https://www.bse.cn/rzrqjyyexxController/detailInfoResult.do";
    let headers = bse_headers("https://www.bse.cn/disclosure/rzrq_trans_list.html");
    let mut out = Vec::new();
    let mut page: i64 = 0;
    let mut total: i64;
    loop {
        let page_s = page.to_string();
        let params: &[(&str, &str)] =
            &[("callback", "cb"), ("transDate", nd.as_str()), ("page", page_s.as_str())];
        let text = client
            .get_text(
                SOURCE_BSE,
                "stock_margin_detail_bse",
                url,
                params,
                Some(&headers),
            )
            .await?;
        let v = parse_bse_jsonp(&text)?;
        let (content, t) = bse_page(&v)?;
        out.extend(content.iter().map(|item| {
            build_row(
                item,
                &[
                    ("证券代码", Col::Str("zqdm")),
                    ("证券简称", Col::Str("zqjc")),
                    ("融资买入额", Col::Num("rzmre")),
                    ("融资余额", Col::Num("rzye")),
                    ("融券卖出量", Col::Num("rqmcl")),
                    ("融券余量", Col::Num("rqyl")),
                    ("融券余额", Col::Num("rqye")),
                    ("融资融券余额", Col::Num("rzrqye")),
                ],
            )
        }));
        total = t;
        page += 1;
        if page >= total {
            break;
        }
    }
    for r in &mut out {
        bse_zfill_code(r);
    }
    Ok(out)
}

/// 北京证券交易所-融资融券数据-标的证券信息 (`stock_margin_bse.py:190`).
pub async fn stock_margin_underlying_info_bse(client: &Client, date: &str) -> Result<Vec<Row>> {
    let nd = bse_norm_date(date)?;
    let url = "https://www.bse.cn/rzrqbdzqController/infoResult.do";
    let headers = bse_headers("https://www.bse.cn/disclosure/rzrq_bdzq_list.html");
    let mut out = Vec::new();
    let mut page: i64 = 0;
    let mut total: i64;
    loop {
        let page_s = page.to_string();
        let params: &[(&str, &str)] = &[
            ("callback", "cb"),
            ("transDate", nd.as_str()),
            ("zqdm", ""),
            ("page", page_s.as_str()),
        ];
        let text = client
            .get_text(
                SOURCE_BSE,
                "stock_margin_underlying_info_bse",
                url,
                params,
                Some(&headers),
            )
            .await?;
        let v = parse_bse_jsonp(&text)?;
        let (content, t) = bse_page(&v)?;
        out.extend(content.iter().map(|item| {
            build_row(
                item,
                &[
                    ("证券代码", Col::Str("zqdm")),
                    ("证券简称", Col::Str("zqjc")),
                    ("融资标的", Col::Str("rzbd")),
                    ("融券标的", Col::Str("rqbd")),
                    ("当日可融资", Col::Str("drkrz")),
                    ("当日可融券", Col::Str("drkrq")),
                ],
            )
        }));
        total = t;
        page += 1;
        if page >= total {
            break;
        }
    }
    for r in &mut out {
        bse_zfill_code(r);
    }
    Ok(out)
}

/// Extract `(content, totalPages)` from a BSE detail/underlying JSONP payload.
fn bse_page(v: &Value) -> Result<(Vec<Value>, i64)> {
    let arr = v.as_array().ok_or_else(|| Error::Parse {
        endpoint: "bse",
        message: "jsonp not array".into(),
    })?;
    let page_info = arr
        .first()
        .and_then(|x| x.get(0))
        .and_then(|x| x.as_object())
        .ok_or_else(|| Error::Parse {
            endpoint: "bse",
            message: "missing pageInfo".into(),
        })?;
    let total = page_info.get("totalPages").and_then(|x| x.as_i64()).unwrap_or(1);
    let content = page_info
        .get("content")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    Ok((content, total))
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/{name}"))
    }

    #[test]
    fn fhps_detail_em() {
        let v: Value =
            serde_json::from_str(&std::fs::read_to_string(fixture("stock_fhps_detail_em.json")).unwrap()).unwrap();
        let data = v.get("result").unwrap().get("data").unwrap().as_array().unwrap();
        let rows = parse_fhps_details(data);
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].0.get("报告期").unwrap(),
            &Value::String("2023-12-31 00:00:00".into())
        );
        assert_eq!(rows[0].0.get("每股收益"), Some(&Value::from(1.75)));
    }

    #[test]
    fn jgdy_tj_em() {
        let v: Value =
            serde_json::from_str(&std::fs::read_to_string(fixture("stock_jgdy_tj_em.json")).unwrap()).unwrap();
        let data = v.get("result").unwrap().get("data").unwrap().as_array().unwrap();
        let rows = parse_jgdy_tj(data);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0.get("代码").unwrap(), &Value::String("300073".into()));
    }

    #[test]
    fn yjkb_em() {
        let v: Value =
            serde_json::from_str(&std::fs::read_to_string(fixture("stock_yjkb_em.json")).unwrap()).unwrap();
        let data = v.get("result").unwrap().get("data").unwrap().as_array().unwrap();
        let rows = parse_yjkb(data);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0.get("序号").unwrap(), &Value::from(1));
    }

    #[test]
    fn yjyg_em() {
        let v: Value =
            serde_json::from_str(&std::fs::read_to_string(fixture("stock_yjyg_em.json")).unwrap()).unwrap();
        let data = v.get("result").unwrap().get("data").unwrap().as_array().unwrap();
        let rows = parse_yjyg(data);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0.get("股票代码").unwrap(), &Value::String("300073".into()));
    }

    #[test]
    fn pg_em() {
        let v: Value =
            serde_json::from_str(&std::fs::read_to_string(fixture("stock_pg_em.json")).unwrap()).unwrap();
        let data = v.get("result").unwrap().get("data").unwrap().as_array().unwrap();
        let rows = parse_pg(data);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0.get("股票代码").unwrap(), &Value::String("600000".into()));
    }

    #[test]
    fn us_valuation_baidu() {
        let v: Value =
            serde_json::from_str(&std::fs::read_to_string(fixture("stock_us_valuation_baidu.json")).unwrap()).unwrap();
        let rows = parse_baidu(&v);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0.get("date").unwrap(), &Value::String("2023-01-01".into()));
        assert_eq!(rows[0].0.get("value"), Some(&Value::from(100.0)));
        assert_eq!(rows[1].0.get("value"), Some(&Value::from(120.5)));
    }

    #[test]
    fn irm_cninfo() {
        let v: Value =
            serde_json::from_str(&std::fs::read_to_string(fixture("stock_irm_cninfo.json")).unwrap()).unwrap();
        let rows = parse_irm_rows(v.get("rows").unwrap().as_array().unwrap());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0.get("股票代码").unwrap(), &Value::String("002594".into()));
        assert_eq!(rows[0].0.get("来源").unwrap(), &Value::String("网站".into()));
    }

    #[test]
    fn irm_ans_cninfo() {
        let v: Value = serde_json::from_str(
            &std::fs::read_to_string(fixture("stock_irm_ans_cninfo.json")).unwrap(),
        )
        .unwrap();
        let rows = parse_irm_ans(&v);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0.get("股票代码").unwrap(), &Value::String("002594".into()));
    }

    #[test]
    fn disclosure_report_cninfo() {
        let v: Value = serde_json::from_str(
            &std::fs::read_to_string(fixture("stock_zh_a_disclosure_report_cninfo.json")).unwrap(),
        )
        .unwrap();
        let rows = parse_announcements(v.get("announcements").unwrap().as_array().unwrap());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0.get("代码").unwrap(), &Value::String("000001".into()));
        assert!(rows[0].0.get("公告链接").unwrap().as_str().unwrap().contains("announcementId=1"));
    }

    #[test]
    fn disclosure_relation_cninfo() {
        let v: Value = serde_json::from_str(
            &std::fs::read_to_string(fixture("stock_zh_a_disclosure_relation_cninfo.json")).unwrap(),
        )
        .unwrap();
        let rows = parse_announcements(v.get("announcements").unwrap().as_array().unwrap());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0.get("代码").unwrap(), &Value::String("000001".into()));
    }

    #[test]
    fn margin_bse() {
        let text = std::fs::read_to_string(fixture("stock_margin_bse.txt")).unwrap();
        let v = parse_bse_jsonp(&text).unwrap();
        let rows = parse_bse_summary(&v);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0.get("融资余额"), Some(&Value::from(1234.5)));
    }

    #[test]
    fn margin_detail_bse() {
        let text = std::fs::read_to_string(fixture("stock_margin_detail_bse.txt")).unwrap();
        let v = parse_bse_jsonp(&text).unwrap();
        let (content, _total) = bse_page(&v).unwrap();
        let rows: Vec<Row> = content
            .iter()
            .map(|item| {
                build_row(
                    item,
                    &[
                        ("证券代码", Col::Str("zqdm")),
                        ("证券简称", Col::Str("zqjc")),
                        ("融资买入额", Col::Num("rzmre")),
                        ("融资余额", Col::Num("rzye")),
                        ("融券卖出量", Col::Num("rqmcl")),
                        ("融券余量", Col::Num("rqyl")),
                        ("融券余额", Col::Num("rqye")),
                        ("融资融券余额", Col::Num("rzrqye")),
                    ],
                )
            })
            .collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0.get("证券代码").unwrap(), &Value::String("830799".into()));
    }

    #[test]
    fn margin_underlying_info_bse() {
        let text = std::fs::read_to_string(fixture("stock_margin_underlying_info_bse.txt")).unwrap();
        let v = parse_bse_jsonp(&text).unwrap();
        let (content, _total) = bse_page(&v).unwrap();
        let rows: Vec<Row> = content
            .iter()
            .map(|item| {
                build_row(
                    item,
                    &[
                        ("证券代码", Col::Str("zqdm")),
                        ("证券简称", Col::Str("zqjc")),
                        ("融资标的", Col::Str("rzbd")),
                        ("融券标的", Col::Str("rqbd")),
                        ("当日可融资", Col::Str("drkrz")),
                        ("当日可融券", Col::Str("drkrq")),
                    ],
                )
            })
            .collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0.get("证券代码").unwrap(), &Value::String("830799".into()));
    }
}
