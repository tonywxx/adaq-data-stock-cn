//! 沪深港通 (Stock Connect) endpoints — Rust reimplementation of akshare's
//! `stock_feature/stock_hsgt_em.py` and `stock_hsgt_min_em.py`.
//!
//! Every function targets Eastmoney's pure-HTTP `datacenter-web` JSON API
//! (or the `push2` intraday endpoint for `stock_hsgt_fund_min_em`); none
//! requires JS-signed params (ADR-0005). Pagination is handled by
//! [`em_dc_rows`] which follows Eastmoney's `result.pages` cursor.
//!
//! | Rust function | akshare source | report / note |
//! |---|---|---|
//! | `stock_hsgt_fund_flow_summary_em` | `stock_hsgt_em.py:18` | `RPT_MUTUAL_QUOTA` |
//! | `stock_hsgt_hist_em` | `stock_hsgt_em.py:1070` | `RPT_MUTUAL_DEAL_HISTORY` |
//! | `stock_hsgt_hold_stock_em` | `stock_hsgt_em.py:171` | `RPT_MUTUAL_STOCK_NORTHSTA` (explicit `trade_date`) |
//! | `stock_hsgt_stock_statistics_em` | `stock_hsgt_em.py:336` | `RPT_MUTUAL_STOCK_NORTHSTA` |
//! | `stock_hsgt_institution_statistics_em` | `stock_hsgt_em.py:778` | `PRT_MUTUAL_ORG_STA` |
//! | `stock_hsgt_board_rank_em` | `stock_hsgt_em.py:1190` | `RPT_MUTUAL_BOARD_HOLDRANK_WEB` (explicit `trade_date`) |
//! | `stock_hsgt_individual_em` | `stock_hsgt_em.py:1512` | `RPT_MUTUAL_HOLDSTOCKNDATE_STA` (A) / `RPT_MUTUAL_STOCK_HOLDRANKS` (HK) |
//! | `stock_hsgt_individual_detail_em` | `stock_hsgt_em.py:1527` | `RPT_MUTUAL_HOLD_DET` |
//! | `stock_hsgt_fund_min_em` | `stock_hsgt_min_em.py:13` | `push2` `kamtbs.rtmin` |
//!
//! ## Notes on fixtures
//! Seven endpoints ship with **real** captured Eastmoney responses saved as
//! fixtures under `tests/fixtures/hsgt_*.json` (structure and field keys
//! verified against live data). Three endpoints — `stock_hsgt_hold_stock_em`,
//! `stock_hsgt_stock_statistics_em` (both `RPT_MUTUAL_STOCK_NORTHSTA`, which is
//! IP-throttled from this build host) and `stock_hsgt_fund_min_em` (`push2`
//! is geo-blocked here) — use **synthetic** fixtures whose field keys follow
//! Eastmoney's standard mutual-stock schema but MUST be calibrated against a
//! live response on first run (the parsers are intentionally tolerant of
//! missing keys so a live parse degrades gracefully rather than hard-failing).

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

const DC: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Read a numeric field that may be a JSON number or a numeric string.
fn f64_opt(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// Read a string field, tolerating numbers rendered as strings.
fn str_opt(v: Option<&Value>) -> Option<String> {
    match v {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    }
}

/// Normalize a date: `YYYYMMDD` -> `YYYY-MM-DD`; pass through otherwise.
fn norm_date(s: &str) -> String {
    let s = s.trim();
    if s.len() == 8 && s.chars().all(|c| c.is_ascii_digit()) {
        format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8])
    } else {
        s.to_string()
    }
}

/// Current epoch milliseconds, used as the Eastmoney `_` cache-buster param.
fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

async fn em_dc_rows(client: &Client, endpoint: &'static str, base: &[(&str, &str)]) -> Result<Vec<Value>> {
    crate::core::pipeline::fetch_dc_all(client, endpoint, base).await
}

/// Extract `result.data` from a single datacenter response as a slice.
fn dc_data(resp: &Value) -> Result<&[Value]> {
    resp.get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .map(|a| a.as_slice())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data".into(),
        })
}

// ---------------------------------------------------------------------------
// stock_hsgt_fund_flow_summary_em  (RPT_MUTUAL_QUOTA)
// ---------------------------------------------------------------------------

/// One row of the 沪深港通 fund-flow summary (akshare `stock_hsgt_fund_flow_summary_em`).
///
/// Columns mirror akshare: 交易日, 类型, 板块, 资金方向, 交易状态, 成交净买额,
/// 资金净流入, 当日资金余额, 上涨数, 持平数, 下跌数, 相关指数, 指数涨跌幅.
/// Amount fields are converted to 万元 (raw CNY / 10000), matching akshare.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HsgtFundFlowSummaryRow {
    /// 交易日 (trade date)
    pub trade_date: String,
    /// 类型 (board type, e.g. 沪港通)
    pub fund_type: Option<String>,
    /// 板块 (mutual type name, e.g. 沪股通)
    pub board: Option<String>,
    /// 资金方向 (funds direction: 流入/流出)
    pub funds_direction: Option<String>,
    /// 交易状态 (trade status)
    pub trade_status: Option<String>,
    /// 成交净买额 (net buy amount, 万元)
    pub net_buy_amt: Option<f64>,
    /// 资金净流入 (net inflow, 万元)
    pub net_inflow: Option<f64>,
    /// 当日资金余额 (day quota balance, 万元)
    pub day_balance: Option<f64>,
    /// 上涨数 (number of rising stocks)
    pub up_count: Option<f64>,
    /// 持平数 (number of flat stocks)
    pub flat_count: Option<f64>,
    /// 下跌数 (number of falling stocks)
    pub down_count: Option<f64>,
    /// 相关指数 (related index name)
    pub index_name: Option<String>,
    /// 指数涨跌幅 (related index change %)
    pub index_change_pct: Option<f64>,
}

/// 沪深港通 fund-flow summary from Eastmoney (`stock_hsgt_fund_flow_summary_em`).
pub async fn stock_hsgt_fund_flow_summary_em(client: &Client) -> Result<Vec<HsgtFundFlowSummaryRow>> {
    let params = [
        ("reportName", "RPT_MUTUAL_QUOTA"),
        ("columns", "TRADE_DATE,MUTUAL_TYPE,BOARD_TYPE,MUTUAL_TYPE_NAME,FUNDS_DIRECTION,INDEX_CODE,INDEX_NAME,BOARD_CODE"),
        ("quoteColumns", "status~07~BOARD_CODE,dayNetAmtIn~07~BOARD_CODE,dayAmtRemain~07~BOARD_CODE,dayAmtThreshold~07~BOARD_CODE,f104~07~BOARD_CODE,f105~07~BOARD_CODE,f106~07~BOARD_CODE,f3~03~INDEX_CODE~INDEX_f3,netBuyAmt~07~BOARD_CODE"),
        ("quoteType", "0"),
        ("pageNumber", "1"),
        ("pageSize", "2000"),
        ("sortTypes", "1"),
        ("sortColumns", "MUTUAL_TYPE"),
        ("source", "WEB"),
        ("client", "WEB"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_hsgt_fund_flow_summary_em", DC, &params)
        .await?;
    parse_hsgt_fund_flow_summary(dc_data(&v)?)
}

/// Parse the `result.data` array of `stock_hsgt_fund_flow_summary_em`.
pub(crate) fn parse_hsgt_fund_flow_summary(data: &[Value]) -> Result<Vec<HsgtFundFlowSummaryRow>> {
    let mut out = Vec::with_capacity(data.len());
    for obj in data {
        out.push(HsgtFundFlowSummaryRow {
            trade_date: str_opt(obj.get("TRADE_DATE")).ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "summary row missing TRADE_DATE".into(),
            })?,
            fund_type: str_opt(obj.get("BOARD_TYPE")),
            board: str_opt(obj.get("MUTUAL_TYPE_NAME")),
            funds_direction: str_opt(obj.get("FUNDS_DIRECTION")),
            trade_status: match obj.get("status") {
                Some(Value::Number(n)) => Some(n.to_string()),
                other => str_opt(other),
            },
            net_buy_amt: f64_opt(obj.get("netBuyAmt")).map(|x| x / 10000.0),
            net_inflow: f64_opt(obj.get("dayNetAmtIn")).map(|x| x / 10000.0),
            day_balance: f64_opt(obj.get("dayAmtRemain")).map(|x| x / 10000.0),
            up_count: f64_opt(obj.get("f104")),
            flat_count: f64_opt(obj.get("f106")),
            down_count: f64_opt(obj.get("f105")),
            index_name: str_opt(obj.get("INDEX_NAME")),
            index_change_pct: f64_opt(obj.get("INDEX_f3")),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// stock_hsgt_hist_em  (RPT_MUTUAL_DEAL_HISTORY)
// ---------------------------------------------------------------------------

/// One row of 沪深港通 historical fund flow (akshare `stock_hsgt_hist_em`).
///
/// Columns mirror akshare: 日期, 当日成交净买额, 买入成交额, 卖出成交额,
/// 历史累计净买额, 当日资金流入, 当日余额, 持股市值, 领涨股-代码, 领涨股,
/// 领涨股-涨跌幅, <指数>-收盘价, <指数>-涨跌幅.
/// Monetary amounts (except 持股市值) are divided by 100; 历史累计净买额 is
/// divided by 100 for 沪股通/深股通 and by 100*10000 for the others.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HsgtHistRow {
    /// 日期 (trade date)
    pub trade_date: String,
    /// 当日成交净买额 (day net deal amount)
    pub net_deal_amt: Option<f64>,
    /// 买入成交额 (buy amount)
    pub buy_amt: Option<f64>,
    /// 卖出成交额 (sell amount)
    pub sell_amt: Option<f64>,
    /// 历史累计净买额 (accumulated net deal amount)
    pub accum_deal_amt: Option<f64>,
    /// 当日资金流入 (day fund inflow)
    pub fund_inflow: Option<f64>,
    /// 当日余额 (day quota balance)
    pub quota_balance: Option<f64>,
    /// 持股市值 (hold market cap)
    pub hold_market_cap: Option<f64>,
    /// 领涨股-代码 (lead stock code)
    pub lead_stock_code: Option<String>,
    /// 领涨股 (lead stock name)
    pub lead_stock_name: Option<String>,
    /// 领涨股-涨跌幅 (lead stock change %)
    pub lead_stock_change_pct: Option<f64>,
    /// <指数>-收盘价 (related index close)
    pub index_close_price: Option<f64>,
    /// <指数>-涨跌幅 (related index change %)
    pub index_change_pct: Option<f64>,
}

/// 沪深港通 historical fund flow from Eastmoney (`stock_hsgt_hist_em`).
///
/// `symbol` selects the series: `北向资金` (default) / `沪股通` / `深股通` /
/// `南向资金` / `港股通沪` / `港股通深`.
pub async fn stock_hsgt_hist_em(client: &Client, symbol: &str) -> Result<Vec<HsgtHistRow>> {
    let type_code = match symbol {
        "北向资金" => "5",
        "沪股通" => "1",
        "深股通" => "3",
        "南向资金" => "6",
        "港股通沪" => "2",
        "港股通深" => "4",
        other => {
            return Err(Error::InvalidParam(format!(
                "unsupported symbol `{other}`; expected one of 北向资金/沪股通/深股通/南向资金/港股通沪/港股通深"
            )))
        }
    };
    // akshare scales 历史累计净买额 by /100 for 沪股通/深股通, /100/10000 otherwise.
    let accum_scale: f64 = if symbol == "沪股通" || symbol == "深股通" {
        100.0
    } else {
        100.0 * 10000.0
    };
    let filter = format!("(MUTUAL_TYPE=\"00{type_code}\")");
    let params = [
        ("reportName", "RPT_MUTUAL_DEAL_HISTORY"),
        ("columns", "ALL"),
        ("pageSize", "1000"),
        ("pageNumber", "1"),
        ("sortColumns", "TRADE_DATE"),
        ("sortTypes", "-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &filter),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_hsgt_hist_em", DC, &params)
        .await?;
    parse_hsgt_hist(dc_data(&v)?, accum_scale)
}

/// Parse the `result.data` array of `stock_hsgt_hist_em`.
/// `accum_scale` reflects the symbol-dependent division of 历史累计净买额.
pub(crate) fn parse_hsgt_hist(data: &[Value], accum_scale: f64) -> Result<Vec<HsgtHistRow>> {
    let mut out = Vec::with_capacity(data.len());
    for obj in data {
        out.push(HsgtHistRow {
            trade_date: str_opt(obj.get("TRADE_DATE")).ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "hist row missing TRADE_DATE".into(),
            })?,
            net_deal_amt: f64_opt(obj.get("NET_DEAL_AMT")).map(|x| x / 100.0),
            buy_amt: f64_opt(obj.get("BUY_AMT")).map(|x| x / 100.0),
            sell_amt: f64_opt(obj.get("SELL_AMT")).map(|x| x / 100.0),
            accum_deal_amt: f64_opt(obj.get("ACCUM_DEAL_AMT")).map(|x| x / accum_scale),
            fund_inflow: f64_opt(obj.get("FUND_INFLOW")).map(|x| x / 100.0),
            quota_balance: f64_opt(obj.get("QUOTA_BALANCE")).map(|x| x / 100.0),
            hold_market_cap: f64_opt(obj.get("HOLD_MARKET_CAP")),
            lead_stock_code: str_opt(obj.get("LEAD_STOCKS_CODE")),
            lead_stock_name: str_opt(obj.get("LEAD_STOCKS_NAME")),
            lead_stock_change_pct: f64_opt(obj.get("LS_CHANGE_RATE")),
            index_close_price: f64_opt(obj.get("INDEX_CLOSE_PRICE")),
            index_change_pct: f64_opt(obj.get("INDEX_CHANGE_RATE")),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// stock_hsgt_hold_stock_em  (RPT_MUTUAL_STOCK_NORTHSTA)  — fixture synthetic
// ---------------------------------------------------------------------------

/// One row of the 沪深港通 individual-stock holding rank
/// (akshare `stock_hsgt_hold_stock_em`).
///
/// Columns mirror akshare: 序号, 代码, 名称, 今日收盘价, 今日涨跌幅,
/// 今日持股-股数, 今日持股-市值, 今日持股-占流通股比, 今日持股-占总股本比,
/// <窗口>增持估计-股数, <窗口>增持估计-市值, <窗口>增持估计-市值增幅,
/// <窗口>增持估计-占流通股比, <窗口>增持估计-占总股本比, 所属板块, 日期.
/// **Field keys are best-effort (synthetic fixture) — calibrate on live run.**
#[derive(Debug, Clone, serde::Serialize)]
pub struct HsgtHoldStockRow {
    /// 序号 (rank)
    pub rank: Option<i64>,
    /// 代码 (stock code)
    pub code: String,
    /// 名称 (stock name)
    pub name: String,
    /// 今日收盘价 (close price)
    pub close: Option<f64>,
    /// 今日涨跌幅 (daily change %)
    pub pct_change: Option<f64>,
    /// 今日持股-股数 (hold shares)
    pub hold_shares: Option<f64>,
    /// 今日持股-市值 (hold market cap)
    pub hold_market_cap: Option<f64>,
    /// 今日持股-占流通股比 (hold ratio of float shares)
    pub hold_ratio_float: Option<f64>,
    /// 今日持股-占总股本比 (hold ratio of total shares)
    pub hold_ratio_total: Option<f64>,
    /// <窗口>增持估计-股数 (est. added shares)
    pub add_shares: Option<f64>,
    /// <窗口>增持估计-市值 (est. added market cap)
    pub add_market_cap: Option<f64>,
    /// <窗口>增持估计-市值增幅 (est. added market-cap change %)
    pub add_market_cap_ratio: Option<f64>,
    /// <窗口>增持估计-占流通股比 (est. added ratio of float shares)
    pub add_ratio_float: Option<f64>,
    /// <窗口>增持估计-占总股本比 (est. added ratio of total shares)
    pub add_ratio_total: Option<f64>,
    /// 所属板块 (board name)
    pub board: Option<String>,
    /// 日期 (trade date)
    pub trade_date: String,
}

/// Map the akshare `indicator` to the Eastmoney `INTERVAL_TYPE` code.
fn hold_indicator_type(indicator: &str) -> &'static str {
    match indicator {
        "今日排行" => "1",
        "3日排行" => "3",
        "5日排行" => "5",
        "10日排行" => "10",
        "月排行" => "M",
        "季排行" => "Q",
        "年排行" => "Y",
        _ => "1",
    }
}

/// 沪深港通 individual-stock holding rank (akshare `stock_hsgt_hold_stock_em`).
///
/// `market` ∈ {北向, 沪股通, 深股通}; `indicator` ∈ {今日排行, 3日排行, 5日排行,
/// 10日排行, 月排行, 季排行, 年排行}. Unlike akshare (which scraps `TRADE_DATE`
/// from the HTML page), this requires an explicit `trade_date` (`YYYY-MM-DD` or
/// `YYYYMMDD`) so the call stays pure datacenter JSON.
pub async fn stock_hsgt_hold_stock_em(
    client: &Client,
    market: &str,
    indicator: &str,
    trade_date: &str,
) -> Result<Vec<HsgtHoldStockRow>> {
    let date = norm_date(trade_date);
    let itype = hold_indicator_type(indicator);
    let market_filter = match market {
        "北向" => format!("(TRADE_DATE='{date}')(INTERVAL_TYPE=\"{itype}\")"),
        "沪股通" => format!("(TRADE_DATE='{date}')(INTERVAL_TYPE=\"{itype}\")(MUTUAL_TYPE=\"001\")"),
        "深股通" => format!("(TRADE_DATE='{date}')(INTERVAL_TYPE=\"{itype}\")(MUTUAL_TYPE=\"003\")"),
        other => {
            return Err(Error::InvalidParam(format!(
                "unsupported market `{other}`; expected one of 北向/沪股通/深股通"
            )))
        }
    };
    let base = [
        ("reportName", "RPT_MUTUAL_STOCK_NORTHSTA"),
        ("columns", "ALL"),
        ("pageSize", "50000"),
        ("pageNumber", "1"),
        ("sortColumns", "ADD_MARKET_CAP"),
        ("sortTypes", "-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &market_filter),
    ];
    let rows = em_dc_rows(client, "stock_hsgt_hold_stock_em", &base).await?;
    parse_hsgt_hold_stock(&rows, indicator)
}

/// Parse `RPT_MUTUAL_STOCK_NORTHSTA` rows. Tolerant of missing keys (synthetic
/// fixture / unverified live schema) so a live parse degrades gracefully.
pub(crate) fn parse_hsgt_hold_stock(data: &[Value], indicator: &str) -> Result<Vec<HsgtHoldStockRow>> {
    let mut out = Vec::with_capacity(data.len());
    for (i, obj) in data.iter().enumerate() {
        out.push(HsgtHoldStockRow {
            rank: obj.get("RN").and_then(|v| v.as_i64()),
            code: str_opt(obj.get("SECURITY_CODE")).ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "hold_stock row missing SECURITY_CODE".into(),
            })?,
            name: str_opt(obj.get("SECURITY_NAME_ABBR"))
                .or_else(|| str_opt(obj.get("SECURITY_NAME")))
                .ok_or_else(|| Error::UpstreamChanged {
                    origin: SOURCE_EASTMONEY,
                    message: "hold_stock row missing SECURITY_NAME".into(),
                })?,
            close: f64_opt(obj.get("CLOSE_PRICE")),
            pct_change: f64_opt(obj.get("CHANGE_RATE")),
            hold_shares: f64_opt(obj.get("HOLD_SHARES")),
            hold_market_cap: f64_opt(obj.get("HOLD_MARKET_CAP")),
            hold_ratio_float: f64_opt(obj.get("HOLD_SHARES_RATIO")),
            hold_ratio_total: f64_opt(obj.get("TOTAL_SHARES_RATIO")),
            add_shares: f64_opt(obj.get("ADD_SHARES")),
            add_market_cap: f64_opt(obj.get("ADD_MARKET_CAP")),
            add_market_cap_ratio: f64_opt(obj.get("ADD_MARKET_CAP_RATIO")),
            add_ratio_float: f64_opt(obj.get("ADD_HOLD_SHARES_RATIO")),
            add_ratio_total: f64_opt(obj.get("ADD_TOTAL_SHARES_RATIO")),
            board: str_opt(obj.get("BOARD_NAME")),
            trade_date: str_opt(obj.get("TRADE_DATE")).unwrap_or_else(|| {
                let _ = indicator;
                String::new()
            }),
        });
        let _ = i;
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// stock_hsgt_stock_statistics_em  (RPT_MUTUAL_STOCK_NORTHSTA)  — fixture synthetic
// ---------------------------------------------------------------------------

/// One row of daily individual-stock holding statistics
/// (akshare `stock_hsgt_stock_statistics_em`).
///
/// Columns mirror akshare: 持股日期, 股票代码, 股票简称, 当日收盘价, 当日涨跌幅,
/// 持股数量, 持股市值, 持股数量占发行股百分比, 持股市值变化-1日, 持股市值变化-5日,
/// 持股市值变化-10日. **Field keys are best-effort (synthetic fixture).**
#[derive(Debug, Clone, serde::Serialize)]
pub struct HsgtStockStatisticsRow {
    /// 持股日期 (hold date)
    pub hold_date: String,
    /// 股票代码 (stock code)
    pub code: String,
    /// 股票简称 (stock name)
    pub name: String,
    /// 当日收盘价 (close price)
    pub close: Option<f64>,
    /// 当日涨跌幅 (daily change %)
    pub pct_change: Option<f64>,
    /// 持股数量 (hold shares)
    pub hold_shares: Option<f64>,
    /// 持股市值 (hold market cap)
    pub hold_market_cap: Option<f64>,
    /// 持股数量占发行股百分比 (hold shares / issued shares %)
    pub hold_ratio: Option<f64>,
    /// 持股市值变化-1日 (hold market-cap change, 1d)
    pub chg_1d: Option<f64>,
    /// 持股市值变化-5日 (hold market-cap change, 5d)
    pub chg_5d: Option<f64>,
    /// 持股市值变化-10日 (hold market-cap change, 10d)
    pub chg_10d: Option<f64>,
}

/// 沪深港通 daily individual-stock holding statistics
/// (akshare `stock_hsgt_stock_statistics_em`).
///
/// `symbol` ∈ {北向持股, 南向持股, 沪股通持股, 深股通持股}; `start_date` /
/// `end_date` are `YYYYMMDD`.
pub async fn stock_hsgt_stock_statistics_em(
    client: &Client,
    symbol: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<HsgtStockStatisticsRow>> {
    let sd = norm_date(start_date);
    let ed = norm_date(end_date);
    let (market_type, mutual_type) = match symbol {
        "北向持股" => ("1", "(\"001\",\"003\")"),
        "南向持股" => ("1", "\"002\""),
        "沪股通持股" => ("1", "\"001\""),
        "深股通持股" => ("1", "\"003\""),
        other => {
            return Err(Error::InvalidParam(format!(
                "unsupported symbol `{other}`; expected one of 北向持股/南向持股/沪股通持股/深股通持股"
            )))
        }
    };
    let single = sd == ed;
    let filter = if single {
        format!("(INTERVAL_TYPE=\"{market_type}\")(MUTUAL_TYPE in {mutual_type})(TRADE_DATE='{sd}')")
    } else {
        format!("(INTERVAL_TYPE=\"{market_type}\")(MUTUAL_TYPE in {mutual_type})(TRADE_DATE>='{sd}')(TRADE_DATE<='{ed}')")
    };
    let base = [
        ("reportName", "RPT_MUTUAL_STOCK_NORTHSTA"),
        ("columns", "ALL"),
        ("pageSize", "1000"),
        ("pageNumber", "1"),
        ("sortColumns", "TRADE_DATE"),
        ("sortTypes", "-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &filter),
    ];
    let rows = em_dc_rows(client, "stock_hsgt_stock_statistics_em", &base).await?;
    parse_hsgt_stock_statistics(&rows)
}

/// Parse `RPT_MUTUAL_STOCK_NORTHSTA` statistics rows (tolerant of missing keys).
pub(crate) fn parse_hsgt_stock_statistics(data: &[Value]) -> Result<Vec<HsgtStockStatisticsRow>> {
    let mut out = Vec::with_capacity(data.len());
    for obj in data {
        out.push(HsgtStockStatisticsRow {
            hold_date: str_opt(obj.get("TRADE_DATE")).ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "stock_statistics row missing TRADE_DATE".into(),
            })?,
            code: str_opt(obj.get("SECURITY_CODE")).ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "stock_statistics row missing SECURITY_CODE".into(),
            })?,
            name: str_opt(obj.get("SECURITY_NAME_ABBR"))
                .or_else(|| str_opt(obj.get("SECURITY_NAME")))
                .ok_or_else(|| Error::UpstreamChanged {
                    origin: SOURCE_EASTMONEY,
                    message: "stock_statistics row missing SECURITY_NAME".into(),
                })?,
            close: f64_opt(obj.get("CLOSE_PRICE")),
            pct_change: f64_opt(obj.get("CHANGE_RATE")),
            hold_shares: f64_opt(obj.get("HOLD_SHARES")),
            hold_market_cap: f64_opt(obj.get("HOLD_MARKET_CAP")),
            hold_ratio: f64_opt(obj.get("HOLD_SHARES_RATIO")),
            chg_1d: f64_opt(obj.get("HOLD_MARKET_CAPONE")),
            chg_5d: f64_opt(obj.get("HOLD_MARKET_CAPFIVE")),
            chg_10d: f64_opt(obj.get("HOLD_MARKET_CAPTEN")),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// stock_hsgt_institution_statistics_em  (PRT_MUTUAL_ORG_STA)
// ---------------------------------------------------------------------------

/// One row of daily institution holding statistics
/// (akshare `stock_hsgt_institution_statistics_em`).
///
/// Columns mirror akshare: 持股日期, 机构名称, 持股只数, 持股市值,
/// 持股市值变化-1日, 持股市值变化-5日, 持股市值变化-10日.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HsgtInstitutionRow {
    /// 持股日期 (hold date)
    pub hold_date: String,
    /// 机构名称 (institution name)
    pub org_name: String,
    /// 持股只数 (number of held stocks)
    pub hold_num: Option<f64>,
    /// 持股市值 (hold market cap)
    pub hold_market_cap: Option<f64>,
    /// 持股市值变化-1日 (1d change)
    pub chg_1d: Option<f64>,
    /// 持股市值变化-5日 (5d change)
    pub chg_5d: Option<f64>,
    /// 持股市值变化-10日 (10d change)
    pub chg_10d: Option<f64>,
}

/// 沪深港通 daily institution holding statistics
/// (akshare `stock_hsgt_institution_statistics_em`).
///
/// `market` ∈ {北向持股, 南向持股, 沪股通持股, 深股通持股}; `start_date` /
/// `end_date` are `YYYYMMDD`.
pub async fn stock_hsgt_institution_statistics_em(
    client: &Client,
    market: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<HsgtInstitutionRow>> {
    let sd = norm_date(start_date);
    let ed = norm_date(end_date);
    let market_type = match market {
        "北向持股" => "N",
        "南向持股" => "S",
        "沪股通持股" => "001",
        "深股通持股" => "003",
        other => {
            return Err(Error::InvalidParam(format!(
                "unsupported market `{other}`; expected one of 北向持股/南向持股/沪股通持股/深股通持股"
            )))
        }
    };
    let base = [
        ("reportName", "PRT_MUTUAL_ORG_STA"),
        ("columns", "ALL"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("sortColumns", "HOLD_DATE"),
        ("sortTypes", "-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &format!("(MARKET_TYPE=\"{market_type}\")(HOLD_DATE>='{sd}')(HOLD_DATE<='{ed}')")),
    ];
    let rows = em_dc_rows(client, "stock_hsgt_institution_statistics_em", &base).await?;
    parse_hsgt_institution(&rows)
}

/// Parse `PRT_MUTUAL_ORG_STA` rows.
pub(crate) fn parse_hsgt_institution(data: &[Value]) -> Result<Vec<HsgtInstitutionRow>> {
    let mut out = Vec::with_capacity(data.len());
    for obj in data {
        out.push(HsgtInstitutionRow {
            hold_date: str_opt(obj.get("HOLD_DATE")).ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "institution row missing HOLD_DATE".into(),
            })?,
            org_name: str_opt(obj.get("ORG_NAME")).ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "institution row missing ORG_NAME".into(),
            })?,
            hold_num: f64_opt(obj.get("HOLD_NUM")),
            hold_market_cap: f64_opt(obj.get("HOLD_MARKET_CAP")),
            chg_1d: f64_opt(obj.get("HOLD_MARKET_CAPONE")),
            chg_5d: f64_opt(obj.get("HOLD_MARKET_CAPFIVE")),
            chg_10d: f64_opt(obj.get("HOLD_MARKET_CAPTEN")),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// stock_hsgt_board_rank_em  (RPT_MUTUAL_BOARD_HOLDRANK_WEB)
// ---------------------------------------------------------------------------

/// One row of the 北向资金 holding board-rank
/// (akshare `stock_hsgt_board_rank_em`).
///
/// Columns mirror akshare: 名称, 最新涨跌幅, 北向资金今日持股-股票只数,
/// 北向资金今日持股-市值, 北向资金今日持股-占板块比, 北向资金今日持股-占北向资金比,
/// 北向资金今日增持估计-股票只数, 北向资金今日增持估计-市值,
/// 北向资金今日增持估计-市值增幅, 北向资金今日增持估计-占板块比,
/// 北向资金今日增持估计-占北向资金比, 报告时间.
/// **Ratio-field key mapping is best-effort — verify against akshare column
/// order on first live run.**
#[derive(Debug, Clone, serde::Serialize)]
pub struct HsgtBoardRankRow {
    /// 名称 (board name)
    pub name: String,
    /// 最新涨跌幅 (latest change %)
    pub latest_pct: Option<f64>,
    /// 北向资金今日持股-股票只数 (northbound hold stock count)
    pub hold_stock_count: Option<f64>,
    /// 北向资金今日持股-市值 (northbound hold market cap)
    pub hold_market_cap: Option<f64>,
    /// 北向资金今日持股-占板块比 (hold ratio of board)
    pub hold_ratio_board: Option<f64>,
    /// 北向资金今日持股-占北向资金比 (hold ratio of northbound)
    pub hold_ratio_north: Option<f64>,
    /// 北向资金今日增持估计-股票只数 (est. added stock count)
    pub add_stock_count: Option<f64>,
    /// 北向资金今日增持估计-市值 (est. added market cap)
    pub add_market_cap: Option<f64>,
    /// 北向资金今日增持估计-市值增幅 (est. added market-cap change %)
    pub add_market_cap_ratio: Option<f64>,
    /// 北向资金今日增持估计-占板块比 (est. added ratio of board)
    pub add_ratio_board: Option<f64>,
    /// 北向资金今日增持估计-占北向资金比 (est. added ratio of northbound)
    pub add_ratio_north: Option<f64>,
    /// 报告时间 (report time)
    pub report_time: Option<String>,
}

/// 北向资金 holding board-rank (akshare `stock_hsgt_board_rank_em`).
///
/// `symbol` ∈ {北向资金增持行业板块排行, 北向资金增持概念板块排行,
/// 北向资金增持地域板块排行}; `indicator` ∈ {今日, 3日, 5日, 10日, 1月, 1季, 1年}.
/// Requires explicit `trade_date` (`YYYY-MM-DD` / `YYYYMMDD`) instead of the
/// HTML-scraped `#bkph_date`.
pub async fn stock_hsgt_board_rank_em(
    client: &Client,
    symbol: &str,
    indicator: &str,
    trade_date: &str,
) -> Result<Vec<HsgtBoardRankRow>> {
    let date = norm_date(trade_date);
    let board = match symbol {
        "北向资金增持行业板块排行" => "5",
        "北向资金增持概念板块排行" => "4",
        "北向资金增持地域板块排行" => "3",
        other => {
            return Err(Error::InvalidParam(format!(
                "unsupported symbol `{other}`; expected one of 北向资金增持行业板块排行/北向资金增持概念板块排行/北向资金增持地域板块排行"
            )))
        }
    };
    let indicator_code = match indicator {
        "今日" => "1",
        "3日" => "3",
        "5日" => "5",
        "10日" => "10",
        "1月" => "M",
        "1季" => "Q",
        "1年" => "Y",
        _ => "1",
    };
    let base = [
        ("reportName", "RPT_MUTUAL_BOARD_HOLDRANK_WEB"),
        ("columns", "ALL"),
        ("quoteColumns", "f3~05~SECURITY_CODE~INDEX_CHANGE_RATIO"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("sortColumns", "ADD_MARKET_CAP"),
        ("sortTypes", "-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &format!("(BOARD_TYPE=\"{board}\")(TRADE_DATE='{date}')(INTERVAL_TYPE=\"{indicator_code}\")")),
    ];
    let rows = em_dc_rows(client, "stock_hsgt_board_rank_em", &base).await?;
    parse_hsgt_board_rank(&rows)
}

/// Parse `RPT_MUTUAL_BOARD_HOLDRANK_WEB` rows (best-effort key mapping).
pub(crate) fn parse_hsgt_board_rank(data: &[Value]) -> Result<Vec<HsgtBoardRankRow>> {
    let mut out = Vec::with_capacity(data.len());
    for obj in data {
        out.push(HsgtBoardRankRow {
            name: str_opt(obj.get("BOARD_NAME")).ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "board_rank row missing BOARD_NAME".into(),
            })?,
            latest_pct: f64_opt(obj.get("INDEX_CHANGE_RATIO")),
            hold_stock_count: f64_opt(obj.get("COMPOSITION_QUANTITY")),
            hold_market_cap: f64_opt(obj.get("HK_VALUE")),
            hold_ratio_board: f64_opt(obj.get("ADD_HK_RATIO")),
            hold_ratio_north: f64_opt(obj.get("BOARD_HK_RATIO")),
            add_stock_count: f64_opt(obj.get("COMPOSITION_QUANTITY_ADD")),
            add_market_cap: f64_opt(obj.get("ADD_MARKET_CAP")),
            add_market_cap_ratio: f64_opt(obj.get("ADD_RATIO")),
            add_ratio_board: f64_opt(obj.get("ADD_BOARD_RATIO")),
            add_ratio_north: f64_opt(obj.get("ADD_HK_RATIO")),
            report_time: str_opt(obj.get("TRADE_DATE")),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// stock_hsgt_individual_em  (RPT_MUTUAL_HOLDSTOCKNDATE_STA A / RPT_MUTUAL_STOCK_HOLDRANKS HK)
// ---------------------------------------------------------------------------

/// One row of a single stock's 沪深港通 holding history
/// (akshare `stock_hsgt_individual_em`). Unifies the A-share and HK branches;
/// A-specific fields (`add_shares` / `add_amt` / `hold_market_cap_chg`) are
/// populated for 6-digit A codes, HK-specific (`chg_1d/5d/10d`) for HK codes.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HsgtIndividualRow {
    /// 持股日期 (hold date)
    pub hold_date: String,
    /// 当日收盘价 (close price)
    pub close: Option<f64>,
    /// 当日涨跌幅 (daily change %)
    pub pct_change: Option<f64>,
    /// 持股数量 (hold shares)
    pub hold_shares: Option<f64>,
    /// 持股市值 (hold market cap)
    pub hold_market_cap: Option<f64>,
    /// 持股数量占A股百分比 (hold ratio of A-shares)
    pub hold_ratio_a: Option<f64>,
    /// 今日增持股数 (A: added shares today)
    pub add_shares: Option<f64>,
    /// 今日增持资金 (A: added amount today)
    pub add_amt: Option<f64>,
    /// 今日持股市值变化 (A: hold market-cap change today)
    pub hold_market_cap_chg: Option<f64>,
    /// 持股市值变化-1日 (HK: 1d change)
    pub chg_1d: Option<f64>,
    /// 持股市值变化-5日 (HK: 5d change)
    pub chg_5d: Option<f64>,
    /// 持股市值变化-10日 (HK: 10d change)
    pub chg_10d: Option<f64>,
}

/// Single stock's 沪深港通 holding history (akshare `stock_hsgt_individual_em`).
///
/// Dispatches on code length: a 6-digit `symbol` hits the A-share report
/// (`RPT_MUTUAL_HOLDSTOCKNDATE_STA`); anything else is treated as an HK code
/// (`RPT_MUTUAL_STOCK_HOLDRANKS`, `SECUCODE="{symbol}.HK"`).
pub async fn stock_hsgt_individual_em(client: &Client, symbol: &str) -> Result<Vec<HsgtIndividualRow>> {
    if symbol.len() == 6 {
        let base = [
            ("reportName", "RPT_MUTUAL_HOLDSTOCKNDATE_STA"),
            ("columns", "ALL"),
            ("pageSize", "500"),
            ("pageNumber", "1"),
            ("sortColumns", "TRADE_DATE"),
            ("sortTypes", "-1"),
            ("source", "WEB"),
            ("client", "WEB"),
            ("filter", &format!("(SECURITY_CODE=\"{symbol}\")(INTERVAL_TYPE=\"1\")")),
        ];
        let rows = em_dc_rows(client, "stock_hsgt_individual_em", &base).await?;
        parse_hsgt_individual_a(&rows)
    } else {
        let base = [
            ("reportName", "RPT_MUTUAL_STOCK_HOLDRANKS"),
            ("columns", "ALL"),
            ("pageSize", "500"),
            ("pageNumber", "1"),
            ("sortColumns", "TRADE_DATE"),
            ("sortTypes", "-1"),
            ("source", "WEB"),
            ("client", "WEB"),
            ("filter", &format!("(SECUCODE=\"{symbol}.HK\")(MUTUAL_TYPE=\"002\")")),
        ];
        let rows = em_dc_rows(client, "stock_hsgt_individual_em", &base).await?;
        parse_hsgt_individual_hk(&rows)
    }
}

/// Parse A-share individual holding rows (`RPT_MUTUAL_HOLDSTOCKNDATE_STA`).
pub(crate) fn parse_hsgt_individual_a(data: &[Value]) -> Result<Vec<HsgtIndividualRow>> {
    let mut out = Vec::with_capacity(data.len());
    for obj in data {
        out.push(HsgtIndividualRow {
            hold_date: str_opt(obj.get("TRADE_DATE")).ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "individual_a row missing TRADE_DATE".into(),
            })?,
            close: f64_opt(obj.get("CLOSE_PRICE")),
            pct_change: f64_opt(obj.get("CHANGE_RATE")),
            hold_shares: f64_opt(obj.get("HOLD_SHARES")),
            hold_market_cap: f64_opt(obj.get("HOLD_MARKET_CAP")),
            hold_ratio_a: f64_opt(obj.get("HOLD_SHARES_RATIO")),
            add_shares: f64_opt(obj.get("ADD_SHARES_REPAIR")),
            add_amt: f64_opt(obj.get("PREDICT_AMC")),
            hold_market_cap_chg: f64_opt(obj.get("HMC_CHANGE")),
            chg_1d: None,
            chg_5d: None,
            chg_10d: None,
        });
    }
    Ok(out)
}

/// Parse HK individual holding rows (`RPT_MUTUAL_STOCK_HOLDRANKS`).
pub(crate) fn parse_hsgt_individual_hk(data: &[Value]) -> Result<Vec<HsgtIndividualRow>> {
    let mut out = Vec::with_capacity(data.len());
    for obj in data {
        out.push(HsgtIndividualRow {
            hold_date: str_opt(obj.get("HOLD_DATE")).or_else(|| str_opt(obj.get("TRADE_DATE"))).ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "individual_hk row missing HOLD_DATE/TRADE_DATE".into(),
            })?,
            close: f64_opt(obj.get("CLOSE_PRICE")),
            pct_change: f64_opt(obj.get("CHANGE_RATE")),
            hold_shares: f64_opt(obj.get("HOLD_SHARES")),
            hold_market_cap: f64_opt(obj.get("HOLD_MARKET_CAP")),
            hold_ratio_a: f64_opt(obj.get("HOLD_SHARES_RATIO")),
            add_shares: None,
            add_amt: None,
            hold_market_cap_chg: None,
            chg_1d: f64_opt(obj.get("HOLD_MARKETCAP_CHG1")),
            chg_5d: f64_opt(obj.get("HOLD_MARKETCAP_CHG5")),
            chg_10d: f64_opt(obj.get("HOLD_MARKETCAP_CHG10")),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// stock_hsgt_individual_detail_em  (RPT_MUTUAL_HOLD_DET)
// ---------------------------------------------------------------------------

/// One row of a single stock's 沪深港通 holding detail per institution
/// (akshare `stock_hsgt_individual_detail_em`).
///
/// Columns mirror akshare: 持股日期, 当日收盘价, 当日涨跌幅, 机构名称, 持股数量,
/// 持股市值, 持股数量占A股百分比, 持股市值变化-1日, 持股市值变化-5日,
/// 持股市值变化-10日.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HsgtIndividualDetailRow {
    /// 持股日期 (hold date)
    pub hold_date: String,
    /// 当日收盘价 (close price)
    pub close: Option<f64>,
    /// 当日涨跌幅 (daily change %)
    pub pct_change: Option<f64>,
    /// 机构名称 (institution name)
    pub org_name: String,
    /// 持股数量 (hold shares)
    pub hold_shares: Option<f64>,
    /// 持股市值 (hold market cap)
    pub hold_market_cap: Option<f64>,
    /// 持股数量占A股百分比 (hold ratio of A-shares)
    pub hold_ratio_a: Option<f64>,
    /// 持股市值变化-1日 (1d change)
    pub chg_1d: Option<f64>,
    /// 持股市值变化-5日 (5d change)
    pub chg_5d: Option<f64>,
    /// 持股市值变化-10日 (10d change)
    pub chg_10d: Option<f64>,
}

/// Single stock's 沪深港通 holding detail per institution
/// (akshare `stock_hsgt_individual_detail_em`). `start_date` / `end_date` are
/// `YYYYMMDD`. Mirrors akshare's MARKET_CODE 003 -> 001 fallback.
pub async fn stock_hsgt_individual_detail_em(
    client: &Client,
    symbol: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<HsgtIndividualDetailRow>> {
    let sd = norm_date(start_date);
    let ed = norm_date(end_date);
    let mut last: Option<Value> = None;
    for market_code in ["003", "001"] {
        let base = [
            ("reportName", "RPT_MUTUAL_HOLD_DET"),
            ("columns", "ALL"),
            ("pageSize", "500"),
            ("pageNumber", "1"),
            ("sortColumns", "HOLD_DATE"),
            ("sortTypes", "-1"),
            ("source", "WEB"),
            ("client", "WEB"),
            ("filter", &format!("(SECURITY_CODE=\"{symbol}\")(MARKET_CODE=\"{market_code}\")(HOLD_DATE>='{sd}')(HOLD_DATE<='{ed}')")),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, "stock_hsgt_individual_detail_em", DC, &base)
            .await?;
        let has_data = v
            .get("result")
            .and_then(|r| r.get("data"))
            .and_then(|d| d.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false);
        if has_data {
            return parse_hsgt_individual_detail(dc_data(&v)?);
        }
        last = Some(v);
    }
    // Both markets empty: parse the last response (yields empty vec).
    parse_hsgt_individual_detail(dc_data(last.as_ref().unwrap())?)
}

/// Parse `RPT_MUTUAL_HOLD_DET` rows.
pub(crate) fn parse_hsgt_individual_detail(data: &[Value]) -> Result<Vec<HsgtIndividualDetailRow>> {
    let mut out = Vec::with_capacity(data.len());
    for obj in data {
        out.push(HsgtIndividualDetailRow {
            hold_date: str_opt(obj.get("HOLD_DATE")).ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "individual_detail row missing HOLD_DATE".into(),
            })?,
            close: f64_opt(obj.get("CLOSE_PRICE")),
            pct_change: f64_opt(obj.get("CHANGE_RATE")),
            org_name: str_opt(obj.get("ORG_NAME")).ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "individual_detail row missing ORG_NAME".into(),
            })?,
            hold_shares: f64_opt(obj.get("HOLD_NUM")),
            hold_market_cap: f64_opt(obj.get("HOLD_MARKET_CAP")),
            hold_ratio_a: f64_opt(obj.get("HOLD_SHARES_RATIO")),
            chg_1d: f64_opt(obj.get("HOLD_MARKET_CAPONE")),
            chg_5d: f64_opt(obj.get("HOLD_MARKET_CAPFIVE")),
            chg_10d: f64_opt(obj.get("HOLD_MARKET_CAPTEN")),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// stock_hsgt_fund_min_em  (push2 kamtbs.rtmin)  — fixture synthetic
// ---------------------------------------------------------------------------

/// One intraday minute row of 沪深港通 fund flow (akshare `stock_hsgt_fund_min_em`).
///
/// Columns mirror akshare: 日期, 时间, 沪股通/港股通沪, 深股通/港股通深, 北向资金/南向资金.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HsgtFundMinRow {
    /// 日期 (trade date)
    pub date: String,
    /// 时间 (intraday time, e.g. "09:30")
    pub time: String,
    /// 沪股通 (north: 沪股通) or 港股通沪 (south: 港股通沪)
    pub sh: Option<f64>,
    /// 深股通 (north: 深股通) or 港股通深 (south: 港股通深)
    pub sz: Option<f64>,
    /// 北向资金 (north) or 南向资金 (south)
    pub net: Option<f64>,
}

/// 沪深港通 intraday fund-flow (akshare `stock_hsgt_fund_min_em`).
///
/// `symbol` ∈ {北向资金, 南向资金}. Reads Eastmoney `push2` `kamtbs.rtmin`.
pub async fn stock_hsgt_fund_min_em(client: &Client, symbol: &str) -> Result<Vec<HsgtFundMinRow>> {
    let ts = now_ms().to_string();
    let params = [
        ("fields1", "f1,f2,f3,f4"),
        ("fields2", "f51,f54,f52,f58,f53,f62,f56,f57,f60,f61"),
        ("ut", "b2884a393a59ad64002292a3e90d46a5"),
        ("_", &ts),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_hsgt_fund_min_em", &crate::core::eastmoney_push::push2_url("/api/qt/kamtbs.rtmin/get").await, &params)
        .await?;
    parse_hsgt_fund_min(&v, symbol)
}

/// Parse the `data.s2n` / `data.n2s` intraday strings of `stock_hsgt_fund_min_em`.
pub(crate) fn parse_hsgt_fund_min(resp: &Value, symbol: &str) -> Result<Vec<HsgtFundMinRow>> {
    let data = resp.get("data").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "missing data".into(),
    })?;
    let (series_key, date_key) = if symbol == "南向资金" {
        ("n2s", "n2sDate")
    } else {
        ("s2n", "s2nDate")
    };
    let series = data.get(series_key).and_then(|s| s.as_array()).ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: format!("missing data.{series_key}"),
    })?;
    let date = data.get(date_key).and_then(|d| d.as_str()).unwrap_or("").to_string();
    let mut out = Vec::with_capacity(series.len());
    for item in series {
        let s = item.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "fund_min entry is not a string".into(),
        })?;
        // akshare: split on ',' then take indices [0,1,3,5]; append date as col -1.
        let p: Vec<&str> = s.split(',').collect();
        if p.len() < 6 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: format!("fund_min entry has {} fields, expected >=6", p.len()),
            });
        }
        out.push(HsgtFundMinRow {
            date: date.clone(),
            time: p[0].to_string(),
            sh: p[1].parse::<f64>().ok(),
            sz: p[3].parse::<f64>().ok(),
            net: p[5].parse::<f64>().ok(),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Tests (offline, fixtures under tests/fixtures/)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Load a fixture under `tests/fixtures/<name>` as a JSON `Value`.
    fn fixture(name: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let txt = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    /// Extract `result.data` from a fixture as a slice.
    fn data_of(name: &str) -> Vec<Value> {
        let v = fixture(name);
        v.get("result")
            .and_then(|r| r.get("data"))
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default()
    }

    #[test]
    fn parses_hsgt_fund_flow_summary() {
        let rows = parse_hsgt_fund_flow_summary(&data_of("hsgt_fund_flow_summary_em.json")).unwrap();
        assert!(!rows.is_empty());
        let r = &rows[0];
        assert_eq!(r.trade_date, "2026-08-14 00:00:00");
        assert_eq!(r.board, Some("沪股通".into()));
        assert_eq!(r.funds_direction, Some("北向".into()));
        assert_eq!(r.up_count, Some(667.0));
        assert_eq!(r.down_count, Some(924.0));
        assert_eq!(r.flat_count, Some(48.0));
        assert_eq!(r.index_change_pct, Some(0.01));
    }

    #[test]
    fn parses_hsgt_hist_north() {
        let rows = parse_hsgt_hist(&data_of("hsgt_hist_em.json"), 100.0 * 10000.0).unwrap();
        assert!(!rows.is_empty());
        let r = &rows[0];
        // Most-recent date row may leave 净买额/累计 fields null (unsettled); assert
        // the always-populated columns instead.
        assert_eq!(r.trade_date, "2026-08-14 00:00:00");
        assert_eq!(r.lead_stock_code, Some("002907.SZ".into()));
        assert_eq!(r.lead_stock_name, Some("华森制药".into()));
        assert_eq!(r.lead_stock_change_pct, Some(10.02));
        assert_eq!(r.index_close_price, Some(3927.18));
        assert_eq!(r.index_change_pct, Some(0.01));
    }

    #[test]
    fn parses_hsgt_hold_stock_synthetic() {
        let rows = parse_hsgt_hold_stock(&data_of("hsgt_hold_stock_em.json"), "5日排行").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "600519");
        assert_eq!(rows[0].name, "贵州茅台");
        assert_eq!(rows[0].close, Some(1431.2));
        assert_eq!(rows[0].hold_shares, Some(82354427.0));
        assert_eq!(rows[0].add_market_cap, Some(56264037.44));
        assert_eq!(rows[0].trade_date, "2024-08-16 00:00:00");
    }

    #[test]
    fn parses_hsgt_stock_statistics_synthetic() {
        let rows = parse_hsgt_stock_statistics(&data_of("hsgt_stock_statistics_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "600519");
        assert_eq!(rows[0].name, "贵州茅台");
        assert_eq!(rows[0].hold_date, "2024-08-16 00:00:00");
        assert_eq!(rows[0].close, Some(1431.2));
        assert_eq!(rows[0].chg_1d, Some(411165619.48));
    }

    #[test]
    fn parses_hsgt_institution() {
        let rows = parse_hsgt_institution(&data_of("hsgt_institution_statistics_em.json")).unwrap();
        assert!(!rows.is_empty());
        let r = &rows[0];
        assert_eq!(r.hold_date, "2024-08-16 00:00:00");
        assert_eq!(r.org_name, "华侨证券经纪(香港)有限公司");
        assert_eq!(r.hold_num, Some(76.0));
        assert_eq!(r.hold_market_cap, Some(15775719.01));
        assert_eq!(r.chg_1d, Some(11006.37));
        assert_eq!(r.chg_5d, Some(-81010.72));
        assert_eq!(r.chg_10d, Some(14915.5));
    }

    #[test]
    fn parses_hsgt_board_rank() {
        let rows = parse_hsgt_board_rank(&data_of("hsgt_board_rank_em.json")).unwrap();
        assert!(!rows.is_empty());
        let r = &rows[0];
        assert_eq!(r.name, "银行");
        assert_eq!(r.latest_pct, Some(-0.54));
        assert_eq!(r.hold_stock_count, Some(42.0));
        assert_eq!(r.add_stock_count, Some(32.0));
        assert_eq!(r.add_market_cap, Some(1190196450.6614));
        assert_eq!(r.add_market_cap_ratio, Some(0.64512398));
    }

    #[test]
    fn parses_hsgt_individual_a() {
        let rows = parse_hsgt_individual_a(&data_of("hsgt_individual_a_em.json")).unwrap();
        assert!(!rows.is_empty());
        let r = &rows[0];
        assert_eq!(r.hold_date, "2024-08-16 00:00:00");
        assert_eq!(r.hold_shares, Some(82354427.0));
        assert_eq!(r.hold_market_cap, Some(117865655922.4));
        assert_eq!(r.hold_ratio_a, Some(6.55));
        assert_eq!(r.add_shares, Some(39399.0));
        assert_eq!(r.add_amt, Some(56264037.4425));
        assert_eq!(r.hold_market_cap_chg, Some(411165619.48));
        assert_eq!(r.chg_1d, None);
    }

    #[test]
    fn parses_hsgt_individual_hk() {
        let rows = parse_hsgt_individual_hk(&data_of("hsgt_individual_hk_em.json")).unwrap();
        assert!(!rows.is_empty());
        let r = &rows[0];
        assert_eq!(r.hold_date, "2026-08-14 00:00:00");
        assert_eq!(r.hold_shares, Some(1057180430.0));
        assert_eq!(r.hold_market_cap, Some(465159389200.0));
        assert_eq!(r.hold_ratio_a, Some(11.61));
        assert!(r.chg_1d.is_some());
        assert!(r.add_shares.is_none());
    }

    #[test]
    fn parses_hsgt_individual_detail() {
        let rows = parse_hsgt_individual_detail(&data_of("hsgt_individual_detail_em.json")).unwrap();
        assert!(!rows.is_empty());
        let r = &rows[0];
        assert_eq!(r.hold_date, "2024-08-16 00:00:00");
        assert_eq!(r.org_name, "微牛证券有限公司");
        assert_eq!(r.hold_shares, Some(400.0));
        assert_eq!(r.hold_market_cap, Some(572480.0));
        assert_eq!(r.close, Some(1431.2));
        assert_eq!(r.chg_1d, Some(1724.0));
        assert_eq!(r.chg_5d, Some(-2240.0));
    }

    #[test]
    fn parses_hsgt_fund_min_synthetic() {
        let v = fixture("hsgt_fund_min_em.json");
        let rows = parse_hsgt_fund_min(&v, "北向资金").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02");
        assert_eq!(rows[0].time, "09:30");
        assert_eq!(rows[0].sh, Some(100.5));
        assert_eq!(rows[0].sz, Some(200.3));
        assert_eq!(rows[0].net, Some(300.8));
    }

    #[test]
    fn hist_rejects_bad_symbol() {
        // Use a throwaway client; the error is returned before any network call.
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = Client::new();
            let r = stock_hsgt_hist_em(&client, "火星资金").await;
            assert!(r.is_err());
        });
    }
}
