//! Eastmoney `push2` / `push2his` realtime spot and intraday-kline ports from
//! akshare `stock_feature/stock_hist_em.py`.
//!
//! Every function below is PURE HTTP against Eastmoney's push2 JSON API —
//! no JS sandbox (`py_mini_racer`), token, or signed-auth is required, so all
//! 13 requested functions are ported. Spot/realtime lists use
//! `push2.eastmoney.com/api/qt/clist/get` (envelope `data.diff`, an array of
//! field objects), while intraday klines use `push2*.eastmoney.com/api/qt/...`
//! (`data.trends` for `trends2/get`, `data.klines` for `stock/kline/get`; each
//! entry is a comma-separated OHLCV string).
//!
//! Field mapping follows Eastmoney field semantics (e.g. `f12` = code,
//! `f14` = name, `f2` = latest price) rather than akshare's positional
//! `columns` list, which is more robust to upstream column reshuffles.
//!
//! ## Ported functions (Rust fn -> akshare source line)
//!
//! | Rust fn | akshare line | endpoint / shape |
//! | --- | --- | --- |
//! | `stock_zh_a_spot_em` | stock_hist_em.py:15 | clist `data.diff` |
//! | `stock_sh_a_spot_em` | stock_hist_em.py:124 | clist `data.diff` |
//! | `stock_sz_a_spot_em` | stock_hist_em.py:232 | clist `data.diff` |
//! | `stock_bj_a_spot_em` | stock_hist_em.py:340 | clist `data.diff` |
//! | `stock_new_a_spot_em` | stock_hist_em.py:448 | clist `data.diff` |
//! | `stock_cy_a_spot_em` | stock_hist_em.py:561 | clist `data.diff` |
//! | `stock_kc_a_spot_em` | stock_hist_em.py:670 | clist `data.diff` |
//! | `stock_zh_ab_comparison_em` | stock_hist_em.py:779 | clist `data.diff` |
//! | `stock_zh_b_spot_em` | stock_hist_em.py:844 | clist `data.diff` |
//! | `stock_hk_main_board_spot_em` | stock_hist_em.py:1310 | clist `data.diff` |
//! | `stock_zh_a_hist_pre_min_em` | stock_hist_em.py:1170 | trends2 `data.trends` |
//! | `stock_hk_hist_min_em` | stock_hist_em.py:1467 | trends2/kline `data.trends`/`klines` |
//! | `stock_us_hist_min_em` | stock_hist_em.py:1758 | trends2 `data.trends` |
//!
//! ## DEFERRED (not ported)
//!
//! none

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use crate::core::json::*;

const SOURCE_EASTMONEY: &str = "eastmoney";
const UT: &str = "bd1d9ddb04089700cf9c27f6f7426281";
const BASE_CLIST: &str = "https://push2.eastmoney.com/api/qt/clist/get";
const BASE_TRENDS2: &str = "https://push2.eastmoney.com/api/qt/stock/trends2/get";
const BASE_KLINE: &str = "https://push2his.eastmoney.com/api/qt/stock/kline/get";

/// Standard 29-field set used by the A-share / B-share / HK-main-board spots.
const FIELDS_STD: &str = "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f12,f13,f14,f15,f16,f17,f18,f20,f21,f23,f24,f25,f22,f11,f62,f128,f136,f115,f152";
/// New-share spot field set (adds `f26` = listing date).
const FIELDS_NEW: &str = "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f12,f13,f14,f15,f16,f17,f18,f20,f21,f23,f24,f25,f26,f22,f11,f62,f128,f136,f115,f152";
/// AB-comparison field set (fltt=1 → raw values, divided by 100 downstream).
const FIELDS_AB: &str = "f201,f202,f203,f196,f200,f197,f152,f12,f13,f14,f1,f2,f4,f3,f199";

const FS_ZH_A: &str = "m:0 t:6,m:0 t:80,m:1 t:2,m:1 t:23,m:0 t:81 s:2048";
const FS_SH: &str = "m:1 t:2,m:1 t:23";
const FS_SZ: &str = "m:0 t:6,m:0 t:80";
const FS_BJ: &str = "m:0 t:81 s:2048";
const FS_NEW: &str = "m:0 f:8,m:1 f:8";
const FS_CY: &str = "m:0 t:80";
const FS_KC: &str = "m:1 t:23";
const FS_ZH_B: &str = "m:0 t:7,m:1 t:3";
const FS_HK_MAIN: &str = "m:128 t:3";
const FS_AB: &str = "m:1+b:BK0498,m:0+b:BK0498";


/// Extract the `data.diff` array from a clist response.
fn diff_array(resp: &Value) -> Result<Vec<Value>> {
    resp.get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|d| d.as_array())
        .cloned()
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff".into(),
        })
}

/// Extract the `data.trends` array from a trends2 response.
fn trend_array(resp: &Value) -> Result<Vec<Value>> {
    resp.get("data")
        .and_then(|d| d.get("trends"))
        .and_then(|d| d.as_array())
        .cloned()
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.trends".into(),
        })
}

/// Extract the `data.klines` array from a stock/kline response.
fn kline_array(resp: &Value) -> Result<Vec<Value>> {
    resp.get("data")
        .and_then(|d| d.get("klines"))
        .and_then(|d| d.as_array())
        .cloned()
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.klines".into(),
        })
}

/// Standard clist query params (single page) for the A/B/HK-main spots.
fn clist_base<'a>(fs: &'a str, fields: &'a str) -> Vec<(&'a str, &'a str)> {
    vec![
        ("pn", "1"),
        ("pz", "100"),
        ("po", "1"),
        ("np", "1"),
        ("ut", UT),
        ("fltt", "2"),
        ("invt", "2"),
        ("fid", "f12"),
        ("fs", fs),
        ("fields", fields),
    ]
}

/// Common A-share realtime spot row (23 output columns).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ZhASpotEmRow {
    /// 序号 (Eastmoney `f1`)
    pub serial: Option<f64>,
    /// 代码 (Eastmoney `f12`)
    pub code: Option<String>,
    /// 名称 (Eastmoney `f14`)
    pub name: Option<String>,
    /// 最新价 (Eastmoney `f2`)
    pub price: Option<f64>,
    /// 涨跌幅 (Eastmoney `f3`)
    pub pct_change: Option<f64>,
    /// 涨跌额 (Eastmoney `f4`)
    pub change: Option<f64>,
    /// 成交量 (Eastmoney `f5`)
    pub volume: Option<f64>,
    /// 成交额 (Eastmoney `f6`)
    pub amount: Option<f64>,
    /// 振幅 (Eastmoney `f7`)
    pub amplitude: Option<f64>,
    /// 换手率 (Eastmoney `f8`)
    pub turnover_rate: Option<f64>,
    /// 市盈率-动态 (Eastmoney `f9`)
    pub pe: Option<f64>,
    /// 量比 (Eastmoney `f10`)
    pub volume_ratio: Option<f64>,
    /// 5分钟涨跌 (Eastmoney `f11`)
    pub five_min_change: Option<f64>,
    /// 最高 (Eastmoney `f15`)
    pub high: Option<f64>,
    /// 最低 (Eastmoney `f16`)
    pub low: Option<f64>,
    /// 今开 (Eastmoney `f17`)
    pub open: Option<f64>,
    /// 昨收 (Eastmoney `f18`)
    pub pre_close: Option<f64>,
    /// 总市值 (Eastmoney `f20`)
    pub total_mv: Option<f64>,
    /// 流通市值 (Eastmoney `f21`)
    pub float_mv: Option<f64>,
    /// 涨速 (Eastmoney `f22`)
    pub speed: Option<f64>,
    /// 市净率 (Eastmoney `f23`)
    pub pb: Option<f64>,
    /// 60日涨跌幅 (Eastmoney `f24`)
    pub pct_60d: Option<f64>,
    /// 年初至今涨跌幅 (Eastmoney `f25`)
    pub pct_ytd: Option<f64>,
}

/// Shared parser for the standard 23-column spot layout.
fn parse_spot_standard(items: &[Value]) -> Result<Vec<ZhASpotEmRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(ZhASpotEmRow {
            serial: opt_f64(item, "f1"),
            code: opt_str(item, "f12"),
            name: opt_str(item, "f14"),
            price: opt_f64(item, "f2"),
            pct_change: opt_f64(item, "f3"),
            change: opt_f64(item, "f4"),
            volume: opt_f64(item, "f5"),
            amount: opt_f64(item, "f6"),
            amplitude: opt_f64(item, "f7"),
            turnover_rate: opt_f64(item, "f8"),
            pe: opt_f64(item, "f9"),
            volume_ratio: opt_f64(item, "f10"),
            five_min_change: opt_f64(item, "f11"),
            high: opt_f64(item, "f15"),
            low: opt_f64(item, "f16"),
            open: opt_f64(item, "f17"),
            pre_close: opt_f64(item, "f18"),
            total_mv: opt_f64(item, "f20"),
            float_mv: opt_f64(item, "f21"),
            speed: opt_f64(item, "f22"),
            pb: opt_f64(item, "f23"),
            pct_60d: opt_f64(item, "f24"),
            pct_ytd: opt_f64(item, "f25"),
        });
    }
    Ok(out)
}

/// New-share realtime spot row (24 columns; adds listing date).
#[derive(Debug, Clone, serde::Serialize)]
pub struct NewASpotEmRow {
    /// 序号 (Eastmoney `f1`)
    pub serial: Option<f64>,
    /// 代码 (Eastmoney `f12`)
    pub code: Option<String>,
    /// 名称 (Eastmoney `f14`)
    pub name: Option<String>,
    /// 最新价 (Eastmoney `f2`)
    pub price: Option<f64>,
    /// 涨跌幅 (Eastmoney `f3`)
    pub pct_change: Option<f64>,
    /// 涨跌额 (Eastmoney `f4`)
    pub change: Option<f64>,
    /// 成交量 (Eastmoney `f5`)
    pub volume: Option<f64>,
    /// 成交额 (Eastmoney `f6`)
    pub amount: Option<f64>,
    /// 振幅 (Eastmoney `f7`)
    pub amplitude: Option<f64>,
    /// 最高 (Eastmoney `f15`)
    pub high: Option<f64>,
    /// 最低 (Eastmoney `f16`)
    pub low: Option<f64>,
    /// 今开 (Eastmoney `f17`)
    pub open: Option<f64>,
    /// 昨收 (Eastmoney `f18`)
    pub pre_close: Option<f64>,
    /// 量比 (Eastmoney `f10`)
    pub volume_ratio: Option<f64>,
    /// 换手率 (Eastmoney `f8`)
    pub turnover_rate: Option<f64>,
    /// 市盈率-动态 (Eastmoney `f9`)
    pub pe: Option<f64>,
    /// 市净率 (Eastmoney `f23`)
    pub pb: Option<f64>,
    /// 上市日期 (Eastmoney `f26`, YYYYMMDD)
    pub listing_date: Option<String>,
    /// 总市值 (Eastmoney `f20`)
    pub total_mv: Option<f64>,
    /// 流通市值 (Eastmoney `f21`)
    pub float_mv: Option<f64>,
    /// 涨速 (Eastmoney `f22`)
    pub speed: Option<f64>,
    /// 5分钟涨跌 (Eastmoney `f11`)
    pub five_min_change: Option<f64>,
    /// 60日涨跌幅 (Eastmoney `f24`)
    pub pct_60d: Option<f64>,
    /// 年初至今涨跌幅 (Eastmoney `f25`)
    pub pct_ytd: Option<f64>,
}

fn parse_spot_new_a(items: &[Value]) -> Result<Vec<NewASpotEmRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(NewASpotEmRow {
            serial: opt_f64(item, "f1"),
            code: opt_str(item, "f12"),
            name: opt_str(item, "f14"),
            price: opt_f64(item, "f2"),
            pct_change: opt_f64(item, "f3"),
            change: opt_f64(item, "f4"),
            volume: opt_f64(item, "f5"),
            amount: opt_f64(item, "f6"),
            amplitude: opt_f64(item, "f7"),
            high: opt_f64(item, "f15"),
            low: opt_f64(item, "f16"),
            open: opt_f64(item, "f17"),
            pre_close: opt_f64(item, "f18"),
            volume_ratio: opt_f64(item, "f10"),
            turnover_rate: opt_f64(item, "f8"),
            pe: opt_f64(item, "f9"),
            pb: opt_f64(item, "f23"),
            listing_date: opt_str(item, "f26"),
            total_mv: opt_f64(item, "f20"),
            float_mv: opt_f64(item, "f21"),
            speed: opt_f64(item, "f22"),
            five_min_change: opt_f64(item, "f11"),
            pct_60d: opt_f64(item, "f24"),
            pct_ytd: opt_f64(item, "f25"),
        });
    }
    Ok(out)
}

/// HK main-board realtime spot row (12 columns).
#[derive(Debug, Clone, serde::Serialize)]
pub struct HkMainBoardSpotEmRow {
    /// 序号 (Eastmoney `f1`)
    pub serial: Option<f64>,
    /// 代码 (Eastmoney `f12`)
    pub code: Option<String>,
    /// 名称 (Eastmoney `f14`)
    pub name: Option<String>,
    /// 最新价 (Eastmoney `f2`)
    pub price: Option<f64>,
    /// 涨跌额 (Eastmoney `f4`)
    pub change: Option<f64>,
    /// 涨跌幅 (Eastmoney `f3`)
    pub pct_change: Option<f64>,
    /// 今开 (Eastmoney `f17`)
    pub open: Option<f64>,
    /// 最高 (Eastmoney `f15`)
    pub high: Option<f64>,
    /// 最低 (Eastmoney `f16`)
    pub low: Option<f64>,
    /// 昨收 (Eastmoney `f18`)
    pub pre_close: Option<f64>,
    /// 成交量 (Eastmoney `f5`)
    pub volume: Option<f64>,
    /// 成交额 (Eastmoney `f6`)
    pub amount: Option<f64>,
}

fn parse_spot_hk_main(items: &[Value]) -> Result<Vec<HkMainBoardSpotEmRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(HkMainBoardSpotEmRow {
            serial: opt_f64(item, "f1"),
            code: opt_str(item, "f12"),
            name: opt_str(item, "f14"),
            price: opt_f64(item, "f2"),
            change: opt_f64(item, "f4"),
            pct_change: opt_f64(item, "f3"),
            open: opt_f64(item, "f17"),
            high: opt_f64(item, "f15"),
            low: opt_f64(item, "f16"),
            pre_close: opt_f64(item, "f18"),
            volume: opt_f64(item, "f5"),
            amount: opt_f64(item, "f6"),
        });
    }
    Ok(out)
}

/// AB-share comparison (比价) row. With `fltt=1` upstream returns raw values
/// (price × 100), so prices / percentages / ratio are divided by 100.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ZhAbComparisonRow {
    /// 序号 (dataframe index)
    pub serial: Option<f64>,
    /// B股代码 (Eastmoney `f201`)
    pub b_code: Option<String>,
    /// B股名称 (Eastmoney `f203`)
    pub b_name: Option<String>,
    /// 最新价B (Eastmoney `f2` / 100)
    pub price_b: Option<f64>,
    /// 涨跌幅B (Eastmoney `f3` / 100)
    pub pct_change_b: Option<f64>,
    /// A股代码 (Eastmoney `f12`)
    pub a_code: Option<String>,
    /// A股名称 (Eastmoney `f14`)
    pub a_name: Option<String>,
    /// 最新价A (Eastmoney `f196` / 100)
    pub price_a: Option<f64>,
    /// 涨跌幅A (Eastmoney `f197` / 100)
    pub pct_change_a: Option<f64>,
    /// 比价 (Eastmoney `f199` / 100)
    pub ratio: Option<f64>,
}

fn parse_ab_comparison(items: &[Value]) -> Result<Vec<ZhAbComparisonRow>> {
    let scale = |v: Option<f64>| v.map(|x| x / 100.0);
    let mut out = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        out.push(ZhAbComparisonRow {
            serial: Some(i as f64 + 1.0),
            b_code: opt_str(item, "f201"),
            b_name: opt_str(item, "f203"),
            price_b: scale(opt_f64(item, "f2")),
            pct_change_b: scale(opt_f64(item, "f3")),
            a_code: opt_str(item, "f12"),
            a_name: opt_str(item, "f14"),
            price_a: scale(opt_f64(item, "f196")),
            pct_change_a: scale(opt_f64(item, "f197")),
            ratio: scale(opt_f64(item, "f199")),
        });
    }
    Ok(out)
}

/// Intraday trend/kline row for `trends2/get` (8 fields: time, O, C, H, L,
/// volume, amount, latest price). Shared by `stock_zh_a_hist_pre_min_em`,
/// `stock_us_hist_min_em`, and `stock_hk_hist_min_em` with `period == "1"`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TrendMinRow {
    /// 时间
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
    /// 成交额
    pub amount: Option<f64>,
    /// 最新价
    pub latest_price: Option<f64>,
}

fn parse_trend(items: &[Value]) -> Result<Vec<TrendMinRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(s) = item.as_str() else {
            continue;
        };
        let p: Vec<&str> = s.split(',').collect();
        if p.len() < 8 {
            continue;
        }
        out.push(TrendMinRow {
            date: p[0].to_string(),
            open: p[1].parse().ok(),
            close: p[2].parse().ok(),
            high: p[3].parse().ok(),
            low: p[4].parse().ok(),
            volume: p[5].parse().ok(),
            amount: p[6].parse().ok(),
            latest_price: p[7].parse().ok(),
        });
    }
    Ok(out)
}

/// Intraday kline row for `stock/kline/get` (11 fields). Used by
/// `stock_hk_hist_min_em` with `period != "1"`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct KlineMinRow {
    /// 时间
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
    /// 成交额
    pub amount: Option<f64>,
    /// 振幅
    pub amplitude: Option<f64>,
    /// 涨跌幅
    pub pct_change: Option<f64>,
    /// 涨跌额
    pub change: Option<f64>,
    /// 换手率
    pub turnover_rate: Option<f64>,
}

fn parse_kline(items: &[Value]) -> Result<Vec<KlineMinRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(s) = item.as_str() else {
            continue;
        };
        let p: Vec<&str> = s.split(',').collect();
        if p.len() < 11 {
            continue;
        }
        out.push(KlineMinRow {
            date: p[0].to_string(),
            open: p[1].parse().ok(),
            close: p[2].parse().ok(),
            high: p[3].parse().ok(),
            low: p[4].parse().ok(),
            volume: p[5].parse().ok(),
            amount: p[6].parse().ok(),
            amplitude: p[7].parse().ok(),
            pct_change: p[8].parse().ok(),
            change: p[9].parse().ok(),
            turnover_rate: p[10].parse().ok(),
        });
    }
    Ok(out)
}

/// CN secid helper: `1.symbol` for SH (6xxxxx), else `0.symbol`.
fn cn_secid(symbol: &str) -> String {
    let market = if symbol.starts_with('6') { 1 } else { 0 };
    format!("{market}.{symbol}")
}

/// Map akshare adjust arg to Eastmoney `fqt` value.
fn fqt(adjust: &str) -> &'static str {
    match adjust {
        "qfq" => "1",
        "hfq" => "2",
        _ => "0",
    }
}

// ===========================================================================
// Spot / realtime clist ports
// ===========================================================================

/// 东方财富网-沪深京 A 股-实时行情 (`stock_zh_a_spot_em`, stock_hist_em.py:15).
pub async fn stock_zh_a_spot_em(client: &Client) -> Result<Vec<ZhASpotEmRow>> {
    let params = clist_base(FS_ZH_A, FIELDS_STD);
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_zh_a_spot_em", BASE_CLIST, &params)
        .await?;
    parse_zh_a_spot_em(&diff_array(&v)?)
}

/// 东方财富网-沪 A 股-实时行情 (`stock_sh_a_spot_em`, stock_hist_em.py:124).
pub async fn stock_sh_a_spot_em(client: &Client) -> Result<Vec<ZhASpotEmRow>> {
    let params = clist_base(FS_SH, FIELDS_STD);
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_sh_a_spot_em", BASE_CLIST, &params)
        .await?;
    parse_sh_a_spot_em(&diff_array(&v)?)
}

/// 东方财富网-深 A 股-实时行情 (`stock_sz_a_spot_em`, stock_hist_em.py:232).
pub async fn stock_sz_a_spot_em(client: &Client) -> Result<Vec<ZhASpotEmRow>> {
    let params = clist_base(FS_SZ, FIELDS_STD);
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_sz_a_spot_em", BASE_CLIST, &params)
        .await?;
    parse_sz_a_spot_em(&diff_array(&v)?)
}

/// 东方财富网-京 A 股-实时行情 (`stock_bj_a_spot_em`, stock_hist_em.py:340).
pub async fn stock_bj_a_spot_em(client: &Client) -> Result<Vec<ZhASpotEmRow>> {
    let params = clist_base(FS_BJ, FIELDS_STD);
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_bj_a_spot_em", BASE_CLIST, &params)
        .await?;
    parse_bj_a_spot_em(&diff_array(&v)?)
}

/// 东方财富网-创业板-实时行情 (`stock_cy_a_spot_em`, stock_hist_em.py:561).
pub async fn stock_cy_a_spot_em(client: &Client) -> Result<Vec<ZhASpotEmRow>> {
    let params = clist_base(FS_CY, FIELDS_STD);
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_cy_a_spot_em", BASE_CLIST, &params)
        .await?;
    parse_cy_a_spot_em(&diff_array(&v)?)
}

/// 东方财富网-科创板-实时行情 (`stock_kc_a_spot_em`, stock_hist_em.py:670).
pub async fn stock_kc_a_spot_em(client: &Client) -> Result<Vec<ZhASpotEmRow>> {
    let params = clist_base(FS_KC, FIELDS_STD);
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_kc_a_spot_em", BASE_CLIST, &params)
        .await?;
    parse_kc_a_spot_em(&diff_array(&v)?)
}

/// 东方财富网- B 股-实时行情 (`stock_zh_b_spot_em`, stock_hist_em.py:844).
pub async fn stock_zh_b_spot_em(client: &Client) -> Result<Vec<ZhASpotEmRow>> {
    let params = clist_base(FS_ZH_B, FIELDS_STD);
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_zh_b_spot_em", BASE_CLIST, &params)
        .await?;
    parse_zh_b_spot_em(&diff_array(&v)?)
}

/// 东方财富网-新股-实时行情 (`stock_new_a_spot_em`, stock_hist_em.py:448).
pub async fn stock_new_a_spot_em(client: &Client) -> Result<Vec<NewASpotEmRow>> {
    let mut params = clist_base(FS_NEW, FIELDS_NEW);
    params.push(("wbp2u", "|0|0|0|web"));
    params[7] = ("fid", "f26");
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_new_a_spot_em", BASE_CLIST, &params)
        .await?;
    parse_new_a_spot_em(&diff_array(&v)?)
}

/// 东方财富网-AB股比价 (`stock_zh_ab_comparison_em`, stock_hist_em.py:779).
pub async fn stock_zh_ab_comparison_em(client: &Client) -> Result<Vec<ZhAbComparisonRow>> {
    let params = [
        ("np", "1"),
        ("fltt", "1"),
        ("invt", "2"),
        ("fs", FS_AB),
        ("fields", FIELDS_AB),
        ("fid", "f199"),
        ("pn", "1"),
        ("pz", "100"),
        ("po", "1"),
        ("dect", "1"),
        ("wbp2u", "|0|0|0|web"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_zh_ab_comparison_em",
            BASE_CLIST,
            &params,
        )
        .await?;
    parse_zh_ab_comparison_em(&diff_array(&v)?)
}

/// 东方财富网-港股-主板-实时行情 (`stock_hk_main_board_spot_em`, stock_hist_em.py:1310).
pub async fn stock_hk_main_board_spot_em(client: &Client) -> Result<Vec<HkMainBoardSpotEmRow>> {
    let params = clist_base(FS_HK_MAIN, FIELDS_STD);
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_hk_main_board_spot_em",
            BASE_CLIST,
            &params,
        )
        .await?;
    parse_hk_main_board_spot_em(&diff_array(&v)?)
}

// ===========================================================================
// Intraday kline / trend ports
// ===========================================================================

/// 东方财富网-沪深京 A 股-每日分时行情(含盘前) (`stock_zh_a_hist_pre_min_em`,
/// stock_hist_em.py:1170). `symbol` is a 6-digit A-share code (e.g. `000001`).
pub async fn stock_zh_a_hist_pre_min_em(client: &Client, symbol: &str) -> Result<Vec<TrendMinRow>> {
    let secid = cn_secid(symbol);
    let params = [
        ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f11,f12,f13"),
        ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58"),
        ("ndays", "1"),
        ("iscr", "1"),
        ("iscca", "0"),
        ("secid", secid.as_str()),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_zh_a_hist_pre_min_em",
            BASE_TRENDS2,
            &params,
        )
        .await?;
    parse_zh_a_hist_pre_min_em(&trend_array(&v)?)
}

/// 东方财富网-港股-每日分时行情 (`stock_hk_hist_min_em`, stock_hist_em.py:1467).
/// With `period == "1"` uses `trends2/get`; otherwise uses `stock/kline/get`.
/// `symbol` is a HK code without market prefix (e.g. `01611`).
pub async fn stock_hk_hist_min_em(
    client: &Client,
    symbol: &str,
    period: &str,
    adjust: &str,
) -> Result<Vec<KlineMinRow>> {
    let secid = format!("116.{symbol}");
    if period == "1" {
        let params = [
            ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f11,f12,f13"),
            ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58"),
            ("iscr", "0"),
            ("ndays", "5"),
            ("secid", secid.as_str()),
        ];
        let v = client
            .get_json(
                SOURCE_EASTMONEY,
                "stock_hk_hist_min_em",
                BASE_TRENDS2,
                &params,
            )
            .await?;
        parse_trend_as_kline(&trend_array(&v)?)
    } else {
        let params = [
            ("fields1", "f1,f2,f3,f4,f5,f6"),
            ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61"),
            ("ut", UT),
            ("klt", period),
            ("fqt", fqt(adjust)),
            ("secid", secid.as_str()),
            ("beg", "0"),
            ("end", "20500000"),
        ];
        let v = client
            .get_json(
                SOURCE_EASTMONEY,
                "stock_hk_hist_min_em",
                BASE_KLINE,
                &params,
            )
            .await?;
        parse_hk_hist_min_em_kline(&kline_array(&v)?)
    }
}

/// 东方财富网-美股-每日分时行情 (`stock_us_hist_min_em`, stock_hist_em.py:1758).
/// `symbol` is the Eastmoney US secid, e.g. `105.ATER`.
pub async fn stock_us_hist_min_em(client: &Client, symbol: &str) -> Result<Vec<TrendMinRow>> {
    let params = [
        ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f11,f12,f13"),
        ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58"),
        ("iscr", "0"),
        ("ndays", "5"),
        ("secid", symbol),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_us_hist_min_em",
            BASE_TRENDS2,
            &params,
        )
        .await?;
    parse_us_hist_min_em(&trend_array(&v)?)
}

// ===========================================================================
// Pure parse functions (for offline golden tests)
// ===========================================================================

/// Parse `data.diff` for the standard A/B/HK-main spot layout.
pub fn parse_zh_a_spot_em(items: &[Value]) -> Result<Vec<ZhASpotEmRow>> {
    parse_spot_standard(items)
}

/// Parse `data.diff` for SH A-share spot.
pub fn parse_sh_a_spot_em(items: &[Value]) -> Result<Vec<ZhASpotEmRow>> {
    parse_spot_standard(items)
}

/// Parse `data.diff` for SZ A-share spot.
pub fn parse_sz_a_spot_em(items: &[Value]) -> Result<Vec<ZhASpotEmRow>> {
    parse_spot_standard(items)
}

/// Parse `data.diff` for BJ A-share spot.
pub fn parse_bj_a_spot_em(items: &[Value]) -> Result<Vec<ZhASpotEmRow>> {
    parse_spot_standard(items)
}

/// Parse `data.diff` for ChiNext (创业板) spot.
pub fn parse_cy_a_spot_em(items: &[Value]) -> Result<Vec<ZhASpotEmRow>> {
    parse_spot_standard(items)
}

/// Parse `data.diff` for STAR (科创板) spot.
pub fn parse_kc_a_spot_em(items: &[Value]) -> Result<Vec<ZhASpotEmRow>> {
    parse_spot_standard(items)
}

/// Parse `data.diff` for B-share spot.
pub fn parse_zh_b_spot_em(items: &[Value]) -> Result<Vec<ZhASpotEmRow>> {
    parse_spot_standard(items)
}

/// Parse `data.diff` for new-share spot.
pub fn parse_new_a_spot_em(items: &[Value]) -> Result<Vec<NewASpotEmRow>> {
    parse_spot_new_a(items)
}

/// Parse `data.diff` for AB-share comparison.
pub fn parse_zh_ab_comparison_em(items: &[Value]) -> Result<Vec<ZhAbComparisonRow>> {
    parse_ab_comparison(items)
}

/// Parse `data.diff` for HK main-board spot.
pub fn parse_hk_main_board_spot_em(items: &[Value]) -> Result<Vec<HkMainBoardSpotEmRow>> {
    parse_spot_hk_main(items)
}

/// Parse `data.trends` for the A-share pre-market intraday trend.
pub fn parse_zh_a_hist_pre_min_em(items: &[Value]) -> Result<Vec<TrendMinRow>> {
    parse_trend(items)
}

/// Parse `data.trends` for the HK intraday trend (`period == "1"`).
pub fn parse_hk_hist_min_em_trend(items: &[Value]) -> Result<Vec<TrendMinRow>> {
    parse_trend(items)
}

/// Parse `data.klines` for the HK intraday kline (`period != "1"`).
pub fn parse_hk_hist_min_em_kline(items: &[Value]) -> Result<Vec<KlineMinRow>> {
    parse_kline(items)
}

/// Parse `data.trends` for the US intraday trend.
pub fn parse_us_hist_min_em(items: &[Value]) -> Result<Vec<TrendMinRow>> {
    parse_trend(items)
}

/// Adapt `data.trends` (8-field) entries into `KlineMinRow` (the extra kline-only
/// fields are left `None`). Used by `stock_hk_hist_min_em` with `period == "1"`,
/// so the function has a single uniform return type.
fn parse_trend_as_kline(items: &[Value]) -> Result<Vec<KlineMinRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(s) = item.as_str() else {
            continue;
        };
        let p: Vec<&str> = s.split(',').collect();
        if p.len() < 8 {
            continue;
        }
        out.push(KlineMinRow {
            date: p[0].to_string(),
            open: p[1].parse().ok(),
            close: p[2].parse().ok(),
            high: p[3].parse().ok(),
            low: p[4].parse().ok(),
            volume: p[5].parse().ok(),
            amount: p[6].parse().ok(),
            amplitude: None,
            pct_change: None,
            change: None,
            turnover_rate: None,
        });
    }
    Ok(out)
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

    /// Extract `data.diff` from a clist fixture.
    fn diff_of(name: &str) -> Vec<Value> {
        fixture(name)["data"]["diff"].as_array().unwrap().clone()
    }

    /// Extract `data.trends` from a trends2 fixture.
    fn trends_of(name: &str) -> Vec<Value> {
        fixture(name)["data"]["trends"].as_array().unwrap().clone()
    }

    /// Extract `data.klines` from a kline fixture.
    fn klines_of(name: &str) -> Vec<Value> {
        fixture(name)["data"]["klines"].as_array().unwrap().clone()
    }

    #[test]
    fn parses_stock_zh_a_spot_em() {
        let rows = parse_zh_a_spot_em(&diff_of("stock_hist_em_spot.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, Some("600000".to_string()));
        assert_eq!(rows[0].name, Some("浦发银行".to_string()));
        assert_eq!(rows[0].price, Some(13.45));
        assert_eq!(rows[0].pct_change, Some(2.30));
        assert_eq!(rows[0].pb, Some(0.7));
        assert_eq!(rows[1].code, Some("000001".to_string()));
        assert_eq!(rows[1].price, Some(-1.20));
        assert_eq!(rows[1].pct_change, Some(-3.10));
    }

    #[test]
    fn parses_stock_sh_a_spot_em() {
        let rows = parse_sh_a_spot_em(&diff_of("stock_hist_em_spot.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, Some("600000".to_string()));
        assert_eq!(rows[0].total_mv, Some(39500000.0));
    }

    #[test]
    fn parses_stock_sz_a_spot_em() {
        let rows = parse_sz_a_spot_em(&diff_of("stock_hist_em_spot.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].code, Some("000001".to_string()));
    }

    #[test]
    fn parses_stock_bj_a_spot_em() {
        let rows = parse_bj_a_spot_em(&diff_of("stock_hist_em_spot.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, Some("浦发银行".to_string()));
    }

    #[test]
    fn parses_stock_cy_a_spot_em() {
        let rows = parse_cy_a_spot_em(&diff_of("stock_hist_em_spot.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].high, Some(13.8));
    }

    #[test]
    fn parses_stock_kc_a_spot_em() {
        let rows = parse_kc_a_spot_em(&diff_of("stock_hist_em_spot.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].low, Some(13.1));
    }

    #[test]
    fn parses_stock_zh_b_spot_em() {
        let rows = parse_zh_b_spot_em(&diff_of("stock_hist_em_spot.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].open, Some(13.2));
    }

    #[test]
    fn parses_stock_new_a_spot_em() {
        let rows = parse_new_a_spot_em(&diff_of("stock_new_a_spot_em.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].code, Some("001234".to_string()));
        assert_eq!(rows[0].name, Some("新股测试".to_string()));
        assert_eq!(rows[0].listing_date, Some("20240101".to_string()));
        assert_eq!(rows[0].price, Some(12.34));
    }

    #[test]
    fn parses_stock_zh_ab_comparison_em() {
        let rows = parse_zh_ab_comparison_em(&diff_of("stock_zh_ab_comparison_em.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].b_code, Some("900901".to_string()));
        assert_eq!(rows[0].a_code, Some("600601".to_string()));
        // raw 1345 / 100
        assert_eq!(rows[0].price_b, Some(13.45));
        assert_eq!(rows[0].price_a, Some(23.45));
        assert_eq!(rows[0].ratio, Some(12.34));
    }

    #[test]
    fn parses_stock_hk_main_board_spot_em() {
        let rows =
            parse_hk_main_board_spot_em(&diff_of("stock_hk_main_board_spot_em.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].code, Some("00700".to_string()));
        assert_eq!(rows[0].name, Some("腾讯控股".to_string()));
        assert_eq!(rows[0].price, Some(372.80));
        assert_eq!(rows[0].pct_change, Some(1.42));
        assert_eq!(rows[0].amount, Some(4590000000.0));
    }

    #[test]
    fn parses_stock_zh_a_hist_pre_min_em() {
        let rows =
            parse_zh_a_hist_pre_min_em(&trends_of("stock_zh_a_hist_pre_min_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02 09:30");
        assert_eq!(rows[0].open, Some(1.0));
        assert_eq!(rows[0].close, Some(2.0));
        assert_eq!(rows[0].high, Some(0.9));
        assert_eq!(rows[0].low, Some(1.5));
        assert_eq!(rows[0].volume, Some(1000.0));
        assert_eq!(rows[0].amount, Some(2000.0));
        assert_eq!(rows[0].latest_price, Some(1.4));
        assert_eq!(rows[1].close, Some(2.1));
    }

    #[test]
    fn parses_stock_hk_hist_min_em_trend() {
        let rows =
            parse_hk_hist_min_em_trend(&trends_of("stock_hk_hist_min_em_trend.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02 09:30");
        assert_eq!(rows[0].close, Some(2.0));
        assert_eq!(rows[1].latest_price, Some(1.5));
    }

    #[test]
    fn parses_stock_hk_hist_min_em_kline() {
        let rows =
            parse_hk_hist_min_em_kline(&klines_of("stock_hk_hist_min_em_kline.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].date, "2024-01-02 09:30");
        assert_eq!(rows[0].amplitude, Some(3.0));
        assert_eq!(rows[0].pct_change, Some(1.2));
        assert_eq!(rows[0].change, Some(0.5));
        assert_eq!(rows[0].turnover_rate, Some(2.0));
    }

    #[test]
    fn parses_stock_us_hist_min_em() {
        let rows = parse_us_hist_min_em(&trends_of("stock_us_hist_min_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02 09:30");
        assert_eq!(rows[0].close, Some(2.0));
    }
}
