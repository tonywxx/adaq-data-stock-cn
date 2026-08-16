//! 银行监管行政处罚数据 (CBIRC / NFRA).
//!
//! Ports `akshare/bank/bank_cbirc_2020.py`. This module owns a single akshare
//! function, `bank_fjcf_table_detail`.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | — | `bank_cbirc_2020.py:111` (`bank_fjcf_table_detail`) | DEFERRED — see below |
//!
//! ## DEFERRED
//!
//! - `bank_fjcf_table_detail` (`bank_cbirc_2020.py:111`) — **HTML-table scraping**.
//!   The function enumerates doc IDs via `bank_fjcf_page_url`, then for each ID
//!   fetches `https://www.nfra.gov.cn/cn/static/data/DocInfo/SelectByDocId/data_docId=<id>.json`
//!   and parses the embedded `data.docClob` with `pd.read_html(StringIO(...))`.
//!   Parsing HTML tables from per-document blobs is explicitly out of scope per the
//!   porting guide (rule 4: HTML-table scraping → DEFER). No implementation, no
//!   fixture.
