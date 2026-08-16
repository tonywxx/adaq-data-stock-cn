//! 奇货可查-指数数据 (qhkc index). Ports `akshare/qhkc_web/qhkc_index.py`.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `qhkc_index` | `qhkc_index.py:21` | 指数详情; POST `index_show.php`, `data.{date,price,volume,...}` |
//! | `qhkc_index_trend` | `qhkc_index.py:77` | 大资金动向; POST `indexes_trend.php`, `data[].{broker,grade,money,...}` |
//! | `qhkc_index_profit_loss` | `qhkc_index.py:149` | 盈亏详情; POST `indexes_profit_loss.php`, `data.{indexes,value,trans_date}` |
//!
//! ## DEFERRED
//!
//! - `qhkc_index` — `official_indexes.php` (id lookup) and `index_show.php`
//!   (`https://www.qhkch.com/ajax/index_show.php`) both return
//!   `{"code":404,"message":"Not Found"}` (probed 2026-08-15). Public JSON API
//!   withdrawn / token-gated.
//! - `qhkc_index_trend` — `indexes_trend.php` now 404s (same cause).
//! - `qhkc_index_profit_loss` — `indexes_profit_loss.php` now 404s (same cause).
//!
//! All three POST plain form data (no JS signature), so the sole blocker is the
//! upstream endpoint no longer serving public data without auth/token.
