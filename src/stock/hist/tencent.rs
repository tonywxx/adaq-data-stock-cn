use serde_json::Value;

use crate::core::client::{Client, SOURCE_TENCENT};
use crate::core::error::{Error, Result};
use crate::stock::hist::HistRow;

const BASE: &str = "https://proxy.finance.qq.com/ifzqgtimg/appstock/app/newfqkline/get";

/// Per-symbol historical OHLC from Tencent (`stock_zh_a_hist_tx`).
///
/// Tencent returns per-year kline arrays; we walk the year range implied by
/// `start_date`/`end_date`, mirroring akshare's per-year loop. Tencent reports
/// `volume` in lots and `amount` in 万元 for most boards; we reconcile to shares /
/// CNY (volume×100, amount×10000) for everything except 科创板 (sh688) and the index
/// sh000*/sz000* families, matching akshare's final output.
pub async fn daily(
    client: &Client,
    symbol: &str,
    period: &str,
    adjust: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<HistRow>> {
    if period != "daily" {
        return Err(Error::InvalidParam(format!(
            "tencent hist only supports daily period, got: {period}"
        )));
    }
    let symbol = normalize_tx_symbol(symbol);
    let start_year = year_of(start_date)?;
    let end_year = year_of(end_date)?;
    let range_end = end_year + 1;

    let mut out = Vec::new();
    for year in start_year..range_end {
        let next_s = (year + 1).to_string();
        let var = format!("kline_day{adjust}{year}");
        let param = format!("{symbol},day,{year}-01-01,{next_s}-12-31,640,{adjust}");
        let params = [
            ("_var", var.as_str()),
            ("param", param.as_str()),
            ("r", "0.8205512681390605"),
        ];
        let v = client
            .get_json(SOURCE_TENCENT, "stock_zh_a_hist_tx", BASE, &params)
            .await?;
        out.extend(parse_klines(&v, &symbol, start_date, end_date)?);
    }
    Ok(out)
}

pub(crate) fn parse_klines(
    resp: &Value,
    symbol: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<HistRow>> {
    let node = resp
        .get("data")
        .and_then(|d| d.get(symbol))
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_TENCENT,
            message: format!("missing data.{symbol}", symbol = symbol),
        })?;
    let rows = node
        .get("day")
        .or_else(|| node.get("hfqday"))
        .or_else(|| node.get("qfqday"))
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_TENCENT,
            message: "missing day/hfqday/qfqday".into(),
        })?;

    let scale_volume = !symbol.starts_with("sh688")
        && !symbol.starts_with("sz399")
        && !symbol.starts_with("sh000")
        && !symbol.starts_with("sz000");

    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let arr = r.as_array().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_TENCENT,
            message: "day entry is not an array".into(),
        })?;
        if arr.len() < 9 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_TENCENT,
                message: format!("day entry has {} fields, expected >= 9", arr.len()),
            });
        }
        let date = str_at(arr, 0);
        if !in_range(&date, start_date, end_date) {
            continue;
        }
        let volume = num_at(arr, 5).map(|v| if scale_volume { v * 100.0 } else { v });
        let amount = num_at(arr, 8).map(|v| v * 10000.0);
        out.push(HistRow {
            symbol: symbol.to_string(),
            date,
            open: num_at(arr, 1),
            close: num_at(arr, 2),
            high: num_at(arr, 3),
            low: num_at(arr, 4),
            volume,
            amount,
            pct_change: None,
            source: SOURCE_TENCENT,
        });
    }
    Ok(out)
}

/// Normalize a bare or prefixed code to Tencent's `market+code` form (ADR-0005).
pub(crate) fn normalize_tx_symbol(symbol: &str) -> String {
    let s = symbol.trim().to_lowercase();
    if s.starts_with("sh") || s.starts_with("sz") || s.starts_with("bj") {
        return s;
    }
    if s.starts_with("600")
        || s.starts_with("601")
        || s.starts_with("603")
        || s.starts_with("605")
        || s.starts_with("688")
        || s.starts_with("900")
    {
        return format!("sh{s}");
    }
    if s.starts_with("000")
        || s.starts_with("001")
        || s.starts_with("002")
        || s.starts_with("003")
        || s.starts_with("200")
        || s.starts_with("300")
        || s.starts_with("301")
    {
        return format!("sz{s}");
    }
    if s.starts_with("430")
        || s.starts_with("440")
        || s.starts_with("830")
        || s.starts_with("831")
        || s.starts_with("832")
        || s.starts_with("833")
        || s.starts_with("839")
    {
        return format!("bj{s}");
    }
    s
}

fn year_of(date: &str) -> Result<i32> {
    let digits: String = date.chars().filter(|c| c.is_ascii_digit()).collect();
    digits
        .get(..4)
        .and_then(|y| y.parse::<i32>().ok())
        .ok_or_else(|| Error::InvalidParam(format!("invalid date: {date}")))
}

/// Lexicographic ISO-date range check (dates are `YYYY-MM-DD`).
fn in_range(date: &str, start: &str, end: &str) -> bool {
    let s = normalize_date(start);
    let e = normalize_date(end);
    date >= s.as_str() && date <= e.as_str()
}

fn normalize_date(d: &str) -> String {
    d.chars().filter(|c| c.is_ascii_digit()).collect()
}

fn str_at(arr: &[Value], i: usize) -> String {
    arr.get(i)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn num_at(arr: &[Value], i: usize) -> Option<f64> {
    match arr.get(i)? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_tencent_hist_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/stock_zh_a_hist_tx.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        // sz000001 -> volume NOT scaled, amount * 10000
        let rows = parse_klines(&v, "sz000001", "20000101", "21000101").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2025-01-02");
        assert_eq!(rows[0].open, Some(10.10));
        assert_eq!(rows[0].close, Some(10.50));
        assert_eq!(rows[0].high, Some(10.80));
        assert_eq!(rows[0].low, Some(9.90));
        assert_eq!(rows[0].volume, Some(1234.0));
        assert_eq!(rows[0].amount, Some(130.0 * 10000.0));
        assert_eq!(rows[0].source, "tencent");
        assert_eq!(rows[1].date, "2025-01-03");
    }

    #[test]
    fn normalizes_tx_symbols() {
        assert_eq!(normalize_tx_symbol("600000"), "sh600000");
        assert_eq!(normalize_tx_symbol("000001"), "sz000001");
        assert_eq!(normalize_tx_symbol("sz000001"), "sz000001");
    }
}
