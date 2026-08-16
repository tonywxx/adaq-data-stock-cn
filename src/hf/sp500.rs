//! S&P 500 minute-bar high-frequency data. Ports `akshare/hf/hf_sp500.py`.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `hf_sp_500` | `hf_sp500.py:14` | downloads `DAT_ASCII_SPXUSD_M1_<year>.csv` (year 2012-2018) from the public FutureSharks/financial-data GitHub repo; semicolon-delimited, no header |
//!
//! The upstream is a plain `pandas.read_table(url, sep=";")` of a public CSV —
//! no JS signature, auth, HTML scraping, or Excel/ZIP. We fetch the raw CSV text
//! via [`Client::get_text`] and parse it with the `csv` crate (delimiter `;`).
//!
//! ## DEFERRED
//!
//! None. Every code path is feasible: the source is a public CSV URL.

use csv::ReaderBuilder;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE: &str = "github";
const BASE: &str =
    "https://github.com/FutureSharks/financial-data/raw/master/pyfinancialdata/data/stocks/histdata/SPXUSD/DAT_ASCII_SPXUSD_M1_{year}.csv";

/// One S&P 500 minute bar. Mirrors akshare's column order
/// `[date, open, high, low, close, price]` (`price` is the histdata volume field).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Sp500Row {
    /// Raw timestamp as delivered by histdata, e.g. `20170102 00:00:00.000`.
    pub date: String,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: Option<f64>,
    /// Volume (the 6th field in the upstream CSV).
    pub price: Option<f64>,
}

/// Parse S&P 500 minute bars from already-fetched CSV text (delimiter `;`,
/// no header). Pure — no I/O. Tolerant of blank/short lines.
pub(crate) fn parse_hf_sp_500(text: &str) -> Result<Vec<Sp500Row>> {
    let mut rdr = ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(false)
        .from_reader(text.as_bytes());
    let mut out = Vec::new();
    for rec in rdr.records() {
        let rec = rec.map_err(|e| Error::Parse {
            endpoint: "hf_sp_500",
            message: e.to_string(),
        })?;
        if rec.len() < 6 {
            continue;
        }
        let date = rec.get(0).unwrap_or_default().to_string();
        let open = rec.get(1).and_then(|s| s.parse::<f64>().ok());
        let high = rec.get(2).and_then(|s| s.parse::<f64>().ok());
        let low = rec.get(3).and_then(|s| s.parse::<f64>().ok());
        let close = rec.get(4).and_then(|s| s.parse::<f64>().ok());
        let price = rec.get(5).and_then(|s| s.parse::<f64>().ok());
        out.push(Sp500Row {
            date,
            open,
            high,
            low,
            close,
            price,
        });
    }
    Ok(out)
}

/// S&P 500 minute data for `year` (2012-2018), default `2017`.
pub async fn hf_sp_500(client: &Client, year: &str) -> Result<Vec<Sp500Row>> {
    let url = BASE.replace("{year}", year);
    let text = client
        .get_text(SOURCE, "hf_sp_500", &url, &[], None)
        .await?;
    parse_hf_sp_500(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        std::fs::read_to_string(p).unwrap()
    }

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    #[test]
    fn parse_hf_sp_500_ok() {
        // Synthetic fixture mirrors the upstream CSV shape (sep=';', no header):
        // DATE TIME;OPEN;HIGH;LOW;CLOSE;VOLUME
        let rows = parse_hf_sp_500(&fixture("hf_sp_500.csv")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].date, "20170102 00:00:00.000");
        assert!(approx(rows[0].open, 2270.45));
        assert!(approx(rows[0].high, 2271.30));
        assert!(approx(rows[0].low, 2269.80));
        assert!(approx(rows[0].close, 2270.90));
        assert!(approx(rows[0].price, 150.0));
        assert_eq!(rows[2].date, "20170102 00:02:00.000");
        assert!(approx(rows[2].close, 2271.80));
    }
}
