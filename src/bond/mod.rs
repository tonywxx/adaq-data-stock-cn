//! Bond market data (akshare `bond` package).
//!
//! Submodules are organized by upstream source:
//! - [`eastmoney`] — Eastmoney datacenter / push2 endpoints.
//! - [`chinamoney`] — ChinaMoney (CFETS) POST endpoints.

pub mod chinamoney;
pub mod eastmoney;
pub mod extra;
pub mod cbond;
pub mod cov;

pub use chinamoney::*;
pub use eastmoney::*;
