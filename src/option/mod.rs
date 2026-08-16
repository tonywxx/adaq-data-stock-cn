//! Options market data (akshare `option` package).
//!
//! Ports of akshare's `option` package public functions: Eastmoney kline-based
//! daily/minute history and Sina real-time/daily option quotes.

pub mod commodity;
pub mod eastmoney;
pub mod extra;
pub mod sina;
pub mod sse;

pub use eastmoney::{option_daily, option_minute};
pub use sina::option_cffex_daily;
pub use sse::{
    option_finance_minute_sina, option_minute_em, option_sse_codes_sina, option_sse_daily_sina,
    option_sse_expire_day_sina, option_sse_greeks_sina, option_sse_list_sina,
    option_sse_minute_sina, option_sse_spot_price_sina, option_sse_underlying_spot_price_sina,
};
