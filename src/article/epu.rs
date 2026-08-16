//! Economic Policy Uncertainty (EPU) index — port of `akshare/article/epu_index.py`.
//!
//! `article_epu_index` (`epu_index.py:12`) is **DEFERRED**.
//!
//! ## DEFERRED
//!
//! `article_epu_index` (`epu_index.py:12`) has two blockers. First, the returned
//! data is a wide year×month pivot table with country-specific column sets (e.g.
//! `Year,Jan,Feb,...,Dec,Annual`); it has no stable typed `Row` shape, so it
//! cannot satisfy the `Vec<Row>` contract without an unstable generic schema.
//! Second, the default `symbol="China"` (→ `SCMP_China`) and several other
//! symbols (`Hong Kong`, `Ireland`, `Chile`, `Colombia`, `Netherlands`,
//! `Singapore`, `Sweden`, `Greece`) are served as `.xlsx` Excel files parsed with
//! `openpyxl` — not plain CSV/JSON. No Excel/zip parser is available in this crate
//! (no new crates allowed). The pure-CSV symbols (`USA`→`US`, `Europe`, `Korea`,
//! `Spain`) are reachable, but implementing only a subset while the default needs
//! Excel would be a partial, misleading port, so the whole function is deferred.
