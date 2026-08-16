//! 东方财富网-数据中心-大宗交易 (akshare `akshare/stock/stock_dzjy_em.py`).
//!
//! Every function here hits the Eastmoney `datacenter-web` JSON endpoint
//! (`https://datacenter-web.eastmoney.com/api/data/v1/get`) with a plain
//! `requests.get` — no JS signing, token, encryption, cookie or HTML scraping.
//! All six public functions in the akshare source are ported:
//!
//! | Rust fn                          | akshare fn                     | reportName                              | Paged |
//! |----------------------------------|--------------------------------|-----------------------------------------|-------|
//! | `stock_dzjy_sctj`                | `stock_dzjy_sctj`              | `PRT_BLOCKTRADE_MARKET_STA`             | yes   |
//! | `stock_dzjy_mrmx`                | `stock_dzjy_mrmx`              | `RPT_DATA_BLOCKTRADE`                   | no    |
//! | `stock_dzjy_mrtj`                | `stock_dzjy_mrtj`              | `RPT_BLOCKTRADE_STA`                    | no    |
//! | `stock_dzjy_hygtj`               | `stock_dzjy_hygtj`             | `RPT_BLOCKTRADE_ACSTA`                  | yes   |
//! | `stock_dzjy_hyyybtj`             | `stock_dzjy_hyyybtj`           | `RPT_BLOCKTRADE_OPERATEDEPTSTATISTICS`  | yes   |
//! | `stock_dzjy_yybph`               | `stock_dzjy_yybph`             | `RPT_BLOCKTRADE_OPERATEDEPT_RANK`       | yes   |
//!
//! ## DEFERRED
//!
//! None. All six public functions are pure-HTTP and ported above.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// Eastmoney source bucket, for rate limiting / error context.
const SOURCE_EASTMONEY: &str = "eastmoney";

/// Eastmoney `datacenter-web` data-center endpoint (shared by every fn here).
const BASE: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";

// ---------------------------------------------------------------------------
// Shared helpers (verbatim per porting brief)
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
// stock_dzjy_sctj — 大宗交易市场交易统计 (PRT_BLOCKTRADE_MARKET_STA)
// ===========================================================================

/// One bulk-trade market-statistic row, port of `stock_dzjy_sctj`
/// (Eastmoney `PRT_BLOCKTRADE_MARKET_STA`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockDzjySctjRow {
    /// `TRADE_DATE` 交易日期
    pub trade_date: String,
    /// `SZ_INDEX` 上证指数
    pub sz_index: Option<f64>,
    /// `SZ_CHANGE_RATE` 上证指数涨跌幅
    pub sz_change_rate: Option<f64>,
    /// `BLOCKTRADE_DEAL_AMT` 大宗交易成交总额
    pub blocktrade_deal_amt: Option<f64>,
    /// `PREMIUM_DEAL_AMT` 溢价成交总额
    pub premium_deal_amt: Option<f64>,
    /// `PREMIUM_RATIO` 溢价成交总额占比
    pub premium_ratio: Option<f64>,
    /// `DISCOUNT_DEAL_AMT` 折价成交总额
    pub discount_deal_amt: Option<f64>,
    /// `DISCOUNT_RATIO` 折价成交总额占比
    pub discount_ratio: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_dzjy_sctj()` — 大宗交易市场交易统计.
///
/// Returns the full market statistics time series across all pages.
pub async fn stock_dzjy_sctj(client: &Client) -> Result<Vec<StockDzjySctjRow>> {
    let params = [
        ("sortColumns", "TRADE_DATE"),
        ("sortTypes", "-1"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("reportName", "PRT_BLOCKTRADE_MARKET_STA"),
        (
            "columns",
            "TRADE_DATE,SZ_INDEX,SZ_CHANGE_RATE,BLOCKTRADE_DEAL_AMT,PREMIUM_DEAL_AMT,\
PREMIUM_RATIO,DISCOUNT_DEAL_AMT,DISCOUNT_RATIO",
        ),
        ("source", "WEB"),
        ("client", "WEB"),
    ];
    let items = paged(client, "stock_dzjy_sctj", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_dzjy_sctj(&synthetic)
}

/// Parse a datacenter `result.data` array into [`StockDzjySctjRow`]s.
pub(crate) fn parse_stock_dzjy_sctj(resp: &Value) -> Result<Vec<StockDzjySctjRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let trade_date = fstr(item, "TRADE_DATE").unwrap_or_default();
        if trade_date.is_empty() {
            continue;
        }
        out.push(StockDzjySctjRow {
            trade_date,
            sz_index: fnum(item, "SZ_INDEX"),
            sz_change_rate: fnum(item, "SZ_CHANGE_RATE"),
            blocktrade_deal_amt: fnum(item, "BLOCKTRADE_DEAL_AMT"),
            premium_deal_amt: fnum(item, "PREMIUM_DEAL_AMT"),
            premium_ratio: fnum(item, "PREMIUM_RATIO"),
            discount_deal_amt: fnum(item, "DISCOUNT_DEAL_AMT"),
            discount_ratio: fnum(item, "DISCOUNT_RATIO"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_dzjy_mrmx — 大宗交易每日明细 (RPT_DATA_BLOCKTRADE)
// ===========================================================================

/// One bulk-trade daily-detail row, port of `stock_dzjy_mrmx`
/// (Eastmoney `RPT_DATA_BLOCKTRADE`).
///
/// akshare relabels columns positionally and drops `SECUCODE` and — for
/// non-A-share symbols — `CHANGE_RATE`/`CLOSE_PRICE`. All real upstream columns
/// are kept here as `Option` so the data is preserved regardless of `symbol`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockDzjyMrmxRow {
    /// `TRADE_DATE` 交易日期
    pub trade_date: String,
    /// `SECURITY_CODE` 证券代码
    pub code: String,
    /// `SECUCODE` 证券全称代码 (e.g. `600000.SH`)
    pub secucode: Option<String>,
    /// `SECURITY_NAME_ABBR` 证券简称
    pub name: String,
    /// `CHANGE_RATE` 涨跌幅
    pub change_rate: Option<f64>,
    /// `CLOSE_PRICE` 收盘价
    pub close_price: Option<f64>,
    /// `DEAL_PRICE` 成交价
    pub deal_price: Option<f64>,
    /// `PREMIUM_RATIO` 折溢率
    pub premium_ratio: Option<f64>,
    /// `DEAL_VOLUME` 成交量
    pub deal_volume: Option<f64>,
    /// `DEAL_AMT` 成交额
    pub deal_amt: Option<f64>,
    /// `TURNOVER_RATE` 成交额/流通市值
    pub turnover_rate: Option<f64>,
    /// `BUYER_NAME` 买方营业部
    pub buyer_name: Option<String>,
    /// `SELLER_NAME` 卖方营业部
    pub seller_name: Option<String>,
    /// `CHANGE_RATE_1DAYS` 上榜后1日涨跌幅
    pub change_rate_1days: Option<f64>,
    /// `CHANGE_RATE_5DAYS` 上榜后5日涨跌幅
    pub change_rate_5days: Option<f64>,
    /// `CHANGE_RATE_10DAYS` 上榜后10日涨跌幅
    pub change_rate_10days: Option<f64>,
    /// `CHANGE_RATE_20DAYS` 上榜后20日涨跌幅
    pub change_rate_20days: Option<f64>,
    /// `BUYER_CODE` 买方营业部代码
    pub buyer_code: Option<String>,
    /// `SELLER_CODE` 卖方营业部代码
    pub seller_code: Option<String>,
    pub source: &'static str,
}

/// Map `symbol` ∈ {"A股","B股","基金","债券"} to the `SECURITY_TYPE_WEB` filter value.
fn security_type_web(symbol: &str) -> Result<&'static str> {
    match symbol {
        "A股" => Ok("1"),
        "B股" => Ok("2"),
        "基金" => Ok("3"),
        "债券" => Ok("4"),
        other => Err(Error::InvalidParam(format!(
            "symbol must be one of {{\"A股\", \"B股\", \"基金\", \"债券\"}}, got {other:?}"
        ))),
    }
}

/// Port of `stock_dzjy_mrmx(symbol, start_date, end_date)` — 大宗交易每日明细.
///
/// `symbol` defaults to `"基金"`; `start_date`/`end_date` are `YYYYMMDD`.
pub async fn stock_dzjy_mrmx(
    client: &Client,
    symbol: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<StockDzjyMrmxRow>> {
    check_date8(start_date, "stock_dzjy_mrmx start_date")?;
    check_date8(end_date, "stock_dzjy_mrmx end_date")?;
    let stype = security_type_web(symbol)?;
    let start = fmt_date8(start_date);
    let end = fmt_date8(end_date);
    let filter = format!("(SECURITY_TYPE_WEB={stype})(TRADE_DATE>='{start}')(TRADE_DATE<='{end}')");
    let params = [
        ("sortColumns", "SECURITY_CODE"),
        ("sortTypes", "1"),
        ("pageSize", "5000"),
        ("pageNumber", "1"),
        ("reportName", "RPT_DATA_BLOCKTRADE"),
        (
            "columns",
            "TRADE_DATE,SECURITY_CODE,SECUCODE,SECURITY_NAME_ABBR,CHANGE_RATE,CLOSE_PRICE,\
DEAL_PRICE,PREMIUM_RATIO,DEAL_VOLUME,DEAL_AMT,TURNOVER_RATE,BUYER_NAME,SELLER_NAME,\
CHANGE_RATE_1DAYS,CHANGE_RATE_5DAYS,CHANGE_RATE_10DAYS,CHANGE_RATE_20DAYS,BUYER_CODE,SELLER_CODE",
        ),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &filter),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_dzjy_mrmx", BASE, &params)
        .await?;
    parse_stock_dzjy_mrmx(&v)
}

/// Parse a datacenter `result.data` array into [`StockDzjyMrmxRow`]s.
pub(crate) fn parse_stock_dzjy_mrmx(resp: &Value) -> Result<Vec<StockDzjyMrmxRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let code = fstr(item, "SECURITY_CODE").unwrap_or_default();
        let name = fstr(item, "SECURITY_NAME_ABBR").unwrap_or_default();
        if code.is_empty() || name.is_empty() {
            continue;
        }
        out.push(StockDzjyMrmxRow {
            trade_date: fstr(item, "TRADE_DATE").unwrap_or_default(),
            code,
            name,
            secucode: fstr(item, "SECUCODE"),
            change_rate: fnum(item, "CHANGE_RATE"),
            close_price: fnum(item, "CLOSE_PRICE"),
            deal_price: fnum(item, "DEAL_PRICE"),
            premium_ratio: fnum(item, "PREMIUM_RATIO"),
            deal_volume: fnum(item, "DEAL_VOLUME"),
            deal_amt: fnum(item, "DEAL_AMT"),
            turnover_rate: fnum(item, "TURNOVER_RATE"),
            buyer_name: fstr(item, "BUYER_NAME"),
            seller_name: fstr(item, "SELLER_NAME"),
            change_rate_1days: fnum(item, "CHANGE_RATE_1DAYS"),
            change_rate_5days: fnum(item, "CHANGE_RATE_5DAYS"),
            change_rate_10days: fnum(item, "CHANGE_RATE_10DAYS"),
            change_rate_20days: fnum(item, "CHANGE_RATE_20DAYS"),
            buyer_code: fstr(item, "BUYER_CODE"),
            seller_code: fstr(item, "SELLER_CODE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_dzjy_mrtj — 大宗交易每日统计 (RPT_BLOCKTRADE_STA)
// ===========================================================================

/// One bulk-trade daily-statistic row, port of `stock_dzjy_mrtj`
/// (Eastmoney `RPT_BLOCKTRADE_STA`). `SECUCODE` is dropped by akshare.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockDzjyMrtjRow {
    /// `TRADE_DATE` 交易日期
    pub trade_date: String,
    /// `SECURITY_CODE` 证券代码
    pub code: String,
    /// `SECUCODE` 证券全称代码
    pub secucode: Option<String>,
    /// `SECURITY_NAME_ABBR` 证券简称
    pub name: String,
    /// `CHANGE_RATE` 涨跌幅
    pub change_rate: Option<f64>,
    /// `CLOSE_PRICE` 收盘价
    pub close_price: Option<f64>,
    /// `AVERAGE_PRICE` 成交价
    pub average_price: Option<f64>,
    /// `PREMIUM_RATIO` 折溢率
    pub premium_ratio: Option<f64>,
    /// `DEAL_NUM` 成交笔数
    pub deal_num: Option<f64>,
    /// `VOLUME` 成交总量
    pub volume: Option<f64>,
    /// `DEAL_AMT` 成交总额
    pub deal_amt: Option<f64>,
    /// `TURNOVERRATE` 成交总额/流通市值
    pub turnover_rate: Option<f64>,
    /// `D1_CLOSE_ADJCHRATE` 上榜后1日涨跌幅
    pub d1_close_adjchrate: Option<f64>,
    /// `D5_CLOSE_ADJCHRATE` 上榜后5日涨跌幅
    pub d5_close_adjchrate: Option<f64>,
    /// `D10_CLOSE_ADJCHRATE` 上榜后10日涨跌幅
    pub d10_close_adjchrate: Option<f64>,
    /// `D20_CLOSE_ADJCHRATE` 上榜后20日涨跌幅
    pub d20_close_adjchrate: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_dzjy_mrtj(start_date, end_date)` — 大宗交易每日统计.
///
/// `start_date`/`end_date` are `YYYYMMDD`.
pub async fn stock_dzjy_mrtj(
    client: &Client,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<StockDzjyMrtjRow>> {
    check_date8(start_date, "stock_dzjy_mrtj start_date")?;
    check_date8(end_date, "stock_dzjy_mrtj end_date")?;
    let start = fmt_date8(start_date);
    let end = fmt_date8(end_date);
    let filter = format!("(TRADE_DATE>='{start}')(TRADE_DATE<='{end}')");
    let params = [
        ("sortColumns", "TURNOVERRATE"),
        ("sortTypes", "-1"),
        ("pageSize", "5000"),
        ("pageNumber", "1"),
        ("reportName", "RPT_BLOCKTRADE_STA"),
        (
            "columns",
            "TRADE_DATE,SECURITY_CODE,SECUCODE,SECURITY_NAME_ABBR,CHANGE_RATE,CLOSE_PRICE,\
AVERAGE_PRICE,PREMIUM_RATIO,DEAL_NUM,VOLUME,DEAL_AMT,TURNOVERRATE,D1_CLOSE_ADJCHRATE,\
D5_CLOSE_ADJCHRATE,D10_CLOSE_ADJCHRATE,D20_CLOSE_ADJCHRATE",
        ),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &filter),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_dzjy_mrtj", BASE, &params)
        .await?;
    parse_stock_dzjy_mrtj(&v)
}

/// Parse a datacenter `result.data` array into [`StockDzjyMrtjRow`]s.
pub(crate) fn parse_stock_dzjy_mrtj(resp: &Value) -> Result<Vec<StockDzjyMrtjRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let code = fstr(item, "SECURITY_CODE").unwrap_or_default();
        let name = fstr(item, "SECURITY_NAME_ABBR").unwrap_or_default();
        if code.is_empty() || name.is_empty() {
            continue;
        }
        out.push(StockDzjyMrtjRow {
            trade_date: fstr(item, "TRADE_DATE").unwrap_or_default(),
            code,
            name,
            secucode: fstr(item, "SECUCODE"),
            change_rate: fnum(item, "CHANGE_RATE"),
            close_price: fnum(item, "CLOSE_PRICE"),
            average_price: fnum(item, "AVERAGE_PRICE"),
            premium_ratio: fnum(item, "PREMIUM_RATIO"),
            deal_num: fnum(item, "DEAL_NUM"),
            volume: fnum(item, "VOLUME"),
            deal_amt: fnum(item, "DEAL_AMT"),
            turnover_rate: fnum(item, "TURNOVERRATE"),
            d1_close_adjchrate: fnum(item, "D1_CLOSE_ADJCHRATE"),
            d5_close_adjchrate: fnum(item, "D5_CLOSE_ADJCHRATE"),
            d10_close_adjchrate: fnum(item, "D10_CLOSE_ADJCHRATE"),
            d20_close_adjchrate: fnum(item, "D20_CLOSE_ADJCHRATE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_dzjy_hygtj — 大宗交易行业成交统计 (RPT_BLOCKTRADE_ACSTA)
// ===========================================================================

/// One active-A-share bulk-trade statistic row, port of `stock_dzjy_hygtj`
/// (Eastmoney `RPT_BLOCKTRADE_ACSTA`). `SECUCODE`/`DATE_TYPE_CODE` dropped by akshare.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockDzjyHygtjRow {
    /// `SECURITY_CODE` 证券代码
    pub code: String,
    /// `SECUCODE` 证券全称代码
    pub secucode: Option<String>,
    /// `SECURITY_NAME_ABBR` 证券简称
    pub name: String,
    /// `CLOSE_PRICE` 最新价
    pub close_price: Option<f64>,
    /// `CHANGE_RATE` 涨跌幅
    pub change_rate: Option<f64>,
    /// `TRADE_DATE` 最近上榜日
    pub trade_date: String,
    /// `DEAL_AMT` 总成交额
    pub deal_amt: Option<f64>,
    /// `PREMIUM_RATIO` 折溢率
    pub premium_ratio: Option<f64>,
    /// `SUM_TURNOVERRATE` 成交总额/流通市值
    pub sum_turnover_rate: Option<f64>,
    /// `DEAL_NUM` 上榜次数-总计
    pub deal_num: Option<f64>,
    /// `PREMIUM_TIMES` 上榜次数-溢价
    pub premium_times: Option<f64>,
    /// `DISCOUNT_TIMES` 上榜次数-折价
    pub discount_times: Option<f64>,
    /// `D1_AVG_ADJCHRATE` 上榜日后平均涨跌幅-1日
    pub d1_avg_adjchrate: Option<f64>,
    /// `D5_AVG_ADJCHRATE` 上榜日后平均涨跌幅-5日
    pub d5_avg_adjchrate: Option<f64>,
    /// `D10_AVG_ADJCHRATE` 上榜日后平均涨跌幅-10日
    pub d10_avg_adjchrate: Option<f64>,
    /// `D20_AVG_ADJCHRATE` 上榜日后平均涨跌幅-20日
    pub d20_avg_adjchrate: Option<f64>,
    pub source: &'static str,
}

/// Map `symbol` ∈ {"近一月","近三月","近六月","近一年"} to the `DATE_TYPE_CODE`.
fn acsta_period(symbol: &str) -> Result<&'static str> {
    match symbol {
        "近一月" => Ok("1"),
        "近三月" => Ok("3"),
        "近六月" => Ok("6"),
        "近一年" => Ok("12"),
        other => Err(Error::InvalidParam(format!(
            "symbol must be one of {{\"近一月\", \"近三月\", \"近六月\", \"近一年\"}}, got {other:?}"
        ))),
    }
}

/// Port of `stock_dzjy_hygtj(symbol)` — 大宗交易行业成交统计.
///
/// `symbol` defaults to `"近三月"`.
pub async fn stock_dzjy_hygtj(client: &Client, symbol: &str) -> Result<Vec<StockDzjyHygtjRow>> {
    let period = acsta_period(symbol)?;
    let filter = format!("(DATE_TYPE_CODE={period})");
    let params = [
        ("sortColumns", "DEAL_NUM,SECURITY_CODE"),
        ("sortTypes", "-1,-1"),
        ("pageSize", "5000"),
        ("pageNumber", "1"),
        ("reportName", "RPT_BLOCKTRADE_ACSTA"),
        (
            "columns",
            "SECURITY_CODE,SECUCODE,SECURITY_NAME_ABBR,CLOSE_PRICE,CHANGE_RATE,TRADE_DATE,\
DEAL_AMT,PREMIUM_RATIO,SUM_TURNOVERRATE,DEAL_NUM,PREMIUM_TIMES,DISCOUNT_TIMES,\
D1_AVG_ADJCHRATE,D5_AVG_ADJCHRATE,D10_AVG_ADJCHRATE,D20_AVG_ADJCHRATE,DATE_TYPE_CODE",
        ),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &filter),
    ];
    let items = paged(client, "stock_dzjy_hygtj", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_dzjy_hygtj(&synthetic)
}

/// Parse a datacenter `result.data` array into [`StockDzjyHygtjRow`]s.
pub(crate) fn parse_stock_dzjy_hygtj(resp: &Value) -> Result<Vec<StockDzjyHygtjRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let code = fstr(item, "SECURITY_CODE").unwrap_or_default();
        let name = fstr(item, "SECURITY_NAME_ABBR").unwrap_or_default();
        if code.is_empty() || name.is_empty() {
            continue;
        }
        out.push(StockDzjyHygtjRow {
            code,
            name,
            secucode: fstr(item, "SECUCODE"),
            close_price: fnum(item, "CLOSE_PRICE"),
            change_rate: fnum(item, "CHANGE_RATE"),
            trade_date: fstr(item, "TRADE_DATE").unwrap_or_default(),
            deal_amt: fnum(item, "DEAL_AMT"),
            premium_ratio: fnum(item, "PREMIUM_RATIO"),
            sum_turnover_rate: fnum(item, "SUM_TURNOVERRATE"),
            deal_num: fnum(item, "DEAL_NUM"),
            premium_times: fnum(item, "PREMIUM_TIMES"),
            discount_times: fnum(item, "DISCOUNT_TIMES"),
            d1_avg_adjchrate: fnum(item, "D1_AVG_ADJCHRATE"),
            d5_avg_adjchrate: fnum(item, "D5_AVG_ADJCHRATE"),
            d10_avg_adjchrate: fnum(item, "D10_AVG_ADJCHRATE"),
            d20_avg_adjchrate: fnum(item, "D20_AVG_ADJCHRATE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_dzjy_hyyybtj — 大宗交易活跃营业部统计 (RPT_BLOCKTRADE_OPERATEDEPTSTATISTICS)
// ===========================================================================

/// One active-operate-dept bulk-trade statistic row, port of `stock_dzjy_hyyybtj`
/// (Eastmoney `RPT_BLOCKTRADE_OPERATEDEPTSTATISTICS`). `OPERATEDEPT_CODE`/
/// `N_DATE` dropped by akshare but kept here.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockDzjyHyyybtjRow {
    /// `OPERATEDEPT_NAME` 营业部名称
    pub dept_name: String,
    /// `OPERATEDEPT_CODE` 营业部代码
    pub dept_code: Option<String>,
    /// `ONLIST_DATE` 最近上榜日
    pub onlist_date: String,
    /// `STOCK_DETAILS` 买入的股票
    pub stock_details: Option<String>,
    /// `BUYER_NUM` 次数总计-买入
    pub buyer_num: Option<f64>,
    /// `SELLER_NUM` 次数总计-卖出
    pub seller_num: Option<f64>,
    /// `TOTAL_BUYAMT` 成交金额统计-买入
    pub total_buy_amt: Option<f64>,
    /// `TOTAL_SELLAMT` 成交金额统计-卖出
    pub total_sell_amt: Option<f64>,
    /// `TOTAL_NETAMT` 成交金额统计-净买入额
    pub total_net_amt: Option<f64>,
    pub source: &'static str,
}

/// Map `symbol` ∈ {"当前交易日","近3日","近5日","近10日","近30日"} to the `N_DATE=-N`.
fn dept_stat_period(symbol: &str) -> Result<&'static str> {
    match symbol {
        "当前交易日" => Ok("1"),
        "近3日" => Ok("3"),
        "近5日" => Ok("5"),
        "近10日" => Ok("10"),
        "近30日" => Ok("30"),
        other => Err(Error::InvalidParam(format!(
            "symbol must be one of {{\"当前交易日\", \"近3日\", \"近5日\", \"近10日\", \"近30日\"}}, got {other:?}"
        ))),
    }
}

/// Port of `stock_dzjy_hyyybtj(symbol)` — 大宗交易活跃营业部统计.
///
/// `symbol` defaults to `"近3日"`.
pub async fn stock_dzjy_hyyybtj(client: &Client, symbol: &str) -> Result<Vec<StockDzjyHyyybtjRow>> {
    let period = dept_stat_period(symbol)?;
    let filter = format!("(N_DATE=-{period})");
    let params = [
        ("sortColumns", "BUYER_NUM,TOTAL_BUYAMT"),
        ("sortTypes", "-1,-1"),
        ("pageSize", "5000"),
        ("pageNumber", "1"),
        ("reportName", "RPT_BLOCKTRADE_OPERATEDEPTSTATISTICS"),
        (
            "columns",
            "OPERATEDEPT_CODE,OPERATEDEPT_NAME,ONLIST_DATE,STOCK_DETAILS,\
BUYER_NUM,SELLER_NUM,TOTAL_BUYAMT,TOTAL_SELLAMT,TOTAL_NETAMT,N_DATE",
        ),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &filter),
    ];
    let items = paged(client, "stock_dzjy_hyyybtj", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_dzjy_hyyybtj(&synthetic)
}

/// Parse a datacenter `result.data` array into [`StockDzjyHyyybtjRow`]s.
pub(crate) fn parse_stock_dzjy_hyyybtj(resp: &Value) -> Result<Vec<StockDzjyHyyybtjRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let dept_name = fstr(item, "OPERATEDEPT_NAME").unwrap_or_default();
        if dept_name.is_empty() {
            continue;
        }
        out.push(StockDzjyHyyybtjRow {
            dept_name,
            dept_code: fstr(item, "OPERATEDEPT_CODE"),
            onlist_date: fstr(item, "ONLIST_DATE").unwrap_or_default(),
            stock_details: fstr(item, "STOCK_DETAILS"),
            buyer_num: fnum(item, "BUYER_NUM"),
            seller_num: fnum(item, "SELLER_NUM"),
            total_buy_amt: fnum(item, "TOTAL_BUYAMT"),
            total_sell_amt: fnum(item, "TOTAL_SELLAMT"),
            total_net_amt: fnum(item, "TOTAL_NETAMT"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_dzjy_yybph — 大宗交易营业部排行 (RPT_BLOCKTRADE_OPERATEDEPT_RANK)
// ===========================================================================

/// One operate-dept bulk-trade ranking row, port of `stock_dzjy_yybph`
/// (Eastmoney `RPT_BLOCKTRADE_OPERATEDEPT_RANK`). `OPERATEDEPT_CODE`/`N_DATE`/
/// `RELATED_ORG_CODE` dropped by akshare but kept here.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockDzjyYybphRow {
    /// `OPERATEDEPT_NAME` 营业部名称
    pub dept_name: String,
    /// `OPERATEDEPT_CODE` 营业部代码
    pub dept_code: Option<String>,
    /// `D1_BUYER_NUM` 上榜后1天-买入次数
    pub d1_buyer_num: Option<f64>,
    /// `D1_AVERAGE_INCREASE` 上榜后1天-平均涨幅
    pub d1_avg_increase: Option<f64>,
    /// `D1_RISE_PROBABILITY` 上榜后1天-上涨概率
    pub d1_rise_probability: Option<f64>,
    /// `D5_BUYER_NUM` 上榜后5天-买入次数
    pub d5_buyer_num: Option<f64>,
    /// `D5_AVERAGE_INCREASE` 上榜后5天-平均涨幅
    pub d5_avg_increase: Option<f64>,
    /// `D5_RISE_PROBABILITY` 上榜后5天-上涨概率
    pub d5_rise_probability: Option<f64>,
    /// `D10_BUYER_NUM` 上榜后10天-买入次数
    pub d10_buyer_num: Option<f64>,
    /// `D10_AVERAGE_INCREASE` 上榜后10天-平均涨幅
    pub d10_avg_increase: Option<f64>,
    /// `D10_RISE_PROBABILITY` 上榜后10天-上涨概率
    pub d10_rise_probability: Option<f64>,
    /// `D20_BUYER_NUM` 上榜后20天-买入次数
    pub d20_buyer_num: Option<f64>,
    /// `D20_AVERAGE_INCREASE` 上榜后20天-平均涨幅
    pub d20_avg_increase: Option<f64>,
    /// `D20_RISE_PROBABILITY` 上榜后20天-上涨概率
    pub d20_rise_probability: Option<f64>,
    pub source: &'static str,
}

/// Map `symbol` ∈ {"近一月","近三月","近六月","近一年"} to the `N_DATE=-N` rank window.
fn dept_rank_period(symbol: &str) -> Result<&'static str> {
    match symbol {
        "近一月" => Ok("30"),
        "近三月" => Ok("90"),
        "近六月" => Ok("180"),
        "近一年" => Ok("360"),
        other => Err(Error::InvalidParam(format!(
            "symbol must be one of {{\"近一月\", \"近三月\", \"近六月\", \"近一年\"}}, got {other:?}"
        ))),
    }
}

/// Port of `stock_dzjy_yybph(symbol)` — 大宗交易营业部排行.
///
/// `symbol` defaults to `"近三月"`.
pub async fn stock_dzjy_yybph(client: &Client, symbol: &str) -> Result<Vec<StockDzjyYybphRow>> {
    let period = dept_rank_period(symbol)?;
    let filter = format!("(N_DATE=-{period})");
    let params = [
        ("sortColumns", "D5_BUYER_NUM,D1_AVERAGE_INCREASE"),
        ("sortTypes", "-1,-1"),
        ("pageSize", "5000"),
        ("pageNumber", "1"),
        ("reportName", "RPT_BLOCKTRADE_OPERATEDEPT_RANK"),
        (
            "columns",
            "OPERATEDEPT_CODE,OPERATEDEPT_NAME,D1_BUYER_NUM,D1_AVERAGE_INCREASE,\
D1_RISE_PROBABILITY,D5_BUYER_NUM,D5_AVERAGE_INCREASE,D5_RISE_PROBABILITY,\
D10_BUYER_NUM,D10_AVERAGE_INCREASE,D10_RISE_PROBABILITY,D20_BUYER_NUM,\
D20_AVERAGE_INCREASE,D20_RISE_PROBABILITY,N_DATE,RELATED_ORG_CODE",
        ),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &filter),
    ];
    let items = paged(client, "stock_dzjy_yybph", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_dzjy_yybph(&synthetic)
}

/// Parse a datacenter `result.data` array into [`StockDzjyYybphRow`]s.
pub(crate) fn parse_stock_dzjy_yybph(resp: &Value) -> Result<Vec<StockDzjyYybphRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let dept_name = fstr(item, "OPERATEDEPT_NAME").unwrap_or_default();
        if dept_name.is_empty() {
            continue;
        }
        out.push(StockDzjyYybphRow {
            dept_name,
            dept_code: fstr(item, "OPERATEDEPT_CODE"),
            d1_buyer_num: fnum(item, "D1_BUYER_NUM"),
            d1_avg_increase: fnum(item, "D1_AVERAGE_INCREASE"),
            d1_rise_probability: fnum(item, "D1_RISE_PROBABILITY"),
            d5_buyer_num: fnum(item, "D5_BUYER_NUM"),
            d5_avg_increase: fnum(item, "D5_AVERAGE_INCREASE"),
            d5_rise_probability: fnum(item, "D5_RISE_PROBABILITY"),
            d10_buyer_num: fnum(item, "D10_BUYER_NUM"),
            d10_avg_increase: fnum(item, "D10_AVERAGE_INCREASE"),
            d10_rise_probability: fnum(item, "D10_RISE_PROBABILITY"),
            d20_buyer_num: fnum(item, "D20_BUYER_NUM"),
            d20_avg_increase: fnum(item, "D20_AVERAGE_INCREASE"),
            d20_rise_probability: fnum(item, "D20_RISE_PROBABILITY"),
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
    fn parses_stock_dzjy_sctj() {
        let rows = parse_stock_dzjy_sctj(&fixture("stock_dzjy_sctj.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].trade_date, "2023-04-03");
        assert_eq!(rows[0].sz_index, Some(3280.5));
        assert_eq!(rows[0].sz_change_rate, Some(-1.2));
        assert_eq!(rows[0].blocktrade_deal_amt, Some(580000000.0));
        assert_eq!(rows[0].premium_deal_amt, Some(120000000.0));
        assert_eq!(rows[0].premium_ratio, Some(0.2069));
        assert_eq!(rows[0].discount_deal_amt, Some(460000000.0));
        assert_eq!(rows[0].discount_ratio, Some(0.7931));
        assert_eq!(rows[0].source, "eastmoney");
        // nulls are preserved as None
        assert_eq!(rows[1].premium_deal_amt, None);
        assert_eq!(rows[1].premium_ratio, None);
    }

    #[test]
    fn parses_stock_dzjy_mrmx() {
        let rows = parse_stock_dzjy_mrmx(&fixture("stock_dzjy_mrmx.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].trade_date, "2022-01-04");
        assert_eq!(rows[0].code, "511990");
        assert_eq!(rows[0].secucode, Some("511990.SH".to_string()));
        assert_eq!(rows[0].name, "华宝添益");
        // 基金 rows have null 涨跌幅/收盘价/折溢率
        assert_eq!(rows[0].change_rate, None);
        assert_eq!(rows[0].close_price, None);
        assert_eq!(rows[0].premium_ratio, None);
        assert_eq!(rows[0].deal_price, Some(100.01));
        assert_eq!(rows[0].deal_volume, Some(500000.0));
        assert_eq!(rows[0].deal_amt, Some(50005000.0));
        assert_eq!(rows[0].buyer_name, Some("机构席位".to_string()));
        assert_eq!(rows[0].seller_name, Some("中信证券".to_string()));
        assert_eq!(rows[0].buyer_code, Some("B001".to_string()));
        assert_eq!(rows[0].seller_code, Some("S002".to_string()));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].code, "511880");
        assert_eq!(rows[1].name, "银华日利");
    }

    #[test]
    fn parses_stock_dzjy_mrtj() {
        let rows = parse_stock_dzjy_mrtj(&fixture("stock_dzjy_mrtj.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].trade_date, "2022-01-05");
        assert_eq!(rows[0].code, "600519");
        assert_eq!(rows[0].secucode, Some("600519.SH".to_string()));
        assert_eq!(rows[0].name, "贵州茅台");
        assert_eq!(rows[0].change_rate, Some(-2.0));
        assert_eq!(rows[0].close_price, Some(1800.0));
        assert_eq!(rows[0].average_price, Some(1750.0));
        assert_eq!(rows[0].premium_ratio, Some(-2.78));
        assert_eq!(rows[0].deal_num, Some(1.0));
        assert_eq!(rows[0].volume, Some(10000.0));
        assert_eq!(rows[0].deal_amt, Some(17500000.0));
        assert_eq!(rows[0].turnover_rate, Some(0.0008));
        assert_eq!(rows[0].d1_close_adjchrate, Some(1.5));
        assert_eq!(rows[0].d20_close_adjchrate, Some(8.1));
        assert_eq!(rows[0].source, "eastmoney");
        // null day-adjust fields
        assert_eq!(rows[1].d1_close_adjchrate, None);
        assert_eq!(rows[1].code, "000001");
    }

    #[test]
    fn parses_stock_dzjy_hygtj() {
        let rows = parse_stock_dzjy_hygtj(&fixture("stock_dzjy_hygtj.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "300750");
        assert_eq!(rows[0].secucode, Some("300750.SZ".to_string()));
        assert_eq!(rows[0].name, "宁德时代");
        assert_eq!(rows[0].close_price, Some(450.0));
        assert_eq!(rows[0].change_rate, Some(3.1));
        assert_eq!(rows[0].trade_date, "2023-04-03");
        assert_eq!(rows[0].deal_amt, Some(900000000.0));
        assert_eq!(rows[0].premium_ratio, Some(-1.2));
        assert_eq!(rows[0].sum_turnover_rate, Some(0.05));
        assert_eq!(rows[0].deal_num, Some(10.0));
        assert_eq!(rows[0].premium_times, Some(4.0));
        assert_eq!(rows[0].discount_times, Some(6.0));
        assert_eq!(rows[0].d1_avg_adjchrate, Some(1.2));
        assert_eq!(rows[0].d20_avg_adjchrate, Some(7.0));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].d1_avg_adjchrate, None);
        assert_eq!(rows[1].name, "招商银行");
    }

    #[test]
    fn parses_stock_dzjy_hyyybtj() {
        let rows = parse_stock_dzjy_hyyybtj(&fixture("stock_dzjy_hyyybtj.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].dept_name, "华泰证券深圳益田路");
        assert_eq!(rows[0].dept_code, Some("10188715".to_string()));
        assert_eq!(rows[0].onlist_date, "2023-04-03");
        assert_eq!(rows[0].stock_details, Some("600519,000001".to_string()));
        assert_eq!(rows[0].buyer_num, Some(8.0));
        assert_eq!(rows[0].seller_num, Some(3.0));
        assert_eq!(rows[0].total_buy_amt, Some(800000000.0));
        assert_eq!(rows[0].total_sell_amt, Some(300000000.0));
        assert_eq!(rows[0].total_net_amt, Some(500000000.0));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].total_net_amt, Some(-300000000.0));
    }

    #[test]
    fn parses_stock_dzjy_yybph() {
        let rows = parse_stock_dzjy_yybph(&fixture("stock_dzjy_yybph.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].dept_name, "华泰证券深圳益田路");
        assert_eq!(rows[0].dept_code, Some("10188715".to_string()));
        assert_eq!(rows[0].d1_buyer_num, Some(12.0));
        assert_eq!(rows[0].d1_avg_increase, Some(3.2));
        assert_eq!(rows[0].d1_rise_probability, Some(0.75));
        assert_eq!(rows[0].d5_buyer_num, Some(50.0));
        assert_eq!(rows[0].d5_avg_increase, Some(2.1));
        assert_eq!(rows[0].d10_buyer_num, Some(90.0));
        assert_eq!(rows[0].d20_rise_probability, Some(0.55));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].dept_name, "中信证券上海分公司");
        assert_eq!(rows[1].d1_avg_increase, Some(-0.5));
        assert_eq!(rows[1].d20_rise_probability, Some(0.25));
    }
}
