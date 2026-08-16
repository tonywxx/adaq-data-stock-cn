use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};
use crate::stock::intraday::IntradayRow;

const UT: &str = "bd1d9ddb04089700cf9c27f6f7426281";
const BASE: &str = "https://70.push2.eastmoney.com/api/qt/stock/details/sse";

/// Intraday tick (time & sales) data from Eastmoney (`stock_intraday_em`).
///
/// Eastmoney streams `data: {...}` SSE frames, each containing `data.details` — an
/// array of comma-joined strings `time,price,volume,-,direction`. We parse every
/// frame and concatenate. `direction` maps 1→卖盘, 2→买盘, 4→中性盘 (per akshare).
pub async fn em(client: &Client, symbol: &str) -> Result<Vec<IntradayRow>> {
    let market_code: &str = if symbol.starts_with('6') { "1" } else { "0" };
    let secid = format!("{market_code}.{symbol}");
    let params = [
        ("fields1", "f1,f2,f3,f4"),
        ("fields2", "f51,f52,f53,f54,f55"),
        ("mpi", "2000"),
        ("ut", UT),
        ("fltt", "2"),
        ("pos", "-0"),
        ("secid", &secid),
        ("wbp2u", "|0|0|0|web"),
    ];
    let text = client
        .get_text(SOURCE_EASTMONEY, "stock_intraday_em", BASE, &params, None)
        .await?;
    parse_stream(&text, symbol)
}

/// Split an SSE body into `data: {...}` frames and parse each.
pub(crate) fn parse_stream(text: &str, symbol: &str) -> Result<Vec<IntradayRow>> {
    let mut out = Vec::new();
    for line in text.split('\n') {
        let trimmed = line.trim_start().strip_prefix("data:").map(str::trim);
        let Some(body) = trimmed else { continue };
        if body.is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(body).map_err(|e| Error::Parse {
            endpoint: "stock_intraday_em",
            message: e.to_string(),
        })?;
        out.extend(parse_details(&v, symbol)?);
    }
    Ok(out)
}

/// Parse the `data.details` array of one SSE frame into [`IntradayRow`]s.
pub(crate) fn parse_details(event: &Value, symbol: &str) -> Result<Vec<IntradayRow>> {
    let details = event
        .get("data")
        .and_then(|d| d.get("details"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.details".into(),
        })?;
    let mut out = Vec::with_capacity(details.len());
    for item in details {
        let s = item.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "detail entry is not a string".into(),
        })?;
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() < 5 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: format!("detail has {} fields, expected >= 5", parts.len()),
            });
        }
        out.push(IntradayRow {
            symbol: symbol.to_string(),
            time: parts[0].to_string(),
            price: parse_f64(parts[1]),
            volume: parse_f64(parts[2]),
            direction: map_direction(parts[4]),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

fn map_direction(code: &str) -> Option<String> {
    match code {
        "1" => Some("卖盘".into()),
        "2" => Some("买盘".into()),
        "4" => Some("中性盘".into()),
        _ => None,
    }
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
    fn parses_eastmoney_intraday_fixture() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/stock_intraday_em.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let rows = parse_stream(&txt, "600000").unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].time, "2025-01-02 09:30:00");
        assert_eq!(rows[0].price, Some(10.10));
        assert_eq!(rows[0].volume, Some(100.0));
        assert_eq!(rows[0].direction, Some("买盘".into()));
        assert_eq!(rows[1].direction, Some("卖盘".into()));
        assert_eq!(rows[2].direction, Some("中性盘".into()));
        assert_eq!(rows[0].source, "eastmoney");
    }
}
