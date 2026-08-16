pub mod core;
pub mod stock;

pub mod alt;
pub mod board;
pub mod bond;
pub mod coin;
pub mod calendar;
pub mod crypto;
pub mod economic;
pub mod forex;
pub mod futures;
pub mod lpr;
pub mod news;
pub mod option;
pub mod fund;
pub mod index;
pub mod rate;
pub mod spot;
pub mod other;

// --- wave-1 new top-level domains (akshare long-tail ports) ---
pub mod air;
pub mod article;
pub mod bank;
pub mod currency;
pub mod event;
pub mod fortune;
pub mod futures_derivative;
pub mod hf;
pub mod qdii;
pub mod qhkc;
pub mod reits;
pub mod video;

pub use core::client::Client;
pub use core::error::{Error, Result};
pub use core::convert;
