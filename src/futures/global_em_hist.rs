//! Eastmoney international/global futures history & exchange-symbol table.
//!
//! Ports two helpers from akshare's Eastmoney futures modules:
//! - `futures_global_hist_em`  ← `futures_hf_em.py:171`
//! - `futures_hist_table_em`   ← `futures_hist_em.py:77`
//!
//! Both hit Eastmoney JSON endpoints (static `ut`/`token`, no JS signing), so
//! they are source-resilient and fully portable. `futures_global_hist_em`
//! returns kline rows encoded as comma-separated strings inside the JSON
//! (`data.klines`); `futures_hist_table_em` walks a nested `redis` listing.
//!
//! ## DEFERRED
//! None in this file.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

/// Static Eastmoney kline `ut` token (from akshare `futures_global_hist_em`).
const KLINE_UT: &str = "f057cbcbce2a86e2866ab8877db1d059";
const KLINE_URL: &str = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
const REDIS_URL: &str = "https://futsse-static.eastmoney.com/redis";

/// One international futures kline row (`futures_global_hist_em`).
///
/// akshare columns: 日期, 代码, 名称, 开盘, 最新价, 最高, 最低, 总量, 涨幅,
/// 持仓, 日增.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GlobalHistRow {
    pub date: String,
    pub code: String,
    pub name: String,
    pub open: Option<f64>,
    pub price: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub volume: Option<f64>,
    pub change_pct: Option<f64>,
    pub open_interest: Option<f64>,
    pub oi_chg: Option<f64>,
}

fn to_f64_opt(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => {
            let t = s.trim().replace(',', "");
            if t.is_empty() || t == "\r" {
                None
            } else {
                t.parse::<f64>().ok()
            }
        }
        _ => None,
    }
}

/// Map an international futures symbol to its Eastmoney market code
/// (akshare `__futures_global_hist_market_code`).
fn market_code(symbol: &str) -> Option<i32> {
    let base: String = symbol.chars().take_while(|c| !c.is_ascii_digit()).collect();
    let b = base.as_str();
    let metal = ["HG", "GC", "SI", "QI", "QO", "MGC", "LTH"];
    let energy = ["CL", "NG", "RB", "HO", "PA", "PL", "QM"];
    let agri = [
        "ZW", "ZM", "ZS", "ZC", "XC", "XK", "XW", "YM", "TY", "US", "EH", "ZL", "ZR", "ZO", "FV",
        "TU", "UL", "NQ", "ES",
    ];
    let china = ["TF", "RT", "CN"];
    let soft = ["SB", "CT", "SF"];
    let lpre = ["LCPT", "LZNT", "LALT", "LTNT", "LLDT", "LNKT"];
    if metal.contains(&b) {
        return Some(101);
    }
    if energy.contains(&b) {
        return Some(102);
    }
    if agri.contains(&b) {
        return Some(103);
    }
    if china.contains(&b) {
        return Some(104);
    }
    if soft.contains(&b) {
        return Some(108);
    }
    if lpre.contains(&b) {
        return Some(109);
    }
    if b == "MPM" {
        return Some(110);
    }
    if b.starts_with('J') {
        return Some(111);
    }
    if ["M", "B", "G"].contains(&b) {
        return Some(112);
    }
    None
}

/// Parse `futures_global_hist_em` rows from an already-fetched `Value`.
pub(crate) fn parse_global_hist(resp: &Value) -> Result<Vec<GlobalHistRow>> {
    let data = resp
        .get("data")
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data".into(),
        })?;
    let klines = data.get("klines").and_then(|v| v.as_array()).ok_or_else(|| {
        Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.klines".into(),
        }
    })?;
    let code = data
        .get("code")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let name = data
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let mut out = Vec::with_capacity(klines.len());
    for line in klines {
        let s = line.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "kline not a string".into(),
        })?;
        let f: Vec<&str> = s.split(',').collect();
        if f.len() < 16 {
            continue;
        }
        let date = f[0].to_string();
        let at = |i: usize| {
            let v = Value::String(f[i].to_string());
            to_f64_opt(&v)
        };
        let mut oi_chg = at(13);
        if let Some(d) = oi_chg && d > 2_147_483_647.0 {
            oi_chg = Some(d - 4_294_967_296.0);
        }
        out.push(GlobalHistRow {
            date,
            code: code.clone(),
            name: name.clone(),
            open: at(1),
            price: at(2),
            high: at(3),
            low: at(4),
            volume: at(5),
            change_pct: at(8),
            open_interest: at(12),
            oi_chg,
        });
    }
    Ok(out)
}

/// International/global futures historical klines (`futures_global_hist_em`).
pub async fn futures_global_hist_em(client: &Client, symbol: &str) -> Result<Vec<GlobalHistRow>> {
    let mc = market_code(symbol).ok_or_else(|| Error::InvalidParam(format!("unknown symbol {symbol}")))?;
    let secid = format!("{mc}.{symbol}");
    let params = [
        ("secid", secid.as_str()),
        ("klt", "101"),
        ("fqt", "1"),
        ("lmt", "6600"),
        ("end", "20500000"),
        ("iscca", "1"),
        ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8"),
        (
            "fields2",
            "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f62,f63,f64",
        ),
        ("ut", KLINE_UT),
        ("forcect", "1"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "futures_global_hist_em", KLINE_URL, &params)
        .await?;
    parse_global_hist(&v)
}

/// One row of the Eastmoney exchange-symbol table (`futures_hist_table_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct HistTableRow {
    pub market_name: String,
    pub name: String,
    pub code: String,
}

/// Parse the combined exchange-symbol list (`Vec<Value>`) into rows.
pub(crate) fn parse_hist_table(items: &[Value]) -> Vec<HistTableRow> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(HistTableRow {
            market_name: item
                .get("mktname")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            name: item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            code: item
                .get("code")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
        });
    }
    out
}

/// Fetch the nested Eastmoney `redis` listing and return the combined
/// exchange-symbol table (`futures_hist_table_em`).
async fn fetch_exchange_symbol_raw(client: &Client) -> Result<Vec<Value>> {
    let gnweb = client
        .get_json(SOURCE_EASTMONEY, "futures_hist_table_em", REDIS_URL, &[("msgid", "gnweb")])
        .await?;
    let markets = gnweb.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "gnweb not an array".into(),
    })?;
    let mut all: Vec<Value> = Vec::new();
    for m in markets {
        let mktid = match m.get("mktid").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let inner = client
            .get_json(
                SOURCE_EASTMONEY,
                "futures_hist_table_em",
                REDIS_URL,
                &[("msgid", &mktid)],
            )
            .await?;
        let inner_arr = inner.as_array().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "inner not an array".into(),
        })?;
        for num in 1..=inner_arr.len() {
            let deeper = client
                .get_json(
                    SOURCE_EASTMONEY,
                    "futures_hist_table_em",
                    REDIS_URL,
                    &[("msgid", &format!("{mktid}_{num}"))],
                )
                .await?;
            if let Some(arr) = deeper.as_array() {
                all.extend(arr.iter().cloned());
            }
        }
    }
    Ok(all)
}

/// Eastmoney exchange-symbol comparison table (`futures_hist_table_em`).
pub async fn futures_hist_table_em(client: &Client) -> Result<Vec<HistTableRow>> {
    let items = fetch_exchange_symbol_raw(client).await?;
    Ok(parse_hist_table(&items))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(name: &str) -> Value {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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
    fn parse_global_hist_ok() {
        let rows = parse_global_hist(&fixture("futures_global_hist_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02");
        assert_eq!(rows[0].code, "HG00Y");
        assert!(approx(rows[0].open, 100.0));
        assert!(approx(rows[0].price, 101.0));
        assert!(approx(rows[0].high, 102.0));
        assert!(approx(rows[0].low, 99.0));
        assert!(approx(rows[0].volume, 12345.0));
        assert!(approx(rows[0].change_pct, 1.5));
        assert!(approx(rows[0].open_interest, 678.0));
        // 日增 signed fix: 4294967280 as u32 -> signed = -16
        assert!(approx(rows[0].oi_chg, -16.0));
    }
    #[test]
    fn market_code_ok() {
        assert_eq!(market_code("HG00Y"), Some(101));
        assert_eq!(market_code("CL00Y"), Some(102));
        assert_eq!(market_code("ES00Y"), Some(103));
        assert_eq!(market_code("GC00Y"), Some(101));
        assert_eq!(market_code("ZZ00Y"), None);
    }
    #[test]
    fn parse_hist_table_ok() {
        let rows = parse_hist_table(fixture("futures_hist_table_em.json").as_array().unwrap());
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].market_name, "上海期货交易所");
        assert_eq!(rows[0].name, "cu");
        assert_eq!(rows[0].code, "cu0000");
    }
}
