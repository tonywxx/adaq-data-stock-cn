//! Tushare-Pro bridge ported (or documented) from `akshare/pro/`.
//!
//! ## DEFERRED — third-party token/session wrapper
//!
//! akshare's `pro` module is a thin client over the Tushare Pro HTTP API. It
//! requires a user-supplied `token` (Tushare account), maintains a session,
//! and returns arbitrarily-shaped per-endpoint frames. It is a third-party
//! credentialed service, not a public no-auth market-data endpoint, and the
//! per-endpoint schemas are unbounded. Per the crate's DEFER policy (third-
//! party token/session) it is recorded here (and in `docs/MAPPING.md`) rather
//! than faked.
//!
//! * `pro_api(token=None)` (`pro/__init__.py`) — returns a `ProApi` object
//!   bound to a Tushare token; callers then invoke named endpoints
//!   (`pro_bar`, `daily`, `stock_basic`, …). The token is mandatory for any
//!   real call.
//!
//! A faithful port would be a separate `adaq-data-tushare` crate with its own
//! token config; it does not belong in this akshare-parity data layer.
