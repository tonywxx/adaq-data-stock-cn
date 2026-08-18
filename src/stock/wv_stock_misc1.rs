//! Miscellaneous A-share data endpoints (port of assorted `akshare/stock/*`
//! functions). All implemented functions hit **plain HTTP** JSON/text
//! endpoints — no JS-signature computation, token, cookie, HTML-scraping, or
//! Excel download is required for any of them.
//!
//! | Rust function | akshare source | endpoint | notes |
//! |---|---|---|---|
//! | `stock_hold_management_person_em` | `stock/stock_hold_control_em.py:111` | Eastmoney `datacenter-web` | executive share-holding detail |
//! | `stock_hot_search_baidu` | `stock/stock_hot_search_baidu.py:15` | Baidu finance JSON | hot-search stocks |
//! | `stock_share_hold_change_sse` | `stock/stock_share_hold.py:21` | SSE `commonQuery.do` | paginated, Referer header |
//! | `stock_share_hold_change_szse` | `stock/stock_share_hold.py:118` | SZSE `ShowReport/data` | paginated JSON |
//! | `stock_share_hold_change_bse` | `stock/stock_share_hold.py:196` | BSE `djgCgbdController` | JSONP-wrapped (`null(...)`) |
//! | `stock_news_main_cx` | `stock/stock_news_cx.py:13` | Caixin news API | tag/summary/url |
//! | `stock_zh_a_tick_tx_js` | `stock/stock_zh_a_tick_tx.py:16` | Tencent `stock.gtimg.cn` text | `eval`-style array parse, no JS engine |
//! | `stock_price_js` | `stock/stock_us_js.py:13` | ushknews target-price API | static `x-app-id` header |
//! | `stock_staq_net_stop` | `stock/stock_stop.py:13` | Eastmoney `push2` clist | two-net & delisted board |
//! | `stock_zh_ah_spot` | `stock/stock_zh_ah_tx.py:40` | Tencent `hk_rank.php` | AH realtime, `~`-split |
//! | `stock_zh_ah_name` | `stock/stock_zh_ah_tx.py:110` | Tencent `hk_rank.php` | AH code/name |
//! | `stock_zh_ah_daily` | `stock/stock_zh_ah_tx.py:157` | Tencent `gtimg` kline | yearly history, `~`-split |

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY, SOURCE_TENCENT};
use crate::core::error::{Error, Result};
use crate::core::json::*;

// Source labels for rate-limit buckets / error context (no new constants in client.rs).
const SOURCE_BAIDU: &str = "baidu";
const SOURCE_CAIXIN: &str = "caixin";
const SOURCE_SSE: &str = "sse";
const SOURCE_SZSE: &str = "szse";
const SOURCE_BSE: &str = "bse";
const SOURCE_USHKNEWS: &str = "ushknews";
const SOURCE_TICK: &str = "tencent";

// ---------------------------------------------------------------------------
// shared parse helpers
// ---------------------------------------------------------------------------

/// Current hour (UTC) as a string — used only as a Tencent/Baidu request param.
fn now_hour() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    ((secs / 3600) % 24).to_string()
}

// ===========================================================================
// stock_hold_management_person_em  (stock/stock_hold_control_em.py:111)
// ===========================================================================

const EM_DATACENTER: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";

/// One executive's share-holding change detail row (Eastmoney `RPT_EXECUTIVE_HOLD_DETAILS`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct HoldManagementPersonRow {
    /// 日期 (CHANGE_DATE)
    pub date: Option<String>,
    /// 代码 (SECURITY_CODE)
    pub code: Option<String>,
    /// 名称 (SECURITY_NAME)
    pub name: Option<String>,
    /// 变动人 (PERSON_NAME)
    pub person: Option<String>,
    /// 变动股数 (CHANGE_SHARES)
    pub change_shares: Option<f64>,
    /// 成交均价 (AVERAGE_PRICE)
    pub avg_price: Option<f64>,
    /// 变动金额 (CHANGE_AMOUNT)
    pub change_amount: Option<f64>,
    /// 变动原因 (CHANGE_REASON)
    pub reason: Option<String>,
    /// 变动比例 (CHANGE_RATIO)
    pub change_ratio: Option<f64>,
    /// 变动后持股数 (CHANGE_AFTER_HOLDNUM)
    pub after_hold: Option<f64>,
    /// 持股种类 (HOLD_TYPE)
    pub hold_type: Option<String>,
    /// 董监高人员姓名 (DSE_PERSON_NAME)
    pub dse_person: Option<String>,
    /// 职务 (POSITION_NAME)
    pub position: Option<String>,
    /// 变动人与董监高的关系 (PERSON_DSE_RELATION)
    pub relation: Option<String>,
    /// 开始时持有 (BEGIN_HOLD_NUM)
    pub begin_hold: Option<f64>,
    /// 结束后持有 (END_HOLD_NUM)
    pub end_hold: Option<f64>,
}

/// Parse `stock_hold_management_person_em` rows from the `result.data` array.
pub(crate) fn parse_hold_management_person(data: &[Value]) -> Vec<HoldManagementPersonRow> {
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(HoldManagementPersonRow {
            date: opt_str(item, "CHANGE_DATE"),
            code: opt_str(item, "SECURITY_CODE"),
            name: opt_str(item, "SECURITY_NAME"),
            person: opt_str(item, "PERSON_NAME"),
            change_shares: opt_f64(item, "CHANGE_SHARES"),
            avg_price: opt_f64(item, "AVERAGE_PRICE"),
            change_amount: opt_f64(item, "CHANGE_AMOUNT"),
            reason: opt_str(item, "CHANGE_REASON"),
            change_ratio: opt_f64(item, "CHANGE_RATIO"),
            after_hold: opt_f64(item, "CHANGE_AFTER_HOLDNUM"),
            hold_type: opt_str(item, "HOLD_TYPE"),
            dse_person: opt_str(item, "DSE_PERSON_NAME"),
            position: opt_str(item, "POSITION_NAME"),
            relation: opt_str(item, "PERSON_DSE_RELATION"),
            begin_hold: opt_f64(item, "BEGIN_HOLD_NUM"),
            end_hold: opt_f64(item, "END_HOLD_NUM"),
        });
    }
    out
}

/// 东方财富-高管持股-人员增减持股变动明细. Defaults `symbol="001308"`, `name="吴远"`.
pub async fn stock_hold_management_person_em(
    client: &Client,
    symbol: &str,
    name: &str,
) -> Result<Vec<HoldManagementPersonRow>> {
    let filter = format!(r#"(SECURITY_CODE="{symbol}")(PERSON_NAME="{name}")"#);
    let params = [
        ("reportName", "RPT_EXECUTIVE_HOLD_DETAILS"),
        ("columns", "ALL"),
        ("quoteColumns", ""),
        ("filter", filter.as_str()),
        ("pageNumber", "1"),
        ("pageSize", "5000"),
        ("sortTypes", "-1,1,1"),
        ("sortColumns", "CHANGE_DATE,SECURITY_CODE,PERSON_NAME"),
        ("source", "WEB"),
        ("client", "WEB"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_hold_management_person_em",
            EM_DATACENTER,
            &params,
        )
        .await?;
    let data = v
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data".into(),
        })?;
    Ok(parse_hold_management_person(data))
}

// ===========================================================================
// stock_hot_search_baidu  (stock/stock_hot_search_baidu.py:15)
// ===========================================================================

const BAIDU_URL: &str = "https://finance.pae.baidu.com/selfselect/listsugrecomm";

/// One hot-searched stock row (Baidu 股市通).
#[derive(Debug, Clone, serde::Serialize)]
pub struct HotSearchRow {
    /// 名称/代码 (name)
    pub name: String,
    /// 涨跌幅 (pxChangeRate)
    pub change_rate: Option<f64>,
    /// 综合热度 (heat)
    pub heat: Option<f64>,
}

/// Parse `stock_hot_search_baidu` rows from the `Result.list.body` array.
pub(crate) fn parse_hot_search(body: &[Value]) -> Vec<HotSearchRow> {
    let mut out = Vec::with_capacity(body.len());
    for item in body {
        let Some(name) = opt_str(item, "name") else {
            continue;
        };
        out.push(HotSearchRow {
            name,
            change_rate: opt_f64(item, "pxChangeRate"),
            heat: opt_f64(item, "heat"),
        });
    }
    out
}

/// 百度股市通-热搜股票. `symbol` ∈ {全市场, A股, 港股, 美股}; `time` ∈ {今日, 1小时}.
/// Defaults `symbol="A股"`, `date="20250616"`, `time="今日"`.
pub async fn stock_hot_search_baidu(
    client: &Client,
    symbol: &str,
    date: &str,
    time: &str,
) -> Result<Vec<HotSearchRow>> {
    let market = match symbol {
        "全市场" => "all",
        "A股" => "ab",
        "港股" => "hk",
        "美股" => "us",
        other => return Err(Error::InvalidParam(format!("unknown symbol: {other}"))),
    };
    let hour = now_hour();
    let params = [
        ("bizType", "wisexmlnew"),
        ("dsp", "iphone"),
        ("product", "search"),
        ("style", "tablelist"),
        ("market", market),
        ("type", time),
        ("day", date),
        ("hour", hour.as_str()),
        ("pn", "0"),
        ("rn", "12"),
        ("finClientType", "pc"),
    ];
    let v = client
        .get_json(SOURCE_BAIDU, "stock_hot_search_baidu", BAIDU_URL, &params)
        .await?;
    let body = v
        .get("Result")
        .and_then(|r| r.get("list"))
        .and_then(|l| l.get("body"))
        .and_then(|b| b.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_BAIDU,
            message: "missing Result.list.body".into(),
        })?;
    Ok(parse_hot_search(body))
}

// ===========================================================================
// stock_share_hold_change_sse  (stock/stock_share_hold.py:21)
// ===========================================================================

const SSE_URL: &str = "https://query.sse.com.cn/commonQuery.do";

/// One SSE executive share-holding change row.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SseShareHoldRow {
    /// 公司代码 (COMPANY_CODE)
    pub company_code: Option<String>,
    /// 公司名称 (COMPANY_ABBR)
    pub company_name: Option<String>,
    /// 姓名 (NAME)
    pub name: Option<String>,
    /// 职务 (DUTY)
    pub duty: Option<String>,
    /// 股票种类 (STOCK_TYPE)
    pub stock_type: Option<String>,
    /// 货币种类 (CURRENCY_TYPE)
    pub currency_type: Option<String>,
    /// 本次变动前持股数 (CURRENT_NUM)
    pub pre_hold_num: Option<f64>,
    /// 变动数 (CHANGE_NUM)
    pub change_num: Option<f64>,
    /// 本次变动平均价格 (CURRENT_AVG_PRICE)
    pub avg_price: Option<f64>,
    /// 变动后持股数 (HOLDSTOCK_NUM)
    pub after_hold_num: Option<f64>,
    /// 变动原因 (CHANGE_REASON)
    pub reason: Option<String>,
    /// 变动日期 (CHANGE_DATE)
    pub change_date: Option<String>,
    /// 填报日期 (FORM_DATE)
    pub form_date: Option<String>,
}

/// Parse `stock_share_hold_change_sse` rows from a page's `result` array.
pub(crate) fn parse_sse_share_hold(result: &[Value]) -> Vec<SseShareHoldRow> {
    let mut out = Vec::with_capacity(result.len());
    for item in result {
        out.push(SseShareHoldRow {
            company_code: opt_str(item, "COMPANY_CODE"),
            company_name: opt_str(item, "COMPANY_ABBR"),
            name: opt_str(item, "NAME"),
            duty: opt_str(item, "DUTY"),
            stock_type: opt_str(item, "STOCK_TYPE"),
            currency_type: opt_str(item, "CURRENCY_TYPE"),
            pre_hold_num: opt_f64(item, "CURRENT_NUM"),
            change_num: opt_f64(item, "CHANGE_NUM"),
            avg_price: opt_f64(item, "CURRENT_AVG_PRICE"),
            after_hold_num: opt_f64(item, "HOLDSTOCK_NUM"),
            reason: opt_str(item, "CHANGE_REASON"),
            change_date: opt_str(item, "CHANGE_DATE"),
            form_date: opt_str(item, "FORM_DATE"),
        });
    }
    out
}

/// Build SSE `commonQuery.do` params for a given page.
fn sse_params(page: u64, company_code: &str) -> Vec<(String, String)> {
    vec![
        ("isPagination".into(), "true".into()),
        ("pageHelp.pageSize".into(), "100".into()),
        ("pageHelp.pageNo".into(), page.to_string()),
        ("pageHelp.beginPage".into(), page.to_string()),
        ("pageHelp.cacheSize".into(), "1".into()),
        ("pageHelp.endPage".into(), page.to_string()),
        ("sqlId".into(), "COMMON_SSE_XXPL_CXJL_SSGSGFBDQK_S".into()),
        ("COMPANY_CODE".into(), company_code.into()),
        ("NAME".into(), String::new()),
        ("BEGIN_DATE".into(), "1990-01-01".into()),
        ("END_DATE".into(), "2050-01-01".into()),
        ("BOARDTYPE".into(), String::new()),
    ]
}

const SSE_HEADERS: &[(&str, &str)] = &[
    ("Host", "query.sse.com.cn"),
    ("Referer", "https://www.sse.com.cn/"),
    (
        "User-Agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/93.0.4577.63 Safari/537.36",
    ),
];

/// 上海证券交易所-董监高人员股份变动. `symbol` ∈ {全部, 具体股票代码}; default `全部`.
pub async fn stock_share_hold_change_sse(
    client: &Client,
    symbol: &str,
) -> Result<Vec<SseShareHoldRow>> {
    let company_code = if symbol == "全部" {
        String::new()
    } else {
        symbol.to_string()
    };
    let owned = sse_params(1, &company_code);
    let p: Vec<(&str, &str)> = owned.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let v = client
        .get_json_with_headers(SOURCE_SSE, "stock_share_hold_change_sse", SSE_URL, &p, Some(SSE_HEADERS))
        .await?;
    let page_count = v
        .get("pageHelp")
        .and_then(|p| p.get("pageCount"))
        .and_then(|c| c.as_u64())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SSE,
            message: "missing pageHelp.pageCount".into(),
        })?;
    let page_count = page_count.min(200);
    let mut out = Vec::new();
    let result0 = v
        .get("result")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SSE,
            message: "missing result".into(),
        })?;
    out.extend(parse_sse_share_hold(result0));
    for page in 2..=page_count {
        let owned = sse_params(page, &company_code);
        let p: Vec<(&str, &str)> =
            owned.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let v = client
            .get_json_with_headers(
                SOURCE_SSE,
                "stock_share_hold_change_sse",
                SSE_URL,
                &p,
                Some(SSE_HEADERS),
            )
            .await?;
        let result = v
            .get("result")
            .and_then(|r| r.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_SSE,
                message: "missing result".into(),
            })?;
        out.extend(parse_sse_share_hold(result));
    }
    Ok(out)
}

// ===========================================================================
// stock_share_hold_change_szse  (stock/stock_share_hold.py:118)
// ===========================================================================

const SZSE_URL: &str = "https://www.szse.cn/api/report/ShowReport/data";

/// One SZSE executive share-holding change row.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SzseShareHoldRow {
    /// 证券代码 (zqdm)
    pub code: Option<String>,
    /// 证券简称 (zqjc)
    pub name: Option<String>,
    /// 董监高姓名 (ggxm)
    pub person: Option<String>,
    /// 变动日期 (jyrq)
    pub change_date: Option<String>,
    /// 变动股份数量 (bdgs)
    pub change_num: Option<f64>,
    /// 成交均价 (bdjj)
    pub avg_price: Option<f64>,
    /// 变动原因 (bdyy)
    pub reason: Option<String>,
    /// 变动比例 (cgbdbl)
    pub change_ratio: Option<f64>,
    /// 当日结存股数 (cgzs)
    pub after_hold: Option<f64>,
    /// 股份变动人姓名 (gdxm)
    pub person_name: Option<String>,
    /// 职务 (zw)
    pub duty: Option<String>,
    /// 变动人与董监高的关系 (gxlb)
    pub relation: Option<String>,
}

/// Parse `stock_share_hold_change_szse` rows from a page's `data` array.
pub(crate) fn parse_szse_share_hold(data: &[Value]) -> Vec<SzseShareHoldRow> {
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(SzseShareHoldRow {
            code: opt_str(item, "zqdm"),
            name: opt_str(item, "zqjc"),
            person: opt_str(item, "ggxm"),
            change_date: opt_str(item, "jyrq"),
            change_num: opt_f64(item, "bdgs"),
            avg_price: opt_f64(item, "bdjj"),
            reason: opt_str(item, "bdyy"),
            change_ratio: opt_f64(item, "cgbdbl"),
            after_hold: opt_f64(item, "cgzs"),
            person_name: opt_str(item, "gdxm"),
            duty: opt_str(item, "zw"),
            relation: opt_str(item, "gxlb"),
        });
    }
    out
}

/// Build SZSE `ShowReport/data` params for a given page.
fn szse_params(page: u64, symbol: &str) -> Vec<(String, String)> {
    let mut v = vec![
        ("SHOWTYPE".into(), "JSON".into()),
        ("CATALOGID".into(), "1801_cxda".into()),
        ("TABKEY".into(), "tab1".into()),
        ("PAGENO".into(), page.to_string()),
        ("random".into(), "0.7874198771222201".into()),
    ];
    if symbol != "全部" {
        v.push(("txtDMorJC".into(), symbol.into()));
    }
    v
}

const SZSE_HEADERS: &[(&str, &str)] = &[(
    "User-Agent",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/93.0.4577.63 Safari/537.36",
)];

/// 深圳证券交易所-董监高人员股份变动. `symbol` ∈ {全部, 具体股票代码}; default `全部`.
pub async fn stock_share_hold_change_szse(
    client: &Client,
    symbol: &str,
) -> Result<Vec<SzseShareHoldRow>> {
    let owned = szse_params(1, symbol);
    let p: Vec<(&str, &str)> = owned.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let v = client
        .get_json_with_headers(
            SOURCE_SZSE,
            "stock_share_hold_change_szse",
            SZSE_URL,
            &p,
            Some(SZSE_HEADERS),
        )
        .await?;
    let arr = v.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SZSE,
        message: "response was not a JSON array".into(),
    })?;
    let first = arr.first().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SZSE,
        message: "empty response array".into(),
    })?;
    let page_count = first
        .get("metadata")
        .and_then(|m| m.get("pagecount"))
        .and_then(|c| c.as_u64())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SZSE,
            message: "missing metadata.pagecount".into(),
        })?;
    let page_count = page_count.min(200);
    let mut out = Vec::new();
    let data0 = first
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SZSE,
            message: "missing data".into(),
        })?;
    out.extend(parse_szse_share_hold(data0));
    for page in 2..=page_count {
        let owned = szse_params(page, symbol);
        let p: Vec<(&str, &str)> =
            owned.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let v = client
            .get_json_with_headers(
                SOURCE_SZSE,
                "stock_share_hold_change_szse",
                SZSE_URL,
                &p,
                Some(SZSE_HEADERS),
            )
            .await?;
        let arr = v.as_array().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SZSE,
            message: "response was not a JSON array".into(),
        })?;
        let first = arr.first().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SZSE,
            message: "empty response array".into(),
        })?;
        let data = first
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_SZSE,
                message: "missing data".into(),
            })?;
        out.extend(parse_szse_share_hold(data));
    }
    Ok(out)
}

// ===========================================================================
// stock_share_hold_change_bse  (stock/stock_share_hold.py:196)
// ===========================================================================

const BSE_URL: &str = "https://www.bse.cn/djgCgbdController/getDjgCgbdList.do";

/// One BSE executive share-holding change row.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BseShareHoldRow {
    /// 代码 (stockCode)
    pub code: Option<String>,
    /// 简称 (stockName)
    pub name: Option<String>,
    /// 姓名 (djgName)
    pub person: Option<String>,
    /// 职务 (duty)
    pub duty: Option<String>,
    /// 变动日期 (changeDate)
    pub change_date: Option<String>,
    /// 变动股数 (changeAmount)
    pub change_amount: Option<f64>,
    /// 变动前持股数 (preAmount)
    pub pre_amount: Option<f64>,
    /// 变动后持股数 (newAmount)
    pub new_amount: Option<f64>,
    /// 变动均价 (price)
    pub price: Option<f64>,
    /// 变动原因 (reason)
    pub reason: Option<String>,
}

/// Parse `stock_share_hold_change_bse` rows from a page's `result.content` array.
pub(crate) fn parse_bse_share_hold(content: &[Value]) -> Vec<BseShareHoldRow> {
    let mut out = Vec::with_capacity(content.len());
    for item in content {
        out.push(BseShareHoldRow {
            code: opt_str(item, "stockCode"),
            name: opt_str(item, "stockName"),
            person: opt_str(item, "djgName"),
            duty: opt_str(item, "duty"),
            change_date: opt_str(item, "changeDate"),
            change_amount: opt_f64(item, "changeAmount"),
            pre_amount: opt_f64(item, "preAmount"),
            new_amount: opt_f64(item, "newAmount"),
            price: opt_f64(item, "price"),
            reason: opt_str(item, "reason"),
        });
    }
    out
}

/// Build BSE `djgCgbdController` params for a given page.
fn bse_params(page: u64, stock_code: &str) -> Vec<(String, String)> {
    vec![
        ("page".into(), page.to_string()),
        ("startTime".into(), String::new()),
        ("endTime".into(), String::new()),
        ("stockCode".into(), stock_code.into()),
        ("djgName".into(), String::new()),
        ("ssgs".into(), "1".into()),
        (
            "sortfield".into(),
            "bean.change_date desc, bean.stock_code asc, bean.change_amount desc, bean.price".into(),
        ),
        ("sorttype".into(), "desc".into()),
    ]
}

const BSE_HEADERS: &[(&str, &str)] = &[(
    "User-Agent",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/93.0.4577.63 Safari/537.36",
)];

/// Strip the `null(...)` JSONP wrapper and parse the inner JSON.
fn strip_bse_jsonp(text: &str) -> Result<Value> {
    let t = text.trim();
    let t = t.strip_prefix("null(").unwrap_or(t);
    let t = t.strip_suffix(')').unwrap_or(t).trim();
    serde_json::from_str(t).map_err(Error::Json)
}

/// 北京证券交易所-董监高及相关人员持股变动. `symbol` ∈ {全部, 具体股票代码};
/// default `全部` (empty `stockCode`).
pub async fn stock_share_hold_change_bse(
    client: &Client,
    symbol: &str,
) -> Result<Vec<BseShareHoldRow>> {
    let stock_code = if symbol == "全部" {
        String::new()
    } else {
        symbol.to_string()
    };
    let mut out = Vec::new();
    let mut page: u64 = 0;
    loop {
        let owned = bse_params(page, &stock_code);
        let p: Vec<(&str, &str)> =
            owned.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let text = client
            .get_text(
                SOURCE_BSE,
                "stock_share_hold_change_bse",
                BSE_URL,
                &p,
                Some(BSE_HEADERS),
            )
            .await?;
        let v = strip_bse_jsonp(&text)?;
        let arr = v.as_array().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_BSE,
            message: "response was not a JSON array".into(),
        })?;
        let first = arr.first().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_BSE,
            message: "empty response array".into(),
        })?;
        let total_pages = first
            .get("result")
            .and_then(|r| r.get("totalPages"))
            .and_then(|t| t.as_u64())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_BSE,
                message: "missing result.totalPages".into(),
            })?;
        let content = first
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_BSE,
                message: "missing result.content".into(),
            })?;
        out.extend(parse_bse_share_hold(content));
        if page + 1 >= total_pages || page >= 200 {
            break;
        }
        page += 1;
    }
    Ok(out)
}

// ===========================================================================
// stock_news_main_cx  (stock/stock_news_cx.py:13)
// ===========================================================================

const CAIXIN_URL: &str = "https://cxdata.caixin.com/api/dataplus/sjtPc/news";

/// One Caixin news row.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NewsMainCxRow {
    /// 标签 (tag)
    pub tag: Option<String>,
    /// 摘要 (summary)
    pub summary: Option<String>,
    /// 链接 (url)
    pub url: Option<String>,
}

/// Parse `stock_news_main_cx` rows from the `data.data` array.
pub(crate) fn parse_news_main_cx(data: &[Value]) -> Vec<NewsMainCxRow> {
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(NewsMainCxRow {
            tag: opt_str(item, "tag"),
            summary: opt_str(item, "summary"),
            url: opt_str(item, "url"),
        });
    }
    out
}

const CAIXIN_HEADERS: &[(&str, &str)] = &[
    (
        "user-agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36",
    ),
    ("referer", "https://cxdata.caixin.com/index/newsTab?tab=latest"),
];

/// 财新网-财新数据通 新闻列表.
pub async fn stock_news_main_cx(client: &Client) -> Result<Vec<NewsMainCxRow>> {
    let params = [("pageNum", "1"), ("pageSize", "100"), ("showLabels", "true")];
    let v = client
        .get_json_with_headers(
            SOURCE_CAIXIN,
            "stock_news_main_cx",
            CAIXIN_URL,
            &params,
            Some(CAIXIN_HEADERS),
        )
        .await?;
    let data = v
        .get("data")
        .and_then(|d| d.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CAIXIN,
            message: "missing data.data".into(),
        })?;
    Ok(parse_news_main_cx(data))
}

// ===========================================================================
// stock_zh_a_tick_tx_js  (stock/stock_zh_a_tick_tx.py:16) — Tencent text feed
// ===========================================================================

const TICK_URL: &str = "http://stock.gtimg.cn/data/index.php";

/// One intraday transaction-detail (tick) row (Tencent).
#[derive(Debug, Clone, serde::Serialize)]
pub struct TickRow {
    /// 成交时间
    pub time: String,
    /// 成交价格
    pub price: Option<f64>,
    /// 价格变动
    pub change: Option<f64>,
    /// 成交量
    pub volume: Option<f64>,
    /// 成交金额
    pub amount: Option<f64>,
    /// 性质 (S→卖盘, B→买盘, M→中性盘)
    pub nature: Option<String>,
}

/// Extract the pipe-joined detail string (the `[1]` element of the embedded
/// array) from a Tencent `stock.gtimg.cn` detail response. The response is of
/// the form `var=["code","row1|row2|..."]`; akshare `eval`s it and takes index
/// 1. We locate the `","` boundary and pull the second quoted string.
fn extract_pipe_string(text: &str) -> Option<String> {
    let start = text.find('[')?;
    let rest = &text[start + 1..];
    let comma = rest.find("\",\"")?;
    let after = &rest[comma + 3..];
    let end = after.find('"')?;
    Some(after[..end].to_string())
}

/// Parse `stock_zh_a_tick_tx_js` rows from the raw Tencent response text.
pub(crate) fn parse_tick_text(text: &str) -> Vec<TickRow> {
    let Some(pipe) = extract_pipe_string(text) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for row in pipe.split('|') {
        if row.is_empty() {
            continue;
        }
        let parts: Vec<&str> = row.split('/').collect();
        // akshare drops the first `/`-split column; keep 6 fields after it.
        let data = if parts.len() == 7 { &parts[1..] } else { &parts[..] };
        if data.len() < 6 {
            continue;
        }
        let nature = match data[5] {
            "S" => Some("卖盘".to_string()),
            "B" => Some("买盘".to_string()),
            "M" => Some("中性盘".to_string()),
            other => Some(other.to_string()),
        };
        out.push(TickRow {
            time: data[0].to_string(),
            price: data[1].parse::<f64>().ok(),
            change: data[2].parse::<f64>().ok(),
            volume: data[3].parse::<f64>().ok(),
            amount: data[4].parse::<f64>().ok(),
            nature,
        });
    }
    out
}

/// 腾讯财经-历史分笔数据. `symbol` e.g. `sz000001`. Paginates until empty.
pub async fn stock_zh_a_tick_tx_js(client: &Client, symbol: &str) -> Result<Vec<TickRow>> {
    let mut out = Vec::new();
    let mut page: u32 = 0;
    loop {
        let page_s = page.to_string();
        let params = [
            ("appn", "detail"),
            ("action", "data"),
            ("c", symbol),
            ("p", page_s.as_str()),
        ];
        let text = client
            .get_text(SOURCE_TICK, "stock_zh_a_tick_tx_js", TICK_URL, &params, None)
            .await?;
        let rows = parse_tick_text(&text);
        if rows.is_empty() {
            break;
        }
        out.extend(rows);
        page += 1;
        if page > 200 {
            break;
        }
    }
    Ok(out)
}

// ===========================================================================
// stock_price_js  (stock/stock_us_js.py:13) — ushknews target prices
// ===========================================================================

const USHKNEWS_URL: &str = "https://calendar-api.ushknews.com/getWebTargetPriceList";

/// One US/HK stock target-price row (ushknews).
#[derive(Debug, Clone, serde::Serialize)]
pub struct PriceTargetRow {
    /// 日期 (date)
    pub date: Option<String>,
    /// 个股名称 (stock_name)
    pub stock_name: Option<String>,
    /// 评级 (rating)
    pub rating: Option<String>,
    /// 先前目标价 (prev_target_price)
    pub prev_target_price: Option<f64>,
    /// 最新目标价 (latest_target_price)
    pub latest_target_price: Option<f64>,
    /// 机构名称 (institution)
    pub institution: Option<String>,
}

/// Parse `stock_price_js` rows from the `data.list` array.
pub(crate) fn parse_price_target(list: &[Value]) -> Vec<PriceTargetRow> {
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        out.push(PriceTargetRow {
            date: opt_str(item, "date"),
            stock_name: opt_str(item, "stock_name"),
            rating: opt_str(item, "rating"),
            prev_target_price: opt_f64(item, "prev_target_price"),
            latest_target_price: opt_f64(item, "latest_target_price"),
            institution: opt_str(item, "institution"),
        });
    }
    out
}

const USHKNEWS_HEADERS: &[(&str, &str)] = &[
    ("accept", "application/json, text/plain, */*"),
    ("accept-encoding", "gzip, deflate, br"),
    ("accept-language", "zh-CN,zh;q=0.9,en;q=0.8"),
    ("cache-control", "no-cache"),
    ("origin", "https://www.ushknews.com"),
    ("pragma", "no-cache"),
    ("referer", "https://www.ushknews.com/"),
    ("sec-ch-ua", "\"Google Chrome\";v=\"107\", \"Chromium\";v=\"107\", \"Not=A?Brand\";v=\"24\""),
    ("sec-ch-ua-mobile", "?0"),
    ("sec-ch-ua-platform", "\"Windows\""),
    ("sec-fetch-dest", "empty"),
    ("sec-fetch-mode", "cors"),
    ("sec-fetch-site", "same-site"),
    (
        "user-agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36",
    ),
    ("x-app-id", "BNsiR9uq7yfW0LVz"),
    ("x-version", "1.0.0"),
];

/// 美股/港股目标价. `symbol` ∈ {us, hk}; default `us`.
pub async fn stock_price_js(client: &Client, symbol: &str) -> Result<Vec<PriceTargetRow>> {
    if symbol != "us" && symbol != "hk" {
        return Err(Error::InvalidParam(format!(
            "stock_price_js: symbol must be us or hk, got {symbol}"
        )));
    }
    let params = [("limit", "20"), ("category", symbol)];
    let v = client
        .get_json_with_headers(
            SOURCE_USHKNEWS,
            "stock_price_js",
            USHKNEWS_URL,
            &params,
            Some(USHKNEWS_HEADERS),
        )
        .await?;
    let list = v
        .get("data")
        .and_then(|d| d.get("list"))
        .and_then(|l| l.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_USHKNEWS,
            message: "missing data.list".into(),
        })?;
    Ok(parse_price_target(list))
}

// ===========================================================================
// stock_staq_net_stop  (stock/stock_stop.py:13) — Eastmoney push2 clist
// ===========================================================================

const STAQ_URL: &str = "https://5.push2.eastmoney.com/api/qt/clist/get";

/// One two-net / delisted stock row.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StaqNetStopRow {
    /// 序号 (synthesized 1-based index)
    pub seq: usize,
    /// 代码 (f12)
    pub code: String,
    /// 名称 (f14)
    pub name: String,
}

/// Parse `stock_staq_net_stop` rows from a push2 `data.diff` array.
pub(crate) fn parse_staq_net_stop(diff: &[Value]) -> Vec<StaqNetStopRow> {
    let mut out = Vec::with_capacity(diff.len());
    for (i, item) in diff.iter().enumerate() {
        let Some(code) = opt_str(item, "f12") else {
            continue;
        };
        let Some(name) = opt_str(item, "f14") else {
            continue;
        };
        out.push(StaqNetStopRow {
            seq: i + 1,
            code,
            name,
        });
    }
    out
}

/// 东方财富-两网及退市 股票列表.
pub async fn stock_staq_net_stop(client: &Client) -> Result<Vec<StaqNetStopRow>> {
    let params = [
        ("pn", "1"),
        ("pz", "50000"),
        ("po", "1"),
        ("np", "2"),
        ("ut", "bd1d9ddb04089700cf9c27f6f7426281"),
        ("fltt", "2"),
        ("invt", "2"),
        ("fid", "f3"),
        ("fs", "m:0 s:3"),
        ("fields", "f12,f14"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_staq_net_stop", STAQ_URL, &params)
        .await?;
    let diff = v
        .get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|x| x.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff".into(),
        })?;
    Ok(parse_staq_net_stop(diff))
}

// ===========================================================================
// stock_zh_ah_spot / stock_zh_ah_name / stock_zh_ah_daily
//   (stock/stock_zh_ah_tx.py:40 / :110 / :157) — Tencent
// ===========================================================================

const HK_RANK_URL: &str = "http://stock.gtimg.cn/data/hk_rank.php";
const HK_KLINE_URL: &str = "http://web.ifzq.gtimg.cn/appstock/app/kline/kline";
const HK_FQ_KLINE_URL: &str = "https://web.ifzq.gtimg.cn/appstock/app/hkfqkline/get";

const HK_HEADERS: &[(&str, &str)] = &[
    ("Referer", "http://stockapp.finance.qq.com/mstats/"),
    (
        "User-Agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/77.0.3865.120 Safari/537.36",
    ),
];

const HK_KLINE_HEADERS: &[(&str, &str)] = &[
    ("Accept", "*/*"),
    ("Accept-Encoding", "gzip, deflate"),
    ("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8"),
    ("Cache-Control", "no-cache"),
    ("Connection", "keep-alive"),
    ("Host", "web.ifzq.gtimg.cn"),
    ("Pragma", "no-cache"),
    ("Referer", "http://gu.qq.com/hk01033/gp"),
    (
        "User-Agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/84.0.4147.125 Safari/537.36",
    ),
];

const HK_STOCK_HEADERS: &[(&str, &str)] = &[(
    "User-Agent",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/77.0.3865.120 Safari/537.36",
)];

/// Build the Tencent `hk_rank.php` params for a given page.
fn hk_rank_params(page: u64) -> Vec<(String, String)> {
    vec![
        ("board".into(), "A_H".into()),
        ("metric".into(), "price".into()),
        ("pageSize".into(), "20".into()),
        ("reqPage".into(), page.to_string()),
        ("order".into(), "decs".into()),
        ("var_name".into(), "list_data".into()),
    ]
}

/// Decode a Tencent response that wraps a JS object literal (akshare uses
/// `demjson`); we slice from the first `{` to the last `}` and parse as JSON.
fn decode_tencent_obj(text: &str) -> Result<Value> {
    let s = text.find('{').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_TENCENT,
        message: "no '{' in tencent response".into(),
    })?;
    let e = text.rfind('}').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_TENCENT,
        message: "no '}' in tencent response".into(),
    })?;
    serde_json::from_str(&text[s..=e]).map_err(Error::Json)
}

/// One AH realtime quote row (Tencent `hk_rank.php`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct AhSpotRow {
    /// 代码
    pub code: String,
    /// 名称
    pub name: String,
    /// 最新价
    pub price: Option<f64>,
    /// 涨跌幅
    pub pct: Option<f64>,
    /// 涨跌额
    pub change: Option<f64>,
    /// 买入
    pub buy: Option<f64>,
    /// 卖出
    pub sell: Option<f64>,
    /// 成交量
    pub volume: Option<f64>,
    /// 成交额
    pub amount: Option<f64>,
    /// 今开
    pub open: Option<f64>,
    /// 昨收
    pub prev_close: Option<f64>,
    /// 最高
    pub high: Option<f64>,
    /// 最低
    pub low: Option<f64>,
}

/// Parse `stock_zh_ah_spot` rows from a `page_data` array (each item is
/// `["code~name~...~..."]`; `~`-split yields 14 fields, the last is dropped).
pub(crate) fn parse_ah_spot(page_data: &[Value]) -> Vec<AhSpotRow> {
    let mut out = Vec::with_capacity(page_data.len());
    for item in page_data {
        let s = item
            .get(0)
            .and_then(|x| x.as_str())
            .or_else(|| item.as_str());
        let Some(s) = s else {
            continue;
        };
        let parts: Vec<&str> = s.split('~').collect();
        if parts.len() < 14 {
            continue;
        }
        let p = |i: usize| parts[i].parse::<f64>().ok();
        out.push(AhSpotRow {
            code: parts[0].to_string(),
            name: parts[1].to_string(),
            price: p(2),
            pct: p(3),
            change: p(4),
            buy: p(5),
            sell: p(6),
            volume: p(7),
            amount: p(8),
            open: p(9),
            prev_close: p(10),
            high: p(11),
            low: p(12),
        });
    }
    out
}

/// One AH code/name pair.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AhNameRow {
    /// 代码
    pub code: String,
    /// 名称
    pub name: String,
}

/// Parse `stock_zh_ah_name` rows (only code & name) from a `page_data` array.
pub(crate) fn parse_ah_name(page_data: &[Value]) -> Vec<AhNameRow> {
    let mut out = Vec::with_capacity(page_data.len());
    for item in page_data {
        let s = item
            .get(0)
            .and_then(|x| x.as_str())
            .or_else(|| item.as_str());
        let Some(s) = s else {
            continue;
        };
        let parts: Vec<&str> = s.split('~').collect();
        if parts.len() < 2 {
            continue;
        }
        out.push(AhNameRow {
            code: parts[0].to_string(),
            name: parts[1].to_string(),
        });
    }
    out
}

/// Fetch the AH page count from `hk_rank.php`.
async fn ah_page_count(client: &Client) -> Result<u64> {
    let owned = hk_rank_params(0);
    let p: Vec<(&str, &str)> = owned.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let text = client
        .get_text(SOURCE_TENCENT, "stock_zh_ah_page_count", HK_RANK_URL, &p, Some(HK_HEADERS))
        .await?;
    let v = decode_tencent_obj(&text)?;
    v.get("data")
        .and_then(|d| d.get("page_count"))
        .and_then(|c| c.as_u64())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_TENCENT,
            message: "missing data.page_count".into(),
        })
}

/// 腾讯财经-港股-AH-实时行情.
pub async fn stock_zh_ah_spot(client: &Client) -> Result<Vec<AhSpotRow>> {
    let page_count = ah_page_count(client).await?.min(200);
    let mut out = Vec::new();
    for page in 0..page_count {
        let owned = hk_rank_params(page);
        let p: Vec<(&str, &str)> =
            owned.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let text = client
            .get_text(SOURCE_TENCENT, "stock_zh_ah_spot", HK_RANK_URL, &p, Some(HK_HEADERS))
            .await?;
        let v = decode_tencent_obj(&text)?;
        let page_data = v
            .get("data")
            .and_then(|d| d.get("page_data"))
            .and_then(|x| x.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_TENCENT,
                message: "missing data.page_data".into(),
            })?;
        out.extend(parse_ah_spot(page_data));
    }
    Ok(out)
}

/// 腾讯财经-港股-AH-股票名称 (code/name pairs).
pub async fn stock_zh_ah_name(client: &Client) -> Result<Vec<AhNameRow>> {
    let page_count = ah_page_count(client).await?.min(200);
    let mut out = Vec::new();
    for page in 0..page_count {
        let owned = hk_rank_params(page);
        let p: Vec<(&str, &str)> =
            owned.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let text = client
            .get_text(SOURCE_TENCENT, "stock_zh_ah_name", HK_RANK_URL, &p, Some(HK_HEADERS))
            .await?;
        let v = decode_tencent_obj(&text)?;
        let page_data = v
            .get("data")
            .and_then(|d| d.get("page_data"))
            .and_then(|x| x.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_TENCENT,
                message: "missing data.page_data".into(),
            })?;
        out.extend(parse_ah_name(page_data));
    }
    Ok(out)
}

/// One AH daily history row (Tencent `gtimg` kline).
#[derive(Debug, Clone, serde::Serialize)]
pub struct AhDailyRow {
    /// 日期
    pub date: String,
    /// 开盘
    pub open: Option<f64>,
    /// 收盘
    pub close: Option<f64>,
    /// 最高
    pub high: Option<f64>,
    /// 最低
    pub low: Option<f64>,
    /// 成交量
    pub volume: Option<f64>,
}

/// Parse `stock_zh_ah_daily` rows from a `day` (or `{adjust}day`) array.
pub(crate) fn parse_ah_daily(day: &[Value]) -> Vec<AhDailyRow> {
    let mut out = Vec::with_capacity(day.len());
    for item in day {
        let Some(arr) = item.as_array() else {
            continue;
        };
        if arr.len() < 6 {
            continue;
        }
        out.push(AhDailyRow {
            date: str_at(arr, 0),
            open: f64_at(arr, 1),
            close: f64_at(arr, 2),
            high: f64_at(arr, 3),
            low: f64_at(arr, 4),
            volume: f64_at(arr, 5),
        });
    }
    out
}

/// 腾讯财经-港股-AH-股票历史行情. `symbol` e.g. `02318`; `adjust` ∈ {"", qfq, hfq}.
pub async fn stock_zh_ah_daily(
    client: &Client,
    symbol: &str,
    start_year: &str,
    end_year: &str,
    adjust: &str,
) -> Result<Vec<AhDailyRow>> {
    let start: i32 = start_year
        .parse()
        .map_err(|_| Error::InvalidParam(format!("stock_zh_ah_daily: bad start_year: {start_year}")))?;
    let end: i32 = end_year
        .parse()
        .map_err(|_| Error::InvalidParam(format!("stock_zh_ah_daily: bad end_year: {end_year}")))?;
    if !adjust.is_empty() && adjust != "qfq" && adjust != "hfq" {
        return Err(Error::InvalidParam(format!(
            "stock_zh_ah_daily: adjust must be '', qfq or hfq, got {adjust}"
        )));
    }
    let mut out = Vec::new();
    for year in start..end {
        let (url, headers): (&str, &[(&str, &str)]) = if adjust.is_empty() {
            (HK_KLINE_URL, HK_KLINE_HEADERS)
        } else {
            (HK_FQ_KLINE_URL, HK_STOCK_HEADERS)
        };
        let param = if adjust.is_empty() {
            format!("hk{symbol},day,{year}-01-01,{}-12-31,640,", year + 1)
        } else {
            format!("hk{symbol},day,{year}-01-01,{}-12-31,640,{adjust}", year + 1)
        };
        let var = format!("kline_day{adjust}{year}");
        let r = format!(
            "{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let owned: Vec<(String, String)> = vec![
            ("_var".into(), var),
            ("param".into(), param),
            ("r".into(), r),
        ];
        let p: Vec<(&str, &str)> = owned.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let text = client
            .get_text(SOURCE_TENCENT, "stock_zh_ah_daily", url, &p, Some(headers))
            .await?;
        let v = decode_tencent_obj(&text)?;
        let hk_key = format!("hk{symbol}");
        let day_key = format!("{adjust}day");
        let day = v
            .get("data")
            .and_then(|d| d.get(&hk_key))
            .and_then(|d| d.get(&day_key))
            .and_then(|x| x.as_array());
        let Some(day) = day else {
            continue;
        };
        out.extend(parse_ah_daily(day));
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

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    fn fixture_json(name: &str) -> Value {
        let p = fixture_path(name);
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    fn fixture_text(name: &str) -> String {
        std::fs::read_to_string(fixture_path(name)).unwrap()
    }

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    // ---- stock_hold_management_person_em ----
    #[test]
    fn parse_hold_management_person_ok() {
        let _fx1 = fixture_json("stock_hold_management_person_em.json");
        let data = _fx1
            .get("result")
            .unwrap()
            .get("data")
            .unwrap()
            .as_array()
            .unwrap();
        let rows = parse_hold_management_person(data);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, Some("001308".into()));
        assert_eq!(rows[0].person, Some("吴远".into()));
        assert!(approx(rows[0].change_shares, 10000.0));
        assert!(approx(rows[0].avg_price, 15.2));
        assert_eq!(rows[1].name, Some("某某股份".into()));
    }

    // ---- stock_hot_search_baidu ----
    #[test]
    fn parse_hot_search_ok() {
        let _fx2 = fixture_json("stock_hot_search_baidu.json");
        let body = _fx2
            .get("Result")
            .unwrap()
            .get("list")
            .unwrap()
            .get("body")
            .unwrap()
            .as_array()
            .unwrap();
        let rows = parse_hot_search(body);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "贵州茅台");
        assert!(approx(rows[0].change_rate, 1.23));
        assert!(approx(rows[0].heat, 98765.0));
        assert_eq!(rows[1].name, "宁德时代");
    }

    // ---- stock_share_hold_change_sse ----
    #[test]
    fn parse_sse_share_hold_ok() {
        let _fx3 = fixture_json("stock_share_hold_change_sse.json");
        let result = _fx3
            .get("result")
            .unwrap()
            .as_array()
            .unwrap();
        let rows = parse_sse_share_hold(result);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].company_code, Some("600000".into()));
        assert_eq!(rows[0].name, Some("张三".into()));
        assert!(approx(rows[0].change_num, 5000.0));
        assert!(approx(rows[0].avg_price, 8.5));
        assert_eq!(rows[1].company_name, Some("示例公司".into()));
    }

    // ---- stock_share_hold_change_szse ----
    #[test]
    fn parse_szse_share_hold_ok() {
        let _fx4 = fixture_json("stock_share_hold_change_szse.json");
        let arr = _fx4
            .as_array()
            .unwrap();
        let data = arr[0].get("data").unwrap().as_array().unwrap();
        let rows = parse_szse_share_hold(data);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, Some("000001".into()));
        assert_eq!(rows[0].person, Some("李四".into()));
        assert!(approx(rows[0].change_num, 3000.0));
        assert!(approx(rows[0].avg_price, 12.3));
        assert!(approx(rows[0].after_hold, 1234567.0));
    }

    // ---- stock_share_hold_change_bse ----
    #[test]
    fn parse_bse_share_hold_ok() {
        let _fx5 = fixture_json("stock_share_hold_change_bse.json");
        let arr = _fx5
            .as_array()
            .unwrap();
        let content = arr[0]
            .get("result")
            .unwrap()
            .get("content")
            .unwrap()
            .as_array()
            .unwrap();
        let rows = parse_bse_share_hold(content);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, Some("430489".into()));
        assert_eq!(rows[0].person, Some("王五".into()));
        assert!(approx(rows[0].change_amount, 2000.0));
        assert!(approx(rows[0].new_amount, 8000.0));
        assert_eq!(rows[1].name, Some("北交所股".into()));
    }

    // ---- stock_news_main_cx ----
    #[test]
    fn parse_news_main_cx_ok() {
        let _fx6 = fixture_json("stock_news_main_cx.json");
        let data = _fx6
            .get("data")
            .unwrap()
            .get("data")
            .unwrap()
            .as_array()
            .unwrap();
        let rows = parse_news_main_cx(data);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].tag, Some("宏观".into()));
        assert_eq!(rows[0].summary, Some("财新PMI超预期".into()));
        assert_eq!(rows[0].url, Some("https://cxdata.caixin.com/1".into()));
    }

    // ---- stock_zh_a_tick_tx_js ----
    #[test]
    fn parse_tick_text_ok() {
        let rows = parse_tick_text(&fixture_text("stock_zh_a_tick_tx_js.txt"));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].time, "09:30:00");
        assert!(approx(rows[0].price, 13.50));
        assert!(approx(rows[0].change, 0.10));
        assert!(approx(rows[0].volume, 100.0));
        assert!(approx(rows[0].amount, 1350000.0));
        assert_eq!(rows[0].nature, Some("买盘".into()));
        assert_eq!(rows[1].nature, Some("卖盘".into()));
    }

    // ---- stock_price_js ----
    #[test]
    fn parse_price_target_ok() {
        let _fx7 = fixture_json("stock_price_js.json");
        let list = _fx7
            .get("data")
            .unwrap()
            .get("list")
            .unwrap()
            .as_array()
            .unwrap();
        let rows = parse_price_target(list);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].stock_name, Some("Apple".into()));
        assert_eq!(rows[0].rating, Some("买入".into()));
        assert!(approx(rows[0].prev_target_price, 180.0));
        assert!(approx(rows[0].latest_target_price, 200.0));
        assert_eq!(rows[0].institution, Some("大摩".into()));
    }

    // ---- stock_staq_net_stop ----
    #[test]
    fn parse_staq_net_stop_ok() {
        let _fx8 = fixture_json("stock_staq_net_stop.json");
        let diff = _fx8
            .get("data")
            .unwrap()
            .get("diff")
            .unwrap()
            .as_array()
            .unwrap();
        let rows = parse_staq_net_stop(diff);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].seq, 1);
        assert_eq!(rows[0].code, "400001");
        assert_eq!(rows[0].name, "某某A");
        assert_eq!(rows[1].code, "400002");
    }

    // ---- stock_zh_ah_spot ----
    #[test]
    fn parse_ah_spot_ok() {
        let _fx9 = fixture_json("stock_zh_ah_spot.json");
        let page_data = _fx9
            .get("data")
            .unwrap()
            .get("page_data")
            .unwrap()
            .as_array()
            .unwrap();
        let rows = parse_ah_spot(page_data);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "HK00001");
        assert_eq!(rows[0].name, "长江实业");
        assert!(approx(rows[0].price, 50.25));
        assert!(approx(rows[0].pct, 1.20));
        assert!(approx(rows[0].low, 49.10));
    }

    // ---- stock_zh_ah_name ----
    #[test]
    fn parse_ah_name_ok() {
        let _fx10 = fixture_json("stock_zh_ah_name.json");
        let page_data = _fx10
            .get("data")
            .unwrap()
            .get("page_data")
            .unwrap()
            .as_array()
            .unwrap();
        let rows = parse_ah_name(page_data);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "HK00001");
        assert_eq!(rows[0].name, "长江实业");
        assert_eq!(rows[1].code, "HK00002");
    }

    // ---- stock_zh_ah_daily ----
    #[test]
    fn parse_ah_daily_ok() {
        let _fx11 = fixture_json("stock_zh_ah_daily.json");
        let day = _fx11
            .get("data")
            .unwrap()
            .get("hk02318")
            .unwrap()
            .get("day")
            .unwrap()
            .as_array()
            .unwrap();
        let rows = parse_ah_daily(day);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2022-01-04");
        assert!(approx(rows[0].open, 45.0));
        assert!(approx(rows[0].close, 46.2));
        assert!(approx(rows[0].high, 46.8));
        assert!(approx(rows[0].low, 44.9));
        assert!(approx(rows[0].volume, 1000000.0));
        assert_eq!(rows[1].date, "2022-01-05");
    }
}
