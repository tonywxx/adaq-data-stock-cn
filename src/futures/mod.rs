//! Futures market data (akshare `futures` package), Eastmoney-backed ports.

pub mod daily;
pub mod inventory;
pub mod spot;

pub use daily::{futures_zh_daily, FuturesDailyRow};
pub use inventory::{futures_inventory, FuturesInventoryRow};
pub use spot::{futures_zh_spot, FuturesSpotRow};
