//! Air-quality domain (空气质量). Ports `akshare/air/*.py`.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `air_quality_hebei` | `air/air_hebei.py:23` | 河北省空气质量预报 (XML feed) |
//! | `air_city_table` | `air/air_zhenqi.py:64` | 真气网全部城市列表 (HTML table) |
//! | `air_quality_hist` | `air/air_zhenqi.py:142` | 真气网历史数据 (JS-signed) |
//! | `air_quality_rank` | `air/air_zhenqi.py:219` | 真气网168城 AQI 排行 (HTML table) |
//! | `air_quality_watch_point` | `air/air_zhenqi.py:99` | 真气网监测点 (JS-signed) |
//!
//! ## DEFERRED
//!
//! Every function in this domain is blocked by a capability outside the allowed
//! dependency set (`reqwest`, `serde`, `serde_json`, `tokio`, `csv`, `thiserror`,
//! `sha2`; no XML/HTML parser, no JS engine):
//!
//! - `air_quality_hebei` (`air/air_hebei.py:23`) — plain `GET
//!   http://218.11.10.130:8080/api/hour/130000.xml` but the response is **XML**,
//!   parsed via `BeautifulSoup(r.content, features="xml")`. No XML/HTML parser
//!   crate is available, so it cannot be parsed without a new dependency.
//! - `air_city_table` (`air/air_zhenqi.py:64`) — response is HTML scraped with
//!   `pd.read_html(StringIO(r.text))[1]` (HTML-table scraping).
//! - `air_quality_rank` (`air/air_zhenqi.py:219`) — response is HTML scraped with
//!   `pd.read_html(...)` (HTML-table scraping; 1..4 tables by date type).
//! - `air_quality_hist` (`air/air_zhenqi.py:142`) — requires executing
//!   `air/outcrypto.js` in a JS engine (`MiniRacer`): `encode_param`,
//!   `hex_md5`, `decryptData`, `b.decode`. JS-signed payload + decrypt.
//! - `air_quality_watch_point` (`air/air_zhenqi.py:99`) — requires executing
//!   `air/crypto.js` in a JS engine (`MiniRacer`): `encode_param`,
//!   `encode_secret`, `decode_result`. JS-signed payload + decrypt.
//!
//! Revisit when an XML/HTML parser and/or a JS-signing helper (port of the
//! akshare `crypto.js`/`outcrypto.js` logic to Rust) is added.

pub mod air_gaps;
pub mod hebei;
pub mod zhenqi;
pub mod air_html_gaps;
