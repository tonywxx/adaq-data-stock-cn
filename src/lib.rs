pub mod core;
pub mod stock;
pub mod stock_fundamental;

pub mod alt;
pub mod board;
pub mod bond;
pub mod calendar;
pub mod coin;
pub mod crypto;
pub mod economic;
pub mod forex;
pub mod fx;
pub mod fund;
pub mod futures;
pub mod index;
pub mod lpr;
pub mod news;
pub mod option;
pub mod other;
pub mod rate;
pub mod spot;

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
pub mod qhkc_web;
pub mod reits;
pub mod video;

// --- wave-3 new top-level domains (akshare long-tail ports) ---
pub mod stock_feature;
pub mod energy;
pub mod registry;
pub mod datasets;
pub mod cal;
pub mod pro;

pub use core::client::Client;
pub use core::convert;
pub use core::error::{Error, Result};
