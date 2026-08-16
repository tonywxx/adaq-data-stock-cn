//! Additional China macro indicators (port of `akshare/economic/macro_china.py`).
//!
//! This leaf module ports the subset of `macro_china.py` functions that are
//! reachable as **pure HTTP** and don't require JS-signed params, HTML scraping,
//! or token-gated sessions:
//!
//! * **Eastmoney `datacenter-web`** `RPT_INDUSTRY_INDEX` — the four Baltic /
//!   cape-size freight indices (`macro_shipping_*`), each filtered by
//!   `INDICATOR_ID`. Shares the `emg_data_array` / `emdc_fetch_indicator`
//!   pattern from `macro_china2.rs`.
//! * **Jin10 `cdn.jin10.com` plain-JSON** — `il_1.json` (shibor), `il_2.json`
//!   (HK interbank), `exchange_rate.json` (RMB central parity), `fs_1.json` /
//!   `fs_2.json` (SH/SZ margin financing) and `sge.json` (SGE report). These are
//!   public CDN JSON documents fetched with only a `?_=<ms>` cache-buster — no
//!   `x-csrf-token` (that token gates `datacenter-api.jin10.com`, a different
//!   host that is deferred elsewhere).
//!
//! | Rust function | akshare line | source |
//! | --- | --- | --- |
//! | `macro_shipping_bci` | macro_china.py:2098 | eastmoney `RPT_INDUSTRY_INDEX` / `EMI00107666` |
//! | `macro_shipping_bdi` | macro_china.py:2109 | eastmoney `RPT_INDUSTRY_INDEX` / `EMI00107664` |
//! | `macro_shipping_bpi` | macro_china.py:2120 | eastmoney `RPT_INDUSTRY_INDEX` / `EMI00107665` |
//! | `macro_shipping_bcti` | macro_china.py:2131 | eastmoney `RPT_INDUSTRY_INDEX` / `EMI00107669` |
//! | `macro_china_shibor_all` | macro_china.py:658 | jin10 `cdn.../il_1.json` |
//! | `macro_china_hk_market_info` | macro_china.py:704 | jin10 `cdn.../il_2.json` |
//! | `macro_china_rmb` | macro_china.py:780 | jin10 `cdn.../exchange_rate.json` |
//! | `macro_china_market_margin_sh` | macro_china.py:919 | jin10 `cdn.../fs_1.json` |
//! | `macro_china_market_margin_sz` | macro_china.py:888 | jin10 `cdn.../fs_2.json` |
//! | `macro_china_au_report` | macro_china.py:953 | jin10 `cdn.../sge.json` |
//!
//! ## DEFERRED (not ported here)
//!
//! * **Already implemented elsewhere (skip to avoid duplication):**
//!   `macro_china_pmi` (macro_china.py:2622, Eastmoney `RPT_ECONOMY_PMI`) is
//!   already ported in `src/economic/macro2.rs`.
//! * **Jin10 `datacenter-api.jin10.com` token-gated** (`__macro_china_base_func`
//!   needs `x-app-id`/`x-csrf-token`): `macro_china_gdp_yearly` (383),
//!   `macro_china_cpi_yearly` (402), `macro_china_cpi_monthly` (421),
//!   `macro_china_ppi_yearly` (440), `macro_china_exports_yoy` (459),
//!   `macro_china_imports_yoy` (480), `macro_china_trade_balance` (502),
//!   `macro_china_industrial_production_yoy` (522), `macro_china_pmi_yearly` (544),
//!   `macro_china_cx_pmi_yearly` (563), `macro_china_cx_services_pmi_yearly` (582),
//!   `macro_china_non_man_pmi` (601), `macro_china_fx_reserves_yearly` (620),
//!   `macro_china_m2_yearly` (639).
//! * **Sina `MacPage_Service.get_pagedata`** (JSONP via `demjson.decode`, paged):
//!   `macro_china_shrzgm` (258, MOFCOM POST + `TLSAdapter`),
//!   `macro_china_urban_unemployment` (318, `data.stats.gov.cn` POST + header
//!   impersonation), `macro_china_society_electricity` (3236),
//!   `macro_china_society_traffic_volume` (3289),
//!   `macro_china_postal_telecommunicational` (3347),
//!   `macro_china_international_tourism_fx` (3381),
//!   `macro_china_passenger_load_factor` (3415), `macro_china_freight_index` (3481,
//!   also GBK `vMacExcle`), `macro_china_central_bank_balance` (3526),
//!   `macro_china_insurance` (3560), `macro_china_supply_of_money` (3594),
//!   `macro_china_foreign_exchange_gold` (3628), `macro_china_retail_price_index` (3663).
//! * **Sina-hosted JS / JSONP requiring fragile text slicing:**
//!   `macro_china_daily_energy` (750, `JS_CHINA_ENERGY_DAILY_URL` from
//!   `economic/cons.py`).
//! * **Not present in `macro_china.py`** (defined in
//!   `akshare/bond/bond_china_money.py:192`, outside this module's source scope):
//!   `macro_china_swap_rate`.

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

const SOURCE_JIN10: &str = "jin10";
const BASE_EM: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";
const BASE_JIN10_IL1: &str = "https://cdn.jin10.com/data_center/reports/il_1.json";
const BASE_JIN10_IL2: &str = "https://cdn.jin10.com/data_center/reports/il_2.json";
const BASE_JIN10_RMB: &str = "https://cdn.jin10.com/data_center/reports/exchange_rate.json";
const BASE_JIN10_FS1: &str = "https://cdn.jin10.com/data_center/reports/fs_1.json";
const BASE_JIN10_FS2: &str = "https://cdn.jin10.com/data_center/reports/fs_2.json";
const BASE_JIN10_SGE: &str = "https://cdn.jin10.com/data_center/reports/sge.json";

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

/// Extract `result.data` (the row array) from a datacenter-web response.
fn emg_data_array(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data".into(),
        })
}

fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Numeric field: accepts a JSON number or a numeric string (Jin10 returns
/// prices/changes as strings, e.g. `"1.3630"`).
fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

/// Numeric scalar: accepts a JSON number or a numeric string.
fn fnum_val(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// Element `idx` of a JSON array as `f64` (number or numeric string).
fn arr_f64(arr: &[Value], idx: usize) -> Option<f64> {
    arr.get(idx).and_then(fnum_val)
}

/// Element `idx` of a JSON array as `String`.
fn arr_str(arr: &[Value], idx: usize) -> Option<String> {
    arr.get(idx).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Fetch a Jin10 CDN JSON document (cache-busted with `?_=<ms>`).
async fn jin10_get(client: &Client, endpoint: &'static str, url: &str) -> Result<Value> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let ts = ts.to_string();
    client
        .get_json(SOURCE_JIN10, endpoint, url, &[("_", ts.as_str())])
        .await
}

/// Extract the `values` object from a Jin10 CDN response.
fn jin10_values(resp: &Value) -> Result<&serde_json::Map<String, Value>> {
    resp.get("values")
        .and_then(Value::as_object)
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_JIN10,
            message: "missing values".into(),
        })
}

// ---------------------------------------------------------------------------
// Eastmoney datacenter: macro_shipping_* (RPT_INDUSTRY_INDEX)
// ---------------------------------------------------------------------------

/// A single observation of an Eastmoney `RPT_INDUSTRY_INDEX` indicator.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EmIndicatorRow {
    /// 日期 (Eastmoney `REPORT_DATE`)
    pub date: String,
    /// 最新值 (Eastmoney `INDICATOR_VALUE`)
    pub latest_value: Option<f64>,
    /// 涨跌幅 (Eastmoney `CHANGE_RATE`)
    pub change_rate: Option<f64>,
    /// 近3月涨跌幅 (Eastmoney `CHANGERATE_3M`)
    pub change_3m: Option<f64>,
    /// 近6月涨跌幅 (Eastmoney `CHANGERATE_6M`)
    pub change_6m: Option<f64>,
    /// 近1年涨跌幅 (Eastmoney `CHANGERATE_1Y`)
    pub change_1y: Option<f64>,
    /// 近2年涨跌幅 (Eastmoney `CHANGERATE_2Y`)
    pub change_2y: Option<f64>,
    /// 近3年涨跌幅 (Eastmoney `CHANGERATE_3Y`)
    pub change_3y: Option<f64>,
}

/// Shared parser for every `RPT_INDUSTRY_INDEX` indicator response.
pub(crate) fn parse_em_indicator(items: &[Value]) -> Result<Vec<EmIndicatorRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(date) = fstr(item, "REPORT_DATE") else {
            continue;
        };
        out.push(EmIndicatorRow {
            date,
            latest_value: fnum(item, "INDICATOR_VALUE"),
            change_rate: fnum(item, "CHANGE_RATE"),
            change_3m: fnum(item, "CHANGERATE_3M"),
            change_6m: fnum(item, "CHANGERATE_6M"),
            change_1y: fnum(item, "CHANGERATE_1Y"),
            change_2y: fnum(item, "CHANGERATE_2Y"),
            change_3y: fnum(item, "CHANGERATE_3Y"),
        });
    }
    Ok(out)
}

/// Fetch a `RPT_INDUSTRY_INDEX` indicator filtered by `INDICATOR_ID`.
async fn emdc_fetch_indicator(
    client: &Client,
    fn_name: &'static str,
    indicator_id: &str,
    page_size: &str,
) -> Result<Vec<Value>> {
    let filter = format!(r#"(INDICATOR_ID="{}")"#, indicator_id);
    let params = [
        ("reportName", "RPT_INDUSTRY_INDEX"),
        (
            "columns",
            "REPORT_DATE,INDICATOR_VALUE,CHANGE_RATE,CHANGERATE_3M,CHANGERATE_6M,\
             CHANGERATE_1Y,CHANGERATE_2Y,CHANGERATE_3Y",
        ),
        ("pageNumber", "1"),
        ("pageSize", page_size),
        ("sortColumns", "REPORT_DATE"),
        ("sortTypes", "-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, fn_name, BASE_EM, &params)
        .await?;
    emg_data_array(&v).cloned()
}

macro_rules! shipping_index {
    ($fn_name:ident, $doc:literal, $indicator_id:expr) => {
        #[doc = $doc]
        pub async fn $fn_name(client: &Client) -> Result<Vec<EmIndicatorRow>> {
            let data =
                emdc_fetch_indicator(client, stringify!($fn_name), $indicator_id, "500").await?;
            parse_em_indicator(&data)
        }
    };
}

shipping_index!(
    macro_shipping_bci,
    "海岬型运费指数(BCI) — Eastmoney `RPT_INDUSTRY_INDEX` indicator `EMI00107666` (akshare/economic/macro_china.py:2098).",
    "EMI00107666"
);
shipping_index!(
    macro_shipping_bdi,
    "波罗的海干散货指数(BDI) — Eastmoney `RPT_INDUSTRY_INDEX` indicator `EMI00107664` (akshare/economic/macro_china.py:2109).",
    "EMI00107664"
);
shipping_index!(
    macro_shipping_bpi,
    "巴拿马型运费指数(BPI) — Eastmoney `RPT_INDUSTRY_INDEX` indicator `EMI00107665` (akshare/economic/macro_china.py:2120).",
    "EMI00107665"
);
shipping_index!(
    macro_shipping_bcti,
    "成品油运输指数(BCTI) — Eastmoney `RPT_INDUSTRY_INDEX` indicator `EMI00107669` (akshare/economic/macro_china.py:2131).",
    "EMI00107669"
);

// ---------------------------------------------------------------------------
// Jin10 CDN: rate / parity / margin / SGE
// ---------------------------------------------------------------------------

/// A (date, term) rate observation from a Jin10 `il_1.json` / `il_2.json` feed.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Jin10RateRow {
    /// 日期 (outer `values` key, e.g. `2026-08-14`)
    pub date: String,
    /// 期限 (inner term key, e.g. `O/N`, `1W`, `1M`)
    pub term: String,
    /// 定价 (first element of the `[price, change]` pair; string in source)
    pub price: Option<f64>,
    /// 涨跌幅 (second element of the `[price, change]` pair)
    pub change: Option<f64>,
}

/// Parse `il_1.json` / `il_2.json`: `values[date][term] = [price, change]`.
pub(crate) fn parse_jin10_rate(resp: &Value) -> Result<Vec<Jin10RateRow>> {
    let values = jin10_values(resp)?;
    let mut out = Vec::new();
    for (date, term_obj) in values {
        let Some(term_obj) = term_obj.as_object() else {
            continue;
        };
        for (term, pair) in term_obj {
            let price = pair.get(0).and_then(|v| match v {
                Value::Number(n) => n.as_f64(),
                Value::String(s) => s.trim().parse::<f64>().ok(),
                _ => None,
            });
            let change = pair.get(1).and_then(|v| match v {
                Value::Number(n) => n.as_f64(),
                Value::String(s) => s.trim().parse::<f64>().ok(),
                _ => None,
            });
            out.push(Jin10RateRow {
                date: date.clone(),
                term: term.clone(),
                price,
                change,
            });
        }
    }
    Ok(out)
}

/// A (date, currency-pair) RMB central-parity observation from `exchange_rate.json`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Jin10RmbRow {
    /// 日期 (outer `values` key, e.g. `2021-05-13`)
    pub date: String,
    /// 货币对 (inner pair key, e.g. `美元/人民币`)
    pub pair: String,
    /// 中间价 (first element of the `[mid, change]` pair)
    pub mid: Option<f64>,
    /// 涨跌幅 (second element of the `[mid, change]` pair)
    pub change: Option<f64>,
}

/// Parse `exchange_rate.json`: `values[date][pair] = [mid, change]`.
pub(crate) fn parse_jin10_rmb(resp: &Value) -> Result<Vec<Jin10RmbRow>> {
    let values = jin10_values(resp)?;
    let mut out = Vec::new();
    for (date, pair_obj) in values {
        let Some(pair_obj) = pair_obj.as_object() else {
            continue;
        };
        for (pair, arr) in pair_obj {
            let mid = arr.get(0).and_then(|v| match v {
                Value::Number(n) => n.as_f64(),
                Value::String(s) => s.trim().parse::<f64>().ok(),
                _ => None,
            });
            let change = arr.get(1).and_then(|v| match v {
                Value::Number(n) => n.as_f64(),
                Value::String(s) => s.trim().parse::<f64>().ok(),
                _ => None,
            });
            out.push(Jin10RmbRow {
                date: date.clone(),
                pair: pair.clone(),
                mid,
                change,
            });
        }
    }
    Ok(out)
}

/// A margin-financing snapshot for one date (SH `fs_1.json` / SZ `fs_2.json`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Jin10MarginRow {
    /// 日期 (outer `values` key)
    pub date: String,
    /// 融资买入额 (index 0)
    pub financing_buy: Option<f64>,
    /// 融资余额 (index 1)
    pub financing_balance: Option<f64>,
    /// 融券卖出量 (index 2)
    pub securities_sell: Option<f64>,
    /// 融券余量 (index 3)
    pub securities_volume: Option<f64>,
    /// 融券余额 (index 4)
    pub securities_balance: Option<f64>,
    /// 融资融券余额 (index 5)
    pub total: Option<f64>,
}

/// Parse `fs_1.json` / `fs_2.json`: `values[date] = [6 numbers]` (some may be null).
pub(crate) fn parse_jin10_margin(resp: &Value) -> Result<Vec<Jin10MarginRow>> {
    let values = jin10_values(resp)?;
    let mut out = Vec::with_capacity(values.len());
    for (date, arr) in values {
        let Some(arr) = arr.as_array() else {
            continue;
        };
        out.push(Jin10MarginRow {
            date: date.clone(),
            financing_buy: arr_f64(arr, 0),
            financing_balance: arr_f64(arr, 1),
            securities_sell: arr_f64(arr, 2),
            securities_volume: arr_f64(arr, 3),
            securities_balance: arr_f64(arr, 4),
            total: arr_f64(arr, 5),
        });
    }
    Ok(out)
}

/// A single Shanghai Gold Exchange (SGE) traded-product record.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SgeRow {
    /// 日期 (outer `values` key)
    pub date: String,
    /// 商品 (field 0)
    pub product: Option<String>,
    /// 开盘价 (field 1)
    pub open: Option<f64>,
    /// 最高价 (field 2)
    pub high: Option<f64>,
    /// 最低价 (field 3)
    pub low: Option<f64>,
    /// 收盘价 (field 4)
    pub close: Option<f64>,
    /// 涨跌 (field 5)
    pub change: Option<f64>,
    /// 涨跌幅 (field 6)
    pub change_pct: Option<f64>,
    /// 加权平均价 (field 7)
    pub weighted_avg: Option<f64>,
    /// 成交量 (field 8)
    pub volume: Option<f64>,
    /// 成交金额 (field 9)
    pub amount: Option<f64>,
    /// 持仓量 (field 10)
    pub position: Option<f64>,
    /// 交收方向 (field 11)
    pub delivery_dir: Option<String>,
    /// 交收量 (field 12)
    pub delivery_volume: Option<f64>,
}

/// Parse `sge.json`: `values[date] = [[13 fields], ...]`.
pub(crate) fn parse_sge(resp: &Value) -> Result<Vec<SgeRow>> {
    let values = jin10_values(resp)?;
    let mut out = Vec::new();
    for (date, recs) in values {
        let Some(recs) = recs.as_array() else {
            continue;
        };
        for rec in recs {
            let Some(rec) = rec.as_array() else {
                continue;
            };
            out.push(SgeRow {
                date: date.clone(),
                product: arr_str(rec, 0),
                open: arr_f64(rec, 1),
                high: arr_f64(rec, 2),
                low: arr_f64(rec, 3),
                close: arr_f64(rec, 4),
                change: arr_f64(rec, 5),
                change_pct: arr_f64(rec, 6),
                weighted_avg: arr_f64(rec, 7),
                volume: arr_f64(rec, 8),
                amount: arr_f64(rec, 9),
                position: arr_f64(rec, 10),
                delivery_dir: arr_str(rec, 11),
                delivery_volume: arr_f64(rec, 12),
            });
        }
    }
    Ok(out)
}

/// 上海银行业同业拆借报告 — Jin10 `il_1.json` (akshare/economic/macro_china.py:658).
pub async fn macro_china_shibor_all(client: &Client) -> Result<Vec<Jin10RateRow>> {
    let v = jin10_get(client, "macro_china_shibor_all", BASE_JIN10_IL1).await?;
    parse_jin10_rate(&v)
}

/// 香港同业拆借报告 — Jin10 `il_2.json` (akshare/economic/macro_china.py:704).
pub async fn macro_china_hk_market_info(client: &Client) -> Result<Vec<Jin10RateRow>> {
    let v = jin10_get(client, "macro_china_hk_market_info", BASE_JIN10_IL2).await?;
    parse_jin10_rate(&v)
}

/// 中国人民币汇率中间价报告 — Jin10 `exchange_rate.json` (akshare/economic/macro_china.py:780).
pub async fn macro_china_rmb(client: &Client) -> Result<Vec<Jin10RmbRow>> {
    let v = jin10_get(client, "macro_china_rmb", BASE_JIN10_RMB).await?;
    parse_jin10_rmb(&v)
}

/// 上海融资融券报告 — Jin10 `fs_1.json` (akshare/economic/macro_china.py:919).
pub async fn macro_china_market_margin_sh(client: &Client) -> Result<Vec<Jin10MarginRow>> {
    let v = jin10_get(client, "macro_china_market_margin_sh", BASE_JIN10_FS1).await?;
    parse_jin10_margin(&v)
}

/// 深圳融资融券报告 — Jin10 `fs_2.json` (akshare/economic/macro_china.py:888).
pub async fn macro_china_market_margin_sz(client: &Client) -> Result<Vec<Jin10MarginRow>> {
    let v = jin10_get(client, "macro_china_market_margin_sz", BASE_JIN10_FS2).await?;
    parse_jin10_margin(&v)
}

/// 上海黄金交易所报告 — Jin10 `sge.json` (akshare/economic/macro_china.py:953).
pub async fn macro_china_au_report(client: &Client) -> Result<Vec<SgeRow>> {
    let v = jin10_get(client, "macro_china_au_report", BASE_JIN10_SGE).await?;
    parse_sge(&v)
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

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

    /// Extract `result.data` from a datacenter fixture (envelope form).
    fn em_data_of(name: &str) -> Vec<Value> {
        emg_data_array(&fixture(name)).unwrap().clone()
    }

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    // ---- Eastmoney shipping (RPT_INDUSTRY_INDEX) ----

    #[test]
    fn parses_macro_shipping_bci() {
        let rows = parse_em_indicator(&em_data_of("macro_shipping_bci.json")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].date, "2026-08-14 00:00:00");
        assert!(approx(rows[0].latest_value, 4538.0));
        assert!(approx(rows[0].change_rate, 1.54396957));
        assert!(approx(rows[0].change_3y, 189.04458599));
    }

    #[test]
    fn parses_macro_shipping_bdi() {
        let rows = parse_em_indicator(&em_data_of("macro_shipping_bdi.json")).unwrap();
        assert_eq!(rows.len(), 3);
        assert!(approx(rows[0].latest_value, 4538.0));
    }

    #[test]
    fn parses_macro_shipping_bpi() {
        let rows = parse_em_indicator(&em_data_of("macro_shipping_bpi.json")).unwrap();
        assert_eq!(rows.len(), 3);
        assert!(approx(rows[0].latest_value, 4538.0));
    }

    #[test]
    fn parses_macro_shipping_bcti() {
        let rows = parse_em_indicator(&em_data_of("macro_shipping_bcti.json")).unwrap();
        assert_eq!(rows.len(), 3);
        assert!(approx(rows[0].latest_value, 4538.0));
    }

    // ---- Jin10 CDN ----

    #[test]
    fn parses_macro_china_shibor_all() {
        let rows = parse_jin10_rate(&fixture("macro_china_shibor_all.json")).unwrap();
        assert_eq!(rows.len(), 16); // 2 dates x 8 terms
        let r = rows
            .iter()
            .find(|x| x.date == "2026-08-14" && x.term == "O/N")
            .unwrap();
        assert!(approx(r.price, 1.3630));
        assert!(approx(r.change, -0.40));
    }

    #[test]
    fn parses_macro_china_hk_market_info() {
        let rows = parse_jin10_rate(&fixture("macro_china_hk_market_info.json")).unwrap();
        assert_eq!(rows.len(), 16); // 2 dates x 8 terms
        let r = rows
            .iter()
            .find(|x| x.date == "2026-08-13" && x.term == "ON")
            .unwrap();
        assert!(approx(r.price, 1.3788));
        assert!(approx(r.change, -0.36));
    }

    #[test]
    fn parses_macro_china_rmb() {
        let rows = parse_jin10_rmb(&fixture("macro_china_rmb.json")).unwrap();
        assert_eq!(rows.len(), 6); // 2 dates x 3 pairs
        let r = rows
            .iter()
            .find(|x| x.date == "2021-05-13" && x.pair == "美元/人民币")
            .unwrap();
        assert!(approx(r.mid, 6.4612));
        assert!(approx(r.change, 354.0));
    }

    #[test]
    fn parses_macro_china_market_margin_sh() {
        let rows = parse_jin10_margin(&fixture("macro_china_market_margin_sh.json")).unwrap();
        assert_eq!(rows.len(), 2);
        let r = rows.iter().find(|x| x.date == "2026-08-13").unwrap();
        assert!(approx(r.financing_buy, 119932012466.0));
        assert!(approx(r.total, 1372937492903.0));
    }

    #[test]
    fn parses_macro_china_market_margin_sz() {
        let rows = parse_jin10_margin(&fixture("macro_china_market_margin_sz.json")).unwrap();
        assert_eq!(rows.len(), 2);
        let r = rows.iter().find(|x| x.date == "2026-08-13").unwrap();
        assert!(approx(r.financing_buy, 121447341612.0));
        // fs_2 may carry nulls for 融券卖出量 / 融券余量.
        assert_eq!(r.securities_sell, None);
        assert!(approx(r.total, 1294408815150.0));
    }

    #[test]
    fn parses_macro_china_au_report() {
        let rows = parse_sge(&fixture("macro_china_au_report.json")).unwrap();
        assert_eq!(rows.len(), 4); // 2 dates x 2 records
        let r = rows
            .iter()
            .find(|x| x.date == "2026-08-14" && x.product.as_deref() == Some("Ag(T+D)"))
            .unwrap();
        assert!(approx(r.open, 15845.0));
        assert!(approx(r.close, 15618.0));
        assert!(approx(r.change_pct, -1.97));
        assert_eq!(r.delivery_dir.as_deref(), Some("多支付给空"));
        assert!(approx(r.delivery_volume, 21480.0));
    }
}
