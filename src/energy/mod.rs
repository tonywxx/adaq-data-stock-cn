//! Carbon-emissions trading endpoints ported (or documented) from
//! `akshare/energy/energy_carbon.py`.
//!
//! ## DEFERRED — none of these are portable as pure-HTTP JSON
//!
//! Every function below is backed by HTML scraping (`pd.read_html` /
//! BeautifulSoup) and/or `akshare.utils.demjson` (a JS-object decoder). There
//! is no clean JSON API to target, and the upstream pages require multi-page
//! pagination + `tqdm` crawls. Per the crate's DEFER policy they are recorded
//! here (and in `docs/MAPPING.md`) rather than faked.
//!
//! * `energy_carbon_domestic` (`energy_carbon.py:33`) — `demjson.decode` of a
//!   JS-padded response from `k.tanjiaoyi.com` (hardcoded `lcnK` token).
//! * `energy_carbon_bj` (`energy_carbon.py:76`) — multi-page `pd.read_html`
//!   crawl of `bjets.com.cn`.
//! * `energy_carbon_sz` (`energy_carbon.py:134`) — multi-page `pd.read_html`
//!   crawl of `cerx.cn` (国内碳情).
//! * `energy_carbon_eu` (`energy_carbon.py:166`) — multi-page `pd.read_html`
//!   crawl of `cerx.cn` (国际碳情).
//! * `energy_carbon_hb` (`energy_carbon.py:198`) — `demjson.decode` of an
//!   embedded `<script>` blob from `hbets.cn`.
//! * `energy_carbon_gz` (`energy_carbon.py:242`) — `pd.read_html` of
//!   `cnemission.com` market history.
//!
//! (`energy_oil_em.energy_oil_hist` / `energy_oil_detail` are Sina/news
//! endpoints tracked under the `spot`/`news` domains instead.)

pub mod energy_html_gaps;
