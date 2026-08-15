//! Extra crypto market data (akshare `crypto` package, `crypto_em` block).
//!
//! Ports akshare's Binance/OKX-backed crypto functions. The local akshare
//! checkout (release-v1.18.91) no longer ships `crypto/crypto_em.py`, so these
//! are reconstructed against the public REST endpoints those akshare functions
//! call (Binance `/api/v3/*` and OKX `/api/v5/market/*`) — both keyless, pure
//! HTTP, and therefore easy to fixture.
//!
//! | akshare fn             | Rust fn                | source            |
//! |------------------------|------------------------|-------------------|
//! | `crypto_hist`          | [`crypto_hist`]        | binance / okx     |
//! | `crypto_spot`          | [`crypto_spot`]        | binance / okx     |
//! | `crypto_info`          | [`crypto_info`]        | binance           |
//! | `crypto_name_map`      | [`crypto_name_map`]    | binance           |
//!
//! Skipped (see module-level note):
//! - `crypto_js_spot` — already ported in [`crate::crypto::js_spot`]; it is the
//!   Jin10 (金十) spot quote, not a Binance/OKX endpoint.
//! - `crypto_kline` — no corresponding implementation exists in this akshare
//!   checkout (`crypto_em.py` was removed in v1.18.91); `crypto_hist` covers the
//!   kline use case. Revisit once an akshare `crypto_em`-style source is pinned.

use chrono::{DateTime, NaiveDate, Utc};
use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// Binance public REST (keyless).
const SOURCE_BINANCE: &str = "binance";
/// OKX public REST (keyless).
const SOURCE_OKX: &str = "okx";

const BINANCE_KLINES: &str = "https://api.binance.com/api/v3/klines";
const BINANCE_TICKER: &str = "https://api.binance.com/api/v3/ticker/24hr";
const BINANCE_EXCHANGE_INFO: &str = "https://api.binance.com/api/v3/exchangeInfo";
const OKX_CANDLES: &str = "https://www.okx.com/api/v5/market/candles";
const OKX_TICKER: &str = "https://www.okx.com/api/v5/market/ticker";

/// Which exchange backend to query. Mirrors akshare's `market` argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Market {
    Binance,
    Okx,
}

impl Market {
    /// Normalize a user-supplied symbol (`BTC_USDT`) to the backend's wire form
    /// (Binance: `BTCUSDT`, no separator; OKX: `BTC-USDT`, dash separator).
    fn normalize_symbol(self, symbol: &str) -> String {
        match self {
            Market::Binance => symbol.replace('_', "").to_uppercase(),
            Market::Okx => symbol.replace('_', "-").to_uppercase(),
        }
    }
}

// ---------------------------------------------------------------------------
// crypto_hist — historical klines (OHLCV)
// ---------------------------------------------------------------------------

/// One OHLCV candle (akshare `crypto_hist` row).
///
/// akshare columns: `日期, 开盘, 最高, 最低, 收盘, 成交量` (plus `成交额`,
/// `成交笔数` carried through from the raw kline array).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CryptoHistRow {
    /// Candle open time, formatted `YYYY-MM-DD HH:MM:SS` (UTC). akshare `日期`.
    pub date: String,
    /// Open price. akshare `开盘`.
    pub open: f64,
    /// High price. akshare `最高`.
    pub high: f64,
    /// Low price. akshare `最低`.
    pub low: f64,
    /// Close price. akshare `收盘`.
    pub close: f64,
    /// Base-asset volume. akshare `成交量`.
    pub volume: f64,
    /// Quote-asset turnover. akshare `成交额`.
    pub quote_volume: Option<f64>,
    /// Number of trades in the candle. akshare `成交笔数`.
    pub trades: Option<u64>,
    /// Backend that produced the row.
    pub source: &'static str,
}

/// Historical OHLCV candles for one symbol.
///
/// `interval` is passed through to the backend (Binance/OKX bar size, e.g.
/// `"1d"`, `"1h"`, `"5m"`). `start_date`/`end_date` are optional `YYYYMMDD`
/// (or `YYYY-MM-DD`) bounds; `limit` caps returned rows (Binance default 500).
pub async fn crypto_hist(
    client: &Client,
    market: Market,
    symbol: &str,
    interval: &str,
    start_date: Option<&str>,
    end_date: Option<&str>,
    limit: Option<u32>,
) -> Result<Vec<CryptoHistRow>> {
    let sym = market.normalize_symbol(symbol);
    match market {
        Market::Binance => {
            let mut params: Vec<(&str, &str)> = Vec::with_capacity(5);
            let sym_ref = sym.as_str();
            params.push(("symbol", sym_ref));
            params.push(("interval", interval));
            let start_s = start_date
                .map(|s| date_to_ms(s).map(|m| m.to_string()))
                .transpose()?;
            if let Some(ref s) = start_s {
                params.push(("startTime", s));
            }
            let end_s = end_date
                .map(|e| date_to_ms(e).map(|m| m.to_string()))
                .transpose()?;
            if let Some(ref s) = end_s {
                params.push(("endTime", s));
            }
            let lim = limit.unwrap_or(500).to_string();
            params.push(("limit", &lim));
            let v = client
                .get_json(SOURCE_BINANCE, "crypto_hist", BINANCE_KLINES, &params)
                .await?;
            parse_hist(&v)
        }
        Market::Okx => {
            let mut params: Vec<(&str, &str)> = Vec::with_capacity(4);
            let sym_ref = sym.as_str();
            params.push(("instId", sym_ref));
            params.push(("bar", interval));
            let after_s = start_date
                .map(|s| date_to_ms(s).map(|m| m.to_string()))
                .transpose()?;
            if let Some(ref s) = after_s {
                params.push(("after", s));
            }
            let before_s = end_date
                .map(|e| date_to_ms(e).map(|m| m.to_string()))
                .transpose()?;
            if let Some(ref s) = before_s {
                params.push(("before", s));
            }
            let lim = limit.map(|l| l.to_string());
            if let Some(ref s) = lim {
                params.push(("limit", s));
            }
            let v = client
                .get_json(SOURCE_OKX, "crypto_hist", OKX_CANDLES, &params)
                .await?;
            parse_hist_okx(&v)
        }
    }
}

/// Parse a Binance `/api/v3/klines` response (a JSON array of 12-element rows).
pub(crate) fn parse_hist(resp: &Value) -> Result<Vec<CryptoHistRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_BINANCE,
        message: "expected a JSON array of klines".into(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for row in arr {
        let cells = match row.as_array() {
            Some(c) => c,
            None => continue,
        };
        let open_time = cells.first().and_then(ms_val);
        let date = open_time
            .and_then(DateTime::<Utc>::from_timestamp_millis)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default();
        let open = match cells.get(1).and_then(num_val) {
            Some(v) => v,
            None => continue,
        };
        let high = match cells.get(2).and_then(num_val) {
            Some(v) => v,
            None => continue,
        };
        let low = match cells.get(3).and_then(num_val) {
            Some(v) => v,
            None => continue,
        };
        let close = match cells.get(4).and_then(num_val) {
            Some(v) => v,
            None => continue,
        };
        let volume = match cells.get(5).and_then(num_val) {
            Some(v) => v,
            None => continue,
        };
        out.push(CryptoHistRow {
            date,
            open,
            high,
            low,
            close,
            volume,
            quote_volume: cells.get(7).and_then(num_val),
            trades: cells.get(8).and_then(|v| match v {
                Value::Number(n) => n.as_u64(),
                Value::String(s) => s.parse().ok(),
                _ => None,
            }),
            source: SOURCE_BINANCE,
        });
    }
    Ok(out)
}

/// Parse an OKX `/api/v5/market/candles` response (`{code,data:[[...]]}`).
/// OKX returns newest-first; we reverse to oldest-first to match Binance.
pub(crate) fn parse_hist_okx(resp: &Value) -> Result<Vec<CryptoHistRow>> {
    let data = resp
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_OKX,
            message: "missing data".into(),
        })?;
    let mut out = Vec::with_capacity(data.len());
    for row in data {
        let cells = match row.as_array() {
            Some(c) => c,
            None => continue,
        };
        let open_time = cells.first().and_then(ms_val);
        let date = open_time
            .and_then(DateTime::<Utc>::from_timestamp_millis)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default();
        let open = match cells.get(1).and_then(num_val) {
            Some(v) => v,
            None => continue,
        };
        let high = match cells.get(2).and_then(num_val) {
            Some(v) => v,
            None => continue,
        };
        let low = match cells.get(3).and_then(num_val) {
            Some(v) => v,
            None => continue,
        };
        let close = match cells.get(4).and_then(num_val) {
            Some(v) => v,
            None => continue,
        };
        let volume = match cells.get(5).and_then(num_val) {
            Some(v) => v,
            None => continue,
        };
        out.push(CryptoHistRow {
            date,
            open,
            high,
            low,
            close,
            volume,
            quote_volume: cells.get(6).and_then(num_val),
            trades: None,
            source: SOURCE_OKX,
        });
    }
    out.reverse();
    Ok(out)
}

// ---------------------------------------------------------------------------
// crypto_spot — latest 24h ticker
// ---------------------------------------------------------------------------

/// One 24h spot ticker quote (akshare `crypto_spot` row).
///
/// akshare columns: `symbol, 价格, 涨跌额, 涨跌幅, 最高, 最低, 成交量, 成交额,
/// 开盘价, 昨日收盘, 加权平均价`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CryptoSpotRow {
    /// Trading pair, e.g. `BTCUSDT`. akshare `symbol`.
    pub symbol: String,
    /// Last price. akshare `价格`.
    pub price: f64,
    /// Absolute 24h change. akshare `涨跌额`.
    pub price_change: Option<f64>,
    /// 24h change percent. akshare `涨跌幅`.
    pub price_change_percent: Option<f64>,
    /// 24h weighted-average price. akshare `加权平均价`.
    pub weighted_avg_price: Option<f64>,
    /// Previous close. akshare `昨日收盘`.
    pub prev_close_price: Option<f64>,
    /// 24h open. akshare `开盘价`.
    pub open_price: Option<f64>,
    /// 24h high. akshare `最高`.
    pub high_price: Option<f64>,
    /// 24h low. akshare `最低`.
    pub low_price: Option<f64>,
    /// Base-asset 24h volume. akshare `成交量`.
    pub volume: Option<f64>,
    /// Quote-asset 24h turnover. akshare `成交额`.
    pub quote_volume: Option<f64>,
    /// Backend that produced the row.
    pub source: &'static str,
}

/// Latest 24h ticker for one symbol.
pub async fn crypto_spot(
    client: &Client,
    market: Market,
    symbol: &str,
) -> Result<Vec<CryptoSpotRow>> {
    let sym = market.normalize_symbol(symbol);
    match market {
        Market::Binance => {
            let params = [("symbol", sym.as_str())];
            let v = client
                .get_json(SOURCE_BINANCE, "crypto_spot", BINANCE_TICKER, &params)
                .await?;
            parse_spot(&v)
        }
        Market::Okx => {
            let params = [("instId", sym.as_str())];
            let v = client
                .get_json(SOURCE_OKX, "crypto_spot", OKX_TICKER, &params)
                .await?;
            parse_spot_okx(&v)
        }
    }
}

/// Parse a Binance `/api/v3/ticker/24hr` response (a single JSON object).
pub(crate) fn parse_spot(resp: &Value) -> Result<Vec<CryptoSpotRow>> {
    let symbol = str_opt(resp, "symbol");
    if symbol.is_empty() {
        return Err(Error::UpstreamChanged {
            origin: SOURCE_BINANCE,
            message: "missing symbol".into(),
        });
    }
    let price = match num_opt(resp, "lastPrice") {
        Some(v) => v,
        None => {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_BINANCE,
                message: "missing lastPrice".into(),
            })
        }
    };
    Ok(vec![CryptoSpotRow {
        symbol,
        price,
        price_change: num_opt(resp, "priceChange"),
        price_change_percent: num_opt(resp, "priceChangePercent"),
        weighted_avg_price: num_opt(resp, "weightedAvgPrice"),
        prev_close_price: num_opt(resp, "prevClosePrice"),
        open_price: num_opt(resp, "openPrice"),
        high_price: num_opt(resp, "highPrice"),
        low_price: num_opt(resp, "lowPrice"),
        volume: num_opt(resp, "volume"),
        quote_volume: num_opt(resp, "quoteVolume"),
        source: SOURCE_BINANCE,
    }])
}

/// Parse an OKX `/api/v5/market/ticker` response (`{code,data:[{...}]}`).
pub(crate) fn parse_spot_okx(resp: &Value) -> Result<Vec<CryptoSpotRow>> {
    let data = resp
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_OKX,
            message: "missing data".into(),
        })?;
    let item = data.first().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_OKX,
        message: "empty ticker data".into(),
    })?;
    let symbol = str_opt(item, "instId");
    if symbol.is_empty() {
        return Err(Error::UpstreamChanged {
            origin: SOURCE_OKX,
            message: "missing instId".into(),
        });
    }
    let price = match num_opt(item, "last") {
        Some(v) => v,
        None => {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_OKX,
                message: "missing last".into(),
            })
        }
    };
    Ok(vec![CryptoSpotRow {
        symbol,
        price,
        price_change: None,
        price_change_percent: num_opt(item, "changeRatePct"),
        weighted_avg_price: None,
        prev_close_price: num_opt(item, "open24h"),
        open_price: num_opt(item, "open24h"),
        high_price: num_opt(item, "high24h"),
        low_price: num_opt(item, "low24h"),
        volume: num_opt(item, "vol24h"),
        quote_volume: num_opt(item, "volCcy24h"),
        source: SOURCE_OKX,
    }])
}

// ---------------------------------------------------------------------------
// crypto_info — per-symbol exchange metadata
// ---------------------------------------------------------------------------

/// Per-symbol exchange metadata (akshare `crypto_info` row).
///
/// akshare columns: `symbol, 标的, 名称, 价格货币, 基础资产精度, 价格精度,
/// 是否允许交易` (status / min-notional carried through from `filters`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CryptoInfoRow {
    /// Pair, e.g. `BTCUSDT`. akshare `symbol`.
    pub symbol: String,
    /// Trading status (`TRADING`, ...). akshare `是否允许交易` (raw status).
    pub status: String,
    /// Base asset. akshare `标的`.
    pub base_asset: String,
    /// Quote asset. akshare `名称` / `价格货币`.
    pub quote_asset: String,
    /// Base-asset precision. akshare `基础资产精度`.
    pub base_asset_precision: Option<u32>,
    /// Quote-asset precision. akshare `价格精度`.
    pub quote_asset_precision: Option<u32>,
    /// Whether spot trading is allowed. akshare `是否允许交易`.
    pub is_spot_trading_allowed: bool,
    /// Minimum order value (NOTIONAL filter). akshare `最小下单额`.
    pub min_notional: Option<f64>,
    /// Minimum order quantity (LOT_SIZE filter). akshare `最小下单量`.
    pub min_qty: Option<f64>,
    /// Backend that produced the row.
    pub source: &'static str,
}

/// Exchange metadata for one Binance symbol (only Binance exposes this via
/// `exchangeInfo`; OKX has no direct akshare `crypto_info` equivalent).
pub async fn crypto_info(client: &Client, symbol: &str) -> Result<Vec<CryptoInfoRow>> {
    let sym = symbol.replace('_', "").to_uppercase();
    let params = [("symbol", sym.as_str())];
    let v = client
        .get_json(
            SOURCE_BINANCE,
            "crypto_info",
            BINANCE_EXCHANGE_INFO,
            &params,
        )
        .await?;
    parse_info(&v)
}

/// Parse a Binance `/api/v3/exchangeInfo?symbol=...` response.
pub(crate) fn parse_info(resp: &Value) -> Result<Vec<CryptoInfoRow>> {
    let symbols = resp
        .get("symbols")
        .and_then(|s| s.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_BINANCE,
            message: "missing symbols".into(),
        })?;
    let mut out = Vec::with_capacity(symbols.len());
    for s in symbols {
        let symbol = str_opt(s, "symbol");
        if symbol.is_empty() {
            continue;
        }
        let (min_qty, min_notional) = parse_filters(s);
        out.push(CryptoInfoRow {
            symbol,
            status: str_opt(s, "status"),
            base_asset: str_opt(s, "baseAsset"),
            quote_asset: str_opt(s, "quoteAsset"),
            base_asset_precision: u32_opt(s, "baseAssetPrecision"),
            quote_asset_precision: u32_opt(s, "quoteAssetPrecision"),
            is_spot_trading_allowed: s
                .get("isSpotTradingAllowed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            min_notional,
            min_qty,
            source: SOURCE_BINANCE,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// crypto_name_map — tradable symbol listing ("symbol map")
// ---------------------------------------------------------------------------

/// One tradable pair in the symbol map (akshare `crypto_name_map` row).
///
/// akshare's upstream symbol map (Chinese-name → code) is unavailable in this
/// akshare checkout, so this is anchored to Binance `exchangeInfo`: it lists
/// every tradable pair with its base/quote assets — the wire symbols you pass
/// to [`crypto_hist`] / [`crypto_spot`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct CryptoNameMapRow {
    /// Pair, e.g. `BTCUSDT`.
    pub symbol: String,
    /// Base asset, e.g. `BTC`.
    pub base_asset: String,
    /// Quote asset, e.g. `USDT`.
    pub quote_asset: String,
    /// Trading status.
    pub status: String,
    /// Whether spot trading is allowed on this pair.
    pub is_spot_trading_allowed: bool,
    /// Backend that produced the row.
    pub source: &'static str,
}

/// List every Binance tradable pair (the "symbol map").
pub async fn crypto_name_map(client: &Client) -> Result<Vec<CryptoNameMapRow>> {
    let v = client
        .get_json(
            SOURCE_BINANCE,
            "crypto_name_map",
            BINANCE_EXCHANGE_INFO,
            &[],
        )
        .await?;
    parse_name_map(&v)
}

/// Parse a Binance `/api/v3/exchangeInfo` response into a symbol map.
pub(crate) fn parse_name_map(resp: &Value) -> Result<Vec<CryptoNameMapRow>> {
    let symbols = resp
        .get("symbols")
        .and_then(|s| s.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_BINANCE,
            message: "missing symbols".into(),
        })?;
    let mut out = Vec::with_capacity(symbols.len());
    for s in symbols {
        let symbol = str_opt(s, "symbol");
        if symbol.is_empty() {
            continue;
        }
        out.push(CryptoNameMapRow {
            symbol,
            base_asset: str_opt(s, "baseAsset"),
            quote_asset: str_opt(s, "quoteAsset"),
            status: str_opt(s, "status"),
            is_spot_trading_allowed: s
                .get("isSpotTradingAllowed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            source: SOURCE_BINANCE,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

/// Parse `YYYYMMDD` (or `YYYY-MM-DD`) to epoch millis.
fn date_to_ms(s: &str) -> Result<i64> {
    let s = s.trim();
    let nd = NaiveDate::parse_from_str(s, "%Y%m%d")
        .or_else(|_| NaiveDate::parse_from_str(s, "%Y-%m-%d"))
        .map_err(|_| Error::InvalidParam(format!("bad date (want YYYYMMDD): {s}")))?;
    let nt = nd
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| Error::InvalidParam(format!("bad date: {s}")))?;
    Ok(nt.and_utc().timestamp_millis())
}

fn str_opt(v: &Value, k: &str) -> String {
    v.get(k)
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string()
}

fn num_opt(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(num_val)
}

fn u32_opt(v: &Value, k: &str) -> Option<u32> {
    match v.get(k) {
        Some(Value::Number(n)) => n.as_u64().map(|x| x as u32),
        Some(Value::String(s)) => s.parse::<u32>().ok(),
        _ => None,
    }
}

/// Numeric coercion for Binance/OKX fields that may be strings or numbers.
fn num_val(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

/// Integer coercion (epoch millis) for Binance/OKX time fields.
fn ms_val(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.parse::<i64>().ok(),
        _ => None,
    }
}

/// Pull `minQty` (LOT_SIZE) and `minNotional` (NOTIONAL) out of a Binance
/// `filters` array. Missing filters yield `None`.
fn parse_filters(symbol: &Value) -> (Option<f64>, Option<f64>) {
    let filters = match symbol.get("filters").and_then(|f| f.as_array()) {
        Some(f) => f,
        None => return (None, None),
    };
    let mut min_qty = None;
    let mut min_notional = None;
    for f in filters {
        let kind = f.get("filterType").and_then(|x| x.as_str()).unwrap_or("");
        match kind {
            "LOT_SIZE" => min_qty = f.get("minQty").and_then(num_val),
            "NOTIONAL" => min_notional = f.get("minNotional").and_then(num_val),
            _ => {}
        }
    }
    (min_qty, min_notional)
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

    #[test]
    fn parses_hist_binance_fixture() {
        let v = fixture("crypto_hist_binance.json");
        let rows = parse_hist(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-01 00:00:00");
        assert_eq!(rows[0].open, 42000.0);
        assert_eq!(rows[0].high, 42500.0);
        assert_eq!(rows[0].low, 41800.0);
        assert_eq!(rows[0].close, 42300.0);
        assert_eq!(rows[0].volume, 100.5);
        assert_eq!(rows[0].quote_volume, Some(4_240_000.0));
        assert_eq!(rows[0].trades, Some(1500));
        assert_eq!(rows[0].source, "binance");
        assert_eq!(rows[1].date, "2024-01-02 00:00:00");
        assert_eq!(rows[1].close, 42800.0);
    }

    #[test]
    fn parses_hist_okx_fixture() {
        let v = fixture("crypto_hist_okx.json");
        let rows = parse_hist_okx(&v).unwrap();
        // OKX returns newest-first; we reverse to oldest-first.
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-01 00:00:00");
        assert_eq!(rows[0].open, 42000.0);
        assert_eq!(rows[0].close, 42300.0);
        assert_eq!(rows[0].volume, 100.5);
        assert_eq!(rows[0].quote_volume, Some(4_240_000.0));
        assert_eq!(rows[0].source, "okx");
        assert_eq!(rows[1].date, "2024-01-02 00:00:00");
    }

    #[test]
    fn parses_spot_binance_fixture() {
        let v = fixture("crypto_spot_binance.json");
        let rows = parse_spot(&v).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "BTCUSDT");
        assert_eq!(rows[0].price, 42300.0);
        assert_eq!(rows[0].price_change, Some(-123.45));
        assert_eq!(rows[0].price_change_percent, Some(-0.29));
        assert_eq!(rows[0].high_price, Some(43000.0));
        assert_eq!(rows[0].low_price, Some(41800.0));
        assert_eq!(rows[0].volume, Some(1000.0));
        assert_eq!(rows[0].quote_volume, Some(42_100_000.0));
        assert_eq!(rows[0].source, "binance");
    }

    #[test]
    fn parses_spot_okx_fixture() {
        let v = fixture("crypto_spot_okx.json");
        let rows = parse_spot_okx(&v).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "BTC-USDT");
        assert_eq!(rows[0].price, 42300.0);
        assert_eq!(rows[0].high_price, Some(43000.0));
        assert_eq!(rows[0].low_price, Some(41800.0));
        assert_eq!(rows[0].volume, Some(1000.0));
        assert_eq!(rows[0].quote_volume, Some(42_100_000.0));
        assert_eq!(rows[0].source, "okx");
    }

    #[test]
    fn parses_info_fixture() {
        let v = fixture("crypto_info.json");
        let rows = parse_info(&v).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "BTCUSDT");
        assert_eq!(rows[0].status, "TRADING");
        assert_eq!(rows[0].base_asset, "BTC");
        assert_eq!(rows[0].quote_asset, "USDT");
        assert_eq!(rows[0].base_asset_precision, Some(8));
        assert_eq!(rows[0].quote_asset_precision, Some(8));
        assert!(rows[0].is_spot_trading_allowed);
        assert_eq!(rows[0].min_qty, Some(0.00001));
        assert_eq!(rows[0].min_notional, Some(10.0));
        assert_eq!(rows[0].source, "binance");
    }

    #[test]
    fn parses_name_map_fixture() {
        let v = fixture("crypto_name_map.json");
        let rows = parse_name_map(&v).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].symbol, "BTCUSDT");
        assert_eq!(rows[0].base_asset, "BTC");
        assert_eq!(rows[0].quote_asset, "USDT");
        assert!(rows[0].is_spot_trading_allowed);
        // last pair has spot trading disabled and is kept (only empty symbols skip)
        assert_eq!(rows[2].symbol, "ETHBTC");
        assert!(!rows[2].is_spot_trading_allowed);
    }

    #[test]
    fn date_to_ms_roundtrip() {
        assert_eq!(date_to_ms("20240101").unwrap(), 1_704_067_200_000);
        assert_eq!(date_to_ms("2024-01-01").unwrap(), 1_704_067_200_000);
        assert!(date_to_ms("nonsense").is_err());
    }
}
