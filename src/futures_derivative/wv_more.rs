//! Futures-derivative ports — second wave.
//!
//! ## DEFERRED
//! * **`futures_display_main_sina`** (`futures_derivative/futures_index_sina.py:89`)
//!   — iterates the five exchanges, calling `match_main_contract`, which decodes
//!   a Sina JS document with `akshare.utils.demjson` (lenient, non-strict JSON).
//!   No strict-JSON Rust equivalent exists without a JS engine / `demjson`, so
//!   this is deferred per the porting DEFER policy (JS execution).
//!
//! ### Deferred table
//! | akshare fn | source | reason |
//! |---|---|---|
//! | `futures_display_main_sina` | `futures_index_sina.py:89` | `demjson` lenient JSON parse of a JS document (needs a JS engine) |

#![allow(dead_code)]

/// Deferred akshare futures-derivative fns in this module (see module doc).
pub const DEFERRED_FNS: &[&str] = &["futures_display_main_sina"];
