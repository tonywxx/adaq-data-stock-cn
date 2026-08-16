//! FX spot quotes (akshare `fx/fx_quote.py`) — 中国外汇交易中心 (chinamoney).
//!
//! The 中国外汇交易中心 (chinamoney) market-data endpoints are **POST** with a
//! `t` (epoch-millis) form param and a fixed `User-Agent` header. They return a
//! JSON object whose `records` array holds one row per currency pair. This is
//! the upstream behind the requested `bank_fx_spot` example.
//!
//! - `fx_spot_quote` — RMB FX spot quotes (`fx_spot_quote`, `rfx-sp-quot.json`).
//! - `fx_pair_quote` — foreign-currency-pair spot quotes (`fx_pair_quote`,
//!   `cpair-quot.json`).

use serde_json::Value;

use crate::alt::{fnum, fstr};
use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_CHINAMONEY: &str = "chinamoney";
const SPOT_URL: &str = "http://www.chinamoney.com.cn/r/cms/www/chinamoney/data/fx/rfx-sp-quot.json";
const PAIR_URL: &str = "http://www.chinamoney.com.cn/r/cms/www/chinamoney/data/fx/cpair-quot.json";

const CHINAMONEY_HEADERS: &[(&str, &str)] = &[(
    "User-Agent",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
     Chrome/61.0.3163.91 Safari/537.36",
)];

/// Current epoch time in milliseconds (the `t` form param chinamoney expects).
fn now_ms() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FxQuote {
    /// Currency pair, e.g. `USD/CNY`.
    pub ccy_pair: String,
    /// Bid price.
    pub bid_prc: Option<f64>,
    /// Ask price.
    pub ask_prc: Option<f64>,
    /// Mid price.
    pub midprice: Option<f64>,
    /// Quote timestamp.
    pub time: String,
    pub source: &'static str,
}

/// RMB FX spot quotes (`fx_spot_quote`).
pub async fn fx_spot_quote(client: &Client) -> Result<Vec<FxQuote>> {
    let t = now_ms();
    let params: [(&str, &str); 1] = [("t", &t)];
    let v = client
        .post_form_json(
            SOURCE_CHINAMONEY,
            "fx_spot_quote",
            SPOT_URL,
            &params,
            Some(CHINAMONEY_HEADERS),
        )
        .await?;
    parse_fx_spot_quote(&v)
}

/// Alias for [`fx_spot_quote`] — the function named `bank_fx_spot` in the task
/// brief (the interbank RMB FX spot feed).
pub async fn bank_fx_spot(client: &Client) -> Result<Vec<FxQuote>> {
    fx_spot_quote(client).await
}

/// Foreign-currency-pair spot quotes (`fx_pair_quote`).
pub async fn fx_pair_quote(client: &Client) -> Result<Vec<FxQuote>> {
    let t = now_ms();
    let params: [(&str, &str); 1] = [("t", &t)];
    let v = client
        .post_form_json(
            SOURCE_CHINAMONEY,
            "fx_pair_quote",
            PAIR_URL,
            &params,
            Some(CHINAMONEY_HEADERS),
        )
        .await?;
    parse_fx_pair_quote(&v)
}

fn parse_records(resp: &Value) -> Result<Vec<FxQuote>> {
    let data = resp
        .get("records")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CHINAMONEY,
            message: "missing records".into(),
        })?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(pair) = fstr(item, "ccyPair") else {
            continue;
        };
        out.push(FxQuote {
            ccy_pair: pair,
            bid_prc: fnum(item, "bidPrc"),
            ask_prc: fnum(item, "askPrc"),
            midprice: fnum(item, "midprice"),
            time: fstr(item, "time").unwrap_or_default(),
            source: SOURCE_CHINAMONEY,
        });
    }
    Ok(out)
}

pub(crate) fn parse_fx_spot_quote(resp: &Value) -> Result<Vec<FxQuote>> {
    parse_records(resp)
}

pub(crate) fn parse_fx_pair_quote(resp: &Value) -> Result<Vec<FxQuote>> {
    parse_records(resp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let txt = std::fs::read_to_string(p).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    #[test]
    fn parses_fx_spot_quote() {
        let rows = parse_fx_spot_quote(&fixture("fx_spot_quote.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].ccy_pair, "USD/CNY");
        assert_eq!(rows[0].bid_prc, Some(7.21));
        assert_eq!(rows[0].ask_prc, Some(7.24));
        assert_eq!(rows[0].midprice, Some(7.225));
        assert_eq!(rows[0].source, "chinamoney");
        assert_eq!(rows[1].ccy_pair, "EUR/CNY");
    }

    #[test]
    fn parses_fx_pair_quote() {
        let rows = parse_fx_pair_quote(&fixture("fx_pair_quote.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].ccy_pair, "AUD/USD");
        assert_eq!(rows[0].bid_prc, Some(0.66));
        assert_eq!(rows[1].ccy_pair, "GBP/USD");
    }
}
