//! Bond market data (akshare `bond` package).
//!
//! Submodules are organized by upstream source:
//! - [`eastmoney`] — Eastmoney datacenter / push2 endpoints.
//! - [`chinamoney`] — ChinaMoney (CFETS) POST endpoints.

pub mod chinamoney;
pub mod eastmoney;

pub use chinamoney::*;
pub use eastmoney::*;
