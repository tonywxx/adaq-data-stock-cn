use crate::core::client::Client;
use crate::core::error::Result;
use crate::core::source::SourceChain;

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
    SourceChain::new()
        .push(move |c| Box::pin(eastmoney::em(c, symbol)))
        .push(move |c| Box::pin(sina::sina(c, symbol, date)))
        .run(client)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::convert;

    #[test]
    fn intraday_row_serializes() {
        let rows = vec![IntradayRow {
            symbol: "600519".into(),
            time: "09:35:00".into(),
            price: Some(1700.0),
            volume: Some(500.0),
            direction: Some("买盘".into()),
            source: "eastmoney",
        }];
        let json = convert::to_json(&rows).unwrap();
        assert!(json.contains("\"symbol\":\"600519\""));
        assert!(json.contains("\"time\":\"09:35:00\""));
        let csv = convert::to_csv(&rows).unwrap();
        assert!(csv.starts_with("symbol,time"));
    }
}
