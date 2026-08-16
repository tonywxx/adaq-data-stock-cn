//! Sina / Eastmoney SSE option endpoints (akshare `option_finance_sina.py`).
//!
//! Ports of the akshare SSE (上海证券交易所) option functions that are pure HTTP
//! (Sina JSON / JSONP plus one Eastmoney JSONP), requiring no JS signing, tokens
//! or cookies.
//!
//! ## Ported
//! - [`option_sse_list_sina`] — `option_finance_sina.py:422`
//! - [`option_sse_expire_day_sina`] — `option_finance_sina.py:441`
//! - [`option_sse_codes_sina`] — `option_finance_sina.py:477`
//! - [`option_sse_spot_price_sina`] — `option_finance_sina.py:542`
//! - [`option_sse_underlying_spot_price_sina`] — `option_finance_sina.py:621`
//! - [`option_sse_greeks_sina`] — `option_finance_sina.py:686`
//! - [`option_sse_minute_sina`] — `option_finance_sina.py:732`
//! - [`option_sse_daily_sina`] — `option_finance_sina.py:776`
//! - [`option_finance_minute_sina`] — `option_finance_sina.py:816`
//! - [`option_minute_em`] — `option_finance_sina.py:865`
//!
//! ## DEFERRED
//! - `option_cffex_sz50_list_sina` / `option_cffex_hs300_list_sina` /
//!   `option_cffex_zz1000_list_sina` (`option_finance_sina.py:28/45/61`) — HTML
//!   `<table>` scrape via BeautifulSoup; not pure HTTP.
//! - `option_cffex_sz50_spot_sina` / `_hs300_` / `_zz1000_` (`:77/150/223`) and
//!   `option_cffex_*_daily_sina` (`:296/337/378`) — already ported via
//!   `src/option/sina.rs` (`option_cffex_spot_sina` / `option_cffex_daily`).

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// Sina source identifier (mirrors `core::client::SOURCE_SINA`).
const SOURCE_SINA: &str = "sina";
/// Eastmoney source identifier (mirrors `core::client::SOURCE_EASTMONEY`).
const SOURCE_EASTMONEY: &str = "eastmoney";

/// Referer used by Sina `hq.sinajs.cn` / `openapi.php` option endpoints.
const SINA_REFERER_QUOTES: &str = "https://stock.finance.sina.com.cn/";
/// Referer used by Sina VIP option endpoints (`CON_SO_` / underlying spot).
const SINA_REFERER_VIP: &str = "https://vip.stock.finance.sina.com.cn/";

// ---------------------------------------------------------------------------
// 1. option_sse_list_sina — contract expiry-month list
// ---------------------------------------------------------------------------

/// A single SSE option contract expiry month (Sina).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SseContractMonthRow {
    /// 合约到期月份 (expiry month, e.g. `"202102"`)
    pub month: Option<String>,
    pub source: &'static str,
}

/// Contract expiry-month list for an SSE ETF option
/// (akshare `option_sse_list_sina`, `option_finance_sina.py:422`).
///
/// `symbol` is `"50ETF"` or `"300ETF"`; `exchange` is normally `"null"`.
/// The upstream `contractMonth` array is `"YYYY-MM"`-shaped; the first entry is
/// dropped (it is the current month) and the rest are concatenated to `"YYYYMM"`.
pub async fn option_sse_list_sina(
    client: &Client,
    symbol: &str,
    exchange: &str,
) -> Result<Vec<SseContractMonthRow>> {
    let url =
        "https://stock.finance.sina.com.cn/futures/api/openapi.php/StockOptionService.getStockName";
    let params = [("exchange", exchange), ("cate", symbol)];
    let v = client
        .get_json(SOURCE_SINA, "option_sse_list_sina", url, &params)
        .await?;
    parse_sse_list(&v)
}

pub(crate) fn parse_sse_list(resp: &Value) -> Result<Vec<SseContractMonthRow>> {
    let months = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.get("contractMonth"))
        .and_then(|m| m.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "list response missing result.data.contractMonth".into(),
        })?;
    let mut out = Vec::new();
    for (i, m) in months.iter().enumerate() {
        // akshare drops the first (current) month.
        if i == 0 {
            continue;
        }
        let s = m.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "contract month is not a string".into(),
        })?;
        let joined: String = s.split('-').collect();
        out.push(SseContractMonthRow {
            month: Some(joined),
            source: SOURCE_SINA,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// 2. option_sse_expire_day_sina — remaining days to expiry
// ---------------------------------------------------------------------------

/// Remaining days until a contract month expires (Sina).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SseExpireDayRow {
    /// 到期日 (expire day, e.g. `"2021-02-24"`)
    pub expire_day: Option<String>,
    /// 剩余天数 (remaining days, may be negative for past months)
    pub remainder_days: Option<i64>,
    pub source: &'static str,
}

/// Remaining days to expiry for an SSE ETF option contract month
/// (akshare `option_sse_expire_day_sina`, `option_finance_sina.py:441`).
///
/// `trade_date` is the expiry month, e.g. `"202102"`; `symbol` is `"50ETF"` /
/// `"300ETF"`; `exchange` is normally `"null"`. When the remaining days are
/// negative the upstream is re-queried with the `XD`-prefixed cate (ex-divident).
pub async fn option_sse_expire_day_sina(
    client: &Client,
    trade_date: &str,
    symbol: &str,
    exchange: &str,
) -> Result<Vec<SseExpireDayRow>> {
    let date = format!("{}-{}", &trade_date[..4], &trade_date[4..]);
    let url = "https://stock.finance.sina.com.cn/futures/api/openapi.php/StockOptionService.getRemainderDay";
    let params = [
        ("exchange", exchange),
        ("cate", symbol),
        ("date", date.as_str()),
    ];
    let v = client
        .get_json(SOURCE_SINA, "option_sse_expire_day_sina", url, &params)
        .await?;
    let remainder = v
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.get("remainderDays"))
        .and_then(|x| x.as_i64())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "expire-day response missing remainderDays".into(),
        })?;

    let rows = if remainder < 0 {
        let cate = format!("XD{symbol}");
        let params2 = [
            ("exchange", exchange),
            ("cate", cate.as_str()),
            ("date", date.as_str()),
        ];
        let v2 = client
            .get_json(SOURCE_SINA, "option_sse_expire_day_sina", url, &params2)
            .await?;
        parse_sse_expire_day(&v2)?
    } else {
        parse_sse_expire_day(&v)?
    };

    Ok(rows)
}

/// Parse an SSE expire-day response (`result.data`) into a single row.
/// The async wrapper re-queries with the `XD`-prefixed cate when remaining
/// days are negative; this handles one response blob.
pub(crate) fn parse_sse_expire_day(resp: &Value) -> Result<Vec<SseExpireDayRow>> {
    let data = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "expire-day response missing result.data".into(),
        })?;
    let expire_day = data
        .get("expireDay")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    let remainder_days = data.get("remainderDays").and_then(|x| x.as_i64());
    Ok(vec![SseExpireDayRow {
        expire_day,
        remainder_days,
        source: SOURCE_SINA,
    }])
}

// ---------------------------------------------------------------------------
// 3. option_sse_codes_sina — call/put contract codes for a month
// ---------------------------------------------------------------------------

/// A single SSE option contract code (Sina `OP_UP_` / `OP_DOWN_`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SseOptionCodeRow {
    /// 序号 (1-based sequence number within the month)
    pub index: Option<i64>,
    /// 期权代码 (option contract code, e.g. `"10003720"`)
    pub code: Option<String>,
    pub source: &'static str,
}

/// Call / put contract codes for an SSE ETF option month
/// (akshare `option_sse_codes_sina`, `option_finance_sina.py:477`).
///
/// `symbol` selects the side: `"看涨期权"` (call, `OP_UP_`) or `"看跌期权"`
/// (put, `OP_DOWN_`); `trade_date` is the expiry month (e.g. `"202202"`);
/// `underlying` is the ETF code (`510050` / `510300`).
pub async fn option_sse_codes_sina(
    client: &Client,
    symbol: &str,
    trade_date: &str,
    underlying: &str,
) -> Result<Vec<SseOptionCodeRow>> {
    let suffix = if symbol == "看涨期权" {
        "OP_UP_"
    } else {
        "OP_DOWN_"
    };
    let list = format!(
        "https://hq.sinajs.cn/list={}{}{}",
        suffix,
        underlying,
        &trade_date[trade_date.len() - 4..]
    );
    let headers = [("Referer", SINA_REFERER_QUOTES)];
    let text = client
        .get_text(
            SOURCE_SINA,
            "option_sse_codes_sina",
            &list,
            &[],
            Some(&headers),
        )
        .await?;
    parse_sse_codes(&text)
}

pub(crate) fn parse_sse_codes(text: &str) -> Result<Vec<SseOptionCodeRow>> {
    let inner = sinajs_inner(text)?;
    let mut out = Vec::new();
    let mut idx: i64 = 1;
    for token in inner {
        if let Some(code) = token.strip_prefix("CON_OP_") {
            out.push(SseOptionCodeRow {
                index: Some(idx),
                code: Some(code.to_string()),
                source: SOURCE_SINA,
            });
            idx += 1;
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// 4. option_sse_spot_price_sina — realtime quote for one contract
// ---------------------------------------------------------------------------

/// Realtime quote for a single SSE option contract (Sina `CON_OP_`).
///
/// Mirrors akshare `option_sse_spot_price_sina` (`option_finance_sina.py:542`),
/// which pivots the upstream comma-joined fields into a 43-field record. Numeric
/// fields are `Option<f64>`; text fields are `Option<String>`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SseSpotPriceRow {
    /// 买量 (bid volume)
    pub bid_volume: Option<f64>,
    /// 买价 (bid price)
    pub bid_price: Option<f64>,
    /// 最新价 (last price)
    pub last_price: Option<f64>,
    /// 卖价 (ask price)
    pub ask_price: Option<f64>,
    /// 卖量 (ask volume)
    pub ask_volume: Option<f64>,
    /// 持仓量 (open interest)
    pub open_interest: Option<f64>,
    /// 涨幅 (change percent)
    pub change_pct: Option<f64>,
    /// 行权价 (strike price)
    pub strike_price: Option<f64>,
    /// 昨收价 (previous close)
    pub prev_close: Option<f64>,
    /// 开盘价 (open)
    pub open: Option<f64>,
    /// 涨停价 (limit-up price)
    pub limit_up: Option<f64>,
    /// 跌停价 (limit-down price)
    pub limit_down: Option<f64>,
    /// 申卖价五 (ask price level 5)
    pub ask5_price: Option<f64>,
    /// 申卖量五 (ask volume level 5)
    pub ask5_volume: Option<f64>,
    /// 申卖价四 (ask price level 4)
    pub ask4_price: Option<f64>,
    /// 申卖量四 (ask volume level 4)
    pub ask4_volume: Option<f64>,
    /// 申卖价三 (ask price level 3)
    pub ask3_price: Option<f64>,
    /// 申卖量三 (ask volume level 3)
    pub ask3_volume: Option<f64>,
    /// 申卖价二 (ask price level 2)
    pub ask2_price: Option<f64>,
    /// 申卖量二 (ask volume level 2)
    pub ask2_volume: Option<f64>,
    /// 申卖价一 (ask price level 1)
    pub ask1_price: Option<f64>,
    /// 申卖量一 (ask volume level 1)
    pub ask1_volume: Option<f64>,
    /// 申买价一 (bid price level 1)
    pub bid1_price: Option<f64>,
    /// 申买量一 (bid volume level 1)
    pub bid1_volume: Option<f64>,
    /// 申买价二 (bid price level 2)
    pub bid2_price: Option<f64>,
    /// 申买量二 (bid volume level 2)
    pub bid2_volume: Option<f64>,
    /// 申买价三 (bid price level 3)
    pub bid3_price: Option<f64>,
    /// 申买量三 (bid volume level 3)
    pub bid3_volume: Option<f64>,
    /// 申买价四 (bid price level 4)
    pub bid4_price: Option<f64>,
    /// 申买量四 (bid volume level 4)
    pub bid4_volume: Option<f64>,
    /// 申买价五 (bid price level 5)
    pub bid5_price: Option<f64>,
    /// 申买量五 (bid volume level 5)
    pub bid5_volume: Option<f64>,
    /// 行情时间 (quote time)
    pub quote_time: Option<String>,
    /// 主力合约标识 (main-contract flag)
    pub main_contract_flag: Option<String>,
    /// 状态码 (status code)
    pub status_code: Option<String>,
    /// 标的证券类型 (underlying security type)
    pub underlying_type: Option<String>,
    /// 标的股票 (underlying stock)
    pub underlying_stock: Option<String>,
    /// 期权合约简称 (option contract short name)
    pub contract_name: Option<String>,
    /// 振幅 (amplitude)
    pub amplitude: Option<f64>,
    /// 最高价 (high)
    pub high: Option<f64>,
    /// 最低价 (low)
    pub low: Option<f64>,
    /// 成交量 (volume)
    pub volume: Option<f64>,
    /// 成交额 (amount)
    pub amount: Option<f64>,
    pub source: &'static str,
}

/// Realtime quote for one SSE option contract
/// (akshare `option_sse_spot_price_sina`, `option_finance_sina.py:542`).
///
/// `symbol` is the contract code (e.g. `"10003720"`); the upstream `CON_OP_`
/// `hq.sinajs.cn` feed is comma-joined inside a quoted string.
pub async fn option_sse_spot_price_sina(
    client: &Client,
    symbol: &str,
) -> Result<Vec<SseSpotPriceRow>> {
    let list = format!("https://hq.sinajs.cn/list=CON_OP_{symbol}");
    let headers = [("Referer", SINA_REFERER_QUOTES)];
    let text = client
        .get_text(
            SOURCE_SINA,
            "option_sse_spot_price_sina",
            &list,
            &[],
            Some(&headers),
        )
        .await?;
    parse_sse_spot_price(&text)
}

pub(crate) fn parse_sse_spot_price(text: &str) -> Result<Vec<SseSpotPriceRow>> {
    let f = sinajs_inner(text)?;
    if f.len() < 43 {
        return Err(Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: format!("spot price expected 43 fields, got {}", f.len()),
        });
    }
    Ok(vec![SseSpotPriceRow {
        bid_volume: field_num(&f, 0),
        bid_price: field_num(&f, 1),
        last_price: field_num(&f, 2),
        ask_price: field_num(&f, 3),
        ask_volume: field_num(&f, 4),
        open_interest: field_num(&f, 5),
        change_pct: field_num(&f, 6),
        strike_price: field_num(&f, 7),
        prev_close: field_num(&f, 8),
        open: field_num(&f, 9),
        limit_up: field_num(&f, 10),
        limit_down: field_num(&f, 11),
        ask5_price: field_num(&f, 12),
        ask5_volume: field_num(&f, 13),
        ask4_price: field_num(&f, 14),
        ask4_volume: field_num(&f, 15),
        ask3_price: field_num(&f, 16),
        ask3_volume: field_num(&f, 17),
        ask2_price: field_num(&f, 18),
        ask2_volume: field_num(&f, 19),
        ask1_price: field_num(&f, 20),
        ask1_volume: field_num(&f, 21),
        bid1_price: field_num(&f, 22),
        bid1_volume: field_num(&f, 23),
        bid2_price: field_num(&f, 24),
        bid2_volume: field_num(&f, 25),
        bid3_price: field_num(&f, 26),
        bid3_volume: field_num(&f, 27),
        bid4_price: field_num(&f, 28),
        bid4_volume: field_num(&f, 29),
        bid5_price: field_num(&f, 30),
        bid5_volume: field_num(&f, 31),
        quote_time: field_str(&f, 32),
        main_contract_flag: field_str(&f, 33),
        status_code: field_str(&f, 34),
        underlying_type: field_str(&f, 35),
        underlying_stock: field_str(&f, 36),
        contract_name: field_str(&f, 37),
        amplitude: field_num(&f, 38),
        high: field_num(&f, 39),
        low: field_num(&f, 40),
        volume: field_num(&f, 41),
        amount: field_num(&f, 42),
        source: SOURCE_SINA,
    }])
}

// ---------------------------------------------------------------------------
// 5. option_sse_underlying_spot_price_sina — underlying ETF realtime quote
// ---------------------------------------------------------------------------

/// Realtime quote for the underlying ETF of an SSE option (Sina `hq.sinajs.cn`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SseUnderlyingSpotRow {
    /// 证券简称 (security short name)
    pub name: Option<String>,
    /// 今日开盘价 (today open)
    pub open: Option<f64>,
    /// 昨日收盘价 (previous close)
    pub prev_close: Option<f64>,
    /// 最近成交价 (last price)
    pub last_price: Option<f64>,
    /// 最高成交价 (high)
    pub high: Option<f64>,
    /// 最低成交价 (low)
    pub low: Option<f64>,
    /// 买入价 (bid price)
    pub bid_price: Option<f64>,
    /// 卖出价 (ask price)
    pub ask_price: Option<f64>,
    /// 成交数量 (volume)
    pub volume: Option<f64>,
    /// 成交金额 (amount)
    pub amount: Option<f64>,
    /// 买数量一 (bid volume level 1)
    pub bid1_volume: Option<f64>,
    /// 买价位一 (bid price level 1)
    pub bid1_price: Option<f64>,
    /// 买数量二 (bid volume level 2)
    pub bid2_volume: Option<f64>,
    /// 买价位二 (bid price level 2)
    pub bid2_price: Option<f64>,
    /// 买数量三 (bid volume level 3)
    pub bid3_volume: Option<f64>,
    /// 买价位三 (bid price level 3)
    pub bid3_price: Option<f64>,
    /// 买数量四 (bid volume level 4)
    pub bid4_volume: Option<f64>,
    /// 买价位四 (bid price level 4)
    pub bid4_price: Option<f64>,
    /// 买数量五 (bid volume level 5)
    pub bid5_volume: Option<f64>,
    /// 买价位五 (bid price level 5)
    pub bid5_price: Option<f64>,
    /// 卖数量一 (ask volume level 1)
    pub ask1_volume: Option<f64>,
    /// 卖价位一 (ask price level 1)
    pub ask1_price: Option<f64>,
    /// 卖数量二 (ask volume level 2)
    pub ask2_volume: Option<f64>,
    /// 卖价位二 (ask price level 2)
    pub ask2_price: Option<f64>,
    /// 卖数量三 (ask volume level 3)
    pub ask3_volume: Option<f64>,
    /// 卖价位三 (ask price level 3)
    pub ask3_price: Option<f64>,
    /// 卖数量四 (ask volume level 4)
    pub ask4_volume: Option<f64>,
    /// 卖价位四 (ask price level 4)
    pub ask4_price: Option<f64>,
    /// 卖数量五 (ask volume level 5)
    pub ask5_volume: Option<f64>,
    /// 卖价位五 (ask price level 5)
    pub ask5_price: Option<f64>,
    /// 行情日期 (quote date)
    pub date: Option<String>,
    /// 行情时间 (quote time)
    pub time: Option<String>,
    /// 停牌状态 (halt status)
    pub halt_status: Option<String>,
    pub source: &'static str,
}

/// Realtime quote for the underlying ETF of an SSE option
/// (akshare `option_sse_underlying_spot_price_sina`, `option_finance_sina.py:621`).
///
/// `symbol` is the underlying ticker, e.g. `"sh510050"` / `"sh510300"`.
pub async fn option_sse_underlying_spot_price_sina(
    client: &Client,
    symbol: &str,
) -> Result<Vec<SseUnderlyingSpotRow>> {
    let list = format!("https://hq.sinajs.cn/list={symbol}");
    let headers = [("Referer", SINA_REFERER_VIP)];
    let text = client
        .get_text(
            SOURCE_SINA,
            "option_sse_underlying_spot_price_sina",
            &list,
            &[],
            Some(&headers),
        )
        .await?;
    parse_sse_underlying_spot_price(&text)
}

pub(crate) fn parse_sse_underlying_spot_price(text: &str) -> Result<Vec<SseUnderlyingSpotRow>> {
    let f = sinajs_inner(text)?;
    if f.len() < 33 {
        return Err(Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: format!("underlying spot expected 33 fields, got {}", f.len()),
        });
    }
    Ok(vec![SseUnderlyingSpotRow {
        name: field_str(&f, 0),
        open: field_num(&f, 1),
        prev_close: field_num(&f, 2),
        last_price: field_num(&f, 3),
        high: field_num(&f, 4),
        low: field_num(&f, 5),
        bid_price: field_num(&f, 6),
        ask_price: field_num(&f, 7),
        volume: field_num(&f, 8),
        amount: field_num(&f, 9),
        bid1_volume: field_num(&f, 10),
        bid1_price: field_num(&f, 11),
        bid2_volume: field_num(&f, 12),
        bid2_price: field_num(&f, 13),
        bid3_volume: field_num(&f, 14),
        bid3_price: field_num(&f, 15),
        bid4_volume: field_num(&f, 16),
        bid4_price: field_num(&f, 17),
        bid5_volume: field_num(&f, 18),
        bid5_price: field_num(&f, 19),
        ask1_volume: field_num(&f, 20),
        ask1_price: field_num(&f, 21),
        ask2_volume: field_num(&f, 22),
        ask2_price: field_num(&f, 23),
        ask3_volume: field_num(&f, 24),
        ask3_price: field_num(&f, 25),
        ask4_volume: field_num(&f, 26),
        ask4_price: field_num(&f, 27),
        ask5_volume: field_num(&f, 28),
        ask5_price: field_num(&f, 29),
        date: field_str(&f, 30),
        time: field_str(&f, 31),
        halt_status: field_str(&f, 32),
        source: SOURCE_SINA,
    }])
}

// ---------------------------------------------------------------------------
// 6. option_sse_greeks_sina — greeks / contract basics
// ---------------------------------------------------------------------------

/// Option greeks and basic info for one SSE contract (Sina `CON_SO_`).
///
/// Mirrors akshare `option_sse_greeks_sina` (`option_finance_sina.py:686`), which
/// uses a special field mapping: `field_list` is zipped with
/// `[data[0]] + data[4:]` of the upstream comma-joined payload.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SseGreeksRow {
    /// 期权合约简称 (option contract short name)
    pub contract_name: Option<String>,
    /// 成交量 (volume)
    pub volume: Option<f64>,
    /// Delta
    pub delta: Option<f64>,
    /// Gamma
    pub gamma: Option<f64>,
    /// Theta
    pub theta: Option<f64>,
    /// Vega
    pub vega: Option<f64>,
    /// 隐含波动率 (implied volatility)
    pub implied_vol: Option<f64>,
    /// 最高价 (high)
    pub high: Option<f64>,
    /// 最低价 (low)
    pub low: Option<f64>,
    /// 交易代码 (trade code)
    pub trade_code: Option<String>,
    /// 行权价 (strike price)
    pub strike_price: Option<f64>,
    /// 最新价 (last price)
    pub last_price: Option<f64>,
    /// 理论价值 (theoretical value)
    pub theoretical_value: Option<f64>,
    pub source: &'static str,
}

/// Greeks and basic info for one SSE option contract
/// (akshare `option_sse_greeks_sina`, `option_finance_sina.py:686`).
///
/// `symbol` is the contract code (e.g. `"10003045"`); the upstream `CON_SO_`
/// feed has 16 comma-joined fields mapped with `[data[0]] + data[4:]`.
pub async fn option_sse_greeks_sina(client: &Client, symbol: &str) -> Result<Vec<SseGreeksRow>> {
    let list = format!("https://hq.sinajs.cn/list=CON_SO_{symbol}");
    let headers = [("Referer", SINA_REFERER_VIP)];
    let text = client
        .get_text(
            SOURCE_SINA,
            "option_sse_greeks_sina",
            &list,
            &[],
            Some(&headers),
        )
        .await?;
    parse_sse_greeks(&text)
}

pub(crate) fn parse_sse_greeks(text: &str) -> Result<Vec<SseGreeksRow>> {
    let f = sinajs_inner(text)?;
    if f.len() < 16 {
        return Err(Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: format!("greeks expected >=16 fields, got {}", f.len()),
        });
    }
    // akshare: list(zip(field_list, [data_list[0]] + data_list[4:]))
    Ok(vec![SseGreeksRow {
        contract_name: field_str(&f, 0),
        volume: field_num(&f, 4),
        delta: field_num(&f, 5),
        gamma: field_num(&f, 6),
        theta: field_num(&f, 7),
        vega: field_num(&f, 8),
        implied_vol: field_num(&f, 9),
        high: field_num(&f, 10),
        low: field_num(&f, 11),
        trade_code: field_str(&f, 12),
        strike_price: field_num(&f, 13),
        last_price: field_num(&f, 14),
        theoretical_value: field_num(&f, 15),
        source: SOURCE_SINA,
    }])
}

// ---------------------------------------------------------------------------
// 7. option_sse_minute_sina — current-session minute bars
// ---------------------------------------------------------------------------

/// A single intraday minute bar for an SSE option (Sina).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SseMinuteRow {
    /// 日期 (date)
    pub date: Option<String>,
    /// 时间 (time)
    pub time: Option<String>,
    /// 价格 (price)
    pub price: Option<f64>,
    /// 成交 (volume)
    pub volume: Option<f64>,
    /// 持仓 (open interest)
    pub open_interest: Option<f64>,
    /// 均价 (average price)
    pub avg_price: Option<f64>,
    pub source: &'static str,
}

/// Current trading-session minute bars for an SSE option
/// (akshare `option_sse_minute_sina`, `option_finance_sina.py:732`).
///
/// `symbol` is the contract code (e.g. `"10003720"`); only the current session
/// is available (no history).
pub async fn option_sse_minute_sina(client: &Client, symbol: &str) -> Result<Vec<SseMinuteRow>> {
    let url = "https://stock.finance.sina.com.cn/futures/api/openapi.php/StockOptionDaylineService.getOptionMinline";
    let sym = format!("CON_OP_{symbol}");
    let params = [("symbol", sym.as_str())];
    let headers = [("Referer", SINA_REFERER_QUOTES)];
    let v = client
        .get_json_with_headers(
            SOURCE_SINA,
            "option_sse_minute_sina",
            url,
            &params,
            Some(&headers),
        )
        .await?;
    parse_sse_minute(&v)
}

pub(crate) fn parse_sse_minute(resp: &Value) -> Result<Vec<SseMinuteRow>> {
    let arr = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "minute response missing result.data".into(),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(SseMinuteRow {
            date: item
                .get("日期")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string()),
            time: item
                .get("时间")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string()),
            price: item.get("价格").and_then(str_f64),
            volume: item.get("成交").and_then(str_f64),
            open_interest: item.get("持仓").and_then(str_f64),
            avg_price: item.get("均价").and_then(str_f64),
            source: SOURCE_SINA,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// 8. option_sse_daily_sina — historical daily OHLCV (JSONP)
// ---------------------------------------------------------------------------

/// A single daily OHLCV bar for an SSE option (Sina JSONP).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SseDailyRow {
    /// 日期 (date)
    pub date: Option<String>,
    /// 开盘 (open)
    pub open: Option<f64>,
    /// 最高 (high)
    pub high: Option<f64>,
    /// 最低 (low)
    pub low: Option<f64>,
    /// 收盘 (close)
    pub close: Option<f64>,
    /// 成交量 (volume)
    pub volume: Option<f64>,
    pub source: &'static str,
}

/// Full historical daily OHLCV for an SSE option contract
/// (akshare `option_sse_daily_sina`, `option_finance_sina.py:776`).
///
/// `symbol` is the contract code (e.g. `"10003889"`); the upstream
/// `jsonp_v2.php/...getSymbolInfo` wraps a `[[date,open,high,low,close,volume], …]`
/// array in parentheses.
pub async fn option_sse_daily_sina(client: &Client, symbol: &str) -> Result<Vec<SseDailyRow>> {
    let url = "https://stock.finance.sina.com.cn/futures/api/jsonp_v2.php//StockOptionDaylineService.getSymbolInfo";
    let sym = format!("CON_OP_{symbol}");
    let params = [("symbol", sym.as_str())];
    let headers = [("Referer", SINA_REFERER_QUOTES)];
    let text = client
        .get_text(
            SOURCE_SINA,
            "option_sse_daily_sina",
            url,
            &params,
            Some(&headers),
        )
        .await?;
    parse_sse_daily(&text)
}

pub(crate) fn parse_sse_daily(text: &str) -> Result<Vec<SseDailyRow>> {
    let v = strip_paren_jsonp(text, SOURCE_SINA)?;
    let arr = v.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "daily payload is not an array".into(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for row in arr {
        let r = row.as_array().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "daily row is not an array".into(),
        })?;
        out.push(SseDailyRow {
            date: r.first().and_then(|x| x.as_str()).map(|s| s.to_string()),
            open: r.get(1).and_then(vnum),
            high: r.get(2).and_then(vnum),
            low: r.get(3).and_then(vnum),
            close: r.get(4).and_then(vnum),
            volume: r.get(5).and_then(vnum),
            source: SOURCE_SINA,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// 9. option_finance_minute_sina — 5-day minute bars
// ---------------------------------------------------------------------------

/// A single intraday minute bar from the 5-day view of an SSE option (Sina).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SseFinanceMinuteRow {
    /// 日期 (date)
    pub date: Option<String>,
    /// 时间 (time)
    pub time: Option<String>,
    /// 价格 (price)
    pub price: Option<f64>,
    /// 均价 (average price)
    pub avg_price: Option<f64>,
    /// 成交量 (volume)
    pub volume: Option<f64>,
    pub source: &'static str,
}

/// Five-day minute bars for an SSE option contract
/// (akshare `option_finance_minute_sina`, `option_finance_sina.py:816`).
///
/// `symbol` is the contract code (e.g. `"10002530"`); the upstream
/// `getFiveDayLine` returns one object per day, each with array-valued
/// `time` / `price` / `volume` / `average_price` / `date` columns (plus a
/// dropped `_` column).
pub async fn option_finance_minute_sina(
    client: &Client,
    symbol: &str,
) -> Result<Vec<SseFinanceMinuteRow>> {
    let url = "https://stock.finance.sina.com.cn/futures/api/openapi.php/StockOptionDaylineService.getFiveDayLine";
    let sym = format!("CON_OP_{symbol}");
    let params = [("symbol", sym.as_str())];
    let headers = [("Referer", SINA_REFERER_QUOTES)];
    let v = client
        .get_json_with_headers(
            SOURCE_SINA,
            "option_finance_minute_sina",
            url,
            &params,
            Some(&headers),
        )
        .await?;
    parse_finance_minute(&v)
}

pub(crate) fn parse_finance_minute(resp: &Value) -> Result<Vec<SseFinanceMinuteRow>> {
    let arr = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "finance-minute response missing result.data".into(),
        })?;
    let mut out = Vec::new();
    for item in arr {
        let time = item.get("time").and_then(|x| x.as_array());
        let price = item.get("price").and_then(|x| x.as_array());
        let volume = item.get("volume").and_then(|x| x.as_array());
        let avg = item.get("average_price").and_then(|x| x.as_array());
        let date = item.get("date").and_then(|x| x.as_array());
        let n = time.map(|t| t.len()).unwrap_or(0);
        for i in 0..n {
            out.push(SseFinanceMinuteRow {
                date: date
                    .and_then(|a| a.get(i))
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string()),
                time: time
                    .and_then(|a| a.get(i))
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string()),
                price: price.and_then(|a| a.get(i)).and_then(vnum),
                avg_price: avg.and_then(|a| a.get(i)).and_then(vnum),
                volume: volume.and_then(|a| a.get(i)).and_then(vnum),
                source: SOURCE_SINA,
            });
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// 10. option_minute_em — Eastmoney intraday minute bars
// ---------------------------------------------------------------------------

/// A single intraday minute bar for an option from Eastmoney.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SseMinuteEmRow {
    /// 时间 (timestamp, e.g. `"2024-03-01 09:30"`)
    pub time: Option<String>,
    /// 收盘 (close)
    pub close: Option<f64>,
    /// 最高 (high)
    pub high: Option<f64>,
    /// 最低 (low)
    pub low: Option<f64>,
    /// 成交量 (volume)
    pub volume: Option<f64>,
    /// 成交额 (amount)
    pub amount: Option<f64>,
    pub source: &'static str,
}

/// Intraday minute bars for an option from Eastmoney
/// (akshare `option_minute_em`, `option_finance_sina.py:865`).
///
/// `secid` is the Eastmoney security id (e.g. `"1.510050"` for the underlying,
/// or `"1.10003720"` for a contract). Callers resolve it via
/// `option_current_em` / `option_current_cffex_em`; this port keeps the
/// dependency-free `secid` parameter rather than re-implementing that lookup.
pub async fn option_minute_em(client: &Client, secid: &str) -> Result<Vec<SseMinuteEmRow>> {
    let url = "https://push2.eastmoney.com/api/qt/stock/trends2/get";
    let params = [
        ("secid", secid),
        (
            "fields1",
            "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f11,f12,f13,f14,f17",
        ),
        ("fields2", "f51,f53,f54,f55,f56,f57,f58"),
        ("iscr", "0"),
        ("iscca", "0"),
        ("ut", "f057cbcbce2a86e2866ab8877db1d059"),
        ("ndays", "1"),
        ("cb", "quotepushdata1"),
    ];
    let text = client
        .get_text(SOURCE_EASTMONEY, "option_minute_em", url, &params, None)
        .await?;
    parse_minute_em(&text)
}

pub(crate) fn parse_minute_em(text: &str) -> Result<Vec<SseMinuteEmRow>> {
    let v = strip_paren_jsonp(text, SOURCE_EASTMONEY)?;
    let trends = v
        .get("data")
        .and_then(|d| d.get("trends"))
        .and_then(|t| t.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "minute_em response missing data.trends".into(),
        })?;
    let mut out = Vec::with_capacity(trends.len());
    for item in trends {
        let s = item.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "minute_em trend entry is not a string".into(),
        })?;
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() < 6 {
            continue;
        }
        out.push(SseMinuteEmRow {
            time: Some(parts[0].to_string()),
            close: parts[1].parse::<f64>().ok(),
            high: parts[2].parse::<f64>().ok(),
            low: parts[3].parse::<f64>().ok(),
            volume: parts[4].parse::<f64>().ok(),
            amount: parts[5].parse::<f64>().ok(),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Extract the comma-joined payload from a `hq.sinajs.cn` response of the form
/// `var hq_str_...="...";` — everything between the first and last `"` — and
/// split it into fields. Mirrors akshare's
/// `data_text[data_text.find('"') + 1 : data_text.rfind('"')].split(",")`.
fn sinajs_inner(text: &str) -> Result<Vec<String>> {
    let start = text.find('"').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "sinajs response missing opening quote".into(),
    })? + 1;
    let end = text.rfind('"').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "sinajs response missing closing quote".into(),
    })?;
    if end <= start {
        return Err(Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "sinajs response has no payload".into(),
        });
    }
    Ok(text[start..end].split(',').map(|s| s.to_string()).collect())
}

/// Strip the JSONP padding (`name(...)`) and parse the inner JSON.
fn strip_paren_jsonp(text: &str, origin: &'static str) -> Result<Value> {
    let start = text.find('(').ok_or_else(|| Error::UpstreamChanged {
        origin,
        message: "jsonp response missing '('".into(),
    })? + 1;
    let end = text.rfind(')').ok_or_else(|| Error::UpstreamChanged {
        origin,
        message: "jsonp response missing ')'".into(),
    })?;
    if end <= start {
        return Err(Error::UpstreamChanged {
            origin,
            message: "jsonp response has no payload".into(),
        });
    }
    serde_json::from_str(&text[start..end]).map_err(Error::Json)
}

/// Parse a numeric field from a `&Value` (number or numeric string).
fn vnum(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// Parse a numeric field held as a string (e.g. Sina openapi values).
fn str_f64(v: &Value) -> Option<f64> {
    v.as_str().and_then(|s| s.trim().parse::<f64>().ok())
}

/// Parse a numeric field from a comma-joined `Vec<String>` by index.
fn field_num(fields: &[String], i: usize) -> Option<f64> {
    fields.get(i).and_then(|s| {
        let t = s.trim();
        if t.is_empty() {
            None
        } else {
            t.parse::<f64>().ok()
        }
    })
}

/// Parse a string field from a comma-joined `Vec<String>` by index.
fn field_str(fields: &[String], i: usize) -> Option<String> {
    fields.get(i).and_then(|s| {
        let t = s.trim();
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    })
}

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

    fn fixture_text(name: &str) -> String {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        std::fs::read_to_string(p).unwrap()
    }

    #[test]
    fn parses_option_sse_list_sina() {
        let v = fixture("option_sse_list_sina.json");
        let rows = parse_sse_list(&v).unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].month, Some("202102".to_string()));
        assert_eq!(rows[3].month, Some("202105".to_string()));
    }

    #[test]
    fn parses_option_sse_expire_day_sina() {
        let v = fixture("option_sse_expire_day_sina.json");
        let rows = parse_sse_expire_day(&v).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].expire_day, Some("2021-02-24".to_string()));
        assert_eq!(rows[0].remainder_days, Some(12));
    }

    #[test]
    fn parses_option_sse_codes_sina() {
        let t = fixture_text("option_sse_codes_sina.txt");
        let rows = parse_sse_codes(&t).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].index, Some(1));
        assert_eq!(rows[0].code, Some("10003720".to_string()));
        assert_eq!(rows[2].code, Some("10003722".to_string()));
    }

    #[test]
    fn parses_option_sse_spot_price_sina() {
        let t = fixture_text("option_sse_spot_price_sina.txt");
        let rows = parse_sse_spot_price(&t).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].bid_volume, Some(10.0));
        assert_eq!(rows[0].last_price, Some(0.0510));
        // empty field -> None
        assert_eq!(rows[0].bid1_volume, None);
        assert_eq!(rows[0].quote_time, Some("2024/03/01 15:00:00".to_string()));
        assert_eq!(rows[0].contract_name, Some("50ETF购3月3500".to_string()));
        assert_eq!(rows[0].source, "sina");
    }

    #[test]
    fn parses_option_sse_underlying_spot_price_sina() {
        let t = fixture_text("option_sse_underlying_spot_price_sina.txt");
        let rows = parse_sse_underlying_spot_price(&t).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, Some("沪深300ETF".to_string()));
        assert_eq!(rows[0].last_price, Some(3.980));
        // empty fields -> None
        assert_eq!(rows[0].ask5_price, None);
        assert_eq!(rows[0].halt_status, None);
        assert_eq!(rows[0].date, Some("2024/03/01".to_string()));
    }

    #[test]
    fn parses_option_sse_greeks_sina() {
        let t = fixture_text("option_sse_greeks_sina.txt");
        let rows = parse_sse_greeks(&t).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].contract_name, Some("50ETF购3月3500".to_string()));
        assert_eq!(rows[0].volume, Some(12345.0));
        assert_eq!(rows[0].delta, Some(0.5123));
        assert_eq!(rows[0].gamma, Some(0.0234));
        assert_eq!(rows[0].theta, Some(-0.0123));
        assert_eq!(rows[0].vega, Some(0.0345));
        assert_eq!(rows[0].implied_vol, Some(0.1823));
        assert_eq!(rows[0].high, Some(0.0525));
        assert_eq!(rows[0].low, Some(0.0495));
        assert_eq!(rows[0].trade_code, Some("10003045".to_string()));
        assert_eq!(rows[0].strike_price, Some(3.500));
        assert_eq!(rows[0].last_price, Some(0.0510));
        assert_eq!(rows[0].theoretical_value, Some(0.0508));
        assert_eq!(rows[0].source, "sina");
    }

    #[test]
    fn parses_option_sse_minute_sina() {
        let v = fixture("option_sse_minute_sina.json");
        let rows = parse_sse_minute(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].time, Some("09:30".to_string()));
        assert_eq!(rows[0].price, Some(0.0501));
        assert_eq!(rows[0].date, Some("2024-03-01".to_string()));
        assert_eq!(rows[1].volume, Some(120.0));
    }

    #[test]
    fn parses_option_sse_daily_sina() {
        let t = fixture_text("option_sse_daily_sina.txt");
        let rows = parse_sse_daily(&t).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, Some("2024-03-01".to_string()));
        assert_eq!(rows[0].open, Some(0.0500));
        assert_eq!(rows[0].close, Some(0.0510));
        assert_eq!(rows[1].close, Some(0.0540));
        assert_eq!(rows[1].volume, Some(23456.0));
    }

    #[test]
    fn parses_option_finance_minute_sina() {
        let v = fixture("option_finance_minute_sina.json");
        let rows = parse_finance_minute(&v).unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].price, Some(0.0501));
        assert_eq!(rows[0].date, Some("2024-03-01".to_string()));
        assert_eq!(rows[3].price, Some(0.0492));
        assert_eq!(rows[3].date, Some("2024-03-04".to_string()));
    }

    #[test]
    fn parses_option_minute_em() {
        let t = fixture_text("option_minute_em.txt");
        let rows = parse_minute_em(&t).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].time, Some("2024-03-01 09:30".to_string()));
        assert_eq!(rows[0].close, Some(0.0510));
        assert_eq!(rows[0].high, Some(0.0520));
        assert_eq!(rows[0].low, Some(0.0500));
        assert_eq!(rows[0].volume, Some(12345.0));
        assert_eq!(rows[0].amount, Some(67890.0));
        assert_eq!(rows[0].source, "eastmoney");
    }
}
