use crate::core::client::Client;
use crate::core::error::{Error, Result};

pub mod eastmoney;
pub mod sina;

/// Canonical, source-agnostic intraday tick (`stock_intraday_em` / `stock_intraday_sina`).
///
/// `direction` (买盘/卖盘/中性盘) is populated by Eastmoney; Sina's bill feed has no
/// such field and leaves it `None`. Every source normalizes into this type.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IntradayRow {
    pub symbol: String,
    pub time: String,
    pub price: Option<f64>,
    pub volume: Option<f64>,
    pub direction: Option<String>,
    pub source: &'static str,
}

/// Aggregated intraday ticks with multi-source fallback (ADR-0010): eastmoney → sina.
/// `date` (YYYYMMDD) is only used by the Sina source.
pub async fn tick(client: &Client, symbol: &str, date: &str) -> Result<Vec<IntradayRow>> {
    if let Ok(rows) = eastmoney::em(client, symbol).await {
        return Ok(rows);
    }
    if let Ok(rows) = sina::sina(client, symbol, date).await {
        return Ok(rows);
    }
    Err(Error::UpstreamChanged {
        origin: "all",
        message: "all intraday sources failed".into(),
    })
}
