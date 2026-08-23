//! 东方财富 / 新浪 资金流向 (fund flow) data — ports `akshare/stock/stock_fund_em.py`
//! (Eastmoney `push2` / `push2his`), `akshare/stock_feature/stock_fund_flow.py`
//! (10jqka, deferred), and `akshare/stock/stock_industry.py` (`stock_sector_detail`).
//!
//! All implemented functions hit **pure HTTP** Eastmoney (`push2.eastmoney.com`,
//! `push2his.eastmoney.com`) or Sina (`vip.stock.finance.sina.com.cn`) JSON
//! endpoints — no JS-signing, token, cookie, HTML-scrape, or Excel download.
//!
//! | Rust function | akshare source | endpoint | notes |
//! |---|---|---|---|
//! | `stock_concept_fund_flow_hist` | `stock/stock_fund_em.py:1136` | push2his daykline | concept → code via push2 clist (`m:90+t:3`) |
//! | `stock_main_fund_flow` | `stock/stock_fund_em.py:1223` | push2 clist | 主力净流入排名, 8 `symbol` presets |
//! | `stock_sector_fund_flow_hist` | `stock/stock_fund_em.py:1024` | push2his daykline | sector → code via push2 clist (`m:90 t:2`) |
//! | `stock_sector_fund_flow_rank` | `stock/stock_fund_em.py:447` | push2 clist | 3 indicators × 3 sector types |
//! | `stock_sector_fund_flow_summary` | `stock/stock_fund_em.py:738` | push2 clist | sector → code, then stocks in `b:<code>` |
//! | `stock_sector_detail` | `stock/stock_industry.py:77` | sina getHQNodeData | Sina 板块成份详情 (paginated) |
//!
//! The 10jqka `stock_fund_flow_*` fns in `stock_feature/stock_fund_flow.py`
//! require a `hexin-v` header computed by executing `ths.js` in a JS engine
//! (`py_mini_racer`) — see the `## DEFERRED` section below.
//!
//! These endpoints are **not** Eastmoney `datacenter-web`, so the
//! `emg_data_array` helper (used by `macro_china2.rs`) does not apply; the
//! `push2` responses carry rows under `data.diff` and `push2his` under
//! `data.klines`.
//!
//! ## DEFERRED
//!
//! * `stock_fund_flow_big_deal` (`stock_feature/stock_fund_flow.py:349`) —
//!   **JS-signed**: requires `hexin-v` header from `ths.js` (`py_mini_racer`).
//! * `stock_fund_flow_concept` (`stock_feature/stock_fund_flow.py:137`) —
//!   **JS-signed**: `hexin-v` header from `ths.js`.
//! * `stock_fund_flow_individual` (`stock_feature/stock_fund_flow.py:41`) —
//!   **JS-signed**: `hexin-v` header from `ths.js`.
//! * `stock_fund_flow_industry` (`stock_feature/stock_fund_flow.py:243`) —
//!   **JS-signed**: `hexin-v` header from `ths.js`.
//!
//! (These four are 同花顺 endpoints that depend on a JS-evaluated `hexin-v`
//! token; implementing them would require bundling/running `ths.js`.)

use std::collections::HashMap;

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use crate::core::json::*;

const SOURCE_EASTMONEY: &str = "eastmoney";
const SOURCE_SINA: &str = "sina";

const PUSH2HIS: &str = "https://push2his.eastmoney.com/api/qt/stock/fflow/daykline/get";

const SINA_NODE_COUNT: &str = "http://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeStockCount";
const SINA_NODE_DATA: &str = "http://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeData";

/// Fixed Eastmoney `ut` token used by the `push2`/`push2his` fund-flow endpoints
/// (a static, well-known value in akshare — not JS-signed, so safe to embed).
const UT: &str = "b2884a393a59ad64002292a3e90d46a5";

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

/// Extract `data.diff` (the row-object array) from a `push2` clist response.
fn emh_diff(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|x| x.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff".into(),
        })
}

/// Extract `data.klines` (the comma-separated string array) from a `push2his`
/// daykline response.
fn emh_klines(resp: &Value) -> Result<Vec<String>> {
    let arr = resp
        .get("data")
        .and_then(|d| d.get("klines"))
        .and_then(|x| x.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.klines".into(),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for v in arr {
        match v {
            Value::String(s) => out.push(s.clone()),
            // Some endpoints wrap each kline as a 1-element array; tolerate that.
            Value::Array(a) if a.len() == 1 => {
                if let Some(Value::String(s)) = a.first() {
                    out.push(s.clone());
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

/// Parse a numeric token from a split kline, returning `None` for `"-"` or
/// out-of-range / unparseable cells.
fn kf(cells: &[&str], idx: usize) -> Option<f64> {
    cells.get(idx).and_then(|s| {
        let t = s.trim();
        if t == "-" || t.is_empty() {
            None
        } else {
            t.parse::<f64>().ok()
        }
    })
}

/// A single day's main/large/extra-large/medium/small net inflow for a concept
/// or industry sector (push2his daykline, `fields2=f51..f65`). Shared by
/// `stock_concept_fund_flow_hist` and `stock_sector_fund_flow_hist`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundFlowHistRow {
    /// 日期 (kline f51)
    pub date: String,
    /// 主力净流入-净额 (kline f52)
    pub main_net_in: Option<f64>,
    /// 主力净流入-净占比 (kline f57)
    pub main_net_pct: Option<f64>,
    /// 超大单净流入-净额 (kline f56)
    pub xxl_net_in: Option<f64>,
    /// 超大单净流入-净占比 (kline f61)
    pub xxl_net_pct: Option<f64>,
    /// 大单净流入-净额 (kline f55)
    pub big_net_in: Option<f64>,
    /// 大单净流入-净占比 (kline f60)
    pub big_net_pct: Option<f64>,
    /// 中单净流入-净额 (kline f54)
    pub mid_net_in: Option<f64>,
    /// 中单净流入-净占比 (kline f59)
    pub mid_net_pct: Option<f64>,
    /// 小单净流入-净额 (kline f53)
    pub small_net_in: Option<f64>,
    /// 小单净流入-净占比 (kline f58)
    pub small_net_pct: Option<f64>,
}

/// Shared parser for a `push2his` daykline `klines` payload (concept & sector
/// histories share the exact same 15-field layout).
fn parse_fflow_klines(klines: &[String]) -> Vec<FundFlowHistRow> {
    let mut out = Vec::with_capacity(klines.len());
    for line in klines {
        let cells: Vec<&str> = line.split(',').collect();
        if cells.is_empty() {
            continue;
        }
        out.push(FundFlowHistRow {
            date: cells[0].to_string(),
            main_net_in: kf(&cells, 1),
            small_net_in: kf(&cells, 2),
            mid_net_in: kf(&cells, 3),
            big_net_in: kf(&cells, 4),
            xxl_net_in: kf(&cells, 5),
            main_net_pct: kf(&cells, 6),
            small_net_pct: kf(&cells, 7),
            mid_net_pct: kf(&cells, 8),
            big_net_pct: kf(&cells, 9),
            xxl_net_pct: kf(&cells, 10),
        });
    }
    out
}

/// Parse `stock_concept_fund_flow_hist` rows from a daykline `klines` payload.
pub(crate) fn parse_concept_fund_flow_hist(klines: &[String]) -> Vec<FundFlowHistRow> {
    parse_fflow_klines(klines)
}

/// Parse `stock_sector_fund_flow_hist` rows from a daykline `klines` payload.
pub(crate) fn parse_sector_fund_flow_hist(klines: &[String]) -> Vec<FundFlowHistRow> {
    parse_fflow_klines(klines)
}

// ---------------------------------------------------------------------------
// name → code resolution (Eastmoney `push2` clist)
// ---------------------------------------------------------------------------

/// Build the `name → code` map returned by akshare's
/// `_get_stock_concept_fund_flow_summary_code` (`stock/stock_fund_em.py:1111`,
/// `fs=m:90+t:3`) / `_get_stock_sector_fund_flow_summary_code`
/// (`stock/stock_fund_em.py:712`, `fs=m:90 t:2`). Both return `data.diff` with
/// `f14`=name, `f12`=code.
pub(crate) fn parse_name_code_map(diff: &[Value]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for item in diff {
        if let (Some(name), Some(code)) = (opt_str(item, "f14"), opt_str(item, "f12")) {
            map.insert(name, code);
        }
    }
    map
}

async fn fetch_name_code_map(
    client: &Client,
    fs: &str,
    ut: &str,
) -> Result<HashMap<String, String>> {
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "fund_flow_name_code_map", &crate::core::eastmoney_push::push2_url("/api/qt/clist/get").await,
            &[
                ("pn", "1"),
                ("pz", "5000"),
                ("po", "1"),
                ("np", "1"),
                ("fields", "f3,f12,f13,f14,f62"),
                ("fid", "f62"),
                ("fs", fs),
                ("ut", ut),
            ],
        )
        .await?;
    let diff = emh_diff(&v)?;
    Ok(parse_name_code_map(diff))
}

// ---------------------------------------------------------------------------
// stock_concept_fund_flow_hist  (stock/stock_fund_em.py:1136)
// ---------------------------------------------------------------------------

/// 东方财富-概念资金流-概念历史资金流 (push2his daykline), default `symbol="数据要素"`.
pub async fn stock_concept_fund_flow_hist(
    client: &Client,
    symbol: &str,
) -> Result<Vec<FundFlowHistRow>> {
    let map = fetch_name_code_map(client, "m:90+t:3", UT).await?;
    let code = map
        .get(symbol)
        .ok_or_else(|| Error::InvalidParam(format!("unknown concept symbol: {symbol}")))?;
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_concept_fund_flow_hist",
            PUSH2HIS,
            &[
                ("lmt", "0"),
                ("klt", "101"),
                ("fields1", "f1,f2,f3,f7"),
                (
                    "fields2",
                    "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f62,f63,f64,f65",
                ),
                ("secid", &format!("90.{code}")),
            ],
        )
        .await?;
    let klines = emh_klines(&v)?;
    Ok(parse_concept_fund_flow_hist(&klines))
}

// ---------------------------------------------------------------------------
// stock_sector_fund_flow_hist  (stock/stock_fund_em.py:1024)
// ---------------------------------------------------------------------------

/// 东方财富-行业资金流-行业历史资金流 (push2his daykline), default `symbol="汽车服务"`.
pub async fn stock_sector_fund_flow_hist(
    client: &Client,
    symbol: &str,
) -> Result<Vec<FundFlowHistRow>> {
    let map = fetch_name_code_map(client, "m:90 t:2", UT).await?;
    let code = map
        .get(symbol)
        .ok_or_else(|| Error::InvalidParam(format!("unknown sector symbol: {symbol}")))?;
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_sector_fund_flow_hist",
            PUSH2HIS,
            &[
                ("lmt", "0"),
                ("klt", "101"),
                ("fields1", "f1,f2,f3,f7"),
                (
                    "fields2",
                    "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f62,f63,f64,f65",
                ),
                ("secid", &format!("90.{code}")),
            ],
        )
        .await?;
    let klines = emh_klines(&v)?;
    Ok(parse_sector_fund_flow_hist(&klines))
}

// ---------------------------------------------------------------------------
// stock_main_fund_flow  (stock/stock_fund_em.py:1223)
// ---------------------------------------------------------------------------

/// A row of the 主力净流入排名 board (push2 clist, `fs` from `symbol`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct MainFundFlowRow {
    /// 序号 (synthesized row index)
    pub seq: usize,
    /// 代码 (f12)
    pub code: String,
    /// 名称 (f14)
    pub name: String,
    /// 最新价 (f2)
    pub price: Option<f64>,
    /// 今日排行榜-主力净占比 (f184)
    pub main_pct_today: Option<f64>,
    /// 今日排行榜-今日排名 (f225)
    pub rank_today: Option<f64>,
    /// 今日排行榜-今日涨跌 (f3)
    pub change_today: Option<f64>,
    /// 5日排行榜-主力净占比 (f165)
    pub main_pct_5d: Option<f64>,
    /// 5日排行榜-5日排名 (f263)
    pub rank_5d: Option<f64>,
    /// 5日排行榜-5日涨跌 (f109)
    pub change_5d: Option<f64>,
    /// 10日排行榜-主力净占比 (f175)
    pub main_pct_10d: Option<f64>,
    /// 10日排行榜-10日排名 (f264)
    pub rank_10d: Option<f64>,
    /// 10日排行榜-10日涨跌 (f160)
    pub change_10d: Option<f64>,
    /// 所属板块 (f100)
    pub sector: Option<String>,
}

/// Parse `stock_main_fund_flow` rows from a push2 clist `data.diff`.
pub(crate) fn parse_main_fund_flow(diff: &[Value]) -> Vec<MainFundFlowRow> {
    let mut out = Vec::with_capacity(diff.len());
    for (i, item) in diff.iter().enumerate() {
        let Some(code) = opt_str(item, "f12") else {
            continue;
        };
        let Some(name) = opt_str(item, "f14") else {
            continue;
        };
        out.push(MainFundFlowRow {
            seq: i + 1,
            code,
            name,
            price: opt_f64(item, "f2"),
            main_pct_today: opt_f64(item, "f184"),
            rank_today: opt_f64(item, "f225"),
            change_today: opt_f64(item, "f3"),
            main_pct_5d: opt_f64(item, "f165"),
            rank_5d: opt_f64(item, "f263"),
            change_5d: opt_f64(item, "f109"),
            main_pct_10d: opt_f64(item, "f175"),
            rank_10d: opt_f64(item, "f264"),
            change_10d: opt_f64(item, "f160"),
            sector: opt_str(item, "f100"),
        });
    }
    out
}

/// 东方财富-主力净流入排名 (push2 clist). `symbol` ∈ {全部股票, 沪深A股, 沪市A股,
/// 科创板, 深市A股, 创业板, 沪市B股, 深市B股}; default `symbol="全部股票"`.
pub async fn stock_main_fund_flow(client: &Client, symbol: &str) -> Result<Vec<MainFundFlowRow>> {
    let symbol_map = [
        (
            "全部股票",
            "m:0+t:6+f:!2,m:0+t:13+f:!2,m:0+t:80+f:!2,m:1+t:2+f:!2,m:1+t:23+f:!2,m:0+t:7+f:!2,m:1+t:3+f:!2",
        ),
        (
            "沪深A股",
            "m:0+t:6+f:!2,m:0+t:13+f:!2,m:0+t:80+f:!2,m:1+t:2+f:!2,m:1+t:23+f:!2",
        ),
        ("沪市A股", "m:1+t:2+f:!2,m:1+t:23+f:!2"),
        ("科创板", "m:1+t:23+f:!2"),
        ("深市A股", "m:0+t:6+f:!2,m:0+t:13+f:!2,m:0+t:80+f:!2"),
        ("创业板", "m:0+t:80+f:!2"),
        ("沪市B股", "m:1+t:3+f:!2"),
        ("深市B股", "m:0+t:7+f:!2"),
    ];
    let fs = symbol_map
        .iter()
        .find(|(k, _)| *k == symbol)
        .map(|(_, v)| *v)
        .ok_or_else(|| Error::InvalidParam(format!("unknown symbol: {symbol}")))?;
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_main_fund_flow", &crate::core::eastmoney_push::push2_url("/api/qt/clist/get").await,
            &[
                ("fid", "f184"),
                ("po", "1"),
                ("pz", "100"),
                ("pn", "1"),
                ("np", "1"),
                ("fltt", "2"),
                ("invt", "2"),
                (
                    "fields",
                    "f2,f3,f12,f13,f14,f62,f184,f225,f165,f263,f109,f175,f264,f160,f100,f124,f265,f1",
                ),
                ("ut", UT),
                ("fs", fs),
            ],
        )
        .await?;
    let diff = emh_diff(&v)?;
    Ok(parse_main_fund_flow(diff))
}

// ---------------------------------------------------------------------------
// stock_sector_fund_flow_rank  (stock/stock_fund_em.py:447)
// ---------------------------------------------------------------------------

/// Period selector for the sector rank / summary endpoints.
#[derive(Debug, Clone, Copy)]
pub enum FundFlowPeriod {
    /// 今日 (today)
    Today,
    /// 5日 (5-day)
    FiveDay,
    /// 10日 (10-day)
    TenDay,
}

impl FundFlowPeriod {
    /// `push2` field keys for the net-inflow / pct columns of this period:
    /// `(main, main_pct, change_pct, xxl, xxl_pct, big, big_pct, mid, mid_pct,
    /// small, small_pct, top_stock)`.
    fn fields(self) -> [&'static str; 12] {
        match self {
            FundFlowPeriod::Today => [
                "f62", "f184", "f3", "f66", "f69", "f72", "f75", "f78", "f81", "f84", "f87", "f204",
            ],
            FundFlowPeriod::FiveDay => [
                "f164", "f165", "f109", "f166", "f167", "f168", "f169", "f170", "f171", "f172",
                "f173", "f257",
            ],
            FundFlowPeriod::TenDay => [
                "f174", "f175", "f160", "f176", "f177", "f178", "f179", "f180", "f181", "f182",
                "f183", "f260",
            ],
        }
    }

    /// `push2` `fid`/`stat` for this period.
    fn fid_stat(self) -> (&'static str, &'static str) {
        match self {
            FundFlowPeriod::Today => ("f62", "1"),
            FundFlowPeriod::FiveDay => ("f164", "5"),
            FundFlowPeriod::TenDay => ("f174", "10"),
        }
    }
}

/// A sector (industry / concept / region) row in the fund-flow ranking board
/// (push2 clist). Column names are period-agnostic; the period is carried by the
/// caller (`stock_sector_fund_flow_rank`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SectorFundFlowRankRow {
    /// 名称 (f14)
    pub name: String,
    /// 涨跌幅 (period: f3 / f109 / f160)
    pub change_pct: Option<f64>,
    /// 主力净流入-净额 (f62 / f164 / f174)
    pub main_net_in: Option<f64>,
    /// 主力净流入-净占比 (f184 / f165 / f175)
    pub main_net_pct: Option<f64>,
    /// 超大单净流入-净额 (f66 / f166 / f176)
    pub xxl_net_in: Option<f64>,
    /// 超大单净流入-净占比 (f69 / f167 / f177)
    pub xxl_net_pct: Option<f64>,
    /// 大单净流入-净额 (f72 / f168 / f178)
    pub big_net_in: Option<f64>,
    /// 大单净流入-净占比 (f75 / f169 / f179)
    pub big_net_pct: Option<f64>,
    /// 中单净流入-净额 (f78 / f170 / f180)
    pub mid_net_in: Option<f64>,
    /// 中单净流入-净占比 (f81 / f171 / f181)
    pub mid_net_pct: Option<f64>,
    /// 小单净流入-净额 (f84 / f172 / f182)
    pub small_net_in: Option<f64>,
    /// 小单净流入-净占比 (f87 / f173 / f183)
    pub small_net_pct: Option<f64>,
    /// 主力净流入最大股 (f204 / f257 / f260)
    pub top_stock: Option<String>,
}

/// Parse `stock_sector_fund_flow_rank` rows from a push2 clist `data.diff`.
pub(crate) fn parse_sector_fund_flow_rank(
    diff: &[Value],
    period: FundFlowPeriod,
) -> Vec<SectorFundFlowRankRow> {
    let f = period.fields();
    let mut out = Vec::with_capacity(diff.len());
    for item in diff {
        let Some(name) = opt_str(item, "f14") else {
            continue;
        };
        out.push(SectorFundFlowRankRow {
            name,
            change_pct: opt_f64(item, f[2]),
            main_net_in: opt_f64(item, f[0]),
            main_net_pct: opt_f64(item, f[1]),
            xxl_net_in: opt_f64(item, f[3]),
            xxl_net_pct: opt_f64(item, f[4]),
            big_net_in: opt_f64(item, f[5]),
            big_net_pct: opt_f64(item, f[6]),
            mid_net_in: opt_f64(item, f[7]),
            mid_net_pct: opt_f64(item, f[8]),
            small_net_in: opt_f64(item, f[9]),
            small_net_pct: opt_f64(item, f[10]),
            top_stock: opt_str(item, f[11]),
        });
    }
    out
}

/// 东方财富-板块资金流-排名 (push2 clist). `indicator` ∈ {今日, 5日, 10日};
/// `sector_type` ∈ {行业资金流, 概念资金流, 地域资金流}. Defaults 今日 / 行业资金流.
pub async fn stock_sector_fund_flow_rank(
    client: &Client,
    indicator: FundFlowPeriod,
    sector_type: &str,
) -> Result<Vec<SectorFundFlowRankRow>> {
    let sector_type_map = [
        ("行业资金流", "2"),
        ("概念资金流", "3"),
        ("地域资金流", "1"),
    ];
    let fs = sector_type_map
        .iter()
        .find(|(k, _)| *k == sector_type)
        .map(|(_, v)| *v)
        .ok_or_else(|| Error::InvalidParam(format!("unknown sector_type: {sector_type}")))?;
    let (fid, stat) = indicator.fid_stat();
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_sector_fund_flow_rank", &crate::core::eastmoney_push::push2_url("/api/qt/clist/get").await,
            &[
                ("pn", "1"),
                ("pz", "100"),
                ("po", "1"),
                ("np", "1"),
                ("ut", UT),
                ("fltt", "2"),
                ("invt", "2"),
                ("fid0", fid),
                ("fs", &format!("m:90 t:{fs}")),
                ("stat", stat),
                (
                    "fields",
                    "f12,f14,f2,f3,f62,f184,f66,f69,f72,f75,f78,f81,f84,f87,f204,f205,f124",
                ),
                ("rt", "52975239"),
            ],
        )
        .await?;
    let diff = emh_diff(&v)?;
    Ok(parse_sector_fund_flow_rank(diff, indicator))
}

// ---------------------------------------------------------------------------
// stock_sector_fund_flow_summary  (stock/stock_fund_em.py:738)
// ---------------------------------------------------------------------------

/// A per-stock row inside a sector's fund-flow summary (push2 clist `b:<code>`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SectorFundFlowSummaryRow {
    /// 序号 (synthesized row index)
    pub seq: usize,
    /// 代码 (f12)
    pub code: String,
    /// 名称 (f14)
    pub name: String,
    /// 最新价 (f2)
    pub price: Option<f64>,
    /// 涨跌幅 (period: f3 / f109 / f160)
    pub change_pct: Option<f64>,
    /// 主力净流入-净额 (f62 / f164 / f174)
    pub main_net_in: Option<f64>,
    /// 主力净流入-净占比 (f184 / f165 / f175)
    pub main_net_pct: Option<f64>,
    /// 超大单净流入-净额 (f66 / f166 / f176)
    pub xxl_net_in: Option<f64>,
    /// 超大单净流入-净占比 (f69 / f167 / f177)
    pub xxl_net_pct: Option<f64>,
    /// 大单净流入-净额 (f72 / f168 / f178)
    pub big_net_in: Option<f64>,
    /// 大单净流入-净占比 (f75 / f169 / f179)
    pub big_net_pct: Option<f64>,
    /// 中单净流入-净额 (f78 / f170 / f180)
    pub mid_net_in: Option<f64>,
    /// 中单净流入-净占比 (f81 / f171 / f181)
    pub mid_net_pct: Option<f64>,
    /// 小单净流入-净额 (f84 / f172 / f182)
    pub small_net_in: Option<f64>,
    /// 小单净流入-净占比 (f87 / f173 / f183)
    pub small_net_pct: Option<f64>,
}

/// Parse `stock_sector_fund_flow_summary` rows from a push2 clist `data.diff`.
pub(crate) fn parse_sector_fund_flow_summary(
    diff: &[Value],
    period: FundFlowPeriod,
) -> Vec<SectorFundFlowSummaryRow> {
    let f = period.fields();
    let mut out = Vec::with_capacity(diff.len());
    for (i, item) in diff.iter().enumerate() {
        let Some(code) = opt_str(item, "f12") else {
            continue;
        };
        let Some(name) = opt_str(item, "f14") else {
            continue;
        };
        out.push(SectorFundFlowSummaryRow {
            seq: i + 1,
            code,
            name,
            price: opt_f64(item, "f2"),
            change_pct: opt_f64(item, f[2]),
            main_net_in: opt_f64(item, f[0]),
            main_net_pct: opt_f64(item, f[1]),
            xxl_net_in: opt_f64(item, f[3]),
            xxl_net_pct: opt_f64(item, f[4]),
            big_net_in: opt_f64(item, f[5]),
            big_net_pct: opt_f64(item, f[6]),
            mid_net_in: opt_f64(item, f[7]),
            mid_net_pct: opt_f64(item, f[8]),
            small_net_in: opt_f64(item, f[9]),
            small_net_pct: opt_f64(item, f[10]),
        });
    }
    out
}

/// 东方财富-行业资金流-xx行业个股资金流 (push2 clist, `b:<code>`). Resolves
/// `symbol` → sector code via the sector name→code map, then lists its stocks.
/// `indicator` ∈ {今日, 5日, 10日}. Default `symbol="电源设备"`, 今日.
pub async fn stock_sector_fund_flow_summary(
    client: &Client,
    symbol: &str,
    indicator: FundFlowPeriod,
) -> Result<Vec<SectorFundFlowSummaryRow>> {
    let map = fetch_name_code_map(client, "m:90 t:2", UT).await?;
    let code = map
        .get(symbol)
        .ok_or_else(|| Error::InvalidParam(format!("unknown sector symbol: {symbol}")))?;
    let f = indicator.fields();
    let fields = format!(
        "f12,f14,f2,{},{},{},{},{},{},{},{},{},{},{}",
        f[2], f[0], f[1], f[3], f[4], f[5], f[6], f[7], f[8], f[9], f[10]
    );
    let (fid, pz) = match indicator {
        FundFlowPeriod::Today => ("f62", "5000"),
        FundFlowPeriod::FiveDay => ("f164", "50000"),
        FundFlowPeriod::TenDay => ("f174", "50000"),
    };
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_sector_fund_flow_summary", &crate::core::eastmoney_push::push2_url("/api/qt/clist/get").await,
            &[
                ("fid", fid),
                ("po", "1"),
                ("pz", pz),
                ("pn", "1"),
                ("np", "2"),
                ("fltt", "2"),
                ("invt", "2"),
                ("fs", &format!("b:{code}")),
                ("fields", &fields),
            ],
        )
        .await?;
    let diff = emh_diff(&v)?;
    Ok(parse_sector_fund_flow_summary(diff, indicator))
}

// ---------------------------------------------------------------------------
// stock_sector_detail  (stock/stock_industry.py:77) — Sina 板块成份详情
// ---------------------------------------------------------------------------

/// A constituent stock of a Sina sector board (`Market_Center.getHQNodeData`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SectorDetailRow {
    /// 股票代码 (symbol)
    pub symbol: String,
    /// 代码 (code)
    pub code: String,
    /// 名称 (name)
    pub name: String,
    /// 最新价 (trade)
    pub trade: Option<f64>,
    /// 涨跌额 (pricechange)
    pub price_change: Option<f64>,
    /// 涨跌幅 (changepercent)
    pub change_percent: Option<f64>,
    /// 买入 (buy)
    pub buy: Option<f64>,
    /// 卖出 (sell)
    pub sell: Option<f64>,
    /// 昨收 (settlement)
    pub settlement: Option<f64>,
    /// 开盘 (open)
    pub open: Option<f64>,
    /// 最高 (high)
    pub high: Option<f64>,
    /// 最低 (low)
    pub low: Option<f64>,
    /// 成交量 (volume)
    pub volume: Option<f64>,
    /// 成交额 (amount)
    pub amount: Option<f64>,
    /// 市盈率 (per)
    pub pe: Option<f64>,
    /// 市净率 (pb)
    pub pb: Option<f64>,
    /// 总市值 (mktcap)
    pub mkt_cap: Option<f64>,
    /// 流通市值 (nmc)
    pub nmc: Option<f64>,
    /// 换手率 (turnoverratio)
    pub turnover_ratio: Option<f64>,
}

/// Parse `stock_sector_detail` rows from a Sina `getHQNodeData` JSON array.
pub(crate) fn parse_sector_detail(arr: &[Value]) -> Vec<SectorDetailRow> {
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let Some(symbol) = opt_str(item, "symbol") else {
            continue;
        };
        let Some(code) = opt_str(item, "code") else {
            continue;
        };
        let Some(name) = opt_str(item, "name") else {
            continue;
        };
        out.push(SectorDetailRow {
            symbol,
            code,
            name,
            trade: opt_f64(item, "trade"),
            price_change: opt_f64(item, "pricechange"),
            change_percent: opt_f64(item, "changepercent"),
            buy: opt_f64(item, "buy"),
            sell: opt_f64(item, "sell"),
            settlement: opt_f64(item, "settlement"),
            open: opt_f64(item, "open"),
            high: opt_f64(item, "high"),
            low: opt_f64(item, "low"),
            volume: opt_f64(item, "volume"),
            amount: opt_f64(item, "amount"),
            pe: opt_f64(item, "per"),
            pb: opt_f64(item, "pb"),
            mkt_cap: opt_f64(item, "mktcap"),
            nmc: opt_f64(item, "nmc"),
            turnover_ratio: opt_f64(item, "turnoverratio"),
        });
    }
    out
}

/// 新浪行业-板块行情-成份详情 (Sina `Market_Center.getHQNodeData`, paginated
/// 80/page). `sector` is a `stock_sector_spot` label, e.g. `"gn_gfgn"`.
pub async fn stock_sector_detail(client: &Client, sector: &str) -> Result<Vec<SectorDetailRow>> {
    let count_v = client
        .get_json(
            SOURCE_SINA,
            "stock_sector_detail_count",
            SINA_NODE_COUNT,
            &[("node", sector)],
        )
        .await?;
    let total: u64 = count_v.as_u64().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "stock count response was not an integer".into(),
    })?;
    let pages = (total / 80)
        .saturating_add(if total.is_multiple_of(80) { 0 } else { 1 })
        .max(1);
    // Bound pagination to avoid runaway loops on malformed `total`.
    let pages = pages.min(200);

    let mut out = Vec::new();
    for page in 1..=pages {
        let v = client
            .get_json(
                SOURCE_SINA,
                "stock_sector_detail",
                SINA_NODE_DATA,
                &[
                    ("page", &page.to_string()),
                    ("num", "80"),
                    ("sort", "symbol"),
                    ("asc", "1"),
                    ("node", sector),
                    ("symbol", ""),
                    ("_s_r_a", "page"),
                ],
            )
            .await?;
        let arr = v.as_array().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "getHQNodeData response was not an array".into(),
        })?;
        out.extend(parse_sector_detail(arr));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// stock_individual_fund_flow  (stock/stock_fund_em.py:20)
// ---------------------------------------------------------------------------

/// 东方财富-个股资金流-历史 (push2his daykline, `secid = <market>.<code>`).
/// `market` ∈ {`sh`, `sz`, `bj`} → Eastmoney market prefix (`sh`→1, else→0).
pub async fn stock_individual_fund_flow(
    client: &Client,
    stock: &str,
    market: &str,
) -> Result<Vec<FundFlowHistRow>> {
    let secid = match market {
        "sh" => format!("1.{stock}"),
        "sz" | "bj" => format!("0.{stock}"),
        other => {
            return Err(Error::InvalidParam(format!(
                "stock_individual_fund_flow: unknown market {other:?} (use sh/sz/bj)"
            )));
        }
    };
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_individual_fund_flow",
            PUSH2HIS,
            &[
                ("lmt", "0"),
                ("klt", "101"),
                ("fields1", "f1,f2,f3,f7"),
                (
                    "fields2",
                    "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f62,f63,f64,f65",
                ),
                ("secid", &secid),
            ],
        )
        .await?;
    let klines = emh_klines(&v)?;
    Ok(parse_fflow_klines(&klines))
}

// ---------------------------------------------------------------------------
// stock_market_fund_flow  (stock/stock_fund_em.py:347)
// ---------------------------------------------------------------------------

/// A single day's main/large/extra-large/medium/small net inflow for the
/// broader market, plus the Shanghai/Shenzhen index close & change.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MarketFundFlowRow {
    /// 日期 (kline f51)
    pub date: String,
    /// 主力净流入-净额 (f52)
    pub main_net_in: Option<f64>,
    /// 主力净流入-净占比 (f57)
    pub main_net_pct: Option<f64>,
    /// 超大单净流入-净额 (f56)
    pub xxl_net_in: Option<f64>,
    /// 超大单净流入-净占比 (f61)
    pub xxl_net_pct: Option<f64>,
    /// 大单净流入-净额 (f55)
    pub big_net_in: Option<f64>,
    /// 大单净流入-净占比 (f60)
    pub big_net_pct: Option<f64>,
    /// 中单净流入-净额 (f54)
    pub mid_net_in: Option<f64>,
    /// 中单净流入-净占比 (f59)
    pub mid_net_pct: Option<f64>,
    /// 小单净流入-净额 (f53)
    pub small_net_in: Option<f64>,
    /// 小单净流入-净占比 (f58)
    pub small_net_pct: Option<f64>,
    /// 上证-收盘价 (f62)
    pub sh_close: Option<f64>,
    /// 上证-涨跌幅 (f63)
    pub sh_pct: Option<f64>,
    /// 深证-收盘价 (f64)
    pub sz_close: Option<f64>,
    /// 深证-涨跌幅 (f65)
    pub sz_pct: Option<f64>,
}

/// Parse `stock_market_fund_flow` rows from a daykline `klines` payload.
pub(crate) fn parse_market_fund_flow(klines: &[String]) -> Vec<MarketFundFlowRow> {
    let mut out = Vec::with_capacity(klines.len());
    for line in klines {
        let cells: Vec<&str> = line.split(',').collect();
        if cells.is_empty() {
            continue;
        }
        out.push(MarketFundFlowRow {
            date: cells[0].to_string(),
            main_net_in: kf(&cells, 1),
            small_net_in: kf(&cells, 2),
            mid_net_in: kf(&cells, 3),
            big_net_in: kf(&cells, 4),
            xxl_net_in: kf(&cells, 5),
            main_net_pct: kf(&cells, 6),
            small_net_pct: kf(&cells, 7),
            mid_net_pct: kf(&cells, 8),
            big_net_pct: kf(&cells, 9),
            xxl_net_pct: kf(&cells, 10),
            sh_close: kf(&cells, 11),
            sh_pct: kf(&cells, 12),
            sz_close: kf(&cells, 13),
            sz_pct: kf(&cells, 14),
        });
    }
    out
}

/// 东方财富-大盘资金流 (push2his daykline, `secid=1.000001` + `secid2=0.399001`).
pub async fn stock_market_fund_flow(client: &Client) -> Result<Vec<MarketFundFlowRow>> {
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_market_fund_flow",
            PUSH2HIS,
            &[
                ("lmt", "0"),
                ("klt", "101"),
                ("secid", "1.000001"),
                ("secid2", "0.399001"),
                ("fields1", "f1,f2,f3,f7"),
                (
                    "fields2",
                    "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f62,f63,f64,f65",
                ),
            ],
        )
        .await?;
    let klines = emh_klines(&v)?;
    Ok(parse_market_fund_flow(&klines))
}

// ---------------------------------------------------------------------------
// stock_individual_fund_flow_rank  (stock/stock_fund_em.py:122)
// ---------------------------------------------------------------------------

/// One row of the market-wide individual-stock fund-flow ranking.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndividualRankRow {
    /// 序号 (synthesized 1-based row index).
    pub rank: u32,
    /// 代码 (f12)
    pub code: String,
    /// 名称 (f14)
    pub name: String,
    /// 最新价 (f2)
    pub price: Option<f64>,
    /// 涨跌幅 (period: f3 / f127 / f109 / f160)
    pub change_pct: Option<f64>,
    /// 主力净流入-净额
    pub main_net_in: Option<f64>,
    /// 主力净流入-净占比
    pub main_net_pct: Option<f64>,
    /// 超大单净流入-净额
    pub xxl_net_in: Option<f64>,
    /// 超大单净流入-净占比
    pub xxl_net_pct: Option<f64>,
    /// 大单净流入-净额
    pub big_net_in: Option<f64>,
    /// 大单净流入-净占比
    pub big_net_pct: Option<f64>,
    /// 中单净流入-净额
    pub mid_net_in: Option<f64>,
    /// 中单净流入-净占比
    pub mid_net_pct: Option<f64>,
    /// 小单净流入-净额
    pub small_net_in: Option<f64>,
    /// 小单净流入-净占比
    pub small_net_pct: Option<f64>,
}

/// Per-indicator `push2` field keys for the 10 net-inflow/pct columns, in the
/// order `(main, main_pct, xxl, xxl_pct, big, big_pct, mid, mid_pct, small,
/// small_pct)`, plus the `change_pct` field and the `fid`.
fn rank_indicator_fields(indicator: &str) -> Option<(&'static str, [&'static str; 10], &'static str)> {
    let net = match indicator {
        "今日" => [
            "f62", "f184", "f66", "f69", "f72", "f75", "f78", "f81", "f84", "f87",
        ],
        "3日" => [
            "f267", "f268", "f269", "f270", "f271", "f272", "f273", "f274", "f275", "f276",
        ],
        "5日" => [
            "f164", "f165", "f166", "f167", "f168", "f169", "f170", "f171", "f172", "f173",
        ],
        "10日" => [
            "f174", "f175", "f176", "f177", "f178", "f179", "f180", "f181", "f182", "f183",
        ],
        _ => return None,
    };
    let (pct_field, fid) = match indicator {
        "今日" => ("f3", "f62"),
        "3日" => ("f127", "f267"),
        "5日" => ("f109", "f164"),
        "10日" => ("f160", "f174"),
        _ => unreachable!(),
    };
    Some((fid, net, pct_field))
}

/// Parse `stock_individual_fund_flow_rank` rows from a push2 clist `data.diff`.
pub(crate) fn parse_individual_rank(
    diff: &[Value],
    net_fields: &[&str; 10],
    pct_field: &str,
) -> Vec<IndividualRankRow> {
    let mut out = Vec::with_capacity(diff.len());
    for (i, item) in diff.iter().enumerate() {
        let Some(name) = opt_str(item, "f14") else {
            continue;
        };
        out.push(IndividualRankRow {
            rank: (i + 1) as u32,
            code: opt_str(item, "f12").unwrap_or_default(),
            name,
            price: opt_f64(item, "f2"),
            change_pct: opt_f64(item, pct_field),
            main_net_in: opt_f64(item, net_fields[0]),
            main_net_pct: opt_f64(item, net_fields[1]),
            xxl_net_in: opt_f64(item, net_fields[2]),
            xxl_net_pct: opt_f64(item, net_fields[3]),
            big_net_in: opt_f64(item, net_fields[4]),
            big_net_pct: opt_f64(item, net_fields[5]),
            mid_net_in: opt_f64(item, net_fields[6]),
            mid_net_pct: opt_f64(item, net_fields[7]),
            small_net_in: opt_f64(item, net_fields[8]),
            small_net_pct: opt_f64(item, net_fields[9]),
        });
    }
    out
}

/// 东方财富-个股资金流-排名 (push2 clist). `indicator` ∈ {今日, 3日, 5日, 10日};
/// defaults to `5日`.
pub async fn stock_individual_fund_flow_rank(
    client: &Client,
    indicator: &str,
) -> Result<Vec<IndividualRankRow>> {
    let (fid, net_fields, pct_field) = rank_indicator_fields(indicator).ok_or_else(|| {
        Error::InvalidParam(format!(
            "stock_individual_fund_flow_rank: unknown indicator {indicator:?} (use 今日/3日/5日/10日)"
        ))
    })?;
    let fields = format!("f12,f14,f2,{pct_field},{}", net_fields.join(","));
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_individual_fund_flow_rank", &crate::core::eastmoney_push::push2_url("/api/qt/clist/get").await,
            &[
                ("pn", "1"),
                ("pz", "100"),
                ("po", "1"),
                ("np", "1"),
                ("ut", UT),
                ("fltt", "2"),
                ("invt", "2"),
                ("fid", fid),
                (
                    "fs",
                    "m:0+t:6+f:!2,m:0+t:13+f:!2,m:0+t:80+f:!2,m:1+t:2+f:!2,m:1+t:23+f:!2,m:0+t:7+f:!2,m:1+t:3+f:!2",
                ),
                ("fields", &fields),
            ],
        )
        .await?;
    let diff = emh_diff(&v)?;
    Ok(parse_individual_rank(diff, &net_fields, pct_field))
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

    /// Extract `data.klines` from a daykline fixture as `Vec<String>`.
    fn klines_of(name: &str) -> Vec<String> {
        emh_klines(&fixture(name)).unwrap()
    }

    /// Extract `data.diff` from a push2 clist fixture.
    fn diff_of(name: &str) -> Vec<Value> {
        emh_diff(&fixture(name)).unwrap().clone()
    }

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    // ---- concept / sector hist (shared FundFlowHistRow) ----

    #[test]
    fn parse_concept_fund_flow_hist_ok() {
        let klines = klines_of("stock_concept_fund_flow_hist.json");
        let rows = parse_concept_fund_flow_hist(&klines);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].date, "2024-01-02");
        assert!(approx(rows[0].main_net_in, 123456789.0));
        assert!(approx(rows[0].xxl_net_pct, 0.45));
        assert!(approx(rows[1].small_net_in, -2000000.0));
        assert!(approx(rows[2].big_net_pct, 0.10));
    }

    #[test]
    fn parse_sector_fund_flow_hist_ok() {
        let klines = klines_of("stock_sector_fund_flow_hist.json");
        let rows = parse_sector_fund_flow_hist(&klines);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].date, "2024-01-02");
        assert!(approx(rows[0].main_net_in, 987654321.0));
        assert!(approx(rows[2].mid_net_pct, 0.05));
    }

    // ---- name → code maps ----

    #[test]
    fn parse_sector_name_code_map_ok() {
        let map = parse_name_code_map(&diff_of("stock_sector_name_code_map.json"));
        assert_eq!(map.get("电源设备"), Some(&"BK1034".to_string()));
        assert_eq!(map.get("汽车服务"), Some(&"BK0731".to_string()));
    }

    #[test]
    fn parse_concept_name_code_map_ok() {
        let map = parse_name_code_map(&diff_of("stock_concept_name_code_map.json"));
        assert_eq!(map.get("数据要素"), Some(&"BK0574".to_string()));
    }

    // ---- main fund flow ----

    #[test]
    fn parse_main_fund_flow_ok() {
        let rows = parse_main_fund_flow(&diff_of("stock_main_fund_flow.json"));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].seq, 1);
        assert_eq!(rows[0].code, "600519");
        assert_eq!(rows[0].name, "贵州茅台");
        assert!(approx(rows[0].price, 1685.0));
        assert!(approx(rows[0].main_pct_today, 2.31));
        assert!(approx(rows[1].change_10d, -3.45));
        assert_eq!(rows[1].sector, Some("白酒".to_string()));
    }

    // ---- sector fund flow rank ----

    #[test]
    fn parse_sector_fund_flow_rank_today() {
        let rows = parse_sector_fund_flow_rank(
            &diff_of("stock_sector_fund_flow_rank.json"),
            FundFlowPeriod::Today,
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "半导体");
        assert!(approx(rows[0].main_net_in, 5500000000.0));
        assert!(approx(rows[0].change_pct, 3.21));
        assert_eq!(rows[0].top_stock, Some("中芯国际".to_string()));
        assert!(approx(rows[1].small_net_pct, 1.10));
    }

    #[test]
    fn parse_sector_fund_flow_rank_5d() {
        let rows = parse_sector_fund_flow_rank(
            &diff_of("stock_sector_fund_flow_rank.json"),
            FundFlowPeriod::FiveDay,
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "半导体");
        assert!(approx(rows[0].main_net_in, 22000000000.0));
        assert!(approx(rows[0].change_pct, 8.50));
    }

    #[test]
    fn parse_sector_fund_flow_rank_10d() {
        let rows = parse_sector_fund_flow_rank(
            &diff_of("stock_sector_fund_flow_rank.json"),
            FundFlowPeriod::TenDay,
        );
        assert_eq!(rows.len(), 2);
        assert!(approx(rows[0].main_net_in, 41000000000.0));
        assert!(approx(rows[0].big_net_pct, 0.55));
    }

    // ---- sector fund flow summary ----

    #[test]
    fn parse_sector_fund_flow_summary_today() {
        let rows = parse_sector_fund_flow_summary(
            &diff_of("stock_sector_fund_flow_summary.json"),
            FundFlowPeriod::Today,
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].seq, 1);
        assert_eq!(rows[0].code, "300750");
        assert_eq!(rows[0].name, "宁德时代");
        assert!(approx(rows[0].price, 182.30));
        assert!(approx(rows[0].main_net_in, 1234567890.0));
        assert!(approx(rows[1].small_net_pct, 4.20));
    }

    #[test]
    fn parse_sector_fund_flow_summary_5d() {
        let rows = parse_sector_fund_flow_summary(
            &diff_of("stock_sector_fund_flow_summary.json"),
            FundFlowPeriod::FiveDay,
        );
        assert_eq!(rows.len(), 2);
        assert!(approx(rows[0].main_net_in, 5000000000.0));
        assert!(approx(rows[0].change_pct, 6.70));
    }

    // ---- sina sector detail ----

    #[test]
    fn parse_sector_detail_ok() {
        let arr = fixture("stock_sector_detail.json")
            .as_array()
            .unwrap()
            .clone();
        let rows = parse_sector_detail(&arr);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "sh600519");
        assert_eq!(rows[0].code, "600519");
        assert_eq!(rows[0].name, "贵州茅台");
        assert!(approx(rows[0].trade, 1685.0));
        assert!(approx(rows[0].change_percent, 1.23));
        assert!(approx(rows[0].volume, 3500000.0));
        assert!(approx(rows[1].mkt_cap, 2000000000000.0));
    }

    // ---- individual fund flow (Eastmoney daykline) ----

    #[test]
    fn parse_individual_fund_flow_ok() {
        let klines = klines_of("stock_individual_fund_flow.json");
        let rows = parse_fflow_klines(&klines);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].date, "2024-01-02");
        assert!(approx(rows[0].main_net_in, 123456789.0));
        assert!(approx(rows[0].small_net_in, -2000000.0));
        assert!(approx(rows[0].xxl_net_pct, 0.07));
        assert!(approx(rows[2].main_net_in, 0.0));
    }

    // ---- market fund flow (Eastmoney daykline, secid=1.000001 / 0.399001) ----

    #[test]
    fn parse_market_fund_flow_ok() {
        let klines = klines_of("stock_market_fund_flow.json");
        let rows = parse_market_fund_flow(&klines);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02");
        assert!(approx(rows[0].main_net_in, 15000000000.0));
        assert!(approx(rows[0].xxl_net_in, 3000000000.0));
        assert!(approx(rows[0].main_net_pct, 0.42));
        assert!(approx(rows[0].sh_close, 3000.12));
        assert!(approx(rows[0].sh_pct, 0.83));
        assert!(approx(rows[0].sz_close, 9500.34));
        assert!(approx(rows[0].sz_pct, 1.21));
        assert!(approx(rows[1].small_net_in, 2000000000.0));
    }

    // ---- individual fund flow rank (Eastmoney push2 clist) ----

    #[test]
    fn parse_individual_fund_flow_rank_ok() {
        let diff = diff_of("stock_individual_fund_flow_rank.json");
        let (_, net_fields, pct_field) = rank_indicator_fields("今日").unwrap();
        let rows = parse_individual_rank(&diff, &net_fields, pct_field);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].rank, 1);
        assert_eq!(rows[0].code, "600519");
        assert_eq!(rows[0].name, "贵州茅台");
        assert!(approx(rows[0].price, 1685.0));
        assert!(approx(rows[0].change_pct, 1.23));
        assert!(approx(rows[0].main_net_in, 123456789.0));
        assert!(approx(rows[0].main_net_pct, 0.45));
        assert!(approx(rows[0].xxl_net_in, 80000000.0));
        assert!(approx(rows[0].xxl_net_pct, 0.30));
        assert!(approx(rows[0].big_net_in, 43456789.0));
        assert!(approx(rows[0].big_net_pct, 0.15));
        assert!(approx(rows[0].mid_net_in, -12345678.0));
        assert!(approx(rows[0].mid_net_pct, -0.05));
        assert!(approx(rows[0].small_net_in, -98765432.0));
        assert!(approx(rows[0].small_net_pct, -0.40));
        assert_eq!(rows[2].name, "宁德时代");
        assert!(approx(rows[2].main_net_in, 200000000.0));
    }

    #[test]
    fn rank_indicator_fields_all_known() {
        for ind in ["今日", "3日", "5日", "10日"] {
            assert!(rank_indicator_fields(ind).is_some(), "indicator {ind} should be supported");
        }
        assert!(rank_indicator_fields("bad").is_none());
    }
}
