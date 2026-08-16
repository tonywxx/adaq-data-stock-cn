//! 奇货可查-资金数据 (qhkc fund). Ports `akshare/qhkc_web/qhkc_fund.py`.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `qhkc_fund_bs` | `qhkc_fund.py:23` | 净持仓分布; POST `fund_bs_pie.php`, `data.datas1`/`datas2` |
//! | `qhkc_fund_position` | `qhkc_fund.py:121` | 总持仓分布; POST `fund_position_pie.php` |
//! | `qhkc_fund_money_change` | `qhkc_fund.py:319` | 成交额分布; POST `fund_deal_pie.php`, `data.datas` |
//!
//! ## DEFERRED
//!
//! - `qhkc_fund_bs` — endpoint `https://qhkch.com/ajax/fund_bs_pie.php` returns
//!   `{"code":404,"message":"Not Found"}` (probed 2026-08-15). The public JSON
//!   API is withdrawn / behind commercial token auth. No token available here.
//! - `qhkc_fund_position` — same: `fund_position_pie.php` now 404s.
//! - `qhkc_fund_money_change` — same: `fund_deal_pie.php` now 404s.
//!
//! All three are pure JSON POSTs in the akshare source (no JS signature), so the
//! only blocker is that the upstream endpoint no longer serves public data.
