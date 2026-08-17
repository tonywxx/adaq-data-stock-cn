use crate::core::client::Client;
use crate::core::error::Result;
use crate::core::source::SourceChain;

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
    SourceChain::new()
        .push(move |c| {
            Box::pin(eastmoney::daily(c, symbol, period, adjust, start_date, end_date))
        })
        .push(move |c| {
            Box::pin(tencent::daily(c, symbol, period, adjust, start_date, end_date))
        })
        .run(client)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::convert;

    #[test]
    fn hist_row_serializes() {
        let rows = vec![HistRow {
            symbol: "600519".into(),
            date: "2024-01-02".into(),
            open: Some(1685.0),
            close: Some(1700.0),
            high: Some(1710.0),
            low: Some(1680.0),
            volume: Some(1_000_000.0),
            amount: Some(1_700_000_000.0),
            pct_change: Some(1.2),
            source: "eastmoney",
        }];
        let json = convert::to_json(&rows).unwrap();
        assert!(json.contains("\"symbol\":\"600519\""));
        assert!(json.contains("\"date\":\"2024-01-02\""));
        let csv = convert::to_csv(&rows).unwrap();
        assert!(csv.starts_with("symbol,date"));
    }
}
