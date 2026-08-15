//! Futures market data (akshare `futures` package), Eastmoney-backed ports.

pub mod daily;
pub mod extra;
pub mod inventory;
pub mod main;
pub mod spot;
pub mod cot;

pub use daily::{futures_zh_daily, FuturesDailyRow};
pub use inventory::{futures_inventory, FuturesInventoryRow};
pub use main::{futures_display, futures_main, FuturesDisplayRow, FuturesMainRow};
pub use spot::{futures_zh_spot, FuturesSpotRow};
