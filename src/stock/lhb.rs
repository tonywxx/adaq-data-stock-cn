//! 东方财富-数据中心-龙虎榜单 (dragon-tiger board), port of akshare
//! `stock_feature/stock_lhb_em.py`.
//!
//! Every function here hits the Eastmoney `datacenter-web` JSON endpoint
//! (`https://datacenter-web.eastmoney.com/api/data/v1/get`) with a plain
//! `requests.get` — no JS signing, token, encryption, cookie or HTML scraping.
//! All ten public functions in the akshare source are ported:
//!
//! | Rust fn                          | akshare fn                     | reportName                         | Paged |
//! |----------------------------------|--------------------------------|------------------------------------|-------|
//! | `stock_lhb_detail_em`            | `stock_lhb_detail_em`          | `RPT_DAILYBILLBOARD_DETAILSNEW`    | yes   |
//! | `stock_lhb_stock_statistic_em`   | `stock_lhb_stock_statistic_em` | `RPT_BILLBOARD_TRADEALL`           | no    |
//! | `stock_lhb_jgmmtj_em`            | `stock_lhb_jgmmtj_em`          | `RPT_ORGANIZATION_TRADE_DETAILS`   | yes   |
//! | `stock_lhb_jgstatistic_em`       | `stock_lhb_jgstatistic_em`     | `RPT_ORGANIZATION_SEATNEW`         | yes   |
//! | `stock_lhb_hyyyb_em`             | `stock_lhb_hyyyb_em`           | `RPT_OPERATEDEPT_ACTIVE`           | yes   |
//! | `stock_lhb_yybph_em`             | `stock_lhb_yybph_em`           | `RPT_RATEDEPT_RETURNT_RANKING`     | yes   |
//! | `stock_lhb_traderstatistic_em`   | `stock_lhb_traderstatistic_em` | `RPT_OPERATEDEPT_LIST_STATISTICS`  | yes   |
//! | `stock_lhb_stock_detail_date_em` | `stock_lhb_stock_detail_date_em` | `RPT_LHB_BOARDDATE`              | no    |
//! | `stock_lhb_stock_detail_em`      | `stock_lhb_stock_detail_em`    | `RPT_BILLBOARD_DAILYDETAILSBUY/SELL` | no  |
//! | `stock_lhb_yyb_detail_em`        | `stock_lhb_yyb_detail_em`      | `RPT_OPERATEDEPT_TRADE_DETAILSNEW` | yes   |
//!
//! ## DEFERRED
//!
//! None. All ten public functions are pure-HTTP and ported above.

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

#[allow(dead_code)]
fn inum(item: &Value, k: &str) -> Option<i64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse::<i64>().ok(),
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

/// Validate the `symbol` cycle parameter shared by the "近X月/年" fns.
fn statistic_cycle(symbol: &str) -> Result<&'static str> {
    match symbol {
        "近一月" => Ok("01"),
        "近三月" => Ok("02"),
        "近六月" => Ok("03"),
        "近一年" => Ok("04"),
        other => Err(Error::InvalidParam(format!(
            "symbol must be one of {{\"近一月\", \"近三月\", \"近六月\", \"近一年\"}}, got {other:?}"
        ))),
    }
}

/// Follow Eastmoney `result.pages` pagination, concatenating every `result.data`
/// page. Used by the fns whose akshare source loops over `total_page_num`.
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
// stock_lhb_detail_em — 龙虎榜详情
// ===========================================================================

/// One dragon-tiger board detail row, port of `stock_lhb_detail_em`.
///
/// Field ids are Eastmoney `RPT_DAILYBILLBOARD_DETAILSNEW` columns. `SECUCODE`
/// and `SECURITY_TYPE_CODE` are dropped (akshare renames them to `-`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockLhbDetailEmRow {
    /// `SECURITY_CODE` 代码
    pub code: String,
    /// `SECURITY_NAME_ABBR` 名称
    pub name: String,
    /// `TRADE_DATE` 上榜日
    pub trade_date: String,
    /// `EXPLAIN` 解读
    pub explain: Option<String>,
    /// `CLOSE_PRICE` 收盘价
    pub close_price: Option<f64>,
    /// `CHANGE_RATE` 涨跌幅
    pub change_rate: Option<f64>,
    /// `BILLBOARD_NET_AMT` 龙虎榜净买额
    pub billboard_net_amt: Option<f64>,
    /// `BILLBOARD_BUY_AMT` 龙虎榜买入额
    pub billboard_buy_amt: Option<f64>,
    /// `BILLBOARD_SELL_AMT` 龙虎榜卖出额
    pub billboard_sell_amt: Option<f64>,
    /// `BILLBOARD_DEAL_AMT` 龙虎榜成交额
    pub billboard_deal_amt: Option<f64>,
    /// `ACCUM_AMOUNT` 市场总成交额
    pub accum_amount: Option<f64>,
    /// `DEAL_NET_RATIO` 净买额占总成交比
    pub deal_net_ratio: Option<f64>,
    /// `DEAL_AMOUNT_RATIO` 成交额占总成交比
    pub deal_amount_ratio: Option<f64>,
    /// `TURNOVERRATE` 换手率
    pub turnover_rate: Option<f64>,
    /// `FREE_MARKET_CAP` 流通市值
    pub free_market_cap: Option<f64>,
    /// `EXPLANATION` 上榜原因
    pub explanation: Option<String>,
    /// `D1_CLOSE_ADJCHRATE` 上榜后1日
    pub d1_close_adjchrate: Option<f64>,
    /// `D2_CLOSE_ADJCHRATE` 上榜后2日
    pub d2_close_adjchrate: Option<f64>,
    /// `D5_CLOSE_ADJCHRATE` 上榜后5日
    pub d5_close_adjchrate: Option<f64>,
    /// `D10_CLOSE_ADJCHRATE` 上榜后10日
    pub d10_close_adjchrate: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_lhb_detail_em(start_date, end_date)`.
///
/// `start_date`/`end_date` are `YYYYMMDD`. Returns detail rows across all pages.
pub async fn stock_lhb_detail_em(
    client: &Client,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<StockLhbDetailEmRow>> {
    check_date8(start_date, "stock_lhb_detail_em start_date")?;
    check_date8(end_date, "stock_lhb_detail_em end_date")?;
    let start = fmt_date8(start_date);
    let end = fmt_date8(end_date);
    let filter = format!("(TRADE_DATE<='{end}')(TRADE_DATE>='{start}')");
    let params = [
        ("sortColumns", "SECURITY_CODE,TRADE_DATE"),
        ("sortTypes", "1,-1"),
        ("pageSize", "5000"),
        ("reportName", "RPT_DAILYBILLBOARD_DETAILSNEW"),
        (
            "columns",
            "SECURITY_CODE,SECUCODE,SECURITY_NAME_ABBR,TRADE_DATE,EXPLAIN,CLOSE_PRICE,\
CHANGE_RATE,BILLBOARD_NET_AMT,BILLBOARD_BUY_AMT,BILLBOARD_SELL_AMT,BILLBOARD_DEAL_AMT,\
ACCUM_AMOUNT,DEAL_NET_RATIO,DEAL_AMOUNT_RATIO,TURNOVERRATE,FREE_MARKET_CAP,EXPLANATION,\
D1_CLOSE_ADJCHRATE,D2_CLOSE_ADJCHRATE,D5_CLOSE_ADJCHRATE,D10_CLOSE_ADJCHRATE,SECURITY_TYPE_CODE",
        ),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &filter),
    ];
    let items = paged(client, "stock_lhb_detail_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_lhb_detail_em(&synthetic)
}

/// Parse a datacenter `result.data` array into [`StockLhbDetailEmRow`]s.
pub(crate) fn parse_stock_lhb_detail_em(resp: &Value) -> Result<Vec<StockLhbDetailEmRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let code = fstr(item, "SECURITY_CODE").unwrap_or_default();
        let name = fstr(item, "SECURITY_NAME_ABBR").unwrap_or_default();
        if code.is_empty() || name.is_empty() {
            continue;
        }
        out.push(StockLhbDetailEmRow {
            code,
            name,
            trade_date: fstr(item, "TRADE_DATE").unwrap_or_default(),
            explain: fstr(item, "EXPLAIN"),
            close_price: fnum(item, "CLOSE_PRICE"),
            change_rate: fnum(item, "CHANGE_RATE"),
            billboard_net_amt: fnum(item, "BILLBOARD_NET_AMT"),
            billboard_buy_amt: fnum(item, "BILLBOARD_BUY_AMT"),
            billboard_sell_amt: fnum(item, "BILLBOARD_SELL_AMT"),
            billboard_deal_amt: fnum(item, "BILLBOARD_DEAL_AMT"),
            accum_amount: fnum(item, "ACCUM_AMOUNT"),
            deal_net_ratio: fnum(item, "DEAL_NET_RATIO"),
            deal_amount_ratio: fnum(item, "DEAL_AMOUNT_RATIO"),
            turnover_rate: fnum(item, "TURNOVERRATE"),
            free_market_cap: fnum(item, "FREE_MARKET_CAP"),
            explanation: fstr(item, "EXPLANATION"),
            d1_close_adjchrate: fnum(item, "D1_CLOSE_ADJCHRATE"),
            d2_close_adjchrate: fnum(item, "D2_CLOSE_ADJCHRATE"),
            d5_close_adjchrate: fnum(item, "D5_CLOSE_ADJCHRATE"),
            d10_close_adjchrate: fnum(item, "D10_CLOSE_ADJCHRATE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_lhb_stock_statistic_em — 个股上榜统计
// ===========================================================================

/// One per-stock listing-statistic row, port of `stock_lhb_stock_statistic_em`.
///
/// `columns=ALL`; the field ids below are inferred from the akshare positional
/// rename and the `sortColumns` (`BILLBOARD_TIMES,LATEST_TDATE,SECURITY_CODE`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockLhbStockStatisticRow {
    /// `SECURITY_CODE` 代码
    pub code: String,
    /// `SECURITY_NAME_ABBR` 名称
    pub name: String,
    /// `LATEST_TDATE` 最近上榜日
    pub latest_trade_date: String,
    /// `CLOSE_PRICE` 收盘价
    pub close_price: Option<f64>,
    /// `CHANGE_RATE` 涨跌幅
    pub change_rate: Option<f64>,
    /// `BILLBOARD_TIMES` 上榜次数
    pub billboard_times: Option<f64>,
    /// `BILLBOARD_NET_AMT` 龙虎榜净买额
    pub billboard_net_amt: Option<f64>,
    /// `BILLBOARD_BUY_AMT` 龙虎榜买入额
    pub billboard_buy_amt: Option<f64>,
    /// `BILLBOARD_SELL_AMT` 龙虎榜卖出额
    pub billboard_sell_amt: Option<f64>,
    /// `BILLBOARD_DEAL_AMT` 龙虎榜总成交额
    pub billboard_deal_amt: Option<f64>,
    /// `BUY_ORG_TIMES` 买方机构次数
    pub buy_org_times: Option<f64>,
    /// `SELL_ORG_TIMES` 卖方机构次数
    pub sell_org_times: Option<f64>,
    /// `INST_BUY_NET_AMT` 机构买入净额
    pub inst_buy_net_amt: Option<f64>,
    /// `INST_BUY_AMT` 机构买入总额
    pub inst_buy_amt: Option<f64>,
    /// `INST_SELL_AMT` 机构卖出总额
    pub inst_sell_amt: Option<f64>,
    /// `M1_CLOSE_ADJCHRATE` 近1个月涨跌幅
    pub m1_close_adjchrate: Option<f64>,
    /// `M3_CLOSE_ADJCHRATE` 近3个月涨跌幅
    pub m3_close_adjchrate: Option<f64>,
    /// `M6_CLOSE_ADJCHRATE` 近6个月涨跌幅
    pub m6_close_adjchrate: Option<f64>,
    /// `Y1_CLOSE_ADJCHRATE` 近1年涨跌幅
    pub y1_close_adjchrate: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_lhb_stock_statistic_em(symbol)`.
///
/// `symbol` ∈ {"近一月", "近三月", "近六月", "近一年"}; mapped to the
/// `STATISTICS_CYCLE` filter value.
pub async fn stock_lhb_stock_statistic_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<StockLhbStockStatisticRow>> {
    let cycle = statistic_cycle(symbol)?;
    let filter = format!("(STATISTICS_CYCLE=\"{cycle}\")");
    let params = [
        ("sortColumns", "BILLBOARD_TIMES,LATEST_TDATE,SECURITY_CODE"),
        ("sortTypes", "-1,-1,1"),
        ("pageSize", "5000"),
        ("pageNumber", "1"),
        ("reportName", "RPT_BILLBOARD_TRADEALL"),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &filter),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_lhb_stock_statistic_em", BASE, &params)
        .await?;
    parse_stock_lhb_stock_statistic_em(&v)
}

/// Parse a datacenter `result.data` array into [`StockLhbStockStatisticRow`]s.
pub(crate) fn parse_stock_lhb_stock_statistic_em(
    resp: &Value,
) -> Result<Vec<StockLhbStockStatisticRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let code = fstr(item, "SECURITY_CODE").unwrap_or_default();
        let name = fstr(item, "SECURITY_NAME_ABBR").unwrap_or_default();
        if code.is_empty() || name.is_empty() {
            continue;
        }
        out.push(StockLhbStockStatisticRow {
            code,
            name,
            latest_trade_date: fstr(item, "LATEST_TDATE").unwrap_or_default(),
            close_price: fnum(item, "CLOSE_PRICE"),
            change_rate: fnum(item, "CHANGE_RATE"),
            billboard_times: fnum(item, "BILLBOARD_TIMES"),
            billboard_net_amt: fnum(item, "BILLBOARD_NET_AMT"),
            billboard_buy_amt: fnum(item, "BILLBOARD_BUY_AMT"),
            billboard_sell_amt: fnum(item, "BILLBOARD_SELL_AMT"),
            billboard_deal_amt: fnum(item, "BILLBOARD_DEAL_AMT"),
            buy_org_times: fnum(item, "BUY_ORG_TIMES"),
            sell_org_times: fnum(item, "SELL_ORG_TIMES"),
            inst_buy_net_amt: fnum(item, "INST_BUY_NET_AMT"),
            inst_buy_amt: fnum(item, "INST_BUY_AMT"),
            inst_sell_amt: fnum(item, "INST_SELL_AMT"),
            m1_close_adjchrate: fnum(item, "M1_CLOSE_ADJCHRATE"),
            m3_close_adjchrate: fnum(item, "M3_CLOSE_ADJCHRATE"),
            m6_close_adjchrate: fnum(item, "M6_CLOSE_ADJCHRATE"),
            y1_close_adjchrate: fnum(item, "Y1_CLOSE_ADJCHRATE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_lhb_jgmmtj_em — 机构买卖每日统计
// ===========================================================================

/// One institution-buy/sell daily row, port of `stock_lhb_jgmmtj_em`.
///
/// `columns=ALL`; field ids inferred from the akshare positional rename and
/// `sortColumns` (`NET_BUY_AMT,TRADE_DATE,SECURITY_CODE`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockLhbJgmmtjRow {
    /// `SECURITY_NAME_ABBR` 名称
    pub name: String,
    /// `SECURITY_CODE` 代码
    pub code: String,
    /// `TRADE_DATE` 上榜日期
    pub trade_date: String,
    /// `CLOSE_PRICE` 收盘价
    pub close_price: Option<f64>,
    /// `CHANGE_RATE` 涨跌幅
    pub change_rate: Option<f64>,
    /// `BUY_ORG_NUM` 买方机构数
    pub buy_org_num: Option<f64>,
    /// `SELL_ORG_NUM` 卖方机构数
    pub sell_org_num: Option<f64>,
    /// `ORG_BUY_AMT` 机构买入总额
    pub org_buy_amt: Option<f64>,
    /// `ORG_SELL_AMT` 机构卖出总额
    pub org_sell_amt: Option<f64>,
    /// `ORG_NET_BUY_AMT` 机构买入净额
    pub org_net_buy_amt: Option<f64>,
    /// `ACCUM_AMOUNT` 市场总成交额
    pub accum_amount: Option<f64>,
    /// `ORG_NET_BUY_RATIO` 机构净买额占总成交额比
    pub org_net_buy_ratio: Option<f64>,
    /// `TURNOVERRATE` 换手率
    pub turnover_rate: Option<f64>,
    /// `FREE_MARKET_CAP` 流通市值
    pub free_market_cap: Option<f64>,
    /// `EXPLANATION` 上榜原因
    pub explanation: Option<String>,
    pub source: &'static str,
}

/// Port of `stock_lhb_jgmmtj_em(start_date, end_date)`.
pub async fn stock_lhb_jgmmtj_em(
    client: &Client,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<StockLhbJgmmtjRow>> {
    check_date8(start_date, "stock_lhb_jgmmtj_em start_date")?;
    check_date8(end_date, "stock_lhb_jgmmtj_em end_date")?;
    let start = fmt_date8(start_date);
    let end = fmt_date8(end_date);
    let filter = format!("(TRADE_DATE>='{start}')(TRADE_DATE<='{end}')");
    let params = [
        ("sortColumns", "NET_BUY_AMT,TRADE_DATE,SECURITY_CODE"),
        ("sortTypes", "-1,-1,1"),
        ("pageSize", "500"),
        ("reportName", "RPT_ORGANIZATION_TRADE_DETAILS"),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &filter),
    ];
    let items = paged(client, "stock_lhb_jgmmtj_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_lhb_jgmmtj_em(&synthetic)
}

/// Parse a datacenter `result.data` array into [`StockLhbJgmmtjRow`]s.
pub(crate) fn parse_stock_lhb_jgmmtj_em(resp: &Value) -> Result<Vec<StockLhbJgmmtjRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let code = fstr(item, "SECURITY_CODE").unwrap_or_default();
        let name = fstr(item, "SECURITY_NAME_ABBR").unwrap_or_default();
        if code.is_empty() || name.is_empty() {
            continue;
        }
        out.push(StockLhbJgmmtjRow {
            name,
            code,
            trade_date: fstr(item, "TRADE_DATE").unwrap_or_default(),
            close_price: fnum(item, "CLOSE_PRICE"),
            change_rate: fnum(item, "CHANGE_RATE"),
            buy_org_num: fnum(item, "BUY_ORG_NUM"),
            sell_org_num: fnum(item, "SELL_ORG_NUM"),
            org_buy_amt: fnum(item, "ORG_BUY_AMT"),
            org_sell_amt: fnum(item, "ORG_SELL_AMT"),
            org_net_buy_amt: fnum(item, "ORG_NET_BUY_AMT"),
            accum_amount: fnum(item, "ACCUM_AMOUNT"),
            org_net_buy_ratio: fnum(item, "ORG_NET_BUY_RATIO"),
            turnover_rate: fnum(item, "TURNOVERRATE"),
            free_market_cap: fnum(item, "FREE_MARKET_CAP"),
            explanation: fstr(item, "EXPLANATION"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_lhb_jgstatistic_em — 机构席位追踪
// ===========================================================================

/// One institution-seat tracking row, port of `stock_lhb_jgstatistic_em`.
///
/// Field ids are the Eastmoney `RPT_ORGANIZATION_SEATNEW` columns (akshare
/// renames these by real name, so ids are known).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockLhbJgstatisticRow {
    /// `SECURITY_CODE` 代码
    pub code: String,
    /// `SECURITY_NAME_ABBR` 名称
    pub name: String,
    /// `CLOSE_PRICE` 收盘价
    pub close_price: Option<f64>,
    /// `CHANGE_RATE` 涨跌幅
    pub change_rate: Option<f64>,
    /// `AMOUNT` 龙虎榜成交金额
    pub amount: Option<f64>,
    /// `ONLIST_TIMES` 上榜次数
    pub onlist_times: Option<f64>,
    /// `BUY_AMT` 机构买入额
    pub buy_amt: Option<f64>,
    /// `BUY_TIMES` 机构买入次数
    pub buy_times: Option<f64>,
    /// `SELL_AMT` 机构卖出额
    pub sell_amt: Option<f64>,
    /// `SELL_TIMES` 机构卖出次数
    pub sell_times: Option<f64>,
    /// `NET_BUY_AMT` 机构净买额
    pub net_buy_amt: Option<f64>,
    /// `M1_CLOSE_ADJCHRATE` 近1个月涨跌幅
    pub m1_close_adjchrate: Option<f64>,
    /// `M3_CLOSE_ADJCHRATE` 近3个月涨跌幅
    pub m3_close_adjchrate: Option<f64>,
    /// `M6_CLOSE_ADJCHRATE` 近6个月涨跌幅
    pub m6_close_adjchrate: Option<f64>,
    /// `Y1_CLOSE_ADJCHRATE` 近1年涨跌幅
    pub y1_close_adjchrate: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_lhb_jgstatistic_em(symbol)`.
pub async fn stock_lhb_jgstatistic_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<StockLhbJgstatisticRow>> {
    let cycle = statistic_cycle(symbol)?;
    let filter = format!("(STATISTICSCYCLE=\"{cycle}\")");
    let params = [
        ("sortColumns", "ONLIST_TIMES,SECURITY_CODE"),
        ("sortTypes", "-1,1"),
        ("pageSize", "5000"),
        ("reportName", "RPT_ORGANIZATION_SEATNEW"),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &filter),
    ];
    let items = paged(client, "stock_lhb_jgstatistic_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_lhb_jgstatistic_em(&synthetic)
}

/// Parse a datacenter `result.data` array into [`StockLhbJgstatisticRow`]s.
pub(crate) fn parse_stock_lhb_jgstatistic_em(
    resp: &Value,
) -> Result<Vec<StockLhbJgstatisticRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let code = fstr(item, "SECURITY_CODE").unwrap_or_default();
        let name = fstr(item, "SECURITY_NAME_ABBR").unwrap_or_default();
        if code.is_empty() || name.is_empty() {
            continue;
        }
        out.push(StockLhbJgstatisticRow {
            code,
            name,
            close_price: fnum(item, "CLOSE_PRICE"),
            change_rate: fnum(item, "CHANGE_RATE"),
            amount: fnum(item, "AMOUNT"),
            onlist_times: fnum(item, "ONLIST_TIMES"),
            buy_amt: fnum(item, "BUY_AMT"),
            buy_times: fnum(item, "BUY_TIMES"),
            sell_amt: fnum(item, "SELL_AMT"),
            sell_times: fnum(item, "SELL_TIMES"),
            net_buy_amt: fnum(item, "NET_BUY_AMT"),
            m1_close_adjchrate: fnum(item, "M1_CLOSE_ADJCHRATE"),
            m3_close_adjchrate: fnum(item, "M3_CLOSE_ADJCHRATE"),
            m6_close_adjchrate: fnum(item, "M6_CLOSE_ADJCHRATE"),
            y1_close_adjchrate: fnum(item, "Y1_CLOSE_ADJCHRATE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_lhb_hyyyb_em — 每日活跃营业部
// ===========================================================================

/// One active sales-department daily row, port of `stock_lhb_hyyyb_em`.
///
/// `columns=ALL`; field ids inferred from the akshare positional rename and
/// `sortColumns` (`TOTAL_NETAMT,ONLIST_DATE,OPERATEDEPT_CODE`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockLhbHyyybRow {
    /// `OPERATEDEPT_NAME` 营业部名称
    pub dept_name: String,
    /// `ONLIST_DATE` 上榜日
    pub onlist_date: String,
    /// `BUY_STOCK_NUM` 买入个股数
    pub buy_stock_num: Option<f64>,
    /// `SELL_STOCK_NUM` 卖出个股数
    pub sell_stock_num: Option<f64>,
    /// `BUY_TOTAL_AMT` 买入总金额
    pub buy_total_amt: Option<f64>,
    /// `SELL_TOTAL_AMT` 卖出总金额
    pub sell_total_amt: Option<f64>,
    /// `TOTAL_NETAMT` 总买卖净额
    pub total_net_amt: Option<f64>,
    /// `OPERATEDEPT_CODE` 营业部代码
    pub dept_code: String,
    /// `BUY_STOCKS` 买入股票
    pub buy_stocks: Option<String>,
    pub source: &'static str,
}

/// Port of `stock_lhb_hyyyb_em(start_date, end_date)`.
pub async fn stock_lhb_hyyyb_em(
    client: &Client,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<StockLhbHyyybRow>> {
    check_date8(start_date, "stock_lhb_hyyyb_em start_date")?;
    check_date8(end_date, "stock_lhb_hyyyb_em end_date")?;
    let start = fmt_date8(start_date);
    let end = fmt_date8(end_date);
    let filter = format!("(ONLIST_DATE>='{start}')(ONLIST_DATE<='{end}')");
    let params = [
        ("sortColumns", "TOTAL_NETAMT,ONLIST_DATE,OPERATEDEPT_CODE"),
        ("sortTypes", "-1,-1,1"),
        ("pageSize", "5000"),
        ("reportName", "RPT_OPERATEDEPT_ACTIVE"),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &filter),
    ];
    let items = paged(client, "stock_lhb_hyyyb_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_lhb_hyyyb_em(&synthetic)
}

/// Parse a datacenter `result.data` array into [`StockLhbHyyybRow`]s.
pub(crate) fn parse_stock_lhb_hyyyb_em(resp: &Value) -> Result<Vec<StockLhbHyyybRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let dept_name = fstr(item, "OPERATEDEPT_NAME").unwrap_or_default();
        let dept_code = fstr(item, "OPERATEDEPT_CODE").unwrap_or_default();
        if dept_name.is_empty() || dept_code.is_empty() {
            continue;
        }
        out.push(StockLhbHyyybRow {
            dept_name,
            onlist_date: fstr(item, "ONLIST_DATE").unwrap_or_default(),
            buy_stock_num: fnum(item, "BUY_STOCK_NUM"),
            sell_stock_num: fnum(item, "SELL_STOCK_NUM"),
            buy_total_amt: fnum(item, "BUY_TOTAL_AMT"),
            sell_total_amt: fnum(item, "SELL_TOTAL_AMT"),
            total_net_amt: fnum(item, "TOTAL_NETAMT"),
            dept_code,
            buy_stocks: fstr(item, "BUY_STOCKS"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_lhb_yybph_em — 营业部排行
// ===========================================================================

/// One sales-department ranking row, port of `stock_lhb_yybph_em`.
///
/// Field ids are the Eastmoney `RPT_RATEDEPT_RETURNT_RANKING` columns.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockLhbYybphRow {
    /// `OPERATEDEPT_NAME` 营业部名称
    pub dept_name: String,
    /// `TOTAL_BUYER_SALESTIMES_1DAY` 上榜后1天-买入次数
    pub b1_times: Option<f64>,
    /// `AVERAGE_INCREASE_1DAY` 上榜后1天-平均涨幅
    pub b1_avg_increase: Option<f64>,
    /// `RISE_PROBABILITY_1DAY` 上榜后1天-上涨概率
    pub b1_rise_probability: Option<f64>,
    /// `TOTAL_BUYER_SALESTIMES_2DAY` 上榜后2天-买入次数
    pub b2_times: Option<f64>,
    /// `AVERAGE_INCREASE_2DAY` 上榜后2天-平均涨幅
    pub b2_avg_increase: Option<f64>,
    /// `RISE_PROBABILITY_2DAY` 上榜后2天-上涨概率
    pub b2_rise_probability: Option<f64>,
    /// `TOTAL_BUYER_SALESTIMES_3DAY` 上榜后3天-买入次数
    pub b3_times: Option<f64>,
    /// `AVERAGE_INCREASE_3DAY` 上榜后3天-平均涨幅
    pub b3_avg_increase: Option<f64>,
    /// `RISE_PROBABILITY_3DAY` 上榜后3天-上涨概率
    pub b3_rise_probability: Option<f64>,
    /// `TOTAL_BUYER_SALESTIMES_5DAY` 上榜后5天-买入次数
    pub b5_times: Option<f64>,
    /// `AVERAGE_INCREASE_5DAY` 上榜后5天-平均涨幅
    pub b5_avg_increase: Option<f64>,
    /// `RISE_PROBABILITY_5DAY` 上榜后5天-上涨概率
    pub b5_rise_probability: Option<f64>,
    /// `TOTAL_BUYER_SALESTIMES_10DAY` 上榜后10天-买入次数
    pub b10_times: Option<f64>,
    /// `AVERAGE_INCREASE_10DAY` 上榜后10天-平均涨幅
    pub b10_avg_increase: Option<f64>,
    /// `RISE_PROBABILITY_10DAY` 上榜后10天-上涨概率
    pub b10_rise_probability: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_lhb_yybph_em(symbol)`.
pub async fn stock_lhb_yybph_em(client: &Client, symbol: &str) -> Result<Vec<StockLhbYybphRow>> {
    let cycle = statistic_cycle(symbol)?;
    let filter = format!("(STATISTICSCYCLE=\"{cycle}\")");
    let params = [
        ("sortColumns", "TOTAL_BUYER_SALESTIMES_1DAY,OPERATEDEPT_CODE"),
        ("sortTypes", "-1,1"),
        ("pageSize", "5000"),
        ("reportName", "RPT_RATEDEPT_RETURNT_RANKING"),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &filter),
    ];
    let items = paged(client, "stock_lhb_yybph_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_lhb_yybph_em(&synthetic)
}

/// Parse a datacenter `result.data` array into [`StockLhbYybphRow`]s.
pub(crate) fn parse_stock_lhb_yybph_em(resp: &Value) -> Result<Vec<StockLhbYybphRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let dept_name = fstr(item, "OPERATEDEPT_NAME").unwrap_or_default();
        if dept_name.is_empty() {
            continue;
        }
        out.push(StockLhbYybphRow {
            dept_name,
            b1_times: fnum(item, "TOTAL_BUYER_SALESTIMES_1DAY"),
            b1_avg_increase: fnum(item, "AVERAGE_INCREASE_1DAY"),
            b1_rise_probability: fnum(item, "RISE_PROBABILITY_1DAY"),
            b2_times: fnum(item, "TOTAL_BUYER_SALESTIMES_2DAY"),
            b2_avg_increase: fnum(item, "AVERAGE_INCREASE_2DAY"),
            b2_rise_probability: fnum(item, "RISE_PROBABILITY_2DAY"),
            b3_times: fnum(item, "TOTAL_BUYER_SALESTIMES_3DAY"),
            b3_avg_increase: fnum(item, "AVERAGE_INCREASE_3DAY"),
            b3_rise_probability: fnum(item, "RISE_PROBABILITY_3DAY"),
            b5_times: fnum(item, "TOTAL_BUYER_SALESTIMES_5DAY"),
            b5_avg_increase: fnum(item, "AVERAGE_INCREASE_5DAY"),
            b5_rise_probability: fnum(item, "RISE_PROBABILITY_5DAY"),
            b10_times: fnum(item, "TOTAL_BUYER_SALESTIMES_10DAY"),
            b10_avg_increase: fnum(item, "AVERAGE_INCREASE_10DAY"),
            b10_rise_probability: fnum(item, "RISE_PROBABILITY_10DAY"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_lhb_traderstatistic_em — 营业部统计
// ===========================================================================

/// One sales-department statistic row, port of `stock_lhb_traderstatistic_em`.
///
/// Field ids are the Eastmoney `RPT_OPERATEDEPT_LIST_STATISTICS` columns.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockLhbTraderstatisticRow {
    /// `OPERATEDEPT_NAME` 营业部名称
    pub dept_name: String,
    /// `AMOUNT` 龙虎榜成交金额
    pub amount: Option<f64>,
    /// `SALES_ONLIST_TIMES` 上榜次数
    pub onlist_times: Option<f64>,
    /// `ACT_BUY` 买入额
    pub act_buy: Option<f64>,
    /// `TOTAL_BUYER_SALESTIMES` 买入次数
    pub total_buyer_times: Option<f64>,
    /// `ACT_SELL` 卖出额
    pub act_sell: Option<f64>,
    /// `TOTAL_SELLER_SALESTIMES` 卖出次数
    pub total_seller_times: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_lhb_traderstatistic_em(symbol)`.
pub async fn stock_lhb_traderstatistic_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<StockLhbTraderstatisticRow>> {
    let cycle = statistic_cycle(symbol)?;
    let filter = format!("(STATISTICSCYCLE=\"{cycle}\")");
    let params = [
        ("sortColumns", "AMOUNT,OPERATEDEPT_CODE"),
        ("sortTypes", "-1,1"),
        ("pageSize", "5000"),
        ("reportName", "RPT_OPERATEDEPT_LIST_STATISTICS"),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &filter),
    ];
    let items = paged(client, "stock_lhb_traderstatistic_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_lhb_traderstatistic_em(&synthetic)
}

/// Parse a datacenter `result.data` array into [`StockLhbTraderstatisticRow`]s.
pub(crate) fn parse_stock_lhb_traderstatistic_em(
    resp: &Value,
) -> Result<Vec<StockLhbTraderstatisticRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let dept_name = fstr(item, "OPERATEDEPT_NAME").unwrap_or_default();
        if dept_name.is_empty() {
            continue;
        }
        out.push(StockLhbTraderstatisticRow {
            dept_name,
            amount: fnum(item, "AMOUNT"),
            onlist_times: fnum(item, "SALES_ONLIST_TIMES"),
            act_buy: fnum(item, "ACT_BUY"),
            total_buyer_times: fnum(item, "TOTAL_BUYER_SALESTIMES"),
            act_sell: fnum(item, "ACT_SELL"),
            total_seller_times: fnum(item, "TOTAL_SELLER_SALESTIMES"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_lhb_stock_detail_date_em — 个股龙虎榜详情-日期
// ===========================================================================

/// One per-stock LHB-trade-date row, port of `stock_lhb_stock_detail_date_em`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockLhbStockDetailDateRow {
    /// `SECURITY_CODE` 股票代码
    pub code: String,
    /// `TRADE_DATE` 交易日
    pub trade_date: String,
    pub source: &'static str,
}

/// Port of `stock_lhb_stock_detail_date_em(symbol)`.
pub async fn stock_lhb_stock_detail_date_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<StockLhbStockDetailDateRow>> {
    let filter = format!("(SECURITY_CODE=\"{symbol}\")");
    let params = [
        ("reportName", "RPT_LHB_BOARDDATE"),
        ("columns", "SECURITY_CODE,TRADE_DATE,TR_DATE"),
        ("filter", &filter),
        ("pageNumber", "1"),
        ("pageSize", "1000"),
        ("sortTypes", "-1"),
        ("sortColumns", "TRADE_DATE"),
        ("source", "WEB"),
        ("client", "WEB"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_lhb_stock_detail_date_em",
            BASE,
            &params,
        )
        .await?;
    parse_stock_lhb_stock_detail_date_em(&v)
}

/// Parse a datacenter `result.data` array into [`StockLhbStockDetailDateRow`]s.
pub(crate) fn parse_stock_lhb_stock_detail_date_em(
    resp: &Value,
) -> Result<Vec<StockLhbStockDetailDateRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let code = fstr(item, "SECURITY_CODE").unwrap_or_default();
        if code.is_empty() {
            continue;
        }
        out.push(StockLhbStockDetailDateRow {
            code,
            trade_date: fstr(item, "TRADE_DATE").unwrap_or_default(),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_lhb_stock_detail_em — 个股龙虎榜详情 (买入/卖出)
// ===========================================================================

/// One per-stock LHB detail (buy/sell side) row, port of `stock_lhb_stock_detail_em`.
///
/// `columns=ALL`; field ids inferred from the akshare positional rename.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockLhbStockDetailRow {
    /// `OPERATEDEPT_NAME` 交易营业部名称
    pub dept_name: String,
    /// `TYPE` 类型
    pub dept_type: Option<String>,
    /// `BUY_AMT` 买入金额
    pub buy_amt: Option<f64>,
    /// `BUY_AMT_RATIO` 买入金额-占总成交比例
    pub buy_amt_ratio: Option<f64>,
    /// `SELL_AMT` 卖出金额
    pub sell_amt: Option<f64>,
    /// `SELL_AMT_RATIO` 卖出金额-占总成交比例
    pub sell_amt_ratio: Option<f64>,
    /// `NET_AMT` 净额
    pub net_amt: Option<f64>,
    pub source: &'static str,
}

/// Map `flag` ∈ {"买入", "卖出"} to the report name + sort column.
fn stock_detail_report(flag: &str) -> Result<(&'static str, &'static str)> {
    match flag {
        "买入" => Ok(("RPT_BILLBOARD_DAILYDETAILSBUY", "BUY")),
        "卖出" => Ok(("RPT_BILLBOARD_DAILYDETAILSSELL", "SELL")),
        other => Err(Error::InvalidParam(format!(
            "stock_lhb_stock_detail_em: flag must be one of {{\"买入\", \"卖出\"}}, got {other:?}"
        ))),
    }
}

/// Port of `stock_lhb_stock_detail_em(symbol, date, flag)`.
pub async fn stock_lhb_stock_detail_em(
    client: &Client,
    symbol: &str,
    date: &str,
    flag: &str,
) -> Result<Vec<StockLhbStockDetailRow>> {
    check_date8(date, "stock_lhb_stock_detail_em date")?;
    let (report, sort_col) = stock_detail_report(flag)?;
    let d = fmt_date8(date);
    let filter = format!("(TRADE_DATE='{d}')(SECURITY_CODE=\"{symbol}\")");
    let params = [
        ("reportName", report),
        ("columns", "ALL"),
        ("filter", &filter),
        ("pageNumber", "1"),
        ("pageSize", "500"),
        ("sortTypes", "-1"),
        ("sortColumns", sort_col),
        ("source", "WEB"),
        ("client", "WEB"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_lhb_stock_detail_em", BASE, &params)
        .await?;
    parse_stock_lhb_stock_detail_em(&v)
}

/// Parse a datacenter `result.data` array into [`StockLhbStockDetailRow`]s.
pub(crate) fn parse_stock_lhb_stock_detail_em(resp: &Value) -> Result<Vec<StockLhbStockDetailRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let dept_name = fstr(item, "OPERATEDEPT_NAME").unwrap_or_default();
        if dept_name.is_empty() {
            continue;
        }
        out.push(StockLhbStockDetailRow {
            dept_name,
            dept_type: fstr(item, "TYPE"),
            buy_amt: fnum(item, "BUY_AMT"),
            buy_amt_ratio: fnum(item, "BUY_AMT_RATIO"),
            sell_amt: fnum(item, "SELL_AMT"),
            sell_amt_ratio: fnum(item, "SELL_AMT_RATIO"),
            net_amt: fnum(item, "NET_AMT"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_lhb_yyb_detail_em — 营业部历史交易明细
// ===========================================================================

/// One sales-department historical trade row, port of `stock_lhb_yyb_detail_em`.
///
/// Field ids are the Eastmoney `RPT_OPERATEDEPT_TRADE_DETAILSNEW` columns.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockLhbYybDetailRow {
    /// `OPERATEDEPT_CODE` 营业部代码
    pub dept_code: String,
    /// `OPERATEDEPT_NAME` 营业部名称
    pub dept_name: String,
    /// `ORG_NAME_ABBR` 营业部简称
    pub dept_abbr: Option<String>,
    /// `TRADE_DATE` 交易日期
    pub trade_date: String,
    /// `SECURITY_CODE` 股票代码
    pub code: String,
    /// `SECURITY_NAME_ABBR` 股票名称
    pub name: String,
    /// `CHANGE_RATE` 涨跌幅
    pub change_rate: Option<f64>,
    /// `ACT_BUY` 买入金额
    pub buy_amt: Option<f64>,
    /// `ACT_SELL` 卖出金额
    pub sell_amt: Option<f64>,
    /// `NET_AMT` 净额
    pub net_amt: Option<f64>,
    /// `EXPLANATION` 上榜原因
    pub explanation: Option<String>,
    /// `D1_CLOSE_ADJCHRATE` 1日后涨跌幅
    pub d1_close_adjchrate: Option<f64>,
    /// `D2_CLOSE_ADJCHRATE` 2日后涨跌幅
    pub d2_close_adjchrate: Option<f64>,
    /// `D3_CLOSE_ADJCHRATE` 3日后涨跌幅
    pub d3_close_adjchrate: Option<f64>,
    /// `D5_CLOSE_ADJCHRATE` 5日后涨跌幅
    pub d5_close_adjchrate: Option<f64>,
    /// `D10_CLOSE_ADJCHRATE` 10日后涨跌幅
    pub d10_close_adjchrate: Option<f64>,
    /// `D20_CLOSE_ADJCHRATE` 20日后涨跌幅
    pub d20_close_adjchrate: Option<f64>,
    /// `D30_CLOSE_ADJCHRATE` 30日后涨跌幅
    pub d30_close_adjchrate: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_lhb_yyb_detail_em(symbol)` where `symbol` is the 营业部代码.
pub async fn stock_lhb_yyb_detail_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<StockLhbYybDetailRow>> {
    let filter = format!("(OPERATEDEPT_CODE=\"{symbol}\")");
    let params = [
        ("sortColumns", "TRADE_DATE,SECURITY_CODE"),
        ("sortTypes", "-1,1"),
        ("pageSize", "100"),
        ("reportName", "RPT_OPERATEDEPT_TRADE_DETAILSNEW"),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &filter),
    ];
    let items = paged(client, "stock_lhb_yyb_detail_em", &params).await?;
    let synthetic = serde_json::json!({ "result": { "data": items } });
    parse_stock_lhb_yyb_detail_em(&synthetic)
}

/// Parse a datacenter `result.data` array into [`StockLhbYybDetailRow`]s.
pub(crate) fn parse_stock_lhb_yyb_detail_em(resp: &Value) -> Result<Vec<StockLhbYybDetailRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let dept_code = fstr(item, "OPERATEDEPT_CODE").unwrap_or_default();
        let dept_name = fstr(item, "OPERATEDEPT_NAME").unwrap_or_default();
        let code = fstr(item, "SECURITY_CODE").unwrap_or_default();
        let name = fstr(item, "SECURITY_NAME_ABBR").unwrap_or_default();
        if dept_code.is_empty() || code.is_empty() {
            continue;
        }
        out.push(StockLhbYybDetailRow {
            dept_code,
            dept_name,
            dept_abbr: fstr(item, "ORG_NAME_ABBR"),
            trade_date: fstr(item, "TRADE_DATE").unwrap_or_default(),
            code,
            name,
            change_rate: fnum(item, "CHANGE_RATE"),
            buy_amt: fnum(item, "ACT_BUY"),
            sell_amt: fnum(item, "ACT_SELL"),
            net_amt: fnum(item, "NET_AMT"),
            explanation: fstr(item, "EXPLANATION"),
            d1_close_adjchrate: fnum(item, "D1_CLOSE_ADJCHRATE"),
            d2_close_adjchrate: fnum(item, "D2_CLOSE_ADJCHRATE"),
            d3_close_adjchrate: fnum(item, "D3_CLOSE_ADJCHRATE"),
            d5_close_adjchrate: fnum(item, "D5_CLOSE_ADJCHRATE"),
            d10_close_adjchrate: fnum(item, "D10_CLOSE_ADJCHRATE"),
            d20_close_adjchrate: fnum(item, "D20_CLOSE_ADJCHRATE"),
            d30_close_adjchrate: fnum(item, "D30_CLOSE_ADJCHRATE"),
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
    fn parses_stock_lhb_detail_em() {
        let rows = parse_stock_lhb_detail_em(&fixture("stock_lhb_detail_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "600519");
        assert_eq!(rows[0].name, "贵州茅台");
        assert_eq!(rows[0].trade_date, "2023-04-03");
        assert_eq!(rows[0].close_price, Some(1700.0));
        assert_eq!(rows[0].billboard_net_amt, Some(-123456789.0));
        assert_eq!(rows[0].explanation, Some("日涨幅偏离值达7%".to_string()));
        assert_eq!(rows[1].close_price, None);
        assert_eq!(rows[1].source, "eastmoney");
    }

    #[test]
    fn parses_stock_lhb_stock_statistic_em() {
        let rows =
            parse_stock_lhb_stock_statistic_em(&fixture("stock_lhb_stock_statistic_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "000001");
        assert_eq!(rows[0].name, "平安银行");
        assert_eq!(rows[0].latest_trade_date, "2024-04-15");
        assert_eq!(rows[0].billboard_times, Some(5.0));
        assert_eq!(rows[0].inst_buy_net_amt, Some(123456.0));
        assert_eq!(rows[1].m1_close_adjchrate, Some(2.3));
    }

    #[test]
    fn parses_stock_lhb_jgmmtj_em() {
        let rows = parse_stock_lhb_jgmmtj_em(&fixture("stock_lhb_jgmmtj_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "300750");
        assert_eq!(rows[0].name, "宁德时代");
        assert_eq!(rows[0].trade_date, "2024-04-17");
        assert_eq!(rows[0].org_net_buy_amt, Some(987654321.0));
        assert_eq!(rows[0].explanation, Some("机构榜".to_string()));
        assert_eq!(rows[1].buy_org_num, None);
    }

    #[test]
    fn parses_stock_lhb_jgstatistic_em() {
        let rows =
            parse_stock_lhb_jgstatistic_em(&fixture("stock_lhb_jgstatistic_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "600036");
        assert_eq!(rows[0].name, "招商银行");
        assert_eq!(rows[0].onlist_times, Some(8.0));
        assert_eq!(rows[0].net_buy_amt, Some(-5000000.0));
        assert_eq!(rows[0].m3_close_adjchrate, Some(-1.5));
        assert_eq!(rows[1].buy_amt, Some(2000000.0));
    }

    #[test]
    fn parses_stock_lhb_hyyyb_em() {
        let rows = parse_stock_lhb_hyyyb_em(&fixture("stock_lhb_hyyyb_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].dept_name, "华泰证券深圳益田路");
        assert_eq!(rows[0].dept_code, "10188715");
        assert_eq!(rows[0].onlist_date, "2024-04-01");
        assert_eq!(rows[0].total_net_amt, Some(123450000.0));
        assert_eq!(rows[0].buy_stocks, Some("000001,600519".to_string()));
        assert_eq!(rows[1].buy_stock_num, None);
    }

    #[test]
    fn parses_stock_lhb_yybph_em() {
        let rows = parse_stock_lhb_yybph_em(&fixture("stock_lhb_yybph_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].dept_name, "中信证券上海分公司");
        assert_eq!(rows[0].b1_times, Some(12.0));
        assert_eq!(rows[0].b1_avg_increase, Some(3.2));
        assert_eq!(rows[0].b1_rise_probability, Some(0.75));
        assert_eq!(rows[0].b10_times, Some(30.0));
        assert_eq!(rows[1].b5_avg_increase, Some(-0.5));
    }

    #[test]
    fn parses_stock_lhb_traderstatistic_em() {
        let rows = parse_stock_lhb_traderstatistic_em(&fixture("stock_lhb_traderstatistic_em.json"))
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].dept_name, "东方财富拉萨团结路");
        assert_eq!(rows[0].amount, Some(567890123.0));
        assert_eq!(rows[0].onlist_times, Some(45.0));
        assert_eq!(rows[0].act_buy, Some(300000000.0));
        assert_eq!(rows[0].total_seller_times, Some(40.0));
        assert_eq!(rows[1].act_sell, Some(100000.0));
    }

    #[test]
    fn parses_stock_lhb_stock_detail_date_em() {
        let rows = parse_stock_lhb_stock_detail_date_em(
            &fixture("stock_lhb_stock_detail_date_em.json"),
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "002901");
        assert_eq!(rows[0].trade_date, "2022-10-12");
        assert_eq!(rows[1].code, "600077");
        assert_eq!(rows[1].trade_date, "2007-04-16");
    }

    #[test]
    fn parses_stock_lhb_stock_detail_em() {
        let rows =
            parse_stock_lhb_stock_detail_em(&fixture("stock_lhb_stock_detail_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].dept_name, "国泰君安上海江苏路");
        assert_eq!(rows[0].buy_amt, Some(150000000.0));
        assert_eq!(rows[0].buy_amt_ratio, Some(0.23));
        assert_eq!(rows[0].net_amt, Some(50000000.0));
        assert_eq!(rows[0].dept_type, Some("机构".to_string()));
        assert_eq!(rows[1].sell_amt_ratio, None);
    }

    #[test]
    fn parses_stock_lhb_yyb_detail_em() {
        let rows =
            parse_stock_lhb_yyb_detail_em(&fixture("stock_lhb_yyb_detail_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].dept_code, "10188715");
        assert_eq!(rows[0].dept_name, "华泰证券深圳益田路");
        assert_eq!(rows[0].code, "000001");
        assert_eq!(rows[0].name, "平安银行");
        assert_eq!(rows[0].trade_date, "2024-04-01");
        assert_eq!(rows[0].net_amt, Some(-2000000.0));
        assert_eq!(rows[0].d1_close_adjchrate, Some(1.2));
        assert_eq!(rows[1].d30_close_adjchrate, None);
    }
}
