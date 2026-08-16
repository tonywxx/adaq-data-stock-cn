//! `stock_feature` **杂项** 波次-3 端点 (高管持股 / 融资融券 / 全球财经快讯 / 新股申购).
//!
//! 本模块归属多个 akshare 源文件:
//!
//! | Rust function                | akshare source                      | 源        | 形态 |
//! |------------------------------|-------------------------------------|-----------|------|
//! | `stock_ggcg_em`              | `stock_gdzjc_em.py:15`              | eastmoney | datacenter-web GET (分页) |
//! | `stock_margin_ratio_pa`      | `stock_margin_sse.py:13`            | pingan    | POST JSON |
//! | `stock_margin_detail_sse`    | `stock_margin_sse.py:137`           | sse       | GET + Referer |
//! | `stock_info_cjzc_em`         | `stock_info.py:21`                  | eastmoney | np-listapi GET (2 页) |
//! | `stock_info_global_em`       | `stock_info.py:61`                  | eastmoney | np-weblist GET |
//! | `stock_info_global_sina`     | `stock_info.py:96`                  | sina      | zhibo GET |
//! | `stock_info_global_futu`     | `stock_info.py:127`                 | futu      | news-site-api GET |
//! | `stock_info_global_ths`      | `stock_info.py:162`                 | ths       | tapp GET (无 JS 签名) |
//! | `stock_xgsglb_em`            | `stock_dxsyl_em.py:128`             | eastmoney | datacenter-web GET (分页) |
//!
//! ## DEFERRED
//!
//! * `stock_info_global_cls` (`stock_info.py:195`) — 财联社端点要求请求参数
//!   `sign = md5(sha1(urlencode(params)))`. 本 crate 仅依赖 `sha2` (Sha256),
//!   没有 `sha1` / `md5`, 且任务硬性规定「不得新增依赖」, 故无法在不引入
//!   crypto crate 的前提下忠实实现其带签名请求, 因此 DEFER (规则: 签名 token
//!   需要但缺依赖时 DEFER, 不要伪造).

use chrono::{FixedOffset, TimeZone, Utc};
use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY, SOURCE_SINA};
use crate::core::error::{Error, Result};

// ===========================================================================
// Shared helpers
// ===========================================================================

fn fstr(item: &Value, k: &str) -> String {
    item.get(k)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

/// Format a Unix timestamp (seconds) as `YYYY-MM-DD HH:MM:SS` in
/// `Asia/Shanghai` (UTC+8, no DST). Mirrors akshare's
/// `datetime.fromtimestamp(int(item)).strftime(...)`.
fn ts_shanghai(sec: i64) -> String {
    let utc = Utc
        .timestamp_opt(sec, 0)
        .single()
        .unwrap_or_else(Utc::now);
    let sh = FixedOffset::east_opt(8 * 3600).unwrap();
    utc.with_timezone(&sh)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

fn ts_shanghai_val(v: Option<&Value>) -> Option<String> {
    let sec = match v? {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    }?;
    Some(ts_shanghai(sec))
}

// ===========================================================================
// stock_ggcg_em — 东方财富-数据中心-特色数据-高管持股
// ===========================================================================

const GGCG_BASE: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";
const GGCG_REPORT: &str = "RPT_SHARE_HOLDER_INCREASE";

const GGCG_SYMBOL_MAP: &[(&str, &str)] = &[
    ("全部", ""),
    ("股东增持", "(DIRECTION=\"增持\")"),
    ("股东减持", "(DIRECTION=\"减持\")"),
];

#[derive(Debug, Clone, serde::Serialize)]
pub struct GgcgRow {
    /// `SECURITY_CODE` 代码
    pub code: String,
    /// `SECURITY_NAME_ABBR` 名称
    pub name: String,
    /// `NEWEST_PRICE` 最新价 (来自 quoteColumns)
    pub newest_price: Option<f64>,
    /// `CHANGE_RATE_QUOTES` 涨跌幅 (来自 quoteColumns)
    pub change_rate: Option<f64>,
    /// `HOLDER_NAME` 股东名称
    pub holder_name: String,
    /// `DIRECTION` 持股变动信息-增减
    pub direction: String,
    /// `CHANGE_NUM` 持股变动信息-变动数量
    pub change_num: Option<f64>,
    /// `RATIO_TOTAL_SHARE` 持股变动信息-占总股本比例
    pub ratio_total_share: Option<f64>,
    /// `RATIO_FLOAT_SHARE` 持股变动信息-占流通股比例
    pub ratio_float_share: Option<f64>,
    /// `HOLD_TOTAL` 变动后持股情况-持股总数
    pub hold_total: Option<f64>,
    /// `HOLD_RATIO_TOTAL` 变动后持股情况-占总股本比例
    pub hold_ratio_total: Option<f64>,
    /// `HOLD_FLOAT` 变动后持股情况-持流通股数
    pub hold_float: Option<f64>,
    /// `HOLD_RATIO_FLOAT` 变动后持股情况-占流通股比例
    pub hold_ratio_float: Option<f64>,
    /// `START_DATE` 变动开始日
    pub start_date: Option<String>,
    /// `END_DATE` 变动截止日
    pub end_date: Option<String>,
    /// `NOTICE_DATE` 公告日
    pub notice_date: Option<String>,
}

pub async fn stock_ggcg_em(client: &Client, symbol: &str) -> Result<Vec<GgcgRow>> {
    let filter = ggcg_filter(symbol)?;
    let mut out = Vec::new();
    let mut page: u32 = 1;
    loop {
        let pn = page.to_string();
        let params = [
            ("sortColumns", "END_DATE,SECURITY_CODE,EITIME"),
            ("sortTypes", "-1,-1,-1"),
            ("pageSize", "500"),
            ("pageNumber", pn.as_str()),
            ("reportName", GGCG_REPORT),
            (
                "quoteColumns",
                "f2~01~SECURITY_CODE~NEWEST_PRICE,f3~01~SECURITY_CODE~CHANGE_RATE_QUOTES",
            ),
            ("quoteType", "0"),
            ("columns", "ALL"),
            ("source", "WEB"),
            ("client", "WEB"),
            ("filter", filter.as_str()),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, "stock_ggcg_em", GGCG_BASE, &params)
            .await?;
        let result = v.get("result").ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result at stock_ggcg_em".into(),
        })?;
        let data = result
            .get("data")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default();
        if data.is_empty() {
            break;
        }
        for item in &data {
            out.push(GgcgRow {
                code: fstr(item, "SECURITY_CODE"),
                name: fstr(item, "SECURITY_NAME_ABBR"),
                newest_price: fnum(item, "NEWEST_PRICE"),
                change_rate: fnum(item, "CHANGE_RATE_QUOTES"),
                holder_name: fstr(item, "HOLDER_NAME"),
                direction: fstr(item, "DIRECTION"),
                change_num: fnum(item, "CHANGE_NUM"),
                ratio_total_share: fnum(item, "RATIO_TOTAL_SHARE"),
                ratio_float_share: fnum(item, "RATIO_FLOAT_SHARE"),
                hold_total: fnum(item, "HOLD_TOTAL"),
                hold_ratio_total: fnum(item, "HOLD_RATIO_TOTAL"),
                hold_float: fnum(item, "HOLD_FLOAT"),
                hold_ratio_float: fnum(item, "HOLD_RATIO_FLOAT"),
                start_date: opt_str(item, "START_DATE"),
                end_date: opt_str(item, "END_DATE"),
                notice_date: opt_str(item, "NOTICE_DATE"),
            });
        }
        let pages = result.get("pages").and_then(|p| p.as_u64()).unwrap_or(1);
        if page as u64 >= pages {
            break;
        }
        page += 1;
    }
    Ok(out)
}

fn ggcg_filter(symbol: &str) -> Result<String> {
    for (k, v) in GGCG_SYMBOL_MAP {
        if *k == symbol {
            return Ok((*v).to_string());
        }
    }
    Err(Error::InvalidParam(format!(
        "unknown symbol for stock_ggcg_em: {symbol} (expected 全部/股东增持/股东减持)"
    )))
}

fn opt_str(item: &Value, k: &str) -> Option<String> {
    let s = item.get(k).and_then(|v| v.as_str())?;
    if s.trim().is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

// ===========================================================================
// stock_margin_ratio_pa — 平安证券-融资融券标的证券名单及保证金比例
// ===========================================================================

const PA_URL: &str = "https://stock.pingan.com/fss/servlet/fsscoreapp/stockSource/mrgRatio";

const PA_MARKET_MAP: &[(&str, &str)] = &[
    ("深市", "00"),
    ("沪市", "10"),
    ("北交所", "30"),
];

#[derive(Debug, Clone, serde::Serialize)]
pub struct MarginRatioPaRow {
    /// `secuCode` 证券代码
    pub secu_code: String,
    /// `secuName` 证券简称
    pub secu_name: String,
    /// `fiMarginRatio` 融资比例
    pub fi_margin_ratio: Option<f64>,
    /// `slMarginRatio` 融券比例
    pub sl_margin_ratio: Option<f64>,
}

pub async fn stock_margin_ratio_pa(
    client: &Client,
    symbol: &str,
    date: &str,
) -> Result<Vec<MarginRatioPaRow>> {
    let market = pa_market(symbol)?;
    if date.len() != 8 || !date.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::InvalidParam(format!(
            "stock_margin_ratio_pa date must be 8 ASCII digits YYYYMMDD, got {date:?}"
        )));
    }
    let setdate = format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..]);
    let body = serde_json::json!({
        "currentPage": 1,
        "pageSize": 50000,
        "type": "bdzq",
        "setdate": setdate,
        "stockMes": "",
        "market": market,
        "appName": "AYLCH5",
        "tokenId": "",
        "appChannel": "LRSP",
        "requestId": "194055910e2075c03e25fabf6ffc5a7f",
        "channel": "pa18",
    });
    let v = client
        .post_json("pingan", "stock_margin_ratio_pa", PA_URL, &body, None)
        .await?;
    let list = v
        .get("data")
        .and_then(|d| d.get("list"))
        .and_then(|l| l.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: "pingan",
            message: "missing data.list at stock_margin_ratio_pa".into(),
        })?;
    let mut out = Vec::new();
    for item in list {
        out.push(MarginRatioPaRow {
            secu_code: fstr(item, "secuCode"),
            secu_name: fstr(item, "secuName"),
            fi_margin_ratio: fnum(item, "fiMarginRatio"),
            sl_margin_ratio: fnum(item, "slMarginRatio"),
        });
    }
    Ok(out)
}

fn pa_market(symbol: &str) -> Result<&'static str> {
    for (k, v) in PA_MARKET_MAP {
        if *k == symbol {
            return Ok(*v);
        }
    }
    Err(Error::InvalidParam(format!(
        "unknown symbol for stock_margin_ratio_pa: {symbol} (expected 深市/沪市/北交所)"
    )))
}

// ===========================================================================
// stock_margin_detail_sse — 上交所-融资融券明细
// ===========================================================================

const SSE_URL: &str =
    "https://query.sse.com.cn/marketdata/tradedata/queryMargin.do";

#[derive(Debug, Clone, serde::Serialize)]
pub struct MarginDetailSseRow {
    /// `信用交易日期`
    pub trade_date: String,
    /// `标的证券代码`
    pub secu_code: String,
    /// `标的证券简称`
    pub secu_name: String,
    /// `融资余额`
    pub fin_balance: Option<f64>,
    /// `融资买入额`
    pub fin_buy_amt: Option<f64>,
    /// `融资偿还额`
    pub fin_repay_amt: Option<f64>,
    /// `融券余量`
    pub sl_balance: Option<f64>,
    /// `融券卖出量`
    pub sl_sell_amt: Option<f64>,
    /// `融券偿还量`
    pub sl_repay_amt: Option<f64>,
}

pub async fn stock_margin_detail_sse(client: &Client, date: &str) -> Result<Vec<MarginDetailSseRow>> {
    if date.len() != 8 || !date.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::InvalidParam(format!(
            "stock_margin_detail_sse date must be 8 ASCII digits YYYYMMDD, got {date:?}"
        )));
    }
    let params = [
        ("isPagination", "true"),
        ("tabType", "mxtype"),
        ("detailsDate", date),
        ("stockCode", ""),
        ("beginDate", ""),
        ("endDate", ""),
        ("pageHelp.pageSize", "5000"),
        ("pageHelp.pageCount", "50"),
        ("pageHelp.pageNo", "1"),
        ("pageHelp.beginPage", "1"),
        ("pageHelp.cacheSize", "1"),
        ("pageHelp.endPage", "21"),
    ];
    let headers = [("Referer", "https://www.sse.com.cn/")];
    let v = client
        .get_json_with_headers(
            "sse",
            "stock_margin_detail_sse",
            SSE_URL,
            &params,
            Some(&headers),
        )
        .await?;
    parse_margin_detail_sse(&v)
}

/// Parse `stock_margin_detail_sse` rows. SSE returns `result` as a list of
/// 13-element arrays (akshare renames them positionally); column indices:
/// 1=信用交易日期, 2=融券偿还量, 3=融券卖出量, 4=融券余量, 7=融资偿还额,
/// 8=融资买入额, 10=融资余额, 11=标的证券简称, 12=标的证券代码.
pub(crate) fn parse_margin_detail_sse(resp: &Value) -> Result<Vec<MarginDetailSseRow>> {
    let arr = resp
        .get("result")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: "sse",
            message: "missing result at stock_margin_detail_sse".into(),
        })?;
    let mut out = Vec::new();
    for row in arr {
        out.push(MarginDetailSseRow {
            trade_date: astr(row, 1),
            secu_code: astr(row, 12),
            secu_name: astr(row, 11),
            fin_balance: anum(row, 10),
            fin_buy_amt: anum(row, 8),
            fin_repay_amt: anum(row, 7),
            sl_balance: anum(row, 4),
            sl_sell_amt: anum(row, 3),
            sl_repay_amt: anum(row, 2),
        });
    }
    Ok(out)
}

fn astr(row: &Value, i: usize) -> String {
    row.get(i)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn anum(row: &Value, i: usize) -> Option<f64> {
    row.get(i).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

// ===========================================================================
// stock_info_cjzc_em — 东方财富-财经早餐
// ===========================================================================

const CJZC_URL: &str = "https://np-listapi.eastmoney.com/comm/web/getNewsByColumns";

#[derive(Debug, Clone, serde::Serialize)]
pub struct CjzcRow {
    /// `title` 标题
    pub title: String,
    /// `summary` 摘要
    pub summary: String,
    /// `showTime` 发布时间
    pub show_time: String,
    /// `uniqueUrl` 链接
    pub url: String,
}

pub async fn stock_info_cjzc_em(client: &Client) -> Result<Vec<CjzcRow>> {
    let mut out = Vec::new();
    for page in 1..=2 {
        let pn = page.to_string();
        let params = [
            ("client", "web"),
            ("biz", "web_news_col"),
            ("column", "1207"),
            ("order", "1"),
            ("needInteractData", "0"),
            ("page_index", pn.as_str()),
            ("page_size", "200"),
            ("req_trace", "1710314682980"),
            (
                "fields",
                "code,showTime,title,mediaName,summary,image,url,uniqueUrl,Np_dst",
            ),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, "stock_info_cjzc_em", CJZC_URL, &params)
            .await?;
        out.extend(parse_cjzc(&v)?);
    }
    Ok(out)
}

pub(crate) fn parse_cjzc(resp: &Value) -> Result<Vec<CjzcRow>> {
    let list = resp
        .get("data")
        .and_then(|d| d.get("list"))
        .and_then(|l| l.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.list at stock_info_cjzc_em".into(),
        })?;
    let mut out = Vec::new();
    for item in list {
        out.push(CjzcRow {
            title: fstr(item, "title"),
            summary: fstr(item, "summary"),
            show_time: fstr(item, "showTime"),
            url: fstr(item, "uniqueUrl"),
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_info_global_em — 东方财富-全球财经快讯
// ===========================================================================

const GLOBAL_EM_URL: &str = "https://np-weblist.eastmoney.com/comm/web/getFastNewsList";

#[derive(Debug, Clone, serde::Serialize)]
pub struct GlobalEmRow {
    /// `title` 标题
    pub title: String,
    /// `summary` 摘要
    pub summary: String,
    /// `showTime` 发布时间
    pub show_time: String,
    /// 链接 (`https://finance.eastmoney.com/a/{code}.html`)
    pub url: String,
}

pub async fn stock_info_global_em(client: &Client) -> Result<Vec<GlobalEmRow>> {
    let params = [
        ("client", "web"),
        ("biz", "web_724"),
        ("fastColumn", "102"),
        ("sortEnd", ""),
        ("pageSize", "200"),
        ("req_trace", "1710315450384"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_info_global_em", GLOBAL_EM_URL, &params)
        .await?;
    parse_global_em(&v)
}

pub(crate) fn parse_global_em(resp: &Value) -> Result<Vec<GlobalEmRow>> {
    let list = resp
        .get("data")
        .and_then(|d| d.get("fastNewsList"))
        .and_then(|l| l.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.fastNewsList at stock_info_global_em".into(),
        })?;
    let mut out = Vec::new();
    for item in list {
        let code = fstr(item, "code");
        out.push(GlobalEmRow {
            title: fstr(item, "title"),
            summary: fstr(item, "summary"),
            show_time: fstr(item, "showTime"),
            url: format!("https://finance.eastmoney.com/a/{code}.html"),
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_info_global_sina — 新浪财经-全球财经快讯
// ===========================================================================

const GLOBAL_SINA_URL: &str = "https://zhibo.sina.com.cn/api/zhibo/feed";

#[derive(Debug, Clone, serde::Serialize)]
pub struct GlobalSinaRow {
    /// `create_time` 时间
    pub time: String,
    /// `rich_text` 内容
    pub content: String,
}

pub async fn stock_info_global_sina(client: &Client) -> Result<Vec<GlobalSinaRow>> {
    let params = [
        ("page", "1"),
        ("page_size", "20"),
        ("zhibo_id", "152"),
        ("tag_id", "0"),
        ("dire", "f"),
        ("dpc", "1"),
        ("pagesize", "20"),
        ("type", "1"),
    ];
    let v = client
        .get_json(SOURCE_SINA, "stock_info_global_sina", GLOBAL_SINA_URL, &params)
        .await?;
    parse_global_sina(&v)
}

pub(crate) fn parse_global_sina(resp: &Value) -> Result<Vec<GlobalSinaRow>> {
    let list = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.get("feed"))
        .and_then(|f| f.get("list"))
        .and_then(|l| l.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "missing result.data.feed.list at stock_info_global_sina".into(),
        })?;
    let mut out = Vec::new();
    for item in list {
        out.push(GlobalSinaRow {
            time: fstr(item, "create_time"),
            content: fstr(item, "rich_text"),
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_info_global_futu — 富途牛牛-快讯
// ===========================================================================

const GLOBAL_FUTU_URL: &str = "https://news.futunn.com/news-site-api/main/get-flash-list";

#[derive(Debug, Clone, serde::Serialize)]
pub struct GlobalFutuRow {
    /// `title` 标题
    pub title: String,
    /// `content` 内容
    pub content: String,
    /// `time` 发布时间 (格式化 YYYY-MM-DD HH:MM:SS)
    pub time: String,
    /// `detailUrl` 链接
    pub url: String,
}

pub async fn stock_info_global_futu(client: &Client) -> Result<Vec<GlobalFutuRow>> {
    let params = [("pageSize", "50")];
    let headers = [(
        "User-Agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/111.0.0.0 Safari/537.36",
    )];
    let v = client
        .get_json_with_headers(
            "futu",
            "stock_info_global_futu",
            GLOBAL_FUTU_URL,
            &params,
            Some(&headers),
        )
        .await?;
    parse_global_futu(&v)
}

pub(crate) fn parse_global_futu(resp: &Value) -> Result<Vec<GlobalFutuRow>> {
    let news = resp
        .get("data")
        .and_then(|d| d.get("data"))
        .and_then(|d| d.get("news"))
        .and_then(|l| l.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: "futu",
            message: "missing data.data.news at stock_info_global_futu".into(),
        })?;
    let mut out = Vec::new();
    for item in news {
        out.push(GlobalFutuRow {
            title: fstr(item, "title"),
            content: fstr(item, "content"),
            time: ts_shanghai_val(item.get("time")).unwrap_or_default(),
            url: fstr(item, "detailUrl"),
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_info_global_ths — 同花顺财经-全球财经直播 (无 JS 签名)
// ===========================================================================

const GLOBAL_THS_URL: &str = "https://news.10jqka.com.cn/tapp/news/push/stock";

#[derive(Debug, Clone, serde::Serialize)]
pub struct GlobalThsRow {
    /// `title` 标题
    pub title: String,
    /// `digest` 内容
    pub content: String,
    /// `rtime` 发布时间 (格式化 YYYY-MM-DD HH:MM:SS)
    pub time: String,
    /// `url` 链接
    pub url: String,
}

pub async fn stock_info_global_ths(client: &Client) -> Result<Vec<GlobalThsRow>> {
    let params = [("page", "1"), ("tag", ""), ("track", "website")];
    let headers = [("Referer", "https://news.10jqka.com.cn/")];
    let v = client
        .get_json_with_headers(
            "ths",
            "stock_info_global_ths",
            GLOBAL_THS_URL,
            &params,
            Some(&headers),
        )
        .await?;
    parse_global_ths(&v)
}

pub(crate) fn parse_global_ths(resp: &Value) -> Result<Vec<GlobalThsRow>> {
    let list = resp
        .get("data")
        .and_then(|d| d.get("list"))
        .and_then(|l| l.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: "ths",
            message: "missing data.list at stock_info_global_ths".into(),
        })?;
    let mut out = Vec::new();
    for item in list {
        out.push(GlobalThsRow {
            title: fstr(item, "title"),
            content: fstr(item, "digest"),
            time: ts_shanghai_val(item.get("rtime")).unwrap_or_default(),
            url: fstr(item, "url"),
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_xgsglb_em — 东方财富-新股申购与中签查询
// ===========================================================================

const XGSG_BASE: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";

const XGSG_MARKET_MAP: &[(&str, &str)] = &[
    (
        "全部股票",
        "(APPLY_DATE>'2010-01-01')",
    ),
    (
        "沪市主板",
        "(APPLY_DATE>'2010-01-01')(SECURITY_TYPE_CODE in (\"058001001\",\"058001008\"))(TRADE_MARKET_CODE in (\"069001001001\",\"069001001003\",\"069001001006\"))",
    ),
    (
        "科创板",
        "(APPLY_DATE>'2010-01-01')(SECURITY_TYPE_CODE in (\"058001001\",\"058001008\"))(TRADE_MARKET_CODE=\"069001001006\")",
    ),
    (
        "深市主板",
        "(APPLY_DATE>'2010-01-01')(SECURITY_TYPE_CODE=\"058001001\")(TRADE_MARKET_CODE in (\"069001002001\",\"069001002002\",\"069001002003\",\"069001002005\"))",
    ),
    (
        "创业板",
        "(APPLY_DATE>'2010-01-01')(SECURITY_TYPE_CODE=\"058001001\")(TRADE_MARKET_CODE=\"069001002002\")",
    ),
];

#[derive(Debug, Clone, serde::Serialize)]
pub struct XgsglbRow {
    /// `SECURITY_CODE` 股票代码
    pub code: String,
    /// `SECURITY_NAME` 股票简称
    pub name: String,
    /// `APPLY_CODE` 申购代码
    pub apply_code: String,
    /// `TRADE_MARKET` 交易所
    pub exchange: String,
    /// `MARKET_TYPE` 板块
    pub board: String,
    /// `ISSUE_NUM` 发行总数
    pub issue_num: Option<f64>,
    /// `ONLINE_ISSUE_NUM` 网上发行
    pub online_issue_num: Option<f64>,
    /// `TOP_APPLY_MARKETCAP` 顶格申购需配市值
    pub top_apply_marketcap: Option<f64>,
    /// `ONLINE_APPLY_UPPER` 申购上限
    pub online_apply_upper: Option<f64>,
    /// `ISSUE_PRICE` 发行价格
    pub issue_price: Option<f64>,
    /// `LATELY_PRICE` 最新价
    pub lately_price: Option<f64>,
    /// `CLOSE_PRICE` 首日收盘价
    pub close_price: Option<f64>,
    /// `APPLY_DATE` 申购日期
    pub apply_date: Option<String>,
    /// `BALLOT_NUM_DATE` 中签号公布日
    pub ballot_num_date: Option<String>,
    /// `BALLOT_PAY_DATE` 中签缴款日期
    pub ballot_pay_date: Option<String>,
    /// `LISTING_DATE` 上市日期
    pub listing_date: Option<String>,
    /// `AFTER_ISSUE_PE` 发行市盈率
    pub after_issue_pe: Option<f64>,
    /// `INDUSTRY_PE_NEW` 行业市盈率
    pub industry_pe_new: Option<f64>,
    /// `ONLINE_ISSUE_LWR` 中签率
    pub online_issue_lwr: Option<f64>,
    /// `INITIAL_MULTIPLE` 询价累计报价倍数
    pub initial_multiple: Option<f64>,
    /// `OFFLINE_EP_OBJECT` 配售对象报价家数
    pub offline_ep_object: Option<f64>,
    /// `CONTINUOUS_1WORD_NUM` 连续一字板数量
    pub continuous_1word_num: Option<f64>,
    /// `TOTAL_CHANGE` 涨幅
    pub total_change: Option<f64>,
    /// `PROFIT` 每中一签获利
    pub profit: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct XgsglbNeeqRow {
    /// `SECURITY_CODE` 代码
    pub code: String,
    /// `SECURITY_NAME_ABBR` 简称
    pub name: String,
    /// `APPLY_CODE` 申购代码
    pub apply_code: String,
    /// `EXPECT_ISSUE_NUM` 发行总数
    pub issue_num: Option<f64>,
    /// `ONLINE_ISSUE_NUM` 网上-发行数量
    pub online_issue_num: Option<f64>,
    /// `APPLY_NUM_UPPER` 网上-申购上限
    pub online_apply_upper: Option<f64>,
    /// `APPLY_AMT_UPPER` 网上-顶格所需资金
    pub apply_amt_upper: Option<f64>,
    /// `ISSUE_PRICE` 发行价格
    pub issue_price: Option<f64>,
    /// `APPLY_DATE` 申购日
    pub apply_date: Option<String>,
    /// `ONLINE_ISSUE_LWR` 中签率
    pub online_issue_lwr: Option<f64>,
    /// `APPLY_AMT_100` 稳获百股需配资金
    pub apply_amt_100: Option<f64>,
    /// `NEWEST_PRICE` 最新价格-价格
    pub newest_price: Option<f64>,
    /// `CLOSE_PRICE / NEWEST_PRICE` 最新价格-累计涨幅
    pub newest_cum_change: Option<f64>,
    /// `SELECT_LISTING_DATE` 上市首日-上市日
    pub listing_date: Option<String>,
    /// `AVERAGE_PRICE` 上市首日-均价
    pub average_price: Option<f64>,
    /// `LD_CLOSE_CHANGE` 上市首日-涨幅
    pub ld_close_change: Option<f64>,
    /// `PER_SHARES_INCOME` 上市首日-每百股获利
    pub per_shares_income: Option<f64>,
    /// `CAPTURE_PROFIT` 上市首日-约合年化收益
    pub capture_profit: Option<f64>,
    /// `ISSUE_PE_RATIO` 发行市盈率
    pub issue_pe_ratio: Option<f64>,
    /// `INDUSTRY_PE_RATIO` 行业市盈率
    pub industry_pe_ratio: Option<f64>,
    /// `VA_AMT` 参与申购资金
    pub va_amt: Option<f64>,
    /// `ORG_VAN` 参与申购人数
    pub org_van: Option<f64>,
}

/// `stock_xgsglb_em` row — either the main A-share board or the 北交所 (NEEQ) board.
#[derive(Debug, Clone, serde::Serialize)]
pub enum XgsglbRowEnum {
    Main(XgsglbRow),
    Neeq(XgsglbNeeqRow),
}

pub async fn stock_xgsglb_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<XgsglbRowEnum>> {
    if symbol == "北交所" {
        let mut out = Vec::new();
        let mut page: u32 = 1;
        loop {
            let pn = page.to_string();
            let params = [
                ("sortColumns", "APPLY_DATE"),
                ("sortTypes", "-1"),
                ("pageSize", "500"),
                ("pageNumber", pn.as_str()),
                ("columns", "ALL"),
                ("reportName", "RPT_NEEQ_ISSUEINFO_LIST"),
                (
                    "quoteColumns",
                    "f14~01~SECURITY_CODE~SECURITY_NAME_ABBR",
                ),
                ("source", "NEEQSELECT"),
                ("client", "WEB"),
            ];
            let v = client
                .get_json(SOURCE_EASTMONEY, "stock_xgsglb_em", XGSG_BASE, &params)
                .await?;
            let result = v.get("result").ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing result at stock_xgsglb_em(neeq)".into(),
            })?;
            let data = result
                .get("data")
                .and_then(|d| d.as_array())
                .cloned()
                .unwrap_or_default();
            if data.is_empty() {
                break;
            }
            for item in &data {
                out.push(XgsglbRowEnum::Neeq(parse_xgsg_neeq(item)));
            }
            let pages = result.get("pages").and_then(|p| p.as_u64()).unwrap_or(1);
            if page as u64 >= pages {
                break;
            }
            page += 1;
        }
        Ok(out)
    } else {
        let filter = xgsg_filter(symbol)?;
        let mut out = Vec::new();
        let mut page: u32 = 1;
        loop {
            let pn = page.to_string();
            let params = [
                ("sortColumns", "APPLY_DATE,SECURITY_CODE"),
                ("sortTypes", "-1,-1"),
                ("pageSize", "5000"),
                ("pageNumber", pn.as_str()),
                ("reportName", "RPTA_APP_IPOAPPLY"),
                (
                    "columns",
                    "SECURITY_CODE,SECURITY_NAME,TRADE_MARKET_CODE,APPLY_CODE,TRADE_MARKET,MARKET_TYPE,ORG_TYPE,ISSUE_NUM,ONLINE_ISSUE_NUM,OFFLINE_PLACING_NUM,TOP_APPLY_MARKETCAP,PREDICT_ONFUND_UPPER,ONLINE_APPLY_UPPER,PREDICT_ONAPPLY_UPPER,ISSUE_PRICE,LATELY_PRICE,CLOSE_PRICE,APPLY_DATE,BALLOT_NUM_DATE,BALLOT_PAY_DATE,LISTING_DATE,AFTER_ISSUE_PE,ONLINE_ISSUE_LWR,INITIAL_MULTIPLE,INDUSTRY_PE_NEW,OFFLINE_EP_OBJECT,CONTINUOUS_1WORD_NUM,TOTAL_CHANGE,PROFIT,LIMIT_UP_PRICE,INFO_CODE,OPEN_PRICE,LD_OPEN_PREMIUM,LD_CLOSE_CHANGE,TURNOVERRATE,LD_HIGH_CHANG,LD_AVERAGE_PRICE,OPEN_DATE,OPEN_AVERAGE_PRICE,PREDICT_PE,PREDICT_ISSUE_PRICE2,PREDICT_ISSUE_PRICE,PREDICT_ISSUE_PRICE1,PREDICT_ISSUE_PE,PREDICT_PE_THREE,ONLINE_APPLY_PRICE,MAIN_BUSINESS",
                ),
                ("filter", filter.as_str()),
                ("source", "WEB"),
                ("client", "WEB"),
            ];
            let v = client
                .get_json(SOURCE_EASTMONEY, "stock_xgsglb_em", XGSG_BASE, &params)
                .await?;
            let result = v.get("result").ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing result at stock_xgsglb_em".into(),
            })?;
            let data = result
                .get("data")
                .and_then(|d| d.as_array())
                .cloned()
                .unwrap_or_default();
            if data.is_empty() {
                break;
            }
            for item in &data {
                out.push(XgsglbRowEnum::Main(parse_xgsg_main(item)));
            }
            let pages = result.get("pages").and_then(|p| p.as_u64()).unwrap_or(1);
            if page as u64 >= pages {
                break;
            }
            page += 1;
        }
        Ok(out)
    }
}

fn xgsg_filter(symbol: &str) -> Result<String> {
    for (k, v) in XGSG_MARKET_MAP {
        if *k == symbol {
            return Ok((*v).to_string());
        }
    }
    Err(Error::InvalidParam(format!(
        "unknown symbol for stock_xgsglb_em: {symbol} (expected 全部股票/沪市主板/科创板/深市主板/创业板)"
    )))
}

fn parse_xgsg_main(item: &Value) -> XgsglbRow {
    XgsglbRow {
        code: fstr(item, "SECURITY_CODE"),
        name: fstr(item, "SECURITY_NAME"),
        apply_code: fstr(item, "APPLY_CODE"),
        exchange: fstr(item, "TRADE_MARKET"),
        board: fstr(item, "MARKET_TYPE"),
        issue_num: fnum(item, "ISSUE_NUM"),
        online_issue_num: fnum(item, "ONLINE_ISSUE_NUM"),
        top_apply_marketcap: fnum(item, "TOP_APPLY_MARKETCAP"),
        online_apply_upper: fnum(item, "ONLINE_APPLY_UPPER"),
        issue_price: fnum(item, "ISSUE_PRICE"),
        lately_price: fnum(item, "LATELY_PRICE"),
        close_price: fnum(item, "CLOSE_PRICE"),
        apply_date: opt_str(item, "APPLY_DATE"),
        ballot_num_date: opt_str(item, "BALLOT_NUM_DATE"),
        ballot_pay_date: opt_str(item, "BALLOT_PAY_DATE"),
        listing_date: opt_str(item, "LISTING_DATE"),
        after_issue_pe: fnum(item, "AFTER_ISSUE_PE"),
        industry_pe_new: fnum(item, "INDUSTRY_PE_NEW"),
        online_issue_lwr: fnum(item, "ONLINE_ISSUE_LWR"),
        initial_multiple: fnum(item, "INITIAL_MULTIPLE"),
        offline_ep_object: fnum(item, "OFFLINE_EP_OBJECT"),
        continuous_1word_num: fnum(item, "CONTINUOUS_1WORD_NUM"),
        total_change: fnum(item, "TOTAL_CHANGE"),
        profit: fnum(item, "PROFIT"),
    }
}

fn parse_xgsg_neeq(item: &Value) -> XgsglbNeeqRow {
    let newest = fnum(item, "NEWEST_PRICE");
    let close = fnum(item, "CLOSE_PRICE");
    let cum = match (close, newest) {
        (Some(c), Some(n)) if n != 0.0 => Some(c / n),
        _ => None,
    };
    XgsglbNeeqRow {
        code: fstr(item, "SECURITY_CODE"),
        name: fstr(item, "SECURITY_NAME_ABBR"),
        apply_code: fstr(item, "APPLY_CODE"),
        issue_num: fnum(item, "EXPECT_ISSUE_NUM"),
        online_issue_num: fnum(item, "ONLINE_ISSUE_NUM"),
        online_apply_upper: fnum(item, "APPLY_NUM_UPPER"),
        apply_amt_upper: fnum(item, "APPLY_AMT_UPPER"),
        issue_price: fnum(item, "ISSUE_PRICE"),
        apply_date: opt_str(item, "APPLY_DATE"),
        online_issue_lwr: fnum(item, "ONLINE_ISSUE_LWR"),
        apply_amt_100: fnum(item, "APPLY_AMT_100"),
        newest_price: newest,
        newest_cum_change: cum,
        listing_date: opt_str(item, "SELECT_LISTING_DATE"),
        average_price: fnum(item, "AVERAGE_PRICE"),
        ld_close_change: fnum(item, "LD_CLOSE_CHANGE"),
        per_shares_income: fnum(item, "PER_SHARES_INCOME"),
        capture_profit: fnum(item, "CAPTURE_PROFIT"),
        issue_pe_ratio: fnum(item, "ISSUE_PE_RATIO"),
        industry_pe_ratio: fnum(item, "INDUSTRY_PE_RATIO"),
        va_amt: fnum(item, "VA_AMT"),
        org_van: fnum(item, "ORG_VAN"),
    }
}

// ---------------------------------------------------------------------------
// test-only thin wrappers (avoid live HTTP in unit tests)
// ---------------------------------------------------------------------------
#[cfg(test)]
fn stock_ggcg_em_inner(resp: &Value) -> Result<Vec<GgcgRow>> {
    let result = resp.get("result").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "missing result".into(),
    })?;
    let data = result.get("data").and_then(|d| d.as_array()).cloned().unwrap_or_default();
    let mut out = Vec::new();
    for item in &data {
        out.push(GgcgRow {
            code: fstr(item, "SECURITY_CODE"),
            name: fstr(item, "SECURITY_NAME_ABBR"),
            newest_price: fnum(item, "NEWEST_PRICE"),
            change_rate: fnum(item, "CHANGE_RATE_QUOTES"),
            holder_name: fstr(item, "HOLDER_NAME"),
            direction: fstr(item, "DIRECTION"),
            change_num: fnum(item, "CHANGE_NUM"),
            ratio_total_share: fnum(item, "RATIO_TOTAL_SHARE"),
            ratio_float_share: fnum(item, "RATIO_FLOAT_SHARE"),
            hold_total: fnum(item, "HOLD_TOTAL"),
            hold_ratio_total: fnum(item, "HOLD_RATIO_TOTAL"),
            hold_float: fnum(item, "HOLD_FLOAT"),
            hold_ratio_float: fnum(item, "HOLD_RATIO_FLOAT"),
            start_date: opt_str(item, "START_DATE"),
            end_date: opt_str(item, "END_DATE"),
            notice_date: opt_str(item, "NOTICE_DATE"),
        });
    }
    Ok(out)
}

#[cfg(test)]
fn stock_margin_ratio_pa_inner(resp: &Value) -> Result<Vec<MarginRatioPaRow>> {
    let list = resp
        .get("data")
        .and_then(|d| d.get("list"))
        .and_then(|l| l.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: "pingan",
            message: "missing data.list".into(),
        })?;
    let mut out = Vec::new();
    for item in list {
        out.push(MarginRatioPaRow {
            secu_code: fstr(item, "secuCode"),
            secu_name: fstr(item, "secuName"),
            fi_margin_ratio: fnum(item, "fiMarginRatio"),
            sl_margin_ratio: fnum(item, "slMarginRatio"),
        });
    }
    Ok(out)
}

// ===========================================================================
// tests
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

    #[test]
    fn parses_ggcg() {
        let rows = stock_ggcg_em_inner(&fixture("stock_ggcg_em.json")).unwrap();
        assert!(!rows.is_empty());
        let r = &rows[0];
        assert_eq!(r.code, "600000");
        assert_eq!(r.name, "浦发银行");
        assert!(approx(r.newest_price, 10.5));
        assert_eq!(r.holder_name, "某某投资");
        assert_eq!(r.direction, "增持");
    }

    #[test]
    fn parses_margin_ratio_pa() {
        let rows = stock_margin_ratio_pa_inner(&fixture("stock_margin_ratio_pa.json")).unwrap();
        assert!(!rows.is_empty());
        let r = &rows[0];
        assert_eq!(r.secu_code, "600000");
        assert_eq!(r.secu_name, "浦发银行");
        assert!(approx(r.fi_margin_ratio, 100.0));
        assert!(approx(r.sl_margin_ratio, 50.0));
    }

    #[test]
    fn parses_margin_detail_sse() {
        let rows = parse_margin_detail_sse(&fixture("stock_margin_detail_sse.json")).unwrap();
        assert!(!rows.is_empty());
        let r = &rows[0];
        assert_eq!(r.trade_date, "2023-09-22");
        assert_eq!(r.secu_code, "600000");
        assert_eq!(r.secu_name, "浦发银行");
        assert!(approx(r.fin_balance, 5000.0));
        assert!(approx(r.sl_balance, 67.0));
    }

    #[test]
    fn parses_cjzc() {
        let rows = parse_cjzc(&fixture("stock_info_cjzc_em.json")).unwrap();
        assert!(!rows.is_empty());
        let r = &rows[0];
        assert_eq!(r.title, "财经早餐标题");
        assert!(r.url.contains("uniqueUrl"));
    }

    #[test]
    fn parses_global_em() {
        let rows = parse_global_em(&fixture("stock_info_global_em.json")).unwrap();
        assert!(!rows.is_empty());
        let r = &rows[0];
        assert_eq!(r.url, "https://finance.eastmoney.com/a/abc123.html");
    }

    #[test]
    fn parses_global_sina() {
        let rows = parse_global_sina(&fixture("stock_info_global_sina.json")).unwrap();
        assert!(!rows.is_empty());
        let r = &rows[0];
        assert_eq!(r.time, "2026-01-13 09:30:00");
        assert_eq!(r.content, "快讯内容");
    }

    #[test]
    fn parses_global_futu() {
        let rows = parse_global_futu(&fixture("stock_info_global_futu.json")).unwrap();
        assert!(!rows.is_empty());
        let r = &rows[0];
        assert_eq!(r.title, "富途快讯");
        assert_eq!(r.time, "2026-01-13 09:30:00");
    }

    #[test]
    fn parses_global_ths() {
        let rows = parse_global_ths(&fixture("stock_info_global_ths.json")).unwrap();
        assert!(!rows.is_empty());
        let r = &rows[0];
        assert_eq!(r.title, "同花顺快讯");
        assert_eq!(r.time, "2026-01-13 09:30:00");
    }

    #[test]
    fn parses_xgsglb_main() {
        let v = fixture("stock_xgsglb_em.json");
        let data = v
            .get("result")
            .unwrap()
            .get("data")
            .unwrap()
            .as_array()
            .unwrap();
        let r = parse_xgsg_main(&data[0]);
        assert_eq!(r.code, "603019");
        assert_eq!(r.name, "中科曙光");
        assert!(approx(r.issue_price, 12.34));
        assert!(approx(r.online_issue_lwr, 0.0123));
    }

    #[test]
    fn parses_xgsglb_neeq() {
        let v = fixture("stock_xgsglb_em.json");
        let data = v
            .get("result_neeq")
            .unwrap()
            .get("data")
            .unwrap()
            .as_array()
            .unwrap();
        let r = parse_xgsg_neeq(&data[0]);
        assert_eq!(r.code, "920001");
        assert_eq!(r.name, "北交测试");
        assert!(approx(r.newest_cum_change, 1.5));
    }
}
