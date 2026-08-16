//! Deferred futures ports (re-triaged, not fakeable in Rust).
//!
//! These akshare functions from the assignment brief were re-read against
//! their source and **deferred** per `docs/PORTING_GUIDE.md` rule #4: they
//! require JS execution, HTML scraping, Excel/ZIP, or third-party
//! auth/aggregator coupling that cannot be replicated source-faithfully.
//!
//! | akshare fn | source | reason |
//! |---|---|---|
//! | `futures_zh_realtime` | `futures_zh_sina.py:91` | depends on `futures_symbol_mark()` which uses `demjson` to decode a JS document (and `py_mini_racer` is imported in the module) |
//! | `futures_comm_js` | `futures_comm_js.py:15` | Jin10 endpoint (Jin10 token gate / fixed app-id), per porting-rule deferral |
//! | `futures_spot_price_daily` | `futures_basis.py:31` | HTML scrape of 生意社 (100ppi.com) via `pandas_read_html_link` |
//! | `get_roll_yield` | `futures_roll_yield.py:23` | derived metric over `get_futures_daily` across all exchanges + `cons` symbol-market/variety mappings (calendar coupling) |
//! | `get_roll_yield_bar` | `futures_roll_yield.py:74` | same as `get_roll_yield` (loops `get_futures_daily` across all markets) |
//! | `get_receipt` | `receipt.py:571` | aggregator over per-market warehouse-receipt helpers that fetch HTML/Excel (CZCE `.xls`/`.xlsx`, SHFE HTML) |
//! | `get_futures_daily` | `futures_daily_bar.py:637` | date-looping dispatcher over per-market daily fns; the CFFEX branch calls `get_cffex_daily` (CFFEX daily, not assigned/ported) and requires the trading `calendar` |

#![allow(dead_code)]

/// Marker so callers can see the deferred set is intentional.
pub const DEFERRED_FNS: &[&str] = &[
    "futures_zh_realtime",
    "futures_comm_js",
    "futures_spot_price_daily",
    "get_roll_yield",
    "get_roll_yield_bar",
    "get_receipt",
    "get_futures_daily",
];
