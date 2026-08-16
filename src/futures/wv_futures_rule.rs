//! Eastmoney futures trading-hours (`futures_trading_hours_em`).
//!
//! Ports akshare `futures_trading_hours_em`. In upstream akshare this function
//! is an empty stub (`def futures_trading_hours_em(): pass`) — it returns
//! nothing. We mirror that by returning an empty row set (no upstream HTTP
//! contract exists to port).

use crate::core::client::Client;
use crate::core::error::Result;

/// Eastmoney futures trading-hours row placeholder.
///
/// akshare's `futures_trading_hours_em` is a stub and yields no columns, so
/// this row is never populated.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesTradingHoursRow {}

/// Eastmoney futures trading hours (`futures_trading_hours_em`).
///
/// Returns an empty set: upstream akshare implements this as `pass`.
pub async fn futures_trading_hours_em(_client: &Client) -> Result<Vec<FuturesTradingHoursRow>> {
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::client::Client;

    #[tokio::test]
    async fn trading_hours_em_is_empty_stub() {
        let client = Client::new();
        let rows = futures_trading_hours_em(&client).await.unwrap();
        assert!(rows.is_empty());
    }
}
