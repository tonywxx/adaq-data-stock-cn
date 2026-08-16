use crate::core::client::Client;
use crate::core::error::{Error, Result};

pub mod eastmoney;
pub mod tencent;

/// Canonical, source-agnostic per-symbol historical OHLC bar (ADR-0001 / ADR-0010).
///
/// Mirrors akshare's `stock_zh_a_hist` / `stock_zh_a_hist_tx` columns. Every source
/// (eastmoney / tencent) normalizes into this type. `volume` is shares and `amount`
/// is CNY; units are reconciled per-source to match akshare's final output.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HistRow {
    pub symbol: String,
    pub date: String,
    pub open: Option<f64>,
    pub close: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub volume: Option<f64>,
    pub amount: Option<f64>,
    pub pct_change: Option<f64>,
    pub source: &'static str,
}

/// Aggregated per-symbol daily/weekly/monthly history with multi-source fallback
/// (ADR-0010): eastmoney → tencent. Returns the first successful source, normalized.
pub async fn daily(
    client: &Client,
    symbol: &str,
    period: &str,
    adjust: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<HistRow>> {
    if let Ok(rows) = eastmoney::daily(client, symbol, period, adjust, start_date, end_date).await {
        return Ok(rows);
    }
    if let Ok(rows) = tencent::daily(client, symbol, period, adjust, start_date, end_date).await {
        return Ok(rows);
    }
    Err(Error::UpstreamChanged {
        origin: "all",
        message: "all hist sources failed".into(),
    })
}
