//! 真气网 (zq12369 / aqistudy) air-quality. Ports `akshare/air/air_zhenqi.py`.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `air_city_table` | `air/air_zhenqi.py:64` | 城市列表 (HTML table) |
//! | `air_quality_hist` | `air/air_zhenqi.py:142` | 历史数据 (JS-signed) |
//! | `air_quality_rank` | `air/air_zhenqi.py:219` | 168 城 AQI 排行 (HTML table) |
//! | `air_quality_watch_point` | `air/air_zhenqi.py:99` | 监测点 (JS-signed) |
//!
//! ## DEFERRED
//!
//! - `air_city_table` (`air/air_zhenqi.py:64`) — response is HTML scraped with
//!   `pd.read_html(StringIO(r.text))[1]` (HTML-table scraping).
//! - `air_quality_rank` (`air/air_zhenqi.py:219`) — response is HTML scraped with
//!   `pd.read_html(...)`; table index 1..4 depends on the `date` type (day /
//!   month / year / 实时). HTML-table scraping.
//! - `air_quality_hist` (`air/air_zhenqi.py:142`) — requires executing
//!   `air/outcrypto.js` in a JS engine (`MiniRacer`): `encode_param`,
//!   `hex_md5`, `decryptData`, `b.decode`. The POST body and response are
//!   JS-signed and AES-decrypted. Cannot be reproduced without a JS engine or a
//!   Rust reimplementation of that crypto.
//! - `air_quality_watch_point` (`air/air_zhenqi.py:99`) — requires executing
//!   `air/crypto.js` in a JS engine (`MiniRacer`): `encode_param`,
//!   `encode_secret`, `decode_result`. JS-signed payload + decrypt.
//!
//! All four blocked by HTML-table scraping and/or JS-signed crypto, neither of
//! which is reachable with the allowed dependency set (no HTML/XML parser, no JS
//! engine).
