//! Options market data (akshare `option` package).
//!
//! Ports of akshare's `option` package public functions: Eastmoney kline-based
//! daily/minute history and Sina real-time/daily option quotes.

pub mod eastmoney;
pub mod extra;
pub mod sina;

pub use eastmoney::{option_daily, option_minute};
pub use sina::{option_cffex_daily, option_sina_spot};
