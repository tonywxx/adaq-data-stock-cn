//! Foreign-exchange market data (akshare `forex` package).

use crate::core::client::Client;
use crate::core::error::Result;

pub mod eastmoney;

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
