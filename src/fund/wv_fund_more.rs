//! Fund "more" functions — deferral-tracking module.
//!
//! This module was assigned 7 akshare fund functions to port. All 7 are
//! **DEFERRED** per the porting policy, so no `pub` functions are implemented
//! here yet. The deferral reasons:
//!
//! | akshare fn | source | reason |
//! |---|---|---|
//! | `fund_etf_dividend_sina` | `fund/fund_etf_sina.py:152` | Sina `hfq.js` returns a `var x = {...}` JS object literal parsed via Python `eval` (non-JSON "special format"); not a clean JSON/JSONP GET, cannot port faithfully offline. |
//! | `fund_individual_basic_info_xq` | `fund/fund_xq.py:13` | danjuanfunds / xueqiu API requires `xq_a_token` cookie / session. |
//! | `fund_individual_achievement_xq` | `fund/fund_xq.py:78` | same xq token gate. |
//! | `fund_individual_analysis_xq` | `fund/fund_xq.py:132` | same xq token gate. |
//! | `fund_individual_profit_probability_xq` | `fund/fund_xq.py:185` | same xq token gate. |
//! | `fund_individual_detail_info_xq` | `fund/fund_xq.py:224` | same xq token gate. |
//! | `fund_individual_detail_hold_xq` | `fund/fund_xq.py:270` | same xq token gate. |
//!
//! Tracking table lives in `docs/_draft_fund.md`. The public entry points
//! will be added here once a clean (token-free / JSON) path is available.
