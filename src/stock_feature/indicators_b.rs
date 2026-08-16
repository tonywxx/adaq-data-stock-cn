//! Remaining `stock_feature` *indicator* endpoints that are **not** portable as
//! pure-HTTP JSON. The tractable half of this family lives in
//! `indicators_a.rs`; everything here is blocked by a legulegu token/session,
//! a `py_mini_racer` JS signature, or HTML-table scraping. Per the crate's
//! DEFER policy they are documented rather than faked. (Authoritative list in
//! `docs/MAPPING.md`.)
//!
//! ### legulegu token/session (`stock_a_indicator.py`, `stock_*_lg.py`)
//! * `get_cookie_csrf` — fetches a page and extracts an `_csrf` meta token +
//!   cookies for legulegu.
//! * `get_token_lg` — MD5 date token for legulegu.
//! * `stock_a_all_pb` — legulegu token + cookie-csrf.
//! * `stock_a_congestion_lg` — legulegu token + cookie-csrf.
//! * `stock_a_gxl_lg` — legulegu token + cookie-csrf.
//! * `stock_buffett_index_lg` — legulegu token + cookie-csrf.
//! * `stock_ebs_lg` — legulegu token + cookie-csrf.
//! * `stock_a_ttm_lyr` — legulegu token + cookie-csrf + `py_mini_racer` JS.
//!
//! ### THS `py_mini_racer` JS signing + HTML scrape (`stock_board_*_ths.py`)
//! * `stock_board_concept_index_ths` / `_info_ths` / `_name_ths` / `_summary_ths`
//! * `stock_board_industry_index_ths` / `_info_ths` / `_name_ths` / `_summary_ths`
//!
//! ### HTML-in-JSON / embedded-`<font>` scrape (`stock_classify_sina.py`)
//! * `stock_classify_board` — returns a nested dict with embedded `<font>` HTML
//!   parsed via BeautifulSoup inside the JSON.
//! * `stock_classify_sina` — depends on `stock_classify_board` + multi-page
//!   Sina JSON.
//!
//! ### `py_mini_racer` JS engine (`stock_cyq_em.py`, `stock_fund_flow.py`)
//! * `stock_cyq_em` — JS engine computes the per-row chip distribution.
//! * `stock_fund_flow_big_deal` — THS `py_mini_racer` signing + HTML scrape.
//!
//! ### HTML-table scrape (`stock_fhps_ths.py`)
//! * `stock_fhps_detail_ths` — `pd.read_html` + THS.
