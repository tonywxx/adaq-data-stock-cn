//! Jin10 datacenter (`datacenter-api.jin10.com`) central-bank interest-rate
//! decisions — port of the 11 `macro_bank_*_interest_rate` functions from
//! `akshare/economic/macro_bank.py`.
//!
//! Every function hits the same Jin10 `reports/list_v2` endpoint and differs
//! only by the `attr_id` parameter. The upstream response is **array-shaped**
//! (not column-named objects):
//!
//! ```json
//! {"data": {"values": [["2024-03-20", "5.50", "5.50", "5.25"], ...]}}
//! ```
//!
//! i.e. each row is `["日期", "今值", "预测值", "前值"]`. The source paginates by
//! walking `max_date` backwards one day at a time until `data.values` is empty;
//! [`fetch_interest_rate`] mirrors that loop. Because the upstream rows are
//! arrays, parsing uses index-based helpers ([`arr_str`] / [`arr_num`]) instead
//! of the key-based `fstr`/`fnum` helpers used by the Eastmoney object-shaped
//! modules.
//!
//! ## Ported functions
//!
//! | Rust fn | akshare line | `attr_id` | report name |
//! | --- | --- | --- | --- |
//! | `macro_bank_usa_interest_rate` | macro_bank.py:101 | 24 | 美联储利率决议报告 |
//! | `macro_bank_euro_interest_rate` | macro_bank.py:112 | 21 | 欧洲央行决议报告 |
//! | `macro_bank_newzealand_interest_rate` | macro_bank.py:124 | 23 | 新西兰利率决议报告 |
//! | `macro_bank_china_interest_rate` | macro_bank.py:136 | 91 | 中国央行决议报告 |
//! | `macro_bank_switzerland_interest_rate` | macro_bank.py:148 | 25 | 瑞士央行决议报告 |
//! | `macro_bank_english_interest_rate` | macro_bank.py:160 | 26 | 英国央行决议报告 |
//! | `macro_bank_australia_interest_rate` | macro_bank.py:172 | 27 | 澳洲联储决议报告 |
//! | `macro_bank_japan_interest_rate` | macro_bank.py:184 | 22 | 日本央行决议报告 |
//! | `macro_bank_russia_interest_rate` | macro_bank.py:196 | 64 | 俄罗斯央行决议报告 |
//! | `macro_bank_india_interest_rate` | macro_bank.py:208 | 68 | 印度央行决议报告 |
//! | `macro_bank_brazil_interest_rate` | macro_bank.py:220 | 55 | 巴西央行决议报告 |
//!
//! ## DEFERRED
//!
//! None. All 11 functions are pure HTTP JSON (`datacenter-api.jin10.com`) with
//! no JS evaluation / token / signature / HTML scraping, so every one is
//! implemented in full.

use chrono::NaiveDate;
use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_JIN10: &str = "jin10";
const BASE: &str = "https://datacenter-api.jin10.com/reports/list_v2";

/// Jin10 `list_v2` request headers (mirrors `akshare/economic/macro_bank.py`).
const REQUEST_HEADERS: &[(&str, &str)] = &[
    ("Accept", "*/*"),
    ("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8"),
    ("Origin", "https://datacenter.jin10.com"),
    ("Referer", "https://datacenter.jin10.com/"),
    ("x-app-id", "rU6QIu7JHe2gOUeR"),
    ("x-version", "1.0.0"),
];

/// Extract a String from array-position `idx` of a Jin10 row (array-shaped).
fn arr_str(arr: &[Value], idx: usize) -> Option<String> {
    arr.get(idx).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Extract a numeric f64 from array-position `idx`, handling both a JSON number
/// and a numeric string (mirrors `pd.to_numeric(..., errors="coerce")`).
fn arr_num(arr: &[Value], idx: usize) -> Option<f64> {
    arr.get(idx).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

/// One central-bank interest-rate decision row.
///
/// Mirrors the 5 output columns of `akshare/economic/macro_bank.py`:
/// `商品` (report name), `日期` (date), `今值` (current), `预测值` (forecast),
/// `前值` (previous). All rates are in percent (%).
#[derive(Debug, Clone, serde::Serialize)]
pub struct InterestRateRow {
    /// 商品 (report name, e.g. 美联储利率决议报告)
    pub name: String,
    /// 日期 (decision date, YYYY-MM-DD)
    pub date: String,
    /// 今值 (current rate, %)
    pub current: Option<f64>,
    /// 预测值 (forecast rate, %)
    pub forecast: Option<f64>,
    /// 前值 (previous rate, %)
    pub previous: Option<f64>,
}

/// Fetch every page of a Jin10 `attr_id` report, mirroring the akshare
/// pagination loop (walk `max_date` backwards one day until `data.values`
/// is empty). Returns the raw row arrays (each row is `["日期","今值","预测值","前值"]`).
async fn fetch_interest_rate(client: &Client, attr_id: &str) -> Result<Vec<Value>> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let ts_str = ts.to_string();
    let mut max_date = String::new();
    let mut out: Vec<Value> = Vec::new();

    loop {
        let params: &[(&str, &str)] = &[
            ("max_date", max_date.as_str()),
            ("category", "ec"),
            ("attr_id", attr_id),
            ("_", ts_str.as_str()),
        ];
        let v = client
            .get_json_with_headers(
                SOURCE_JIN10,
                "jin10_reports_list_v2",
                BASE,
                params,
                Some(REQUEST_HEADERS),
            )
            .await?;

        let Some(values) = v
            .get("data")
            .and_then(|d| d.get("values"))
            .and_then(|a| a.as_array())
        else {
            break;
        };
        if values.is_empty() {
            break;
        }

        out.extend(values.iter().cloned());

        // Next page starts one day before the last observed date.
        let last_date = values
            .last()
            .and_then(|row| row.get(0))
            .and_then(|d| d.as_str());
        match last_date.and_then(prev_day) {
            Some(next) => max_date = next,
            None => break,
        }
    }

    Ok(out)
}

/// Yesterday (ISO `YYYY-MM-DD`) of `date`, or `None` if `date` is unparseable.
fn prev_day(date: &str) -> Option<String> {
    let d = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    d.pred_opt().map(|d| d.format("%Y-%m-%d").to_string())
}

/// Parse raw Jin10 row arrays into typed [`InterestRateRow`]s, tagging each
/// with its `name` (the report name, fixed per upstream function).
pub fn parse_interest_rate(name: &str, items: &[Value]) -> Result<Vec<InterestRateRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(arr) = item.as_array() else {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_JIN10,
                message: "interest-rate row is not an array".into(),
            });
        };
        let Some(date) = arr_str(arr, 0) else {
            continue;
        };
        out.push(InterestRateRow {
            name: name.to_string(),
            date,
            current: arr_num(arr, 1),
            forecast: arr_num(arr, 2),
            previous: arr_num(arr, 3),
        });
    }
    Ok(out)
}

/// Generate a `pub async fn` per central-bank interest-rate report.
macro_rules! interest_rate {
    ($fn_name:ident, $doc:literal, $attr_id:expr, $name:expr) => {
        #[doc = $doc]
        pub async fn $fn_name(client: &Client) -> Result<Vec<InterestRateRow>> {
            let rows = fetch_interest_rate(client, $attr_id).await?;
            parse_interest_rate($name, &rows)
        }
    };
}

interest_rate!(
    macro_bank_usa_interest_rate,
    "美联储利率决议报告 (USA Fed funds rate decision), 1982-09-27 至今 — Jin10 `attr_id=24` (akshare/economic/macro_bank.py:101).",
    "24",
    "美联储利率决议报告"
);
interest_rate!(
    macro_bank_euro_interest_rate,
    "欧洲央行决议报告 (ECB rate decision), 1999-01-01 至今 — Jin10 `attr_id=21` (akshare/economic/macro_bank.py:112).",
    "21",
    "欧洲央行决议报告"
);
interest_rate!(
    macro_bank_newzealand_interest_rate,
    "新西兰联储决议报告 (RBNZ rate decision), 1999-04-01 至今 — Jin10 `attr_id=23` (akshare/economic/macro_bank.py:124).",
    "23",
    "新西兰利率决议报告"
);
interest_rate!(
    macro_bank_china_interest_rate,
    "中国央行决议报告 (PBoC rate decision), 1999-01-05 至今 — Jin10 `attr_id=91` (akshare/economic/macro_bank.py:136).",
    "91",
    "中国央行决议报告"
);
interest_rate!(
    macro_bank_switzerland_interest_rate,
    "瑞士央行决议报告 (SNB rate decision), 2008-03-13 至今 — Jin10 `attr_id=25` (akshare/economic/macro_bank.py:148).",
    "25",
    "瑞士央行决议报告"
);
interest_rate!(
    macro_bank_english_interest_rate,
    "英国央行决议报告 (BoE rate decision), 1970-01-01 至今 — Jin10 `attr_id=26` (akshare/economic/macro_bank.py:160).",
    "26",
    "英国央行决议报告"
);
interest_rate!(
    macro_bank_australia_interest_rate,
    "澳洲联储决议报告 (RBA rate decision), 1980-02-01 至今 — Jin10 `attr_id=27` (akshare/economic/macro_bank.py:172).",
    "27",
    "澳洲联储决议报告"
);
interest_rate!(
    macro_bank_japan_interest_rate,
    "日本央行决议报告 (BoJ rate decision), 2008-02-14 至今 — Jin10 `attr_id=22` (akshare/economic/macro_bank.py:184).",
    "22",
    "日本央行决议报告"
);
interest_rate!(
    macro_bank_russia_interest_rate,
    "俄罗斯央行决议报告 (CBR rate decision), 2003-06-01 至今 — Jin10 `attr_id=64` (akshare/economic/macro_bank.py:196).",
    "64",
    "俄罗斯央行决议报告"
);
interest_rate!(
    macro_bank_india_interest_rate,
    "印度央行决议报告 (RBI rate decision), 2000-08-01 至今 — Jin10 `attr_id=68` (akshare/economic/macro_bank.py:208).",
    "68",
    "印度央行决议报告"
);
interest_rate!(
    macro_bank_brazil_interest_rate,
    "巴西央行决议报告 (BCB rate decision), 2008-02-01 至今 — Jin10 `attr_id=55` (akshare/economic/macro_bank.py:220).",
    "55",
    "巴西央行决议报告"
);

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

    /// Extract the `data.values` array from a Jin10 fixture response.
    fn values_of(name: &str) -> Vec<Value> {
        fixture(name)
            .get("data")
            .and_then(|d| d.get("values"))
            .and_then(|a| a.as_array())
            .cloned()
            .unwrap()
    }

    #[test]
    fn parses_macro_bank_usa_interest_rate() {
        let rows = parse_interest_rate(
            "美联储利率决议报告",
            &values_of("macro_bank_usa_interest_rate.json"),
        )
        .unwrap();
        assert_eq!(rows.len(), 3);

        assert_eq!(rows[0].name, "美联储利率决议报告");
        assert_eq!(rows[0].date, "2024-03-20");
        assert_eq!(rows[0].current, Some(5.5));
        assert_eq!(rows[0].forecast, Some(5.5));
        assert_eq!(rows[0].previous, Some(5.25));

        // Numeric strings are coerced too.
        assert_eq!(rows[1].date, "2024-01-31");
        assert_eq!(rows[1].current, Some(5.5));
        assert_eq!(rows[1].forecast, None);
        assert_eq!(rows[1].previous, Some(5.0));

        // A null current yields `None`.
        assert_eq!(rows[2].date, "2023-12-13");
        assert_eq!(rows[2].current, None);
        assert_eq!(rows[2].previous, Some(5.5));
    }
}
