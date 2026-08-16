//! Realized volatility (Oxford-Man / Risk Lab) — port of `akshare/article/risk_rv.py`.
//!
//! `article_oman_rv` (`risk_rv.py:18`), `article_oman_rv_short` (`risk_rv.py:78`)
//! and `article_rlab_rv` (`risk_rv.py:117`) are **DEFERRED**.
//!
//! ## DEFERRED
//!
//! * `article_oman_rv` (`risk_rv.py:18`) — Data is embedded as a JS object literal
//!   inside an HTML `<p>` tag (`visualization-data.js?...`), extracted via
//!   BeautifulSoup + JSON substring slicing. Not plain CSV/JSON.
//! * `article_oman_rv_short` (`risk_rv.py:78`) — Same pattern, from
//!   `front-page-chart.js`; HTML / JS-in-HTML scraping.
//! * `article_rlab_rv` (`risk_rv.py:117`) — Scrapes an HTML `<p>` text body
//!   (`https://dachxiu.chicagobooth.edu/data.php`) split by ticker symbol.
//!   HTML text scraping, not plain CSV/JSON.
