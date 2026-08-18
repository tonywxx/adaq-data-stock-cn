//! Option implied-volatility index (QVIX) from optbbs.com.
//!
//! Ported from `akshare/index/index_option_qvix.py`. optbbs serves two plain
//! CSV feeds, both fetched over HTTP with no JS/token:
//!
//! * Daily: `http://1.optbbs.com/d/csv/d/k.csv` — a single wide CSV (≈87
//!   columns) covering every underlying. Each daily function selects a specific
//!   set of column indices and renames them to `date,o,h,l,c`.
//! * Minute: `http://1.optbbs.com/d/csv/d/vix<X>.csv` — one file per underlying,
//!   two columns `time,qvix` (akshare keeps `iloc[:, :2]`).
//!
//! The daily CSV is GBK-encoded upstream; only ASCII columns (the date and the
//! numeric OHLC fields) are consumed, so a lossy UTF-8 decode via `get_text`
//! preserves them. All 18 functions return [`QvixRow`]; minute rows populate
//! `date` (= time) and `close` (= qvix) while `open/high/low` are `None`.
//!
//! Ported functions (line = `akshare/index/index_option_qvix.py`):
//! * [`index_option_50etf_qvix`] (L28), [`index_option_50etf_min_qvix`] (L51)
//! * [`index_option_300etf_qvix`] (L68), [`index_option_300etf_min_qvix`] (L91)
//! * [`index_option_500etf_qvix`] (L108), [`index_option_500etf_min_qvix`] (L131)
//! * [`index_option_cyb_qvix`] (L148), [`index_option_cyb_min_qvix`] (L171)
//! * [`index_option_kcb_qvix`] (L188), [`index_option_kcb_min_qvix`] (L211)
//! * [`index_option_100etf_qvix`] (L228), [`index_option_100etf_min_qvix`] (L251)
//! * [`index_option_300index_qvix`] (L268), [`index_option_300index_min_qvix`] (L291)
//! * [`index_option_1000index_qvix`] (L308), [`index_option_1000index_min_qvix`] (L331)
//! * [`index_option_50index_qvix`] (L348), [`index_option_50index_min_qvix`] (L371)

use csv::ReaderBuilder;

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use crate::core::json::*;

/// Upstream source identifier for optbbs.com.
const SOURCE_OPTBBS: &str = "optbbs";

/// Daily wide CSV covering every underlying (akshare `__get_optbbs_daily`).
const K_CSV_URL: &str = "http://1.optbbs.com/d/csv/d/k.csv";

/// Base URL for the per-underlying minute CSVs (akshare `vix<X>.csv`).
const MINUTE_CSV_BASE: &str = "http://1.optbbs.com/d/csv/d/";

/// One QVIX observation.
///
/// For daily feeds `date` is the trading date and `open/high/low/close` are the
/// VIX OHLC. For minute feeds `date` holds the timestamp, `close` holds the VIX
/// value, and `open/high/low` are `None` (the upstream only publishes `qvix`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct QvixRow {
    /// Daily: trading date. Minute: `time` timestamp.
    pub date: String,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: Option<f64>,
    pub source: &'static str,
}

/// Which feed + selector a public function maps to.
enum QvixKind {
    /// Daily feed, with the 5 column indices `[date, open, high, low, close]`.
    Daily([usize; 5]),
    /// Minute feed, with the upstream CSV file name (e.g. `vix50.csv`).
    Minute(&'static str),
}

/// Single fetch+parse dispatcher shared by all 18 public functions.
async fn fetch_qvix(client: &Client, kind: QvixKind) -> Result<Vec<QvixRow>> {
    match kind {
        QvixKind::Daily(cols) => fetch_daily(client, cols).await,
        QvixKind::Minute(file) => fetch_minute(client, file).await,
    }
}

/// Fetch the daily `k.csv` and select the requested column slice.
async fn fetch_daily(client: &Client, cols: [usize; 5]) -> Result<Vec<QvixRow>> {
    let text = client
        .get_text(SOURCE_OPTBBS, "qvix_daily", K_CSV_URL, &[], None)
        .await?;
    let parsed = parse_k_csv(&text)?;
    Ok(parsed.iter().map(|r| select_daily(r, cols)).collect())
}

/// Fetch one minute CSV and parse `time`/`qvix` into [`QvixRow`]s.
async fn fetch_minute(client: &Client, file: &str) -> Result<Vec<QvixRow>> {
    let url = format!("{MINUTE_CSV_BASE}{file}");
    let text = client
        .get_text(SOURCE_OPTBBS, "qvix_minute", &url, &[], None)
        .await?;
    parse_minute(&text)
}

/// Parse the wide daily CSV into raw string rows (header consumed).
pub(crate) fn parse_k_csv(text: &str) -> Result<Vec<Vec<String>>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(text.as_bytes());
    let mut out = Vec::new();
    for rec in rdr.records() {
        let rec = rec.map_err(|e| Error::Csv(e.to_string()))?;
        out.push(rec.iter().map(|f| f.to_string()).collect());
    }
    Ok(out)
}

/// Project a raw daily row onto [`QvixRow`] using the given column indices.
pub(crate) fn select_daily(row: &[String], cols: [usize; 5]) -> QvixRow {
    let get = |i: usize| row.get(i).map(|s| s.as_str()).unwrap_or("");
    QvixRow {
        date: get(cols[0]).to_string(),
        open: parse_f64_str(get(cols[1])),
        high: parse_f64_str(get(cols[2])),
        low: parse_f64_str(get(cols[3])),
        close: parse_f64_str(get(cols[4])),
        source: SOURCE_OPTBBS,
    }
}

/// Parse a minute CSV (`time,qvix`) into [`QvixRow`]s.
pub(crate) fn parse_minute(text: &str) -> Result<Vec<QvixRow>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(text.as_bytes());
    let mut out = Vec::new();
    for rec in rdr.records() {
        let rec = rec.map_err(|e| Error::Csv(e.to_string()))?;
        out.push(QvixRow {
            date: rec.get(0).unwrap_or("").to_string(),
            open: None,
            high: None,
            low: None,
            close: parse_f64_str(rec.get(1).unwrap_or("")),
            source: SOURCE_OPTBBS,
        });
    }
    Ok(out)
}

/// Generate the 18 public async functions, each delegating to [`fetch_qvix`].
macro_rules! define_qvix_fns {
    ( $( ($name:ident, $doc:literal, $kind:ident, $arg:expr) ),+ $(,)? ) => {
        $(
            #[doc = $doc]
            pub async fn $name(client: &Client) -> Result<Vec<QvixRow>> {
                fetch_qvix(client, QvixKind::$kind($arg)).await
            }
        )+
    };
}

define_qvix_fns! {
    (index_option_50etf_qvix, "50ETF 期权波动率指数 QVIX (daily). akshare/index/index_option_qvix.py:28", Daily, [0, 1, 2, 3, 4]),
    (index_option_50etf_min_qvix, "50 ETF 期权波动率指数 QVIX-分时 (minute). akshare/index/index_option_qvix.py:51", Minute, "vix50.csv"),

    (index_option_300etf_qvix, "300 ETF 期权波动率指数 QVIX (daily). akshare/index/index_option_qvix.py:68", Daily, [0, 9, 10, 11, 12]),
    (index_option_300etf_min_qvix, "300 ETF 期权波动率指数 QVIX-分时 (minute). akshare/index/index_option_qvix.py:91", Minute, "vix300.csv"),

    (index_option_500etf_qvix, "500 ETF 期权波动率指数 QVIX (daily). akshare/index/index_option_qvix.py:108", Daily, [0, 67, 68, 69, 70]),
    (index_option_500etf_min_qvix, "500 ETF 期权波动率指数 QVIX-分时 (minute). akshare/index/index_option_qvix.py:131", Minute, "vix500.csv"),

    (index_option_cyb_qvix, "创业板 期权波动率指数 QVIX (daily). akshare/index/index_option_qvix.py:148", Daily, [0, 71, 72, 73, 74]),
    (index_option_cyb_min_qvix, "创业板 期权波动率指数 QVIX-分时 (minute). akshare/index/index_option_qvix.py:171", Minute, "vixcyb.csv"),

    (index_option_kcb_qvix, "科创板 期权波动率指数 QVIX (daily). akshare/index/index_option_qvix.py:188", Daily, [0, 83, 84, 85, 86]),
    (index_option_kcb_min_qvix, "科创板 期权波动率指数 QVIX-分时 (minute). akshare/index/index_option_qvix.py:211", Minute, "vixkcb.csv"),

    (index_option_100etf_qvix, "深证100ETF 期权波动率指数 QVIX (daily). akshare/index/index_option_qvix.py:228", Daily, [0, 75, 76, 77, 78]),
    (index_option_100etf_min_qvix, "深证100ETF 期权波动率指数 QVIX-分时 (minute). akshare/index/index_option_qvix.py:251", Minute, "vix100.csv"),

    (index_option_300index_qvix, "中证300股指 期权波动率指数 QVIX (daily). akshare/index/index_option_qvix.py:268", Daily, [0, 17, 18, 19, 20]),
    (index_option_300index_min_qvix, "中证300股指 期权波动率指数 QVIX-分时 (minute). akshare/index/index_option_qvix.py:291", Minute, "vixindex.csv"),

    (index_option_1000index_qvix, "中证1000股指 期权波动率指数 QVIX (daily). akshare/index/index_option_qvix.py:308", Daily, [0, 25, 26, 27, 28]),
    (index_option_1000index_min_qvix, "中证1000股指 期权波动率指数 QVIX-分时 (minute). akshare/index/index_option_qvix.py:331", Minute, "vixindex1000.csv"),

    (index_option_50index_qvix, "上证50股指 期权波动率指数 QVIX (daily). akshare/index/index_option_qvix.py:348", Daily, [0, 79, 80, 81, 82]),
    (index_option_50index_min_qvix, "上证50股指 期权波动率指数 QVIX-分时 (minute). akshare/index/index_option_qvix.py:371", Minute, "vix50index.csv"),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_csv(name: &str) -> String {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        std::fs::read_to_string(p).unwrap()
    }

    #[test]
    fn parses_qvix_daily_fixture() {
        let text = fixture_csv("qvix_daily.csv");
        let rows = parse_k_csv(&text).unwrap();
        assert_eq!(rows.len(), 2);

        // 50ETF uses columns [0,1,2,3,4]
        let r0 = select_daily(&rows[0], [0, 1, 2, 3, 4]);
        assert_eq!(r0.date, "2024-01-02");
        assert_eq!(r0.open, Some(18.5));
        assert_eq!(r0.high, Some(19.2));
        assert_eq!(r0.low, Some(18.1));
        assert_eq!(r0.close, Some(18.9));
        assert_eq!(r0.source, SOURCE_OPTBBS);

        let r1 = select_daily(&rows[1], [0, 1, 2, 3, 4]);
        assert_eq!(r1.date, "2024-01-03");
        assert_eq!(r1.open, None); // empty cell coerces to None
        assert_eq!(r1.high, Some(19.0));
        assert_eq!(r1.low, Some(18.0));
        assert_eq!(r1.close, Some(18.7));

        // 300index uses columns [0,17,18,19,20] — confirms index-based selection
        let i0 = select_daily(&rows[0], [0, 17, 18, 19, 20]);
        assert_eq!(i0.open, Some(20.1));
        assert_eq!(i0.high, Some(21.3));
        assert_eq!(i0.low, Some(19.8));
        assert_eq!(i0.close, Some(20.5));
        let i1 = select_daily(&rows[1], [0, 17, 18, 19, 20]);
        assert_eq!(i1.open, None);
    }

    #[test]
    fn parses_qvix_minute_fixture() {
        let text = fixture_csv("qvix_minute.csv");
        let rows = parse_minute(&text).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02 09:30");
        assert_eq!(rows[0].close, Some(18.50));
        assert_eq!(rows[0].open, None);
        assert_eq!(rows[1].date, "2024-01-02 09:31");
        assert_eq!(rows[1].close, Some(18.62));
        assert_eq!(rows[1].source, SOURCE_OPTBBS);
    }
}
