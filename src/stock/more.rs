//! Additional A-share stock endpoints ported from akshare.
//!
//! | Rust fn                          | akshare fn                     | Source            | Notes                                         |
//! |----------------------------------|--------------------------------|-------------------|-----------------------------------------------|
//! | `stock_zh_a_st_em`               | `stock_zh_a_st_em`             | Eastmoney push2   | 风险警示板 (ST) spot list, `clist/get`        |
//! | `stock_a_high_low_statistics`    | `stock_a_high_low_statistics`  | 乐咕乐股 (legulegu) | 创新高/新低数量, pure JSON                    |
//! | `stock_a_below_net_asset_statistics` | `stock_a_below_net_asset_statistics` | 乐咕乐股 (legulegu) | 破净股统计, static token, pure JSON   |
//! | `stock_account_statistics_em`    | `stock_account_statistics_em`  | Eastmoney datacenter | 股票账户统计 (`stock_em_account` in brief) |
//! | `stock_zt_pool_em`               | `stock_zt_pool_em`             | Eastmoney push2ex | 涨停股池, `data.pool`                         |
//!
//! All five ports are pure-JSON HTTP (no JS signing, no HTML scraping, no
//! encryption). The brief's suggested `stock_em_account` maps to akshare's
//! `stock_account_statistics_em` in this akshare checkout.
//!
//! ## Skips (not ported here)
//!
//! - `stock_a_all_pb` / `stock_a_ttm_lyr` / `stock_a_congestion_lg` /
//!   `stock_a_gxl_lg` — 乐咕乐股 endpoints that require `get_token_lg()` +
//!   `get_cookie_csrf()` (JS-executed token / cookie signing). Out of scope.
//! - `stock_bj_a_spot_em` / `stock_cy_a_spot_em` — Eastmoney `clist/get` spot
//!   boards. Omitted to avoid overlapping the already-ported `stock_zh_a_spot`
//!   concept (they are near-duplicates of the clist pattern used below).
//! - `stock_zt_pool_previous_em` / `_strong_em` / `_sub_new_em` / `_zbgc_em` /
//!   `_dtgc_em` — sibling 涨停板 push2ex pools. Only `stock_zt_pool_em` (涨停股池)
//!   is ported for scope; the rest share the same `data.pool` shape and can be
//!   added by mirroring `stock_zt_pool_em`.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

/// 乐咕乐股 (legulegu) source bucket, for rate limiting / error context.
///
/// The crate's `client` only predefines Eastmoney / Sina / Tencent buckets, but
/// `get_json` accepts any `&'static str` source id, so we use a dedicated one.
const SOURCE_LEGULEGU: &str = "legulegu";

/// Eastmoney `clist/get` endpoint base (matches `src/board/mod.rs` `CLIST_BASE`).
const CLIST_BASE: &str = "https://push2.eastmoney.com/api/qt/clist/get";

/// Static Eastmoney `ut` token (no JS signing required, ADR-0005).
const CLIST_UT: &str = "bd1d9ddb04089700cf9c27f6f7426281";

/// Default page size, mirroring akshare (`pz=100`).
const PAGE_SIZE: u32 = 100;

// ---------------------------------------------------------------------------
// Shared helpers (mirror src/stock/holder.rs and src/board/mod.rs)
// ---------------------------------------------------------------------------

/// Read a string field, defaulting to `""` when missing/null (matches akshare,
/// which keeps such columns as empty strings rather than dropping the row).
fn fstr(item: &Value, k: &str) -> String {
    item.get(k)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

/// Read a numeric field that may be a JSON number or a plain numeric string.
fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
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

/// Format an Eastmoney HHMMSS-style time (integer or string) as a zero-padded
/// 6-char string, matching akshare's `str.zfill(6)`. Returns `None` when null.
fn fmt_time(v: &Value) -> Option<String> {
    match v {
        Value::Number(n) => n.as_i64().map(|x| format!("{x:06}")),
        Value::String(s) => Some(s.trim().to_string()),
        _ => None,
    }
}

// ===========================================================================
// stock_zh_a_st_em — 东方财富-风险警示板 (ST stock spot list)
// ===========================================================================

/// One ST (risk-warning) stock quote row, port of `stock_zh_a_st_em`.
///
/// Field ids are Eastmoney `push2` `clist/get` `fNNN` keys (akshare
/// `stock_zh_a_special.py`). `fltt=2` means prices/percentages are already
/// human-readable numbers (not scaled integers).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockZhAStEmRow {
    /// `f12` 代码
    pub code: String,
    /// `f13` 市场 (1=沪, 0=深)
    pub market: Option<f64>,
    /// `f14` 名称
    pub name: String,
    /// `f2` 最新价
    pub price: Option<f64>,
    /// `f3` 涨跌幅 (%)
    pub pct_change: Option<f64>,
    /// `f4` 涨跌额
    pub change: Option<f64>,
    /// `f5` 成交量 (手)
    pub volume: Option<f64>,
    /// `f6` 成交额 (元)
    pub amount: Option<f64>,
    /// `f7` 振幅 (%)
    pub amplitude: Option<f64>,
    /// `f8` 换手率 (%)
    pub turnover: Option<f64>,
    /// `f9` 市盈率-动态
    pub pe_ttm: Option<f64>,
    /// `f10` 量比
    pub volume_ratio: Option<f64>,
    /// `f15` 最高
    pub high: Option<f64>,
    /// `f16` 最低
    pub low: Option<f64>,
    /// `f17` 今开
    pub open: Option<f64>,
    /// `f18` 昨收
    pub pre_close: Option<f64>,
    /// `f20` 总市值
    pub total_mktcap: Option<f64>,
    /// `f21` 流通市值
    pub float_mktcap: Option<f64>,
    /// `f23` 市净率
    pub pb: Option<f64>,
    pub source: &'static str,
}

/// Eastmoney `fs` filter for the risk-warning (ST) board: 沪深 A 股中 ST 板块.
const FS_ST: &str = "m:0 f:4,m:1 f:4";
/// Sort by `f3` (涨跌幅).
const FID_ST: &str = "f3";
/// Field list, copied verbatim from akshare `stock_zh_a_special.py`. The parser
/// maps by `fNNN` key (not by column position), so the mid-list re-ordering in
/// akshare's positional `columns=` rename is irrelevant here.
const FIELDS_ST: &str = "f2,f3,f4,f5,f6,f7,f8,f9,f10,f12,f13,f14,f15,f16,f17,\
f18,f20,f21,f23,f24,f25,f22,f11,f62,f128,f136,f115,f152";

/// Port of `stock_zh_a_st_em()`.
///
/// Fetches the full ST board via paginated `clist/get` (mirrors akshare's
/// `fetch_paginated_data`). `endpoint` is recorded as `"stock_zh_a_st_em"`.
pub async fn stock_zh_a_st_em(client: &Client) -> Result<Vec<StockZhAStEmRow>> {
    let mut out = Vec::new();
    let mut pn = 1u32;
    loop {
        let v = fetch_clist(client, "stock_zh_a_st_em", FS_ST, FID_ST, FIELDS_ST, pn).await?;
        let total = v
            .get("data")
            .and_then(|d| d.get("total"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        let rows = parse_st(&v)?;
        if rows.is_empty() {
            break;
        }
        out.extend(rows);
        if (pn as u64) * PAGE_SIZE as u64 >= total {
            break;
        }
        pn += 1;
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    }
    Ok(out)
}

/// Fetch one page of an Eastmoney `clist/get` query.
async fn fetch_clist(
    client: &Client,
    endpoint: &'static str,
    fs: &str,
    fid: &str,
    fields: &str,
    pn: u32,
) -> Result<Value> {
    let pn_s = pn.to_string();
    let pz_s = PAGE_SIZE.to_string();
    let params = [
        ("pn", pn_s.as_str()),
        ("pz", pz_s.as_str()),
        ("po", "1"),
        ("np", "1"),
        ("ut", CLIST_UT),
        ("fltt", "2"),
        ("invt", "2"),
        ("fid", fid),
        ("fs", fs),
        ("fields", fields),
    ];
    client
        .get_json(SOURCE_EASTMONEY, endpoint, CLIST_BASE, &params)
        .await
}

/// Parse an Eastmoney `clist/get` `data.diff` array into [`StockZhAStEmRow`]s.
pub(crate) fn parse_st(resp: &Value) -> Result<Vec<StockZhAStEmRow>> {
    let diff = resp
        .get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff at stock_zh_a_st_em".into(),
        })?;
    let mut out = Vec::with_capacity(diff.len());
    for item in diff {
        let code = fstr(item, "f12");
        let name = fstr(item, "f14");
        if code.is_empty() || name.is_empty() {
            continue;
        }
        out.push(StockZhAStEmRow {
            code,
            market: fnum(item, "f13"),
            name,
            price: fnum(item, "f2"),
            pct_change: fnum(item, "f3"),
            change: fnum(item, "f4"),
            volume: fnum(item, "f5"),
            amount: fnum(item, "f6"),
            amplitude: fnum(item, "f7"),
            turnover: fnum(item, "f8"),
            pe_ttm: fnum(item, "f9"),
            volume_ratio: fnum(item, "f10"),
            high: fnum(item, "f15"),
            low: fnum(item, "f16"),
            open: fnum(item, "f17"),
            pre_close: fnum(item, "f18"),
            total_mktcap: fnum(item, "f20"),
            float_mktcap: fnum(item, "f21"),
            pb: fnum(item, "f23"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_a_high_low_statistics — 乐咕乐股-创新高/新低股票数量
// ===========================================================================

/// One daily row of new-high/new-low counts, port of
/// `stock_a_high_low_statistics`.
///
/// 乐咕乐股 returns a top-level JSON array of objects; `indexCode` is dropped
/// (akshare keeps `date`, `close` and the `high*`/`low*` N-day counts).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockAHighLowRow {
    /// `date` 日期 (ISO date string from legulegu)
    pub date: String,
    /// `close` 收盘
    pub close: Option<f64>,
    /// `high20` 20日新高数量
    pub high20: Option<f64>,
    /// `low20` 20日新低数量
    pub low20: Option<f64>,
    /// `high60` 60日新高数量
    pub high60: Option<f64>,
    /// `low60` 60日新低数量
    pub low60: Option<f64>,
    /// `high120` 120日新高数量
    pub high120: Option<f64>,
    /// `low120` 120日新低数量
    pub low120: Option<f64>,
    pub source: &'static str,
}

/// Valid `symbol` choices for `stock_a_high_low_statistics`.
const HIGH_LOW_SYMBOLS: &[&str] = &["all", "sz50", "hs300", "zz500"];

/// Validate `symbol` and build the 乐咕乐股 URL for `stock_a_high_low_statistics`.
///
/// Returns `Error::InvalidParam` for anything outside `HIGH_LOW_SYMBOLS`.
pub(crate) fn high_low_request(symbol: &str) -> Result<(String, &'static str)> {
    if !HIGH_LOW_SYMBOLS.contains(&symbol) {
        return Err(Error::InvalidParam(format!(
            "stock_a_high_low_statistics: symbol must be one of {HIGH_LOW_SYMBOLS:?}, got {symbol:?}"
        )));
    }
    let url =
        format!("https://www.legulegu.com/stockdata/member-ship/get-high-low-statistics/{symbol}");
    Ok((url, SOURCE_LEGULEGU))
}

/// Port of `stock_a_high_low_statistics(symbol)`.
///
/// `symbol` ∈ {"all", "sz50", "hs300", "zz500"}; it is embedded in the URL path.
pub async fn stock_a_high_low_statistics(
    client: &Client,
    symbol: &str,
) -> Result<Vec<StockAHighLowRow>> {
    let (url, source) = high_low_request(symbol)?;
    let v = client
        .get_json(source, "stock_a_high_low_statistics", &url, &[])
        .await?;
    parse_high_low(&v)
}

/// Parse a top-level 乐咕乐股 array into [`StockAHighLowRow`]s.
pub(crate) fn parse_high_low(resp: &Value) -> Result<Vec<StockAHighLowRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_LEGULEGU,
        message: "high-low response is not a JSON array".into(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(StockAHighLowRow {
            date: fstr(item, "date"),
            close: fnum(item, "close"),
            high20: fnum(item, "high20"),
            low20: fnum(item, "low20"),
            high60: fnum(item, "high60"),
            low60: fnum(item, "low60"),
            high120: fnum(item, "high120"),
            low120: fnum(item, "low120"),
            source: SOURCE_LEGULEGU,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_a_below_net_asset_statistics — 乐咕乐股-破净股统计
// ===========================================================================

/// One daily row of below-net-asset (破净) statistics, port of
/// `stock_a_below_net_asset_statistics`.
///
/// 乐咕乐股 returns a top-level JSON array; `below_net_asset_ratio` is computed
/// as `below_net_asset / total_company` (akshare does the same, rounded to 4dp).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockABelowNetAssetRow {
    /// `date` 日期
    pub date: String,
    /// `belowNetAsset` 破净股数量
    pub below_net_asset: Option<f64>,
    /// `totalCompany` 公司总数
    pub total_company: Option<f64>,
    /// 破净股占比 = below_net_asset / total_company
    pub below_net_asset_ratio: Option<f64>,
    pub source: &'static str,
}

/// 乐咕乐股 `marketId` values for each `symbol`.
fn below_net_asset_market_id(symbol: &str) -> Result<&'static str> {
    match symbol {
        "全部A股" => Ok("1"),
        "沪深300" => Ok("000300.XSHG"),
        "上证50" => Ok("000016.SH"),
        "中证500" => Ok("000905.SH"),
        other => Err(Error::InvalidParam(format!(
            "stock_a_below_net_asset_statistics: symbol must be one of \
             {{\"全部A股\", \"沪深300\", \"上证50\", \"中证500\"}}, got {other:?}"
        ))),
    }
}

/// Port of `stock_a_below_net_asset_statistics(symbol)`.
///
/// `symbol` ∈ {"全部A股", "沪深300", "上证50", "中证500"}. A static `token` is
/// embedded (no JS signing, ADR-0005).
pub async fn stock_a_below_net_asset_statistics(
    client: &Client,
    symbol: &str,
) -> Result<Vec<StockABelowNetAssetRow>> {
    let market_id = below_net_asset_market_id(symbol)?;
    let params = [
        ("marketId", market_id),
        ("token", "325843825a2745a2a8f9b9e3355cb864"),
    ];
    let v = client
        .get_json(
            SOURCE_LEGULEGU,
            "stock_a_below_net_asset_statistics",
            "https://legulegu.com/stockdata/below-net-asset-statistics-data",
            &params,
        )
        .await?;
    parse_below_net_asset(&v)
}

/// Parse a top-level 乐咕乐股 array into [`StockABelowNetAssetRow`]s.
pub(crate) fn parse_below_net_asset(resp: &Value) -> Result<Vec<StockABelowNetAssetRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_LEGULEGU,
        message: "below-net-asset response is not a JSON array".into(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let below = fnum(item, "belowNetAsset");
        let total = fnum(item, "totalCompany");
        let ratio = match (below, total) {
            (Some(b), Some(t)) if t != 0.0 => Some(b / t),
            _ => None,
        };
        out.push(StockABelowNetAssetRow {
            date: fstr(item, "date"),
            below_net_asset: below,
            total_company: total,
            below_net_asset_ratio: ratio,
            source: SOURCE_LEGULEGU,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_account_statistics_em — 东方财富-股票账户统计
// ===========================================================================

/// One row of investor-account statistics, port of `stock_account_statistics_em`
/// (the brief's `stock_em_account`).
///
/// Field ids follow Eastmoney datacenter report `RPT_STOCK_OPEN_DATA`. Values are
/// `Option<f64>`; `stat_date` is the report's `STATISTICS_DATE` (e.g. `"2024-01"`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockAccountStatisticsRow {
    /// `STATISTICS_DATE` 数据日期
    pub stat_date: String,
    /// `NEW_NUMBER` 新增投资者-数量
    pub new_investors: Option<f64>,
    /// `NEW_NUMBER_RATIO` 新增投资者-环比 (%)
    pub new_investors_mom: Option<f64>,
    /// `NEW_NUMBER_SAME` 新增投资者-同比 (%)
    pub new_investors_yoy: Option<f64>,
    /// `END_NUMBER` 期末投资者-总量
    pub end_total: Option<f64>,
    /// `END_A_NUMBER` 期末投资者-A股账户
    pub end_a_accounts: Option<f64>,
    /// `END_B_NUMBER` 期末投资者-B股账户
    pub end_b_accounts: Option<f64>,
    /// `SH_INDEX` 上证指数-收盘
    pub sh_close: Option<f64>,
    /// `SH_INDEX_PCT` 上证指数-涨跌幅 (%)
    pub sh_pct: Option<f64>,
    /// `TOTAL_MARKET_CAP` 沪深总市值
    pub total_mktcap: Option<f64>,
    /// `AVG_MARKET_CAP` 沪深户均市值
    pub avg_mktcap: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_account_statistics_em()`.
///
/// Queries Eastmoney datacenter report `RPT_STOCK_OPEN_DATA`. Up to `pageSize`
/// (500) rows are returned, sorted by `STATISTICS_DATE` descending (akshare
/// then re-sorts ascending; we keep the upstream order and let callers sort).
pub async fn stock_account_statistics_em(
    client: &Client,
) -> Result<Vec<StockAccountStatisticsRow>> {
    let params = [
        ("reportName", "RPT_STOCK_OPEN_DATA"),
        ("columns", "ALL"),
        ("pageSize", "500"),
        ("sortColumns", "STATISTICS_DATE"),
        ("sortTypes", "-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("p", "1"),
        ("pageNo", "1"),
        ("pageNum", "1"),
        ("pageNumber", "1"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_account_statistics_em",
            "https://datacenter-web.eastmoney.com/api/data/v1/get",
            &params,
        )
        .await?;
    parse_account_statistics(&v)
}

/// Parse an Eastmoney datacenter `result.data` array into [`StockAccountStatisticsRow`]s.
pub(crate) fn parse_account_statistics(resp: &Value) -> Result<Vec<StockAccountStatisticsRow>> {
    let arr = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data at stock_account_statistics_em".into(),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(StockAccountStatisticsRow {
            stat_date: fstr(item, "STATISTICS_DATE"),
            new_investors: fnum(item, "NEW_NUMBER"),
            new_investors_mom: fnum(item, "NEW_NUMBER_RATIO"),
            new_investors_yoy: fnum(item, "NEW_NUMBER_SAME"),
            end_total: fnum(item, "END_NUMBER"),
            end_a_accounts: fnum(item, "END_A_NUMBER"),
            end_b_accounts: fnum(item, "END_B_NUMBER"),
            sh_close: fnum(item, "SH_INDEX"),
            sh_pct: fnum(item, "SH_INDEX_PCT"),
            total_mktcap: fnum(item, "TOTAL_MARKET_CAP"),
            avg_mktcap: fnum(item, "AVG_MARKET_CAP"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_zt_pool_em — 东方财富-涨停股池
// ===========================================================================

/// One limit-up (涨停) stock in the 涨停股池, port of `stock_zt_pool_em`.
///
/// Source keys are Eastmoney `push2ex` `getTopicZTPool` `data.pool` item fields.
/// `price` is divided by 1000 (akshare does `最新价 / 1000`). `first_time` /
/// `last_time` are zero-padded HHMMSS strings (akshare `str.zfill(6)`).
/// `zt_stat` is the `涨停统计` `"days/ct"` string.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockZtPoolRow {
    /// `c` 代码
    pub code: String,
    /// `n` 名称
    pub name: String,
    /// `p` 最新价 (÷1000)
    pub price: Option<f64>,
    /// `zdp` 涨跌幅 (%)
    pub pct_change: Option<f64>,
    /// `amount` 成交额 (元)
    pub amount: Option<f64>,
    /// `ltsz` 流通市值
    pub float_mktcap: Option<f64>,
    /// `tshare` 总市值
    pub total_mktcap: Option<f64>,
    /// `hs` 换手率 (%)
    pub turnover: Option<f64>,
    /// `lb` 连板数
    pub consecutive_boards: Option<f64>,
    /// `fbt` 首次封板时间 (HHMMSS, zero-padded)
    pub first_time: Option<String>,
    /// `lbt` 最后封板时间 (HHMMSS, zero-padded)
    pub last_time: Option<String>,
    /// `fd` 封板资金
    pub seal_funds: Option<f64>,
    /// `zbc` 炸板次数
    pub explode_count: Option<f64>,
    /// `hy` 所属行业
    pub industry: String,
    /// `zt` 涨停统计 (`days/ct`)
    pub zt_stat: Option<String>,
    pub source: &'static str,
}

/// Port of `stock_zt_pool_em(date)`.
///
/// `date` is a trading day `YYYYMMDD`. Returns an empty `Vec` when upstream
/// returns `data: null` (no limit-up pool for that day, akshare returns empty).
pub async fn stock_zt_pool_em(client: &Client, date: &str) -> Result<Vec<StockZtPoolRow>> {
    check_date8(date, "stock_zt_pool_em date")?;
    let params = [
        ("ut", "7eea3edcaed734bea9cbfc24409ed989"),
        ("dpt", "wz.ztzt"),
        ("Pageindex", "0"),
        ("pagesize", "10000"),
        ("sort", "fbt:asc"),
        ("date", date),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_zt_pool_em",
            "https://push2ex.eastmoney.com/getTopicZTPool",
            &params,
        )
        .await?;
    parse_zt_pool(&v)
}

/// Parse an Eastmoney `push2ex` `data.pool` array into [`StockZtPoolRow`]s.
///
/// `data: null` → empty `Vec` (mirrors akshare returning an empty frame).
pub(crate) fn parse_zt_pool(resp: &Value) -> Result<Vec<StockZtPoolRow>> {
    let data = resp.get("data").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "missing data at stock_zt_pool_em".into(),
    })?;
    if data.is_null() {
        return Ok(Vec::new());
    }
    let pool =
        data.get("pool")
            .and_then(|p| p.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing data.pool at stock_zt_pool_em".into(),
            })?;
    let mut out = Vec::with_capacity(pool.len());
    for item in pool {
        let code = fstr(item, "c");
        let name = fstr(item, "n");
        if code.is_empty() || name.is_empty() {
            continue;
        }
        let zt_days = item
            .get("zt")
            .and_then(|z| z.get("days"))
            .and_then(|v| v.as_i64());
        let zt_ct = item
            .get("zt")
            .and_then(|z| z.get("ct"))
            .and_then(|v| v.as_i64());
        let zt_stat = match (zt_days, zt_ct) {
            (Some(d), Some(c)) => Some(format!("{d}/{c}")),
            _ => None,
        };
        out.push(StockZtPoolRow {
            code,
            name,
            price: fnum(item, "p").map(|x| x / 1000.0),
            pct_change: fnum(item, "zdp"),
            amount: fnum(item, "amount"),
            float_mktcap: fnum(item, "ltsz"),
            total_mktcap: fnum(item, "tshare"),
            turnover: fnum(item, "hs"),
            consecutive_boards: fnum(item, "lb"),
            first_time: fmt_time(item.get("fbt").unwrap_or(&Value::Null)),
            last_time: fmt_time(item.get("lbt").unwrap_or(&Value::Null)),
            seal_funds: fnum(item, "fd"),
            explode_count: fnum(item, "zbc"),
            industry: fstr(item, "hy"),
            zt_stat,
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
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let txt = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    #[test]
    fn parses_st_em() {
        let v = fixture("stock_zh_a_st_em.json");
        let rows = parse_st(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "600000");
        assert_eq!(rows[0].name, "浦发银行");
        assert_eq!(rows[0].market, Some(1.0));
        assert_eq!(rows[0].price, Some(3.45));
        assert_eq!(rows[0].pct_change, Some(-4.96));
        assert_eq!(rows[0].change, Some(-0.18));
        assert_eq!(rows[0].volume, Some(123456.0));
        assert_eq!(rows[0].amount, Some(425678900.0));
        assert_eq!(rows[0].high, Some(3.62));
        assert_eq!(rows[0].low, Some(3.40));
        assert_eq!(rows[0].open, Some(3.60));
        assert_eq!(rows[0].pre_close, Some(3.63));
        assert_eq!(rows[0].total_mktcap, Some(101234567890.0));
        assert_eq!(rows[0].float_mktcap, Some(98765432100.0));
        assert_eq!(rows[0].pb, Some(0.45));
        assert_eq!(rows[0].source, "eastmoney");

        assert_eq!(rows[1].code, "000001");
        assert_eq!(rows[1].name, "平安银行ST");
        assert_eq!(rows[1].pe_ttm, Some(-8.3));
        assert_eq!(rows[1].volume_ratio, Some(1.02));
    }

    #[test]
    fn parses_high_low() {
        let v = fixture("stock_a_high_low_statistics.json");
        let rows = parse_high_low(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02");
        assert_eq!(rows[0].close, Some(2974.93));
        assert_eq!(rows[0].high20, Some(123.0));
        assert_eq!(rows[0].low20, Some(45.0));
        assert_eq!(rows[0].high60, Some(200.0));
        assert_eq!(rows[0].low60, Some(80.0));
        assert_eq!(rows[0].high120, Some(300.0));
        assert_eq!(rows[0].low120, Some(150.0));
        assert_eq!(rows[0].source, "legulegu");
        assert_eq!(rows[1].date, "2024-01-03");
        assert_eq!(rows[1].close, Some(2967.25));
    }

    #[test]
    fn high_low_request_rejects_unsupported() {
        assert!(high_low_request("bogus").is_err());
        assert!(high_low_request("").is_err());
        let (url, source) = high_low_request("all").unwrap();
        assert_eq!(source, "legulegu");
        assert_eq!(
            url,
            "https://www.legulegu.com/stockdata/member-ship/get-high-low-statistics/all"
        );
        assert!(high_low_request("hs300").is_ok());
    }

    #[test]
    fn parses_below_net_asset() {
        let v = fixture("stock_a_below_net_asset_statistics.json");
        let rows = parse_below_net_asset(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02");
        assert_eq!(rows[0].below_net_asset, Some(350.0));
        assert_eq!(rows[0].total_company, Some(5000.0));
        assert!((rows[0].below_net_asset_ratio.unwrap() - 0.07).abs() < 1e-9);
        assert_eq!(rows[1].below_net_asset, Some(360.0));
        assert_eq!(rows[1].total_company, Some(5002.0));
        assert!((rows[1].below_net_asset_ratio.unwrap() - 360.0 / 5002.0).abs() < 1e-9);
    }

    #[test]
    fn parses_account_statistics() {
        let v = fixture("stock_account_statistics_em.json");
        let rows = parse_account_statistics(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].stat_date, "2024-01");
        assert_eq!(rows[0].new_investors, Some(12.34));
        assert_eq!(rows[0].new_investors_mom, Some(5.6));
        assert_eq!(rows[0].new_investors_yoy, Some(3.2));
        assert_eq!(rows[0].end_total, Some(22000.5));
        assert_eq!(rows[0].end_a_accounts, Some(21000.3));
        assert_eq!(rows[0].end_b_accounts, Some(1000.2));
        assert_eq!(rows[0].sh_close, Some(2950.12));
        assert_eq!(rows[0].sh_pct, Some(0.85));
        assert_eq!(rows[0].total_mktcap, Some(800000.0));
        assert_eq!(rows[0].avg_mktcap, Some(36.36));
        assert_eq!(rows[1].stat_date, "2023-12");
        assert_eq!(rows[1].sh_pct, Some(-0.5));
    }

    #[test]
    fn parses_zt_pool() {
        let v = fixture("stock_zt_pool_em.json");
        let rows = parse_zt_pool(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "600519");
        assert_eq!(rows[0].name, "贵州茅台");
        // price is raw * 1000 → ÷1000
        assert_eq!(rows[0].price, Some(1685.0));
        assert_eq!(rows[0].pct_change, Some(10.0));
        assert_eq!(rows[0].consecutive_boards, Some(2.0));
        assert_eq!(rows[0].first_time, Some("093500".to_string()));
        assert_eq!(rows[0].last_time, Some("094000".to_string()));
        assert_eq!(rows[0].seal_funds, Some(300000000.0));
        assert_eq!(rows[0].explode_count, Some(1.0));
        assert_eq!(rows[0].industry, "白酒");
        assert_eq!(rows[0].zt_stat, Some("2/3".to_string()));

        assert_eq!(rows[1].code, "000001");
        assert_eq!(rows[1].price, Some(123.0));
        assert_eq!(rows[1].first_time, Some("1000000".to_string()));
        assert_eq!(rows[1].zt_stat, Some("1/1".to_string()));
    }

    #[test]
    fn parses_zt_pool_null_data() {
        let v = fixture("stock_zt_pool_em_empty.json");
        let rows = parse_zt_pool(&v).unwrap();
        assert!(rows.is_empty());
    }
}
