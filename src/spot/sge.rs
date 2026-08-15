//! Shanghai Gold Exchange (SGE) data ported from `akshare/spot/spot_sge.py`.
//!
//! Source functions and their akshare line numbers:
//!
//! | Rust fn                     | akshare fn                   | source line |
//! | --------------------------- | ---------------------------- | ----------- |
//! | `spot_symbol_table_sge`     | `spot_symbol_table_sge`      | 17          |
//! | `spot_quotations_sge`       | `spot_quotations_sge`        | 50          |
//! | `spot_hist_sge`             | `spot_hist_sge`              | 109         |
//! | `spot_golden_benchmark_sge` | `spot_golden_benchmark_sge`  | 163         |
//! | `spot_silver_benchmark_sge` | `spot_silver_benchmark_sge`  | 194         |
//!
//! All five functions return JSON directly from `www.sge.com.cn` (no HTML
//! scraping, JS rendering, token, or Excel download), so every function is
//! implemented.
//!
//! ## DEFERRED
//!
//! None. No SGE function in `spot_sge.py` requires HTML scraping, JS execution,
//! an auth token, or an Excel download, so nothing is deferred.

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use serde_json::Value;

const SOURCE_SGE: &str = "sge";

const QUOTATIONS_URL: &str = "https://www.sge.com.cn/graph/quotations";
const DAILYHQ_URL: &str = "https://www.sge.com.cn/graph/Dailyhq";
const GOLDEN_BENCHMARK_URL: &str = "https://www.sge.com.cn/graph/DayilyJzj";
const SILVER_BENCHMARK_URL: &str = "https://www.sge.com.cn/graph/DayilyShsilverJzj";

const SGE_HEADERS: &[(&str, &str)] = &[
    ("Referer", "https://www.sge.com.cn/"),
    ("Origin", "https://www.sge.com.cn"),
    ("X-Requested-With", "XMLHttpRequest"),
];

/// Static symbol table of SGE instruments (from `spot_symbol_table_sge`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SymbolRow {
    pub index: u32,
    pub symbol: String,
}

/// Real-time quotation row for a single SGE instrument (from `spot_quotations_sge`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct QuotationRow {
    pub symbol: String,
    pub time: String,
    pub price: Option<f64>,
    pub update_time: String,
}

/// Historical daily OHLC row for a single SGE instrument (from `spot_hist_sge`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct HistRow {
    pub date: String,
    pub open: Option<f64>,
    pub close: Option<f64>,
    pub low: Option<f64>,
    pub high: Option<f64>,
}

/// Benchmark (Shanghai Gold / Silver) price row (from `spot_*_benchmark_sge`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BenchmarkRow {
    pub trade_date: String,
    pub evening_price: Option<f64>,
    pub morning_price: Option<f64>,
}

/// Extract a string field `k` from a JSON object, if present and a string.
pub fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(Value::as_str).map(String::from)
}

/// Extract a numeric field `k` from a JSON object, if present and a number.
pub fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(Value::as_f64)
}

/// Convert a unix-epoch millisecond timestamp to a `YYYY-MM-DD` date string (UTC).
fn ms_to_date(ms: f64) -> Option<String> {
    let secs = (ms / 1000.0) as i64;
    chrono::DateTime::from_timestamp(secs, 0).map(|dt| dt.date_naive().format("%Y-%m-%d").to_string())
}

/// 上海黄金交易所-数据资讯-行情走势-品种表
///
/// Pure static table of SGE instrument symbols (no network call).
pub fn spot_symbol_table_sge() -> Result<Vec<SymbolRow>> {
    let symbols = [
        "Au99.99", "Au99.95", "Au100g", "Pt99.95", "Ag(T+D)", "Au(T+D)", "mAu(T+D)",
        "Au(T+N1)", "Au(T+N2)", "Ag99.99", "iAu99.99", "Au99.5", "iAu100g", "iAu99.5",
        "PGC30g", "NYAuTN06", "NYAuTN12",
    ];
    let rows = symbols
        .into_iter()
        .enumerate()
        .map(|(i, symbol)| SymbolRow {
            index: (i + 1) as u32,
            symbol: symbol.to_string(),
        })
        .collect();
    Ok(rows)
}

/// Parse the `spot_quotations_sge` JSON response into rows.
///
/// The response is an object with parallel lists `heyue` (symbol), `times`
/// (HH:MM), `data` (price) and a single `delaystr` update timestamp. Rows whose
/// `time` is not earlier than the update time are dropped, then sorted by time.
pub fn parse_quotations(resp: &Value) -> Result<Vec<QuotationRow>> {
    let heyue = resp
        .get("heyue")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SGE,
            message: "missing heyue".into(),
        })?;
    let times = resp
        .get("times")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SGE,
            message: "missing times".into(),
        })?;
    let data = resp
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SGE,
            message: "missing data".into(),
        })?;
    let delaystr = fstr(resp, "delaystr").unwrap_or_default();
    let update_time = delaystr.split_whitespace().nth(1).unwrap_or("").to_string();

    let n = heyue.len();
    let mut rows = Vec::with_capacity(n);
    for i in 0..n {
        let symbol = heyue
            .get(i)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let time = times
            .get(i)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let price = data.get(i).and_then(Value::as_f64);
        rows.push(QuotationRow {
            symbol,
            time,
            price,
            update_time: delaystr.clone(),
        });
    }
    if !update_time.is_empty() {
        rows.retain(|r| r.time < update_time);
    }
    rows.sort_by(|a, b| a.time.cmp(&b.time));
    Ok(rows)
}

/// 上海黄金交易所-实时行情数据
///
/// Live intraday quotations for the given SGE instrument `symbol`.
pub async fn spot_quotations_sge(client: &Client, symbol: &str) -> Result<Vec<QuotationRow>> {
    let resp = client
        .get_json_with_headers(
            SOURCE_SGE,
            "quotations",
            QUOTATIONS_URL,
            &[("instid", symbol)],
            Some(SGE_HEADERS),
        )
        .await?;
    parse_quotations(&resp)
}

/// Parse the `spot_hist_sge` JSON response into rows.
///
/// The response has a `time` list of `[date, open, close, low, high]` rows.
pub fn parse_hist(resp: &Value) -> Result<Vec<HistRow>> {
    let time = resp
        .get("time")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SGE,
            message: "missing time".into(),
        })?;
    let mut rows = Vec::with_capacity(time.len());
    for entry in time {
        let arr = entry.as_array().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SGE,
            message: "time entry is not an array".into(),
        })?;
        let date = arr
            .first()
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        rows.push(HistRow {
            date,
            open: arr.get(1).and_then(Value::as_f64),
            close: arr.get(2).and_then(Value::as_f64),
            low: arr.get(3).and_then(Value::as_f64),
            high: arr.get(4).and_then(Value::as_f64),
        });
    }
    Ok(rows)
}

/// 上海黄金交易所-数据资讯-行情走势-历史数据
///
/// Historical daily OHLC for the given SGE instrument `symbol`.
pub async fn spot_hist_sge(client: &Client, symbol: &str) -> Result<Vec<HistRow>> {
    let resp = client
        .post_form_json(
            SOURCE_SGE,
            "dailyhq",
            DAILYHQ_URL,
            &[("instid", symbol)],
            Some(SGE_HEADERS),
        )
        .await?;
    parse_hist(&resp)
}

/// Parse a `spot_*_benchmark_sge` JSON response into rows.
///
/// The response has `wp` (evening price) and `zp` (morning price) lists, each a
/// `[epoch_ms, price]` pair. Morning prices are aligned to `wp` by index.
fn parse_benchmark(resp: &Value) -> Result<Vec<BenchmarkRow>> {
    let wp = resp
        .get("wp")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SGE,
            message: "missing wp".into(),
        })?;
    let zp = resp
        .get("zp")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SGE,
            message: "missing zp".into(),
        })?;
    let mut rows = Vec::with_capacity(wp.len());
    for (i, entry) in wp.iter().enumerate() {
        let arr = entry.as_array().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SGE,
            message: "wp entry is not an array".into(),
        })?;
        let ms = arr.first().and_then(Value::as_f64).unwrap_or(0.0);
        let trade_date = ms_to_date(ms).unwrap_or_default();
        let evening_price = arr.get(1).and_then(Value::as_f64);
        let morning_price = zp
            .get(i)
            .and_then(Value::as_array)
            .and_then(|a| a.get(1))
            .and_then(Value::as_f64);
        rows.push(BenchmarkRow {
            trade_date,
            evening_price,
            morning_price,
        });
    }
    Ok(rows)
}

/// Parse the `spot_golden_benchmark_sge` JSON response into rows.
pub fn parse_golden_benchmark(resp: &Value) -> Result<Vec<BenchmarkRow>> {
    parse_benchmark(resp)
}

/// Parse the `spot_silver_benchmark_sge` JSON response into rows.
pub fn parse_silver_benchmark(resp: &Value) -> Result<Vec<BenchmarkRow>> {
    parse_benchmark(resp)
}

/// 上海黄金交易所-数据资讯-上海金基准价-历史数据
pub async fn spot_golden_benchmark_sge(client: &Client) -> Result<Vec<BenchmarkRow>> {
    let resp = client
        .post_form_json(
            SOURCE_SGE,
            "golden_benchmark",
            GOLDEN_BENCHMARK_URL,
            &[],
            Some(SGE_HEADERS),
        )
        .await?;
    parse_golden_benchmark(&resp)
}

/// 上海黄金交易所-数据资讯-上海银基准价-历史数据
pub async fn spot_silver_benchmark_sge(client: &Client) -> Result<Vec<BenchmarkRow>> {
    let resp = client
        .post_form_json(
            SOURCE_SGE,
            "silver_benchmark",
            SILVER_BENCHMARK_URL,
            &[],
            Some(SGE_HEADERS),
        )
        .await?;
    parse_silver_benchmark(&resp)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Value {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    #[test]
    fn symbol_table_matches_akshare() {
        let rows = spot_symbol_table_sge().unwrap();
        assert_eq!(rows.len(), 17);
        assert_eq!(rows[0].index, 1);
        assert_eq!(rows[0].symbol, "Au99.99");
        assert_eq!(rows.last().unwrap().index, 17);
        assert_eq!(rows.last().unwrap().symbol, "NYAuTN12");
    }

    #[test]
    fn fnum_extracts_numeric() {
        let v = serde_json::json!({"close": 451.2});
        assert_eq!(fnum(&v, "close"), Some(451.2));
        assert_eq!(fnum(&v, "open"), None);
    }

    #[test]
    fn parse_quotations_drops_late_and_sorts() {
        let resp = fixture("spot_quotations_sge.json");
        let rows = parse_quotations(&resp).unwrap();
        // "16:00" is after the 15:30:00 update time and is dropped.
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].time, "09:00");
        assert_eq!(rows[0].price, Some(450.12));
        assert_eq!(rows.last().unwrap().time, "15:30");
        assert_eq!(rows.last().unwrap().price, Some(451.50));
        assert_eq!(rows[0].update_time, "2025-04-11 15:30:00");
    }

    #[test]
    fn parse_hist_rows() {
        let resp = fixture("spot_hist_sge.json");
        let rows = parse_hist(&resp).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].date, "2025-04-11");
        assert_eq!(rows[0].open, Some(450.1));
        assert_eq!(rows[0].close, Some(451.2));
        assert_eq!(rows[0].low, Some(449.0));
        assert_eq!(rows[0].high, Some(452.3));
        assert_eq!(rows[2].date, "2025-04-09");
    }

    #[test]
    fn parse_golden_benchmark_rows() {
        let resp = fixture("spot_golden_benchmark_sge.json");
        let rows = parse_golden_benchmark(&resp).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].trade_date, "2024-04-10");
        assert_eq!(rows[0].evening_price, Some(560.12));
        assert_eq!(rows[0].morning_price, Some(561.30));
        assert_eq!(rows[2].trade_date, "2024-04-08");
        assert_eq!(rows[2].evening_price, Some(558.45));
    }

    #[test]
    fn parse_silver_benchmark_rows() {
        let resp = fixture("spot_silver_benchmark_sge.json");
        let rows = parse_silver_benchmark(&resp).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].trade_date, "2024-04-10");
        assert_eq!(rows[0].evening_price, Some(7.12));
        assert_eq!(rows[0].morning_price, Some(7.15));
        assert_eq!(rows[2].trade_date, "2024-04-08");
    }
}
