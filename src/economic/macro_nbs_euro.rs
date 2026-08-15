//! Euro-area macro indicators — port of `akshare/economic/macro_euro.py`
//! (jin10 `datacenter-api` reports) plus the NBS functions from
//! `akshare/economic/macro_china_nbs.py` (deferred — see below).
//!
//! ## Function → source line (macro_euro.py)
//!
//! Every `macro_euro_*` function hits the jin10 `datacenter-api.jin10.com`
//! report API. Two calls per indicator: `GET /reports/dates` lists the
//! available report dates, then for every 20th date `GET /reports/list_v2`
//! returns `data.values` (rows) and `data.keys` (column names
//! `日期 / 今值 / 预测值 / 前值`). All share fixed public headers
//! (`x-app-id`, `x-csrf-token` are dummy constants shipped with akshare) — no
//! real token / JS / HTML. A single [`jin10_ec_report`] helper serves them all;
//! the per-fn `pub async fn`s only supply the `attr_id` and the Chinese label.
//!
//! | Rust fn | akshare line | attr_id |
//! | --- | --- | --- |
//! | `macro_euro_gdp_yoy` | 24 | 84 |
//! | `macro_euro_cpi_mom` | 81 | 84 |
//! | `macro_euro_cpi_yoy` | 137 | 8 |
//! | `macro_euro_ppi_mom` | 196 | 36 |
//! | `macro_euro_retail_sales_mom` | 254 | 38 |
//! | `macro_euro_employment_change_qoq` | 313 | 14 |
//! | `macro_euro_unemployment_rate_mom` | 369 | 46 |
//! | `macro_euro_trade_balance` | 428 | 43 |
//! | `macro_euro_current_account_mom` | 487 | 11 |
//! | `macro_euro_industrial_production_mom` | 546 | 19 |
//! | `macro_euro_manufacturing_pmi` | 605 | 30 |
//! | `macro_euro_services_pmi` | 664 | 41 |
//! | `macro_euro_zew_economic_sentiment` | 723 | 48 |
//! | `macro_euro_sentix_investor_confidence` | 781 | 40 |
//!
//! ## Function → source line (macro_china_nbs.py)
//!
//! | Rust fn | akshare line | status |
//! | --- | --- | --- |
//! | `macro_china_nbs_nation` | 517 | **DEFERRED** (see below) |
//! | `macro_china_nbs_region` | 566 | **DEFERRED** (see below) |
//!
//! ## DEFERRED
//!
//! * `macro_china_nbs_nation` / `macro_china_nbs_region` (lines 517 / 566) —
//!   the National Bureau of Statistics API requires a `curl_cffi`
//!   `impersonate="chrome"` session warm-up plus a multi-step catalog-tree
//!   navigation (resolve path → indicators → region catalogs → esData POST).
//!   Not a single clean datacenter JSON call, so deferred.
//! * `macro_euro_lme_holding` / `macro_euro_lme_stock` (lines 839 / 870) — the
//!   LME endpoint returns `values` as nested arrays of stringified tuples
//!   (`"[x, y, z]"`) parsed via Python `eval`, not a clean datacenter row
//!   layout; deferred to avoid fragile `eval`-style parsing.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_JIN10: &str = "jin10";
const JIN10_DATES: &str = "https://datacenter-api.jin10.com/reports/dates";
const JIN10_LIST: &str = "https://datacenter-api.jin10.com/reports/list_v2";

const JIN10_HEADERS: &[(&str, &str)] = &[
    (
        "user-agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
         Chrome/107.0.0.0 Safari/537.36",
    ),
    ("x-app-id", "rU6QIu7JHe2gOUeR"),
    ("x-csrf-token", "x-csrf-token"),
    ("x-version", "1.0.0"),
];

/// Extract a string field, if present.
fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Extract a numeric field, accepting either a JSON number or a numeric string.
fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

/// A single observation of a jin10 euro-area macro indicator.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EuroReportRow {
    /// Indicator label (akshare `商品`), e.g. 欧元区季度GDP年率.
    pub product: String,
    /// Report date (akshare `日期`).
    pub date: String,
    /// Actual / current value (akshare `今值`).
    pub actual: Option<f64>,
    /// Forecast value (akshare `预测值`).
    pub forecast: Option<f64>,
    /// Previous value (akshare `前值`).
    pub previous: Option<f64>,
    pub source: &'static str,
}

/// Fetch one euro-area indicator (`category=ec`) from the jin10 reports API.
///
/// Mirrors akshare: pull the date list, then page every 20th date through
/// `list_v2`, concatenating the `日期 / 今值 / 预测值 / 前值` rows and tagging
/// each with `product`.
async fn jin10_ec_report(
    client: &Client,
    fn_name: &'static str,
    attr_id: &str,
    product: &str,
) -> Result<Vec<EuroReportRow>> {
    let dates_v = client
        .get_json_with_headers(
            SOURCE_JIN10,
            fn_name,
            JIN10_DATES,
            &[("category", "ec"), ("attr_id", attr_id)],
            Some(JIN10_HEADERS),
        )
        .await?;
    let date_list: Vec<String> = dates_v
        .get("data")
        .and_then(|d| d.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let mut out = Vec::new();
    for date in date_list.iter().step_by(20) {
        let params = [("max_date", date.as_str()), ("category", "ec"), ("attr_id", attr_id)];
        let v = client
            .get_json_with_headers(SOURCE_JIN10, fn_name, JIN10_LIST, &params, Some(JIN10_HEADERS))
            .await?;
        out.extend(parse_jin10_ec_report(&v, product)?);
    }
    out.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(out)
}

/// Pure parser for a jin10 `reports/list_v2` response: extract the
/// `日期 / 今值 / 预测值 / 前值` columns by name and tag every row with
/// `product`.
pub fn parse_jin10_ec_report(resp: &Value, product: &str) -> Result<Vec<EuroReportRow>> {
    let data = resp.get("data").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_JIN10,
        message: "missing data".into(),
    })?;
    let values = data.get("values").and_then(|v| v.as_array()).ok_or_else(|| {
        Error::UpstreamChanged {
            origin: SOURCE_JIN10,
            message: "missing data.values".into(),
        }
    })?;
    let keys = data
        .get("keys")
        .and_then(|k| k.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_JIN10,
            message: "missing data.keys".into(),
        })?;

    // The `日期` column is mandatory; the others are optional.
    let has_date = keys
        .iter()
        .any(|k| k.get("name").and_then(|n| n.as_str()) == Some("日期"));
    if !has_date {
        return Err(Error::UpstreamChanged {
            origin: SOURCE_JIN10,
            message: "missing 日期 column".into(),
        });
    }

    let mut out = Vec::with_capacity(values.len());
    for row in values {
        let row_arr = match row.as_array() {
            Some(a) => a,
            None => continue,
        };
        // Re-key the positional row by its column names so the shared `fstr` /
        // `fnum` helpers apply uniformly.
        let obj = Value::Object({
            let mut m = serde_json::Map::new();
            for (i, k) in keys.iter().enumerate() {
                if let Some(name) = k.get("name").and_then(|n| n.as_str())
                    && let Some(v) = row_arr.get(i)
                {
                    m.insert(name.to_string(), v.clone());
                }
            }
            m
        });
        let Some(date) = fstr(&obj, "日期") else {
            continue;
        };
        out.push(EuroReportRow {
            product: product.to_string(),
            date,
            actual: fnum(&obj, "今值"),
            forecast: fnum(&obj, "预测值"),
            previous: fnum(&obj, "前值"),
            source: SOURCE_JIN10,
        });
    }
    // jin10 returns newest-first; expose ascending by date for stable ordering.
    out.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(out)
}

macro_rules! jin10_euro_fn {
    ($rust_fn:ident, $line:literal, $attr:literal, $label:literal) => {
        #[doc = concat!("Euro-area indicator (`", stringify!($rust_fn), "`, akshare line ", $line, ").")]
        pub async fn $rust_fn(client: &Client) -> Result<Vec<EuroReportRow>> {
            jin10_ec_report(client, stringify!($rust_fn), $attr, $label).await
        }
    };
}

jin10_euro_fn!(macro_euro_gdp_yoy, "24", "84", "欧元区季度GDP年率");
jin10_euro_fn!(macro_euro_cpi_mom, "81", "84", "欧元区CPI月率");
jin10_euro_fn!(macro_euro_cpi_yoy, "137", "8", "欧元区CPI年率");
jin10_euro_fn!(macro_euro_ppi_mom, "196", "36", "欧元区PPI月率");
jin10_euro_fn!(macro_euro_retail_sales_mom, "254", "38", "欧元区零售销售月率");
jin10_euro_fn!(macro_euro_employment_change_qoq, "313", "14", "欧元区季调后就业人数季率");
jin10_euro_fn!(macro_euro_unemployment_rate_mom, "369", "46", "欧元区失业率");
jin10_euro_fn!(macro_euro_trade_balance, "428", "43", "欧元区未季调贸易帐");
jin10_euro_fn!(macro_euro_current_account_mom, "487", "11", "欧元区经常帐");
jin10_euro_fn!(macro_euro_industrial_production_mom, "546", "19", "欧元区工业产出月率");
jin10_euro_fn!(macro_euro_manufacturing_pmi, "605", "30", "欧元区制造业PMI初值");
jin10_euro_fn!(macro_euro_services_pmi, "664", "41", "欧元区服务业PMI终值");
jin10_euro_fn!(macro_euro_zew_economic_sentiment, "723", "48", "欧元区ZEW经济景气指数");
jin10_euro_fn!(
    macro_euro_sentix_investor_confidence,
    "781",
    "40",
    "欧元区Sentix投资者信心指数"
);

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let txt = std::fs::read_to_string(p).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    #[test]
    fn parses_jin10_ec_report() {
        let rows = parse_jin10_ec_report(&fixture("macro_euro_report.json"), "欧元区季度GDP年率").unwrap();
        assert_eq!(rows.len(), 3);
        // Sorted ascending by date.
        assert_eq!(rows[0].date, "2018-10-31");
        assert_eq!(rows[0].product, "欧元区季度GDP年率");
        assert_eq!(rows[0].actual, Some(1.0));
        assert_eq!(rows[0].forecast, Some(0.9));
        assert_eq!(rows[0].previous, Some(1.1));
        assert_eq!(rows[2].date, "2019-04-30");
        assert_eq!(rows[2].actual, Some(1.3));
    }

    #[test]
    fn rejects_missing_columns() {
        let mut v = fixture("macro_euro_report.json");
        v["data"]["keys"] = serde_json::json!([{"name": "今值"}, {"name": "预测值"}]);
        assert!(parse_jin10_ec_report(&v, "x").is_err());
    }
}
