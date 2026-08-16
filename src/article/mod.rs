//! Article domain — academic / economic data ports from `akshare/article/*`.
//!
//! One leaf module per akshare source file. Implemented functions return typed
//! `Vec<Row>`; functions blocked by Excel/HTML/JS scraping are documented as
//! `DEFERRED` in their leaf module's `## DEFERRED` section.
//!
//! | Rust function | akshare source | status | notes |
//! |---|---|---|---|
//! | `fred_md` | `fred_md.py:13` | DONE | FRED-MD monthly CSV (S3) |
//! | `fred_qd` | `fred_md.py:28` | DONE | FRED-QD quarterly CSV (S3) |
//! | `article_epu_index` | `epu_index.py:12` | DEFERRED | wide pivot + Excel |
//! | `article_ff_crr` | `ff_factor.py:17` | DEFERRED | HTML table scraping |
//! | `article_oman_rv` | `risk_rv.py:18` | DEFERRED | JS-in-HTML |
//! | `article_oman_rv_short` | `risk_rv.py:78` | DEFERRED | JS-in-HTML |
//! | `article_rlab_rv` | `risk_rv.py:117` | DEFERRED | HTML text scraping |
//!
//! ## DEFERRED
//!
//! * `article_epu_index` (`epu_index.py:12`) — wide year×month pivot with
//!   country-specific columns (no stable typed `Row`); default `China` (→
//!   `SCMP_China`) and `Hong Kong`/`Ireland`/`Chile`/`Colombia`/`Netherlands`/
//!   `Singapore`/`Sweden`/`Greece` are served as `.xlsx` (openpyxl). See
//!   `epu.rs`.
//! * `article_ff_crr` (`ff_factor.py:17`) — HTML `<table>` scraping via
//!   `pd.read_html`. See `ff.rs`.
//! * `article_oman_rv` (`risk_rv.py:18`), `article_oman_rv_short`
//!   (`risk_rv.py:78`), `article_rlab_rv` (`risk_rv.py:117`) — data embedded in
//!   HTML/JS, scraped with BeautifulSoup. See `risk_rv.rs`.

pub mod epu;
pub mod ff;
pub mod fred;
pub mod risk_rv;
