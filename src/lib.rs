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

pub use core::client::Client;
pub use core::error::{Error, Result};
pub use core::convert;
