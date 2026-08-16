//! CFFEX daily trading data (`futures_hist_daily_cffex`).
//!
//! Ports akshare `futures_hist_daily_cffex`: the China Financial Futures
//! Exchange (CFFEX) publishes a per-day CSV at
//! `http://www.cffex.com.cn/sj/hqsj/rtj/{YYYYMM}/{DD}/{YYYYMMDD}_1.csv`.
//! akshare reads it as GBK; we fetch as text and parse CSV manually
//! (the response is a fixed 14-column layout). `date` is `YYYYMMDD`.

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// CFFEX daily trading row (`futures_hist_daily_cffex`).
///
/// akshare columns: symbol, date, open, high, low, close, volume,
/// open_interest, turnover, settle, pre_settle, variety.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesHistDailyCffexRow {
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

/// CFFEX daily trading data (`futures_hist_daily_cffex`).
///
/// `date` is `YYYYMMDD` (akshare default `20260403`).
pub async fn futures_hist_daily_cffex(client: &Client, date: &str) -> Result<Vec<FuturesHistDailyCffexRow>> {
    if date.len() != 8 || !date.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::InvalidParam("date must be YYYYMMDD".into()));
    }
    let ym = &date[0..6];
    let d = &date[6..8];
    let url = format!("http://www.cffex.com.cn/sj/hqsj/rtj/{ym}/{d}/{date}_1.csv");
    let text = client
        .get_text("cffex", "futures_hist_daily_cffex", &url, &[], None)
        .await?;
    parse_cffex_daily(&text, date)
}

/// Parse a CFFEX `rtj` CSV document into rows.
pub(crate) fn parse_cffex_daily(text: &str, date: &str) -> Result<Vec<FuturesHistDailyCffexRow>> {
    let lines: Vec<&str> = text.lines().map(|l| l.trim_end()).collect();
    // Line 0 is the header; data rows follow.
    let mut out = Vec::new();
    for line in lines.iter().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        let p: Vec<&str> = line.split(',').collect();
        if p.len() < 14 {
            continue;
        }
        let symbol = p[0].trim().to_string();
        // akshare drops 小计 / 合计 and option families IO/MO/HO.
        if symbol.is_empty()
            || symbol == "小计"
            || symbol == "合计"
            || symbol.contains("IO")
            || symbol.contains("MO")
            || symbol.contains("HO")
        {
            continue;
        }
        let variety = symbol
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect::<String>();
        out.push(FuturesHistDailyCffexRow {
            symbol,
            date: date.to_string(),
            open: parse_f64(p[1]),
            high: parse_f64(p[2]),
            low: parse_f64(p[3]),
            close: parse_f64(p[8]),
            volume: parse_f64(p[4]),
            open_interest: parse_f64(p[6]),
            turnover: parse_f64(p[5]),
            settle: parse_f64(p[9]),
            pre_settle: parse_f64(p[10]),
            variety,
        });
    }
    Ok(out)
}

fn parse_f64(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        t.parse::<f64>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_cffex_daily_fixture() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/futures_hist_daily_cffex.csv");
        let txt = std::fs::read_to_string(path).unwrap();
        let rows = parse_cffex_daily(&txt, "20260403").unwrap();
        // 小计 row is dropped.
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "IF2503");
        assert_eq!(rows[0].date, "20260403");
        assert_eq!(rows[0].open, Some(3000.0));
        assert_eq!(rows[0].high, Some(3080.0));
        assert_eq!(rows[0].low, Some(2980.0));
        assert_eq!(rows[0].close, Some(3050.0));
        assert_eq!(rows[0].volume, Some(120000.0));
        assert_eq!(rows[0].open_interest, Some(150000.0));
        assert_eq!(rows[0].turnover, Some(3.6e9));
        assert_eq!(rows[0].settle, Some(3040.0));
        assert_eq!(rows[0].pre_settle, Some(3030.0));
        assert_eq!(rows[0].variety, "IF");
        assert_eq!(rows[1].symbol, "IC2503");
    }
}
