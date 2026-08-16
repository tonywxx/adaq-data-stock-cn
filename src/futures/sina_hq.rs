//! Sina external/foreign futures subscribe-symbol table.
//!
//! Ports `futures_hq_subscribe_exchange_symbol` ← `futures_hq_sina.py:58`.
//!
//! This helper is a **pure static mapping** (no network, no `demjson`, no JS):
//! akshare builds a `DataFrame` directly from a hardcoded `inner_dict`. We
//! return the same table as a `Vec`.
//!
//! ## DEFERRED
//! * `futures_zh_realtime` (`futures_zh_sina.py:91`) — depends on
//!   `futures_symbol_mark()`, which uses `demjson` to decode a JS document and
//!   `py_mini_racer`. Not fakeable without a JS engine / `demjson`.

/// One Sina external-futures subscribe symbol (`futures_hq_subscribe_exchange_symbol`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct HqSubscribeRow {
    pub symbol: String,
    pub code: String,
}

/// Sina external/foreign futures subscribe-symbol table.
///
/// Pure static mapping (akshare `futures_hq_sina.py:58`); no network call.
pub fn futures_hq_subscribe_exchange_symbol() -> Vec<HqSubscribeRow> {
    let inner: &[(&str, &str)] = &[
        ("新加坡铁矿石", "FEF"),
        ("马棕油", "FCPO"),
        ("日橡胶", "RSS3"),
        ("美国原糖", "RS"),
        ("CME比特币期货", "BTC"),
        ("NYBOT-棉花", "CT"),
        ("LME镍3个月", "NID"),
        ("LME铅3个月", "PBD"),
        ("LME锡3个月", "SND"),
        ("LME锌3个月", "ZSD"),
        ("LME铝3个月", "AHD"),
        ("LME铜3个月", "CAD"),
        ("CBOT-黄豆", "S"),
        ("CBOT-小麦", "W"),
        ("CBOT-玉米", "C"),
        ("CBOT-黄豆油", "BO"),
        ("CBOT-黄豆粉", "SM"),
        ("日本橡胶", "TRB"),
        ("COMEX铜", "HG"),
        ("NYMEX天然气", "NG"),
        ("NYMEX原油", "CL"),
        ("COMEX白银", "SI"),
        ("COMEX黄金", "GC"),
        ("CME-瘦肉猪", "LHC"),
        ("布伦特原油", "OIL"),
        ("伦敦金", "XAU"),
        ("伦敦银", "XAG"),
        ("伦敦铂金", "XPT"),
        ("伦敦钯金", "XPD"),
        ("欧洲碳排放", "EUA"),
    ];
    inner
        .iter()
        .map(|(symbol, code)| HqSubscribeRow {
            symbol: symbol.to_string(),
            code: code.to_string(),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// futures_foreign_commodity_realtime — 新浪外盘期货实时行情
// https://finance.sina.com.cn/money/future/hf.html
// ---------------------------------------------------------------------------

use crate::core::client::Client;
use crate::core::error::Result;

const SOURCE_SINA: &str = "sina";

/// One Sina external-futures realtime row (`futures_foreign_commodity_realtime`).
///
/// Mirrors akshare's selected columns: symbol, current_price, current_price_rmb,
/// bid, ask, high, low, time, last_settle_price, open, hold, date.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ForeignCommodityRow {
    pub symbol: String,
    pub current_price: Option<f64>,
    pub current_price_rmb: Option<f64>,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub time: Option<String>,
    pub last_settle_price: Option<f64>,
    pub open: Option<f64>,
    pub hold: Option<f64>,
    pub date: Option<String>,
}

/// Parse Sina `hq.sinajs.cn` text into `ForeignCommodityRow`s. `pub(crate)` so
/// tests can call directly.
pub(crate) fn parse_foreign_commodity(text: &str) -> Result<Vec<ForeignCommodityRow>> {
    let mut out = Vec::new();
    for line in text.split(';') {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let eq = match line.find('=') {
            Some(i) => i,
            None => continue,
        };
        let code = line[..eq].strip_prefix("var hq_str_hf_").unwrap_or(&line[..eq]);
        let valpart = line[eq + 1..]
            .trim()
            .trim_start_matches('"')
            .trim_end_matches('"')
            .trim_end_matches(';');
        if valpart.is_empty() {
            continue;
        }
        let vals: Vec<&str> = valpart.split(',').collect();
        let n = vals.len();
        let g = |i: usize| -> Option<&str> {
            vals.get(i).and_then(|s| if s.is_empty() { None } else { Some(*s) })
        };
        let num = |i: usize| -> Option<f64> { g(i).and_then(|s| s.parse::<f64>().ok()) };
        out.push(ForeignCommodityRow {
            symbol: code.to_string(),
            current_price: num(0),
            current_price_rmb: if n >= 15 { num(14) } else { None },
            bid: num(2),
            ask: num(3),
            high: num(4),
            low: num(5),
            time: g(6).map(str::to_string),
            last_settle_price: num(7),
            open: num(8),
            hold: num(9),
            date: g(12).map(str::to_string),
        });
    }
    Ok(out)
}

/// 新浪-外盘期货-行情数据 (`futures_foreign_commodity_realtime`, `futures_hq_sina.py:103`).
///
/// `symbol` is a comma-separated Sina external-futures code list (e.g. `"XAU,CL"`);
/// each is prefixed with `hf_` for the request. Requires Sina's `Referer`.
pub async fn futures_foreign_commodity_realtime(
    client: &Client,
    symbol: &str,
) -> Result<Vec<ForeignCommodityRow>> {
    let list: String = symbol
        .split(',')
        .map(|s| format!("hf_{}", s.trim()))
        .collect::<Vec<_>>()
        .join(",");
    let params = [("list", list.as_str())];
    let headers = [("Referer", "https://finance.sina.com.cn/")];
    let text = client
        .get_text(
            SOURCE_SINA,
            "futures_foreign_commodity_realtime",
            "https://hq.sinajs.cn/",
            &params,
            Some(&headers),
        )
        .await?;
    parse_foreign_commodity(&text)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subscribe_table_ok() {
        let rows = futures_hq_subscribe_exchange_symbol();
        assert_eq!(rows.len(), 30);
        assert_eq!(rows[0].symbol, "新加坡铁矿石");
        assert_eq!(rows[0].code, "FEF");
        assert_eq!(rows[24].code, "OIL");
        assert!(rows.iter().all(|r| !r.symbol.is_empty() && !r.code.is_empty()));
    }
}
