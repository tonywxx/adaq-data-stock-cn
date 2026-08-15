//! Eastmoney fund-flow endpoints (reimplementation of akshare's
//! `stock_fund_em.py` individual/market/rank fund flow and `stock_hsgt_em.py`
//! 沪深港通 fund flow). All endpoints are pure HTTP against Eastmoney; no JS
//! signing is required (ADR-0005).
//!
//! Ported functions:
//! - `stock_individual_fund_flow` -> [`individual_fund_flow`]
//! - `stock_individual_fund_flow_rank` -> [`individual_fund_flow_rank`]
//! - `stock_market_fund_flow` -> [`market_fund_flow`]
//! - `stock_hsgt_fund_flow_summary_em` -> [`hsgt_fund_flow_summary_em`]
//! - `stock_hsgt_hist_em` -> [`hsgt_hist_em`]
//!
//! Skipped (not present in this akshare checkout, only referenced in changelog):
//! `stock_hsgt_north_net_flow_in_em`, `stock_hsgt_north_cash_em`.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Parse a kline CSV cell (`&str`) into an `Option<f64>`; empty/garbage -> None.
fn parse_f64(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        t.parse::<f64>().ok()
    }
}

/// Read an Eastmoney JSON field that may be a number or a numeric string.
/// Accepts the `Option<&Value>` returned by `obj.get(...)`.
fn f64_opt(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => parse_f64(s),
        _ => None,
    }
}

/// Read an Eastmoney JSON field that may be a string or a number rendered as string.
/// Accepts the `Option<&Value>` returned by `obj.get(...)`.
fn str_opt(v: Option<&Value>) -> Option<String> {
    match v {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    }
}

/// Current epoch milliseconds, used as the Eastmoney `_` cache-buster param.
fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// stock_individual_fund_flow
// ---------------------------------------------------------------------------

/// One day of individual-stock fund-flow data (akshare `stock_individual_fund_flow`).
///
/// Columns mirror akshare: 日期, 收盘价, 涨跌幅, 主力净流入-净额, 主力净流入-净占比,
/// 超大单净流入-净额, 超大单净流入-净占比, 大单净流入-净额, 大单净流入-净占比,
/// 中单净流入-净额, 中单净流入-净占比, 小单净流入-净额, 小单净流入-净占比.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndividualFundFlowRow {
    /// 日期 (trade date, e.g. "2024-01-02")
    pub date: String,
    /// 收盘价 (closing price)
    pub close: Option<f64>,
    /// 涨跌幅 (daily percentage change)
    pub pct_change: Option<f64>,
    /// 主力净流入-净额 (main force net inflow, CNY)
    pub main_net_inflow: Option<f64>,
    /// 主力净流入-净占比 (main force net inflow percentage)
    pub main_net_inflow_pct: Option<f64>,
    /// 超大单净流入-净额 (extra-large order net inflow, CNY)
    pub xlarge_net_inflow: Option<f64>,
    /// 超大单净流入-净占比 (extra-large order net inflow percentage)
    pub xlarge_net_inflow_pct: Option<f64>,
    /// 大单净流入-净额 (large order net inflow, CNY)
    pub large_net_inflow: Option<f64>,
    /// 大单净流入-净占比 (large order net inflow percentage)
    pub large_net_inflow_pct: Option<f64>,
    /// 中单净流入-净额 (medium order net inflow, CNY)
    pub mid_net_inflow: Option<f64>,
    /// 中单净流入-净占比 (medium order net inflow percentage)
    pub mid_net_inflow_pct: Option<f64>,
    /// 小单净流入-净额 (small order net inflow, CNY)
    pub small_net_inflow: Option<f64>,
    /// 小单净流入-净占比 (small order net inflow percentage)
    pub small_net_inflow_pct: Option<f64>,
}

/// Individual-stock daily fund flow from Eastmoney (`stock_individual_fund_flow`).
///
/// `market` is one of `sh` (Shanghai, secid prefix 1), `sz` (Shenzhen, 0) or
/// `bj` (Beijing, 0).
pub async fn individual_fund_flow(
    client: &Client,
    stock: &str,
    market: &str,
) -> Result<Vec<IndividualFundFlowRow>> {
    let market_code = match market {
        "sh" => "1",
        "sz" => "0",
        "bj" => "0",
        other => {
            return Err(Error::InvalidParam(format!(
                "unsupported market `{other}`; expected one of sh/sz/bj"
            )))
        }
    };
    let secid = format!("{market_code}.{stock}");
    let ts = now_ms().to_string();
    let params = [
        ("lmt", "0"),
        ("klt", "101"),
        ("secid", &secid),
        ("fields1", "f1,f2,f3,f7"),
        (
            "fields2",
            "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f62,f63,f64,f65",
        ),
        ("ut", "b2884a393a59ad64002292a3e90d46a5"),
        ("_", &ts),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_individual_fund_flow",
            "https://push2his.eastmoney.com/api/qt/stock/fflow/daykline/get",
            &params,
        )
        .await?;
    parse_individual_fund_flow(&v)
}

/// Parse the `data.klines` CSV array of `stock_individual_fund_flow`.
pub(crate) fn parse_individual_fund_flow(resp: &Value) -> Result<Vec<IndividualFundFlowRow>> {
    let klines = resp
        .get("data")
        .and_then(|d| d.get("klines"))
        .and_then(|k| k.as_array());
    let Some(klines) = klines else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(klines.len());
    for line in klines {
        let s = line.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "individual fund-flow kline entry is not a string".into(),
        })?;
        let p: Vec<&str> = s.split(',').collect();
        if p.len() < 15 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: format!("individual kline has {} fields, expected 15", p.len()),
            });
        }
        out.push(IndividualFundFlowRow {
            date: p[0].to_string(),
            close: parse_f64(p[11]),
            pct_change: parse_f64(p[12]),
            main_net_inflow: parse_f64(p[1]),
            main_net_inflow_pct: parse_f64(p[6]),
            xlarge_net_inflow: parse_f64(p[5]),
            xlarge_net_inflow_pct: parse_f64(p[10]),
            large_net_inflow: parse_f64(p[4]),
            large_net_inflow_pct: parse_f64(p[9]),
            mid_net_inflow: parse_f64(p[3]),
            mid_net_inflow_pct: parse_f64(p[8]),
            small_net_inflow: parse_f64(p[2]),
            small_net_inflow_pct: parse_f64(p[7]),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// stock_market_fund_flow
// ---------------------------------------------------------------------------

/// One day of broad-market (Shanghai + Shenzhen) fund-flow data
/// (akshare `stock_market_fund_flow`).
///
/// Columns mirror akshare: 日期, 上证-收盘价, 上证-涨跌幅, 深证-收盘价, 深证-涨跌幅,
/// then 主力/超大单/大单/中单/小单 净流入-净额 and 净占比.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MarketFundFlowRow {
    /// 日期 (trade date)
    pub date: String,
    /// 上证-收盘价 (Shanghai Composite close)
    pub sh_close: Option<f64>,
    /// 上证-涨跌幅 (Shanghai Composite daily change %)
    pub sh_pct_change: Option<f64>,
    /// 深证-收盘价 (Shenzhen Component close)
    pub sz_close: Option<f64>,
    /// 深证-涨跌幅 (Shenzhen Component daily change %)
    pub sz_pct_change: Option<f64>,
    /// 主力净流入-净额 (main force net inflow, CNY)
    pub main_net_inflow: Option<f64>,
    /// 主力净流入-净占比 (main force net inflow percentage)
    pub main_net_inflow_pct: Option<f64>,
    /// 超大单净流入-净额 (extra-large order net inflow, CNY)
    pub xlarge_net_inflow: Option<f64>,
    /// 超大单净流入-净占比 (extra-large order net inflow percentage)
    pub xlarge_net_inflow_pct: Option<f64>,
    /// 大单净流入-净额 (large order net inflow, CNY)
    pub large_net_inflow: Option<f64>,
    /// 大单净流入-净占比 (large order net inflow percentage)
    pub large_net_inflow_pct: Option<f64>,
    /// 中单净流入-净额 (medium order net inflow, CNY)
    pub mid_net_inflow: Option<f64>,
    /// 中单净流入-净占比 (medium order net inflow percentage)
    pub mid_net_inflow_pct: Option<f64>,
    /// 小单净流入-净额 (small order net inflow, CNY)
    pub small_net_inflow: Option<f64>,
    /// 小单净流入-净占比 (small order net inflow percentage)
    pub small_net_inflow_pct: Option<f64>,
}

/// Broad-market daily fund flow from Eastmoney (`stock_market_fund_flow`).
/// Tracks Shanghai (secid 1.000001) and Shenzhen (secid 0.399001) indices.
pub async fn market_fund_flow(client: &Client) -> Result<Vec<MarketFundFlowRow>> {
    let ts = now_ms().to_string();
    let params = [
        ("lmt", "0"),
        ("klt", "101"),
        ("secid", "1.000001"),
        ("secid2", "0.399001"),
        ("fields1", "f1,f2,f3,f7"),
        (
            "fields2",
            "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f62,f63,f64,f65",
        ),
        ("ut", "b2884a393a59ad64002292a3e90d46a5"),
        ("_", &ts),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_market_fund_flow",
            "https://push2his.eastmoney.com/api/qt/stock/fflow/daykline/get",
            &params,
        )
        .await?;
    parse_market_fund_flow(&v)
}

/// Parse the `data.klines` CSV array of `stock_market_fund_flow`.
pub(crate) fn parse_market_fund_flow(resp: &Value) -> Result<Vec<MarketFundFlowRow>> {
    let klines = resp
        .get("data")
        .and_then(|d| d.get("klines"))
        .and_then(|k| k.as_array());
    let Some(klines) = klines else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(klines.len());
    for line in klines {
        let s = line.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "market fund-flow kline entry is not a string".into(),
        })?;
        let p: Vec<&str> = s.split(',').collect();
        if p.len() < 15 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: format!("market kline has {} fields, expected 15", p.len()),
            });
        }
        out.push(MarketFundFlowRow {
            date: p[0].to_string(),
            sh_close: parse_f64(p[11]),
            sh_pct_change: parse_f64(p[12]),
            sz_close: parse_f64(p[13]),
            sz_pct_change: parse_f64(p[14]),
            main_net_inflow: parse_f64(p[1]),
            main_net_inflow_pct: parse_f64(p[6]),
            xlarge_net_inflow: parse_f64(p[5]),
            xlarge_net_inflow_pct: parse_f64(p[10]),
            large_net_inflow: parse_f64(p[4]),
            large_net_inflow_pct: parse_f64(p[9]),
            mid_net_inflow: parse_f64(p[3]),
            mid_net_inflow_pct: parse_f64(p[8]),
            small_net_inflow: parse_f64(p[2]),
            small_net_inflow_pct: parse_f64(p[7]),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// stock_individual_fund_flow_rank
// ---------------------------------------------------------------------------

/// Maps a semantic fund-flow column to its Eastmoney `f`-code for a given
/// indicator window (今日/3日/5日/10日). The field set changes per indicator,
/// so the row struct holds the window-agnostic semantic values.
struct RankFieldMap {
    pct: &'static str,
    main: &'static str,
    main_pct: &'static str,
    xl: &'static str,
    xl_pct: &'static str,
    big: &'static str,
    big_pct: &'static str,
    mid: &'static str,
    mid_pct: &'static str,
    small: &'static str,
    small_pct: &'static str,
}

/// Resolve the `fid`, `fields` and per-indicator `f`-code map for a rank window.
fn rank_field_map(indicator: &str) -> Result<(String, String, RankFieldMap)> {
    match indicator {
        "今日" => Ok((
            "f62".into(),
            "f12,f14,f2,f3,f62,f184,f66,f69,f72,f75,f78,f81,f84,f87,f204,f205,f124".into(),
            RankFieldMap {
                pct: "f3",
                main: "f62",
                main_pct: "f184",
                xl: "f66",
                xl_pct: "f69",
                big: "f72",
                big_pct: "f75",
                mid: "f78",
                mid_pct: "f81",
                small: "f84",
                small_pct: "f87",
            },
        )),
        "3日" => Ok((
            "f267".into(),
            "f12,f14,f2,f127,f267,f268,f269,f270,f271,f272,f273,f274,f275,f276,f257,f258,f124".into(),
            RankFieldMap {
                pct: "f127",
                main: "f267",
                main_pct: "f268",
                xl: "f269",
                xl_pct: "f270",
                big: "f271",
                big_pct: "f272",
                mid: "f273",
                mid_pct: "f274",
                small: "f275",
                small_pct: "f276",
            },
        )),
        "5日" => Ok((
            "f164".into(),
            "f12,f14,f2,f109,f164,f165,f166,f167,f168,f169,f170,f171,f172,f173,f257,f258,f124".into(),
            RankFieldMap {
                pct: "f109",
                main: "f164",
                main_pct: "f165",
                xl: "f166",
                xl_pct: "f167",
                big: "f168",
                big_pct: "f169",
                mid: "f170",
                mid_pct: "f171",
                small: "f172",
                small_pct: "f173",
            },
        )),
        "10日" => Ok((
            "f174".into(),
            "f12,f14,f2,f160,f174,f175,f176,f177,f178,f179,f180,f181,f182,f183,f260,f261,f124".into(),
            RankFieldMap {
                pct: "f160",
                main: "f174",
                main_pct: "f175",
                xl: "f176",
                xl_pct: "f177",
                big: "f178",
                big_pct: "f179",
                mid: "f180",
                mid_pct: "f181",
                small: "f182",
                small_pct: "f183",
            },
        )),
        other => Err(Error::InvalidParam(format!(
            "unsupported indicator `{other}`; expected one of 今日/3日/5日/10日"
        ))),
    }
}

/// One row of the individual-stock fund-flow ranking (akshare
/// `stock_individual_fund_flow_rank`).
///
/// Columns mirror akshare (for the selected window, default 5日): 代码, 名称, 最新价,
/// <窗口>涨跌幅, <窗口>主力净流入-净额, <窗口>主力净流入-净占比,
/// <窗口>超大单净流入-净额, <窗口>超大单净流入-净占比, <窗口>大单净流入-净额,
/// <窗口>大单净流入-净占比, <窗口>中单净流入-净额, <窗口>中单净流入-净占比,
/// <窗口>小单净流入-净额, <窗口>小单净流入-净占比.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndividualFundFlowRankRow {
    /// 代码 (stock code)
    pub code: String,
    /// 名称 (stock name)
    pub name: String,
    /// 最新价 (latest price)
    pub latest_price: Option<f64>,
    /// <窗口>涨跌幅 (window percentage change)
    pub pct_change: Option<f64>,
    /// <窗口>主力净流入-净额 (main force net inflow, CNY)
    pub main_net_inflow: Option<f64>,
    /// <窗口>主力净流入-净占比 (main force net inflow percentage)
    pub main_net_inflow_pct: Option<f64>,
    /// <窗口>超大单净流入-净额 (extra-large order net inflow, CNY)
    pub xlarge_net_inflow: Option<f64>,
    /// <窗口>超大单净流入-净占比 (extra-large order net inflow percentage)
    pub xlarge_net_inflow_pct: Option<f64>,
    /// <窗口>大单净流入-净额 (large order net inflow, CNY)
    pub large_net_inflow: Option<f64>,
    /// <窗口>大单净流入-净占比 (large order net inflow percentage)
    pub large_net_inflow_pct: Option<f64>,
    /// <窗口>中单净流入-净额 (medium order net inflow, CNY)
    pub mid_net_inflow: Option<f64>,
    /// <窗口>中单净流入-净占比 (medium order net inflow percentage)
    pub mid_net_inflow_pct: Option<f64>,
    /// <窗口>小单净流入-净额 (small order net inflow, CNY)
    pub small_net_inflow: Option<f64>,
    /// <窗口>小单净流入-净占比 (small order net inflow percentage)
    pub small_net_inflow_pct: Option<f64>,
}

/// Individual-stock fund-flow ranking from Eastmoney (`stock_individual_fund_flow_rank`).
///
/// `indicator` selects the ranking window: `今日` / `3日` / `5日` / `10日`
/// (default `5日`).
pub async fn individual_fund_flow_rank(
    client: &Client,
    indicator: &str,
) -> Result<Vec<IndividualFundFlowRankRow>> {
    let (fid, fields, map) = rank_field_map(indicator)?;
    let params = [
        ("fid", fid.as_str()),
        ("po", "1"),
        ("pz", "100"),
        ("pn", "1"),
        ("np", "1"),
        ("fltt", "2"),
        ("invt", "2"),
        ("ut", "b2884a393a59ad64002292a3e90d46a5"),
        (
            "fs",
            "m:0+t:6+f:!2,m:0+t:13+f:!2,m:0+t:80+f:!2,m:1+t:2+f:!2,m:1+t:23+f:!2,m:0+t:7+f:!2,m:1+t:3+f:!2",
        ),
        ("fields", fields.as_str()),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_individual_fund_flow_rank",
            "https://push2.eastmoney.com/api/qt/clist/get",
            &params,
        )
        .await?;
    parse_individual_fund_flow_rank(&v, &map)
}

/// Parse the `data.diff` array of `stock_individual_fund_flow_rank` for a window.
pub(crate) fn parse_individual_fund_flow_rank(
    resp: &Value,
    map: &RankFieldMap,
) -> Result<Vec<IndividualFundFlowRankRow>> {
    let diff = resp
        .get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff".into(),
        })?;
    let mut out = Vec::with_capacity(diff.len());
    for obj in diff {
        out.push(IndividualFundFlowRankRow {
            code: str_opt(obj.get("f12")).ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "rank row missing f12 (code)".into(),
            })?,
            name: str_opt(obj.get("f14")).ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "rank row missing f14 (name)".into(),
            })?,
            latest_price: f64_opt(obj.get("f2")),
            pct_change: f64_opt(obj.get(map.pct)),
            main_net_inflow: f64_opt(obj.get(map.main)),
            main_net_inflow_pct: f64_opt(obj.get(map.main_pct)),
            xlarge_net_inflow: f64_opt(obj.get(map.xl)),
            xlarge_net_inflow_pct: f64_opt(obj.get(map.xl_pct)),
            large_net_inflow: f64_opt(obj.get(map.big)),
            large_net_inflow_pct: f64_opt(obj.get(map.big_pct)),
            mid_net_inflow: f64_opt(obj.get(map.mid)),
            mid_net_inflow_pct: f64_opt(obj.get(map.mid_pct)),
            small_net_inflow: f64_opt(obj.get(map.small)),
            small_net_inflow_pct: f64_opt(obj.get(map.small_pct)),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// stock_hsgt_fund_flow_summary_em
// ---------------------------------------------------------------------------

/// One row of the 沪深港通 (HK-Shanghai-Shenzhen Stock Connect) fund-flow summary
/// (akshare `stock_hsgt_fund_flow_summary_em`).
///
/// Columns mirror akshare: 交易日, 类型, 板块, 资金方向, 交易状态, 成交净买额,
/// 资金净流入, 当日资金余额, 上涨数, 持平数, 下跌数, 相关指数, 指数涨跌幅.
/// Amount fields are converted to 万元 (divide raw CNY by 10000), matching akshare.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HsgtFundFlowSummaryRow {
    /// 交易日 (trade date)
    pub trade_date: String,
    /// 类型 (board type, e.g. 沪股通/深股通)
    pub fund_type: Option<String>,
    /// 板块 (mutual type name)
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
pub async fn hsgt_fund_flow_summary_em(client: &Client) -> Result<Vec<HsgtFundFlowSummaryRow>> {
    let params = [
        ("reportName", "RPT_MUTUAL_QUOTA"),
        (
            "columns",
            "TRADE_DATE,MUTUAL_TYPE,BOARD_TYPE,MUTUAL_TYPE_NAME,FUNDS_DIRECTION,INDEX_CODE,INDEX_NAME,BOARD_CODE",
        ),
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
        .get_json(
            SOURCE_EASTMONEY,
            "stock_hsgt_fund_flow_summary_em",
            "https://datacenter-web.eastmoney.com/api/data/v1/get",
            &params,
        )
        .await?;
    parse_hsgt_fund_flow_summary(&v)
}

/// Parse the `result.data` array of `stock_hsgt_fund_flow_summary_em`.
pub(crate) fn parse_hsgt_fund_flow_summary(resp: &Value) -> Result<Vec<HsgtFundFlowSummaryRow>> {
    let data = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data".into(),
        })?;
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
            trade_status: str_opt(obj.get("status")),
            net_buy_amt: f64_opt(obj.get("netBuyAmt")).map(|x| x / 10000.0),
            net_inflow: f64_opt(obj.get("dayNetAmtIn")).map(|x| x / 10000.0),
            day_balance: f64_opt(obj.get("dayAmtRemain")).map(|x| x / 10000.0),
            up_count: f64_opt(obj.get("f104")),
            flat_count: f64_opt(obj.get("f106")),
            down_count: f64_opt(obj.get("f105")),
            index_name: str_opt(obj.get("INDEX_NAME")),
            index_change_pct: f64_opt(obj.get("f3")),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// stock_hsgt_hist_em
// ---------------------------------------------------------------------------

/// One row of 沪深港通 historical fund flow (akshare `stock_hsgt_hist_em`).
///
/// Columns mirror akshare: 日期, 当日成交净买额, 买入成交额, 卖出成交额,
/// 历史累计净买额, 当日资金流入, 当日余额, 持股市值, 领涨股-代码, 领涨股,
/// 领涨股-涨跌幅, <指数>-收盘价, <指数>-涨跌幅.
/// Monetary amounts (except 持股市值) are divided by 100; 历史累计净买额 is divided
/// by 100 for 沪股通/深股通 and by 100*10000 for the others, matching akshare.
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
pub async fn hsgt_hist_em(client: &Client, symbol: &str) -> Result<Vec<HsgtHistRow>> {
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
        ("sortColumns", "TRADE_DATE"),
        ("sortTypes", "-1"),
        ("pageSize", "1000"),
        ("pageNumber", "1"),
        ("reportName", "RPT_MUTUAL_DEAL_HISTORY"),
        ("columns", "ALL"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &filter),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_hsgt_hist_em",
            "https://datacenter-web.eastmoney.com/api/data/v1/get",
            &params,
        )
        .await?;
    parse_hsgt_hist(&v, accum_scale)
}

/// Parse the `result.data` array of `stock_hsgt_hist_em`.
/// `accum_scale` reflects the symbol-dependent division of 历史累计净买额.
pub(crate) fn parse_hsgt_hist(resp: &Value, accum_scale: f64) -> Result<Vec<HsgtHistRow>> {
    let data = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data".into(),
        })?;
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

    #[test]
    fn parses_individual_fund_flow() {
        let v = fixture("flow_individual_fund_flow.json");
        let rows = parse_individual_fund_flow(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02");
        assert_eq!(rows[0].close, Some(10.50));
        assert_eq!(rows[0].pct_change, Some(-1.20));
        assert_eq!(rows[0].main_net_inflow, Some(1_234_567_890.0));
        assert_eq!(rows[0].xlarge_net_inflow, Some(5_678_901.0));
        assert_eq!(rows[0].small_net_inflow, Some(-234_567.0));
        assert_eq!(rows[1].date, "2024-01-03");
        assert_eq!(rows[1].main_net_inflow, Some(2_345_678_901.0));
    }

    #[test]
    fn parses_market_fund_flow() {
        let v = fixture("flow_market_fund_flow.json");
        let rows = parse_market_fund_flow(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02");
        assert_eq!(rows[0].sh_close, Some(2950.12));
        assert_eq!(rows[0].sh_pct_change, Some(-0.50));
        assert_eq!(rows[0].sz_close, Some(9350.45));
        assert_eq!(rows[0].sz_pct_change, Some(0.80));
        assert_eq!(rows[0].main_net_inflow, Some(1_234_567_890.0));
        assert_eq!(rows[1].sz_pct_change, Some(-0.10));
    }

    #[test]
    fn parses_individual_fund_flow_rank_5d() {
        let v = fixture("flow_individual_fund_flow_rank.json");
        // Default indicator is 5日; build the matching field map.
        let (_fid, _fields, map) = rank_field_map("5日").unwrap();
        let rows = parse_individual_fund_flow_rank(&v, &map).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "600519");
        assert_eq!(rows[0].name, "贵州茅台");
        assert_eq!(rows[0].latest_price, Some(1685.00));
        assert_eq!(rows[0].pct_change, Some(3.45));
        assert_eq!(rows[0].main_net_inflow, Some(1_234_567_890.0));
        assert_eq!(rows[0].xlarge_net_inflow, Some(5_678_901.0));
        assert_eq!(rows[0].large_net_inflow, Some(456_789.0));
        assert_eq!(rows[0].mid_net_inflow, Some(-345_678.0));
        assert_eq!(rows[0].small_net_inflow, Some(-234_567.0));
        assert_eq!(rows[1].main_net_inflow, Some(-987_654_321.0));
    }

    #[test]
    fn parses_individual_fund_flow_rank_invalid_indicator() {
        assert!(rank_field_map("7日").is_err());
    }

    #[test]
    fn parses_hsgt_fund_flow_summary() {
        let v = fixture("flow_hsgt_fund_flow_summary_em.json");
        let rows = parse_hsgt_fund_flow_summary(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].trade_date, "2024-01-02T00:00:00");
        assert_eq!(rows[0].fund_type, Some("沪股通".into()));
        assert_eq!(rows[0].board, Some("北向".into()));
        assert_eq!(rows[0].funds_direction, Some("流入".into()));
        assert_eq!(rows[0].trade_status, Some("正常".into()));
        assert_eq!(rows[0].net_inflow, Some(500_000.0));
        assert_eq!(rows[0].net_buy_amt, Some(300_000.0));
        assert_eq!(rows[0].day_balance, Some(400_000.0));
        assert_eq!(rows[0].up_count, Some(800.0));
        assert_eq!(rows[0].flat_count, Some(100.0));
        assert_eq!(rows[0].down_count, Some(300.0));
        assert_eq!(rows[0].index_name, Some("上证指数".into()));
        assert_eq!(rows[0].index_change_pct, Some(0.85));
        assert_eq!(rows[1].net_inflow, Some(-200_000.0));
    }

    #[test]
    fn parses_hsgt_hist_north() {
        let v = fixture("flow_hsgt_hist_em.json");
        // 北向资金 uses a 100*10000 accumulation scale.
        let rows = parse_hsgt_hist(&v, 100.0 * 10000.0).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].trade_date, "2024-01-02T00:00:00");
        assert_eq!(rows[0].fund_inflow, Some(500_000.0));
        assert_eq!(rows[0].net_deal_amt, Some(300_000.0));
        assert_eq!(rows[0].quota_balance, Some(450_000.0));
        assert_eq!(rows[0].buy_amt, Some(800_000.0));
        assert_eq!(rows[0].sell_amt, Some(500_000.0));
        assert_eq!(rows[0].accum_deal_amt, Some(1_500_000.0));
        assert_eq!(rows[0].lead_stock_code, Some("600519".into()));
        assert_eq!(rows[0].lead_stock_name, Some("贵州茅台".into()));
        assert_eq!(rows[0].lead_stock_change_pct, Some(2.34));
        assert_eq!(rows[0].index_close_price, Some(2950.12));
        assert_eq!(rows[0].index_change_pct, Some(0.85));
        assert_eq!(rows[0].hold_market_cap, Some(250_000_000_000.0));
        assert_eq!(rows[1].accum_deal_amt, Some(1_490_000.0));
    }
}
