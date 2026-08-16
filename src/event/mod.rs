//! 百度地图慧眼-百度迁徙数据 (Baidu Migration / huiyan).
//!
//! Ports `akshare/event/migration.py`. This module owns two akshare functions:
//! `migration_area_baidu` and `migration_scale_baidu`.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | — | `migration.py:16` (`migration_area_baidu`) | DEFERRED — see below |
//! | — | `migration.py:56` (`migration_scale_baidu`) | DEFERRED — see below |
//!
//! ## DEFERRED
//!
//! - `migration_area_baidu` (`migration.py:16`) — **not pure JSON / third-party
//!   anti-bot**. Hits `https://huiyan.baidu.com/migration/cityrank.jsonp?dt=&id=&type=&date=`
//!   which returns a **JSONP** payload (`callback({...});`) requiring wrapper
//!   stripping, not a clean JSON document. The huiyan endpoint is protected by
//!   Baidu anti-bot and normally needs a valid browser session/cookie; the akshare
//!   source supplies no `ak` token or session, so pure-JSON reach is unreliable.
//!   Per porting guide (rule 4: third-party auth/sessions → DEFER).
//! - `migration_scale_baidu` (`migration.py:56`) — **same reason**: hits
//!   `https://huiyan.baidu.com/migration/historycurve.jsonp?dt=&id=&type=` (JSONP
//!   behind Baidu anti-bot, no `ak` token available). DEFERRED.
//!
//! No implementation, no fixtures.
