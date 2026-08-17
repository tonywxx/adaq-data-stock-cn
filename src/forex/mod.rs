//! Foreign-exchange market data (akshare `forex` package).

use crate::core::client::Client;
use crate::core::error::Result;

pub mod eastmoney;
pub mod extra;

/// Canonical real-time FX spot quote (normalized, source-agnostic).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ForexSpotQuote {
    pub code: String,
    pub name: String,
    pub price: Option<f64>,
    pub change: Option<f64>,
    pub pct_change: Option<f64>,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub pre_close: Option<f64>,
    pub source: &'static str,
}

/// Canonical historical FX kline row.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ForexHistRow {
    pub symbol: String,
    pub date: String,
    pub open: Option<f64>,
    pub close: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub amplitude: Option<f64>,
    pub source: &'static str,
}

/// All FX spot quotes (Eastmoney `forex_spot_em`).
pub async fn spot(client: &Client) -> Result<Vec<ForexSpotQuote>> {
    eastmoney::spot(client).await
}

/// Historical FX kline for a symbol (Eastmoney `forex_hist_em`).
pub async fn hist(client: &Client, symbol: &str) -> Result<Vec<ForexHistRow>> {
    eastmoney::hist(client, symbol).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::convert;

    #[test]
    fn forex_spot_quote_serializes() {
        let quotes = vec![ForexSpotQuote {
            code: "USD".into(),
            name: "美元".into(),
            price: Some(7.1),
            change: Some(0.01),
            pct_change: Some(0.14),
            open: Some(7.09),
            high: Some(7.11),
            low: Some(7.08),
            pre_close: Some(7.09),
            source: "eastmoney",
        }];
        let json = convert::to_json(&quotes).unwrap();
        assert!(json.contains("\"code\":\"USD\""));
        assert!(json.contains("\"price\":7.1"));
        let csv = convert::to_csv(&quotes).unwrap();
        assert!(csv.starts_with("code,name,price"));
    }

    #[test]
    fn forex_hist_row_serializes() {
        let rows = vec![ForexHistRow {
            symbol: "USD".into(),
            date: "2024-01-01".into(),
            open: Some(7.0),
            close: Some(7.1),
            high: Some(7.2),
            low: Some(6.9),
            amplitude: Some(0.5),
            source: "eastmoney",
        }];
        let json = convert::to_json(&rows).unwrap();
        assert!(json.contains("\"date\":\"2024-01-01\""));
        let csv = convert::to_csv(&rows).unwrap();
        assert!(csv.starts_with("symbol,date"));
    }
}
