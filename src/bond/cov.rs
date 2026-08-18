//! 沪深可转债行情 (Sina / Eastmoney) — port of `akshare/bond/bond_zh_cov.py`.
//!
//! Ported public functions (akshare source line numbers):
//! - [`bond_zh_hs_cov_spot`]   — `akshare/bond/bond_zh_cov.py:46`  (Sina `getHQNodeDataSimple`)
//! - [`bond_zh_hs_cov_daily`]  — `akshare/bond/bond_zh_cov.py:65`  (daily kline)
//! - [`bond_zh_hs_cov_min`]    — `akshare/bond/bond_zh_cov.py:131` (intraday / minute kline)
//! - [`bond_zh_hs_cov_pre_min`]- `akshare/bond/bond_zh_cov.py:264` (pre-market minute trends)
//! - [`bond_zh_cov_info`]      — `akshare/bond/bond_zh_cov.py:542` (convertible-bond basic info)
//!
//! Source notes / deviations:
//! - `spot` uses the Sina `Market_Center.getHQNodeDataSimple` feed (pure JSON, no token/cookie),
//!   paginated via the `getHQNodeStockCountSimple` count endpoint (80 rows/page), exactly like akshare.
//! - akshare's `bond_zh_hs_cov_daily` originally calls Sina's `klc_kl.js` endpoint, which returns a
//!   JS-encrypted payload decoded with a `py_mini_racer` VM (`hk_js_decode`). That decode is not
//!   portable to a pure-Rust offline build, so we port `daily` against Eastmoney's `push2his` kline
//!   feed (`data.klines`), which yields the identical column set
//!   (date, open, close, high, low, volume, amount, amplitude, pct_change, change, turnover).
//! - `min` / `pre_min` / `info` already hit Eastmoney in akshare and are ported as-is
//!   (`push2his` kline, `push2` trends2, `datacenter-web` report). No token/cookie required.
//!
//! NOT ported (skipped per task — already implemented elsewhere or out of scope):
//! - `_get_zh_bond_hs_cov_page_count` (`bond_zh_cov.py:28`) / `_code_id_map` (`bond_zh_cov.py:89`)
//!   — private helpers; their logic is inlined where needed (page count for `spot`).
//! - `bond_zh_cov` (`bond_zh_cov.py:309`) — ported in `src/bond/eastmoney.rs`.
//! - `bond_cov_comparison` (`bond_zh_cov.py:465`) — ported in `src/bond/eastmoney.rs`.
//! - `bond_zh_cov_value_analysis` (`bond_zh_cov.py:627`) — ported in `src/bond/eastmoney.rs`.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};
use crate::core::json::*;

const SOURCE_SINA: &str = "sina";

// --- Sina spot (getHQNodeDataSimple) ---------------------------------------

const SPOT_NODE: &str = "hskzz_z";
const SPOT_URL: &str = "http://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeDataSimple";
const SPOT_COUNT_URL: &str = "http://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeStockCountSimple";
const SPOT_PAGE_SIZE: u32 = 80;

// --- Eastmoney kline / trends / datacenter ---------------------------------

const KLINE_HIS_URL: &str = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
const TRENDS_URL: &str = "https://push2.eastmoney.com/api/qt/stock/trends2/get";
const INFO_URL: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";
const INFO_REPORT: &str = "RPT_BOND_CB_LIST";
const INFO_QUOTE_COLUMNS: &str = "f2~01~CONVERT_STOCK_CODE~CONVERT_STOCK_PRICE,\
f235~10~SECURITY_CODE~TRANSFER_PRICE,f236~10~SECURITY_CODE~TRANSFER_VALUE,\
f2~10~SECURITY_CODE~CURRENT_BOND_PRICE,f237~10~SECURITY_CODE~TRANSFER_PREMIUM_RATIO,\
f239~10~SECURITY_CODE~RESALE_TRIG_PRICE,f240~10~SECURITY_CODE~REDEEM_TRIG_PRICE,\
f23~01~CONVERT_STOCK_CODE~PBV_RATIO";
const KLINE_FIELDS1: &str = "f1,f2,f3,f4,f5";
const KLINE_FIELDS2: &str = "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61";
const KLINE_UT: &str = "7eea3edcaed734bea9cbfc24409ed989";
const TRENDS_FIELDS1: &str = "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f11,f12,f13";
const TRENDS_FIELDS2: &str = "f51,f52,f53,f54,f55,f56,f57,f58";

// ===========================================================================
// Row types
// ===========================================================================

/// 沪深可转债实时行情 (`bond_zh_hs_cov_spot`).
///
/// Field names follow Sina's `getHQNodeDataSimple` feed.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondZhHsCovSpot {
    /// 代码 — `code` (bare numeric code; market prefix stripped)
    pub symbol: String,
    /// 名称 — `name`
    pub name: String,
    /// 最新价 — `trade`
    pub price: Option<f64>,
    /// 涨跌额 — `pricechange`
    pub change: Option<f64>,
    /// 涨跌幅(%) — `changepercent`
    pub pct_change: Option<f64>,
    /// 今开 — `open`
    pub open: Option<f64>,
    /// 最高 — `high`
    pub high: Option<f64>,
    /// 最低 — `low`
    pub low: Option<f64>,
    /// 昨收 — `settlement`
    pub pre_close: Option<f64>,
    /// 成交量 — `volume`
    pub volume: Option<f64>,
    /// 成交额 — `amount`
    pub amount: Option<f64>,
    /// 换手率(%) — `turnoverratio`
    pub turnover_rate: Option<f64>,
    pub source: &'static str,
}

/// 沪深可转债日行情 (`bond_zh_hs_cov_daily`).
///
/// Parsed from Eastmoney `data.klines` (comma-joined: date,open,close,high,low,
/// volume,amount,amplitude,pct_change,change,turnover).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondZhHsCovDaily {
    /// 日期 — `f51` (kline time, `YYYY-MM-DD`)
    pub date: String,
    /// 开盘 — `f52`
    pub open: Option<f64>,
    /// 收盘 — `f53`
    pub close: Option<f64>,
    /// 最高 — `f54`
    pub high: Option<f64>,
    /// 最低 — `f55`
    pub low: Option<f64>,
    /// 成交量 — `f56`
    pub volume: Option<f64>,
    /// 成交额 — `f57`
    pub amount: Option<f64>,
    /// 振幅(%) — `f58`
    pub amplitude: Option<f64>,
    /// 涨跌幅(%) — `f59`
    pub pct_change: Option<f64>,
    /// 涨跌额 — `f60`
    pub change: Option<f64>,
    /// 换手率(%) — `f61`
    pub turnover: Option<f64>,
    pub source: &'static str,
}

/// 沪深可转债分时/分钟行情 (`bond_zh_hs_cov_min` / `bond_zh_hs_cov_pre_min`).
///
/// `period == "1"` (or `pre_min`) parses Eastmoney `data.trends` (8 fields: the
/// `amplitude`/`pct_change`/`change`/`turnover` columns are `None`); any other
/// period parses `data.klines` (11 fields: `latest_price` is `None`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondZhHsCovMin {
    /// 时间 — `f51`
    pub datetime: String,
    /// 开盘 — `f52`
    pub open: Option<f64>,
    /// 收盘 — `f53`
    pub close: Option<f64>,
    /// 最高 — `f54`
    pub high: Option<f64>,
    /// 最低 — `f55`
    pub low: Option<f64>,
    /// 成交量 — `f56`
    pub volume: Option<f64>,
    /// 成交额 — `f57`
    pub amount: Option<f64>,
    /// 最新价 — `f58` (trends only)
    pub latest_price: Option<f64>,
    /// 振幅(%) — `f58` (klines only)
    pub amplitude: Option<f64>,
    /// 涨跌幅(%) — `f59` (klines only)
    pub pct_change: Option<f64>,
    /// 涨跌额 — `f60` (klines only)
    pub change: Option<f64>,
    /// 换手率(%) — `f61` (klines only)
    pub turnover: Option<f64>,
    pub source: &'static str,
}

/// 可转债资料-基本信息 (`bond_zh_cov_info`).
///
/// Field names follow Eastmoney datacenter `RPT_BOND_CB_LIST` (filtered by code).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondZhCovInfo {
    /// 债券代码 — `SECURITY_CODE`
    pub security_code: String,
    /// 债券简称 — `SECURITY_NAME_ABBR`
    pub security_name: Option<String>,
    /// 正股代码 — `CONVERT_STOCK_CODE`
    pub convert_stock_code: Option<String>,
    /// 正股简称 — `SECURITY_SHORT_NAME`
    pub convert_stock_name: Option<String>,
    /// 信用评级 — `RATING`
    pub rating: Option<String>,
    /// 转股价 — `TRANSFER_PRICE` (quote-injected)
    pub transfer_price: Option<f64>,
    /// 转股价值 — `TRANSFER_VALUE` (quote-injected)
    pub transfer_value: Option<f64>,
    /// 债现价 — `CURRENT_BOND_PRICE` (quote-injected)
    pub current_bond_price: Option<f64>,
    /// 转股溢价率 — `TRANSFER_PREMIUM_RATIO` (quote-injected)
    pub transfer_premium_ratio: Option<f64>,
    /// 发行规模(亿元) — `ACTUAL_ISSUE_SCALE`
    pub issue_scale: Option<f64>,
    /// 申购日期 — `PUBLIC_START_DATE`
    pub public_start_date: Option<String>,
    /// 上市时间 — `LISTING_DATE`
    pub listing_date: Option<String>,
    pub source: &'static str,
}

// ===========================================================================
// Public functions
// ===========================================================================

/// 沪深可转债实时行情 (`bond_zh_hs_cov_spot`, `bond_zh_cov.py:46`).
///
/// Walks Sina's `getHQNodeDataSimple` pages (80 rows/page) for node `hskzz_z`.
pub async fn bond_zh_hs_cov_spot(client: &Client) -> Result<Vec<BondZhHsCovSpot>> {
    let count_text = client
        .get_text(
            SOURCE_SINA,
            "bond_zh_hs_cov_spot",
            SPOT_COUNT_URL,
            &[("node", SPOT_NODE)],
            None,
        )
        .await?;
    let count: u32 = extract_first_number(&count_text)
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "could not parse total convertible-bond count".into(),
        })?
        .max(1);
    let total_pages = count.div_ceil(SPOT_PAGE_SIZE);

    let mut out = Vec::new();
    for page in 1..=total_pages {
        let page_s = page.to_string();
        let params = [
            ("page", page_s.as_str()),
            ("num", "80"),
            ("sort", "symbol"),
            ("asc", "1"),
            ("node", SPOT_NODE),
            ("_s_r_a", "page"),
        ];
        let text = client
            .get_text(SOURCE_SINA, "bond_zh_hs_cov_spot", SPOT_URL, &params, None)
            .await?;
        let v: Value = serde_json::from_str(&text).map_err(|e| Error::Parse {
            endpoint: "bond_zh_hs_cov_spot",
            message: e.to_string(),
        })?;
        out.extend(parse_cov_spot(&v)?);
    }
    Ok(out)
}

/// 沪深可转债历史日行情 (`bond_zh_hs_cov_daily`, `bond_zh_cov.py:65`).
///
/// `symbol` is a market-prefixed convertible-bond code, e.g. `"sh010107"`.
/// Returns daily klines from Eastmoney `push2his` (`data.klines`).
pub async fn bond_zh_hs_cov_daily(client: &Client, symbol: &str) -> Result<Vec<BondZhHsCovDaily>> {
    let sec = secid(symbol)?;
    let params = [
        ("secid", sec.as_str()),
        ("klt", "101"),
        ("fqt", "0"),
        ("lmt", "1000"),
        ("end", "20500101"),
        ("iscr", "0"),
        ("iscca", "1"),
        ("fields1", KLINE_FIELDS1),
        ("fields2", KLINE_FIELDS2),
        ("ut", KLINE_UT),
        ("forcect", "1"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "bond_zh_hs_cov_daily",
            KLINE_HIS_URL,
            &params,
        )
        .await?;
    parse_cov_klines(&v)
}

/// 沪深可转债分时/分钟行情 (`bond_zh_hs_cov_min`, `bond_zh_cov.py:131`).
///
/// `symbol` is a market-prefixed convertible-bond code, e.g. `"sz128039"`.
/// `period` is one of `{"1","5","15","30","60"}`. `period == "1"` returns
/// intraday trends (`data.trends`); otherwise returns minute klines (`data.klines`).
pub async fn bond_zh_hs_cov_min(
    client: &Client,
    symbol: &str,
    period: &str,
) -> Result<Vec<BondZhHsCovMin>> {
    let sec = secid(symbol)?;
    if period == "1" {
        let params = [
            ("secid", sec.as_str()),
            ("fields1", TRENDS_FIELDS1),
            ("fields2", TRENDS_FIELDS2),
            ("iscr", "0"),
            ("iscca", "0"),
            ("ndays", "1"),
            ("ut", KLINE_UT),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, "bond_zh_hs_cov_min", TRENDS_URL, &params)
            .await?;
        parse_cov_trends(&v)
    } else {
        let params = [
            ("secid", sec.as_str()),
            ("klt", period),
            ("fqt", "0"),
            ("lmt", "66"),
            ("end", "20500101"),
            ("iscr", "0"),
            ("iscca", "1"),
            ("fields1", KLINE_FIELDS1),
            ("fields2", KLINE_FIELDS2),
            ("ut", KLINE_UT),
            ("forcect", "1"),
        ];
        let v = client
            .get_json(
                SOURCE_EASTMONEY,
                "bond_zh_hs_cov_min",
                KLINE_HIS_URL,
                &params,
            )
            .await?;
        parse_cov_klines_min(&v)
    }
}

/// 沪深可转债盘前分时行情 (`bond_zh_hs_cov_pre_min`, `bond_zh_cov.py:264`).
///
/// `symbol` is a market-prefixed convertible-bond code, e.g. `"sh113570"`.
/// Returns pre-market trends from Eastmoney `push2` (`data.trends`).
pub async fn bond_zh_hs_cov_pre_min(client: &Client, symbol: &str) -> Result<Vec<BondZhHsCovMin>> {
    let sec = secid(symbol)?;
    let params = [
        ("fields1", TRENDS_FIELDS1),
        ("fields2", TRENDS_FIELDS2),
        ("ndays", "1"),
        ("iscr", "1"),
        ("iscca", "0"),
        ("secid", sec.as_str()),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "bond_zh_hs_cov_pre_min",
            TRENDS_URL,
            &params,
        )
        .await?;
    parse_cov_trends(&v)
}

/// 可转债资料-基本信息 (`bond_zh_cov_info`, `bond_zh_cov.py:542`).
///
/// `symbol` is a convertible-bond code, e.g. `"123121"`. Returns the Eastmoney
/// `RPT_BOND_CB_LIST` record (indicator `基本信息`) for that code.
pub async fn bond_zh_cov_info(client: &Client, symbol: &str) -> Result<Vec<BondZhCovInfo>> {
    if symbol.is_empty() {
        return Err(Error::InvalidParam("symbol must not be empty".into()));
    }
    let filter = format!("(SECURITY_CODE=\"{symbol}\")");
    let params = [
        ("reportName", INFO_REPORT),
        ("columns", "ALL"),
        ("quoteColumns", INFO_QUOTE_COLUMNS),
        ("quoteType", "0"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "bond_zh_cov_info", INFO_URL, &params)
        .await?;
    parse_cov_info(&v)
}

// ===========================================================================
// Parsers
// ===========================================================================

pub(crate) fn parse_cov_spot(resp: &Value) -> Result<Vec<BondZhHsCovSpot>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "expected a JSON array".into(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(BondZhHsCovSpot {
            symbol: norm_code(opt_str_or(item, "code", ""), opt_str_or(item, "symbol", "")),
            name: opt_str_or(item, "name", ""),
            price: opt_f64(item, "trade"),
            change: opt_f64(item, "pricechange"),
            pct_change: opt_f64(item, "changepercent"),
            open: opt_f64(item, "open"),
            high: opt_f64(item, "high"),
            low: opt_f64(item, "low"),
            pre_close: opt_f64(item, "settlement"),
            volume: opt_f64(item, "volume"),
            amount: opt_f64(item, "amount"),
            turnover_rate: opt_f64(item, "turnoverratio"),
            source: SOURCE_SINA,
        });
    }
    Ok(out)
}

pub(crate) fn parse_cov_klines(resp: &Value) -> Result<Vec<BondZhHsCovDaily>> {
    let klines = klines_array(resp)?;
    let mut out = Vec::with_capacity(klines.len());
    for item in klines {
        let s = item.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "kline entry is not a string".into(),
        })?;
        let p = split_csv(s);
        out.push(BondZhHsCovDaily {
            date: field(&p, 0).unwrap_or_default().to_string(),
            open: num_at(&p, 1),
            close: num_at(&p, 2),
            high: num_at(&p, 3),
            low: num_at(&p, 4),
            volume: num_at(&p, 5),
            amount: num_at(&p, 6),
            amplitude: num_at(&p, 7),
            pct_change: num_at(&p, 8),
            change: num_at(&p, 9),
            turnover: num_at(&p, 10),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

pub(crate) fn parse_cov_klines_min(resp: &Value) -> Result<Vec<BondZhHsCovMin>> {
    let klines = klines_array(resp)?;
    let mut out = Vec::with_capacity(klines.len());
    for item in klines {
        let s = item.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "kline entry is not a string".into(),
        })?;
        let p = split_csv(s);
        out.push(BondZhHsCovMin {
            datetime: field(&p, 0).unwrap_or_default().to_string(),
            open: num_at(&p, 1),
            close: num_at(&p, 2),
            high: num_at(&p, 3),
            low: num_at(&p, 4),
            volume: num_at(&p, 5),
            amount: num_at(&p, 6),
            latest_price: None,
            amplitude: num_at(&p, 7),
            pct_change: num_at(&p, 8),
            change: num_at(&p, 9),
            turnover: num_at(&p, 10),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

pub(crate) fn parse_cov_trends(resp: &Value) -> Result<Vec<BondZhHsCovMin>> {
    let trends = resp
        .get("data")
        .and_then(|d| d.get("trends"))
        .and_then(|t| t.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.trends".into(),
        })?;
    let mut out = Vec::with_capacity(trends.len());
    for item in trends {
        let s = item.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "trend entry is not a string".into(),
        })?;
        let p = split_csv(s);
        out.push(BondZhHsCovMin {
            datetime: field(&p, 0).unwrap_or_default().to_string(),
            open: num_at(&p, 1),
            close: num_at(&p, 2),
            high: num_at(&p, 3),
            low: num_at(&p, 4),
            volume: num_at(&p, 5),
            amount: num_at(&p, 6),
            latest_price: num_at(&p, 7),
            amplitude: None,
            pct_change: None,
            change: None,
            turnover: None,
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

pub(crate) fn parse_cov_info(resp: &Value) -> Result<Vec<BondZhCovInfo>> {
    let data = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data".into(),
        })?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(BondZhCovInfo {
            security_code: opt_str_or(item, "SECURITY_CODE", ""),
            security_name: fstr_opt(item, "SECURITY_NAME_ABBR"),
            convert_stock_code: fstr_opt(item, "CONVERT_STOCK_CODE"),
            convert_stock_name: fstr_opt(item, "SECURITY_SHORT_NAME"),
            rating: fstr_opt(item, "RATING"),
            transfer_price: opt_f64(item, "TRANSFER_PRICE"),
            transfer_value: opt_f64(item, "TRANSFER_VALUE"),
            current_bond_price: opt_f64(item, "CURRENT_BOND_PRICE"),
            transfer_premium_ratio: opt_f64(item, "TRANSFER_PREMIUM_RATIO"),
            issue_scale: opt_f64(item, "ACTUAL_ISSUE_SCALE"),
            public_start_date: fstr_opt(item, "PUBLIC_START_DATE"),
            listing_date: fstr_opt(item, "LISTING_DATE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// Shared helpers
// ===========================================================================

/// Build an Eastmoney `secid` (`"1.010107"` for `sh`, `"0.xxx"` for `sz`).
fn secid(symbol: &str) -> Result<String> {
    if symbol.len() < 3 {
        return Err(Error::InvalidParam(format!(
            "symbol `{symbol}` must be market-prefixed, e.g. sh010107"
        )));
    }
    let (market, code) = symbol.split_at(2);
    let m = match market {
        "sh" => "1",
        "sz" => "0",
        other => {
            return Err(Error::InvalidParam(format!(
                "unsupported market prefix `{other}` (expected sh/sz)"
            )));
        }
    };
    Ok(format!("{m}.{code}"))
}

fn klines_array(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("data")
        .and_then(|d| d.get("klines"))
        .and_then(|k| k.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.klines".into(),
        })
}

fn split_csv(s: &str) -> Vec<&str> {
    s.split(',').collect()
}

/// Index into a comma-split row, returning `None` for a missing/empty segment.
fn field<'a>(parts: &'a [&'a str], i: usize) -> Option<&'a str> {
    parts.get(i).map(|s| s.trim()).filter(|s| !s.is_empty())
}

fn num_at(parts: &[&str], i: usize) -> Option<f64> {
    field(parts, i).and_then(|v| v.parse::<f64>().ok())
}

/// Sina's `code`/`symbol` fields may carry a market prefix; keep the bare code.
fn norm_code(primary: String, fallback: String) -> String {
    let s = if !primary.is_empty() {
        primary
    } else {
        fallback
    };
    let s = s.trim();
    if let Some(rest) = s
        .strip_prefix("sh")
        .or_else(|| s.strip_prefix("sz"))
        .or_else(|| s.strip_prefix("bj"))
        && rest.chars().all(|c| c.is_ascii_digit())
    {
        return rest.to_string();
    }
    s.to_string()
}

/// Pull the first run of digits out of a response body (Sina's count endpoint).
fn extract_first_number(text: &str) -> Option<u32> {
    let digits: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse::<u32>().ok()
}

fn fstr_opt(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let txt = std::fs::read_to_string(p).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    #[test]
    fn parses_bond_zh_hs_cov_spot_fixture() {
        let v = fixture("bond_zh_hs_cov_spot.json");
        let rows = parse_cov_spot(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "010107");
        assert_eq!(rows[0].name, "21国债(7)");
        assert_eq!(rows[0].price, Some(101.23));
        assert_eq!(rows[0].pct_change, Some(0.118));
        assert_eq!(rows[0].pre_close, Some(101.11));
        assert_eq!(rows[0].volume, Some(123456.0));
        assert_eq!(rows[0].source, "sina");
        assert_eq!(rows[1].symbol, "123000");
        assert_eq!(rows[1].name, "某某转债");
        assert_eq!(rows[1].price, Some(115.50));
        assert_eq!(rows[1].change, Some(-1.20));
    }

    #[test]
    fn parses_bond_zh_hs_cov_daily_klines_fixture() {
        let v = fixture("bond_zh_hs_cov_daily.json");
        let rows = parse_cov_klines(&v).unwrap();
        assert_eq!(rows.len(), 2);
        // Row 0: full fields
        assert_eq!(rows[0].date, "2024-01-02");
        assert_eq!(rows[0].open, Some(100.0));
        assert_eq!(rows[0].close, Some(101.5));
        assert_eq!(rows[0].high, Some(102.0));
        assert_eq!(rows[0].low, Some(99.5));
        assert_eq!(rows[0].volume, Some(1234567.0));
        assert_eq!(rows[0].amount, Some(125000000.0));
        assert_eq!(rows[0].amplitude, Some(2.5));
        assert_eq!(rows[0].pct_change, Some(1.5));
        assert_eq!(rows[0].change, Some(1.5));
        assert_eq!(rows[0].turnover, Some(0.80));
        assert_eq!(rows[0].source, "eastmoney");
        // Row 1: None (empty open) and 0.0 (volume "0") cases
        assert_eq!(rows[1].date, "2024-01-03");
        assert_eq!(rows[1].open, None);
        assert_eq!(rows[1].close, Some(102.0));
        assert_eq!(rows[1].volume, Some(0.0));
        assert_eq!(rows[1].amplitude, None);
        assert_eq!(rows[1].pct_change, None);
    }

    #[test]
    fn parses_bond_zh_hs_cov_min_klines_fixture() {
        let v = fixture("bond_zh_hs_cov_min.json");
        let rows = parse_cov_klines_min(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].datetime, "2024-01-02 09:30");
        assert_eq!(rows[0].open, Some(100.0));
        assert_eq!(rows[0].close, Some(100.5));
        assert_eq!(rows[0].high, Some(101.0));
        assert_eq!(rows[0].low, Some(99.8));
        assert_eq!(rows[0].volume, Some(5000.0));
        assert_eq!(rows[0].latest_price, None);
        assert_eq!(rows[0].amplitude, Some(0.2));
        assert_eq!(rows[0].pct_change, Some(0.5));
        assert_eq!(rows[0].change, Some(0.5));
        assert_eq!(rows[0].turnover, Some(0.10));
        assert_eq!(rows[1].datetime, "2024-01-02 09:31");
        assert_eq!(rows[1].close, Some(101.0));
    }

    #[test]
    fn parses_bond_zh_hs_cov_min_trends_fixture() {
        let v = fixture("bond_zh_hs_cov_min.json");
        let rows = parse_cov_trends(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].datetime, "2024-01-02 09:30");
        assert_eq!(rows[0].open, Some(100.0));
        assert_eq!(rows[0].close, Some(100.5));
        assert_eq!(rows[0].high, Some(101.0));
        assert_eq!(rows[0].low, Some(99.8));
        assert_eq!(rows[0].volume, Some(5000.0));
        assert_eq!(rows[0].amount, Some(500000.0));
        assert_eq!(rows[0].latest_price, Some(100.5));
        // trends path has no kline-only columns
        assert_eq!(rows[0].amplitude, None);
        assert_eq!(rows[0].pct_change, None);
        assert_eq!(rows[1].latest_price, Some(101.0));
    }

    #[test]
    fn parses_bond_zh_hs_cov_pre_min_fixture() {
        let v = fixture("bond_zh_hs_cov_pre_min.json");
        let rows = parse_cov_trends(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].datetime, "2024-01-02 09:30");
        assert_eq!(rows[0].open, Some(99.0));
        assert_eq!(rows[0].close, Some(99.5));
        assert_eq!(rows[0].high, Some(99.8));
        assert_eq!(rows[0].low, Some(98.9));
        assert_eq!(rows[0].volume, Some(4000.0));
        assert_eq!(rows[0].latest_price, Some(99.5));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].datetime, "2024-01-02 09:31");
        assert_eq!(rows[1].close, Some(100.0));
    }

    #[test]
    fn parses_bond_zh_cov_info_fixture() {
        let v = fixture("bond_zh_cov_info.json");
        let rows = parse_cov_info(&v).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].security_code, "123121");
        assert_eq!(rows[0].security_name.as_deref(), Some("XYZ转债"));
        assert_eq!(rows[0].convert_stock_code.as_deref(), Some("300123"));
        assert_eq!(rows[0].convert_stock_name.as_deref(), Some("XYZ股份"));
        assert_eq!(rows[0].rating.as_deref(), Some("AA"));
        assert_eq!(rows[0].transfer_price, Some(35.59));
        assert_eq!(rows[0].transfer_value, Some(98.19));
        assert_eq!(rows[0].current_bond_price, Some(100.0));
        assert_eq!(rows[0].transfer_premium_ratio, Some(1.83));
        assert_eq!(rows[0].issue_scale, Some(6.0));
        assert_eq!(
            rows[0].public_start_date.as_deref(),
            Some("2021-01-01 00:00:00")
        );
        assert_eq!(rows[0].listing_date.as_deref(), Some("2021-02-01 00:00:00"));
        assert_eq!(rows[0].source, "eastmoney");
    }
}
