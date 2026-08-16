//! Fortune / wealth-ranking domain (port of akshare `fortune/` and
//! `movie/artist_yien.py`).
//!
//! | Rust function | akshare source | status |
//! |---|---|---|
//! | `xincaifu_rank` | `fortune/fortune_xincaifu_500.py:15` | DONE — pure HTTP-JSONP (`data.rows`) |
//! | `index_bloomberg_billionaires` | `fortune/fortune_bloomberg.py:65` | DEFERRED — HTML table scrape (`soup.find(class="table-chart")`) |
//! | `index_bloomberg_billionaires_hist` | `fortune/fortune_bloomberg.py:14` | DEFERRED — HTML table scrape (areppim.com `BeautifulSoup`) |
//! | `forbes_rank` | `fortune/fortune_forbes_500.py:14` | DEFERRED — `pd.read_html` table scrape of forbeschina.com |
//! | `hurun_rank` | `fortune/fortune_hurun.py:16` | DEFERRED — needs HTML scrape of `<select id="exampleFormControlSelect1">` to resolve `num` code before the JSON API call |
//! | `business_value_artist` | `movie/artist_yien.py:65` | DEFERRED — endata response is JS-decrypted via `py_mini_racer` (`jm.js`) |
//! | `online_value_artist` | `movie/artist_yien.py:103` | DEFERRED — same JS-decrypt (`jm.js`) requirement as `business_value_artist` |
//!
//! ## DEFERRED
//!
//! Six of the seven functions in this domain rely on HTML-table scraping or a
//! JS-decrypt step and are **not** pure HTTP-JSON, so they are skipped per the
//! porting rules:
//!
//! * **`index_bloomberg_billionaires`** / **`index_bloomberg_billionaires_hist`**
//!   parse HTML `<table>` markup with BeautifulSoup (bloomberg.com /
//!   stats.areppim.com). No JSON API.
//! * **`forbes_rank`** uses `pd.read_html(r.text)[0]` to scrape a Forbes China
//!   ranking table out of HTML.
//! * **`hurun_rank`** first scrapes a `<select>` dropdown to map
//!   `indicator`+`year` → `num` code; only *then* calls the JSON endpoint
//!   `HsRankDetailsList`. The code-mapping scrape is the blocker.
//! * **`business_value_artist`** / **`online_value_artist`** POST to
//!   `endata.com.cn/API/GetData.ashx` but the response body is encrypted and
//!   must be run through a JS VM (`webInstace.shell` in `jm.js`). We cannot
//!   replicate the decrypt without the JS engine.

pub mod xincaifu;
