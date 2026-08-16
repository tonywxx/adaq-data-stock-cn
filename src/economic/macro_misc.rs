//! Miscellaneous macro-economic endpoints (port of a grab-bag of akshare
//! `economic/*` sources that do not fit the Eastmoney `datacenter-web` shape).
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `macro_info_ws` | `macro_info_ws.py:38` | wallstreetcn `api-one-wscn.awtmt.com/apiv1/finance/macrodatas` (plain JSON) |
//! | `macro_fx_sentiment` | `macro_other.py:53` | jin10 `datacenter-api.jin10.com/sentiment/datas` (plain JSON, constant headers) |
//!
//! ## DEFERRED
//!
//! The following akshare functions in scope were **not** ported, with reasons:
//!
//! * **`macro_cons_gold` / `macro_cons_silver` / `macro_cons_opec_month`**
//!   (`macro_constitute.py:17/82/147`) — jin10 `datacenter-api.jin10.com`
//!   `reports/list_v2` & `reports/dates`, which require the `x-csrf-token`
//!   header and paginate over `max_date` (multi-request cursor loop). Token-gated
//!   + multi-page → DEFER.
//! * **`macro_euro_lme_holding` / `macro_euro_lme_stock`** (`macro_euro.py:839/870`)
//!   — `cdn.jin10.com/data_center/reports/lme_*.json` stores each cell as a
//!   **stringified Python tuple** (`"[0, 0, 0]"`) that the source parses with
//!   `eval(str(x))[i]`. No `eval` in Rust → DEFER.
//! * **`macro_rmb_loan` / `macro_stock_finance`** (`macro_finance_ths.py:50/15`)
//!   — 同花顺 pages parsed with `pd.read_html` (HTML `<table>` scrape) → DEFER.
//! * **`macro_cnbs`** (`marco_cnbs.py:12`) — downloads an `.xlsx` from
//!   `114.115.232.154:8080/handler/download.ashx` via `pd.read_excel` → DEFER.
//! * **`macro_china_nbs_nation` / `macro_china_nbs_region`** (`macro_china_nbs.py:517/566`)
//!   — require a `curl_cffi` session with `impersonate="chrome"` TLS
//!   fingerprinting, a pre-warmed page GET, then multi-step catalog-tree
//!   traversal (`queryIndexTreeAsync` → `queryIndicatorsByCid` → `esData` POST).
//!   No TLS impersonation / multi-step session → DEFER.

use std::collections::HashMap;

use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_WALLSTREETCN: &str = "wallstreetcn";
const SOURCE_JIN10: &str = "jin10";

const WS_BASE: &str = "https://api-one-wscn.awtmt.com/apiv1/finance/macrodatas";
const FX_SENTIMENT_BASE: &str = "https://datacenter-api.jin10.com/sentiment/datas";

/// Read a numeric cell that may be a JSON number or a numeric string (mirrors
/// akshare's `pd.to_numeric(..., errors="coerce")`.
fn cell_num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// Read a string field.
fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(str::to_string)
}

/// Format a wallstreetcn unix-seconds `public_date` to `Asia/Shanghai`
/// (`%Y-%m-%d %H:%M:%S`), matching akshare's `utc=True` + `tz_convert("Asia/Shanghai")`.
/// Mainland China is UTC+8 year-round (no DST), so a fixed +8 offset is exact.
fn fmt_ws_time(ts: i64) -> String {
    let utc = Utc.timestamp_opt(ts, 0).single().unwrap_or_else(Utc::now);
    let cst = utc.with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap());
    cst.format("%Y-%m-%d %H:%M:%S").to_string()
}

// ===========================================================================
// macro_info_ws  (akshare macro_info_ws.py:38)
// ===========================================================================

/// A single macro calendar entry from wallstreetcn.
///
/// Mirrors akshare's output columns: 时间(public_date), 地区(country), 事件(title),
/// 重要性(importance), 今值(actual), 预期(forecast), 前值(previous), 链接(uri).
/// akshare folds `revised` into `前值` when present; we replicate that here.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MacroInfoWsRow {
    /// 时间 — `public_date` converted to `Asia/Shanghai` (akshare `%Y-%m-%d %H:%M:%S`).
    pub time: String,
    /// 地区 (akshare `country`)
    pub region: Option<String>,
    /// 事件 (akshare `title`)
    pub event: Option<String>,
    /// 重要性 (akshare `importance`)
    pub importance: Option<f64>,
    /// 今值 (akshare `actual`)
    pub actual: Option<f64>,
    /// 预期 (akshare `forecast`)
    pub forecast: Option<f64>,
    /// 前值 (akshare `previous`; overwritten by `revised` when present)
    pub previous: Option<f64>,
    /// 链接 (akshare `uri`)
    pub uri: Option<String>,
}

/// Parse `macro_info_ws` rows from the already-fetched `Value`.
///
/// Reads `data.items[]`; each item's `public_date` (unix seconds) is converted to
/// `Asia/Shanghai`. `previous` uses `revised` when it is non-null (akshare behavior).
pub(crate) fn parse_macro_info_ws(resp: &Value) -> Result<Vec<MacroInfoWsRow>> {
    let items = resp
        .get("data")
        .and_then(|d| d.get("items"))
        .and_then(|i| i.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_WALLSTREETCN,
            message: "missing data.items".into(),
        })?;
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let ts = item
            .get("public_date")
            .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
            .unwrap_or(0);
        let revised = item
            .get("revised")
            .and_then(|v| if v.is_null() { None } else { cell_num(v) });
        let previous = revised.or_else(|| item.get("previous").and_then(cell_num));
        out.push(MacroInfoWsRow {
            time: fmt_ws_time(ts),
            region: fstr(item, "country"),
            event: fstr(item, "title"),
            importance: item.get("importance").and_then(cell_num),
            actual: item.get("actual").and_then(cell_num),
            forecast: item.get("forecast").and_then(cell_num),
            previous,
            uri: fstr(item, "uri"),
        });
    }
    Ok(out)
}

/// 华尔街见闻-日历-宏观 (wallstreetcn `macrodatas`).
///
/// `date` is `YYYYMMDD` (akshare default `20240514`); the API takes a `[start,end)`
/// one-day window as unix timestamps, so we convert to a `[date 00:00, date+1 00:00)`
/// UTC range — matching akshare's `__convert_date_format` + one-day `new_datetime`.
pub async fn macro_info_ws(client: &Client, date: &str) -> Result<Vec<MacroInfoWsRow>> {
    let day = chrono::NaiveDate::parse_from_str(date, "%Y%m%d")
        .map_err(|e| Error::InvalidParam(format!("macro_info_ws: bad date `{date}`: {e}")))?;
    let start = DateTime::<Utc>::from_naive_utc_and_offset(day.and_hms_opt(0, 0, 0).unwrap(), Utc);
    let end = start + chrono::Duration::days(1);
    let start_s = start.timestamp().to_string();
    let end_s = end.timestamp().to_string();
    let params = [("start", start_s.as_str()), ("end", end_s.as_str())];
    let v = client
        .get_json(SOURCE_WALLSTREETCN, "macro_info_ws", WS_BASE, &params)
        .await?;
    parse_macro_info_ws(&v)
}

// ===========================================================================
// macro_fx_sentiment  (akshare macro_other.py:53)
// ===========================================================================

/// One day's FX speculative-sentiment report.
///
/// akshare transposes `data.values` into a wide frame: a `date` column plus one
/// numeric column per currency pair. We keep the dynamic pairs as a `HashMap`
/// (keys = pair names, e.g. `澳元兑日元`, `现货黄金兑美元`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct MacroFxSentimentRow {
    /// 日期 (akshare `date`)
    pub date: String,
    /// Currency-pair → long/short position ratio (akshare's wide columns).
    pub pairs: HashMap<String, Option<f64>>,
}

/// Parse `macro_fx_sentiment` rows from the already-fetched `Value`.
///
/// `data.values` is an object whose `date` key holds the row labels and every
/// other key is a currency pair with a parallel numeric array. We zip them into
/// one row per date (mirroring akshare's `.T` then `reset_index`).
pub(crate) fn parse_macro_fx_sentiment(resp: &Value) -> Result<Vec<MacroFxSentimentRow>> {
    let values = resp
        .get("data")
        .and_then(|d| d.get("values"))
        .and_then(|v| v.as_object())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_JIN10,
            message: "missing data.values".into(),
        })?;
    let dates = values
        .get("date")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_JIN10,
            message: "missing data.values.date".into(),
        })?;
    let n = dates.len();
    let mut out: Vec<MacroFxSentimentRow> = (0..n)
        .map(|_| MacroFxSentimentRow {
            date: String::new(),
            pairs: HashMap::new(),
        })
        .collect();
    for (i, d) in dates.iter().enumerate() {
        if let Some(s) = d.as_str() {
            out[i].date = s.to_string();
        }
    }
    for (key, arr) in values {
        if key == "date" {
            continue;
        }
        let Some(arr) = arr.as_array() else {
            continue;
        };
        for (i, val) in arr.iter().enumerate().take(n) {
            out[i].pairs.insert(key.clone(), cell_num(val));
        }
    }
    Ok(out)
}

/// 金十数据-外汇-投机情绪报告 (jin10 `sentiment/datas`).
///
/// `start_date`/`end_date` are `YYYYMMDD` (akshare default `20221011`/`20221017`);
/// we reformat to `YYYY-MM-DD` as the API expects. Headers are constant
/// (no dynamic token), so this is pure HTTP JSON.
pub async fn macro_fx_sentiment(
    client: &Client,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<MacroFxSentimentRow>> {
    let fmt = |s: &str| -> String {
        let d: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
        if d.len() == 8 {
            format!("{}-{}-{}", &d[0..4], &d[4..6], &d[6..8])
        } else {
            s.to_string()
        }
    };
    let start_s = fmt(start_date);
    let end_s = fmt(end_date);
    let cp = String::new();
    let params = [
        ("start_date", start_s.as_str()),
        ("end_date", end_s.as_str()),
        ("currency_pair", cp.as_str()),
    ];
    let headers = [
        ("x-app-id", "rU6QIu7JHe2gOUeR"),
        ("x-csrf-token", ""),
        ("x-version", "1.0.0"),
        (
            "referer",
            "https://datacenter.jin10.com/reportType/dc_ssi_trends",
        ),
        ("origin", "https://datacenter.jin10.com"),
    ];
    let v = client
        .get_json_with_headers(
            SOURCE_JIN10,
            "macro_fx_sentiment",
            FX_SENTIMENT_BASE,
            &params,
            Some(&headers),
        )
        .await?;
    parse_macro_fx_sentiment(&v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    #[test]
    fn parse_macro_info_ws_ok() {
        let rows = parse_macro_info_ws(&fixture("macro_info_ws.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].time, "2024-05-14 00:00:00");
        assert_eq!(rows[0].region.as_deref(), Some("中国"));
        assert_eq!(rows[0].event.as_deref(), Some("中国4月社会融资规模"));
        assert_eq!(rows[0].importance, Some(3.0));
        assert!(approx(rows[0].actual, 1.2));
        assert!(approx(rows[0].forecast, 1.5));
        assert!(approx(rows[0].previous, 1.0));
        // second row: `revised` overrides `previous`
        assert_eq!(rows[1].time, "2024-05-15 00:00:00");
        assert!(approx(rows[1].previous, 0.95));
        assert_eq!(rows[1].uri.as_deref(), Some("https://wallstreetcn.com/x/2"));
    }

    #[test]
    fn parse_macro_fx_sentiment_ok() {
        let rows = parse_macro_fx_sentiment(&fixture("macro_fx_sentiment.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2022-10-11");
        assert!(approx(*rows[0].pairs.get("澳元兑日元").unwrap(), 12.3));
        assert!(approx(*rows[0].pairs.get("澳元兑美元").unwrap(), 0.63));
        assert!(approx(*rows[1].pairs.get("澳元兑日元").unwrap(), 12.4));
        assert!(approx(*rows[1].pairs.get("澳元兑美元").unwrap(), 0.64));
    }
}
