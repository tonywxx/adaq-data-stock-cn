//! China macro indicators — token-free Jin10 static `cdn.jin10.com` reports.
//!
//! This is the final sweep of `akshare/economic/macro_china.py`. Every Eastmoney
//! `datacenter-web` `RPT_*` endpoint and every Jin10 `data_center/reports/*.json`
//! endpoint in that file is already ported (see `china.rs`, `macro2.rs`,
//! `macro_china2.rs`, `macro_china_more.rs`, `extra.rs`). The only functions left
//! are the Jin10 `datacenter-api` ("reportType") indicators and the Sina /
//! Mofcom / stats.gov.cn ones.
//!
//! The Jin10 `reportType` indicators are normally reached through
//! `__macro_china_base_func`, which needs an `x-csrf-token`. **However**, the
//! identical time-series data is also published as a plain static JavaScript
//! file on the public CDN `https://cdn.jin10.com/dc/reports/<name>_all.js`
//! (no token, no `Referer` — the same host family as the already-ported
//! `data_center/reports/*.json` endpoints). These `*_all.js` URLs are the
//! `JS_CHINA_*_URL` constants in `akshare/economic/cons.py`. They are the
//! tractable subset this module ports: fetch the `.js` text, extract the first
//! JSON array via bracket scanning, parse `[date, value]` pairs.
//!
//! ## DONE (14 functions, share one `Row`)
//!
//! | Rust fn | akshare fn / line | CDN `.js` file |
//! | --- | --- | --- |
//! | `macro_china_gdp_yearly` | macro_china.py:383 | dc_chinese_gdp_yoy_all.js |
//! | `macro_china_cpi_yearly` | macro_china.py:402 | dc_chinese_cpi_yoy_all.js |
//! | `macro_china_cpi_monthly` | macro_china.py:421 | dc_chinese_cpi_mom_all.js |
//! | `macro_china_ppi_yearly` | macro_china.py:440 | dc_chinese_ppi_yoy_all.js |
//! | `macro_china_exports_yoy` | macro_china.py:459 | dc_chinese_exports_yoy_all.js |
//! | `macro_china_imports_yoy` | macro_china.py:480 | dc_chinese_imports_yoy_all.js |
//! | `macro_china_trade_balance` | macro_china.py:502 | dc_chinese_trade_balance_all.js |
//! | `macro_china_industrial_production_yoy` | macro_china.py:522 | dc_chinese_industrial_production_yoy_all.js |
//! | `macro_china_pmi_yearly` | macro_china.py:544 | dc_chinese_manufacturing_pmi_all.js |
//! | `macro_china_cx_pmi_yearly` | macro_china.py:563 | dc_chinese_caixin_manufacturing_pmi_all.js |
//! | `macro_china_cx_services_pmi_yearly` | macro_china.py:582 | dc_chinese_caixin_services_pmi_all.js |
//! | `macro_china_non_man_pmi` | macro_china.py:601 | dc_chinese_non_manufacturing_pmi_all.js |
//! | `macro_china_fx_reserves_yearly` | macro_china.py:620 | dc_chinese_fx_reserves_all.js |
//! | `macro_china_m2_yearly` | macro_china.py:639 | dc_chinese_m2_money_supply_yoy_all.js |
//!
//! Shared `Row`: [`Jin10JsIndicatorRow`] (date + value). The extracted array is
//! normalized to `[date, value]` pairs; if a row carries more elements only the
//! first two are used.
//!
//! ## DEFERRED (skipped, not tractable)
//!
//! * **Sina `MacPage_Service.get_pagedata` JSONP** (needs Sina `Referer` /
//!   JSONP padding unwrap, paged `from`/`num`): `macro_china_society_electricity`
//!   (3236), `macro_china_society_traffic_volume` (3289),
//!   `macro_china_postal_telecommunicational` (3347),
//!   `macro_china_international_tourism_fx` (3381),
//!   `macro_china_passenger_load_factor` (3415), `macro_china_freight_index`
//!   (3481), `macro_china_central_bank_balance` (3526),
//!   `macro_china_insurance` (3560), `macro_china_supply_of_money` (3594),
//!   `macro_china_foreign_exchange_gold` (3628),
//!   `macro_china_retail_price_index` (3663). Sina token/JSONP → DEFERRED.
//! * **Mofcom `data.mofcom.gov.cn` POST** (`TLSAdapter` + session):
//!   `macro_china_shrzgm` (258) → DEFERRED.
//! * **`data.stats.gov.cn` POST** (header impersonation, `curl_requests`):
//!   `macro_china_urban_unemployment` (318) → DEFERRED.
//! * **Sina-hosted JS text-slicing** (`JS_CHINA_ENERGY_DAILY_URL`):
//!   `macro_china_daily_energy` (750) → DEFERRED (already noted in
//!   `macro_china_more.rs`).

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_JIN10: &str = "jin10";
const BASE_JIN10_JS: &str = "https://cdn.jin10.com/dc/reports";

/// A single `(date, value)` observation from a Jin10 `dc/reports/*_all.js` feed.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Jin10JsIndicatorRow {
    /// 日期 (first element of each `[date, value]` pair).
    pub date: String,
    /// 数值 (second element of each pair; number or numeric string in source).
    pub value: Option<f64>,
}

/// Extract the first balanced JSON array from a Jin10 `.js` document.
///
/// The CDN files wrap their payload in a JS assignment, e.g.
/// `var data = [["2024-01-20", 5.2], ...];`. We find the first `[`, scan to its
/// matching `]` (honouring string literals / escapes), and parse that slice as
/// JSON. This works whether the payload is a top-level array or an object that
/// *contains* an array of pairs.
pub(crate) fn extract_first_array(text: &str) -> Result<Vec<Value>> {
    let start = text.find('[').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_JIN10,
        message: "no JSON array found in jin10 .js document".into(),
    })?;

    let mut depth: i32 = 0;
    let mut in_str = false;
    let mut escaped = false;
    let mut end = None;
    for (i, c) in text[start..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if in_str {
            match c {
                '\\' => escaped = true,
                '"' => in_str = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(start + i);
                    break;
                }
            }
            _ => {}
        }
    }
    let end = end.ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_JIN10,
        message: "unbalanced JSON array in jin10 .js document".into(),
    })?;

    serde_json::from_str(&text[start..=end]).map_err(|e| Error::UpstreamChanged {
        origin: SOURCE_JIN10,
        message: format!("jin10 .js array is not valid JSON: {e}"),
    })
}

/// Parse a Jin10 `dc/reports` array into typed rows.
///
/// Accepts the extracted `&[Value]` (each element a `[date, value, ...]` pair).
/// Rows that are not 2+ element arrays, or whose date is missing, are skipped.
pub(crate) fn parse_jin10_js(items: &[Value]) -> Vec<Jin10JsIndicatorRow> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(pair) = item.as_array() else {
            continue;
        };
        let Some(date) = pair.first().and_then(|v| v.as_str()).map(|s| s.to_string()) else {
            continue;
        };
        let value = pair.get(1).and_then(|v| match v {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.trim().parse::<f64>().ok(),
            _ => None,
        });
        out.push(Jin10JsIndicatorRow { date, value });
    }
    out
}

/// Fetch a Jin10 `dc/reports/*_all.js` document (cache-busted with `?v=1&_=<ms>`).
async fn jin10_js_get(client: &Client, fn_name: &'static str, file: &str) -> Result<String> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let ts = ts.to_string();
    let url = format!("{BASE_JIN10_JS}/{file}?v=1&_={ts}");
    client
        .get_text(SOURCE_JIN10, fn_name, &url, &[("_", ts.as_str())], None)
        .await
}

/// Generate a token-free Jin10 `dc/reports/*_all.js` fetcher + parser.
macro_rules! jin10_js {
    ($fn_name:ident, $doc:literal, $file:expr) => {
        #[doc = $doc]
        pub async fn $fn_name(client: &Client) -> Result<Vec<Jin10JsIndicatorRow>> {
            let text = jin10_js_get(client, stringify!($fn_name), $file).await?;
            let arr = extract_first_array(&text)?;
            Ok(parse_jin10_js(&arr))
        }
    };
}

jin10_js!(
    macro_china_gdp_yearly,
    "中国 GDP 年率报告 — Jin10 token-free `cdn.jin10.com/dc/reports/dc_chinese_gdp_yoy_all.js` (akshare/economic/macro_china.py:383; primary API is x-csrf-token-gated).",
    "dc_chinese_gdp_yoy_all.js"
);
jin10_js!(
    macro_china_cpi_yearly,
    "中国年度 CPI 报告 — Jin10 token-free `dc_chinese_cpi_yoy_all.js` (macro_china.py:402; primary API is x-csrf-token-gated).",
    "dc_chinese_cpi_yoy_all.js"
);
jin10_js!(
    macro_china_cpi_monthly,
    "中国月度 CPI 报告 — Jin10 token-free `dc_chinese_cpi_mom_all.js` (macro_china.py:421; primary API is x-csrf-token-gated).",
    "dc_chinese_cpi_mom_all.js"
);
jin10_js!(
    macro_china_ppi_yearly,
    "中国年度 PPI 报告 — Jin10 token-free `dc_chinese_ppi_yoy_all.js` (macro_china.py:440; primary API is x-csrf-token-gated).",
    "dc_chinese_ppi_yoy_all.js"
);
jin10_js!(
    macro_china_exports_yoy,
    "中国以美元计算出口年率报告 — Jin10 token-free `dc_chinese_exports_yoy_all.js` (macro_china.py:459; primary API is x-csrf-token-gated).",
    "dc_chinese_exports_yoy_all.js"
);
jin10_js!(
    macro_china_imports_yoy,
    "中国以美元计算进口年率报告 — Jin10 token-free `dc_chinese_imports_yoy_all.js` (macro_china.py:480; primary API is x-csrf-token-gated).",
    "dc_chinese_imports_yoy_all.js"
);
jin10_js!(
    macro_china_trade_balance,
    "中国以美元计算贸易帐报告 — Jin10 token-free `dc_chinese_trade_balance_all.js` (macro_china.py:502; primary API is x-csrf-token-gated).",
    "dc_chinese_trade_balance_all.js"
);
jin10_js!(
    macro_china_industrial_production_yoy,
    "中国规模以上工业增加值年率报告 — Jin10 token-free `dc_chinese_industrial_production_yoy_all.js` (macro_china.py:522; primary API is x-csrf-token-gated).",
    "dc_chinese_industrial_production_yoy_all.js"
);
jin10_js!(
    macro_china_pmi_yearly,
    "中国官方制造业 PMI 报告 — Jin10 token-free `dc_chinese_manufacturing_pmi_all.js` (macro_china.py:544; primary API is x-csrf-token-gated).",
    "dc_chinese_manufacturing_pmi_all.js"
);
jin10_js!(
    macro_china_cx_pmi_yearly,
    "中国财新制造业 PMI 终值报告 — Jin10 token-free `dc_chinese_caixin_manufacturing_pmi_all.js` (macro_china.py:563; primary API is x-csrf-token-gated).",
    "dc_chinese_caixin_manufacturing_pmi_all.js"
);
jin10_js!(
    macro_china_cx_services_pmi_yearly,
    "中国财新服务业 PMI 报告 — Jin10 token-free `dc_chinese_caixin_services_pmi_all.js` (macro_china.py:582; primary API is x-csrf-token-gated).",
    "dc_chinese_caixin_services_pmi_all.js"
);
jin10_js!(
    macro_china_non_man_pmi,
    "中国非制造业 PMI 报告 — Jin10 token-free `dc_chinese_non_manufacturing_pmi_all.js` (macro_china.py:601; primary API is x-csrf-token-gated).",
    "dc_chinese_non_manufacturing_pmi_all.js"
);
jin10_js!(
    macro_china_fx_reserves_yearly,
    "中国外汇储备报告 — Jin10 token-free `dc_chinese_fx_reserves_all.js` (macro_china.py:620; primary API is x-csrf-token-gated).",
    "dc_chinese_fx_reserves_all.js"
);
jin10_js!(
    macro_china_m2_yearly,
    "中国 M2 货币供应年率报告 — Jin10 token-free `dc_chinese_m2_money_supply_yoy_all.js` (macro_china.py:639; primary API is x-csrf-token-gated).",
    "dc_chinese_m2_money_supply_yoy_all.js"
);

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let p = p.join("tests/fixtures").join(name);
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    // Bracket-scan extractor (text -> JSON array).
    #[test]
    fn extracts_jin10_js_array() {
        let text = "var data = [[\"2024-01-20\", 5.2], [\"2023-01-20\", 3.0]];";
        let arr = extract_first_array(text).unwrap();
        assert_eq!(arr.len(), 2);
        let rows = parse_jin10_js(&arr);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-20");
        assert!(approx(rows[0].value, 5.2));
        assert!(approx(rows[1].value, 3.0));
    }

    // Object-wrapped payload still yields the inner pair array.
    #[test]
    fn extracts_jin10_js_object_wrapper() {
        let text = "var x = {\"list\": [[\"2024-01-20\", 5.2]]};";
        let arr = extract_first_array(text).unwrap();
        let rows = parse_jin10_js(&arr);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].date, "2024-01-20");
        assert!(approx(rows[0].value, 5.2));
    }

    #[test]
    fn parses_macro_china_gdp_yearly() {
        let rows = parse_jin10_js(fixture("macro_china_gdp_yearly.json").as_array().unwrap());
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-20");
        assert!(approx(rows[0].value, 5.2));
        assert_eq!(rows[1].date, "2023-01-20");
        assert!(approx(rows[1].value, 3.0));
    }

    #[test]
    fn parses_macro_china_cpi_yearly() {
        let rows = parse_jin10_js(fixture("macro_china_cpi_yearly.json").as_array().unwrap());
        assert_eq!(rows.len(), 2);
        assert!(approx(rows[0].value, 0.2));
    }

    #[test]
    fn parses_macro_china_cpi_monthly() {
        let rows = parse_jin10_js(fixture("macro_china_cpi_monthly.json").as_array().unwrap());
        assert_eq!(rows.len(), 2);
        assert!(approx(rows[1].value, -0.3));
    }

    #[test]
    fn parses_macro_china_ppi_yearly() {
        let rows = parse_jin10_js(fixture("macro_china_ppi_yearly.json").as_array().unwrap());
        assert_eq!(rows.len(), 2);
        assert!(approx(rows[0].value, -2.7));
    }

    #[test]
    fn parses_macro_china_exports_yoy() {
        let rows = parse_jin10_js(fixture("macro_china_exports_yoy.json").as_array().unwrap());
        assert_eq!(rows.len(), 2);
        assert!(approx(rows[0].value, 7.1));
    }

    #[test]
    fn parses_macro_china_imports_yoy() {
        let rows = parse_jin10_js(fixture("macro_china_imports_yoy.json").as_array().unwrap());
        assert_eq!(rows.len(), 2);
        assert!(approx(rows[1].value, 2.0));
    }

    #[test]
    fn parses_macro_china_trade_balance() {
        let rows = parse_jin10_js(fixture("macro_china_trade_balance.json").as_array().unwrap());
        assert_eq!(rows.len(), 2);
        assert!(approx(rows[0].value, 85.0));
    }

    #[test]
    fn parses_macro_china_industrial_production_yoy() {
        let rows =
            parse_jin10_js(fixture("macro_china_industrial_production_yoy.json").as_array().unwrap());
        assert_eq!(rows.len(), 2);
        assert!(approx(rows[0].value, 6.8));
    }

    #[test]
    fn parses_macro_china_pmi_yearly() {
        let rows = parse_jin10_js(fixture("macro_china_pmi_yearly.json").as_array().unwrap());
        assert_eq!(rows.len(), 2);
        assert!(approx(rows[1].value, 49.0));
    }

    #[test]
    fn parses_macro_china_cx_pmi_yearly() {
        let rows = parse_jin10_js(fixture("macro_china_cx_pmi_yearly.json").as_array().unwrap());
        assert_eq!(rows.len(), 2);
        assert!(approx(rows[0].value, 51.0));
    }

    #[test]
    fn parses_macro_china_cx_services_pmi_yearly() {
        let rows =
            parse_jin10_js(fixture("macro_china_cx_services_pmi_yearly.json").as_array().unwrap());
        assert_eq!(rows.len(), 2);
        assert!(approx(rows[1].value, 52.0));
    }

    #[test]
    fn parses_macro_china_non_man_pmi() {
        let rows = parse_jin10_js(fixture("macro_china_non_man_pmi.json").as_array().unwrap());
        assert_eq!(rows.len(), 2);
        assert!(approx(rows[0].value, 50.8));
    }

    #[test]
    fn parses_macro_china_fx_reserves_yearly() {
        let rows = parse_jin10_js(fixture("macro_china_fx_reserves_yearly.json").as_array().unwrap());
        assert_eq!(rows.len(), 2);
        assert!(approx(rows[1].value, 32000.0));
    }

    #[test]
    fn parses_macro_china_m2_yearly() {
        let rows = parse_jin10_js(fixture("macro_china_m2_yearly.json").as_array().unwrap());
        assert_eq!(rows.len(), 2);
        assert!(approx(rows[0].value, 8.7));
    }
}
