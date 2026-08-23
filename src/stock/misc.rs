//! Miscellaneous A-share "misc" endpoints ported from `akshare`.
//!
//! Functions ported (pure-HTTP; no JS engine / encrypted headers required):
//! - `stock_zh_a_hist_min_em` — Eastmoney minute K-lines
//!   (akshare: `akshare/stock_feature/stock_hist_em.py::stock_zh_a_hist_min_em`)
//! - `stock_zh_a_minute` — Sina minute K-lines, unadjusted path only
//!   (akshare: `akshare/stock/stock_zh_a_sina.py::stock_zh_a_minute`)
//! - `stock_zh_a_new` — Sina 次新股 (new-share) list
//!   (akshare: `akshare/stock/stock_zh_a_special.py::stock_zh_a_new`)
//! - `stock_zh_a_stop` — Eastmoney 两网及退市 (delisted / STAQ-net) board
//!   (akshare: `akshare/stock/stock_zh_a_special.py::stock_zh_a_stop_em`)
//! - `stock_summary` — Eastmoney A-share overview (all A-shares via `clist`)
//!   (akshare `stock_summary` is documented as an Eastmoney stock overview; this
//!   ports the Eastmoney A-share `clist` data which is the practical source)
//!
//! Skipped (see report):
//! - `stock_restricted` — no such function exists in this akshare checkout
//!   (no 限售股解禁 endpoint); closest is pledge-ratio (`stock_gpzy_*`).
//! - `stock_zh_a_daily_tx` — identical to akshare `stock_zh_a_hist_tx`, already
//!   ported in `src/stock/hist/tencent.rs` as `daily`; duplicating would be wasteful.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY, SOURCE_SINA};
use crate::core::error::{Error, Result};

// ===========================================================================
// stock_zh_a_hist_min_em — Eastmoney minute K-lines
// ===========================================================================

const HIST_MIN_KLINE_URL: &str = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
const HIST_MIN_TRENDS_URL: &str = "https://push2his.eastmoney.com/api/qt/stock/trends2/get";
const HIST_MIN_UT: &str = "7eea3edcaed734bea9cbfc24409ed989";

/// One Eastmoney minute K-line bar (akshare `stock_zh_a_hist_min_em`).
///
/// The 1-minute (`trends2`) path yields `avg_price`; the 5/15/30/60-minute
/// (`kline`) path yields `amplitude`, `pct_change`, `change`, `turnover`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HistMinRow {
    /// 时间 — timestamp, e.g. "2023-01-03 09:31:00" (1-min) or "2023-01-03" (kline)
    pub time: String,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: Option<f64>,
    pub volume: Option<f64>,
    pub amount: Option<f64>,
    /// 均价 (1-min `trends2` path only)
    pub avg_price: Option<f64>,
    /// 振幅 (kline path only)
    pub amplitude: Option<f64>,
    /// 涨跌幅 (kline path only)
    pub pct_change: Option<f64>,
    /// 涨跌额 (kline path only)
    pub change: Option<f64>,
    /// 换手率 (kline path only)
    pub turnover: Option<f64>,
}

/// Eastmoney minute K-lines.
///
/// `period` is one of `1`/`5`/`15`/`30`/`60`; `adjust` is `""`/`qfq`/`hfq`.
/// `start_date`/`end_date` are accepted for signature parity with akshare but
/// the upstream `kline` path already bounds the window (`beg`/`end`); the
/// `trends2` (1-min) path returns the last 5 sessions.
pub async fn stock_zh_a_hist_min_em(
    client: &Client,
    symbol: &str,
    _start_date: &str,
    _end_date: &str,
    period: &str,
    adjust: &str,
) -> Result<Vec<HistMinRow>> {
    if !matches!(period, "1" | "5" | "15" | "30" | "60") {
        return Err(Error::InvalidParam(format!(
            "period must be one of 1/5/15/30/60, got {period}"
        )));
    }
    let fqt = match adjust {
        "" => "0",
        "qfq" => "1",
        "hfq" => "2",
        other => {
            return Err(Error::InvalidParam(format!(
                "adjust must be '', 'qfq' or 'hfq', got {other}"
            )));
        }
    };
    let market = if symbol.starts_with('6') { 1 } else { 0 };
    let secid = format!("{market}.{symbol}");

    let (url, params, is_trends) = if period == "1" {
        let params = [
            ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f11,f12,f13"),
            ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58"),
            ("ut", HIST_MIN_UT),
            ("ndays", "5"),
            ("iscr", "0"),
            ("secid", secid.as_str()),
        ];
        (HIST_MIN_TRENDS_URL, params.to_vec(), true)
    } else {
        let params = [
            ("fields1", "f1,f2,f3,f4,f5,f6"),
            ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61"),
            ("ut", HIST_MIN_UT),
            ("klt", period),
            ("fqt", fqt),
            ("secid", secid.as_str()),
            ("beg", "0"),
            ("end", "20500000"),
        ];
        (HIST_MIN_KLINE_URL, params.to_vec(), false)
    };

    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_zh_a_hist_min_em", url, &params)
        .await?;
    parse_hist_min(&v, is_trends)
}

/// Map an Eastmoney minute-K-line response to [`HistMinRow`]s.
pub(crate) fn parse_hist_min(resp: &Value, is_trends: bool) -> Result<Vec<HistMinRow>> {
    let data = match resp.get("data") {
        Some(d) if !d.is_null() => d,
        _ => return Ok(Vec::new()),
    };
    let arr = if is_trends {
        data.get("trends").and_then(|v| v.as_array())
    } else {
        data.get("klines").and_then(|v| v.as_array())
    };
    let arr = match arr {
        Some(a) => a,
        None => return Ok(Vec::new()),
    };

    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let s = item.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "kline entry is not a CSV string".into(),
        })?;
        let p: Vec<&str> = s.split(',').collect();
        if is_trends {
            if p.len() < 8 {
                continue;
            }
            out.push(HistMinRow {
                time: p[0].to_string(),
                open: num_cell(p[1]),
                high: num_cell(p[3]),
                low: num_cell(p[4]),
                close: num_cell(p[2]),
                volume: num_cell(p[5]),
                amount: num_cell(p[6]),
                avg_price: num_cell(p[7]),
                amplitude: None,
                pct_change: None,
                change: None,
                turnover: None,
            });
        } else {
            if p.len() < 11 {
                continue;
            }
            out.push(HistMinRow {
                time: p[0].to_string(),
                open: num_cell(p[1]),
                high: num_cell(p[3]),
                low: num_cell(p[4]),
                close: num_cell(p[2]),
                volume: num_cell(p[5]),
                amount: num_cell(p[6]),
                avg_price: None,
                amplitude: num_cell(p[7]),
                pct_change: num_cell(p[8]),
                change: num_cell(p[9]),
                turnover: num_cell(p[10]),
            });
        }
    }
    Ok(out)
}

// ===========================================================================
// stock_zh_a_minute — Sina minute K-lines (unadjusted path only)
// ===========================================================================

const MINUTE_SINA_URL: &str =
    "https://quotes.sina.cn/cn/api/jsonp_v2.php/=/CN_MarketDataService.getKLineData";

/// One Sina minute bar (akshare `stock_zh_a_minute`, unadjusted).
#[derive(Debug, Clone, serde::Serialize)]
pub struct MinuteRow {
    /// 时间 (day)
    pub day: String,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: Option<f64>,
    pub volume: Option<f64>,
    pub amount: Option<f64>,
}

/// Sina minute K-lines.
///
/// Only the unadjusted (`adjust = ""`) path is supported. akshare's `qfq`/`hfq`
/// paths call `stock_zh_a_daily`, which requires the Sina daily factor computed
/// via `py_mini_racer` (JS engine) — intentionally skipped.
pub async fn stock_zh_a_minute(
    client: &Client,
    symbol: &str,
    period: &str,
    adjust: &str,
) -> Result<Vec<MinuteRow>> {
    if !matches!(period, "1" | "5" | "15" | "30" | "60") {
        return Err(Error::InvalidParam(format!(
            "period must be one of 1/5/15/30/60, got {period}"
        )));
    }
    if !adjust.is_empty() {
        return Err(Error::InvalidParam(
            "stock_zh_a_minute qfq/hfq adjustment requires the Sina daily factor \
             (py_mini_racer JS signing) — not implemented; use adjust=\"\""
                .into(),
        ));
    }
    let params = [
        ("symbol", symbol),
        ("scale", period),
        ("ma", "no"),
        ("datalen", "1970"),
    ];
    let text = client
        .get_text(
            SOURCE_SINA,
            "stock_zh_a_minute",
            MINUTE_SINA_URL,
            &params,
            None,
        )
        .await?;
    parse_minute(&text)
}

/// Parse a Sina JSONP minute response into [`MinuteRow`]s.
///
/// The response is JSONP (`var x=(...)`); we slice from the first `[` to the
/// last `]` and parse the inner JSON array. Each element is an object keyed by
/// `day`/`open`/`high`/`low`/`close`/`volume`/`amount`.
pub(crate) fn parse_minute(text: &str) -> Result<Vec<MinuteRow>> {
    let start = text.find('[').ok_or_else(|| Error::Parse {
        endpoint: "stock_zh_a_minute",
        message: "no JSON array in response".into(),
    })?;
    let end = text.rfind(']').ok_or_else(|| Error::Parse {
        endpoint: "stock_zh_a_minute",
        message: "no closing bracket in response".into(),
    })?;
    if end <= start {
        return Err(Error::Parse {
            endpoint: "stock_zh_a_minute",
            message: "malformed JSON array".into(),
        });
    }
    let arr: Value = serde_json::from_str(&text[start..=end]).map_err(Error::Json)?;
    let arr = arr.as_array().ok_or_else(|| Error::Parse {
        endpoint: "stock_zh_a_minute",
        message: "expected a JSON array".into(),
    })?;

    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(MinuteRow {
            day: str_field(item, "day"),
            open: num_field(item, "open"),
            high: num_field(item, "high"),
            low: num_field(item, "low"),
            close: num_field(item, "close"),
            volume: num_field(item, "volume"),
            amount: num_field(item, "amount"),
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_zh_a_new — Sina 次新股 (new-share) list
// ===========================================================================

const NEW_SHARE_COUNT_URL: &str = "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeStockCount";
const NEW_SHARE_DATA_URL: &str = "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeData";

/// One Sina new-share entry (akshare `stock_zh_a_new`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct NewShareRow {
    /// symbol, e.g. "sh600519"
    pub symbol: String,
    /// code, e.g. "600519"
    pub code: String,
    pub name: String,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub volume: Option<f64>,
    pub amount: Option<f64>,
    /// 总市值
    pub mktcap: Option<f64>,
    /// 换手率
    pub turnover_ratio: Option<f64>,
}

/// Sina 次新股 (new-share) list.
///
/// Mirrors akshare: fetch the total count, then paginate `getHQNodeData`
/// (`num=80`). NOTE: Sina serves this endpoint as `gb2312`; the crate's
/// `Client` decodes per `Content-Type`/UTF-8, so CJK `name` values may be
/// mangled live — numeric fields are unaffected. Pure-JSON, no JS required.
pub async fn stock_zh_a_new(client: &Client) -> Result<Vec<NewShareRow>> {
    let count_v = client
        .get_json(
            SOURCE_SINA,
            "stock_zh_a_new",
            NEW_SHARE_COUNT_URL,
            &[("node", "new_stock")],
        )
        .await?;
    let total = match count_v.as_u64() {
        Some(n) => n,
        None => count_v
            .as_str()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_SINA,
                message: "new-share count is not an integer".into(),
            })?,
    };
    let pages = total.div_ceil(80);

    let mut out = Vec::new();
    for page in 1..=pages {
        let page_s = page.to_string();
        let params = [
            ("page", page_s.as_str()),
            ("num", "80"),
            ("sort", "symbol"),
            ("asc", "1"),
            ("node", "new_stock"),
            ("symbol", ""),
            ("_s_r_a", "page"),
        ];
        let v = client
            .get_json(SOURCE_SINA, "stock_zh_a_new", NEW_SHARE_DATA_URL, &params)
            .await?;
        out.extend(parse_new_shares(&v)?);
    }
    Ok(out)
}

/// Map a Sina new-share page (JSON array of objects) to [`NewShareRow`]s.
pub(crate) fn parse_new_shares(resp: &Value) -> Result<Vec<NewShareRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "new-share response is not a JSON array".into(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(NewShareRow {
            symbol: str_field(item, "symbol"),
            code: str_field(item, "code"),
            name: str_field(item, "name"),
            open: num_field(item, "open"),
            high: num_field(item, "high"),
            low: num_field(item, "low"),
            volume: num_field(item, "volume"),
            amount: num_field(item, "amount"),
            mktcap: num_field(item, "mktcap"),
            turnover_ratio: num_field(item, "turnoverratio"),
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_zh_a_stop — Eastmoney 两网及退市 (delisted / STAQ-net) board
// ===========================================================================

const STOP_UT: &str = "bd1d9ddb04089700cf9c27f6f7426281";
const STOP_FIELDS: &str = "f2,f3,f4,f5,f6,f7,f8,f9,f10,f12,f13,f14,f15,f16,f17,f18,f20,f21,f23,f24,f25,f22,f11,f62,f128,f136,f115,f152";

/// One Eastmoney delisted / STAQ-net board entry (akshare `stock_zh_a_stop_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StopRow {
    pub code: String,
    pub name: String,
    /// 最新价 (f2)
    pub price: Option<f64>,
    /// 涨跌幅 (f3)
    pub pct_change: Option<f64>,
    /// 涨跌额 (f4)
    pub change: Option<f64>,
    /// 成交量 (f5)
    pub volume: Option<f64>,
    /// 成交额 (f6)
    pub amount: Option<f64>,
    /// 振幅 (f7)
    pub amplitude: Option<f64>,
    /// 换手率 (f8)
    pub turnover_rate: Option<f64>,
    /// 市盈率-动态 (f9)
    pub pe: Option<f64>,
    /// 量比 (f10)
    pub volume_ratio: Option<f64>,
    /// 最高 (f15)
    pub high: Option<f64>,
    /// 最低 (f16)
    pub low: Option<f64>,
    /// 今开 (f17)
    pub open: Option<f64>,
    /// 昨收 (f18)
    pub pre_close: Option<f64>,
    /// 市净率 (f24)
    pub pb: Option<f64>,
}

/// Eastmoney 两网及退市 (delisted / STAQ-net) board.
///
/// Ports akshare `stock_zh_a_stop_em` (`fs = "m:0 s:3"`). Returns the first
/// page (`pz=100`); the board is small so pagination is not needed here.
pub async fn stock_zh_a_stop(client: &Client) -> Result<Vec<StopRow>> {
    let params = [
        ("pn", "1"),
        ("pz", "100"),
        ("po", "1"),
        ("np", "1"),
        ("ut", STOP_UT),
        ("fltt", "2"),
        ("invt", "2"),
        ("fid", "f3"),
        ("fs", "m:0 s:3"),
        ("fields", STOP_FIELDS),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_zh_a_stop", &crate::core::eastmoney_push::push2_url("/api/qt/clist/get").await, &params)
        .await?;
    parse_stop(&v)
}

/// Map an Eastmoney `clist` `diff` to [`StopRow`]s.
pub(crate) fn parse_stop(resp: &Value) -> Result<Vec<StopRow>> {
    let diff = resp
        .get("data")
        .and_then(|d| d.get("diff"))
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff".into(),
        })?;
    let mut out = Vec::new();
    for item in diff_items(diff) {
        out.push(StopRow {
            code: str_field(item, "f12"),
            name: str_field(item, "f14"),
            price: num_field(item, "f2"),
            pct_change: num_field(item, "f3"),
            change: num_field(item, "f4"),
            volume: num_field(item, "f5"),
            amount: num_field(item, "f6"),
            amplitude: num_field(item, "f7"),
            turnover_rate: num_field(item, "f8"),
            pe: num_field(item, "f9"),
            volume_ratio: num_field(item, "f10"),
            high: num_field(item, "f15"),
            low: num_field(item, "f16"),
            open: num_field(item, "f17"),
            pre_close: num_field(item, "f18"),
            pb: num_field(item, "f24"),
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_summary — Eastmoney A-share overview (all A-shares via `clist`)
// ===========================================================================

const SUMMARY_UT: &str = "bd1d9ddb04089700cf9c27f6f7426281";
const SUMMARY_FS: &str = "m:0 t:6,m:0 t:80,m:1 t:2,m:1 t:23,m:0 t:81 s:2048";
const SUMMARY_FIELDS: &str =
    "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f12,f13,f14,f15,f16,f17,f18,f20,f21,f23,f24";
const SUMMARY_PAGE_SIZE: u32 = 1000;

/// One A-share overview row (Eastmoney `clist`): code, name and key quote fields.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SummaryRow {
    pub code: String,
    pub name: String,
    /// 最新价 (f2)
    pub price: Option<f64>,
    /// 涨跌幅 (f3)
    pub pct_change: Option<f64>,
    /// 涨跌额 (f4)
    pub change: Option<f64>,
    /// 成交量 (f5)
    pub volume: Option<f64>,
    /// 成交额 (f6)
    pub amount: Option<f64>,
    /// 换手率 (f8)
    pub turnover_rate: Option<f64>,
    /// 市盈率-动态 (f9)
    pub pe: Option<f64>,
    /// 市净率 (f23)
    pub pb: Option<f64>,
    /// 总市值 (f20)
    pub total_mv: Option<f64>,
    /// 流通市值 (f21)
    pub float_mv: Option<f64>,
}

/// Eastmoney A-share overview — every A-share with its latest quote.
///
/// This is the practical source behind akshare's "Eastmoney stock overview":
/// the `push2` A-share `clist`. We walk pages until `total` is covered.
pub async fn stock_summary(client: &Client) -> Result<Vec<SummaryRow>> {
    let mut out = Vec::new();
    let mut pn: u32 = 1;
    loop {
        let pn_s = pn.to_string();
        let pz_s = SUMMARY_PAGE_SIZE.to_string();
        let params = [
            ("pn", pn_s.as_str()),
            ("pz", pz_s.as_str()),
            ("po", "1"),
            ("np", "1"),
            ("ut", SUMMARY_UT),
            ("fltt", "2"),
            ("invt", "2"),
            ("fid", "f12"),
            ("fs", SUMMARY_FS),
            ("fields", SUMMARY_FIELDS),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, "stock_summary", &crate::core::eastmoney_push::push2_url("/api/qt/clist/get").await, &params)
            .await?;
        let diff = v
            .get("data")
            .and_then(|d| d.get("diff"))
            .and_then(|d| d.as_array());
        let diff = match diff {
            Some(d) => d,
            None => break,
        };
        if diff.is_empty() {
            break;
        }
        out.extend(parse_summary(&v)?);
        let total = v
            .get("data")
            .and_then(|d| d.get("total"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        if (pn as u64) * SUMMARY_PAGE_SIZE as u64 >= total {
            break;
        }
        pn += 1;
    }
    Ok(out)
}

/// Map an Eastmoney `clist` `diff` to [`SummaryRow`]s.
pub(crate) fn parse_summary(resp: &Value) -> Result<Vec<SummaryRow>> {
    let diff = resp
        .get("data")
        .and_then(|d| d.get("diff"))
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff".into(),
        })?;
    let mut out = Vec::new();
    for item in diff_items(diff) {
        out.push(SummaryRow {
            code: str_field(item, "f12"),
            name: str_field(item, "f14"),
            price: num_field(item, "f2"),
            pct_change: num_field(item, "f3"),
            change: num_field(item, "f4"),
            volume: num_field(item, "f5"),
            amount: num_field(item, "f6"),
            turnover_rate: num_field(item, "f8"),
            pe: num_field(item, "f9"),
            pb: num_field(item, "f23"),
            total_mv: num_field(item, "f20"),
            float_mv: num_field(item, "f21"),
        });
    }
    Ok(out)
}

// ===========================================================================
// shared helpers
// ===========================================================================

/// Iterate Eastmoney `diff` as items, tolerating both an array and an object
/// (some Eastmoney endpoints return `diff` as a dict of dicts).
fn diff_items(diff: &Value) -> Vec<&Value> {
    match diff {
        Value::Array(a) => a.iter().collect(),
        Value::Object(m) => m.values().collect(),
        _ => Vec::new(),
    }
}

fn str_field(item: &Value, k: &str) -> String {
    item.get(k)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn num_field(item: &Value, k: &str) -> Option<f64> {
    match item.get(k) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    }
}

/// Parse a CSV cell to `f64`, treating empty / null / nan / `--` as `None`.
fn num_cell(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() || t.eq_ignore_ascii_case("null") || t.eq_ignore_ascii_case("nan") || t == "--"
    {
        None
    } else {
        t.parse::<f64>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn fixture_text(name: &str) -> String {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        std::fs::read_to_string(path).unwrap()
    }

    #[test]
    fn parses_hist_min_kline_fixture() {
        let v = fixture("stock_zh_a_hist_min_em.json");
        let rows = parse_hist_min(&v, false).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].time, "2023-01-03");
        assert_eq!(rows[0].open, Some(11.0));
        assert_eq!(rows[0].close, Some(12.0));
        assert_eq!(rows[0].high, Some(13.0));
        assert_eq!(rows[0].low, Some(10.5));
        assert_eq!(rows[0].volume, Some(10000.0));
        assert_eq!(rows[0].amount, Some(500000.0));
        assert_eq!(rows[0].amplitude, Some(2.5));
        assert_eq!(rows[0].pct_change, Some(1.5));
        assert_eq!(rows[0].change, Some(0.3));
        assert_eq!(rows[0].turnover, Some(0.8));
        assert_eq!(rows[0].avg_price, None);
    }

    #[test]
    fn parses_hist_min_trends_fixture() {
        let v = fixture("stock_zh_a_hist_min_em_trends.json");
        let rows = parse_hist_min(&v, true).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].time, "2023-01-03 09:31:00");
        assert_eq!(rows[0].open, Some(11.0));
        assert_eq!(rows[0].close, Some(12.0));
        assert_eq!(rows[0].avg_price, Some(11.5));
        assert_eq!(rows[0].amplitude, None);
    }

    #[test]
    fn parses_minute_fixture() {
        let text = fixture_text("stock_zh_a_minute.json");
        let rows = parse_minute(&text).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].day, "2024-01-02 09:31:00");
        assert_eq!(rows[0].open, Some(10.10));
        assert_eq!(rows[0].high, Some(10.80));
        assert_eq!(rows[0].low, Some(9.90));
        assert_eq!(rows[0].close, Some(10.50));
        assert_eq!(rows[0].volume, Some(123456.0));
        assert_eq!(rows[0].amount, Some(650000.0));
        assert_eq!(rows[1].day, "2024-01-02 09:32:00");
        assert_eq!(rows[1].close, Some(10.60));
    }

    #[test]
    fn parses_new_shares_fixture() {
        let v = fixture("stock_zh_a_new.json");
        let rows = parse_new_shares(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "sh600519");
        assert_eq!(rows[0].code, "600519");
        assert_eq!(rows[0].name, "贵州茅台");
        assert_eq!(rows[0].open, Some(1000.0));
        assert_eq!(rows[0].high, Some(1050.0));
        assert_eq!(rows[0].low, Some(980.0));
        assert_eq!(rows[0].volume, Some(50000.0));
        assert_eq!(rows[0].amount, Some(50000000.0));
        assert_eq!(rows[0].mktcap, Some(20000000000.0));
        assert_eq!(rows[0].turnover_ratio, Some(2.5));
    }

    #[test]
    fn parses_stop_fixture() {
        let v = fixture("stock_zh_a_stop.json");
        let rows = parse_stop(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "600000");
        assert_eq!(rows[0].name, "浦发银行");
        assert_eq!(rows[0].price, Some(7.23));
        assert_eq!(rows[0].pct_change, Some(-1.50));
        assert_eq!(rows[0].change, Some(-0.11));
        assert_eq!(rows[0].volume, Some(12345678.0));
        assert_eq!(rows[0].amount, Some(89000000.0));
        assert_eq!(rows[0].amplitude, Some(2.10));
        assert_eq!(rows[0].turnover_rate, Some(0.42));
        assert_eq!(rows[0].pe, Some(4.50));
        assert_eq!(rows[0].volume_ratio, Some(1.20));
        assert_eq!(rows[0].high, Some(7.40));
        assert_eq!(rows[0].low, Some(7.20));
        assert_eq!(rows[0].open, Some(7.35));
        assert_eq!(rows[0].pre_close, Some(7.34));
        assert_eq!(rows[0].pb, Some(0.55));
        assert_eq!(rows[1].code, "000001");
    }

    #[test]
    fn parses_summary_fixture() {
        let v = fixture("stock_summary.json");
        let rows = parse_summary(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "600000");
        assert_eq!(rows[0].name, "浦发银行");
        assert_eq!(rows[0].price, Some(7.23));
        assert_eq!(rows[0].pct_change, Some(-1.50));
        assert_eq!(rows[0].volume, Some(12345678.0));
        assert_eq!(rows[0].amount, Some(89000000.0));
        assert_eq!(rows[0].turnover_rate, Some(0.42));
        assert_eq!(rows[0].pe, Some(4.50));
        assert_eq!(rows[0].pb, Some(0.55));
        assert_eq!(rows[0].total_mv, Some(212345678900.0));
        assert_eq!(rows[0].float_mv, Some(198765432100.0));
    }
}
