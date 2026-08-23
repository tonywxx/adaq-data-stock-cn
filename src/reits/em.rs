//! Eastmoney REITs (基础设施公募 REITs) quotes — port of `akshare/reits/reits_basic.py`.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `reits_realtime_em` | `reits/reits_basic.py:45` | 沪深 REITs 实时行情, `push2` clist |
//! | `reits_hist_em` | `reits/reits_basic.py:116` | 单只 REIT 历史 K 线, `push2his` kline |
//!
//! ## Endpoint shape (push2, not datacenter-web)
//!
//! Unlike `macro_china2` (Eastmoney `datacenter-web` with `result.data`), these
//! two functions hit the **push2 quote API**. Rows live under:
//!
//! * realtime → `data.diff` (array of field objects, `fltt=2` so numerics are
//!   JSON strings, e.g. `f2`,`f3`,`f6`).
//! * hist → `data.klines` (array of **CSV strings**, one per trading day).
//!
//! The `emg_data_array` helper from `macro_china2` (`result.data`) therefore
//! does not apply; we use `push2_diff_array` / `push2_klines` instead.
//!
//! ## `ut` token
//!
//! Both requests carry a `ut` query param. In akshare this is a **static public
//! constant** (`bd1d9ddb04089700cf9c27f6f7426281` / `f057cbcbce2a86e2866ab8877db1d059`),
//! not a per-request JS signature, so no `execjs`/token negotiation is needed —
//! these functions are fully portable.
//!
//! ## DEFERRED
//!
//! None technically required. `reits_hist_min_em` (`reits/reits_basic.py:173`,
//! push2 `trends2`) is feasible (static `ut`) but **out of the requested port
//! scope**; `__reits_code_market_map` is implemented privately to power
//! `reits_hist_em`'s `secid` lookup.

use std::collections::HashMap;

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use crate::core::json::*;

const SOURCE: &str = "eastmoney";
const PUSH2HIS_URL: &str = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
const PUSH2_UT: &str = "bd1d9ddb04089700cf9c27f6f7426281";
const PUSH2HIS_UT: &str = "f057cbcbce2a86e2866ab8877db1d059";

// ---------------------------------------------------------------------------
// Shared helpers (push2 response shape)
// ---------------------------------------------------------------------------

/// Extract `data.diff` (the realtime row array) from a push2 clist response.
fn push2_diff_array(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing data.diff".into(),
        })
}

/// Extract `data.klines` (the CSV-string array) from a push2his kline response.
fn push2_klines(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("data")
        .and_then(|d| d.get("klines"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing data.klines".into(),
        })
}

// ---------------------------------------------------------------------------
// reits_realtime_em  (akshare reits/reits_basic.py:45)
// ---------------------------------------------------------------------------

/// One REIT realtime quote row.
///
/// Mirrors akshare's output columns: 序号, 代码, 名称, 最新价, 涨跌额, 涨跌幅,
/// 成交量, 成交额, 开盘价, 最高价, 最低价, 昨收 (mapped from push2 fields
/// `f2`/`f3`/`f4`/`f5`/`f6`/`f12`/`f14`/`f15`/`f16`/`f17`/`f18`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReitsRealtimeRow {
    /// 序号 (1-based row index)
    pub seq: usize,
    /// 代码 (Eastmoney `f12`)
    pub code: String,
    /// 名称 (Eastmoney `f14`)
    pub name: String,
    /// 最新价 (Eastmoney `f2`)
    pub latest_price: Option<f64>,
    /// 涨跌额 (Eastmoney `f4`)
    pub change_amount: Option<f64>,
    /// 涨跌幅 (Eastmoney `f3`)
    pub change_pct: Option<f64>,
    /// 成交量 (Eastmoney `f5`)
    pub volume: Option<f64>,
    /// 成交额 (Eastmoney `f6`)
    pub amount: Option<f64>,
    /// 开盘价 (Eastmoney `f17`)
    pub open: Option<f64>,
    /// 最高价 (Eastmoney `f15`)
    pub high: Option<f64>,
    /// 最低价 (Eastmoney `f16`)
    pub low: Option<f64>,
    /// 昨收 (Eastmoney `f18`)
    pub prev_close: Option<f64>,
}

/// Parse `reits_realtime_em` rows from a `data.diff` array (pure, no I/O).
pub(crate) fn parse_reits_realtime(diff: &[Value]) -> Result<Vec<ReitsRealtimeRow>> {
    let mut out = Vec::with_capacity(diff.len());
    for (i, item) in diff.iter().enumerate() {
        out.push(ReitsRealtimeRow {
            seq: i + 1,
            code: opt_str(item, "f12").unwrap_or_default(),
            name: opt_str(item, "f14").unwrap_or_default(),
            latest_price: opt_f64(item, "f2"),
            change_amount: opt_f64(item, "f4"),
            change_pct: opt_f64(item, "f3"),
            volume: opt_f64(item, "f5"),
            amount: opt_f64(item, "f6"),
            open: opt_f64(item, "f17"),
            high: opt_f64(item, "f15"),
            low: opt_f64(item, "f16"),
            prev_close: opt_f64(item, "f18"),
        });
    }
    Ok(out)
}

/// 东方财富网-行情中心-REITs-沪深 REITs-实时行情 (push2 clist, akshare `reits_realtime_em`, reits_basic.py:45).
pub async fn reits_realtime_em(client: &Client) -> Result<Vec<ReitsRealtimeRow>> {
    let params: &[(&str, &str)] = &[
        ("pn", "1"),
        ("pz", "100"),
        ("po", "1"),
        ("np", "1"),
        ("ut", PUSH2_UT),
        ("fltt", "2"),
        ("invt", "2"),
        ("fid", "f3"),
        ("fs", "m:1 t:9 e:97,m:0 t:10 e:97"),
        ("fields", "f2,f3,f4,f5,f6,f12,f14,f15,f16,f17,f18"),
    ];
    let v = client
        .get_json(SOURCE, "reits_realtime_em", &crate::core::eastmoney_push::push2_url("/api/qt/clist/get").await, params)
        .await?;
    let diff = push2_diff_array(&v)?;
    parse_reits_realtime(diff)
}

// ---------------------------------------------------------------------------
// reits_hist_em  (akshare reits/reits_basic.py:116)
// ---------------------------------------------------------------------------

/// One REIT daily K-line row.
///
/// Mirrors akshare's selected columns: 日期, 今开, 最高, 最低, 最新价, 成交量,
/// 成交额, 振幅, 换手 (from push2his `klines` CSV, fields
/// `f51`/`f52`/`f53`/`f54`/`f55`/`f56`/`f57`/`f58`/`f61`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReitsHistRow {
    /// 日期 (Eastmoney `f51`)
    pub date: String,
    /// 今开 (Eastmoney `f52`)
    pub open: Option<f64>,
    /// 最高 (Eastmoney `f54`)
    pub high: Option<f64>,
    /// 最低 (Eastmoney `f55`)
    pub low: Option<f64>,
    /// 最新价 (Eastmoney `f53`)
    pub close: Option<f64>,
    /// 成交量 (Eastmoney `f56`)
    pub volume: Option<f64>,
    /// 成交额 (Eastmoney `f57`)
    pub amount: Option<f64>,
    /// 振幅 (Eastmoney `f58`)
    pub amplitude: Option<f64>,
    /// 换手 (Eastmoney `f61`)
    pub turnover: Option<f64>,
}

/// Parse `reits_hist_em` rows from a `data.klines` array of CSV strings (pure, no I/O).
pub(crate) fn parse_reits_hist(klines: &[Value]) -> Result<Vec<ReitsHistRow>> {
    let mut out = Vec::with_capacity(klines.len());
    for item in klines {
        let Some(s) = item.as_str() else {
            continue;
        };
        let p: Vec<&str> = s.split(',').collect();
        // fields2 has 14 fields; we need index 10 (换手) at minimum.
        if p.len() < 11 {
            continue;
        }
        out.push(ReitsHistRow {
            date: p[0].to_string(),
            open: p[1].parse::<f64>().ok(),
            high: p[3].parse::<f64>().ok(),
            low: p[4].parse::<f64>().ok(),
            close: p[2].parse::<f64>().ok(),
            volume: p[5].parse::<f64>().ok(),
            amount: p[6].parse::<f64>().ok(),
            amplitude: p[7].parse::<f64>().ok(),
            turnover: p[10].parse::<f64>().ok(),
        });
    }
    Ok(out)
}

/// Map a REIT code to its Eastmoney market id (1=SH, 0=SZ) via the push2 clist
/// `f12`/`f13` fields. Private helper powering `reits_hist_em`'s `secid` lookup
/// (akshare `__reits_code_market_map`).
async fn reits_code_market_map(client: &Client) -> Result<HashMap<String, String>> {
    let params: &[(&str, &str)] = &[
        ("pn", "1"),
        ("pz", "100"),
        ("po", "1"),
        ("np", "1"),
        ("ut", PUSH2_UT),
        ("fltt", "2"),
        ("invt", "2"),
        ("fid", "f3"),
        ("fs", "m:1 t:9 e:97,m:0 t:10 e:97"),
        ("fields", "f12,f13"),
    ];
    let v = client
        .get_json(SOURCE, "reits_code_market_map", &crate::core::eastmoney_push::push2_url("/api/qt/clist/get").await, params)
        .await?;
    let diff = push2_diff_array(&v)?;
    let mut map = HashMap::with_capacity(diff.len());
    for item in diff {
        let Some(code) = opt_str(item, "f12") else {
            continue;
        };
        let market = opt_f64(item, "f13").map(|m| m as i64).unwrap_or(0).to_string();
        map.insert(code, market);
    }
    Ok(map)
}

/// 东方财富网-行情中心-REITs-沪深 REITs-历史行情 (push2his kline, akshare `reits_hist_em`, reits_basic.py:116).
pub async fn reits_hist_em(client: &Client, symbol: &str) -> Result<Vec<ReitsHistRow>> {
    let map = reits_code_market_map(client).await?;
    let market = map.get(symbol).ok_or_else(|| {
        Error::InvalidParam(format!(
            "unknown reits symbol `{symbol}`; not found in eastmoney REITs list"
        ))
    })?;
    let secid = format!("{market}.{symbol}");
    let params: Vec<(&str, &str)> = vec![
        ("secid", &secid),
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
        ("ut", PUSH2HIS_UT),
        ("forcect", "1"),
    ];
    let v = client
        .get_json(SOURCE, "reits_hist_em", PUSH2HIS_URL, &params)
        .await?;
    let klines = push2_klines(&v)?;
    parse_reits_hist(klines)
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
    fn parses_reits_realtime_em() {
        let v = fixture("reits_realtime_em.json");
        let rows = parse_reits_realtime(push2_diff_array(&v).unwrap()).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].seq, 1);
        assert_eq!(rows[0].code, "508097");
        assert_eq!(rows[0].name, "中金山东高速REIT");
        assert!(approx(rows[0].latest_price, 1.234));
        assert!(approx(rows[0].change_pct, 2.34));
        assert!(approx(rows[0].amount, 15234567.89));
        assert!(approx(rows[0].prev_close, 1.206));
        assert_eq!(rows[1].seq, 2);
        assert_eq!(rows[1].code, "180501");
        assert!(approx(rows[1].change_pct, -1.23));
        assert!(approx(rows[1].high, 3.5));
    }

    #[test]
    fn parses_reits_hist_em() {
        let v = fixture("reits_hist_em.json");
        let rows = parse_reits_hist(push2_klines(&v).unwrap()).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].date, "2024-01-02");
        assert!(approx(rows[0].open, 1.05));
        assert!(approx(rows[0].close, 1.10));
        assert!(approx(rows[0].high, 1.12));
        assert!(approx(rows[0].low, 1.03));
        assert!(approx(rows[0].volume, 123456.0));
        assert!(approx(rows[0].amount, 678901.23));
        assert!(approx(rows[0].amplitude, 8.50));
        assert!(approx(rows[0].turnover, 1.20));
        assert_eq!(rows[2].date, "2024-01-04");
        assert!(approx(rows[2].turnover, 0.53));
    }
}
