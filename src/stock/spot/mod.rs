use crate::core::client::Client;
use crate::core::error::{Error, Result};

pub mod eastmoney;
pub mod sina;
pub mod tencent;

/// Canonical, source-agnostic real-time A-share spot quote (ADR-0001 / ADR-0010).
///
/// Every source (`eastmoney` / `sina` / `tencent`) normalizes into this type so the
/// caller gets a stable shape regardless of which upstream answered.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SpotQuote {
    pub code: String,
    pub name: String,
    pub price: Option<f64>,
    pub pct_change: Option<f64>,
    pub change: Option<f64>,
    pub volume: Option<f64>,
    pub amount: Option<f64>,
    pub turnover_rate: Option<f64>,
    pub pe: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub open: Option<f64>,
    pub pre_close: Option<f64>,
    pub total_mv: Option<f64>,
    pub float_mv: Option<f64>,
    pub source: &'static str,
}

/// Aggregated real-time A-share spot with multi-source fallback (ADR-0010):
/// eastmoney → sina → tencent. Returns the first successful source, normalized.
pub async fn realtime(client: &Client) -> Result<Vec<SpotQuote>> {
    if let Ok(rows) = eastmoney::spot(client).await {
        return Ok(rows);
    }
    if let Ok(rows) = sina::spot(client).await {
        return Ok(rows);
    }
    if let Ok(rows) = tencent::spot(client).await {
        return Ok(rows);
    }
    Err(Error::UpstreamChanged {
        origin: "all",
        message: "all spot sources failed".into(),
    })
}
