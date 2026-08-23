//! Additional A-share / global index endpoints ported from `akshare/index/`.
//!
//! This module collects the remaining tractable **pure-JSON** index endpoints
//! (Eastmoney `push2`/`push2his` and a couple of Sina feeds) that were not
//! covered by the sibling modules (`cons`, `cx`, `extra`, `qvix`,
//! `research_sw`, `stock_hk_us_zh`). Every function here hits Eastmoney
//! `datacenter`/`push2` (no JS / token / `execjs` / `MiniRacer`) or a plain
//! Sina JSON endpoint, so the whole surface ports with offline parser tests.
//!
//! | Rust function | akshare source | note |
//! |---|---|---|
//! | `index_zh_a_hist` | `index/index_zh_em.py:42` | Eastmoney `push2his` kline; resolves `secid` via `index_code_id_map_em` with the `1/0/2/47` fallback chain |
//! | `index_zh_a_daily` | `index/index_zh_em.py:42` | daily alias of `index_zh_a_hist` (`klt=101`) |
//! | `index_zh_a_hist_min_em` | `index/index_zh_em.py:178` | intraday: `trends2` for `period="1"`, else `kline` (`fqt=1`) |
//! | `index_zh_a_spot` | `index/index_stock_zh.py:129` (`__stock_zh_main_spot_em`) | Eastmoney `clist` 沪深重要指数 (`fltt=2`, values already real) |
//! | `index_global_spot_em` | `index/index_global_em.py:15` | Eastmoney `clist` 全球指数 (`fltt=1`, ÷100) |
//! | `index_global_hist_em` | `index/index_global_em.py:95` | Eastmoney `push2his` kline; `symbol` is the Chinese name from `index_global_em_symbol_map` |
//!
//! ## DEFERRED
//!
//! * The functions named in the task brief (`index_value_name`, `index_vix`,
//!   `index_institute_*`) do **not exist** in this akshare checkout (the
//!   `index/` package has been reorganized); the tractable JSON endpoints
//!   available were ported instead.
//! * `index_sw.py` (`sw_index_first/second/third_info`, `sw_index_third_cons`)
//!   — HTML scrape via `BeautifulSoup` / `pd.read_html`; no JSON API; DEFERRED
//!   (already noted in `extra.rs`).
//! * `index_drewry.py` (`drewry_wci_index`) — slices a `window.infographicData`
//!   blob out of a `<script>` and decodes with `demjson`; HTML + JS-object
//!   decoding, DEFERRED.
//! * `index_stock_zh_csindex.py` (`stock_zh_index_value_csindex`) — downloads
//!   `<symbol>indicator.xls` and parses with `pd.read_excel`; Excel download,
//!   DEFERRED.
//! * `index_stock_us_sina.py` / `stock_hk_index_daily_sina` — decrypt Sina feeds
//!   with `py_mini_racer` + `akshare.stock.cons.zh/hk_js_decode` (JS-signed
//!   payload); no JS runtime in the crate, DEFERRED.
//! * `index_research_fund_sw.py` (`index_hist_fund_sw`, `index_realtime_fund_sw`)
//!   — POST a JSON body; `Client` only supports query-string POST, DEFERRED.

use std::collections::HashMap;

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};
use crate::core::json::*;

// Eastmoney quote API base URLs (push2 = realtime clist, push2his = history).
const EM_KLINE_URL: &str = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
const EM_TRENDS_URL: &str = "https://push2his.eastmoney.com/api/qt/stock/trends2/get";
// Static `ut` tokens copied verbatim from akshare (not JS-computed at runtime).
const EM_UT: &str = "bd1d9ddb04089700cf9c27f6f7426281";
const EM_UT2: &str = "7eea3edcaed734bea9cbfc24409ed989";

/// `fs` filter for the A-share index code→id map (akshare `index_code_id_map_em`).
const EM_CODE_MAP_FS: &str = "b:MK0010,m:1+t:1,m:0 t:5,m:1+s:3,m:0+t:5,m:2";

/// `fs` filter for the 全球指数 realtime clist (akshare `index_global_spot_em`).
const EM_GLOBAL_FS: &str = "i:1.000001,i:0.399001,i:0.399005,i:0.399006,i:1.000300,i:100.HSI,\
i:100.HSCEI,i:124.HSCCI,i:100.TWII,i:100.N225,i:100.KOSPI200,i:100.KS11,i:100.STI,i:100.SENSEX,\
i:100.KLSE,i:100.SET,i:100.PSI,i:100.KSE100,i:100.VNINDEX,i:100.JKSE,i:100.CSEALL,i:100.SX5E,\
i:100.FTSE,i:100.MCX,i:100.AXX,i:100.FCHI,i:100.GDAXI,i:100.RTS,i:100.IBEX,i:100.PSI20,i:100.OMXC20,\
i:100.BFX,i:100.AEX,i:100.WIG,i:100.OMXSPI,i:100.SSMI,i:100.HEX,i:100.OSEBX,i:100.ATX,i:100.MIB,\
i:100.ASE,i:100.ICEXI,i:100.PX,i:100.ISEQ,i:100.DJIA,i:100.SPX,i:100.NDX,i:100.TSX,i:100.BVSP,\
i:100.MXX,i:100.AS51,i:100.AORD,i:100.NZ50,i:100.UDI,i:100.BDI,i:100.CRB";

// ---------------------------------------------------------------------------
// Shared field helpers
// ---------------------------------------------------------------------------


/// Read a field that may be a JSON string or number, returning it as a `String`
/// (used for Eastmoney `f13` ids, which are numbers in some responses).
fn fid(item: &Value, k: &str) -> String {
    match item.get(k) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}


/// `fnum` divided by `div` (Eastmoney returns some feeds scaled ×100).
fn fnum_div(item: &Value, k: &str, div: f64) -> Option<f64> {
    opt_f64(item, k).map(|x| x / div)
}


/// Extract the `data.diff` array (a row array) from an Eastmoney clist response.
fn em_diff_array(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff".into(),
        })
}

/// Extract the `data.klines` array (array of CSV strings) from a kline response.
fn em_klines_array(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("data")
        .and_then(|d| d.get("klines"))
        .and_then(|k| k.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.klines".into(),
        })
}

/// Convert `YYYYMMDD` to `YYYY-MM-DD` for lexicographic date filtering.
fn ymd(d: &str) -> String {
    if d.len() >= 8 {
        format!("{}-{}-{}", &d[0..4], &d[4..6], &d[6..8])
    } else {
        d.to_string()
    }
}

// ---------------------------------------------------------------------------
// secid resolution (mirrors akshare index_code_id_map_em + fallback chain)
// ---------------------------------------------------------------------------

/// Build the `code -> internal market id` map that Eastmoney kline lookups need
/// to assemble a `secid` (`marketId.code`). Mirrors akshare `index_code_id_map_em`.
async fn index_code_id_map_em(client: &Client) -> Result<HashMap<String, String>> {
    let params = [
        ("pn", "1"),
        ("pz", "100"),
        ("po", "1"),
        ("np", "1"),
        ("ut", EM_UT),
        ("fltt", "2"),
        ("invt", "2"),
        ("fid", "f3"),
        ("fs", EM_CODE_MAP_FS),
        ("fields", "f3,f12,f13"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "index_code_id_map_em", &crate::core::eastmoney_push::push2_url("/api/qt/clist/get").await,
            &params,
        )
        .await?;
    let diff = em_diff_array(&v)?;
    let mut m = HashMap::new();
    for item in diff {
        let code = opt_str_or(item, "f12", "");
        if !code.is_empty() {
            m.insert(code, fid(item, "f13"));
        }
    }
    Ok(m)
}

/// Candidate `secid`s for an A-share index code: the resolved market id first,
/// then the static fallbacks akshare tries (`1`, `0`, `2`, `47`).
async fn zh_a_secid_candidates(client: &Client, symbol: &str) -> Result<Vec<String>> {
    let map = index_code_id_map_em(client).await?;
    let mut c = Vec::with_capacity(5);
    if let Some(id) = map.get(symbol) {
        c.push(format!("{id}.{symbol}"));
    }
    for m in ["1", "0", "2", "47"] {
        c.push(format!("{m}.{symbol}"));
    }
    Ok(c)
}

/// Try each candidate `secid` against the kline endpoint and return the first
/// response that actually carries rows, or `None` if none resolve.
async fn fetch_zh_a_kline(
    client: &Client,
    symbol: &str,
    klt: &str,
    fqt: &str,
) -> Result<Option<Value>> {
    let candidates = zh_a_secid_candidates(client, symbol).await?;
    for secid in candidates {
        let params: Vec<(&str, &str)> = vec![
            ("secid", &secid),
            ("ut", EM_UT2),
            ("fields1", "f1,f2,f3,f4,f5,f6"),
            ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61"),
            ("klt", klt),
            ("fqt", fqt),
            ("beg", "0"),
            ("end", "20500000"),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, "index_zh_a_hist", EM_KLINE_URL, &params)
            .await?;
        let has = v
            .get("data")
            .and_then(|d| d.get("klines"))
            .and_then(|k| k.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false);
        if has {
            return Ok(Some(v));
        }
    }
    Ok(None)
}

/// Try each candidate `secid` for an intraday (minute) feed. Returns the first
/// response that carries rows plus a flag for whether it came from the `trends2`
/// (`period="1"`) or `kline` (`period!=1`) endpoint.
async fn fetch_zh_a_min(
    client: &Client,
    symbol: &str,
    period: &str,
) -> Result<Option<(Value, bool)>> {
    let candidates = zh_a_secid_candidates(client, symbol).await?;
    if period == "1" {
        for secid in candidates {
            let params: Vec<(&str, &str)> = vec![
                ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f11,f12,f13"),
                ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58"),
                ("iscr", "0"),
                ("ndays", "5"),
                ("secid", &secid),
            ];
            let v = client
                .get_json(
                    SOURCE_EASTMONEY,
                    "index_zh_a_hist_min_em",
                    EM_TRENDS_URL,
                    &params,
                )
                .await?;
            let has = v
                .get("data")
                .and_then(|d| d.get("trends"))
                .and_then(|t| t.as_array())
                .map(|a| !a.is_empty())
                .unwrap_or(false);
            if has {
                return Ok(Some((v, true)));
            }
        }
    } else {
        for secid in candidates {
            let params: Vec<(&str, &str)> = vec![
                ("secid", &secid),
                ("ut", EM_UT2),
                ("fields1", "f1,f2,f3,f4,f5,f6"),
                ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61"),
                ("klt", period),
                ("fqt", "1"),
                ("beg", "0"),
                ("end", "20500000"),
            ];
            let v = client
                .get_json(
                    SOURCE_EASTMONEY,
                    "index_zh_a_hist_min_em",
                    EM_KLINE_URL,
                    &params,
                )
                .await?;
            let has = v
                .get("data")
                .and_then(|d| d.get("klines"))
                .and_then(|k| k.as_array())
                .map(|a| !a.is_empty())
                .unwrap_or(false);
            if has {
                return Ok(Some((v, false)));
            }
        }
    }
    Ok(None)
}

// ===========================================================================
// index_zh_a_hist / index_zh_a_daily  (index_zh_em.py:42)
// ===========================================================================

/// A single daily bar of an A-share index (Eastmoney `push2his` kline).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ZhAHistRow {
    /// 日期
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
    pub turnover: Option<f64>,
}

/// Parse `index_zh_a_hist` kline rows. The upstream emits 11 fields; fixtures
/// commonly carry 8, so trailing columns are read when present and `None`d
/// otherwise.
pub(crate) fn parse_index_zh_a_hist(resp: &Value) -> Result<Vec<ZhAHistRow>> {
    let klines = em_klines_array(resp)?;
    let mut out = Vec::with_capacity(klines.len());
    for line in klines {
        let s = line.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "zh a hist kline entry is not a string".into(),
        })?;
        let p: Vec<&str> = s.split(',').collect();
        if p.len() < 8 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: format!("zh a hist kline has {} fields, expected >= 8", p.len()),
            });
        }
        out.push(ZhAHistRow {
            date: p[0].to_string(),
            open: parse_f64_str(p[1]),
            close: parse_f64_str(p[2]),
            high: parse_f64_str(p[3]),
            low: parse_f64_str(p[4]),
            volume: parse_f64_str(p[5]),
            amount: parse_f64_str(p[6]),
            amplitude: parse_f64_str(p[7]),
            pct_change: p.get(8).and_then(|s| parse_f64_str(s)),
            change: p.get(9).and_then(|s| parse_f64_str(s)),
            turnover: p.get(10).and_then(|s| parse_f64_str(s)),
        });
    }
    Ok(out)
}

/// 东方财富网-中国股票指数-行情数据 (daily/weekly/monthly). `period` is one of
/// `{"daily","weekly","monthly"}`; `start_date`/`end_date` use `YYYYMMDD`.
pub async fn index_zh_a_hist(
    client: &Client,
    symbol: &str,
    period: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<ZhAHistRow>> {
    let klt = match period {
        "weekly" => "102",
        "monthly" => "103",
        _ => "101",
    };
    let v = fetch_zh_a_kline(client, symbol, klt, "0").await?;
    let mut rows = match v {
        Some(v) => parse_index_zh_a_hist(&v)?,
        None => return Ok(Vec::new()),
    };
    let s = ymd(start_date);
    let e = ymd(end_date);
    rows.retain(|r| r.date.as_str() >= s.as_str() && r.date.as_str() <= e.as_str());
    Ok(rows)
}

/// 东方财富网-中国股票指数-日频行情数据 (daily alias of [`index_zh_a_hist`]).
pub async fn index_zh_a_daily(
    client: &Client,
    symbol: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<ZhAHistRow>> {
    index_zh_a_hist(client, symbol, "daily", start_date, end_date).await
}

// ===========================================================================
// index_zh_a_hist_min_em  (index_zh_em.py:178)
// ===========================================================================

/// A single intraday (minute) observation of an A-share index.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ZhAMinRow {
    /// 时间
    pub time: String,
    /// 代码 (from `data.code`)
    pub code: Option<String>,
    /// 名称 (from `data.name`)
    pub name: Option<String>,
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
    /// 均价 (trends2 path only)
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

/// Parse the `trends2` (`period="1"`) response: `data.trends` CSV strings with
/// `[时间,开盘,收盘,最高,最低,成交量,成交额,均价]` (8 fields).
pub(crate) fn parse_index_zh_a_hist_min_trends(resp: &Value) -> Result<Vec<ZhAMinRow>> {
    let data = resp.get("data").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "missing data".into(),
    })?;
    let code = opt_str_or(data, "code", "");
    let name = opt_str_or(data, "name", "");
    let trends = data
        .get("trends")
        .and_then(|t| t.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.trends".into(),
        })?;
    let mut out = Vec::with_capacity(trends.len());
    for line in trends {
        let s = line.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "zh a min trend entry is not a string".into(),
        })?;
        let p: Vec<&str> = s.split(',').collect();
        if p.len() < 8 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: format!("zh a min trend has {} fields, expected >= 8", p.len()),
            });
        }
        out.push(ZhAMinRow {
            time: p[0].to_string(),
            code: Some(code.clone()),
            name: Some(name.clone()),
            open: parse_f64_str(p[1]),
            close: parse_f64_str(p[2]),
            high: parse_f64_str(p[3]),
            low: parse_f64_str(p[4]),
            volume: parse_f64_str(p[5]),
            amount: parse_f64_str(p[6]),
            avg_price: parse_f64_str(p[7]),
            amplitude: None,
            pct_change: None,
            change: None,
            turnover: None,
        });
    }
    Ok(out)
}

/// Parse the `kline` (`period!="1"`) response: `data.klines` CSV strings with
/// `[时间,开盘,收盘,最高,最低,成交量,成交额,振幅,涨跌幅,涨跌额,换手率]`.
pub(crate) fn parse_index_zh_a_hist_min_kline(resp: &Value) -> Result<Vec<ZhAMinRow>> {
    let data = resp.get("data").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "missing data".into(),
    })?;
    let code = opt_str_or(data, "code", "");
    let name = opt_str_or(data, "name", "");
    let klines = em_klines_array(resp)?;
    let mut out = Vec::with_capacity(klines.len());
    for line in klines {
        let s = line.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "zh a min kline entry is not a string".into(),
        })?;
        let p: Vec<&str> = s.split(',').collect();
        if p.len() < 7 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: format!("zh a min kline has {} fields, expected >= 7", p.len()),
            });
        }
        out.push(ZhAMinRow {
            time: p[0].to_string(),
            code: Some(code.clone()),
            name: Some(name.clone()),
            open: parse_f64_str(p[1]),
            close: parse_f64_str(p[2]),
            high: parse_f64_str(p[3]),
            low: parse_f64_str(p[4]),
            volume: parse_f64_str(p[5]),
            amount: parse_f64_str(p[6]),
            avg_price: None,
            amplitude: p.get(7).and_then(|s| parse_f64_str(s)),
            pct_change: p.get(8).and_then(|s| parse_f64_str(s)),
            change: p.get(9).and_then(|s| parse_f64_str(s)),
            turnover: p.get(10).and_then(|s| parse_f64_str(s)),
        });
    }
    Ok(out)
}

/// 东方财富网-指数数据-分时/分钟行情. `period` is `"1"` (trends2) or a kline
/// `klt` (`"5"`,`"15"`,`"30"`,`"60"`). `start_date`/`end_date` use `YYYYMMDD`.
pub async fn index_zh_a_hist_min_em(
    client: &Client,
    symbol: &str,
    period: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<ZhAMinRow>> {
    let fetched = fetch_zh_a_min(client, symbol, period).await?;
    let (v, is_trends) = match fetched {
        Some(x) => x,
        None => return Ok(Vec::new()),
    };
    let mut rows = if is_trends {
        parse_index_zh_a_hist_min_trends(&v)?
    } else {
        parse_index_zh_a_hist_min_kline(&v)?
    };
    let s = ymd(start_date);
    let e = ymd(end_date);
    rows.retain(|r| r.time.as_str() >= s.as_str() && r.time.as_str() <= e.as_str());
    Ok(rows)
}

// ===========================================================================
// index_zh_a_spot  (index_stock_zh.py:129 __stock_zh_main_spot_em)
// ===========================================================================

/// A realtime quote of a major A-share index (`fltt=2`, values already real).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ZhASpotRow {
    /// 序号
    pub index: Option<u32>,
    /// 代码 (Eastmoney `f12`)
    pub code: String,
    /// 名称 (Eastmoney `f14`)
    pub name: String,
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
    /// 量比 (Eastmoney `f10`)
    pub volume_ratio: Option<f64>,
    /// 最高 (Eastmoney `f15`)
    pub high: Option<f64>,
    /// 最低 (Eastmoney `f16`)
    pub low: Option<f64>,
    /// 今开 (Eastmoney `f17`)
    pub open: Option<f64>,
    /// 昨收 (Eastmoney `f18`)
    pub pre_close: Option<f64>,
}

/// Parse `index_zh_a_spot` rows from a `data.diff` array (`fltt=2` → no ÷100).
pub(crate) fn parse_index_zh_a_spot(items: &[Value]) -> Vec<ZhASpotRow> {
    let mut out = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        out.push(ZhASpotRow {
            index: Some((i + 1) as u32),
            code: opt_str_or(item, "f12", ""),
            name: opt_str_or(item, "f14", ""),
            price: opt_f64(item, "f2"),
            pct_change: opt_f64(item, "f3"),
            change: opt_f64(item, "f4"),
            volume: opt_f64(item, "f5"),
            amount: opt_f64(item, "f6"),
            amplitude: opt_f64(item, "f7"),
            volume_ratio: opt_f64(item, "f10"),
            high: opt_f64(item, "f15"),
            low: opt_f64(item, "f16"),
            open: opt_f64(item, "f17"),
            pre_close: opt_f64(item, "f18"),
        });
    }
    out
}

/// 东方财富网-行情中心-沪深重要指数-实时行情 (`clist`, `fltt=2`).
pub async fn index_zh_a_spot(client: &Client) -> Result<Vec<ZhASpotRow>> {
    let params = [
        ("pn", "1"),
        ("pz", "100"),
        ("po", "1"),
        ("np", "1"),
        ("ut", EM_UT),
        ("fltt", "2"),
        ("invt", "2"),
        ("dect", "1"),
        ("wbp2u", "|0|0|0|web"),
        ("fid", ""),
        ("fs", "b:MK0010"),
        (
            "fields",
            "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f12,f13,f14,f15,f16,f17,f18,f20,f21,f23,f24,f25,f26,f22,f11,f62,f128,f136,f115,f152",
        ),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "index_zh_a_spot", &crate::core::eastmoney_push::push2_url("/api/qt/clist/get").await, &params)
        .await?;
    Ok(parse_index_zh_a_spot(em_diff_array(&v)?))
}

// ===========================================================================
// index_global_spot_em  (index_global_em.py:15)
// ===========================================================================

/// A realtime quote of a global index (`fltt=1`, values scaled ×100 → ÷100).
#[derive(Debug, Clone, serde::Serialize)]
pub struct GlobalSpotEmRow {
    /// 序号
    pub index: Option<u32>,
    /// 代码 (Eastmoney `f12`)
    pub code: String,
    /// 名称 (Eastmoney `f14`)
    pub name: String,
    /// 最新价 (Eastmoney `f2` ÷100)
    pub price: Option<f64>,
    /// 涨跌额 (Eastmoney `f4` ÷100)
    pub change: Option<f64>,
    /// 涨跌幅 (Eastmoney `f3` ÷100)
    pub pct_change: Option<f64>,
    /// 今开 (Eastmoney `f17` ÷100)
    pub open: Option<f64>,
    /// 最高 (Eastmoney `f15` ÷100)
    pub high: Option<f64>,
    /// 最低 (Eastmoney `f16` ÷100)
    pub low: Option<f64>,
    /// 昨收 (Eastmoney `f18` ÷100)
    pub pre_close: Option<f64>,
    /// 振幅 (Eastmoney `f7` ÷100)
    pub amplitude: Option<f64>,
    /// 最新行情时间 (Eastmoney `f124`, unix seconds)
    pub time: Option<i64>,
}

/// Parse `index_global_spot_em` rows from a `data.diff` array (`fltt=1` → ÷100).
pub(crate) fn parse_index_global_spot_em(items: &[Value]) -> Vec<GlobalSpotEmRow> {
    let mut out = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        out.push(GlobalSpotEmRow {
            index: Some((i + 1) as u32),
            code: opt_str_or(item, "f12", ""),
            name: opt_str_or(item, "f14", ""),
            price: fnum_div(item, "f2", 100.0),
            change: fnum_div(item, "f4", 100.0),
            pct_change: fnum_div(item, "f3", 100.0),
            open: fnum_div(item, "f17", 100.0),
            high: fnum_div(item, "f15", 100.0),
            low: fnum_div(item, "f16", 100.0),
            pre_close: fnum_div(item, "f18", 100.0),
            amplitude: fnum_div(item, "f7", 100.0),
            time: item.get("f124").and_then(|v| v.as_i64()),
        });
    }
    out
}

/// 东方财富网-行情中心-全球指数-实时行情 (`clist`, `fltt=1`).
pub async fn index_global_spot_em(client: &Client) -> Result<Vec<GlobalSpotEmRow>> {
    let params = [
        ("np", "2"),
        ("fltt", "1"),
        ("invt", "2"),
        ("fs", EM_GLOBAL_FS),
        (
            "fields",
            "f12,f13,f14,f292,f1,f2,f4,f3,f152,f17,f18,f15,f16,f7,f124",
        ),
        ("fid", "f3"),
        ("pn", "1"),
        ("pz", "200"),
        ("po", "1"),
        ("dect", "1"),
        ("wbp2u", "|0|0|0|web"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "index_global_spot_em", &crate::core::eastmoney_push::push2_url("/api/qt/clist/get").await,
            &params,
        )
        .await?;
    Ok(parse_index_global_spot_em(em_diff_array(&v)?))
}

// ===========================================================================
// index_global_hist_em  (index_global_em.py:95)
// ===========================================================================

/// A single historical bar of a global index (Eastmoney `push2his` kline).
#[derive(Debug, Clone, serde::Serialize)]
pub struct GlobalHistEmRow {
    /// 日期
    pub date: String,
    /// 代码 (from `data.code`)
    pub code: String,
    /// 名称 (from `data.name`)
    pub name: String,
    /// 今开
    pub open: Option<f64>,
    /// 最新价
    pub close: Option<f64>,
    /// 最高
    pub high: Option<f64>,
    /// 最低
    pub low: Option<f64>,
    /// 振幅
    pub amplitude: Option<f64>,
}

/// Static mirror of akshare `index_global_em_symbol_map` (name → market, code).
const INDEX_GLOBAL_EM_SYMBOLS: &[(&str, &str, &str)] = &[
    ("波罗的海BDI指数", "100", "BDI"),
    ("葡萄牙PSI20", "100", "PSI20"),
    ("菲律宾马尼拉", "100", "PSI"),
    ("泰国SET", "100", "SET"),
    ("俄罗斯RTS", "100", "RTS"),
    ("巴基斯坦卡拉奇", "100", "KSE100"),
    ("越南胡志明", "100", "VNINDEX"),
    ("红筹指数", "124", "HSCCI"),
    ("印尼雅加达综合", "100", "JKSE"),
    ("希腊雅典ASE", "100", "ASE"),
    ("墨西哥BOLSA", "100", "MXX"),
    ("挪威OSEBX", "100", "OSEBX"),
    ("巴西BOVESPA", "100", "BVSP"),
    ("波兰WIG", "100", "WIG"),
    ("印度孟买SENSEX", "100", "SENSEX"),
    ("布拉格指数", "100", "PX"),
    ("荷兰AEX", "100", "AEX"),
    ("冰岛ICEX", "100", "ICEXI"),
    ("斯里兰卡科伦坡", "100", "CSEALL"),
    ("富时新加坡海峡时报", "100", "STI"),
    ("富时意大利MIB", "100", "MIB"),
    ("路透CRB商品指数", "100", "CRB"),
    ("比利时BFX", "100", "BFX"),
    ("富时AIM全股", "100", "AXX"),
    ("新西兰50", "100", "NZ50"),
    ("上证指数", "1", "000001"),
    ("国企指数", "100", "HSCEI"),
    ("沪深300", "1", "000300"),
    ("英国富时100", "100", "FTSE"),
    ("中小100", "0", "399005"),
    ("瑞士SMI", "100", "SSMI"),
    ("西班牙IBEX35", "100", "IBEX"),
    ("瑞典OMXSPI", "100", "OMXSPI"),
    ("爱尔兰综合", "100", "ISEQ"),
    ("韩国KOSPI", "100", "KS11"),
    ("深证成指", "0", "399001"),
    ("韩国KOSPI200", "100", "KOSPI200"),
    ("芬兰赫尔辛基", "100", "HEX"),
    ("恒生指数", "100", "HSI"),
    ("欧洲斯托克50", "100", "SX5E"),
    ("美元指数", "100", "UDI"),
    ("法国CAC40", "100", "FCHI"),
    ("台湾加权", "100", "TWII"),
    ("英国富时250", "100", "MCX"),
    ("富时马来西亚KLCI", "100", "KLSE"),
    ("OMX哥本哈根20", "100", "OMXC20"),
    ("道琼斯", "100", "DJIA"),
    ("奥地利ATX", "100", "ATX"),
    ("加拿大S&P/TSX", "100", "TSX"),
    ("德国DAX30", "100", "GDAXI"),
    ("创业板指", "0", "399006"),
    ("澳大利亚普通股", "100", "AORD"),
    ("标普500", "100", "SPX"),
    ("澳大利亚标普200", "100", "AS51"),
    ("日经225", "100", "N225"),
    ("纳斯达克", "100", "NDX"),
];

/// Look up a global-index Chinese name in `INDEX_GLOBAL_EM_SYMBOLS`.
fn global_em_symbol(symbol: &str) -> Option<(&'static str, &'static str)> {
    INDEX_GLOBAL_EM_SYMBOLS
        .iter()
        .find(|(name, _, _)| *name == symbol)
        .map(|(_, market, code)| (*market, *code))
}

/// Parse `index_global_hist_em` kline rows (14 upstream fields; we keep
/// `[日期,今开,最新价,最高,最低,振幅]`, plus `data.code`/`data.name`).
pub(crate) fn parse_index_global_hist_em(resp: &Value) -> Result<Vec<GlobalHistEmRow>> {
    let data = resp.get("data").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "missing data".into(),
    })?;
    let code = opt_str_or(data, "code", "");
    let name = opt_str_or(data, "name", "");
    let klines = em_klines_array(resp)?;
    let mut out = Vec::with_capacity(klines.len());
    for line in klines {
        let s = line.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "global hist kline entry is not a string".into(),
        })?;
        let p: Vec<&str> = s.split(',').collect();
        if p.len() < 8 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: format!("global hist kline has {} fields, expected >= 8", p.len()),
            });
        }
        out.push(GlobalHistEmRow {
            date: p[0].to_string(),
            code: code.clone(),
            name: name.clone(),
            open: parse_f64_str(p[1]),
            close: parse_f64_str(p[2]),
            high: parse_f64_str(p[3]),
            low: parse_f64_str(p[4]),
            amplitude: parse_f64_str(p[7]),
        });
    }
    Ok(out)
}

/// 东方财富网-行情中心-全球指数-历史行情 (`push2his` kline). `symbol` is the
/// Chinese index name (e.g. `"美元指数"`); resolved via `global_em_symbol`.
pub async fn index_global_hist_em(client: &Client, symbol: &str) -> Result<Vec<GlobalHistEmRow>> {
    let (market, code) = global_em_symbol(symbol).ok_or_else(|| {
        Error::InvalidParam(format!("unknown global em symbol: {symbol}"))
    })?;
    let secid = format!("{market}.{code}");
    let params: Vec<(&str, &str)> = vec![
        ("secid", &secid),
        ("klt", "101"),
        ("fqt", "1"),
        ("lmt", "50000"),
        ("end", "20500000"),
        ("iscca", "1"),
        ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8"),
        ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f62,f63,f64"),
        ("ut", "f057cbcbce2a86e2866ab8877db1d059"),
        ("forcect", "1"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "index_global_hist_em",
            EM_KLINE_URL,
            &params,
        )
        .await?;
    parse_index_global_hist_em(&v)
}

// ===========================================================================
// Offline parse tests (≥1 per ported function)
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

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    #[test]
    fn parse_index_zh_a_hist_ok() {
        let rows = parse_index_zh_a_hist(&fixture("index_zh_a_hist_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2025-01-02");
        assert!(approx(rows[0].open, 3200.0));
        assert!(approx(rows[0].close, 3250.0));
        assert!(approx(rows[0].high, 3260.0));
        assert!(approx(rows[0].low, 3190.0));
        assert!(approx(rows[0].volume, 234567890.0));
        assert!(approx(rows[0].amount, 2500000000.0));
        assert!(approx(rows[0].amplitude, 2.20));
        // trailing fields absent in the 8-field fixture
        assert_eq!(rows[0].pct_change, None);
        assert_eq!(rows[0].change, None);
        assert_eq!(rows[0].turnover, None);
        assert_eq!(rows[1].date, "2025-01-03");
    }

    #[test]
    fn parse_index_zh_a_daily_ok() {
        // daily shares the hist parser / kline shape
        let rows = parse_index_zh_a_hist(&fixture("index_zh_a_daily.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2025-01-02");
        assert!(approx(rows[0].close, 3250.0));
        assert!(approx(rows[0].amplitude, 2.20));
    }

    #[test]
    fn parse_index_zh_a_hist_min_trends_ok() {
        let rows = parse_index_zh_a_hist_min_trends(&fixture("index_zh_a_hist_min_em_trends.json"))
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].time, "2025-03-17 09:31:00");
        assert_eq!(rows[0].code, Some("399006".to_string()));
        assert_eq!(rows[0].name, Some("创业板指".to_string()));
        assert!(approx(rows[0].open, 2000.0));
        assert!(approx(rows[0].close, 2010.0));
        assert!(approx(rows[0].avg_price, 2008.0));
        assert_eq!(rows[0].amplitude, None);
        assert_eq!(rows[1].close, Some(2012.0));
    }

    #[test]
    fn parse_index_zh_a_hist_min_kline_ok() {
        let rows = parse_index_zh_a_hist_min_kline(&fixture("index_zh_a_hist_min_em_kline.json"))
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].time, "2025-03-17 09:35:00");
        assert_eq!(rows[0].code, Some("000300".to_string()));
        assert_eq!(rows[0].name, Some("沪深300".to_string()));
        assert!(approx(rows[0].open, 3500.0));
        assert!(approx(rows[0].close, 3510.0));
        assert!(approx(rows[0].amplitude, 0.90));
        assert!(approx(rows[0].pct_change, 0.30));
        assert!(approx(rows[0].change, 10.0));
        assert!(approx(rows[0].turnover, 0.05));
        assert_eq!(rows[0].avg_price, None);
        assert_eq!(rows[1].close, Some(3505.0));
    }

    #[test]
    fn parse_index_zh_a_spot_ok() {
        let rows = parse_index_zh_a_spot(em_diff_array(&fixture("index_zh_a_spot.json")).unwrap());
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].index, Some(1));
        assert_eq!(rows[0].code, "000001");
        assert_eq!(rows[0].name, "上证指数");
        assert!(approx(rows[0].price, 3200.50));
        assert!(approx(rows[0].pct_change, 1.20));
        assert!(approx(rows[0].change, 38.00));
        assert!(approx(rows[0].amplitude, 1.18));
        assert!(approx(rows[0].volume_ratio, 1.0));
        assert!(approx(rows[0].high, 3210.00));
        assert!(approx(rows[0].low, 3180.00));
        assert!(approx(rows[0].pre_close, 3162.50));
        assert_eq!(rows[1].name, "深证成指");
    }

    #[test]
    fn parse_index_global_spot_em_ok() {
        let rows =
            parse_index_global_spot_em(em_diff_array(&fixture("index_global_spot_em.json")).unwrap());
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].index, Some(1));
        assert_eq!(rows[0].code, "000001");
        assert_eq!(rows[0].name, "上证指数");
        // fltt=1 → raw values are ×100, parser divides by 100
        assert!(approx(rows[0].price, 3200.50));
        assert!(approx(rows[0].change, 38.0));
        assert!(approx(rows[0].pct_change, 1.2));
        assert!(approx(rows[0].open, 3195.0));
        assert!(approx(rows[0].high, 3210.0));
        assert!(approx(rows[0].low, 3180.0));
        assert!(approx(rows[0].pre_close, 3162.5));
        assert!(approx(rows[0].amplitude, 9.5));
        assert_eq!(rows[0].time, Some(1709289600));
        assert_eq!(rows[1].code, "HSI");
    }

    #[test]
    fn parse_index_global_hist_em_ok() {
        let rows = parse_index_global_hist_em(&fixture("index_global_hist_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2025-01-02");
        assert_eq!(rows[0].code, "UDI");
        assert_eq!(rows[0].name, "美元指数");
        assert!(approx(rows[0].open, 102.5));
        assert!(approx(rows[0].close, 103.1));
        assert!(approx(rows[0].high, 103.4));
        assert!(approx(rows[0].low, 102.3));
        assert!(approx(rows[0].amplitude, 0.30));
        assert_eq!(rows[1].close, Some(102.8));
    }

    #[test]
    fn global_em_symbol_lookup_ok() {
        assert_eq!(global_em_symbol("美元指数"), Some(("100", "UDI")));
        assert_eq!(global_em_symbol("上证指数"), Some(("1", "000001")));
        assert_eq!(global_em_symbol("恒生指数"), Some(("100", "HSI")));
        assert_eq!(global_em_symbol("不存在的指数"), None);
    }
}
