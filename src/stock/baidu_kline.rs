//! 百度股市通 K 线（含 MA5/10/20）— ports `baidu_kline_with_ma` from the
//! `simonlin1212/a-stock-data` skill.
//!
//! Returns OHLC + volume + amount + MA5/10/20 columns. Baidu expects the plain
//! numeric code (`600519`), not a market-prefixed one.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_BAIDU: &str = "baidu";
const BAIDU_KLINE_URL: &str = "https://finance.pae.baidu.com/selfselect/getstockquotation";

/// One daily bar from Baidu (with MA5/10/20 when the window is warm).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BaiduKlineRow {
    /// `time` (YYYY-MM-DD)
    pub date: String,
    pub open: Option<f64>,
    pub close: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub volume: Option<f64>,
    pub amount: Option<f64>,
    /// `ma5avgprice`
    pub ma5: Option<f64>,
    /// `ma10avgprice`
    pub ma10: Option<f64>,
    /// `ma20avgprice`
    pub ma20: Option<f64>,
    pub source: &'static str,
}

/// Port of `baidu_kline_with_ma(code, start_time)`.
///
/// `code` may be market-prefixed (`sh600519`) or plain (`600519`); Baidu wants
/// the plain numeric code, so any market prefix is stripped. `start_time` is an
/// optional `YYYY-MM-DD` lower bound (empty = full history).
pub async fn baidu_kline_with_ma(
    client: &Client,
    code: &str,
    start_time: &str,
) -> Result<Vec<BaiduKlineRow>> {
    let plain = strip_prefix(code);
    let params = [
        ("all", "1"),
        ("isIndex", "false"),
        ("isBk", "false"),
        ("isBlock", "false"),
        ("isFutures", "false"),
        ("isStock", "true"),
        ("newFormat", "1"),
        ("group", "quotation_kline_ab"),
        ("finClientType", "pc"),
        ("code", plain.as_str()),
        ("start_time", start_time),
        ("ktype", "1"),
    ];
    let v = client
        .get_json_with_headers(
            SOURCE_BAIDU,
            "baidu_kline_with_ma",
            BAIDU_KLINE_URL,
            &params,
            Some(&[
                ("Accept", "application/vnd.finance-web.v1+json"),
                ("Origin", "https://gushitong.baidu.com"),
                ("Referer", "https://gushitong.baidu.com/"),
            ]),
        )
        .await?;
    parse_baidu_kline(&v)
}

/// Strip a 2-char market prefix (`sh`/`sz`/`bj`, any case) if present.
fn strip_prefix(code: &str) -> String {
    let c = code.trim();
    if c.len() > 2 {
        let (p, rest) = c.split_at(2);
        if matches!(p.to_ascii_lowercase().as_str(), "sh" | "sz" | "bj")
            && rest.chars().all(|x| x.is_ascii_digit())
        {
            return rest.to_string();
        }
    }
    c.to_string()
}

/// Parse a Baidu `Result.newMarketData` object into [`BaiduKlineRow`]s.
///
/// `marketData` is a `;`-delimited string; each row is `,`-delimited and aligned
/// positionally to `keys`. `--` means "no data yet" (early MA columns).
pub(crate) fn parse_baidu_kline(resp: &Value) -> Result<Vec<BaiduKlineRow>> {
    let md = resp
        .get("Result")
        .and_then(|r| r.get("newMarketData"))
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_BAIDU,
            message: "missing Result.newMarketData".into(),
        })?;
    let keys = md
        .get("keys")
        .and_then(|k| k.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_BAIDU,
            message: "missing keys".into(),
        })?;
    let pos = |name: &str| keys.iter().position(|k| k.as_str() == Some(name));
    let i_time = pos("time");
    let i_open = pos("open");
    let i_close = pos("close");
    let i_high = pos("high");
    let i_low = pos("low");
    let i_vol = pos("volume");
    let i_amt = pos("amount");
    let i_ma5 = pos("ma5avgprice");
    let i_ma10 = pos("ma10avgprice");
    let i_ma20 = pos("ma20avgprice");
    let market = md
        .get("marketData")
        .and_then(|m| m.as_str())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_BAIDU,
            message: "missing marketData".into(),
        })?;
    let cell = |cols: &[&str], i: Option<usize>| -> Option<f64> {
        i.and_then(|i| cols.get(i).copied())
            .filter(|s| *s != "--")
            .and_then(|s| s.parse::<f64>().ok())
    };
    let mut out = Vec::new();
    for row in market.split(';') {
        if row.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = row.split(',').collect();
        let date = i_time
            .and_then(|i| cols.get(i))
            .map(|s| s.to_string())
            .unwrap_or_default();
        if date.is_empty() {
            continue;
        }
        out.push(BaiduKlineRow {
            date,
            open: cell(&cols, i_open),
            close: cell(&cols, i_close),
            high: cell(&cols, i_high),
            low: cell(&cols, i_low),
            volume: cell(&cols, i_vol),
            amount: cell(&cols, i_amt),
            ma5: cell(&cols, i_ma5),
            ma10: cell(&cols, i_ma10),
            ma20: cell(&cols, i_ma20),
            source: SOURCE_BAIDU,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_baidu_kline_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/baidu_kline.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_baidu_kline(&v).unwrap();
        assert!(rows.len() > 1000);
        assert_eq!(rows[0].date, "2018-06-04");
        assert_eq!(rows[0].close, Some(497.17));
        // early rows have no warm MA window
        assert!(rows[0].ma5.is_none());
        assert!(rows[0].ma20.is_none());
        // later rows do
        let last = rows.last().unwrap();
        assert!(last.ma20.is_some());
        assert!(last.close.is_some());
    }

    #[test]
    fn strips_market_prefix() {
        assert_eq!(strip_prefix("sh600519"), "600519");
        assert_eq!(strip_prefix("SZ000001"), "000001");
        assert_eq!(strip_prefix("600519"), "600519");
    }
}
