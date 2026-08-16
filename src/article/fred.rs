//! FRED-MD / FRED-QD monthly & quarterly macroeconomic databases.
//!
//! Ports `akshare/article/fred_md.py`. Both functions fetch a plain CSV from the
//! FRED McCracken database S3 bucket (exactly as akshare does) and parse it into
//! typed rows — no HTML / JS / token / Excel barriers, so both are FEASIBLE.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `fred_md` | `fred_md.py:13` | `.../fred-md/monthly/{date}.csv` |
//! | `fred_qd` | `fred_md.py:28` | `.../fred-md/quarterly/{date}.csv` |
//!
//! ## DEFERRED
//!
//! None.
//!
//! NOTE: the upstream S3 endpoint `s3.amazonaws.com/files.fred.stlouisfed.org/...`
//! currently returns HTTP 403 (AccessDenied) for anonymous GETs — the bucket now
//! blocks direct access. The parser below is faithful to akshare's CSV layout;
//! live fetches may fail until the endpoint is restored or mirrored. The offline
//! tests use synthetic `tests/fixtures/fred_md.csv` / `fred_qd.csv` that mirror
//! the exact CSV shape (first column = date, remaining columns = FRED series).

use std::collections::HashMap;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_FRED: &str = "fred";
const BASE_MONTHLY: &str = "https://s3.amazonaws.com/files.fred.stlouisfed.org/fred-md/monthly";
const BASE_QUARTERLY: &str = "https://s3.amazonaws.com/files.fred.stlouisfed.org/fred-md/quarterly";

/// One observation row of a FRED-MD/QD vintage: a date plus the full set of FRED
/// series values for that date (the CSV's remaining columns, keyed by series id).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FredMdRow {
    /// Observation date (first CSV column, e.g. `sasdate`).
    pub date: String,
    /// Series id -> value map (all CSV columns after the date column).
    pub series: HashMap<String, Option<f64>>,
}

/// Parse a FRED-MD/QD CSV text into typed rows. The first column is the date and
/// every remaining column is a FRED series. Pure (no I/O).
pub(crate) fn parse_fred_csv(text: &str) -> Result<Vec<FredMdRow>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(text.as_bytes());
    let headers = reader
        .headers()
        .map_err(|e| Error::UpstreamChanged {
            origin: SOURCE_FRED,
            message: format!("csv header: {e}"),
        })?
        .iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if headers.len() < 2 {
        return Err(Error::UpstreamChanged {
            origin: SOURCE_FRED,
            message: "expected at least a date column + one series column".into(),
        });
    }
    let series_cols = &headers[1..];
    let mut out = Vec::new();
    for rec in reader.records() {
        let rec = rec.map_err(|e| Error::UpstreamChanged {
            origin: SOURCE_FRED,
            message: format!("csv record: {e}"),
        })?;
        let date = rec.get(0).unwrap_or("").to_string();
        let mut series = HashMap::with_capacity(series_cols.len());
        for (i, name) in series_cols.iter().enumerate() {
            let v = rec.get(i + 1).and_then(|s| {
                let s = s.trim();
                if s.is_empty() {
                    None
                } else {
                    s.parse::<f64>().ok()
                }
            });
            series.insert(name.clone(), v);
        }
        out.push(FredMdRow { date, series });
    }
    Ok(out)
}

/// FRED-MD monthly data for the given vintage `date` (e.g. `"2020-01"`).
pub async fn fred_md(client: &Client, date: &str) -> Result<Vec<FredMdRow>> {
    let url = format!("{BASE_MONTHLY}/{date}.csv");
    let text = client
        .get_text(SOURCE_FRED, "fred_md", &url, &[], None)
        .await?;
    parse_fred_csv(&text)
}

/// FRED-QD quarterly data for the given vintage `date` (e.g. `"2020-01"`).
pub async fn fred_qd(client: &Client, date: &str) -> Result<Vec<FredMdRow>> {
    let url = format!("{BASE_QUARTERLY}/{date}.csv");
    let text = client
        .get_text(SOURCE_FRED, "fred_qd", &url, &[], None)
        .await?;
    parse_fred_csv(&text)
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
    fn parses_fred_md() {
        let rows = parse_fred_csv(&fixture_csv("fred_md.csv")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "1959-01");
        assert_eq!(rows[0].series.get("RPI"), Some(&Some(100.0)));
        assert_eq!(rows[0].series.get("INDPRO"), Some(&Some(12.3)));
        assert_eq!(rows[0].series.get("IPC"), Some(&Some(4.5)));
        assert_eq!(rows[0].series.get("CPU"), Some(&Some(2.1)));
        assert_eq!(rows[1].date, "1959-02");
        assert_eq!(rows[1].series.get("RPI"), Some(&Some(100.5)));
        // missing value parses to None
        assert_eq!(rows[1].series.get("CPU"), Some(&None));
    }

    #[test]
    fn parses_fred_qd() {
        let rows = parse_fred_csv(&fixture_csv("fred_qd.csv")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "1959-01");
        assert_eq!(rows[0].series.len(), 2);
        assert_eq!(rows[0].series.get("INDPRO"), Some(&Some(12.3)));
        assert_eq!(rows[1].date, "1959-02");
    }
}
