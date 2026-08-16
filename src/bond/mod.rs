//! Bond market data (akshare `bond` package).
//!
//! Submodules are organized by upstream source:
//! - [`eastmoney`] — Eastmoney datacenter / push2 endpoints.
//! - [`chinamoney`] — ChinaMoney (CFETS) POST endpoints.

pub mod cbond;
pub mod chinamoney;
pub mod chinamoney_pub;
pub mod cov;
pub mod eastmoney;
pub mod extra;
pub mod jisilu;
pub mod zh;

pub use chinamoney::*;
pub use eastmoney::*;

pub mod wv_bond_misc;
pub mod excel_gaps;
