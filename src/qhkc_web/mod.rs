//! 奇货可查 website domain (port of akshare `qhkc_web/`).
//!
//! | Rust function | akshare source | status |
//! |---|---|---|
//! | `qhkc_tool_gdp` | `qhkc_web/qhkc_tool.py:111` | DONE — HTML table scrape (see note in `qhkc_html_gaps.rs`: upstream AJAX is dead, structure verified) |
//!
//! The sibling `qhkc` domain covers the authenticated/API surface; this domain
//! covers the public website HTML tables.

pub mod qhkc_html_gaps;
