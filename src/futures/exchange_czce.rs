//! Zhengzhou Commodity Exchange (CZCE) daily trading data.
//!
//! Ports `get_czce_daily` ← `futures_daily_bar.py:341`.
//!
//! CZCE publishes a pipe-delimited text file
//! (`FutureDataDaily.txt`); akshare splits it on `|` and maps the fixed
//! column order onto `OUTPUT_COLUMNS`. No JS signing / HTML scrape / Excel.
//!
//! ## DEFERRED
//! None in this file.

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE: &str = "czce";

/// One CZCE daily trading row (`get_czce_daily`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CzceDailyRow {
    pub symbol: String,
    pub date: String,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: Option<f64>,
    pub volume: Option<f64>,
    pub open_interest: Option<f64>,
    pub turnover: Option<f64>,
    pub settle: Option<f64>,
    pub pre_settle: Option<f64>,
    pub variety: String,
}

fn to_f64(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', "");
    if t.is_empty() || t == "\r" {
        None
    } else {
        t.parse::<f64>().ok()
    }
}

/// Extract the 1–2 leading letters of a CZCE symbol (`^[A-Za-z]{1,2}[0-9]+`).
fn czce_variety(symbol: &str) -> Option<String> {
    let bytes = symbol.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
        i += 1;
    }
    if i == 0 || i > 2 || i >= bytes.len() {
        return None;
    }
    for b in &bytes[i..] {
        if !b.is_ascii_digit() {
            return None;
        }
    }
    Some(symbol[..i].to_string())
}

/// Parse `get_czce_daily` rows from the raw pipe-delimited text.
///
/// Mirrors akshare's `date > 2015-11-11` branch: drop the last 3 lines,
/// skip lines starting with `小`, then take the header (line 1) and data
/// (line 2+).
pub(crate) fn parse_czce_daily(text: &str, date: &str) -> Result<Vec<CzceDailyRow>> {
    let trimmed = text.trim_end();
    let lines: Vec<&str> = trimmed.split('\n').collect();
    if lines.len() < 5 {
        // not enough content (header + data); akshare returns empty
        return Ok(Vec::new());
    }
    let kept = if lines.len() >= 3 {
        &lines[..lines.len() - 3]
    } else {
        &lines[..]
    };
    let mut cells: Vec<Vec<String>> = Vec::with_capacity(kept.len());
    for line in kept {
        if line.is_empty() || line.chars().next().map(|c| c == '小').unwrap_or(false) {
            continue;
        }
        cells.push(line.replace(' ', "").split('|').map(|s| s.to_string()).collect());
    }
    if cells.len() < 3 {
        return Ok(Vec::new());
    }
    let header0 = cells[1].first().map(|s| s.as_str()).unwrap_or("");
    if !["品种月份", "品种代码", "合约代码"].contains(&header0) {
        return Ok(Vec::new());
    }
    // CZCE_COLUMNS order in the data row (row[0] = symbol):
    // pre_settle, open, high, low, close, settle, change1, change2,
    // volume, open_interest, oi_chg, turnover, final_settle
    let mut out = Vec::new();
    for row in &cells[2..] {
        if row.len() < 14 {
            continue;
        }
        let symbol = row[0].clone();
        let variety = match czce_variety(&symbol) {
            Some(v) => v,
            None => continue,
        };
        let num = |i: usize| to_f64(&row[i]);
        out.push(CzceDailyRow {
            symbol,
            date: date.to_string(),
            open: num(2),
            high: num(3),
            low: num(4),
            close: num(5),
            volume: num(9),
            open_interest: num(10),
            turnover: num(12),
            settle: num(6),
            pre_settle: num(1),
            variety,
        });
    }
    Ok(out)
}

/// CZCE daily trading data (`get_czce_daily`). `date` is `YYYYMMDD`.
pub async fn get_czce_daily(client: &Client, date: &str) -> Result<Vec<CzceDailyRow>> {
    if date.len() != 8 || !date.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::InvalidParam("date must be YYYYMMDD".into()));
    }
    let y = &date[0..4];
    let url = format!(
        "http://www.czce.com.cn/cn/DFSStaticFiles/Future/{y}/{date}/FutureDataDaily.txt"
    );
    let text = client.get_text(SOURCE, "get_czce_daily", &url, &[], None).await?;
    parse_czce_daily(&text, date)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture_text(name: &str) -> String {
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
    fn parse_czce_daily_ok() {
        let rows = parse_czce_daily(&fixture_text("get_czce_daily.txt"), "20250205").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "CF001");
        assert_eq!(rows[0].variety, "CF");
        assert_eq!(rows[0].date, "20250205");
        assert!(approx(rows[0].open, 101.0));
        assert!(approx(rows[0].high, 102.0));
        assert!(approx(rows[0].low, 99.0));
        assert!(approx(rows[0].close, 100.0));
        assert!(approx(rows[0].settle, 100.5));
        assert!(approx(rows[0].pre_settle, 100.0));
        assert!(approx(rows[0].volume, 1234.0));
        assert!(approx(rows[0].open_interest, 5678.0));
        assert!(approx(rows[0].turnover, 123456.0));
    }
}
