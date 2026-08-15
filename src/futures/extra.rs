//! Extra futures / commodity endpoints ported from akshare.
//!
//! This module collects a batch of futures & commodity data endpoints that are
//! *not* already covered by the sibling modules (`daily`, `inventory`, `spot`):
//!
//! | akshare function                    | source   | ported here as              |
//! |-------------------------------------|----------|-----------------------------|
//! | `futures_zh_daily_sina`             | Sina     | [`futures_zh_daily_sina`]   |
//! | `futures_foreign_hist` (exported as `futures_foreign` in the public API) | Sina | [`futures_foreign`] |
//! | `futures_inventory_em`              | Eastmoney| [`futures_inventory_em`]    |
//! | `futures_comex_inventory`           | Eastmoney| [`futures_comex_inventory`] |
//!
//! All four are pure-HTTP (JSON or JSONP) with no JS signing / encryption, so
//! they map cleanly onto [`Client::get_json`] / [`Client::get_text`].
//!
//! ## Skipped endpoints from the requested list
//!
//! - `futures_zh_daily` (Eastmoney) — already implemented in `src/futures/daily.rs`
//!   as `futures_zh_daily` (the `futures_hist_em` Eastmoney kline).
//! - `futures_inventory` (the public-API name for `futures_inventory_em`) — already
//!   implemented in `src/futures/inventory.rs` as `futures_inventory`. We still port
//!   the Eastmoney inventory logic here as `futures_inventory_em` for completeness /
//!   parity, see note on that function.
//! - `futures_spot_price` / `futures_spot` / `commodity_futures_spot_price` — the
//!   Sina `futures_zh_spot` requires JS signing (`py_mini_racer`, `rn` token) and is
//!   not portable; the akshare `futures_spot.py` path does not exist in this akshare
//!   version, and `futures_spot_stock_em` scrapes embedded JSON out of an HTML page
//!   with `demjson`. All are skipped (JS / HTML scraping, not clean JSON APIs).
//! - `futures_spot_price` (basis, `futures_basis.py`) — scrapes HTML tables from
//!   100ppi.com via `pandas_read_html_link`; skipped (HTML scraping).
//! - `futures_roll_yield` — derived by re-fetching daily bars and computing; not a
//!   single clean endpoint; skipped.

use std::collections::HashMap;
use std::collections::HashSet;

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY, SOURCE_SINA};
use crate::core::error::{Error, Result};

/// Eastmoney datacenter "get" API (used by the inventory endpoints).
const EM_URL: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";

/// Date baked into the Sina JSONP callback / `type` partition for the domestic
/// daily kline (mirrors akshare `futures_zh_daily_sina`'s hardcoded `date`).
const SINA_DAILY_DATE: &str = "20210412";
/// Date baked into the Sina JSONP callback for the foreign daily kline.
const SINA_FOREIGN_DATE: &str = "2025_3_5";

/// Strip the JSONP wrapper `var NAME=([...]);` down to the inner JSON array text.
///
/// Sina's `jsonp.php` endpoints echo `var <cb>=(` ... `);`. We locate the first
/// `=(` and the last `);` and return the slice in between, which is a plain JSON
/// array that [`serde_json`] can parse.
fn strip_jsonp_array(text: &str) -> Result<&str> {
    let start = text.find("=(").map(|i| i + 2).ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "JSONP array wrapper `=(` not found".into(),
    })?;
    let end = text.rfind(");").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "JSONP array terminator `);` not found".into(),
    })?;
    if end <= start {
        return Err(Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "malformed JSONP response".into(),
        });
    }
    Ok(&text[start..end])
}

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

// ---------------------------------------------------------------------------
// futures_zh_daily_sina — Sina domestic futures daily K-line (JSONP)
// ---------------------------------------------------------------------------

/// One day of a Chinese futures contract's daily OHLC from Sina
/// (`futures_zh_daily_sina`, `InnerFuturesNewService.getDailyKLine`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesZhDailySinaRow {
    /// akshare column: date
    pub date: String,
    /// akshare column: open
    pub open: Option<f64>,
    /// akshare column: high
    pub high: Option<f64>,
    /// akshare column: low
    pub low: Option<f64>,
    /// akshare column: close
    pub close: Option<f64>,
    /// akshare column: volume
    pub volume: Option<f64>,
    /// akshare column: hold (open interest)
    pub hold: Option<f64>,
    /// akshare column: settle (settlement price)
    pub settle: Option<f64>,
}

/// Daily history for a Chinese futures contract from Sina (`futures_zh_daily_sina`).
///
/// `symbol` is a Sina futures symbol such as `"RB0"` (main continuous) or
/// `"RB2410"`. Returns the full daily K-line history for that symbol.
pub async fn futures_zh_daily_sina(client: &Client, symbol: &str) -> Result<Vec<FuturesZhDailySinaRow>> {
    let callback = format!("var%20_V{SINA_DAILY_DATE}");
    let url = format!(
        "https://stock2.finance.sina.com.cn/futures/api/jsonp.php/{callback}=/InnerFuturesNewService.getDailyKLine"
    );
    let type_param = format!(
        "{}_{}_{}",
        &SINA_DAILY_DATE[0..4],
        &SINA_DAILY_DATE[4..6],
        &SINA_DAILY_DATE[6..8]
    );
    let params: [(&str, &str); 2] = [("symbol", symbol), ("type", &type_param)];
    let headers: [(&str, &str); 2] = [
        ("Referer", "https://finance.sina.com.cn/"),
        ("User-Agent", "Mozilla/5.0 (compatible; adaq-data-stock-cn/0.1)"),
    ];
    let text = client
        .get_text(SOURCE_SINA, "futures_zh_daily_sina", &url, &params, Some(&headers))
        .await?;
    parse_zh_daily_sina_text(&text)
}

/// Parse a raw Sina JSONP response body into [`FuturesZhDailySinaRow`]s.
pub(crate) fn parse_zh_daily_sina_text(text: &str) -> Result<Vec<FuturesZhDailySinaRow>> {
    let arr = strip_jsonp_array(text)?;
    let value: Value = serde_json::from_str(arr).map_err(Error::Json)?;
    parse_zh_daily_sina(&value)
}

/// Parse the inner JSON array (as a [`Value`]) into [`FuturesZhDailySinaRow`]s.
pub(crate) fn parse_zh_daily_sina(resp: &Value) -> Result<Vec<FuturesZhDailySinaRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "expected a JSON array".into(),
    })?;
    Ok(arr.iter().map(parse_zh_daily_sina_item).collect())
}

fn parse_zh_daily_sina_item(item: &Value) -> FuturesZhDailySinaRow {
    FuturesZhDailySinaRow {
        date: fstr(item, "date"),
        open: fnum(item, "open"),
        high: fnum(item, "high"),
        low: fnum(item, "low"),
        close: fnum(item, "close"),
        volume: fnum(item, "volume"),
        hold: fnum(item, "hold"),
        settle: fnum(item, "settle"),
    }
}

// ---------------------------------------------------------------------------
// futures_foreign — Sina foreign / global futures daily K-line (JSONP)
// ---------------------------------------------------------------------------

/// One day of a foreign (global) futures contract's daily OHLC from Sina
/// (`futures_foreign_hist`, `GlobalFuturesService.getGlobalFuturesDailyKLine`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesForeignRow {
    /// akshare column: date
    pub date: String,
    /// akshare column: open
    pub open: Option<f64>,
    /// akshare column: high
    pub high: Option<f64>,
    /// akshare column: low
    pub low: Option<f64>,
    /// akshare column: close
    pub close: Option<f64>,
    /// akshare column: volume
    pub volume: Option<f64>,
    /// akshare column: position (open interest)
    pub open_interest: Option<f64>,
}

/// Daily history for a foreign (global) futures contract from Sina
/// (`futures_foreign_hist`, exposed as `futures_foreign` in akshare's public API).
///
/// `symbol` is a Sina global-futures code such as `"ZSD"` or `"JY"`.
pub async fn futures_foreign(client: &Client, symbol: &str) -> Result<Vec<FuturesForeignRow>> {
    let callback = format!("var%20_S{SINA_FOREIGN_DATE}");
    let url = format!(
        "https://stock2.finance.sina.com.cn/futures/api/jsonp.php/{callback}=/GlobalFuturesService.getGlobalFuturesDailyKLine"
    );
    let params: [(&str, &str); 3] = [
        ("symbol", symbol),
        ("_", SINA_FOREIGN_DATE),
        ("source", "web"),
    ];
    let headers: [(&str, &str); 2] = [
        ("Referer", "https://finance.sina.com.cn/"),
        ("User-Agent", "Mozilla/5.0 (compatible; adaq-data-stock-cn/0.1)"),
    ];
    let text = client
        .get_text(SOURCE_SINA, "futures_foreign", &url, &params, Some(&headers))
        .await?;
    parse_foreign_text(&text)
}

/// Parse a raw Sina JSONP response body into [`FuturesForeignRow`]s.
pub(crate) fn parse_foreign_text(text: &str) -> Result<Vec<FuturesForeignRow>> {
    let arr = strip_jsonp_array(text)?;
    let value: Value = serde_json::from_str(arr).map_err(Error::Json)?;
    parse_foreign(&value)
}

/// Parse the inner JSON array (as a [`Value`]) into [`FuturesForeignRow`]s.
pub(crate) fn parse_foreign(resp: &Value) -> Result<Vec<FuturesForeignRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "expected a JSON array".into(),
    })?;
    Ok(arr.iter().map(parse_foreign_item).collect())
}

fn parse_foreign_item(item: &Value) -> FuturesForeignRow {
    FuturesForeignRow {
        date: fstr(item, "date"),
        open: fnum(item, "open"),
        high: fnum(item, "high"),
        low: fnum(item, "low"),
        close: fnum(item, "close"),
        volume: fnum(item, "volume"),
        open_interest: fnum(item, "position"),
    }
}

// ---------------------------------------------------------------------------
// futures_inventory_em — Eastmoney commodity inventory (registered warrants)
// ---------------------------------------------------------------------------

/// One row of Eastmoney commodity inventory data (`futures_inventory_em`,
/// `RPT_FUTU_STOCKDATA`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesInventoryEmRow {
    /// akshare column: 日期 (TRADE_DATE)
    pub trade_date: String,
    /// akshare column: 库存 (ON_WARRANT_NUM), registered warrant quantity
    pub on_warrant_num: Option<f64>,
    /// akshare column: 增减 (ADDCHANGE), daily change
    pub add_change: Option<f64>,
}

/// Commodity inventory (registered warrants) for a futures product from Eastmoney
/// (`futures_inventory_em`).
///
/// `symbol` may be a Chinese product name (e.g. `"豆一"`) or a product code
/// (e.g. `"a"`); it is resolved against Eastmoney's `RPT_FUTU_POSITIONCODE` map.
///
/// NOTE: this mirrors the Eastmoney inventory logic already exposed as
/// `futures_inventory` in `src/futures/inventory.rs`. It is re-exposed here under
/// the explicit `futures_inventory_em` name for parity with the akshare
/// `futures_inventory_em` module; consolidate into the `inventory` module if
/// desired.
pub async fn futures_inventory_em(client: &Client, symbol: &str) -> Result<Vec<FuturesInventoryEmRow>> {
    let map_params: [(&str, &str); 7] = [
        ("reportName", "RPT_FUTU_POSITIONCODE"),
        ("columns", "TRADE_MARKET_CODE,TRADE_CODE,TRADE_TYPE"),
        ("filter", "(IS_MAINCODE=\"1\")"),
        ("pageNumber", "1"),
        ("pageSize", "500"),
        ("source", "WEB"),
        ("client", "WEB"),
    ];
    let map = client
        .get_json(SOURCE_EASTMONEY, "futures_inventory_em_map", EM_URL, &map_params)
        .await?;
    let product_id = resolve_inventory_symbol(&map, symbol)?;

    let filter = format!("(SECURITY_CODE=\"{product_id}\")(TRADE_DATE>='2020-10-28')");
    let params: [(&str, &str); 9] = [
        ("reportName", "RPT_FUTU_STOCKDATA"),
        ("columns", "SECURITY_CODE,TRADE_DATE,ON_WARRANT_NUM,ADDCHANGE"),
        ("filter", &filter),
        ("pageNumber", "1"),
        ("pageSize", "500"),
        ("sortTypes", "-1"),
        ("sortColumns", "TRADE_DATE"),
        ("source", "WEB"),
        ("client", "WEB"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "futures_inventory_em", EM_URL, &params)
        .await?;
    parse_inventory_em(&v)
}

/// Resolve a user-supplied `symbol` (Chinese name or product code) to an
/// Eastmoney `TRADE_CODE` using the `RPT_FUTU_POSITIONCODE` response.
fn resolve_inventory_symbol(map: &Value, symbol: &str) -> Result<String> {
    let data = map
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "inventory symbol map missing result.data".into(),
        })?;
    let mut type_to_code: HashMap<String, String> = HashMap::new();
    let mut codes: HashSet<String> = HashSet::new();
    for item in data {
        let t = item.get("TRADE_TYPE").and_then(|v| v.as_str()).unwrap_or_default();
        let c = item.get("TRADE_CODE").and_then(|v| v.as_str()).unwrap_or_default();
        if !t.is_empty() {
            type_to_code.insert(t.to_string(), c.to_string());
        }
        if !c.is_empty() {
            codes.insert(c.to_string());
        }
    }
    if let Some(code) = type_to_code.get(symbol) {
        return Ok(code.clone());
    }
    if codes.contains(symbol) {
        return Ok(symbol.to_string());
    }
    Err(Error::InvalidParam(format!(
        "unknown futures inventory symbol: {symbol}"
    )))
}

/// Parse the Eastmoney `RPT_FUTU_STOCKDATA` response into [`FuturesInventoryEmRow`]s.
pub(crate) fn parse_inventory_em(resp: &Value) -> Result<Vec<FuturesInventoryEmRow>> {
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
        out.push(FuturesInventoryEmRow {
            trade_date: fstr(item, "TRADE_DATE"),
            on_warrant_num: fnum(item, "ON_WARRANT_NUM"),
            add_change: fnum(item, "ADDCHANGE"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// futures_comex_inventory — Eastmoney COMEX gold/silver inventory
// ---------------------------------------------------------------------------

/// One row of Eastmoney COMEX gold/silver inventory data (`futures_comex_inventory`,
/// `RPT_FUTUOPT_GOLDSIL`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesComexInventoryRow {
    /// akshare column: 日期 (REPORT_DATE)
    pub report_date: String,
    /// COMEX inventory, tonnes (STORAGE_TON)
    pub storage_ton: Option<f64>,
    /// COMEX inventory, ounces (STORAGE_OUNCE)
    pub storage_ounce: Option<f64>,
}

/// COMEX gold/silver inventory from Eastmoney (`futures_comex_inventory`).
///
/// `symbol` is one of `{"黄金", "白银"}` (gold / silver).
pub async fn futures_comex_inventory(
    client: &Client,
    symbol: &str,
) -> Result<Vec<FuturesComexInventoryRow>> {
    let indicator = match symbol {
        "黄金" => "EMI00069026",
        "白银" => "EMI00069027",
        other => {
            return Err(Error::InvalidParam(format!(
                "unknown COMEX symbol: {other} (expected 黄金 or 白银)"
            )))
        }
    };
    let filter = format!("(INDICATOR_ID1=\"{indicator}\")(@STORAGE_TON<>\"NULL\")");
    let params: [(&str, &str); 10] = [
        ("sortColumns", "REPORT_DATE"),
        ("sortTypes", "-1"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("reportName", "RPT_FUTUOPT_GOLDSIL"),
        ("columns", "ALL"),
        ("quoteColumns", ""),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", &filter),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "futures_comex_inventory", EM_URL, &params)
        .await?;
    parse_comex_inventory(&v)
}

/// Parse the Eastmoney `RPT_FUTUOPT_GOLDSIL` response into [`FuturesComexInventoryRow`]s.
pub(crate) fn parse_comex_inventory(resp: &Value) -> Result<Vec<FuturesComexInventoryRow>> {
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
        out.push(FuturesComexInventoryRow {
            report_date: fstr(item, "REPORT_DATE"),
            storage_ton: fnum(item, "STORAGE_TON"),
            storage_ounce: fnum(item, "STORAGE_OUNCE"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Tests (offline — fixtures only)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let txt = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    #[test]
    fn parses_futures_zh_daily_sina_fixture() {
        let v = fixture("futures_zh_daily_sina.json");
        let rows = parse_zh_daily_sina(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2021-04-12");
        assert_eq!(rows[0].open, Some(5000.0));
        assert_eq!(rows[0].close, Some(5050.0));
        assert_eq!(rows[0].settle, Some(5040.0));
        assert_eq!(rows[1].date, "2021-04-13");
        assert_eq!(rows[1].high, Some(5150.0));
        assert_eq!(rows[1].hold, Some(152000.0));
    }

    #[test]
    fn parses_futures_zh_daily_sina_jsonp_text() {
        // The fixture is the inner array; wrap it the way Sina would to prove the
        // JSONP stripper works end-to-end.
        let arr = fixture("futures_zh_daily_sina.json");
        let wrapped = format!("var _V20210412=({});", serde_json::to_string(&arr).unwrap());
        let rows = parse_zh_daily_sina_text(&wrapped).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].volume, Some(120000.0));
    }

    #[test]
    fn parses_futures_foreign_fixture() {
        let v = fixture("futures_foreign.json");
        let rows = parse_foreign(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2014-01-20");
        assert_eq!(rows[0].open, Some(2082.0));
        assert_eq!(rows[0].close, Some(2076.5));
        assert_eq!(rows[0].open_interest, Some(0.0));
        assert_eq!(rows[1].date, "2014-01-21");
        assert_eq!(rows[1].volume, Some(1318.0));
    }

    #[test]
    fn parses_futures_foreign_jsonp_text() {
        let arr = fixture("futures_foreign.json");
        let wrapped = format!("var _S2025_3_5=({});", serde_json::to_string(&arr).unwrap());
        let rows = parse_foreign_text(&wrapped).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].high, Some(2092.5));
    }

    #[test]
    fn parses_futures_inventory_em_fixture() {
        let v = fixture("futures_inventory_em.json");
        let rows = parse_inventory_em(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].trade_date, "2024-01-02");
        assert_eq!(rows[0].on_warrant_num, Some(12345.0));
        assert_eq!(rows[0].add_change, Some(-100.0));
        assert_eq!(rows[1].trade_date, "2024-01-03");
        assert_eq!(rows[1].on_warrant_num, Some(12245.0));
    }

    #[test]
    fn parses_futures_comex_inventory_fixture() {
        let v = fixture("futures_comex_inventory.json");
        let rows = parse_comex_inventory(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].report_date, "2024-01-02");
        assert_eq!(rows[0].storage_ton, Some(12345.6));
        assert_eq!(rows[0].storage_ounce, Some(398765.4));
        assert_eq!(rows[1].report_date, "2024-01-03");
        assert_eq!(rows[1].storage_ton, Some(12300.1));
    }
}
