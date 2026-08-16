//! Futures market data (akshare `futures` package), Eastmoney-backed ports.

pub mod cot;
pub mod daily;
pub mod extra;
pub mod inventory;
pub mod main;
pub mod sina;
pub mod spot;

pub use daily::{FuturesDailyRow, futures_zh_daily};
pub use inventory::{FuturesInventoryRow, futures_inventory};
pub use main::{FuturesDisplayRow, FuturesMainRow, futures_display, futures_main};
pub use spot::{FuturesSpotRow, futures_zh_spot};
