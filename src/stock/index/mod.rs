use crate::core::client::Client;
use crate::core::error::Result;
use crate::core::source::SourceChain;

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
    SourceChain::new()
        .push(|c| Box::pin(eastmoney::spot(c)))
        .push(|c| Box::pin(sina::spot(c)))
        .run(client)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::convert;

    #[test]
    fn index_spot_quote_serializes() {
        let quotes = vec![IndexSpotQuote {
            code: "000001".into(),
            name: "上证指数".into(),
            price: Some(3000.0),
            pct_change: Some(0.5),
            change: Some(15.0),
            volume: Some(200_000_000.0),
            amount: Some(300_000_000_000.0),
            open: Some(2985.0),
            high: Some(3010.0),
            low: Some(2980.0),
            pre_close: Some(2985.0),
            source: "eastmoney",
        }];
        let json = convert::to_json(&quotes).unwrap();
        assert!(json.contains("\"code\":\"000001\""));
        let csv = convert::to_csv(&quotes).unwrap();
        assert!(csv.starts_with("code,name,price"));
    }
}
