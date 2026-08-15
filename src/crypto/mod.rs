//! Crypto market data (akshare `crypto` package).
//!
//! Ports akshare's crypto public functions. All current sources are Jin10
//! (金十数据, `datacenter-api.jin10.com`); requests carry Jin10's public
//! `X-App-Id` header — no signing/secret is required.
//!
//! | akshare fn               | Rust fn                  | source |
//! |--------------------------|--------------------------|--------|
//! | `crypto_js_spot`         | [`crypto_js_spot`]       | jin10  |
//! | `crypto_bitcoin_cme`     | [`crypto_bitcoin_cme`]   | jin10  |
//! | `crypto_bitcoin_hold_report` | [`crypto_bitcoin_hold_report`] | jin10 |

pub mod bitcoin_cme;
pub mod bitcoin_hold;
pub mod js_spot;

pub use bitcoin_cme::{crypto_bitcoin_cme, CryptoCme};
pub use bitcoin_hold::{crypto_bitcoin_hold_report, CryptoHold};
pub use js_spot::{crypto_js_spot, CryptoSpot};
