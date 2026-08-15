use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// Jin10 (金十数据) data center — public `X-App-Id`, no signing/secret required.
const SOURCE_JIN10: &str = "jin10";

const SPOT_URL: &str = "https://datacenter-api.jin10.com/crypto_currency/list";

/// Real-time spot quotes for major cryptocurrencies (akshare `crypto_js_spot`).
///
/// One row per (exchange, pair) snapshot. Mirrors akshare's `data_df` columns
/// after the Chinese rename; upstream JSON keys are kept (incl. the upstream
/// `hightest_price` typo, mapped to `high_24h`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CryptoSpot {
    pub market: String,
    pub symbol: String,
    pub last_price: Option<f64>,
    pub change: Option<f64>,
    pub change_pct: Option<f64>,
    pub high_24h: Option<f64>,
    pub low_24h: Option<f64>,
    pub volume_24h: Option<f64>,
    pub updated_at: String,
    pub source: &'static str,
}

/// Fetch real-time crypto spot quotes from Jin10 (`crypto_js_spot`).
pub async fn crypto_js_spot(client: &Client) -> Result<Vec<CryptoSpot>> {
    let headers = [
        (
            "user-agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36",
        ),
        ("x-app-id", "rU6QIu7JHe2gOUeR"),
        ("x-csrf-token", "x-csrf-token"),
        ("x-version", "1.0.0"),
    ];
    let text = client
        .get_text(SOURCE_JIN10, "crypto_js_spot", SPOT_URL, &[], Some(&headers))
        .await?;
    let v: Value = serde_json::from_str(&text).map_err(Error::Json)?;
    parse(&v)
}

/// Parse a `crypto_js_spot` response. `data` is a JSON array of objects; rows
/// missing a `currency_pair` are skipped (the batch is not failed).
pub(crate) fn parse(resp: &Value) -> Result<Vec<CryptoSpot>> {
    let arr = resp
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_JIN10,
            message: "missing data array".into(),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let symbol = match item.get("currency_pair").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => continue,
        };
        let market = str_or(item, "bourse").unwrap_or_default();
        let last_price = item.get("price").and_then(num);
        let change = item.get("up_down").and_then(num);
        let change_pct = item.get("up_down_rate").and_then(num);
        let high_24h = item.get("hightest_price").and_then(num); // upstream typo
        let low_24h = item.get("lowest_price").and_then(num);
        let volume_24h = item.get("volume").and_then(num);
        let updated_at = str_or(item, "reported_at").unwrap_or_default();
        out.push(CryptoSpot {
            market,
            symbol,
            last_price,
            change,
            change_pct,
            high_24h,
            low_24h,
            volume_24h,
            updated_at,
            source: SOURCE_JIN10,
        });
    }
    Ok(out)
}

fn str_or(item: &Value, k: &str) -> Option<String> {
    item.get(k)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_js_spot_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/crypto_js_spot.json");
        let v: Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let rows = parse(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].market, "Bitfinex(香港)");
        assert_eq!(rows[0].symbol, "LTCUSD");
        assert_eq!(rows[0].last_price, Some(67.465));
        assert_eq!(rows[0].change, Some(0.59));
        assert_eq!(rows[0].change_pct, Some(0.87));
        assert_eq!(rows[0].high_24h, Some(68.867));
        assert_eq!(rows[0].volume_24h, Some(6893.13));
        assert_eq!(rows[0].updated_at, "2023-10-02 22:45:09");
        assert_eq!(rows[0].source, "jin10");
        assert_eq!(rows[1].symbol, "BTCUSD");
        assert_eq!(rows[1].last_price, Some(28309.0));
        assert_eq!(rows[1].change_pct, Some(4.40));
    }
}
