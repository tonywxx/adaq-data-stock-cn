use crate::core::client::Client;
use crate::core::error::{Error, Result};

pub mod eastmoney;
pub mod extra;
pub mod more;
pub mod sina;

/// Canonical, source-agnostic real-time index spot quote (ADR-0001 / ADR-0010).
///
/// Mirrors [`crate::stock::spot::SpotQuote`] but for indices. Every source
/// (eastmoney / sina) normalizes into this type.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexSpotQuote {
    pub code: String,
    pub name: String,
    pub price: Option<f64>,
    pub pct_change: Option<f64>,
    pub change: Option<f64>,
    pub volume: Option<f64>,
    pub amount: Option<f64>,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub pre_close: Option<f64>,
    pub source: &'static str,
}

/// Aggregated real-time index spot with multi-source fallback (ADR-0010):
/// eastmoney → sina. Returns the first successful source, normalized.
pub async fn spot(client: &Client) -> Result<Vec<IndexSpotQuote>> {
    if let Ok(rows) = eastmoney::spot(client).await {
        return Ok(rows);
    }
    if let Ok(rows) = sina::spot(client).await {
        return Ok(rows);
    }
    Err(Error::UpstreamChanged {
        origin: "all",
        message: "all index spot sources failed".into(),
    })
}
