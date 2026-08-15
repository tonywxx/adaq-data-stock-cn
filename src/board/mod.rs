//! Eastmoney sector / concept board endpoints (行业板块 / 概念板块 / 地域板块).
//!
//! Ports the most prominent public functions of akshare's
//! `stock_board_industry_em` / `stock_board_concept_em` modules to an async,
//! source-resilient Rust API. All endpoints use Eastmoney's static `ut` token
//! and the `push2` `clist/get` pattern (no JS signing, ADR-0005).
//!
//! Public surface (mirrors akshare function names):
//! - [`stock_board_industry_name_em`](industry::stock_board_industry_name_em)
//! - [`stock_board_industry_cons_em`](industry::stock_board_industry_cons_em)
//! - [`stock_board_concept_name_em`](concept::stock_board_concept_name_em)
//! - [`stock_board_concept_cons_em`](concept::stock_board_concept_cons_em)

pub mod industry;
pub mod concept;

pub use industry::{stock_board_industry_cons_em, stock_board_industry_name_em};
pub use concept::{stock_board_concept_cons_em, stock_board_concept_name_em};

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::Result;

/// Static Eastmoney `ut` token (no JS signing required, ADR-0005).
pub(crate) const UT: &str = "bd1d9ddb04089700cf9c27f6f7426281";

/// Eastmoney `clist/get` endpoint base (canonical host used by akshare variants).
pub(crate) const CLIST_BASE: &str = "https://push2.eastmoney.com/api/qt/clist/get";

/// Default page size, mirroring akshare (`pz=100`).
pub(crate) const PAGE_SIZE: u32 = 100;

/// A sector / concept board as returned by the `*_name_em` endpoints.
///
/// Maps Eastmoney `data.diff` f12/f14/f2/f3/f15/f16/f17/f18/f5/f6/f8.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BoardRow {
    pub code: String,
    pub name: String,
    pub price: Option<f64>,
    pub pct_change: Option<f64>,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub pre_close: Option<f64>,
    pub volume: Option<f64>,
    pub amount: Option<f64>,
    pub turnover: Option<f64>,
}

/// A constituent stock of a board (`*_cons_em` endpoints).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BoardConsRow {
    pub code: String,
    pub name: String,
    pub price: Option<f64>,
    pub pct_change: Option<f64>,
    /// The `BKxxxx` board code this constituent belongs to.
    pub board_code: String,
}

/// Fetch one page of a `clist/get` board query.
pub(crate) async fn fetch_clist_page(
    client: &Client,
    endpoint: &'static str,
    fs: &str,
    fid: &str,
    fields: &str,
    pn: u32,
    pz: u32,
) -> Result<Value> {
    let pn_s = pn.to_string();
    let pz_s = pz.to_string();
    let params = [
        ("pn", pn_s.as_str()),
        ("pz", pz_s.as_str()),
        ("po", "1"),
        ("np", "1"),
        ("ut", UT),
        ("fltt", "2"),
        ("invt", "2"),
        ("fid", fid),
        ("fs", fs),
        ("fields", fields),
    ];
    client
        .get_json(crate::core::client::SOURCE_EASTMONEY, endpoint, CLIST_BASE, &params)
        .await
}

pub(crate) fn fstr(item: &Value, k: &str) -> String {
    item.get(k)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

pub(crate) fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    })
}
