//! `futures_derivative` domain — reimplementation of
//! `akshare/futures_derivative/*.py` (exchange contract-info, Sina main
//! continuous, 玄田 hog data).
//!
//! | Rust module      | akshare sources covered                              |
//! | ---------------- | --------------------------------------------------- |
//! | `contract_info`  | `futures_contract_info_dce/gfex/ine/shfe.py`        |
//! | `sina`           | `futures_index_sina.py`                             |
//! | `hog`            | `futures_hog.py`                                    |
//!
//! ## DEFERRED (see each leaf module `//!` doc for details)
//! - `futures_contract_info_cffex` / `futures_contract_info_czce` — XML parsing
//!   (needs an XML crate; `Cargo.toml` may not be edited).
//! - `futures_hold_pos_sina` (`futures_cot_sina.py`) — HTML table scraping
//!   (`pd.read_html`).
//! - `futures_display_main_sina` (`futures_index_sina.py`) — `demjson` lenient
//!   JSON parse of a JS file (no strict-JSON equivalent available).
//! - `futures_spot_sys` (`futures_spot_sys.py`) — BeautifulSoup HTML scraping +
//!   `pd.read_html`.

pub mod contract_info;
pub mod hog;
pub mod sina;

// --- second-wave futures-derivative ports (this agent) ---
pub mod wv_more;
