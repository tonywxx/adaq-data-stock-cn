//! 奇货可查 (qhkc) domain — DEFERRED.
//!
//! Every `akshare/qhkc_web/*` function hits a `qhkch.com/ajax/*` JSON endpoint
//! (or an HTML page). Live probes from this environment on 2026-08-15 confirm the
//! public AJAX API has been withdrawn:
//!
//! - `POST https://qhkch.com/ajax/official_indexes.php` → `{"code":404,"message":"Not Found"}`
//! - `POST https://www.qhkch.com/ajax/index_show.php`    → `{"code":404,"message":"Not Found"}`
//! - `POST https://qhkch.com/ajax/toolbox_foreign.php`   → `{"code":404,"message":"Not Found"}`
//!
//! The root site still loads, but the data AJAX endpoints are gone / now sit
//! behind commercial token auth, so the data is no longer reachable by a plain
//! HTTP call. Per the porting guide, DEFER rather than fake. See each leaf
//! module's `## DEFERRED` section for per-function reasons.

pub mod fund;
pub mod index;
pub mod tool;
