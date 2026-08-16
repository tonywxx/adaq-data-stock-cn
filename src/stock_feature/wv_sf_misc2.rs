//! Misc `stock_feature` endpoints (wave-3 follow-up).
//!
//! Each leaf function ports one akshare `stock_feature/*` public function.
//! Sources: Eastmoney datacenter (`datacenter.eastmoney.com` / `datacenter-web.eastmoney.com`
//! / `reportapi.eastmoney.com`), Baidu 股市通 (`finance.pae.baidu.com` /
//! `finance.baidu.com`), and CnInfo (`www.cninfo.com.cn`, plain JSON POST).
//! No JS-signature / token / HTML-scraping is required by any endpoint here,
//! so all are implemented faithfully.
//!
//! | Rust function | akshare source | 源 | 形态 |
//! |---|---|---|---|
//! | `stock_yzxdr_em` | `stock_yzxdr_em.py:16` | eastmoney | datacenter GET (分页) |
//! | `stock_zh_vote_baidu` | `stock_zh_vote_baidu.py:13` | baidu | JSON GET (4 周期) |
//! | `stock_research_report_em` | `stock_research_report_em.py:16` | eastmoney | reportapi GET (分页) |
//! | `stock_hk_valuation_baidu` | `stock_hk_valuation_baidu.py:14` | baidu | opendata GET |
//! | `stock_report_disclosure` | `stock_yjyg_cninfo.py:13` | cninfo | JSON POST |
//! | `stock_tfp_em` | `stock_tfp_em.py:13` | eastmoney | datacenter-web GET (分页) |

use chrono::Datelike;

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

/// Local source identifiers (no shared const exists for these origins).
const SOURCE_BAIDU: &str = "baidu";
const SOURCE_CNINFO: &str = "cninfo";

// ===========================================================================
// Shared helpers
// ===========================================================================

/// Read a string field (null/other -> None).
fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(str::to_string)
}

/// Read a numeric field; accepts both JSON numbers and numeric strings.
fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

/// Look up `key` in a static `(label, code)` map.
fn map_lookup(map: &[(&str, &str)], key: &str, kind: &str) -> Result<String> {
    for (k, v) in map {
        if *k == key {
            return Ok((*v).to_string());
        }
    }
    Err(Error::InvalidParam(format!("unknown {kind}: {key}")))
}

// ===========================================================================
// 一致行动人 — stock_yzxdr_em (stock_yzxdr_em.py:16)
// ===========================================================================

const YZXDR_BASE: &str = "https://datacenter.eastmoney.com/api/data/get";

#[derive(Debug, Clone, serde::Serialize)]
pub struct YzxdrRow {
    pub index: usize,
    pub symbol: Option<String>,
    pub name: Option<String>,
    /// 一致行动人
    pub person: Option<String>,
    /// 股东排名
    pub holder_rank: Option<f64>,
    /// 持股数量
    pub hold_num: Option<f64>,
    /// 持股比例
    pub hold_ratio: Option<f64>,
    /// 持股数量变动
    pub hold_change: Option<f64>,
    pub industry: Option<String>,
    /// 公告日期
    pub notice_date: Option<String>,
}

/// Parse `stock_yzxdr_em` rows from a `result.data` array (1-based `index` like akshare).
pub(crate) fn parse_yzxdr(items: &[Value]) -> Vec<YzxdrRow> {
    let mut out = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        out.push(YzxdrRow {
            index: i + 1,
            symbol: fstr(item, "SECURITY_CODE"),
            name: fstr(item, "SECURITY_NAME_ABBR"),
            person: fstr(item, "PERSON_NAME"),
            holder_rank: fnum(item, "HOLDER_RANK"),
            hold_num: fnum(item, "HOLD_NUM"),
            hold_ratio: fnum(item, "HOLD_RATIO"),
            hold_change: fnum(item, "HOLD_CHANGE_NUM"),
            industry: fstr(item, "INDUSTRY_NAME"),
            notice_date: fstr(item, "NOTICE_DATE"),
        });
    }
    out
}

/// 东方财富网-数据中心-特色数据-一致行动人 (akshare `stock_yzxdr_em.py:16`).
pub async fn stock_yzxdr_em(client: &Client, date: &str) -> Result<Vec<YzxdrRow>> {
    if date.len() != 8 {
        return Err(Error::InvalidParam(format!("date must be YYYYMMDD, got {date}")));
    }
    let date_fmt = format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..]);
    let filter = format!("(enddate='{date_fmt}')");
    let mut all: Vec<Value> = Vec::new();
    let mut page: u32 = 1;
    loop {
        let page_s = page.to_string();
        let params = [
            ("type", "RPTA_WEB_YZXDRINDEX"),
            ("sty", "ALL"),
            ("source", "WEB"),
            ("p", page_s.as_str()),
            ("ps", "500"),
            ("st", "noticedate"),
            ("sr", "-1"),
            ("var", "mwUyirVm"),
            ("filter", filter.as_str()),
            ("rt", "53575609"),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, "stock_yzxdr_em", YZXDR_BASE, &params)
            .await?;
        let result = v.get("result").ok_or_else(|| Error::UpstreamChanged {
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
        all.extend(data.iter().cloned());
        let pages = result.get("pages").and_then(|p| p.as_u64()).unwrap_or(1);
        if page as u64 >= pages {
            break;
        }
        page += 1;
    }
    Ok(parse_yzxdr(&all))
}

// ===========================================================================
// 百度股市通-股评投票 — stock_zh_vote_baidu (stock_zh_vote_baidu.py:13)
// ===========================================================================

const VOTE_BAIDU_BASE: &str = "https://finance.pae.baidu.com/vapi/v1/stockvoterecords";

#[derive(Debug, Clone, serde::Serialize)]
pub struct VoteBaiduRow {
    /// 周期 (day/week/month/year)
    pub period: Option<String>,
    /// 看涨
    pub bullish: Option<f64>,
    /// 看跌
    pub bearish: Option<f64>,
    /// 看涨比例
    pub bullish_ratio: Option<f64>,
    /// 看跌比例
    pub bearish_ratio: Option<f64>,
}

/// Parse `stock_zh_vote_baidu` rows from a `Result.voteRecords.voteRes` array,
/// one row per period (day/week/month/year) matched by `type`.
pub(crate) fn parse_vote_baidu(resp: &Value) -> Result<Vec<VoteBaiduRow>> {
    let vote_res = resp
        .get("Result")
        .and_then(|r| r.get("voteRecords"))
        .and_then(|vr| vr.get("voteRes"))
        .and_then(|a| a.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_BAIDU,
            message: "missing Result.voteRecords.voteRes".into(),
        })?;
    let mut out = Vec::new();
    for period in ["day", "week", "month", "year"] {
        let Some(item) = vote_res
            .iter()
            .find(|it| it.get("type").and_then(|t| t.as_str()) == Some(period))
        else {
            continue;
        };
        out.push(VoteBaiduRow {
            period: fstr(item, "type"),
            bullish: fnum(item, "up"),
            bearish: fnum(item, "down"),
            bullish_ratio: fnum(item, "upRatio"),
            bearish_ratio: fnum(item, "downRatio"),
        });
    }
    Ok(out)
}

/// 百度股市通- A 股或指数-股评-投票 (akshare `stock_zh_vote_baidu.py:13`).
pub async fn stock_zh_vote_baidu(
    client: &Client,
    symbol: &str,
    indicator: &str,
) -> Result<Vec<VoteBaiduRow>> {
    let finance_type = match indicator {
        "股票" => "stock",
        "指数" => "index",
        other => return Err(Error::InvalidParam(format!("unknown indicator: {other}"))),
    };
    let mut out = Vec::new();
    for period in ["day", "week", "month", "year"] {
        let params = [
            ("code", symbol),
            ("market", "ab"),
            ("finance_type", finance_type),
            ("select_type", period),
            ("from_smart_app", "0"),
            ("method", "query"),
            ("finClientType", "pc"),
        ];
        let v = client
            .get_json(SOURCE_BAIDU, "stock_zh_vote_baidu", VOTE_BAIDU_BASE, &params)
            .await?;
        out.extend(parse_vote_baidu(&v)?);
    }
    Ok(out)
}

// ===========================================================================
// 东方财富-个股研报 — stock_research_report_em (stock_research_report_em.py:16)
// ===========================================================================

const REPORT_EM_BASE: &str = "https://reportapi.eastmoney.com/report/list";

#[derive(Debug, Clone, serde::Serialize)]
pub struct ResearchReportRow {
    pub index: usize,
    /// 股票代码
    pub stock_code: Option<String>,
    /// 股票简称
    pub stock_name: Option<String>,
    /// 报告名称
    pub title: Option<String>,
    /// 东财评级
    pub em_rating_name: Option<String>,
    /// 机构
    pub org_name: Option<String>,
    /// 近一月个股研报数
    pub count: Option<f64>,
    /// 当年盈利预测-收益
    pub predict_this_year_eps: Option<f64>,
    /// 当年盈利预测-市盈率
    pub predict_this_year_pe: Option<f64>,
    /// 次年盈利预测-收益
    pub predict_next_year_eps: Option<f64>,
    /// 次年盈利预测-市盈率
    pub predict_next_year_pe: Option<f64>,
    /// 后年盈利预测-收益
    pub predict_next_two_year_eps: Option<f64>,
    /// 后年盈利预测-市盈率
    pub predict_next_two_year_pe: Option<f64>,
    /// 行业
    pub industry: Option<String>,
    /// 日期
    pub publish_date: Option<String>,
    /// 报告 PDF 链接 (derived from infoCode)
    pub pdf_url: Option<String>,
}

/// Parse `stock_research_report_em` rows from a `data` array (1-based `index` like akshare).
pub(crate) fn parse_research_report(items: &[Value]) -> Vec<ResearchReportRow> {
    let mut out = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let pdf_url = fstr(item, "infoCode")
            .map(|c| format!("https://pdf.dfcfw.com/pdf/H3_{c}_1.pdf"));
        out.push(ResearchReportRow {
            index: i + 1,
            stock_code: fstr(item, "stockCode"),
            stock_name: fstr(item, "stockName"),
            title: fstr(item, "title"),
            em_rating_name: fstr(item, "emRatingName"),
            org_name: fstr(item, "orgSName"),
            count: fnum(item, "count"),
            predict_this_year_eps: fnum(item, "predictThisYearEps"),
            predict_this_year_pe: fnum(item, "predictThisYearPe"),
            predict_next_year_eps: fnum(item, "predictNextYearEps"),
            predict_next_year_pe: fnum(item, "predictNextYearPe"),
            predict_next_two_year_eps: fnum(item, "predictNextTwoYearEps"),
            predict_next_two_year_pe: fnum(item, "predictNextTwoYearPe"),
            industry: fstr(item, "indvInduName"),
            publish_date: fstr(item, "publishDate"),
            pdf_url,
        });
    }
    out
}

/// 东方财富网-数据中心-研究报告-个股研报 (akshare `stock_research_report_em.py:16`).
pub async fn stock_research_report_em(client: &Client, symbol: &str) -> Result<Vec<ResearchReportRow>> {
    let end_time = format!("{}-01-01", chrono::Local::now().year() + 1);
    let mut all: Vec<Value> = Vec::new();
    let total_page: i64 = {
        let params = [
            ("industryCode", "*"),
            ("pageSize", "5000"),
            ("industry", "*"),
            ("rating", "*"),
            ("ratingChange", "*"),
            ("beginTime", "2000-01-01"),
            ("endTime", end_time.as_str()),
            ("pageNo", "1"),
            ("fields", ""),
            ("qType", "0"),
            ("orgCode", ""),
            ("code", symbol),
            ("rcode", ""),
            ("p", "1"),
            ("pageNum", "1"),
            ("pageNumber", "1"),
        ];
        let v = client
            .get_json(
                SOURCE_EASTMONEY,
                "stock_research_report_em",
                REPORT_EM_BASE,
                &params,
            )
            .await?;
        let data = v.get("data").and_then(|d| d.as_array()).ok_or_else(|| {
            Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing data".into(),
            }
        })?;
        all.extend(data.iter().cloned());
        v.get("TotalPage").and_then(|t| t.as_i64()).unwrap_or(1).max(1)
    };
    for page in 2..=total_page {
        let page_s = page.to_string();
        let params = [
            ("industryCode", "*"),
            ("pageSize", "5000"),
            ("industry", "*"),
            ("rating", "*"),
            ("ratingChange", "*"),
            ("beginTime", "2000-01-01"),
            ("endTime", end_time.as_str()),
            ("pageNo", page_s.as_str()),
            ("fields", ""),
            ("qType", "0"),
            ("orgCode", ""),
            ("code", symbol),
            ("rcode", ""),
            ("p", page_s.as_str()),
            ("pageNum", page_s.as_str()),
            ("pageNumber", page_s.as_str()),
        ];
        let v = client
            .get_json(
                SOURCE_EASTMONEY,
                "stock_research_report_em",
                REPORT_EM_BASE,
                &params,
            )
            .await?;
        if let Some(data) = v.get("data").and_then(|d| d.as_array()) {
            all.extend(data.iter().cloned());
        }
    }
    Ok(parse_research_report(&all))
}

// ===========================================================================
// 百度股市通-港股估值 — stock_hk_valuation_baidu (stock_hk_valuation_baidu.py:14)
// ===========================================================================

const HK_VAL_BAIDU_BASE: &str = "https://finance.baidu.com/opendata";

#[derive(Debug, Clone, serde::Serialize)]
pub struct HkValuationRow {
    pub date: Option<String>,
    pub value: Option<f64>,
}

/// Parse `stock_hk_valuation_baidu` rows from the nested
/// `Result[0].DisplayData.resultData.tplData.result.chartInfo[0].body` array.
pub(crate) fn parse_hk_valuation(resp: &Value) -> Result<Vec<HkValuationRow>> {
    let result = resp
        .get("Result")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_BAIDU,
            message: "missing Result array".into(),
        })?;
    let first = result.first().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_BAIDU,
        message: "empty Result array".into(),
    })?;
    let chart = first
        .get("DisplayData")
        .and_then(|d| d.get("resultData"))
        .and_then(|rd| rd.get("tplData"))
        .and_then(|tp| tp.get("result"))
        .and_then(|rs| rs.get("chartInfo"))
        .and_then(|ci| ci.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_BAIDU,
            message: "missing ...chartInfo".into(),
        })?;
    let chart0 = chart.first().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_BAIDU,
        message: "empty chartInfo".into(),
    })?;
    let body = chart0
        .get("body")
        .and_then(|b| b.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_BAIDU,
            message: "missing ...body".into(),
        })?;
    let mut out = Vec::with_capacity(body.len());
    for item in body {
        out.push(HkValuationRow {
            date: fstr(item, "date"),
            value: fnum(item, "value"),
        });
    }
    Ok(out)
}

/// 百度股市通-港股-财务报表-估值数据 (akshare `stock_hk_valuation_baidu.py:14`).
///
/// `indicator`/`period` are passed through verbatim (Chinese labels) as akshare does.
pub async fn stock_hk_valuation_baidu(
    client: &Client,
    symbol: &str,
    indicator: &str,
    period: &str,
) -> Result<Vec<HkValuationRow>> {
    let params = [
        ("openapi", "1"),
        ("dspName", "iphone"),
        ("tn", "tangram"),
        ("client", "app"),
        ("query", indicator),
        ("code", symbol),
        ("word", ""),
        ("resource_id", "51171"),
        ("market", "hk"),
        ("tag", indicator),
        ("chart_select", period),
        ("industry_select", ""),
        ("skip_industry", "1"),
        ("finClientType", "pc"),
    ];
    let v = client
        .get_json(
            SOURCE_BAIDU,
            "stock_hk_valuation_baidu",
            HK_VAL_BAIDU_BASE,
            &params,
        )
        .await?;
    parse_hk_valuation(&v)
}

// ===========================================================================
// 巨潮资讯-预约披露 — stock_report_disclosure (stock_yjyg_cninfo.py:13)
// ===========================================================================

const CNINFO_BASE: &str = "http://www.cninfo.com.cn/new/information/getPrbookInfo";

const DISCLOSURE_MARKET_MAP: &[(&str, &str)] = &[
    ("沪深京", "szsh"),
    ("深市", "sz"),
    ("深主板", "szmb"),
    ("创业板", "szcn"),
    ("沪市", "sh"),
    ("沪主板", "shmb"),
    ("科创板", "shkcp"),
    ("北交所", "bj"),
];

#[derive(Debug, Clone, serde::Serialize)]
pub struct ReportDisclosureRow {
    /// 股票代码
    pub stock_code: Option<String>,
    /// 股票简称
    pub stock_name: Option<String>,
    /// 首次预约
    pub first_appointment: Option<String>,
    /// 初次变更
    pub first_change: Option<String>,
    /// 二次变更
    pub second_change: Option<String>,
    /// 三次变更
    pub third_change: Option<String>,
    /// 实际披露
    pub actual_disclosure: Option<String>,
}

/// Parse `stock_report_disclosure` rows from a `prbookinfos` array.
pub(crate) fn parse_disclosure(items: &[Value]) -> Vec<ReportDisclosureRow> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(ReportDisclosureRow {
            stock_code: fstr(item, "stockCode"),
            stock_name: fstr(item, "stockName"),
            first_appointment: fstr(item, "firstDate"),
            first_change: fstr(item, "firstChange"),
            second_change: fstr(item, "secondChange"),
            third_change: fstr(item, "thirdChange"),
            actual_disclosure: fstr(item, "actualDate"),
        });
    }
    out
}

/// 巨潮资讯-首页-数据-预约披露 (akshare `stock_yjyg_cninfo.py:13`).
pub async fn stock_report_disclosure(
    client: &Client,
    market: &str,
    period: &str,
) -> Result<Vec<ReportDisclosureRow>> {
    let market_code = map_lookup(DISCLOSURE_MARKET_MAP, market, "market")?;
    if period.len() < 4 {
        return Err(Error::InvalidParam(format!("period too short: {period}")));
    }
    let year = &period[..4];
    let section_time = section_time_for(year, period)
        .ok_or_else(|| Error::InvalidParam(format!("unknown period: {period}")))?;
    let params = [
        ("sectionTime", section_time.as_str()),
        ("firstTime", ""),
        ("lastTime", ""),
        ("market", market_code.as_str()),
        ("stockCode", ""),
        ("orderClos", ""),
        ("isDesc", ""),
        ("pagesize", "10000"),
        ("pagenum", "1"),
    ];
    let v = client
        .post_form_json(SOURCE_CNINFO, "stock_report_disclosure", CNINFO_BASE, &params, None)
        .await?;
    let items = v
        .get("prbookinfos")
        .and_then(|a| a.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CNINFO,
            message: "missing prbookinfos".into(),
        })?;
    Ok(parse_disclosure(items))
}

/// Map a CnInfo `period` label (e.g. `"2021年报"`) to its `sectionTime` value
/// (e.g. `"2021-12-31"`). Returns `None` for unknown periods.
fn section_time_for(year: &str, period: &str) -> Option<String> {
    if period == format!("{year}一季").as_str() {
        Some(format!("{year}-03-31"))
    } else if period == format!("{year}半年报").as_str() {
        Some(format!("{year}-06-30"))
    } else if period == format!("{year}三季").as_str() {
        Some(format!("{year}-09-30"))
    } else if period == format!("{year}年报").as_str() {
        Some(format!("{year}-12-31"))
    } else {
        None
    }
}

// ===========================================================================
// 东方财富-停复牌信息 — stock_tfp_em (stock_tfp_em.py:13)
// ===========================================================================

const TFP_EM_BASE: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";

#[derive(Debug, Clone, serde::Serialize)]
pub struct TfpRow {
    pub index: usize,
    /// 代码
    pub symbol: Option<String>,
    /// 名称
    pub name: Option<String>,
    /// 停牌时间
    pub suspend_time: Option<String>,
    /// 停牌截止时间
    pub suspend_end_time: Option<String>,
    /// 停牌期限
    pub suspend_term: Option<String>,
    /// 停牌原因
    pub suspend_reason: Option<String>,
    /// 所属市场
    pub market: Option<String>,
    /// 预计复牌时间
    pub expected_resume_time: Option<String>,
}

/// Parse `stock_tfp_em` rows from a `result.data` array (1-based `index` like akshare).
pub(crate) fn parse_tfp(items: &[Value]) -> Vec<TfpRow> {
    let mut out = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        out.push(TfpRow {
            index: i + 1,
            symbol: fstr(item, "SECURITY_CODE"),
            name: fstr(item, "SECURITY_NAME_ABBR"),
            suspend_time: fstr(item, "SUSPEND_TIME"),
            suspend_end_time: fstr(item, "SUSPEND_END_TIME"),
            suspend_term: fstr(item, "SUSPEND_TERM"),
            suspend_reason: fstr(item, "SUSPEND_REASON"),
            market: fstr(item, "MARKET"),
            expected_resume_time: fstr(item, "EXPECTED_RESUME_DATE"),
        });
    }
    out
}

/// 东方财富网-数据中心-特色数据-停复牌信息 (akshare `stock_tfp_em.py:13`).
pub async fn stock_tfp_em(client: &Client, date: &str) -> Result<Vec<TfpRow>> {
    if date.len() != 8 {
        return Err(Error::InvalidParam(format!("date must be YYYYMMDD, got {date}")));
    }
    let date_fmt = format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..]);
    let filter = format!(r#"(MARKET="全部")(DATETIME='{date_fmt}')"#);
    let mut all: Vec<Value> = Vec::new();
    let mut page: u32 = 1;
    loop {
        let page_s = page.to_string();
        let params = [
            ("sortColumns", "SUSPEND_START_DATE"),
            ("sortTypes", "-1"),
            ("pageSize", "500"),
            ("pageNumber", page_s.as_str()),
            ("reportName", "RPT_CUSTOM_SUSPEND_DATA_INTERFACE"),
            ("columns", "ALL"),
            ("source", "WEB"),
            ("client", "WEB"),
            ("filter", filter.as_str()),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, "stock_tfp_em", TFP_EM_BASE, &params)
            .await?;
        let result = v.get("result").ok_or_else(|| Error::UpstreamChanged {
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
        all.extend(data.iter().cloned());
        let pages = result.get("pages").and_then(|p| p.as_u64()).unwrap_or(1);
        if page as u64 >= pages {
            break;
        }
        page += 1;
    }
    Ok(parse_tfp(&all))
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

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    // ---- stock_yzxdr_em ----

    #[test]
    fn parse_yzxdr_ok() {
        let v = fixture("stock_yzxdr_em.json");
        let data = v["result"]["data"].as_array().unwrap();
        let rows = parse_yzxdr(data);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].index, 1);
        assert_eq!(rows[0].symbol.as_deref(), Some("600000"));
        assert_eq!(rows[0].name.as_deref(), Some("浦发银行"));
        assert_eq!(rows[0].person.as_deref(), Some("张三一致行动人"));
        assert!(approx(rows[0].hold_num, 123456.0));
        assert!(approx(rows[0].hold_ratio, 12.5));
        assert!(approx(rows[0].hold_change, -1000.0));
        assert_eq!(rows[1].symbol.as_deref(), Some("000001"));
        assert_eq!(rows[1].notice_date.as_deref(), Some("2024-09-30"));
    }

    // ---- stock_zh_vote_baidu ----

    #[test]
    fn parse_vote_baidu_ok() {
        let v = fixture("stock_zh_vote_baidu.json");
        let rows = parse_vote_baidu(&v).unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].period.as_deref(), Some("day"));
        assert!(approx(rows[0].bullish, 120.0));
        assert!(approx(rows[0].bearish, 80.0));
        assert!(approx(rows[0].bullish_ratio, 60.0));
        assert!(approx(rows[1].bullish, 540.0));
        assert!(approx(rows[3].bearish, 300.0));
    }

    // ---- stock_research_report_em ----

    #[test]
    fn parse_research_report_ok() {
        let v = fixture("stock_research_report_em.json");
        let data = v["data"].as_array().unwrap();
        let rows = parse_research_report(data);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].index, 1);
        assert_eq!(rows[0].stock_code.as_deref(), Some("000001"));
        assert_eq!(rows[0].stock_name.as_deref(), Some("平安银行"));
        assert_eq!(rows[0].em_rating_name.as_deref(), Some("买入"));
        assert_eq!(rows[0].org_name.as_deref(), Some("中信证券"));
        assert!(approx(rows[0].count, 12.0));
        assert!(approx(rows[0].predict_this_year_eps, 2.3));
        assert_eq!(
            rows[0].pdf_url.as_deref(),
            Some("https://pdf.dfcfw.com/pdf/H3_AP20240101_1.pdf")
        );
        assert_eq!(rows[1].stock_code.as_deref(), Some("600000"));
    }

    // ---- stock_hk_valuation_baidu ----

    #[test]
    fn parse_hk_valuation_ok() {
        let v = fixture("stock_hk_valuation_baidu.json");
        let rows = parse_hk_valuation(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date.as_deref(), Some("2023-01-01"));
        assert!(approx(rows[0].value, 1000.5));
        assert!(approx(rows[1].value, 1200.0));
    }

    // ---- stock_report_disclosure ----

    #[test]
    fn parse_disclosure_ok() {
        let v = fixture("stock_report_disclosure.json");
        let items = v["prbookinfos"].as_array().unwrap();
        let rows = parse_disclosure(items);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].stock_code.as_deref(), Some("000001"));
        assert_eq!(rows[0].stock_name.as_deref(), Some("平安银行"));
        assert_eq!(rows[0].first_appointment.as_deref(), Some("2024-03-01"));
        assert_eq!(rows[0].actual_disclosure.as_deref(), Some("2024-03-15"));
        assert_eq!(rows[1].stock_code.as_deref(), Some("600000"));
    }

    // ---- stock_tfp_em ----

    #[test]
    fn parse_tfp_ok() {
        let v = fixture("stock_tfp_em.json");
        let data = v["result"]["data"].as_array().unwrap();
        let rows = parse_tfp(data);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].index, 1);
        assert_eq!(rows[0].symbol.as_deref(), Some("000001"));
        assert_eq!(rows[0].name.as_deref(), Some("平安银行"));
        assert_eq!(rows[0].suspend_reason.as_deref(), Some("重大事项"));
        assert_eq!(rows[0].market.as_deref(), Some("深圳"));
        assert_eq!(rows[1].symbol.as_deref(), Some("600000"));
    }
}
