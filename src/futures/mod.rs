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

pub mod wv_futures_cffex;

pub mod wv_futures_index;

pub mod wv_futures_news;

pub mod wv_futures_rule;

pub mod wv_futures_settle;

// --- assigned futures ports (this agent) ---
pub mod global_em_hist;
pub mod exchange_shfe;
pub mod exchange_ine;
pub mod exchange_dce;
pub mod exchange_gfex;
pub mod exchange_czce;
pub mod sina_hq;
pub mod deferred;

pub use sina_hq::{ForeignCommodityRow, futures_foreign_commodity_realtime};

// --- re-triage pass: pure-JSON ports + deferrals (this agent) ---
pub mod global_spot_em;
pub mod warehouse_receipt_shfe;
pub mod deferred_more;

pub use global_spot_em::{GlobalSpotEmRow, futures_global_spot_em};
pub use warehouse_receipt_shfe::{ShfeWarehouseReceiptRow, futures_shfe_warehouse_receipt};

// --- second-wave futures ports (this agent) ---
pub mod wv_futures_more;

// --- futures "gaps" ports: sina main-continuous listing + 99qh inventory ---
pub mod fut_gaps;

// --- futures "gaps" ports: HTML-table scraping (pd.read_html helpers) ---
pub mod html_gaps;
