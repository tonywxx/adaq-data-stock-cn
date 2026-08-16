//! Fortune / wealth-ranking domain (port of akshare `fortune/` and
//! `movie/artist_yien.py`).
//!
//! | Rust function | akshare source | status |
//! |---|---|---|
//! | `xincaifu_rank` | `fortune/fortune_xincaifu_500.py:15` | DONE — pure HTTP-JSONP (`data.rows`) |
//! | `index_bloomberg_billionaires` | `fortune/fortune_bloomberg.py:65` | DONE — HTML scrape of `div.table-chart` (`fortune_html_gaps.rs`) |
//! | `index_bloomberg_billionaires_hist` | `fortune/fortune_bloomberg.py:14` | DONE — HTML table scrape (areppim.com) (`fortune_html_gaps.rs`) |
//! | `forbes_rank` | `fortune/fortune_forbes_500.py:14` | DONE — `pd.read_html` table scrape of forbeschina.com (`fortune_html_gaps.rs`) |
//! | `hurun_rank` | `fortune/fortune_hurun.py:16` | DONE — HTML scrape of indicator dropdown + year `<select>` then `HsRankDetailsList` JSON (`fortune_html_gaps.rs`) |
//! | `business_value_artist` | `movie/artist_yien.py:65` | DEFERRED — endata response is JS-decrypted via `py_mini_racer` (`jm.js`) |
//! | `online_value_artist` | `movie/artist_yien.py:103` | DEFERRED — same JS-decrypt (`jm.js`) requirement as `business_value_artist` |
//!
//! ## DEFERRED
//!
//! Two functions in this domain rely on a JS-decrypt step and are **not**
//! pure HTTP-JSON, so they are skipped per the porting rules:
//!
//! * **`business_value_artist`** / **`online_value_artist`** POST to
//!   `endata.com.cn/API/GetData.ashx` but the response body is encrypted and
//!   must be run through a JS VM (`webInstace.shell` in `jm.js`). We cannot
//!   replicate the decrypt without the JS engine.

pub mod xincaifu;
pub mod fortune_html_gaps;
