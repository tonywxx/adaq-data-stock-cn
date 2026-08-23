//! 东方财富-行情报价 (Eastmoney realtime quote / bid-ask).
//!
//! Ports `akshare/stock/stock_ask_bid_em.py:13`. Single JSON GET to
//! `push2.eastmoney.com/api/qt/stock/get`; the `data` object holds bid/ask
//! levels plus summary fields, which akshare flattens into a (item, value)
//! two-column table (item labels are a mix of English and Chinese, matching
//! akshare exactly).
//!
//! | Rust fn | akshare fn | source |
//! |---|---|---|
//! | `stock_bid_ask_em` | `stock_bid_ask_em` | eastmoney | `akshare/stock/stock_ask_bid_em.py:13` |
//!
//! ## DEFERRED
//! None.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::eastmoney_push::push2_url;
use crate::core::error::{Error, Result};

#[derive(Debug, Clone, serde::Serialize)]
pub struct BidAskRow {
    /// 指标名 (item; mixed English/Chinese labels, exactly as akshare).
    pub item: String,
    /// 指标值 (value).
    pub value: Option<f64>,
}

fn num(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => {
            let t = s.trim().replace(',', "");
            if t.is_empty() {
                None
            } else {
                t.parse().ok()
            }
        }
        _ => None,
    }
}

/// (item label, eastmoney field key, multiplier). Mirrors the akshare
/// `tick_dict` insertion order so `item`/`value` columns line up 1:1.
const SPEC: &[(&str, &str, f64)] = &[
    ("sell_5", "f31", 1.0),
    ("sell_5_vol", "f32", 100.0),
    ("sell_4", "f33", 1.0),
    ("sell_4_vol", "f34", 100.0),
    ("sell_3", "f35", 1.0),
    ("sell_3_vol", "f36", 100.0),
    ("sell_2", "f37", 1.0),
    ("sell_2_vol", "f38", 100.0),
    ("sell_1", "f39", 1.0),
    ("sell_1_vol", "f40", 100.0),
    ("buy_1", "f19", 1.0),
    ("buy_1_vol", "f20", 100.0),
    ("buy_2", "f17", 1.0),
    ("buy_2_vol", "f18", 100.0),
    ("buy_3", "f15", 1.0),
    ("buy_3_vol", "f16", 100.0),
    ("buy_4", "f13", 1.0),
    ("buy_4_vol", "f14", 100.0),
    ("buy_5", "f11", 1.0),
    ("buy_5_vol", "f12", 100.0),
    ("最新", "f43", 1.0),
    ("均价", "f71", 1.0),
    ("涨幅", "f170", 1.0),
    ("涨跌", "f169", 1.0),
    ("总手", "f47", 1.0),
    ("金额", "f48", 1.0),
    ("换手", "f168", 1.0),
    ("量比", "f50", 1.0),
    ("最高", "f44", 1.0),
    ("最低", "f45", 1.0),
    ("今开", "f46", 1.0),
    ("昨收", "f60", 1.0),
    ("涨停", "f51", 1.0),
    ("跌停", "f52", 1.0),
    ("外盘", "f49", 1.0),
    ("内盘", "f161", 1.0),
];

/// Parse `stock_bid_ask_em` rows from the already-fetched `Value` (`data` object).
pub(crate) fn parse_bid_ask(resp: &Value) -> Result<Vec<BidAskRow>> {
    let data = resp
        .get("data")
        .and_then(|d| d.as_object())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data at stock_bid_ask_em".into(),
        })?;
    let mut out = Vec::with_capacity(SPEC.len());
    for (label, key, mult) in SPEC {
        let v = num(data.get(*key)).map(|x| x * mult);
        out.push(BidAskRow {
            item: label.to_string(),
            value: v,
        });
    }
    Ok(out)
}

/// Port of `stock_bid_ask_em(symbol)` — Eastmoney realtime quote / bid-ask.
pub async fn stock_bid_ask_em(client: &Client, symbol: &str) -> Result<Vec<BidAskRow>> {
    let market = if symbol.starts_with('6') { 1 } else { 0 };
    let secid = format!("{market}.{symbol}");
    let params = [
        ("fltt", "2"),
        ("invt", "2"),
        ("fields", "f120,f121,f122,f174,f175,f59,f163,f43,f57,f58,f169,f170,f46,f44,f51,f168,f47,f164,f116,f60,f45,f52,f50,f48,f167,f117,f71,f161,f49,f530,f135,f136,f137,f138,f139,f141,f142,f144,f145,f147,f148,f140,f143,f146,f149,f55,f62,f162,f92,f173,f104,f105,f84,f85,f183,f184,f185,f186,f187,f188,f189,f190,f191,f192,f107,f111,f86,f177,f78,f110,f262,f263,f264,f267,f268,f255,f256,f257,f258,f127,f199,f128,f198,f259,f260,f261,f171,f277,f278,f279,f288,f152,f250,f251,f252,f253,f254,f269,f270,f271,f272,f273,f274,f275,f276,f265,f266,f289,f290,f286,f285,f292,f293,f294,f295"),
        ("secid", secid.as_str()),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_bid_ask_em",
            &push2_url("/api/qt/stock/get").await,
            &params,
        )
        .await?;
    parse_bid_ask(&v)
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
    fn parses_bid_ask() {
        let rows = parse_bid_ask(&fixture("stock_bid_ask_em.json")).unwrap();
        assert_eq!(rows.len(), SPEC.len());
        // 卖一 = f39 = 10.58
        let sell1 = rows.iter().find(|r| r.item == "sell_1").unwrap();
        assert!(approx(sell1.value, 10.58));
        // 卖一量 = f40 * 100 = 1100 * 100
        let sell1v = rows.iter().find(|r| r.item == "sell_1_vol").unwrap();
        assert!(approx(sell1v.value, 110000.0));
        // 最新 = f43
        let latest = rows.iter().find(|r| r.item == "最新").unwrap();
        assert!(approx(latest.value, 10.48));
        // 买一 = f19
        let buy1 = rows.iter().find(|r| r.item == "buy_1").unwrap();
        assert!(approx(buy1.value, 10.45));
    }
}
