use crate::core::client::Client;
use crate::core::error::Result;
use crate::core::source::SourceChain;

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
    SourceChain::new()
        .push(|c| Box::pin(eastmoney::spot(c)))
        .push(|c| Box::pin(sina::spot(c)))
        .push(|c| Box::pin(tencent::spot(c)))
        .run(client)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::convert;

    #[test]
    fn spot_quote_serializes() {
        let quotes = vec![SpotQuote {
            code: "600519".into(),
            name: "贵州茅台".into(),
            price: Some(1700.0),
            pct_change: Some(1.2),
            change: Some(20.0),
            volume: Some(1_000_000.0),
            amount: Some(1_700_000_000.0),
            turnover_rate: Some(0.8),
            pe: Some(30.0),
            high: Some(1710.0),
            low: Some(1680.0),
            open: Some(1685.0),
            pre_close: Some(1680.0),
            total_mv: Some(2_000_000_000_000.0),
            float_mv: Some(1_500_000_000_000.0),
            source: "eastmoney",
        }];
        let json = convert::to_json(&quotes).unwrap();
        assert!(json.contains("\"code\":\"600519\""));
        let csv = convert::to_csv(&quotes).unwrap();
        assert!(csv.starts_with("code,name,price"));
    }
}
