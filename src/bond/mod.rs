//! Bond market data (akshare `bond` package).
//!
//! Submodules are organized by upstream source:
//! - [`eastmoney`] — Eastmoney datacenter / push2 endpoints.
//! - [`chinamoney`] — ChinaMoney (CFETS) POST endpoints.

pub mod cbond;
pub mod chinamoney;
pub mod cov;
pub mod eastmoney;
pub mod extra;

pub use chinamoney::*;
pub use eastmoney::*;
