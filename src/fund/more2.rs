//! Further Eastmoney / Sina / SSE / THS fund endpoints (akshare `fund` package).
//!
//! Ports a coherent, tractable batch of akshare fund functions that are
//! reachable with ordinary HTTP — Eastmoney `push2his` kline/trends, Eastmoney
//! `datacenter`/rank/`f10`/`Data` JSON or light-JSONP (no JS execution), Sina
//! JSONP, SSE `commonQuery.do` JSON, and THS JSONP — all without JS signing,
//! HTML scraping, or Excel/ZIP parsing (no `py_mini_racer`/`pd.read_html`/
//! `read_excel`).
//!
//! Provenance / mapping for this akshare checkout (`/Users/tony/github/akshare`):
//!
//! | Rust fn                        | akshare source                       | upstream shape                                  |
//! |--------------------------------|--------------------------------------|-------------------------------------------------|
//! | `fund_etf_hist_min_em`         | `fund_etf_em.py::fund_etf_hist_min_em` | `push2his` trends2/kline (`Data.trends`/`.klines`) |
//! | `fund_lof_hist_min_em`         | `fund_lof_em.py::fund_lof_hist_min_em` | `push2his` trends2/kline                         |
//! | `fund_money_rank_em`           | `fund_rank_em.py::fund_money_rank_em` | JSON `api.fund.eastmoney.com/FundRank/GetHbRankList` |
//! | `fund_lcx_rank_em`             | `fund_rank_em.py::fund_lcx_rank_em`  | JSON `api.fund.eastmoney.com/FundRank/GetLcRankList` |
//! | `fund_new_found_em`            | `fund_init_em.py::fund_new_found_em` | `var newfunddata={...}` JSON (`datas` arrays)     |
//! | `fund_announcement_dividend_em`| `fund_announcement_em.py::...dividend` | JSON `api.fund.eastmoney.com/f10/JJGG` (type=2) |
//! | `fund_announcement_report_em`  | `fund_announcement_em.py::...report` | JSON `f10/JJGG` (type=3)                          |
//! | `fund_announcement_personnel_em`| `fund_announcement_em.py::...personnel`| JSON `f10/JJGG` (type=4)                         |
//! | `fund_etf_scale_sse`           | `fund_etf_sse.py::fund_etf_scale_sse`| JSON `query.sse.com.cn/commonQuery.do` (`result`) |
//! | `fund_cf_em`                   | `fund_fhsp_em.py::fund_cf_em`        | `[[...]]` array (`funddataIndex_Interface.aspx` dt=9) |
//! | `fund_fh_rank_em`              | `fund_fhsp_em.py::fund_fh_rank_em`   | `[[...]]` array (`funddataIndex_Interface.aspx` dt=10) |
//! | `fund_scale_open_sina`         | `fund_scale_sina.py::fund_scale_open_sina` | Sina JSONP `NetValueReturnOpen` (`data` objects) |
//! | `fund_scale_close_sina`        | `fund_scale_sina.py::fund_scale_close_sina` | Sina JSONP `NetValueReturnClose`               |
//! | `fund_scale_structured_sina`   | `fund_scale_sina.py::fund_scale_structured_sina` | Sina JSONP `NetValueReturnCX`              |
//! | `fund_etf_category_ths`        | `fund_etf_ths.py::fund_etf_category_ths` | THS JSONP `data/Net/info/...` (`data.data` objects) |
//! | `fund_etf_spot_ths`            | `fund_etf_ths.py::fund_etf_spot_ths`  | THS JSONP (symbol=ETF wrapper)                    |
//!
//! DEFERRED (not ported):
//! - `fund_open_fund_rank_em` / `fund_exchange_rank_em` (`fund_rank_em.py`) —
//!   `rankhandler.aspx` returns `var apidata={...}` via `demjson` with a
//!   per-request random `v` token; needs cookie/JS-eval context.
//! - `fund_aum_em` / `fund_aum_hist_em` (`fund_aum_em.py`) — `pd.read_html` HTML.
//! - `fund_rating_*` (`fund_rating.py`) — BeautifulSoup HTML scraping of `<script>`.
//! - `fund_etf_hist_sina` / `fund_etf_dividend_sina` (`fund_etf_sina.py`) —
//!   `py_mini_racer` JS decryption.
//! - `fund_etf_scale_szse` / `fund_scale_daily_szse` (`*_szse.py`) — `xlsx` Excel.
//! - `fund_position_lg` (`fund_position_lg.py`) — needs `legulegu` token + cookie.
//! - `fund_info_ths` (`fund_info_ths.py`) — BeautifulSoup HTML.
//! - `fund_new_found_ths` (`fund_init_ths.py`) — JSON embedded in an HTML page
//!   (bracket-scan extraction); fragile, not a clean endpoint.
//! - `fund_xq` (`fund_xq.py`) — Xueqiu/danjuanfunds requires a session cookie.
//! - `fund_portfolio_*` / `fund_fee_em` / `fund_overview_em` — HTML / `demjson`.
//! - `fund_fh_em` (`fund_fhsp_em.py`) — already ported in `more.rs`.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

// ---------------------------------------------------------------------------
// URLs + fixed upstream params
// ---------------------------------------------------------------------------

const PUSH2HIS_URL: &str = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
const PUSH2HIS_TRENDS_URL: &str = "https://push2his.eastmoney.com/api/qt/stock/trends2/get";
const PUSH2HIS_UT: &str = "7eea3edcaed734bea9cbfc24409ed989";

const HB_RANK_URL: &str = "https://api.fund.eastmoney.com/FundRank/GetHbRankList";
const LC_RANK_URL: &str = "https://api.fund.eastmoney.com/FundRank/GetLcRankList";

const NEW_FOUND_URL: &str = "https://fund.eastmoney.com/data/FundNewIssue.aspx";
const ANNOUNCE_URL: &str = "http://api.fund.eastmoney.com/f10/JJGG";
const SSE_URL: &str = "https://query.sse.com.cn/commonQuery.do";
const FUND_DATA_URL: &str = "https://fund.eastmoney.com/Data/funddataIndex_Interface.aspx";

const SINA_SCALE_URL_PREFIX: &str =
    "http://vip.stock.finance.sina.com.cn/fund_center/data/jsonp.php/IO.XSRV2.CallbackList['J2cW8KXheoWKdSHc']/NetValueReturn_Service";
const THS_URL: &str = "https://fund.10jqka.com.cn/data/Net/info";

// ---------------------------------------------------------------------------
// shared positional + unwrap helpers
// ---------------------------------------------------------------------------

/// Treat an array item or object's ordered values as a positional cell list
/// (akshare renames `pd.DataFrame` columns by position, so positional parsing
/// matches upstream regardless of object vs array payloads).
fn cells_of(item: &Value) -> Vec<&Value> {
    match item {
        Value::Array(a) => a.iter().collect(),
        Value::Object(o) => o.values().collect(),
        _ => Vec::new(),
    }
}

/// Positional string cell (empty string when missing).
fn cstr(cells: &[&Value], i: usize) -> String {
    cells
        .get(i)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

/// Positional numeric cell (Number, or numeric/String with a trailing `%`).
fn cf64(cells: &[&Value], i: usize) -> Option<f64> {
    cells.get(i).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().trim_end_matches('%').trim().parse::<f64>().ok(),
        _ => None,
    })
}

/// Extract the substring between the first `{` and the last `}` (used for
/// `var name={...}` JSON, Sina/THS JSONP, and SSE-style payloads).
fn extract_braces(text: &str) -> Result<&str> {
    let s = text.find('{').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "expected '{' in payload".into(),
    })?;
    let e = text.rfind('}').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "expected '}' in payload".into(),
    })?;
    Ok(&text[s..=e])
}

/// Extract the `[[...]]` array that ends just before `end_marker` (used for
/// `funddataIndex_Interface.aspx` CSV-array payloads). No JS evaluation.
fn extract_js_array(text: &str, end_marker: &str) -> Result<Value> {
    let start = text.find("[[").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "expected '[[' in payload".into(),
    })?;
    let end = text.find(end_marker).ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: format!("expected end marker '{end_marker}'"),
    })?;
    serde_json::from_str(&text[start..end]).map_err(Error::Json)
}

/// Eastmoney `push2` secid market prefix: Shanghai (1) for 5*/6*, else Shenzhen (0).
fn market_id(symbol: &str) -> &'static str {
    if symbol.starts_with('5') || symbol.starts_with('6') {
        "1"
    } else {
        "0"
    }
}

// ===========================================================================
// fund_etf_hist_min_em / fund_lof_hist_min_em — push2his intraday kline
// ===========================================================================

/// ETF intraday (minute) quote row (akshare `fund_etf_hist_min_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct EtfMinRow {
    pub symbol: String,
    pub time: String,
    pub open: Option<f64>,
    pub close: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub volume: Option<f64>,
    pub amount: Option<f64>,
    pub avg: Option<f64>,
    pub source: &'static str,
}

/// LOF intraday (minute) quote row (akshare `fund_lof_hist_min_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct LofMinRow {
    pub symbol: String,
    pub time: String,
    pub open: Option<f64>,
    pub close: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub volume: Option<f64>,
    pub amount: Option<f64>,
    pub avg: Option<f64>,
    pub source: &'static str,
}

/// ETF intraday history (`push2his` trends2 when `period=="1"`, otherwise
/// kline with `klt=period`). akshare `fund_etf_em.py::fund_etf_hist_min_em`.
pub async fn fund_etf_hist_min_em(
    client: &Client,
    symbol: &str,
    start_date: &str,
    end_date: &str,
    period: &str,
    _adjust: &str,
) -> Result<Vec<EtfMinRow>> {
    let secid = format!("{}.{symbol}", market_id(symbol));
    if period == "1" {
        let params = [
            ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f11,f12,f13"),
            ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58"),
            ("ut", PUSH2HIS_UT),
            ("ndays", "5"),
            ("iscr", "0"),
            ("secid", &secid),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, "fund_etf_hist_min_em", PUSH2HIS_TRENDS_URL, &params)
            .await?;
        let arr = v
            .get("data")
            .and_then(|d| d.get("trends"))
            .and_then(|t| t.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing data.trends".into(),
            })?;
        Ok(parse_etf_min_trends(arr, symbol))
    } else {
        let params = [
            ("fields1", "f1,f2,f3,f4,f5,f6"),
            ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61"),
            ("ut", PUSH2HIS_UT),
            ("klt", period),
            ("fqt", "0"),
            ("secid", &secid),
            ("beg", start_date),
            ("end", end_date),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, "fund_etf_hist_min_em", PUSH2HIS_URL, &params)
            .await?;
        let arr = v
            .get("data")
            .and_then(|d| d.get("klines"))
            .and_then(|k| k.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing data.klines".into(),
            })?;
        Ok(parse_etf_min_kline(arr, symbol))
    }
}

/// LOF intraday history (`push2his` trends2/kline). akshare
/// `fund_lof_em.py::fund_lof_hist_min_em`.
pub async fn fund_lof_hist_min_em(
    client: &Client,
    symbol: &str,
    start_date: &str,
    end_date: &str,
    period: &str,
    _adjust: &str,
) -> Result<Vec<LofMinRow>> {
    let secid = format!("{}.{symbol}", market_id(symbol));
    if period == "1" {
        let params = [
            ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f11,f12,f13"),
            ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58"),
            ("ut", PUSH2HIS_UT),
            ("ndays", "5"),
            ("iscr", "0"),
            ("secid", &secid),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, "fund_lof_hist_min_em", PUSH2HIS_TRENDS_URL, &params)
            .await?;
        let arr = v
            .get("data")
            .and_then(|d| d.get("trends"))
            .and_then(|t| t.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing data.trends".into(),
            })?;
        Ok(parse_lof_min_trends(arr, symbol))
    } else {
        let params = [
            ("fields1", "f1,f2,f3,f4,f5,f6"),
            ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61"),
            ("ut", PUSH2HIS_UT),
            ("klt", period),
            ("fqt", "0"),
            ("secid", &secid),
            ("beg", start_date),
            ("end", end_date),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, "fund_lof_hist_min_em", PUSH2HIS_URL, &params)
            .await?;
        let arr = v
            .get("data")
            .and_then(|d| d.get("klines"))
            .and_then(|k| k.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing data.klines".into(),
            })?;
        Ok(parse_lof_min_kline(arr, symbol))
    }
}

fn parse_etf_min_trends(lines: &[Value], symbol: &str) -> Vec<EtfMinRow> {
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        let s = match line.as_str() {
            Some(s) => s,
            None => continue,
        };
        let p: Vec<&str> = s.split(',').collect();
        if p.len() < 8 {
            continue;
        }
        out.push(EtfMinRow {
            symbol: symbol.to_string(),
            time: p[0].to_string(),
            open: p[1].parse().ok(),
            close: p[2].parse().ok(),
            high: p[3].parse().ok(),
            low: p[4].parse().ok(),
            volume: p[5].parse().ok(),
            amount: p[6].parse().ok(),
            avg: p[7].parse().ok(),
            source: SOURCE_EASTMONEY,
        });
    }
    out
}

fn parse_etf_min_kline(lines: &[Value], symbol: &str) -> Vec<EtfMinRow> {
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        let s = match line.as_str() {
            Some(s) => s,
            None => continue,
        };
        let p: Vec<&str> = s.split(',').collect();
        if p.len() < 7 {
            continue;
        }
        out.push(EtfMinRow {
            symbol: symbol.to_string(),
            time: p[0].to_string(),
            open: p[1].parse().ok(),
            close: p[2].parse().ok(),
            high: p[3].parse().ok(),
            low: p[4].parse().ok(),
            volume: p[5].parse().ok(),
            amount: p[6].parse().ok(),
            avg: None,
            source: SOURCE_EASTMONEY,
        });
    }
    out
}

fn parse_lof_min_trends(lines: &[Value], symbol: &str) -> Vec<LofMinRow> {
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        let s = match line.as_str() {
            Some(s) => s,
            None => continue,
        };
        let p: Vec<&str> = s.split(',').collect();
        if p.len() < 8 {
            continue;
        }
        out.push(LofMinRow {
            symbol: symbol.to_string(),
            time: p[0].to_string(),
            open: p[1].parse().ok(),
            close: p[2].parse().ok(),
            high: p[3].parse().ok(),
            low: p[4].parse().ok(),
            volume: p[5].parse().ok(),
            amount: p[6].parse().ok(),
            avg: p[7].parse().ok(),
            source: SOURCE_EASTMONEY,
        });
    }
    out
}

fn parse_lof_min_kline(lines: &[Value], symbol: &str) -> Vec<LofMinRow> {
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        let s = match line.as_str() {
            Some(s) => s,
            None => continue,
        };
        let p: Vec<&str> = s.split(',').collect();
        if p.len() < 7 {
            continue;
        }
        out.push(LofMinRow {
            symbol: symbol.to_string(),
            time: p[0].to_string(),
            open: p[1].parse().ok(),
            close: p[2].parse().ok(),
            high: p[3].parse().ok(),
            low: p[4].parse().ok(),
            volume: p[5].parse().ok(),
            amount: p[6].parse().ok(),
            avg: None,
            source: SOURCE_EASTMONEY,
        });
    }
    out
}

// ===========================================================================
// fund_money_rank_em / fund_lcx_rank_em — Eastmoney FundRank JSON
// ===========================================================================

/// Money-market fund ranking row (akshare `fund_money_rank_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundMoneyRankRow {
    pub seq: Option<f64>,
    pub code: String,
    pub name: String,
    pub date: String,
    pub million_income: Option<f64>,
    pub annual_7d: Option<f64>,
    pub annual_14d: Option<f64>,
    pub annual_28d: Option<f64>,
    pub m1: Option<f64>,
    pub m3: Option<f64>,
    pub m6: Option<f64>,
    pub y1: Option<f64>,
    pub y2: Option<f64>,
    pub y3: Option<f64>,
    pub y5: Option<f64>,
    pub ytd: Option<f64>,
    pub total: Option<f64>,
    pub fee: Option<f64>,
    pub source: &'static str,
}

/// Wealth-management (理财) fund ranking row (akshare `fund_lcx_rank_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundLcxRankRow {
    pub seq: Option<f64>,
    pub code: String,
    pub name: String,
    pub date: String,
    pub million_income: Option<f64>,
    pub annual_7d: Option<f64>,
    pub annual_14d: Option<f64>,
    pub annual_28d: Option<f64>,
    pub w1: Option<f64>,
    pub m1: Option<f64>,
    pub m3: Option<f64>,
    pub m6: Option<f64>,
    pub ytd: Option<f64>,
    pub total: Option<f64>,
    pub buyable: String,
    pub fee: Option<f64>,
    pub source: &'static str,
}

/// Money-market fund ranking (akshare `fund_rank_em.py::fund_money_rank_em`).
pub async fn fund_money_rank_em(client: &Client) -> Result<Vec<FundMoneyRankRow>> {
    let params = [
        ("intCompany", "0"),
        ("MinsgType", ""),
        ("IsSale", "1"),
        ("strSortCol", "SYL_1N"),
        ("orderType", "desc"),
        ("pageIndex", "1"),
        ("pageSize", "10000"),
    ];
    let headers = [("Referer", "https://fund.eastmoney.com/fundguzhi.html")];
    let v = client
        .get_json_with_headers(SOURCE_EASTMONEY, "fund_money_rank_em", HB_RANK_URL, &params, Some(&headers))
        .await?;
    let arr = v
        .get("Data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing Data".into(),
        })?;
    Ok(parse_money_rank(arr))
}

/// Wealth-management fund ranking (akshare `fund_rank_em.py::fund_lcx_rank_em`).
pub async fn fund_lcx_rank_em(client: &Client) -> Result<Vec<FundLcxRankRow>> {
    let params = [
        ("intCompany", "0"),
        ("MinsgType", "undefined"),
        ("IsSale", "1"),
        ("strSortCol", "SYL_Z"),
        ("orderType", "desc"),
        ("pageIndex", "1"),
        ("pageSize", "50"),
        ("FBQ", ""),
        ("callback", "jQuery18303264654966943197_1603867158043"),
    ];
    let headers = [("Referer", "https://fund.eastmoney.com/fundguzhi.html")];
    let v = client
        .get_json_with_headers(SOURCE_EASTMONEY, "fund_lcx_rank_em", LC_RANK_URL, &params, Some(&headers))
        .await?;
    let arr = v
        .get("Data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing Data".into(),
        })?;
    Ok(parse_lcx_rank(arr))
}

fn parse_money_rank(items: &[Value]) -> Vec<FundMoneyRankRow> {
    items
        .iter()
        .map(|item| {
            let c = cells_of(item);
            FundMoneyRankRow {
                seq: cf64(&c, 0),
                code: cstr(&c, 7),
                name: cstr(&c, 8),
                date: cstr(&c, 9),
                million_income: cf64(&c, 10),
                annual_7d: cf64(&c, 11),
                annual_14d: cf64(&c, 13),
                annual_28d: cf64(&c, 14),
                m1: cf64(&c, 15),
                m3: cf64(&c, 16),
                m6: cf64(&c, 17),
                y1: cf64(&c, 18),
                y2: cf64(&c, 19),
                y3: cf64(&c, 20),
                y5: cf64(&c, 21),
                ytd: cf64(&c, 22),
                total: cf64(&c, 23),
                fee: cf64(&c, 25),
                source: SOURCE_EASTMONEY,
            }
        })
        .collect()
}

fn parse_lcx_rank(items: &[Value]) -> Vec<FundLcxRankRow> {
    items
        .iter()
        .map(|item| {
            let c = cells_of(item);
            FundLcxRankRow {
                seq: cf64(&c, 0),
                code: cstr(&c, 2),
                name: cstr(&c, 3),
                date: cstr(&c, 4),
                million_income: cf64(&c, 5),
                annual_7d: cf64(&c, 6),
                annual_14d: cf64(&c, 8),
                annual_28d: cf64(&c, 9),
                w1: cf64(&c, 1),
                m1: cf64(&c, 10),
                m3: cf64(&c, 11),
                m6: cf64(&c, 12),
                ytd: cf64(&c, 13),
                total: cf64(&c, 14),
                buyable: cstr(&c, 15),
                fee: cf64(&c, 16),
                source: SOURCE_EASTMONEY,
            }
        })
        .collect()
}

// ===========================================================================
// fund_new_found_em — FundNewIssue.aspx `var newfunddata={...}`
// ===========================================================================

/// Newly-established fund row (akshare `fund_init_em.py::fund_new_found_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundNewFoundRow {
    pub code: String,
    pub name: String,
    pub company: String,
    pub fund_type: String,
    pub subscribe_period: Option<f64>,
    pub establish_date: String,
    pub establish_gain: Option<f64>,
    pub manager: String,
    pub purchase_status: String,
    pub fee: Option<f64>,
    pub source: &'static str,
}

/// Newly-established funds (akshare `fund_init_em.py::fund_new_found_em`).
pub async fn fund_new_found_em(client: &Client) -> Result<Vec<FundNewFoundRow>> {
    let params = [
        ("t", "xcln"),
        ("sort", "jzrgq,desc"),
        ("y", ""),
        ("page", "1,50000"),
        ("isbuy", "1"),
        ("v", "0.4069919776543214"),
    ];
    let text = client
        .get_text(SOURCE_EASTMONEY, "fund_new_found_em", NEW_FOUND_URL, &params, None)
        .await?;
    let json = extract_braces(&text)?;
    let v: Value = serde_json::from_str(json).map_err(Error::Json)?;
    let arr = v
        .get("datas")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing datas".into(),
        })?;
    Ok(parse_new_found(arr))
}

fn parse_new_found(items: &[Value]) -> Vec<FundNewFoundRow> {
    items
        .iter()
        .map(|item| {
            let c = cells_of(item);
            FundNewFoundRow {
                code: cstr(&c, 0),
                name: cstr(&c, 1),
                company: cstr(&c, 2),
                fund_type: cstr(&c, 4),
                subscribe_period: cf64(&c, 5),
                establish_date: cstr(&c, 6),
                establish_gain: cf64(&c, 7),
                manager: cstr(&c, 8),
                purchase_status: cstr(&c, 9),
                fee: cf64(&c, 10),
                source: SOURCE_EASTMONEY,
            }
        })
        .collect()
}

// ===========================================================================
// fund_announcement_*_em — api.fund.eastmoney.com/f10/JJGG
// ===========================================================================

/// Fund announcement row (akshare `fund_announcement_em.py`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundAnnouncementRow {
    pub code: String,
    pub title: String,
    pub name: String,
    pub date: String,
    pub report_id: String,
    pub category: String,
    pub source: &'static str,
}

/// Fund dividend announcements (akshare `fund_announcement_em.py::fund_announcement_dividend_em`).
pub async fn fund_announcement_dividend_em(client: &Client, symbol: &str) -> Result<Vec<FundAnnouncementRow>> {
    announce(client, symbol, "2", "分红配送").await
}

/// Fund periodic-report announcements (akshare `fund_announcement_em.py::fund_announcement_report_em`).
pub async fn fund_announcement_report_em(client: &Client, symbol: &str) -> Result<Vec<FundAnnouncementRow>> {
    announce(client, symbol, "3", "定期报告").await
}

/// Fund personnel-change announcements (akshare `fund_announcement_em.py::fund_announcement_personnel_em`).
pub async fn fund_announcement_personnel_em(client: &Client, symbol: &str) -> Result<Vec<FundAnnouncementRow>> {
    announce(client, symbol, "4", "人事调整").await
}

async fn announce(
    client: &Client,
    symbol: &str,
    r#type: &str,
    category: &str,
) -> Result<Vec<FundAnnouncementRow>> {
    let referer = format!("http://fundf10.eastmoney.com/jjgg_{symbol}_{type}.html");
    let headers = [("Referer", referer.as_str())];
    let params = [
        ("fundcode", symbol),
        ("pageIndex", "1"),
        ("pageSize", "1000"),
        ("type", r#type),
        ("_", "1"),
    ];
    let v = client
        .get_json_with_headers(SOURCE_EASTMONEY, "fund_announcement_em", ANNOUNCE_URL, &params, Some(&headers))
        .await?;
    let arr = v
        .get("Data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing Data".into(),
        })?;
    Ok(parse_announcement(arr, category))
}

fn parse_announcement(items: &[Value], category: &str) -> Vec<FundAnnouncementRow> {
    let cat = category.to_string();
    items
        .iter()
        .map(|item| {
            let c = cells_of(item);
            FundAnnouncementRow {
                code: cstr(&c, 0),
                title: cstr(&c, 1),
                name: cstr(&c, 2),
                date: cstr(&c, 5),
                report_id: cstr(&c, 7),
                category: cat.clone(),
                source: SOURCE_EASTMONEY,
            }
        })
        .collect()
}

// ===========================================================================
// fund_etf_scale_sse — query.sse.com.cn/commonQuery.do
// ===========================================================================

/// SSE ETF share/scale row (akshare `fund_etf_sse.py::fund_etf_scale_sse`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundEtfScaleSseRow {
    pub seq: Option<f64>,
    pub code: String,
    pub name: String,
    pub etf_type: String,
    pub stat_date: String,
    /// Total shares in 份 (TOT_VOL × 10000, mirroring akshare).
    pub shares: Option<f64>,
    pub source: &'static str,
}

/// SSE ETF share data (akshare `fund_etf_sse.py::fund_etf_scale_sse`).
pub async fn fund_etf_scale_sse(client: &Client, date: &str) -> Result<Vec<FundEtfScaleSseRow>> {
    let data_str = format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8]);
    let params = [
        ("isPagination", "true"),
        ("pageHelp.pageSize", "10000"),
        ("pageHelp.pageNo", "1"),
        ("pageHelp.beginPage", "1"),
        ("pageHelp.cacheSize", "1"),
        ("pageHelp.endPage", "1"),
        ("sqlId", "COMMON_SSE_ZQPZ_ETFZL_XXPL_ETFGM_SEARCH_L"),
        ("STAT_DATE", &data_str),
    ];
    let headers = [("Referer", "https://www.sse.com.cn/")];
    let v = client
        .get_json_with_headers("sse", "fund_etf_scale_sse", SSE_URL, &params, Some(&headers))
        .await?;
    let arr = v
        .get("result")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: "sse",
            message: "missing result".into(),
        })?;
    Ok(parse_etf_scale_sse(arr))
}

fn parse_etf_scale_sse(items: &[Value]) -> Vec<FundEtfScaleSseRow> {
    items
        .iter()
        .map(|item| FundEtfScaleSseRow {
            seq: item.get("NUM").and_then(|v| v.as_f64()),
            code: item.get("SEC_CODE").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            name: item.get("SEC_NAME").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            etf_type: item.get("ETF_TYPE").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            stat_date: item.get("STAT_DATE").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            shares: item.get("TOT_VOL").and_then(|v| v.as_f64()).map(|x| x * 10000.0),
            source: "sse",
        })
        .collect()
}

// ===========================================================================
// fund_cf_em / fund_fh_rank_em — funddataIndex_Interface.aspx CSV arrays
// ===========================================================================

/// Fund split (拆分) row (akshare `fund_fhsp_em.py::fund_cf_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundCfRow {
    pub seq: Option<f64>,
    pub code: String,
    pub name: String,
    pub split_date: String,
    pub split_type: String,
    pub split_ratio: Option<f64>,
    pub source: &'static str,
}

/// Fund cumulative-dividend ranking row (akshare `fund_fhsp_em.py::fund_fh_rank_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundFhRankRow {
    pub seq: Option<f64>,
    pub code: String,
    pub name: String,
    pub total_dividend: Option<f64>,
    pub total_count: Option<f64>,
    pub establish_date: String,
    pub source: &'static str,
}

/// Fund splits (akshare `fund_fhsp_em.py::fund_cf_em`, dt=9).
pub async fn fund_cf_em(client: &Client, year: &str, typ: &str, rank: &str, sort: &str) -> Result<Vec<FundCfRow>> {
    let text = fund_data_text(client, "9", year, typ, rank, sort).await?;
    let v = extract_js_array(&text, ";var jjcf_jjgs")?;
    let outer = v
        .as_array()
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "fund_cf_em: expected array".into(),
        })?;
    // Eastmoney wraps the row list one level deeper: `[[[row],[row]]]`.
    let arr: &[Value] = match outer.as_slice() {
        [Value::Array(inner)] => &inner[..],
        other => other,
    };
    Ok(parse_cf(arr))
}

/// Fund cumulative-dividend ranking (akshare `fund_fhsp_em.py::fund_fh_rank_em`, dt=10).
pub async fn fund_fh_rank_em(client: &Client) -> Result<Vec<FundFhRankRow>> {
    let text = fund_data_text(client, "10", "2025", "", "FHFCZ", "desc").await?;
    let v = extract_js_array(&text, ";var fhph_jjgs")?;
    let outer = v
        .as_array()
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "fund_fh_rank_em: expected array".into(),
        })?;
    // Eastmoney wraps the row list one level deeper: `[[[row],[row]]]`.
    let arr: &[Value] = match outer.as_slice() {
        [Value::Array(inner)] => &inner[..],
        other => other,
    };
    Ok(parse_fh_rank(arr))
}

async fn fund_data_text(
    client: &Client,
    dt: &str,
    year: &str,
    typ: &str,
    rank: &str,
    sort: &str,
) -> Result<String> {
    let params = [
        ("dt", dt),
        ("page", "1"),
        ("rank", rank),
        ("sort", sort),
        ("gs", ""),
        ("ftype", typ),
        ("year", year),
    ];
    client
        .get_text(SOURCE_EASTMONEY, "fund_fhsp_em", FUND_DATA_URL, &params, None)
        .await
}

fn parse_cf(items: &[Value]) -> Vec<FundCfRow> {
    items
        .iter()
        .map(|item| {
            let c = cells_of(item);
            FundCfRow {
                seq: cf64(&c, 0),
                code: cstr(&c, 1),
                name: cstr(&c, 2),
                split_date: cstr(&c, 3),
                split_type: cstr(&c, 4),
                split_ratio: cf64(&c, 5),
                source: SOURCE_EASTMONEY,
            }
        })
        .collect()
}

fn parse_fh_rank(items: &[Value]) -> Vec<FundFhRankRow> {
    items
        .iter()
        .map(|item| {
            let c = cells_of(item);
            FundFhRankRow {
                seq: cf64(&c, 0),
                code: cstr(&c, 1),
                name: cstr(&c, 2),
                total_dividend: cf64(&c, 3),
                total_count: cf64(&c, 4),
                establish_date: cstr(&c, 5),
                source: SOURCE_EASTMONEY,
            }
        })
        .collect()
}

// ===========================================================================
// fund_scale_*_sina — Sina NetValueReturn JSONP
// ===========================================================================

/// Sina fund-scale row (akshare `fund_scale_sina.py`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundScaleSinaRow {
    pub seq: u32,
    pub code: String,
    pub name: String,
    pub nav: Option<f64>,
    pub total_scale: Option<f64>,
    pub latest_shares: Option<f64>,
    pub establish_date: String,
    pub manager: String,
    pub update_date: String,
    pub category: String,
    pub source: &'static str,
}

/// Open-end fund scale (akshare `fund_scale_sina.py::fund_scale_open_sina`).
pub async fn fund_scale_open_sina(client: &Client, symbol: &str) -> Result<Vec<FundScaleSinaRow>> {
    let type2 = match symbol {
        "股票型基金" => "2",
        "混合型基金" => "1",
        "债券型基金" => "3",
        "货币型基金" => "5",
        "QDII基金" => "6",
        _ => return Err(Error::InvalidParam(format!("unknown symbol: {symbol}"))),
    };
    let url = format!("{SINA_SCALE_URL_PREFIX}.NetValueReturnOpen");
    let params = [
        ("page", "1"),
        ("num", "10000"),
        ("sort", "zmjgm"),
        ("asc", "0"),
        ("ccode", ""),
        ("type2", type2),
        ("type3", ""),
    ];
    let headers = [("Referer", "https://vip.stock.finance.sina.com.cn/fund_center/index.html#jjhqetf")];
    let text = client
        .get_text("sina", "fund_scale_open_sina", &url, &params, Some(&headers))
        .await?;
    let arr = sina_data_array(&text, "开放式基金")?;
    Ok(parse_scale_sina(&arr, symbol))
}

/// Closed-end fund scale (akshare `fund_scale_sina.py::fund_scale_close_sina`).
pub async fn fund_scale_close_sina(client: &Client) -> Result<Vec<FundScaleSinaRow>> {
    let url = format!("{SINA_SCALE_URL_PREFIX}.NetValueReturnClose");
    let params = [
        ("page", "1"),
        ("num", "1000"),
        ("sort", "zmjgm"),
        ("asc", "0"),
        ("ccode", ""),
        ("type2", ""),
        ("type3", ""),
    ];
    let headers = [("Referer", "https://vip.stock.finance.sina.com.cn/fund_center/index.html#jjhqetf")];
    let text = client
        .get_text("sina", "fund_scale_close_sina", &url, &params, Some(&headers))
        .await?;
    let arr = sina_data_array(&text, "封闭式基金")?;
    Ok(parse_scale_sina(&arr, "封闭式基金"))
}

/// Structured (分级子) fund scale (akshare `fund_scale_sina.py::fund_scale_structured_sina`).
pub async fn fund_scale_structured_sina(client: &Client) -> Result<Vec<FundScaleSinaRow>> {
    let url = format!("{SINA_SCALE_URL_PREFIX}.NetValueReturnCX");
    let params = [
        ("page", "1"),
        ("num", "1000"),
        ("sort", "zmjgm"),
        ("asc", "0"),
        ("ccode", ""),
        ("type2", ""),
        ("type3", ""),
    ];
    let headers = [("Referer", "https://vip.stock.finance.sina.com.cn/fund_center/index.html#jjhqetf")];
    let text = client
        .get_text("sina", "fund_scale_structured_sina", &url, &params, Some(&headers))
        .await?;
    let arr = sina_data_array(&text, "分级子基金")?;
    Ok(parse_scale_sina(&arr, "分级子基金"))
}

fn sina_data_array(text: &str, category: &str) -> Result<Vec<Value>> {
    let json = extract_braces(text)?;
    let v: Value = serde_json::from_str(json).map_err(Error::Json)?;
    v.get("data")
        .and_then(|d| d.as_array())
        .map(|a| a.to_vec())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: "sina",
            message: format!("fund_scale_{category}_sina: missing data"),
        })
}

fn parse_scale_sina(items: &[Value], category: &str) -> Vec<FundScaleSinaRow> {
    let cat = category.to_string();
    items
        .iter()
        .enumerate()
        .map(|(i, item)| FundScaleSinaRow {
            seq: (i + 1) as u32,
            code: item.get("symbol").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            name: item.get("sname").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            nav: item.get("dwjz").and_then(|v| v.as_f64()),
            total_scale: item.get("zmjgm").and_then(|v| v.as_f64()),
            latest_shares: item.get("zjzfe").and_then(|v| v.as_f64()),
            establish_date: item.get("clrq").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            manager: item.get("jjjl").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            update_date: item.get("jzrq").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            category: cat.clone(),
            source: "sina",
        })
        .collect()
}

// ===========================================================================
// fund_etf_*_ths — THS Net/info JSONP
// ===========================================================================

/// THS ETF daily-NAV row (akshare `fund_etf_ths.py`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundEtfThsRow {
    pub seq: u32,
    pub code: String,
    pub type_name: String,
    pub net: Option<f64>,
    pub name: String,
    pub total_net: Option<f64>,
    pub new_net: Option<f64>,
    pub new_total_net: Option<f64>,
    pub new_date: String,
    pub net1: Option<f64>,
    pub total_net1: Option<f64>,
    pub range: Option<f64>,
    pub rate: Option<f64>,
    pub shstat: String,
    pub sgstat: String,
    pub source: &'static str,
}

/// THS fund NAV by category (akshare `fund_etf_ths.py::fund_etf_category_ths`).
pub async fn fund_etf_category_ths(client: &Client, symbol: &str, date: &str) -> Result<Vec<FundEtfThsRow>> {
    let inner = match symbol {
        "股票型" => "gpx",
        "债券型" => "zqx",
        "混合型" => "hhx",
        "ETF" => "ETF",
        "LOF" => "LOF",
        "QDII" => "QDII",
        "保本型" => "bbx",
        "指数型" => "zsx",
        "" => "all",
        _ => "ETF",
    };
    let inner_date = if date.is_empty() {
        "0".to_string()
    } else {
        format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8])
    };
    let url = format!("{THS_URL}/{inner}_rate_desc_{inner_date}_0_1_9999_0_0_0_jsonp_g.html");
    let text = client
        .get_text("ths", "fund_etf_category_ths", &url, &[], None)
        .await?;
    let json = extract_braces(&text)?;
    let v: Value = serde_json::from_str(json).map_err(Error::Json)?;
    let data = v
        .get("data")
        .and_then(|d| d.get("data"))
        .and_then(|d| d.as_object())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: "ths",
            message: "missing data.data".into(),
        })?;
    let items: Vec<Value> = data.values().cloned().collect();
    Ok(parse_etf_ths(&items))
}

/// THS ETF spot (akshare `fund_etf_ths.py::fund_etf_spot_ths`, symbol=ETF).
pub async fn fund_etf_spot_ths(client: &Client, date: &str) -> Result<Vec<FundEtfThsRow>> {
    fund_etf_category_ths(client, "ETF", date).await
}

fn parse_etf_ths(items: &[Value]) -> Vec<FundEtfThsRow> {
    items
        .iter()
        .enumerate()
        .map(|(i, item)| FundEtfThsRow {
            seq: (i + 1) as u32,
            code: item.get("code").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            type_name: item.get("typename").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            net: item.get("net").and_then(|v| v.as_f64()),
            name: item.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            total_net: item.get("totalnet").and_then(|v| v.as_f64()),
            new_net: item.get("newnet").and_then(|v| v.as_f64()),
            new_total_net: item.get("newtotalnet").and_then(|v| v.as_f64()),
            new_date: item.get("newdate").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            net1: item.get("net1").and_then(|v| v.as_f64()),
            total_net1: item.get("totalnet1").and_then(|v| v.as_f64()),
            range: item.get("ranges").and_then(|v| v.as_f64()),
            rate: item.get("rate").and_then(|v| v.as_f64()),
            shstat: item.get("shstat").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            sgstat: item.get("sgstat").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            source: "ths",
        })
        .collect()
}

// ===========================================================================
// Offline golden tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let p = p.join("tests/fixtures").join(name);
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    /// Float approx that never panics (mirrors required `approx` helper shape).
    fn approx(a: Option<f64>, b: f64) -> bool {
        (a.unwrap_or(f64::NAN) - b).abs() < 1e-6
    }

    #[test]
    fn parses_fund_etf_hist_min_em_trends() {
        let v = fixture("fund_etf_hist_min_em_trends.json");
        let arr = v.get("data").unwrap().get("trends").unwrap().as_array().unwrap();
        let rows = parse_etf_min_trends(arr, "510050");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "510050");
        assert_eq!(rows[0].time, "2025-04-10 09:32:00");
        assert!(approx(rows[0].open, 2.85));
        assert!(approx(rows[0].avg, 2.855));
        assert_eq!(rows[1].close, Some(2.88));
        assert_eq!(rows[0].source, "eastmoney");
    }

    #[test]
    fn parses_fund_etf_hist_min_em_kline() {
        let v = fixture("fund_etf_hist_min_em_kline.json");
        let arr = v.get("data").unwrap().get("klines").unwrap().as_array().unwrap();
        let rows = parse_etf_min_kline(arr, "510050");
        assert_eq!(rows.len(), 2);
        assert!(approx(rows[0].open, 2.85));
        assert!(approx(rows[0].amount, 2000.0));
        assert_eq!(rows[0].avg, None);
    }

    #[test]
    fn parses_fund_lof_hist_min_em_trends() {
        let v = fixture("fund_lof_hist_min_em_trends.json");
        let arr = v.get("data").unwrap().get("trends").unwrap().as_array().unwrap();
        let rows = parse_lof_min_trends(arr, "166009");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].symbol, "166009");
        assert!(approx(rows[1].high, 2.89));
    }

    #[test]
    fn parses_fund_lof_hist_min_em_kline() {
        let v = fixture("fund_lof_hist_min_em_kline.json");
        let arr = v.get("data").unwrap().get("klines").unwrap().as_array().unwrap();
        let rows = parse_lof_min_kline(arr, "166009");
        assert_eq!(rows.len(), 1);
        assert!(approx(rows[0].volume, 1100.0));
    }

    #[test]
    fn parses_fund_money_rank_em() {
        let v = fixture("fund_money_rank_em.json");
        let arr = v.get("Data").unwrap().as_array().unwrap();
        let rows = parse_money_rank(arr);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].code, "000001");
        assert_eq!(rows[0].name, "华夏货币");
        assert!(approx(rows[0].million_income, 1.5));
        assert!(approx(rows[0].annual_7d, 2.5));
        assert!(approx(rows[0].y1, 1.1));
        assert!(approx(rows[0].total, 10.0));
    }

    #[test]
    fn parses_fund_lcx_rank_em() {
        let v = fixture("fund_lcx_rank_em.json");
        let arr = v.get("Data").unwrap().as_array().unwrap();
        let rows = parse_lcx_rank(arr);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].code, "000001");
        assert!(approx(rows[0].million_income, 1.2));
        assert!(approx(rows[0].w1, 0.1));
        assert!(approx(rows[0].ytd, 5.5));
        assert_eq!(rows[0].buyable, "可购买");
    }

    #[test]
    fn parses_fund_new_found_em() {
        let v = fixture("fund_new_found_em.json");
        let arr = v.get("datas").unwrap().as_array().unwrap();
        let rows = parse_new_found(arr);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "000001");
        assert_eq!(rows[0].company, "华夏基金");
        assert!(approx(rows[0].subscribe_period, 100.5));
        assert_eq!(rows[0].establish_date, "2024-01-01");
        assert!(approx(rows[0].fee, 0.15));
    }

    #[test]
    fn parses_fund_announcement_dividend_em() {
        let v = fixture("fund_announcement_em.json");
        let arr = v.get("Data").unwrap().as_array().unwrap();
        let rows = parse_announcement(arr, "分红配送");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "000001");
        assert_eq!(rows[0].title, "关于分红的公告");
        assert_eq!(rows[0].date, "2025-01-10");
        assert_eq!(rows[0].report_id, "RPT001");
        assert_eq!(rows[0].category, "分红配送");
    }

    #[test]
    fn parses_fund_announcement_report_em() {
        let v = fixture("fund_announcement_em.json");
        let arr = v.get("Data").unwrap().as_array().unwrap();
        let rows = parse_announcement(arr, "定期报告");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].report_id, "RPT002");
        assert_eq!(rows[1].category, "定期报告");
    }

    #[test]
    fn parses_fund_announcement_personnel_em() {
        let v = fixture("fund_announcement_em.json");
        let arr = v.get("Data").unwrap().as_array().unwrap();
        let rows = parse_announcement(arr, "人事调整");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "华夏成长");
        assert_eq!(rows[0].category, "人事调整");
    }

    #[test]
    fn parses_fund_etf_scale_sse() {
        let v = fixture("fund_etf_scale_sse.json");
        let arr = v.get("result").unwrap().as_array().unwrap();
        let rows = parse_etf_scale_sse(arr);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "510050");
        assert_eq!(rows[0].etf_type, "股票ETF");
        assert!(approx(rows[0].shares, 10000.0));
        assert_eq!(rows[1].stat_date, "2025-01-15");
    }

    #[test]
    fn parses_fund_cf_em() {
        let v = extract_js_array(
            &std::fs::read_to_string(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures")
                    .join("fund_cf_em.json"),
            )
            .unwrap(),
            ";var jjcf_jjgs",
        )
        .unwrap();
        let arr = v.as_array().unwrap();
        // Eastmoney wraps the row list one level deeper: `[[[row],[row]]]`.
        let rows_arr: &[Value] = match arr.as_slice() {
            [Value::Array(inner)] => &inner[..],
            other => other,
        };
        let rows = parse_cf(rows_arr);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "000001");
        assert_eq!(rows[0].split_date, "2025-01-10");
        assert!(approx(rows[0].split_ratio, 0.5));
        assert_eq!(rows[1].split_type, "份额折算");
    }

    #[test]
    fn parses_fund_fh_rank_em() {
        let v = extract_js_array(
            &std::fs::read_to_string(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures")
                    .join("fund_fh_rank_em.json"),
            )
            .unwrap(),
            ";var fhph_jjgs",
        )
        .unwrap();
        let arr = v.as_array().unwrap();
        // Eastmoney wraps the row list one level deeper: `[[[row],[row]]]`.
        let rows_arr: &[Value] = match arr.as_slice() {
            [Value::Array(inner)] => &inner[..],
            other => other,
        };
        let rows = parse_fh_rank(rows_arr);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "000001");
        assert!(approx(rows[0].total_dividend, 5.5));
        assert!(approx(rows[0].total_count, 3.0));
        assert_eq!(rows[1].establish_date, "2002-05-08");
    }

    #[test]
    fn parses_fund_scale_open_sina() {
        let v = fixture("fund_scale_open_sina.json");
        let arr = v.get("data").unwrap().as_array().unwrap();
        let rows = parse_scale_sina(arr, "股票型基金");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "000001");
        assert!(approx(rows[0].nav, 1.234));
        assert!(approx(rows[0].total_scale, 500.0));
        assert_eq!(rows[0].manager, "张三");
        assert_eq!(rows[0].source, "sina");
    }

    #[test]
    fn parses_fund_scale_close_sina() {
        let v = fixture("fund_scale_close_sina.json");
        let arr = v.get("data").unwrap().as_array().unwrap();
        let rows = parse_scale_sina(arr, "封闭式基金");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].code, "500001");
        assert_eq!(rows[0].category, "封闭式基金");
    }

    #[test]
    fn parses_fund_scale_structured_sina() {
        let v = fixture("fund_scale_structured_sina.json");
        let arr = v.get("data").unwrap().as_array().unwrap();
        let rows = parse_scale_sina(arr, "分级子基金");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].code, "150001");
        assert_eq!(rows[0].category, "分级子基金");
    }

    #[test]
    fn parses_fund_etf_category_ths() {
        let v = fixture("fund_etf_category_ths.json");
        let data = v.get("data").unwrap().get("data").unwrap().as_object().unwrap();
        let items: Vec<Value> = data.values().cloned().collect();
        let rows = parse_etf_ths(&items);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "510050");
        assert_eq!(rows[0].name, "华夏上证50ETF");
        assert!(approx(rows[0].net, 1.0));
        assert!(approx(rows[0].rate, 1.0));
        assert_eq!(rows[1].sgstat, "开放");
        assert_eq!(rows[0].source, "ths");
    }
}
