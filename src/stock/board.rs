//! 东方财富 / 同花顺 板块行情 (port of `akshare/stock/stock_board_concept_em.py`,
//! `akshare/stock/stock_board_industry_em.py`, `akshare/stock_feature/stock_board_concept_ths.py`,
//! `akshare/stock_feature/stock_board_industry_ths.py`).
//!
//! ## Ported functions (Eastmoney `push2` / `push2his` JSON API)
//!
//! These hit plain JSON endpoints (no HTML scraping, no JS-signed `ut` on the
//! data path — the static `ut` token used only for name→code resolution is a
//! hardcoded magic string in akshare, not a per-request JS signature):
//!
//! | Rust function | akshare source | endpoint |
//! | --- | --- | --- |
//! | `stock_board_concept_hist_em` | `stock_board_concept_em.py:181` | `push2his/.../kline/get` |
//! | `stock_board_concept_hist_min_em` | `stock_board_concept_em.py:273` | `push2his/.../kline/get` (period ∈ {5,15,30,60}) |
//! | `stock_board_concept_hist_min_em_trends` | `stock_board_concept_em.py:273` | `push2his/.../trends2/get` (period = "1") |
//! | `stock_board_concept_spot_em` | `stock_board_concept_em.py:131` | `push2/.../stock/get` |
//! | `stock_board_industry_hist_em` | `stock_board_industry_em.py:261` | `push2his/.../kline/get` |
//! | `stock_board_industry_hist_min_em` | `stock_board_industry_em.py:351` | `push2his/.../kline/get` (period ∈ {5,15,30,60}) |
//! | `stock_board_industry_hist_min_em_trends` | `stock_board_industry_em.py:351` | `push2his/.../trends2/get` (period = "1") |
//! | `stock_board_industry_spot_em` | `stock_board_industry_em.py:211` | `push2/.../stock/get` |
//!
//! `stock_board_*_hist_em` / `*_spot_em` accept either an Eastmoney board code
//! (`BK\d+`, used directly) or a board name (resolved to a code via the
//! `clist/get` endpoint). The kline `klines` / `trends` arrays are CSV strings
//! split on `,`; the spot `data` object carries per-field codes (`f43`…`f171`)
//! and is mapped to a single wide row (akshare flattens it to a long
//! `item`/`value` frame — we keep the normalized wide shape used elsewhere in
//! this crate, applying akshare's `value *= 1e-2` scaling except for volume
//! (`f47`) and amount (`f48`), which stay unscaled).
//!
//! ## DEFERRED (not ported)
//!
//! All 10 同花顺 (THS) functions below require `py_mini_racer` JS execution to
//! derive a `v` / `hexin-v` cookie plus `BeautifulSoup` / `pandas.read_html`
//! HTML-table scraping (and `demjson` for `index` data). They are **not**
//! implementable with a plain HTTP `get_json` call, so they are DEFERRED:
//!
//! * `stock_board_concept_index_ths` (`stock_board_concept_ths.py:124`) — JS cookie + `demjson` parse of `d.10jqka.com.cn` `.js`.
//! * `stock_board_concept_info_ths` (`stock_board_concept_ths.py:91`) — `BeautifulSoup` `.board-infos` scrape.
//! * `stock_board_concept_name_ths` (`stock_board_concept_ths.py:71`) — JS cookie + `BeautifulSoup` `.cate_inner` scrape.
//! * `stock_board_concept_summary_ths` (`stock_board_concept_ths.py:273`) — JS cookie + `read_html` scrape.
//! * `stock_board_industry_index_ths` (`stock_board_industry_ths.py:121`) — JS cookie + `demjson` parse.
//! * `stock_board_industry_info_ths` (`stock_board_industry_ths.py:88`) — `BeautifulSoup` `.board-infos` scrape.
//! * `stock_board_industry_name_ths` (`stock_board_industry_ths.py:68`) — JS cookie + `BeautifulSoup` scrape.
//! * `stock_board_industry_summary_ths` (`stock_board_industry_ths.py:331`) — JS cookie + `read_html` scrape.
//! * `stock_ipo_benefit_ths` (`stock_board_industry_ths.py:274`) — JS cookie + `read_html` scrape.
//! * `stock_xgsr_ths` (`stock_board_industry_ths.py:222`) — JS cookie + `read_html` scrape.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE: &str = "eastmoney";

/// Kline (daily/weekly/monthly & minute) endpoint.
const PUSH2_KLINE: &str = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
/// Intraday (period = "1") trends endpoint.
const PUSH2_TRENDS: &str = "https://push2his.eastmoney.com/api/qt/stock/trends2/get";
/// Real-time spot quote endpoint.
const PUSH2_STOCK: &str = "https://91.push2.eastmoney.com/api/qt/stock/get";
/// Board name→code listing endpoint (used for name resolution).
const PUSH2_CLIST: &str = "https://push2.eastmoney.com/api/qt/clist/get";
/// Static Eastmoney `ut` magic token (hardcoded in akshare source; not a
/// per-request JS signature, so name resolution stays feasible).
const EM_UT: &str = "bd1d9ddb04089700cf9c27f6f7426281";

/// A single (daily/weekly/monthly or minute-K-line) observation for a board.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BoardKlineRow {
    /// 日期 / 日期时间 (Eastmoney `f51`)
    pub date: String,
    /// 开盘 (Eastmoney `f52`)
    pub open: Option<f64>,
    /// 收盘 (Eastmoney `f53`)
    pub close: Option<f64>,
    /// 最高 (Eastmoney `f54`)
    pub high: Option<f64>,
    /// 最低 (Eastmoney `f55`)
    pub low: Option<f64>,
    /// 涨跌幅 (Eastmoney `f59`)
    pub pct: Option<f64>,
    /// 涨跌额 (Eastmoney `f60`)
    pub change: Option<f64>,
    /// 成交量 (Eastmoney `f56`)
    pub volume: Option<f64>,
    /// 成交额 (Eastmoney `f57`)
    pub amount: Option<f64>,
    /// 振幅 (Eastmoney `f58`)
    pub amplitude: Option<f64>,
    /// 换手率 (Eastmoney `f61`)
    pub turnover: Option<f64>,
}

/// A single intraday (period = "1") trend observation for a board.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BoardMinTrendsRow {
    /// 日期时间 (Eastmoney `f51`)
    pub datetime: String,
    /// 开盘 (Eastmoney `f52`)
    pub open: Option<f64>,
    /// 收盘 (Eastmoney `f53`)
    pub close: Option<f64>,
    /// 最高 (Eastmoney `f54`)
    pub high: Option<f64>,
    /// 最低 (Eastmoney `f55`)
    pub low: Option<f64>,
    /// 成交量 (Eastmoney `f56`)
    pub volume: Option<f64>,
    /// 成交额 (Eastmoney `f57`)
    pub amount: Option<f64>,
    /// 最新价 (Eastmoney `f58`)
    pub latest: Option<f64>,
}

/// A single real-time spot quote for a board (wide, one row).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BoardSpotRow {
    /// 最新 (Eastmoney `f43`)
    pub latest: Option<f64>,
    /// 最高 (Eastmoney `f44`)
    pub high: Option<f64>,
    /// 最低 (Eastmoney `f45`)
    pub low: Option<f64>,
    /// 开盘 (Eastmoney `f46`)
    pub open: Option<f64>,
    /// 成交量 (Eastmoney `f47`, unscaled)
    pub volume: Option<f64>,
    /// 成交额 (Eastmoney `f48`, unscaled)
    pub amount: Option<f64>,
    /// 涨跌幅 (Eastmoney `f170`)
    pub pct: Option<f64>,
    /// 振幅 (Eastmoney `f171`)
    pub amplitude: Option<f64>,
    /// 换手率 (Eastmoney `f168`)
    pub turnover: Option<f64>,
    /// 涨跌额 (Eastmoney `f169`)
    pub change: Option<f64>,
}

/// Read a numeric field that may be a JSON number or a numeric string.
fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// Read the `idx`-th element of a CSV-split kline/trend line as `f64`.
fn csv_num(parts: &[&str], idx: usize) -> Option<f64> {
    parts.get(idx).and_then(|s| s.trim().parse::<f64>().ok())
}

/// Parse board kline rows from the full `kline/get` response (`data.klines`).
/// Returns an empty `Vec` when `klines` is absent/null (no data for the range).
pub(crate) fn parse_board_kline(resp: &Value) -> Result<Vec<BoardKlineRow>> {
    let Some(klines) = resp
        .get("data")
        .and_then(|d| d.get("klines"))
        .and_then(|k| k.as_array())
    else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(klines.len());
    for line in klines {
        let Some(s) = line.as_str() else { continue };
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() < 11 {
            continue;
        }
        out.push(BoardKlineRow {
            date: parts[0].to_string(),
            open: csv_num(&parts, 1),
            close: csv_num(&parts, 2),
            high: csv_num(&parts, 3),
            low: csv_num(&parts, 4),
            pct: csv_num(&parts, 8),
            change: csv_num(&parts, 9),
            volume: csv_num(&parts, 5),
            amount: csv_num(&parts, 6),
            amplitude: csv_num(&parts, 7),
            turnover: csv_num(&parts, 10),
        });
    }
    Ok(out)
}

/// Parse intraday (period = "1") trend rows from the `trends2/get` response.
pub(crate) fn parse_board_min_trends(resp: &Value) -> Result<Vec<BoardMinTrendsRow>> {
    let Some(trends) = resp
        .get("data")
        .and_then(|d| d.get("trends"))
        .and_then(|t| t.as_array())
    else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(trends.len());
    for line in trends {
        let Some(s) = line.as_str() else { continue };
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() < 8 {
            continue;
        }
        out.push(BoardMinTrendsRow {
            datetime: parts[0].to_string(),
            open: csv_num(&parts, 1),
            close: csv_num(&parts, 2),
            high: csv_num(&parts, 3),
            low: csv_num(&parts, 4),
            volume: csv_num(&parts, 5),
            amount: csv_num(&parts, 6),
            latest: csv_num(&parts, 7),
        });
    }
    Ok(out)
}

/// Parse a real-time spot quote from the `stock/get` response (`data` object).
pub(crate) fn parse_board_spot(resp: &Value) -> Result<Vec<BoardSpotRow>> {
    let data = resp
        .get("data")
        .filter(|v| !v.is_null())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing data".into(),
        })?;
    // akshare: value *= 1e-2 for all fields, then 成交量(f47) and 成交额(f48)
    // *= 1e2, which nets to "unscaled" for those two.
    let unscaled = |k: &str| num(data.get(k)?);
    let scaled = |k: &str| num(data.get(k)?).map(|x| x * 1e-2);
    Ok(vec![BoardSpotRow {
        latest: scaled("f43"),
        high: scaled("f44"),
        low: scaled("f45"),
        open: scaled("f46"),
        volume: unscaled("f47"),
        amount: unscaled("f48"),
        pct: scaled("f170"),
        amplitude: scaled("f171"),
        turnover: scaled("f168"),
        change: scaled("f169"),
    }])
}

/// True when `symbol` is an Eastmoney board code like `BK0818`.
fn is_bk_code(symbol: &str) -> bool {
    symbol.len() > 2 && symbol.starts_with("BK") && symbol[2..].chars().all(|c| c.is_ascii_digit())
}

/// Resolve a board `symbol` (name or `BK` code) to its Eastmoney board code via
/// the `clist/get` listing endpoint.
async fn resolve_board_code(client: &Client, symbol: &str, fs: &str, fid: &str) -> Result<String> {
    if is_bk_code(symbol) {
        return Ok(symbol.to_string());
    }
    let params = [
        ("pn", "1"),
        ("pz", "1000"),
        ("po", "1"),
        ("np", "1"),
        ("ut", EM_UT),
        ("fltt", "2"),
        ("invt", "2"),
        ("fid", fid),
        ("fs", fs),
        ("fields", "f12,f14"),
    ];
    let v = client
        .get_json(SOURCE, "board_resolve", PUSH2_CLIST, &params)
        .await?;
    let Some(diff) = v
        .get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|x| x.as_array())
    else {
        return Err(Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing data.diff in clist response".into(),
        });
    };
    for item in diff {
        let code = item.get("f12").and_then(|x| x.as_str());
        let name = item.get("f14").and_then(|x| x.as_str());
        if let (Some(c), Some(n)) = (code, name)
            && n == symbol
        {
            return Ok(c.to_string());
        }
    }
    Err(Error::NotFound {
        endpoint: "board_resolve",
        message: format!("no Eastmoney board named `{symbol}`"),
    })
}

/// Resolve a concept-board name/code to its `BK` code.
pub(crate) async fn resolve_concept_code(client: &Client, symbol: &str) -> Result<String> {
    resolve_board_code(client, symbol, "m:90 t:3 f:!50", "f12").await
}

/// Resolve an industry-board name/code to its `BK` code.
pub(crate) async fn resolve_industry_code(client: &Client, symbol: &str) -> Result<String> {
    resolve_board_code(client, symbol, "m:90 t:2 f:!50", "f3").await
}

/// Map a hist `period` to Eastmoney `klt`.
fn hist_klt(period: &str) -> &'static str {
    match period {
        "weekly" => "102",
        "monthly" => "103",
        _ => "101", // daily (default)
    }
}

/// Map an `adjust` to Eastmoney `fqt`.
fn adjust_fqt(adjust: &str) -> &'static str {
    match adjust {
        "qfq" => "1",
        "hfq" => "2",
        _ => "0", // "" (default)
    }
}

/// Fetch and parse board kline (daily/weekly/monthly) rows.
async fn fetch_board_hist(
    client: &Client,
    fn_name: &'static str,
    secid: &str,
    klt: &str,
    fqt: &str,
    beg: &str,
    end: &str,
) -> Result<Vec<BoardKlineRow>> {
    let params = [
        ("secid", secid),
        ("fields1", "f1,f2,f3,f4,f5,f6"),
        ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61"),
        ("klt", klt),
        ("fqt", fqt),
        ("beg", beg),
        ("end", end),
        ("smplmt", "10000"),
        ("lmt", "1000000"),
    ];
    let v = client
        .get_json(SOURCE, fn_name, PUSH2_KLINE, &params)
        .await?;
    parse_board_kline(&v)
}

/// Fetch and parse board minute-K-line (period ∈ {5,15,30,60}) rows.
async fn fetch_board_min_kline(
    client: &Client,
    fn_name: &'static str,
    secid: &str,
    klt: &str,
) -> Result<Vec<BoardKlineRow>> {
    let params = [
        ("secid", secid),
        ("fields1", "f1,f2,f3,f4,f5,f6"),
        ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61"),
        ("klt", klt),
        ("fqt", "1"),
        ("beg", "0"),
        ("end", "20500101"),
        ("smplmt", "10000"),
        ("lmt", "1000000"),
    ];
    let v = client
        .get_json(SOURCE, fn_name, PUSH2_KLINE, &params)
        .await?;
    parse_board_kline(&v)
}

/// Fetch and parse intraday (period = "1") trend rows.
async fn fetch_board_min_trends(
    client: &Client,
    fn_name: &'static str,
    secid: &str,
) -> Result<Vec<BoardMinTrendsRow>> {
    let params = [
        ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f11,f12,f13"),
        ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58"),
        ("iscr", "0"),
        ("ndays", "1"),
        ("secid", secid),
    ];
    let v = client
        .get_json(SOURCE, fn_name, PUSH2_TRENDS, &params)
        .await?;
    parse_board_min_trends(&v)
}

/// Fetch and parse a real-time spot quote.
async fn fetch_board_spot(
    client: &Client,
    fn_name: &'static str,
    secid: &str,
) -> Result<Vec<BoardSpotRow>> {
    let params = [
        ("fields", "f43,f44,f45,f46,f47,f48,f170,f171,f168,f169"),
        ("mpi", "1000"),
        ("invt", "2"),
        ("fltt", "1"),
        ("secid", secid),
    ];
    let v = client
        .get_json(SOURCE, fn_name, PUSH2_STOCK, &params)
        .await?;
    parse_board_spot(&v)
}

// ===========================================================================
// Concept boards (akshare/stock/stock_board_concept_em.py)
// ===========================================================================

/// 东方财富-概念板块-历史行情. Defaults `symbol="绿色电力"`, `period="daily"`,
/// `start_date="20220101"`, `end_date="20221128"`, `adjust=""`.
pub async fn stock_board_concept_hist_em(client: &Client) -> Result<Vec<BoardKlineRow>> {
    stock_board_concept_hist_em_opts(client, "绿色电力", "daily", "20220101", "20221128", "").await
}

/// 东方财富-概念板块-历史行情 with explicit params (`period`: daily/weekly/monthly;
/// `adjust`: ""/qfq/hfq). `symbol` may be a `BK` code or a board name.
pub async fn stock_board_concept_hist_em_opts(
    client: &Client,
    symbol: &str,
    period: &str,
    start_date: &str,
    end_date: &str,
    adjust: &str,
) -> Result<Vec<BoardKlineRow>> {
    let code = resolve_concept_code(client, symbol).await?;
    let secid = format!("90.{code}");
    fetch_board_hist(
        client,
        "stock_board_concept_hist_em",
        &secid,
        hist_klt(period),
        adjust_fqt(adjust),
        start_date,
        end_date,
    )
    .await
}

/// 东方财富-概念板块-分时历史行情 (period ∈ {5,15,30,60}). Defaults `symbol="长寿药"`, `period="5"`.
pub async fn stock_board_concept_hist_min_em(client: &Client) -> Result<Vec<BoardKlineRow>> {
    stock_board_concept_hist_min_em_opts(client, "长寿药", "5").await
}

/// 东方财富-概念板块-分时历史行情 (minute K-line) with explicit params.
pub async fn stock_board_concept_hist_min_em_opts(
    client: &Client,
    symbol: &str,
    period: &str,
) -> Result<Vec<BoardKlineRow>> {
    let code = resolve_concept_code(client, symbol).await?;
    let secid = format!("90.{code}");
    fetch_board_min_kline(client, "stock_board_concept_hist_min_em", &secid, period).await
}

/// 东方财富-概念板块-分时历史行情 (intraday, period = "1"). Defaults `symbol="长寿药"`.
pub async fn stock_board_concept_hist_min_em_trends(
    client: &Client,
) -> Result<Vec<BoardMinTrendsRow>> {
    stock_board_concept_hist_min_em_trends_opts(client, "长寿药").await
}

/// 东方财富-概念板块-分时历史行情 (intraday, period = "1") with explicit `symbol`.
pub async fn stock_board_concept_hist_min_em_trends_opts(
    client: &Client,
    symbol: &str,
) -> Result<Vec<BoardMinTrendsRow>> {
    let code = resolve_concept_code(client, symbol).await?;
    let secid = format!("90.{code}");
    fetch_board_min_trends(client, "stock_board_concept_hist_min_em_trends", &secid).await
}

/// 东方财富-概念板块-实时行情. Defaults `symbol="可燃冰"`.
pub async fn stock_board_concept_spot_em(client: &Client) -> Result<Vec<BoardSpotRow>> {
    stock_board_concept_spot_em_opts(client, "可燃冰").await
}

/// 东方财富-概念板块-实时行情 with explicit `symbol` (board code or name).
pub async fn stock_board_concept_spot_em_opts(
    client: &Client,
    symbol: &str,
) -> Result<Vec<BoardSpotRow>> {
    let code = resolve_concept_code(client, symbol).await?;
    let secid = format!("90.{code}");
    fetch_board_spot(client, "stock_board_concept_spot_em", &secid).await
}

// ===========================================================================
// Industry boards (akshare/stock/stock_board_industry_em.py)
// ===========================================================================

/// 东方财富-行业板块-历史行情. Defaults `symbol="小金属"`, `period="日k"`,
/// `start_date="20211201"`, `end_date="20220401"`, `adjust=""`.
pub async fn stock_board_industry_hist_em(client: &Client) -> Result<Vec<BoardKlineRow>> {
    stock_board_industry_hist_em_opts(client, "小金属", "日k", "20211201", "20220401", "").await
}

/// 东方财富-行业板块-历史行情 with explicit params (`period`: 日k/周k/月k).
pub async fn stock_board_industry_hist_em_opts(
    client: &Client,
    symbol: &str,
    period: &str,
    start_date: &str,
    end_date: &str,
    adjust: &str,
) -> Result<Vec<BoardKlineRow>> {
    let code = resolve_industry_code(client, symbol).await?;
    let secid = format!("90.{code}");
    // akshare industry `period` uses 日k/周k/月k → 101/102/103.
    let klt = match period {
        "周k" => "102",
        "月k" => "103",
        _ => "101",
    };
    fetch_board_hist(
        client,
        "stock_board_industry_hist_em",
        &secid,
        klt,
        adjust_fqt(adjust),
        start_date,
        end_date,
    )
    .await
}

/// 东方财富-行业板块-分时历史行情 (period ∈ {5,15,30,60}). Defaults `symbol="小金属"`, `period="5"`.
pub async fn stock_board_industry_hist_min_em(client: &Client) -> Result<Vec<BoardKlineRow>> {
    stock_board_industry_hist_min_em_opts(client, "小金属", "5").await
}

/// 东方财富-行业板块-分时历史行情 (minute K-line) with explicit params.
pub async fn stock_board_industry_hist_min_em_opts(
    client: &Client,
    symbol: &str,
    period: &str,
) -> Result<Vec<BoardKlineRow>> {
    let code = resolve_industry_code(client, symbol).await?;
    let secid = format!("90.{code}");
    fetch_board_min_kline(client, "stock_board_industry_hist_min_em", &secid, period).await
}

/// 东方财富-行业板块-分时历史行情 (intraday, period = "1"). Defaults `symbol="小金属"`.
pub async fn stock_board_industry_hist_min_em_trends(
    client: &Client,
) -> Result<Vec<BoardMinTrendsRow>> {
    stock_board_industry_hist_min_em_trends_opts(client, "小金属").await
}

/// 东方财富-行业板块-分时历史行情 (intraday, period = "1") with explicit `symbol`.
pub async fn stock_board_industry_hist_min_em_trends_opts(
    client: &Client,
    symbol: &str,
) -> Result<Vec<BoardMinTrendsRow>> {
    let code = resolve_industry_code(client, symbol).await?;
    let secid = format!("90.{code}");
    fetch_board_min_trends(client, "stock_board_industry_hist_min_em_trends", &secid).await
}

/// 东方财富-行业板块-实时行情. Defaults `symbol="小金属"`.
pub async fn stock_board_industry_spot_em(client: &Client) -> Result<Vec<BoardSpotRow>> {
    stock_board_industry_spot_em_opts(client, "小金属").await
}

/// 东方财富-行业板块-实时行情 with explicit `symbol` (board code or name).
pub async fn stock_board_industry_spot_em_opts(
    client: &Client,
    symbol: &str,
) -> Result<Vec<BoardSpotRow>> {
    let code = resolve_industry_code(client, symbol).await?;
    let secid = format!("90.{code}");
    fetch_board_spot(client, "stock_board_industry_spot_em", &secid).await
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

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    #[test]
    fn parse_concept_hist() {
        let rows = parse_board_kline(&fixture("stock_board_concept_hist_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2022-01-04");
        assert!(approx(rows[0].open, 12.34));
        assert!(approx(rows[0].close, 12.56));
        assert!(approx(rows[0].high, 12.80));
        assert!(approx(rows[0].low, 12.10));
        assert!(approx(rows[0].pct, 1.23));
        assert!(approx(rows[0].change, 0.15));
        assert!(approx(rows[0].volume, 1000000.0));
        assert!(approx(rows[0].amount, 1.25e8));
        assert!(approx(rows[0].amplitude, 5.6));
        assert!(approx(rows[0].turnover, 2.1));
        assert!(approx(rows[1].close, 12.70));
    }

    #[test]
    fn parse_concept_hist_min_kline() {
        let rows = parse_board_kline(&fixture("stock_board_concept_hist_min_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2022-01-04 09:35");
        assert!(approx(rows[0].open, 12.34));
        assert!(approx(rows[0].pct, 0.5));
        assert!(approx(rows[1].close, 12.50));
    }

    #[test]
    fn parse_concept_hist_min_trends() {
        let rows = parse_board_min_trends(&fixture("stock_board_concept_hist_min_em_trends.json"))
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].datetime, "2022-01-04 09:30");
        assert!(approx(rows[0].open, 12.30));
        assert!(approx(rows[0].close, 12.35));
        assert!(approx(rows[0].amount, 5.0e7));
        assert!(approx(rows[0].latest, 12.35));
        assert!(approx(rows[1].latest, 12.40));
    }

    #[test]
    fn parse_concept_spot() {
        let rows = parse_board_spot(&fixture("stock_board_concept_spot_em.json")).unwrap();
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        // scaling: f43 raw 123456 * 1e-2 = 1234.56
        assert!(approx(r.latest, 1234.56));
        assert!(approx(r.high, 1280.00));
        assert!(approx(r.low, 1210.00));
        assert!(approx(r.open, 1220.00));
        // volume / amount unscaled
        assert!(approx(r.volume, 500000.0));
        assert!(approx(r.amount, 8.0e8));
        assert!(approx(r.pct, 2.56));
        assert!(approx(r.amplitude, 5.67));
        assert!(approx(r.turnover, 3.45));
        assert!(approx(r.change, 30.78));
    }

    #[test]
    fn parse_industry_hist() {
        let rows = parse_board_kline(&fixture("stock_board_industry_hist_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2021-12-01");
        assert!(approx(rows[0].close, 2345.67));
        assert!(approx(rows[0].pct, -0.5));
        assert!(approx(rows[1].close, 2300.00));
    }

    #[test]
    fn parse_industry_hist_min_kline() {
        let rows = parse_board_kline(&fixture("stock_board_industry_hist_min_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2021-12-01 13:05");
        assert!(approx(rows[0].open, 2340.0));
        assert!(approx(rows[0].turnover, 1.2));
        assert!(approx(rows[1].close, 2350.0));
    }

    #[test]
    fn parse_industry_hist_min_trends() {
        let rows = parse_board_min_trends(&fixture("stock_board_industry_hist_min_em_trends.json"))
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].datetime, "2021-12-01 09:30");
        assert!(approx(rows[0].open, 2340.0));
        assert!(approx(rows[0].volume, 800000.0));
        assert!(approx(rows[1].latest, 2360.0));
    }

    #[test]
    fn parse_industry_spot() {
        let rows = parse_board_spot(&fixture("stock_board_industry_spot_em.json")).unwrap();
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert!(approx(r.latest, 3456.78));
        assert!(approx(r.high, 3500.00));
        assert!(approx(r.volume, 900000.0));
        assert!(approx(r.amount, 2.0e9));
        assert!(approx(r.pct, 1.50));
        assert!(approx(r.change, 51.00));
    }

    #[test]
    fn parse_kline_empty_when_no_data() {
        let v = serde_json::json!({ "data": { "klines": null } });
        assert_eq!(parse_board_kline(&v).unwrap().len(), 0);
    }

    #[test]
    fn parse_spot_missing_data_errors() {
        let v = serde_json::json!({ "data": null });
        assert!(parse_board_spot(&v).is_err());
    }

    #[test]
    fn is_bk_code_detects_codes() {
        assert!(is_bk_code("BK0818"));
        assert!(is_bk_code("BK1027"));
        assert!(!is_bk_code("可燃冰"));
        assert!(!is_bk_code("BK"));
    }
}
