//! `stock_feature` **涨停板池 (limit-up pool)** endpoints
//! (`stock_zt_pool_em.py`, `stock_zt_pool_sina.py`, `stock_zt_pool_hist.py`).
//!
//! These endpoints (e.g. `stock_zt_pool_em`, `stock_zt_pool_sina`,
//! `stock_zt_pool_previous`, `stock_zt_pool_strong`, `stock_zt_pool_sub_new`,
//! `stock_zt_pool_dt`, `stock_zt_pool_hist`) are mostly Eastmoney `datacenter`
//! / Sina JSON and are therefore **tractable** — but they are not ported in
//! this wave. They are scheduled for a dedicated `stock`-focused port wave and
//! are enumerated here so the `stock_feature` surface is explicit. See
//! `docs/MAPPING.md` for the running inventory.
