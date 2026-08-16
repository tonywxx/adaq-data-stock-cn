//! Alternative / real-world data domains (akshare `air`, `energy`, `movie`,
//! `bank`-fx, `fortune`, ...).
//!
//! This module ports akshare's "alternative data" public functions that hit
//! Eastmoney / JSON / POST-JSON upstreams. HTML-scrape and JS-signed endpoints
//! are intentionally **excluded** (see each submodule's docs and the delivery
//! report):
//!
//! - `air` (空气质量) — every function uses JS signing (`outcrypto.js` /
//!   `crypto.js` via `py_mini_racer`) or `pd.read_html` HTML scraping.
//! - `bank` (银行罚单) — CBIRC/NFRAC pages are HTML-scraped.
//! - `coiling` / `spot_price_qh` (螺纹钢) — a per-request `_pcc` token is
//!   fetched from a response header first and the symbol→productId map is
//!   scraped from a `__NEXT_DATA__` script tag. Not a clean JSON endpoint.
//! - `epidemic`, `food`, `education`, `weather` — no clean JSON public function
//!   exists in this akshare version (no source module). `fortune` rankings are
//!   HTML-scraped and `fortune_lotto` (彩票) has no source file here.
//!
//! Implemented submodules:
//! - `energy` — Eastmoney `datacenter-web` GET/JSON (`energy_oil_hist`,
//!   `energy_oil_detail`).
//! - `movie` — 艺恩 (endata) POST/JSON (`movie_boxoffice_daily`,
//!   `movie_boxoffice_realtime`), via `Client::post_form_json`.
//! - `fx` — 中国外汇交易中心 (chinamoney) POST/JSON (`fx_spot_quote`,
//!   `fx_pair_quote`); this is the upstream behind the requested `bank_fx_spot`.
//!
//! All row structs derive `Debug`, `Clone`, `serde::Serialize` and carry a
//! `source: &'static str` field for provenance. Parse functions are tolerant:
//! malformed upstream rows are skipped (`continue`).

pub mod energy;
pub mod fx;
pub mod movie;
pub mod movie_yien;

pub use energy::*;
pub use fx::*;
pub use movie::*;

use serde_json::Value;

/// Extract a string field, if present.
pub(crate) fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Extract a numeric field, accepting either a JSON number or a numeric string.
pub(crate) fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    })
}

/// Extract an integer field (some upstreams encode ints as strings).
pub(crate) fn fint(item: &Value, k: &str) -> Option<i64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.parse::<i64>().ok(),
        _ => None,
    })
}
