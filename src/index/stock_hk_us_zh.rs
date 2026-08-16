//! HK / US / CN stock-index functions ported from akshare.
//!
//! Ports the pure-HTTP index endpoints from `akshare/index/`:
//!
//! | Rust function | akshare source | source / note |
//! |---|---|---|
//! | `stock_hk_index_spot_em` | `index/index_stock_hk.py:148` | Eastmoney push2 clist (`ut` is the static token the crate already uses in `stock/index/eastmoney.rs`) |
//! | `stock_hk_index_daily_em` | `index/index_stock_hk.py:235` | Eastmoney push2his kline; builds `secid` from the spot list |
//! | `stock_hk_index_spot_sina` | `index/index_stock_hk.py:54` | Sina `hq.sinajs.cn` text, fixed code list + `Referer` |
//! | `stock_zh_index_daily_em` | `index/index_stock_zh.py:428` | Eastmoney push2his kline |
//! | `stock_zh_index_daily_tx` | `index/index_stock_zh.py:354` | Tencent JSONP kline (`_var` wrapper stripped) |
//!
//! These are Eastmoney quote / Sina / Tencent endpoints, **not** the
//! `datacenter-web` API, so the `emg_data_array` helper does not apply; the
//! response shapes are `data.diff` (clist) and `data.klines` (kline).
//!
//! ## DEFERRED
//!
//! * `stock_hk_index_daily_sina` (`index/index_stock_hk.py:121`) — decrypts the
//!   Sina daily feed with `py_mini_racer` + `akshare.stock.cons.hk_js_decode`
//!   (JS-signed payload). No JS runtime in the crate.
//! * `index_us_stock_sina` (`index/index_stock_us_sina.py:18`) — decrypts with
//!   `py_mini_racer` + `akshare.stock.cons.zh_js_decode` (JS-signed payload).
//! * `stock_zh_index_value_csindex` (`index/index_stock_zh_csindex.py:72`) —
//!   downloads `<symbol>indicator.xls` and parses with `pd.read_excel`
//!   (Excel/ ZIP download, not JSON).
//! * `index_hist_fund_sw` (`index/index_research_fund_sw.py:61`) — POSTs a JSON
//!   request body (`json=payload`); the `Client` only supports query-string
//!   POST via `post_form_json`, so a JSON body cannot be sent.
//! * `index_realtime_fund_sw` (`index/index_research_fund_sw.py:15`) — same
//!   JSON-body POST blocker as `index_hist_fund_sw`.

use std::collections::HashMap;

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY, SOURCE_SINA};
use crate::core::error::{Error, Result};

const SOURCE_TENCENT: &str = "tencent";

// Eastmoney quote API base URLs (push2 = realtime clist, push2his = history kline).
const EM_CLIST_URL: &str = "https://push2.eastmoney.com/api/qt/clist/get";
const EM_KLINE_URL: &str = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
// Static `ut` token copied verbatim from akshare (not JS-computed at runtime).
const EM_UT: &str = "bd1d9ddb04089700cf9c27f6f7426281";

// Sina HK index realtime feed (fixed code list from akshare source).
const SINA_HK_LIST: &str = "hkCES100,hkCES120,hkCES280,hkCES300,hkCESA80,hkCESG10,\
hkCESHKM,hkCSCMC,hkCSHK100,hkCSHKDIV,hkCSHKLC,hkCSHKLRE,hkCSHKMCS,hkCSHKME,hkCSHKPE,hkCSHKSE,\
hkCSI300,hkCSRHK50,hkGEM,hkHKL,hkHSCCI,hkHSCEI,hkHSI,hkHSMBI,hkHSMOGI,hkHSMPI,hkHSTECH,hkSSE180,\
hkSSE180GV,hkSSE380,hkSSE50,hkSSECEQT,hkSSECOMP,hkSSEDIV,hkSSEITOP,hkSSEMCAP,hkSSEMEGA,hkVHSI";

// Tencent index history JSONP endpoint.
const TENCENT_KLINE_URL: &str =
    "https://proxy.finance.qq.com/ifzqgtimg/appstock/app/newfqkline/get";

// ---------------------------------------------------------------------------
// Row structs
// ---------------------------------------------------------------------------

/// HK index realtime quote from Eastmoney (`stock_hk_index_spot_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct HkIndexSpotEmRow {
    /// 序号 (Eastmoney `index`, 1-based)
    pub index: Option<u32>,
    /// 内部编号 (Eastmoney `f13`, used to build `secid` for kline lookups)
    pub inner_code: Option<String>,
    /// 代码 (Eastmoney `f12`)
    pub code: String,
    /// 名称 (Eastmoney `f14`)
    pub name: String,
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

/// HK index daily OHLC from Eastmoney (`stock_hk_index_daily_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct HkIndexDailyEmRow {
    pub date: String,
    pub open: Option<f64>,
    pub latest: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
}

/// HK index realtime quote from Sina (`stock_hk_index_spot_sina`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct HkIndexSpotSinaRow {
    pub code: String,
    pub name: String,
    pub latest: Option<f64>,
    pub change: Option<f64>,
    pub pct_change: Option<f64>,
    pub pre_close: Option<f64>,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
}

/// CN index daily OHLC from Eastmoney (`stock_zh_index_daily_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ZhIndexDailyEmRow {
    pub date: String,
    pub open: Option<f64>,
    pub close: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub volume: Option<f64>,
    pub amount: Option<f64>,
}

/// CN index daily OHLC from Tencent (`stock_zh_index_daily_tx`, 前复权).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ZhIndexDailyTxRow {
    pub date: String,
    pub open: Option<f64>,
    pub close: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub amount: Option<f64>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fstr(item: &Value, k: &str) -> String {
    item.get(k)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    })
}

fn parse_f64(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        t.parse::<f64>().ok()
    }
}

/// Strip a JSONP wrapper (`callback={...}` or `var x={...}`) down to the bare
/// JSON object so `serde_json` can parse it.
fn strip_jsonp(text: &str) -> &str {
    let start = text.find('{').unwrap_or(0);
    let end = text.rfind('}').map(|i| i + 1).unwrap_or(text.len());
    &text[start..end]
}

/// Extract the `data.diff` array from an Eastmoney clist response.
fn em_diff_array(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff".into(),
        })
}

/// Extract the `data.klines` array from an Eastmoney kline response.
fn em_klines_array(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("data")
        .and_then(|d| d.get("klines"))
        .and_then(|k| k.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.klines".into(),
        })
}

// ---------------------------------------------------------------------------
// stock_hk_index_spot_em  (akshare index_stock_hk.py:148)
// ---------------------------------------------------------------------------

/// Parse `stock_hk_index_spot_em` rows from a push2 clist response.
pub(crate) fn parse_hk_index_spot_em(resp: &Value) -> Result<Vec<HkIndexSpotEmRow>> {
    let diff = em_diff_array(resp)?;
    let mut out = Vec::with_capacity(diff.len());
    for (i, item) in diff.iter().enumerate() {
        out.push(HkIndexSpotEmRow {
            index: Some((i + 1) as u32),
            inner_code: fnum(item, "f13").map(|n| n.to_string()),
            code: fstr(item, "f12"),
            name: fstr(item, "f14"),
            price: fnum(item, "f2"),
            change: fnum(item, "f4"),
            pct_change: fnum(item, "f3"),
            open: fnum(item, "f17"),
            high: fnum(item, "f15"),
            low: fnum(item, "f16"),
            pre_close: fnum(item, "f18"),
            volume: fnum(item, "f5"),
            amount: fnum(item, "f6"),
        });
    }
    Ok(out)
}

/// 东方财富网-行情中心-港股-指数实时行情 (push2 clist, `fs=m:124,m:125,m:305`).
pub async fn stock_hk_index_spot_em(client: &Client) -> Result<Vec<HkIndexSpotEmRow>> {
    let params = [
        ("pn", "1"),
        ("pz", "100"),
        ("po", "1"),
        ("np", "1"),
        ("ut", EM_UT),
        ("fltt", "2"),
        ("invt", "2"),
        ("wbp2u", "|0|0|0|web"),
        ("fid", "f3"),
        ("fs", "m:124,m:125,m:305"),
        (
            "fields",
            "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f12,f13,f14,f15,f16,f17,f18,f20,f21,f23,f24,f25,f26,f22,f33,f11,f62,f128,f136,f115,f152",
        ),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_hk_index_spot_em",
            EM_CLIST_URL,
            &params,
        )
        .await?;
    parse_hk_index_spot_em(&v)
}

/// Build the `symbol -> internal code` map that `stock_hk_index_daily_em` needs
/// to assemble its `secid`. Mirrors akshare's `_symbol_code_dict` (plus the
/// hardcoded `HSAHP -> 100`).
async fn hk_index_spot_code_map(client: &Client) -> Result<HashMap<String, String>> {
    let rows = stock_hk_index_spot_em(client).await?;
    let mut m = HashMap::new();
    for r in rows {
        if let Some(inner) = &r.inner_code {
            m.insert(r.code.clone(), inner.clone());
        }
    }
    m.insert("HSAHP".to_string(), "100".to_string());
    Ok(m)
}

// ---------------------------------------------------------------------------
// stock_hk_index_daily_em  (akshare index_stock_hk.py:235)
// ---------------------------------------------------------------------------

/// Parse `stock_hk_index_daily_em` kline rows (`date,open,high,low,latest`).
pub(crate) fn parse_hk_index_klines(resp: &Value) -> Result<Vec<HkIndexDailyEmRow>> {
    let klines = em_klines_array(resp)?;
    let mut out = Vec::with_capacity(klines.len());
    for line in klines {
        let s = line.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "hk index kline entry is not a string".into(),
        })?;
        let p: Vec<&str> = s.split(',').collect();
        if p.len() < 5 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: format!("hk index kline has {} fields, expected >= 5", p.len()),
            });
        }
        out.push(HkIndexDailyEmRow {
            date: p[0].to_string(),
            open: parse_f64(p[1]),
            latest: parse_f64(p[2]),
            high: parse_f64(p[3]),
            low: parse_f64(p[4]),
        });
    }
    Ok(out)
}

/// 东方财富网-港股-股票指数数据 (push2his kline). Resolves `secid` from the
/// spot list, then fetches daily OHLC. `symbol` is the HK index code
/// (e.g. `"HSTECH"`).
pub async fn stock_hk_index_daily_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<HkIndexDailyEmRow>> {
    let map = hk_index_spot_code_map(client).await?;
    let inner = map.get(symbol).ok_or_else(|| {
        Error::InvalidParam(format!(
            "unknown HK index symbol: {symbol} (not in spot list)"
        ))
    })?;
    let secid = format!("{inner}.{symbol}");
    let params = [
        ("secid", secid.as_str()),
        ("klt", "101"),
        ("fqt", "1"),
        ("lmt", "10000"),
        ("end", "20500000"),
        ("iscca", "1"),
        ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8"),
        (
            "fields2",
            "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f62,f63,f64",
        ),
        ("ut", EM_UT),
        ("forcect", "1"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_hk_index_daily_em",
            EM_KLINE_URL,
            &params,
        )
        .await?;
    parse_hk_index_klines(&v)
}

// ---------------------------------------------------------------------------
// stock_hk_index_spot_sina  (akshare index_stock_hk.py:54)
// ---------------------------------------------------------------------------

/// Parse `stock_hk_index_spot_sina` rows from the raw `hq.sinajs.cn` text body.
///
/// Each non-empty line looks like `var hkHSI="hkHSI,恒生指数,28938.73,...";`.
/// akshare keeps columns `[代码,名称,最新价,涨跌额,涨跌幅,昨收,今开,最高,最低]`,
/// which map to inner positions `[0,1,6,7,8,3,2,4,5]`.
pub(crate) fn parse_hk_index_spot_sina(text: &str) -> Vec<HkIndexSpotSinaRow> {
    let mut out = Vec::new();
    for line in text.lines() {
        let inner = match line.split('"').nth(1) {
            Some(s) => s,
            None => continue,
        };
        let p: Vec<&str> = inner.split(',').collect();
        if p.len() < 9 {
            continue;
        }
        out.push(HkIndexSpotSinaRow {
            code: p[0].to_string(),
            name: p[1].to_string(),
            latest: parse_f64(p[6]),
            change: parse_f64(p[7]),
            pct_change: parse_f64(p[8]),
            pre_close: parse_f64(p[3]),
            open: parse_f64(p[2]),
            high: parse_f64(p[4]),
            low: parse_f64(p[5]),
        });
    }
    out
}

/// 新浪财经-行情中心-港股指数 (fixed code list, requires `Referer`).
pub async fn stock_hk_index_spot_sina(client: &Client) -> Result<Vec<HkIndexSpotSinaRow>> {
    let url = format!("https://hq.sinajs.cn/list={SINA_HK_LIST}");
    let headers = [("Referer", "https://vip.stock.finance.sina.com.cn/")];
    let text = client
        .get_text(
            SOURCE_SINA,
            "stock_hk_index_spot_sina",
            &url,
            &[],
            Some(&headers),
        )
        .await?;
    Ok(parse_hk_index_spot_sina(&text))
}

// ---------------------------------------------------------------------------
// stock_zh_index_daily_em  (akshare index_stock_zh.py:428)
// ---------------------------------------------------------------------------

/// Parse `stock_zh_index_daily_em` kline rows (`date,open,close,high,low,volume,amount`).
pub(crate) fn parse_zh_index_klines(resp: &Value) -> Result<Vec<ZhIndexDailyEmRow>> {
    let klines = em_klines_array(resp)?;
    let mut out = Vec::with_capacity(klines.len());
    for line in klines {
        let s = line.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "zh index kline entry is not a string".into(),
        })?;
        let p: Vec<&str> = s.split(',').collect();
        if p.len() < 7 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: format!("zh index kline has {} fields, expected >= 7", p.len()),
            });
        }
        out.push(ZhIndexDailyEmRow {
            date: p[0].to_string(),
            open: parse_f64(p[1]),
            close: parse_f64(p[2]),
            high: parse_f64(p[3]),
            low: parse_f64(p[4]),
            volume: parse_f64(p[5]),
            amount: parse_f64(p[6]),
        });
    }
    Ok(out)
}

/// 东方财富网-股票指数数据 (push2his kline). `symbol` carries a market prefix
/// (`sz`/`sh`/`csi`/`bj`); `start_date`/`end_date` use `YYYYMMDD`.
pub async fn stock_zh_index_daily_em(
    client: &Client,
    symbol: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<ZhIndexDailyEmRow>> {
    let market = if symbol.contains("sz") || symbol.contains("bj") {
        "0"
    } else if symbol.contains("sh") {
        "1"
    } else if symbol.contains("csi") {
        "2"
    } else {
        return Err(Error::InvalidParam(format!(
            "cannot infer market for index symbol: {symbol}"
        )));
    };
    let code = symbol
        .replace("sz", "")
        .replace("sh", "")
        .replace("bj", "")
        .replace("csi", "");
    let secid = format!("{market}.{code}");
    let params = [
        ("secid", secid.as_str()),
        ("fields1", "f1,f2,f3,f4,f5"),
        ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58"),
        ("klt", "101"),
        ("fqt", "0"),
        ("beg", start_date),
        ("end", end_date),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_zh_index_daily_em",
            EM_KLINE_URL,
            &params,
        )
        .await?;
    parse_zh_index_klines(&v)
}

// ---------------------------------------------------------------------------
// stock_zh_index_daily_tx  (akshare index_stock_zh.py:354)
// ---------------------------------------------------------------------------

/// Parse `stock_zh_index_daily_tx` rows from a Tencent JSONP response value.
///
/// `data[symbol]` holds either `"day"` or `"qfqday"`, each a list of
/// `[date, open, close, high, low, amount]` rows (前复权).
pub(crate) fn parse_zh_index_tx(resp: &Value, symbol: &str) -> Result<Vec<ZhIndexDailyTxRow>> {
    let data = resp
        .get("data")
        .and_then(|d| d.get(symbol))
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_TENCENT,
            message: format!("missing data.{symbol}"),
        })?;
    let arr = data
        .get("day")
        .or_else(|| data.get("qfqday"))
        .and_then(|a| a.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_TENCENT,
            message: format!("missing data.{symbol}.day/qfqday"),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for row in arr {
        let a = row.as_array().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_TENCENT,
            message: "tx index row is not an array".into(),
        })?;
        if a.len() < 6 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_TENCENT,
                message: format!("tx index row has {} fields, expected >= 6", a.len()),
            });
        }
        let get = |i: usize| a.get(i).and_then(|v| v.as_str()).and_then(parse_f64);
        out.push(ZhIndexDailyTxRow {
            date: a
                .first()
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            open: get(1),
            close: get(2),
            high: get(3),
            low: get(4),
            amount: get(5),
        });
    }
    Ok(out)
}

/// 腾讯证券-日频-指数历史数据 (前复权). `symbol` carries a market prefix
/// (e.g. `"sh000919"`); `start_date`/`end_date` use `YYYYMMDD`.
pub async fn stock_zh_index_daily_tx(
    client: &Client,
    symbol: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<ZhIndexDailyTxRow>> {
    let param = format!("{symbol},day,{start_date},{end_date},320,qfq");
    let params = [
        ("_var", "kline_dayqfq"),
        ("param", param.as_str()),
        ("r", "0.8205512681390605"),
    ];
    let text = client
        .get_text(
            SOURCE_TENCENT,
            "stock_zh_index_daily_tx",
            TENCENT_KLINE_URL,
            &params,
            None,
        )
        .await?;
    let json = strip_jsonp(&text);
    let v: Value = serde_json::from_str(json).map_err(|e| Error::Parse {
        endpoint: "stock_zh_index_daily_tx",
        message: e.to_string(),
    })?;
    parse_zh_index_tx(&v, symbol)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn read_fixture(name: &str) -> String {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        std::fs::read_to_string(p).unwrap()
    }

    fn json_fixture(name: &str) -> Value {
        serde_json::from_str(&read_fixture(name)).unwrap()
    }

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    #[test]
    fn parse_hk_index_spot_em_ok() {
        let rows = parse_hk_index_spot_em(&json_fixture("stock_hk_index_spot_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "HSTECH");
        assert_eq!(rows[0].name, "恒生科技指数");
        assert_eq!(rows[0].index, Some(1));
        assert_eq!(rows[0].inner_code, Some("100".to_string()));
        assert!(approx(rows[0].price, 5123.45));
        assert!(approx(rows[0].pct_change, -1.23));
        assert!(approx(rows[0].pre_close, 5187.0));
        // map helper: code -> inner_code, plus the hardcoded HSAHP entry
        let mut map = HashMap::new();
        for r in &rows {
            if let Some(inner) = &r.inner_code {
                map.insert(r.code.clone(), inner.clone());
            }
        }
        map.insert("HSAHP".to_string(), "100".to_string());
        assert_eq!(map.get("HSTECH"), Some(&"100".to_string()));
    }

    #[test]
    fn parse_hk_index_klines_ok() {
        let rows = parse_hk_index_klines(&json_fixture("stock_hk_index_daily_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02");
        assert!(approx(rows[0].open, 5200.0));
        assert!(approx(rows[0].latest, 5123.45));
        assert!(approx(rows[0].high, 5250.0));
        assert!(approx(rows[0].low, 5100.0));
        assert_eq!(rows[1].date, "2024-01-03");
    }

    #[test]
    fn parse_hk_index_spot_sina_ok() {
        let text = read_fixture("stock_hk_index_spot_sina.json");
        let rows = parse_hk_index_spot_sina(&text);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "hkHSI");
        assert_eq!(rows[0].name, "恒生指数");
        assert!(approx(rows[0].latest, 28938.73));
        assert!(approx(rows[0].change, -33.99));
        assert!(approx(rows[0].pct_change, -0.117));
        assert!(approx(rows[0].pre_close, 28972.72));
        assert!(approx(rows[0].open, 28938.73));
        assert!(approx(rows[0].high, 29008.39));
        assert!(approx(rows[0].low, 28691.33));
        assert_eq!(rows[1].code, "hkHSTECH");
    }

    #[test]
    fn parse_zh_index_klines_ok() {
        let rows = parse_zh_index_klines(&json_fixture("stock_zh_index_daily_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02");
        assert!(approx(rows[0].open, 3200.0));
        assert!(approx(rows[0].close, 3250.0));
        assert!(approx(rows[0].high, 3260.0));
        assert!(approx(rows[0].low, 3190.0));
        assert!(approx(rows[0].volume, 150000000.0));
        assert!(approx(rows[0].amount, 2500000000.0));
        assert_eq!(rows[1].date, "2024-01-03");
    }

    #[test]
    fn parse_zh_index_tx_ok() {
        // Shared fixture (also used by src/stock/index/extra.rs): pure JSON,
        // symbol `sh000922`, string-encoded numbers.
        let text = read_fixture("stock_zh_index_daily_tx.json");
        let v: Value = serde_json::from_str(strip_jsonp(&text)).unwrap();
        let rows = parse_zh_index_tx(&v, "sh000922").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2025-01-02");
        assert!(approx(rows[0].open, 1093.105));
        assert!(approx(rows[0].close, 1101.525));
        assert!(approx(rows[0].high, 1102.000));
        assert!(approx(rows[0].low, 1089.000));
        assert!(approx(rows[0].amount, 123456.0));
        assert_eq!(rows[1].date, "2025-01-03");
    }
}
