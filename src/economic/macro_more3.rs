//! Additional macro / economic indicators — China social-financing, urban
//! unemployment, and Jin10 constituent ETF/OPEC holdings.
//!
//! Ports these akshare functions (verified feasible: clean JSON endpoints):
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `macro_china_shrzgm` | `macro_china.py:258` | MOFCOM `datamofcom/front/gnmy/shrzgmQuery` POST, JSON array of objects |
//! | `macro_china_urban_unemployment` | `macro_china.py:318` | `data.stats.gov.cn` `esData` POST + JSON payload, clean JSON |
//! | `macro_cons_gold` | `macro_constitute.py:17` | Jin10 `reports/list_v2` `category=etf&attr_id=1` |
//! | `macro_cons_silver` | `macro_constitute.py:82` | Jin10 `reports/list_v2` `category=etf&attr_id=2` |
//! | `macro_cons_opec_month` | `macro_constitute.py:147` | Jin10 `reports/dates` + `reports/list` `category=opec` |
//!
//! ## DEFERRED (recorded in `docs/_draft_econ.md`, not implemented here)
//! * `macro_china_daily_energy` (`macro_china.py:750`): Jin10 CDN `.js` file
//!   (`cdn.jin10.com/dc/reports/dc_qihuo_energy_report_all.js`) with embedded
//!   JSON extracted via text-slice — same "not plain JSON" trigger as
//!   `macro_china_freight_index`.
//! * `macro_china_freight_index`, `macro_china_nbs_nation`, `macro_china_nbs_region`,
//!   `macro_euro_lme_holding`, `macro_euro_lme_stock`, and all 40 `macro_usa_*`
//!   functions are deferred per the assignment's DEFER policy (session-gated
//!   Jin10 `x-csrf-token`, curl_cffi warmup, `eval`'d tuples, etc.).
//!
//! ## Notes on output shapes
//! `macro_cons_opec_month` emits a **long** table (`date`, `country`, `value`)
//! rather than akshare's wide (date × country) pivot, because the upstream
//! country set is dynamic. Each upstream date report is parsed by taking the
//! first row as the country labels and the last metric row as the values.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_MOFCOM: &str = "mofcom";
const SOURCE_STATS: &str = "stats";
const SOURCE_JIN10: &str = "jin10";

// MOFCOM social financing scale increment.
const SHRSZGM_URL: &str = "https://data.mofcom.gov.cn/datamofcom/front/gnmy/shrzgmQuery";

// NBS / stats.gov.cn urban surveyed unemployment rate.
const URBAN_UNEMPLOYMENT_URL: &str =
    "https://data.stats.gov.cn/dg/website/publicrelease/web/external/stream/esData";

// Jin10 datacenter-api.
const JIN10_LIST_V2: &str = "https://datacenter-api.jin10.com/reports/list_v2";
const JIN10_DATES: &str = "https://datacenter-api.jin10.com/reports/dates";
const JIN10_LIST: &str = "https://datacenter-api.jin10.com/reports/list";

/// One row of China's aggregate financing to the real economy (社会融资规模增量),
/// as returned by MOFCOM `shrzgmQuery`. Mirrors akshare's renamed columns.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChinaShrzgmRow {
    /// 月份 — reporting month (`YYYYMM`-ish string from upstream).
    pub month: String,
    /// 社会融资规模增量 — total aggregate financing increment.
    pub total: Option<f64>,
    /// 其中-人民币贷款 — RMB loans.
    pub rmb_loan: Option<f64>,
    /// 其中-委托贷款外币贷款 — entrusted loans (foreign currency).
    pub entrusted_loan_fx: Option<f64>,
    /// 其中-委托贷款 — entrusted loans.
    pub entrusted_loan: Option<f64>,
    /// 其中-信托贷款 — trust loans.
    pub trust_loan: Option<f64>,
    /// 其中-未贴现银行承兑汇票 — undiscounted bank acceptance bills.
    pub undiscounted_bank_acceptance: Option<f64>,
    /// 其中-企业债券 — corporate bonds.
    pub corporate_bond: Option<f64>,
    /// 其中-非金融企业境内股票融资 — equity financing by non-financial enterprises.
    pub equity_financing: Option<f64>,
}

/// One observed China urban surveyed unemployment rate (城镇调查失业率).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChinaUrbanUnemploymentRow {
    /// Reporting month, normalized to `YYYYMM` (e.g. `202401`).
    pub date: String,
    /// Indicator label with the ` (%)` suffix stripped (e.g. `全国`).
    pub item: String,
    /// Unemployment rate value (`%`).
    pub value: Option<f64>,
}

/// One daily holding row for a Jin10 constituent ETF (gold / silver).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConsEtfRow {
    /// Commodity label (`黄金` / `白银`).
    pub commodity: String,
    /// Report date (`YYYY-MM-DD`).
    pub date: String,
    /// 总库存 — total holdings.
    pub total_holding: Option<f64>,
    /// 增持/减持 — increase / decrease vs prior.
    pub change: Option<f64>,
    /// 总价值 — total value.
    pub total_value: Option<f64>,
}

/// One (date, country) production observation for OPEC monthly report.
/// Emitted in long format because upstream country columns are dynamic.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OpecMonthRow {
    /// Report date (`YYYY-MM-DD`).
    pub date: String,
    /// Country / aggregate label (e.g. `沙特`, `欧佩克产量`).
    pub country: String,
    /// Production value (last metric row of the upstream report).
    pub value: Option<f64>,
}

fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// macro_china_shrzgm
// ---------------------------------------------------------------------------

/// Parse a MOFCOM `shrzgmQuery` JSON array of objects into [`ChinaShrzgmRow`]s.
/// Rows missing `月份` are skipped.
pub(crate) fn parse_shrzgm(resp: &Value) -> Vec<ChinaShrzgmRow> {
    let arr = match resp.as_array() {
        Some(a) => a,
        None => return Vec::new(),
    };
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let month = match item.get("月份").and_then(|v| v.as_str()) {
            Some(m) => m.to_string(),
            None => continue,
        };
        out.push(ChinaShrzgmRow {
            month,
            total: item.get("社会融资规模增量").and_then(num),
            rmb_loan: item.get("其中-人民币贷款").and_then(num),
            entrusted_loan_fx: item.get("其中-委托贷款外币贷款").and_then(num),
            entrusted_loan: item.get("其中-委托贷款").and_then(num),
            trust_loan: item.get("其中-信托贷款").and_then(num),
            undiscounted_bank_acceptance: item.get("其中-未贴现银行承兑汇票").and_then(num),
            corporate_bond: item.get("其中-企业债券").and_then(num),
            equity_financing: item.get("其中-非金融企业境内股票融资").and_then(num),
        });
    }
    out
}

/// 社会融资规模增量统计 — MOFCOM `datamofcom/front/gnmy/shrzgmQuery`
/// (`macro_china.py:258`). Clean JSON POST, no params/headers required.
pub async fn macro_china_shrzgm(client: &Client) -> Result<Vec<ChinaShrzgmRow>> {
    let resp = client
        .post_form_json(SOURCE_MOFCOM, "macro_china_shrzgm", SHRSZGM_URL, &[], None)
        .await?;
    Ok(parse_shrzgm(&resp))
}

// ---------------------------------------------------------------------------
// macro_china_urban_unemployment
// ---------------------------------------------------------------------------

/// Parse a `data.stats.gov.cn` `esData` response into [`ChinaUrbanUnemploymentRow`]s.
/// Only `城镇调查失业率` items with a non-null value are kept.
pub(crate) fn parse_urban_unemployment(resp: &Value) -> Vec<ChinaUrbanUnemploymentRow> {
    let mut out = Vec::new();
    let ok = resp.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
    let data = match resp.get("data").and_then(|v| v.as_array()) {
        Some(d) if ok => d,
        _ => return out,
    };
    for month_item in data {
        let raw = month_item.get("name").and_then(|v| v.as_str());
        let raw = match raw {
            Some(r) => r,
            None => continue,
        };
        // "2024年01月" -> "202401"
        let parts: Vec<&str> = raw.split('年').collect();
        if parts.len() != 2 {
            continue;
        }
        let year = parts[0];
        let month = parts[1].replace("月", "").trim().to_string();
        let month_clean = format!("{}{:0>2}", year, month);
        let values = match month_item.get("values").and_then(|v| v.as_array()) {
            Some(v) => v,
            None => continue,
        };
        for v in values {
            let name = v.get("_name").and_then(|x| x.as_str()).unwrap_or("");
            if name != "城镇调查失业率" {
                continue;
            }
            let rate = v.get("value");
            if rate.is_none() || rate == Some(&Value::Null) {
                continue;
            }
            let value = num(rate.unwrap());
            if value.is_none() {
                continue;
            }
            let showname = v
                .get("i_showname")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .replace(" (%)", "");
            out.push(ChinaUrbanUnemploymentRow {
                date: month_clean.clone(),
                item: showname,
                value,
            });
        }
    }
    out
}

/// 城镇调查失业率 — `data.stats.gov.cn` `esData` POST (`macro_china.py:318`).
/// Sends the fixed `curl_requests` payload (a `cid`/`indicatorIds` report query).
pub async fn macro_china_urban_unemployment(
    client: &Client,
) -> Result<Vec<ChinaUrbanUnemploymentRow>> {
    let payload = serde_json::json!({
        "cid": "ee3b7046b390415b9b7745e3d16f6052",
        "indicatorIds": [
            "3888eac6062945a79c8a27e5f13d4953",
            "1d550f3ec77a463bb607d4a3427e1465",
            "1c1b2d9ab24048bfadc5c7d9510dc663",
            "3921da310de24f14b6457c235657baf9",
            "bd6da1abb26046c2acb38aa701d90e86",
            "7bc1bd5daeac48ae8bb413c34ece1d08",
            "c03a36c9562246b6bc8aab010951ef1c",
            "1061f276ce354907b0b9900c266cf851",
            "40ab91b1ef4948e89633c5c7f55b9713"
        ],
        "daCatalogId": "",
        "das": [{"text": "全国", "value": "000000000000"}],
        "dts": ["199001MM-203601MM"],
        "showType": "1",
        "rootId": "fc982599aa684be7969d7b90b1bd0e84"
    });
    let headers: Option<&[(&str, &str)]> = Some(&[
        ("Origin", "https://data.stats.gov.cn"),
        (
            "Referer",
            "https://data.stats.gov.cn/dg/website/page.html#/pc/national/monthData",
        ),
    ]);
    let resp = client
        .post_json(
            SOURCE_STATS,
            "macro_china_urban_unemployment",
            URBAN_UNEMPLOYMENT_URL,
            &payload,
            headers,
        )
        .await?;
    Ok(parse_urban_unemployment(&resp))
}

// ---------------------------------------------------------------------------
// macro_cons_gold / macro_cons_silver (Jin10 ETF holdings)
// ---------------------------------------------------------------------------

/// Parse a Jin10 `reports/list_v2` `data.values` array (list of 4-element
/// lists `[date, 总库存, 增持/减持, 总价值]`) for one commodity.
pub(crate) fn parse_cons_etf_values(values: &[Value], commodity: &str) -> Vec<ConsEtfRow> {
    let mut out = Vec::with_capacity(values.len());
    for row in values {
        let arr = match row.as_array() {
            Some(a) => a,
            None => continue,
        };
        let date = arr.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
        if date.is_empty() {
            continue;
        }
        out.push(ConsEtfRow {
            commodity: commodity.to_string(),
            date,
            total_holding: arr.get(1).and_then(num),
            change: arr.get(2).and_then(num),
            total_value: arr.get(3).and_then(num),
        });
    }
    out
}

/// Subtract one day from a `YYYY-MM-DD` string (used to advance Jin10 pagination).
fn prev_day(date: &str) -> String {
    if let Ok(dt) = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        return (dt - chrono::Days::new(1)).format("%Y-%m-%d").to_string();
    }
    date.to_string()
}

/// Shared paginated fetch for Jin10 ETF holdings (`category=etf`).
async fn cons_etf(
    client: &Client,
    endpoint: &'static str,
    commodity: &str,
    attr_id: &str,
) -> Result<Vec<ConsEtfRow>> {
    let headers: Option<&[(&str, &str)]> = Some(&[
        ("x-csrf-token", "x-csrf-token"),
        ("x-app-id", "rU6QIu7JHe2gOUeR"),
        ("x-version", "1.0.0"),
    ]);
    let mut max_date = String::new();
    let mut out = Vec::new();
    let mut guard = 0usize;
    loop {
        guard += 1;
        if guard > 1000 {
            break;
        }
        let ts = chrono::Utc::now().timestamp_millis().to_string();
        let params = [
            ("category", "etf"),
            ("attr_id", attr_id),
            ("max_date", max_date.as_str()),
            ("_", ts.as_str()),
        ];
        let resp = client
            .get_json_with_headers(SOURCE_JIN10, endpoint, JIN10_LIST_V2, &params, headers)
            .await?;
        let values = match resp.get("data").and_then(|d| d.get("values")).and_then(|v| v.as_array()) {
            Some(v) if !v.is_empty() => v,
            _ => break,
        };
        for row in parse_cons_etf_values(values, commodity) {
            out.push(row);
        }
        match values.last().and_then(|v| v.get(0)).and_then(|v| v.as_str()) {
            Some(d) => max_date = prev_day(d),
            None => break,
        }
    }
    Ok(out)
}

/// 全球最大黄金 ETF—SPDR Gold Trust 持仓报告 (`macro_constitute.py:17`).
pub async fn macro_cons_gold(client: &Client) -> Result<Vec<ConsEtfRow>> {
    cons_etf(client, "macro_cons_gold", "黄金", "1").await
}

/// 全球最大白银 ETF—SPDR Silver Trust 持仓报告 (`macro_constitute.py:82`).
pub async fn macro_cons_silver(client: &Client) -> Result<Vec<ConsEtfRow>> {
    cons_etf(client, "macro_cons_silver", "白银", "2").await
}

// ---------------------------------------------------------------------------
// macro_cons_opec_month (Jin10 OPEC monthly report)
// ---------------------------------------------------------------------------

/// Parse one Jin10 OPEC `reports/list` response: `values[0]` holds country
/// labels, the last `values` row holds the metric values.
pub(crate) fn parse_opec_date(date: &str, data: &Value) -> Vec<OpecMonthRow> {
    let mut out = Vec::new();
    let values = match data.get("values").and_then(|v| v.as_array()) {
        Some(v) if v.len() >= 2 => v,
        _ => return out,
    };
    let countries = match values[0].as_array() {
        Some(c) => c,
        None => return out,
    };
    let metric = match values.last().and_then(|v| v.as_array()) {
        Some(m) => m,
        None => return out,
    };
    for (i, c) in countries.iter().enumerate() {
        let country = match c.as_str() {
            Some(s) => s.to_string(),
            None => continue,
        };
        out.push(OpecMonthRow {
            date: date.to_string(),
            country,
            value: metric.get(i).and_then(num),
        });
    }
    out
}

/// 欧佩克报告-月度 (`macro_constitute.py:147`). Iterates all reported dates
/// (newest first) from `reports/dates`, fetching each `reports/list`.
pub async fn macro_cons_opec_month(client: &Client) -> Result<Vec<OpecMonthRow>> {
    let headers: Option<&[(&str, &str)]> = Some(&[
        ("x-csrf-token", ""),
        ("x-app-id", "rU6QIu7JHe2gOUeR"),
        ("x-version", "1.0.0"),
    ]);
    let ts = chrono::Utc::now().timestamp_millis().to_string();
    let dates_params = [("category", "opec"), ("_", ts.as_str())];
    let dates_resp = client
        .get_json_with_headers(SOURCE_JIN10, "macro_cons_opec_month", JIN10_DATES, &dates_params, headers)
        .await?;
    let dates = dates_resp
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_JIN10,
            message: "missing data.dates".into(),
        })?;
    let mut out = Vec::new();
    for d in dates.iter().rev() {
        let date = match d.as_str() {
            Some(s) => s.to_string(),
            None => continue,
        };
        let list_params = [
            ("category", "opec"),
            ("date", date.as_str()),
            ("_", ts.as_str()),
        ];
        let resp = client
            .get_json_with_headers(SOURCE_JIN10, "macro_cons_opec_month", JIN10_LIST, &list_params, headers)
            .await?;
        if let Some(data) = resp.get("data") {
            out.extend(parse_opec_date(&date, data));
        }
    }
    Ok(out)
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
    fn parses_shrzgm_rows() {
        let rows = parse_shrzgm(&fixture("macro_china_shrzgm.json"));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].month, "2024-01");
        assert!(approx(rows[0].total, 65000.0));
        assert!(approx(rows[0].rmb_loan, 48000.0));
        assert!(approx(rows[1].entrusted_loan, 500.0));
        assert_eq!(rows[1].month, "2024-02");
    }

    #[test]
    fn parses_urban_unemployment_rows() {
        let rows = parse_urban_unemployment(&fixture("macro_china_urban_unemployment.json"));
        // Only `城镇调查失业率` items with a value contribute rows.
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "202401");
        assert_eq!(rows[0].item, "全国");
        assert!(approx(rows[0].value, 5.2));
        assert_eq!(rows[1].date, "202402");
        assert!(approx(rows[1].value, 5.3));
    }

    #[test]
    fn parses_cons_gold_values() {
        let resp = fixture("macro_cons_gold.json");
        let values = resp.get("data").and_then(|d| d.get("values")).unwrap().as_array().unwrap();
        let rows = parse_cons_etf_values(values, "黄金");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02");
        assert_eq!(rows[0].commodity, "黄金");
        assert!(approx(rows[0].total_holding, 873.5));
        assert!(approx(rows[0].change, -1.2));
        assert!(approx(rows[0].total_value, 1234567.89));
        assert_eq!(rows[1].date, "2024-01-03");
        assert!(approx(rows[1].change, 0.5));
    }

    #[test]
    fn parses_cons_silver_values() {
        let resp = fixture("macro_cons_silver.json");
        let values = resp.get("data").and_then(|d| d.get("values")).unwrap().as_array().unwrap();
        let rows = parse_cons_etf_values(values, "白银");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].commodity, "白银");
        assert!(approx(rows[0].total_holding, 13500.0));
        assert!(approx(rows[0].total_value, 305000.0));
    }

    #[test]
    fn parses_opec_date() {
        let resp = fixture("macro_cons_opec_month.json");
        let data = resp.get("data").unwrap();
        let rows = parse_opec_date("2024-01-18", data);
        // 4 countries in the fixture's first row.
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].date, "2024-01-18");
        assert_eq!(rows[0].country, "阿尔及利亚");
        assert!(approx(rows[0].value, 101.0));
        assert_eq!(rows[3].country, "欧佩克产量");
        assert!(approx(rows[3].value, 2800.0));
    }
}
