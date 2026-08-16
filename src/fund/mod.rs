//! Fund / ETF / LOF / open-end fund data (akshare `fund` package).
//!
//! Ports the most prominent Eastmoney-backed akshare public functions for the
//! fund domain: real-time ETF/LOF spot, ETF/LOF daily history (kline), and
//! open-end fund NAV history. All functions are async and take `&Client`.
//!
//! Eastmoney endpoints use static `ut` tokens + `push2`/`push2his` kline +
//! `clist` — no JS signing (ADR-0005). The only exception is `fund_open_fund_info`,
//! whose upstream is a JS file containing a JSON array; we extract that array
//! without evaluating JS.

pub mod amac;
pub mod em;
pub mod etf;
pub mod extra;
pub mod lof;
pub mod more;
pub mod more2;
pub mod open_fund;

pub use etf::{EtfHistRow, EtfSpotRow, fund_etf_hist_em, fund_etf_spot_em};
pub use lof::{LofSpotRow, fund_lof_spot_em};
pub use open_fund::{OpenFundNavRow, fund_open_fund_info};

/// Extract a string field from an Eastmoney `clist` item.
pub(crate) fn fstr(item: &serde_json::Value, k: &str) -> String {
    item.get(k)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

/// Extract a numeric field from an Eastmoney item (numbers or numeric strings).
pub(crate) fn fnum(item: &serde_json::Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    })
}

/// Parse a possibly-empty numeric string (kline CSV cells).
pub(crate) fn parse_f64(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        t.parse::<f64>().ok()
    }
}

pub mod wv_fund_misc;
pub mod wv_fund_more;

// ## DEFERRED (assigned in this batch but not ported — exact reasons)
//
// - `fund_individual_basic_info_xq` (`fund_xq.py:13`) — 雪球/danjuanfunds
//   requires `xq_a_token` cookie / xueqiu session (DEFER: `fund_individual_*_xq`).
// - `fund_individual_achievement_xq` (`fund_xq.py:78`) — same xq token gate.
// - `fund_individual_analysis_xq` (`fund_xq.py:132`) — same xq token gate.
// - `fund_individual_profit_probability_xq` (`fund_xq.py:185`) — same xq token gate.
// - `fund_individual_detail_info_xq` (`fund_xq.py:224`) — same xq token gate.
// - `fund_individual_detail_hold_xq` (`fund_xq.py:270`) — same xq token gate.
//
// Note: the other assigned functions (`amac_fund_abs`, `amac_person_bond_org_list`,
// `fund_hk_rank_em`, `fund_lof_hist_em`, `fund_new_found_ths`) were already ported
// in `wv_fund_misc.rs` by a prior pass, so this batch adds no new leaf module.

pub mod excel_gaps;
