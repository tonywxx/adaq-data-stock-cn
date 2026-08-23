//! Extra index / stock-index endpoints ported from `akshare`.
//!
//! This module exposes akshare-compatible, source-specific functions for index
//! spot quotes, index daily history, index constituents and the Tencent index
//! daily series. The aggregated, normalized entry points live in
//! [`crate::stock::index`] (`spot`, `eastmoney`, `sina`); this file is the
//! akshare-named surface so callers can reach a specific upstream directly.
//!
//! All fetch calls go through [`Client`] (retry/backoff, per-source rate
//! limiting, concurrency cap). No `unwrap()` in non-test code.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY, SOURCE_TENCENT};
use crate::core::error::{Error, Result};
use crate::core::json::*;

const EM_UT: &str = "bd1d9ddb04089700cf9c27f6f7426281";
const EM_KLINE_URL: &str = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
const TX_KLINE_URL: &str = "https://proxy.finance.qq.com/ifzqgtimg/appstock/app/newfqkline/get";

/// Broad A-share index filter (Shanghai + Shenzhen + CSI) for [`index_zh_a_spot`].
const SPOT_FS: &str = "m:1 t:1,m:0 t:5,m:2";

// ---------------------------------------------------------------------------
// index_zh_a_spot — Eastmoney index spot list (akshare `index_zh_a_spot`)
// ---------------------------------------------------------------------------

/// Real-time A-share index spot list (akshare `index_zh_a_spot`, Eastmoney `clist`).
///
/// Returns the full list of Shanghai / Shenzhen / CSI indices with their latest
/// quote. Pure HTTP JSON (no JS/decryption).
pub async fn index_zh_a_spot(client: &Client) -> Result<Vec<IndexSpotRow>> {
    let params = [
        ("pn", "1"),
        ("pz", "1000"),
        ("po", "1"),
        ("np", "1"),
        ("ut", EM_UT),
        ("fltt", "2"),
        ("invt", "2"),
        ("wbp2u", "|0|0|0|web"),
        ("fid", "f12"),
        ("fs", SPOT_FS),
        (
            "fields",
            "f1,f2,f3,f4,f5,f6,f7,f10,f12,f13,f14,f15,f16,f17,f18",
        ),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "index_zh_a_spot", &crate::core::eastmoney_push::push2_url("/api/qt/clist/get").await, &params)
        .await?;
    parse_spot(&v)
}

/// One index row from [`index_zh_a_spot`] (Eastmoney `f`-field columns).
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexSpotRow {
    /// 代码 (f12) — index code, e.g. "000001"
    pub code: String,
    /// 名称 (f14) — index name, e.g. "上证指数"
    pub name: String,
    /// 最新价 (f2)
    pub price: Option<f64>,
    /// 涨跌幅 % (f3)
    pub change_percent: Option<f64>,
    /// 涨跌额 (f4)
    pub change: Option<f64>,
    /// 成交量 (f5)
    pub volume: Option<f64>,
    /// 成交额 (f6)
    pub amount: Option<f64>,
    /// 振幅 % (f7)
    pub amplitude: Option<f64>,
    /// 量比 (f10)
    pub volume_ratio: Option<f64>,
    /// 最高 (f15)
    pub high: Option<f64>,
    /// 最低 (f16)
    pub low: Option<f64>,
    /// 今开 (f17)
    pub open: Option<f64>,
    /// 昨收 (f18)
    pub prev_close: Option<f64>,
}

/// Parse an Eastmoney `clist` spot response into [`IndexSpotRow`]s.
pub(crate) fn parse_spot(resp: &Value) -> Result<Vec<IndexSpotRow>> {
    let data = resp.get("data");
    let diff = match data.and_then(|d| d.get("diff")) {
        Some(Value::Array(a)) => a,
        Some(_) => {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "data.diff is not an array".into(),
            });
        }
        None => {
            if data.is_none() {
                return Ok(Vec::new());
            }
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing data.diff".into(),
            });
        }
    };
    let mut out = Vec::with_capacity(diff.len());
    for item in diff {
        out.push(IndexSpotRow {
            code: opt_str_or(item, "f12", ""),
            name: opt_str_or(item, "f14", ""),
            price: opt_f64(item, "f2"),
            change_percent: opt_f64(item, "f3"),
            change: opt_f64(item, "f4"),
            volume: opt_f64(item, "f5"),
            amount: opt_f64(item, "f6"),
            amplitude: opt_f64(item, "f7"),
            volume_ratio: opt_f64(item, "f10"),
            high: opt_f64(item, "f15"),
            low: opt_f64(item, "f16"),
            open: opt_f64(item, "f17"),
            prev_close: opt_f64(item, "f18"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// index_zh_a_daily — Eastmoney index daily OHLCV (akshare `index_zh_a_daily`)
// ---------------------------------------------------------------------------

/// Daily index OHLCV history (akshare `index_zh_a_daily`, Eastmoney `kline`).
///
/// `symbol` uses the akshare index format, e.g. `"sh000001"`, `"sz399001"`,
/// `"csi931151"`. `adjust` is `""` (no adjust) / `"qfq"` (forward) / `"hfq"`
/// (backward). `start_date` / `end_date` are `"YYYYMMDD"` (Eastmoney `beg`/`end`).
/// Pure HTTP JSON (no JS/decryption).
pub async fn index_zh_a_daily(
    client: &Client,
    symbol: &str,
    start_date: &str,
    end_date: &str,
    adjust: &str,
) -> Result<Vec<IndexDailyRow>> {
    let secid = index_symbol_to_secid(symbol)?;
    let fqt = adjust_to_fqt(adjust)?;
    let params = [
        ("secid", secid.as_str()),
        ("fields1", "f1,f2,f3,f4,f5"),
        ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58"),
        ("klt", "101"),
        ("fqt", fqt),
        ("beg", start_date),
        ("end", end_date),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "index_zh_a_daily", EM_KLINE_URL, &params)
        .await?;
    parse_daily(&v)
}

/// One daily index bar from [`index_zh_a_daily`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexDailyRow {
    /// 日期 (f51) — trading date "YYYY-MM-DD"
    pub date: String,
    /// 开盘 (f52)
    pub open: Option<f64>,
    /// 收盘 (f53)
    pub close: Option<f64>,
    /// 最高 (f54)
    pub high: Option<f64>,
    /// 最低 (f55)
    pub low: Option<f64>,
    /// 成交量 (f56)
    pub volume: Option<f64>,
    /// 成交额 (f57)
    pub amount: Option<f64>,
}

/// Parse an Eastmoney `kline` response into [`IndexDailyRow`]s.
pub(crate) fn parse_daily(resp: &Value) -> Result<Vec<IndexDailyRow>> {
    let data = resp.get("data");
    let klines = match data.and_then(|d| d.get("klines")) {
        Some(Value::Array(a)) => a,
        Some(_) => {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "data.klines is not an array".into(),
            });
        }
        None => {
            if data.is_none() {
                return Ok(Vec::new());
            }
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing data.klines".into(),
            });
        }
    };
    let mut out = Vec::with_capacity(klines.len());
    for line in klines {
        let s = line.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "kline entry is not a string".into(),
        })?;
        let p: Vec<&str> = s.split(',').collect();
        if p.len() < 8 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: format!("index kline has {} fields, expected >= 8", p.len()),
            });
        }
        out.push(IndexDailyRow {
            date: p[0].to_string(),
            open: parse_f64_str(p[1]),
            close: parse_f64_str(p[2]),
            high: parse_f64_str(p[3]),
            low: parse_f64_str(p[4]),
            volume: parse_f64_str(p[5]),
            amount: parse_f64_str(p[6]),
        });
    }
    Ok(out)
}

/// Map an akshare index symbol (`sh/sz/bj/csi` prefix) to an Eastmoney `secid`.
fn index_symbol_to_secid(symbol: &str) -> Result<String> {
    let (market, code) = if let Some(rest) = symbol.strip_prefix("sh") {
        ("1", rest)
    } else if let Some(rest) = symbol.strip_prefix("sz") {
        ("0", rest)
    } else if let Some(rest) = symbol.strip_prefix("bj") {
        ("0", rest)
    } else if let Some(rest) = symbol.strip_prefix("csi") {
        ("2", rest)
    } else {
        return Err(Error::InvalidParam(format!(
            "unrecognized index symbol (expected sh/sz/bj/csi prefix): {symbol}"
        )));
    };
    Ok(format!("{market}.{code}"))
}

/// Map an akshare `adjust` string to Eastmoney `fqt` (0/1/2).
fn adjust_to_fqt(adjust: &str) -> Result<&'static str> {
    Ok(match adjust {
        "" | "none" | "raw" => "0",
        "qfq" => "1",
        "hfq" => "2",
        other => {
            return Err(Error::InvalidParam(format!(
                "unknown adjust (expected qfq/hfq/empty): {other}"
            )));
        }
    })
}

// ---------------------------------------------------------------------------
// stock_zh_index_daily — Tencent index daily (akshare `stock_zh_index_daily`)
// ---------------------------------------------------------------------------

/// Daily index OHLC history from Tencent (akshare `stock_zh_index_daily`).
///
/// **Note:** the original Sina `stock_zh_index_daily` decrypts its payload with an
/// in-browser JS routine (`hk_js_decode` executed via `py_mini_racer`) — that is
/// JS/encryption, so this port uses the equivalent pure-HTTP Tencent kline
/// endpoint instead. The Tencent response is JSONP-padded (`kline_dayqfq={...}`);
/// the padding is stripped before parsing.
///
/// `symbol` is the Sina-style code, e.g. `"sh000922"`. `start_date` / `end_date`
/// are `"YYYY-MM-DD"`.
pub async fn stock_zh_index_daily(
    client: &Client,
    symbol: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<IndexDailyTxRow>> {
    let param = format!("{symbol},day,{start_date},{end_date},320,qfq");
    let params = [
        ("_var", "kline_dayqfq"),
        ("param", param.as_str()),
        ("r", "0.8205512681390605"),
    ];
    let text = client
        .get_text(
            SOURCE_TENCENT,
            "stock_zh_index_daily",
            TX_KLINE_URL,
            &params,
            None,
        )
        .await?;
    // Strip the JSONP-style `kline_dayqfq=` assignment prefix.
    let json_str = text
        .split_once('=')
        .map(|(_, v)| v.trim())
        .unwrap_or(text.trim());
    let v: Value = serde_json::from_str(json_str).map_err(Error::Json)?;
    parse_tx_daily(&v, symbol)
}

/// One daily index bar from [`stock_zh_index_daily`] (Tencent `day` array).
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexDailyTxRow {
    /// 日期 — trading date "YYYY-MM-DD"
    pub date: String,
    /// 开盘
    pub open: Option<f64>,
    /// 收盘
    pub close: Option<f64>,
    /// 最高
    pub high: Option<f64>,
    /// 最低
    pub low: Option<f64>,
    /// 成交额 (akshare labels the Tencent `day` 6th field "amount")
    pub amount: Option<f64>,
}

/// Parse a Tencent kline JSON value into [`IndexDailyTxRow`]s.
pub(crate) fn parse_tx_daily(resp: &Value, symbol: &str) -> Result<Vec<IndexDailyTxRow>> {
    let data = &resp["data"];
    // Tencent returns `data: []` when there is no history.
    if data.is_array() {
        return Ok(Vec::new());
    }
    let sym_obj = data.get(symbol).ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_TENCENT,
        message: format!("missing symbol object for {symbol}"),
    })?;
    let rows = sym_obj
        .get("day")
        .or_else(|| sym_obj.get("qfqday"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_TENCENT,
            message: format!("missing data.{symbol}.day/qfqday"),
        })?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let arr = r.as_array().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_TENCENT,
            message: "day entry is not an array".into(),
        })?;
        if arr.len() < 6 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_TENCENT,
                message: format!("day row has {} fields, expected >= 6", arr.len()),
            });
        }
        out.push(IndexDailyTxRow {
            date: str_at(arr, 0),
            open: f64_at(arr, 1),
            close: f64_at(arr, 2),
            high: f64_at(arr, 3),
            low: f64_at(arr, 4),
            amount: f64_at(arr, 5),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// index_stock_cons — Eastmoney index constituents (akshare `index_stock_cons`)
// ---------------------------------------------------------------------------

/// Latest constituents of an index (akshare `index_stock_cons`).
///
/// **Note:** the current akshare `index_stock_cons` scrapes a Sina HTML page
/// (`BeautifulSoup` + `pandas.read_html`). That needs an HTML parser, which is
/// unavailable without editing `Cargo.toml`, so this port uses the equivalent
/// Eastmoney constituent query (akshare documents this function as the Eastmoney
/// source). Pure HTTP JSON.
///
/// `symbol` is the numeric index code, e.g. `"000300"`, `"399639"`.
pub async fn index_stock_cons(client: &Client, symbol: &str) -> Result<Vec<IndexConsRow>> {
    let fs = format!("b:{symbol}");
    let params = [
        ("pn", "1"),
        ("pz", "5000"),
        ("po", "1"),
        ("np", "1"),
        ("ut", EM_UT),
        ("fltt", "2"),
        ("invt", "2"),
        ("fid", "f3"),
        ("fs", fs.as_str()),
        ("fields", "f12,f13,f14"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "index_stock_cons", &crate::core::eastmoney_push::push2_url("/api/qt/clist/get").await, &params)
        .await?;
    parse_cons(&v)
}

/// One constituent row from [`index_stock_cons`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexConsRow {
    /// 品种代码 (f12) — constituent stock code, e.g. "600519"
    pub code: String,
    /// 市场 (f13) — 1 = Shanghai, 0 = Shenzhen
    pub market: Option<i64>,
    /// 品种名称 (f14) — constituent stock name
    pub name: String,
}

/// Parse an Eastmoney `clist` constituent response into [`IndexConsRow`]s.
pub(crate) fn parse_cons(resp: &Value) -> Result<Vec<IndexConsRow>> {
    let data = resp.get("data");
    let diff = match data.and_then(|d| d.get("diff")) {
        Some(Value::Array(a)) => a,
        Some(_) => {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "data.diff is not an array".into(),
            });
        }
        None => {
            if data.is_none() {
                return Ok(Vec::new());
            }
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing data.diff".into(),
            });
        }
    };
    let mut out = Vec::with_capacity(diff.len());
    for item in diff {
        out.push(IndexConsRow {
            code: opt_str_or(item, "f12", ""),
            market: item.get("f13").and_then(|v| v.as_i64()),
            name: opt_str_or(item, "f14", ""),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// offline parse tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let text = std::fs::read_to_string(path).expect("fixture missing");
        serde_json::from_str(&text).expect("fixture is not valid JSON")
    }

    #[test]
    fn test_parse_index_zh_a_spot() {
        let v = fixture("index_zh_a_spot.json");
        let rows = parse_spot(&v).expect("parse index_zh_a_spot");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "000001");
        assert_eq!(rows[0].name, "上证指数");
        assert_eq!(rows[0].price, Some(3200.50));
        assert_eq!(rows[0].change_percent, Some(1.20));
        assert_eq!(rows[0].change, Some(38.00));
        assert_eq!(rows[0].volume, Some(234567890.0));
        assert_eq!(rows[0].amount, Some(2500000000.0));
        assert_eq!(rows[0].high, Some(3210.00));
        assert_eq!(rows[0].low, Some(3180.00));
        assert_eq!(rows[0].open, Some(3195.00));
        assert_eq!(rows[0].prev_close, Some(3162.50));
        assert_eq!(rows[1].code, "399001");
        assert_eq!(rows[1].name, "深证成指");
    }

    #[test]
    fn test_parse_index_zh_a_daily() {
        let v = fixture("index_zh_a_daily.json");
        let rows = parse_daily(&v).expect("parse index_zh_a_daily");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2025-01-02");
        assert_eq!(rows[0].open, Some(3200.0));
        assert_eq!(rows[0].close, Some(3250.0));
        assert_eq!(rows[0].high, Some(3260.0));
        assert_eq!(rows[0].low, Some(3190.0));
        assert_eq!(rows[0].volume, Some(234567890.0));
        assert_eq!(rows[0].amount, Some(2500000000.0));
        assert_eq!(rows[1].date, "2025-01-03");
        assert_eq!(rows[1].close, Some(3220.0));
    }

    #[test]
    fn test_parse_stock_zh_index_daily_tx() {
        let v = fixture("stock_zh_index_daily_tx.json");
        let rows = parse_tx_daily(&v, "sh000922").expect("parse stock_zh_index_daily_tx");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2025-01-02");
        assert_eq!(rows[0].open, Some(1093.105));
        assert_eq!(rows[0].close, Some(1101.525));
        assert_eq!(rows[0].high, Some(1102.000));
        assert_eq!(rows[0].low, Some(1089.000));
        assert_eq!(rows[0].amount, Some(123456.0));
        assert_eq!(rows[1].date, "2025-01-03");
        assert_eq!(rows[1].close, Some(1095.300));
    }

    #[test]
    fn test_parse_index_stock_cons() {
        let v = fixture("index_stock_cons_em.json");
        let rows = parse_cons(&v).expect("parse index_stock_cons");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].code, "600519");
        assert_eq!(rows[0].market, Some(1));
        assert_eq!(rows[0].name, "贵州茅台");
        assert_eq!(rows[1].code, "601318");
        assert_eq!(rows[1].market, Some(1));
        assert_eq!(rows[2].code, "000001");
        assert_eq!(rows[2].market, Some(0));
        assert_eq!(rows[2].name, "平安银行");
    }

    #[test]
    fn test_index_symbol_to_secid() {
        assert_eq!(index_symbol_to_secid("sh000001").unwrap(), "1.000001");
        assert_eq!(index_symbol_to_secid("sz399001").unwrap(), "0.399001");
        assert_eq!(index_symbol_to_secid("bj899050").unwrap(), "0.899050");
        assert_eq!(index_symbol_to_secid("csi931151").unwrap(), "2.931151");
        assert!(index_symbol_to_secid("000001").is_err());
    }

    #[test]
    fn test_adjust_to_fqt() {
        assert_eq!(adjust_to_fqt("").unwrap(), "0");
        assert_eq!(adjust_to_fqt("qfq").unwrap(), "1");
        assert_eq!(adjust_to_fqt("hfq").unwrap(), "2");
        assert!(adjust_to_fqt("xyz").is_err());
    }
}
