//! Fama–French data library — port of `akshare/article/ff_factor.py`.
//!
//! `article_ff_crr` (`ff_factor.py:17`) is **DEFERRED**.
//!
//! ## DEFERRED
//!
//! * `article_ff_crr` (`ff_factor.py:17`) — Parses HTML `<table>` elements from
//!   `http://mba.tuck.dartmouth.edu/pages/faculty/ken.french/data_library.html`
//!   via `pd.read_html` (BeautifulSoup / lxml HTML-table scraping). This crate has
//!   no HTML parser and the data is not available as plain CSV/JSON, so it cannot
//!   be ported without HTML scraping (explicitly out of scope per the Porting
//!   Guide).
