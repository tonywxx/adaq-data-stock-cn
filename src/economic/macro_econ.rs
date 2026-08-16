//! Macro / economic indicators — shipping freight indices (Eastmoney datacenter)
//! plus deferred Jin10-gated US macro indicators.
//!
//! Ports `akshare/economic/macro_china.py` (`_em_macro_1`) and
//! `akshare/economic/macro_usa.py` (`__macro_usa_base_func`).
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `macro_shipping_bci` | `macro_china.py:2098` | `_em_macro_1("EMI00107666")` |
//! | `macro_shipping_bdi` | `macro_china.py:2109` | `_em_macro_1("EMI00107664")` |
//! | `macro_shipping_bpi` | `macro_china.py:2120` | `_em_macro_1("EMI00107665")` |
//! | `macro_shipping_bcti` | `macro_china.py:2131` | `_em_macro_1("EMI00107669")` |
//!
//! ## DONE
//! 4 Eastmoney `datacenter-web` shipping-index functions, sharing one
//! `ShippingIndexRow` and one parser. They mirror akshare's `_em_macro_1`
//! (reportName `RPT_INDUSTRY_INDEX`, filter by `INDICATOR_ID`, paginated over
//! `result.pages`). Output columns match akshare: 日期 / 最新值 / 涨跌幅 /
//! 近3月涨跌幅 / 近6月涨跌幅 / 近1年涨跌幅 / 近2年涨跌幅 / 近3年涨跌幅.
//!
//! ## DEFERRED
//! * **39 `macro_usa_*` functions** (`macro_usa.py:167`–`942`): every one routes
//!   through `__macro_usa_base_func`, which calls `datacenter-api.jin10.com` with
//!   an `x-csrf-token` header (Jin10 token gate — explicit DEFER trigger).
//!   Listed individually below.
//! * `macro_china_freight_index` (`macro_china.py:3481`): Sina
//!   `quotes.sina.cn/mac/view/vMacExcle.php` JSONP/GBK text-slice response
//!   (Sina `Referer`/`decode("gbk")`); already DEFERRED in `macro_china3.rs`.
//!
//! ### DEFERRED macro_usa functions (Jin10 `x-csrf-token` gate)
//! `macro_usa_adp_employment` (374), `macro_usa_api_crude_stock` (534),
//! `macro_usa_building_permits` (763), `macro_usa_business_inventories` (668),
//! `macro_usa_cb_consumer_confidence` (862), `macro_usa_core_cpi_monthly` (205),
//! `macro_usa_core_pce_price` (392), `macro_usa_core_ppi` (515),
//! `macro_usa_cpi_monthly` (186), `macro_usa_current_account` (448),
//! `macro_usa_durable_goods_orders` (611), `macro_usa_eia_crude_rate` (923),
//! `macro_usa_exist_home_sales` (782), `macro_usa_export_price` (281),
//! `macro_usa_factory_orders` (630), `macro_usa_gdp_monthly` (167),
//! `macro_usa_house_price_index` (801), `macro_usa_house_starts` (725),
//! `macro_usa_import_price` (262), `macro_usa_industrial_production` (592),
//! `macro_usa_initial_jobless` (942), `macro_usa_ism_non_pmi` (687),
//! `macro_usa_ism_pmi` (573), `macro_usa_job_cuts` (338), `macro_usa_lmci` (301),
//! `macro_usa_michigan_consumer_sentiment` (902),
//! `macro_usa_nahb_house_market_index` (706), `macro_usa_new_home_sales` (744),
//! `macro_usa_nfib_small_business` (881), `macro_usa_non_farm` (356),
//! `macro_usa_pending_home_sales` (841), `macro_usa_personal_spending` (224),
//! `macro_usa_pmi` (554), `macro_usa_ppi` (496),
//! `macro_usa_real_consumer_spending` (410), `macro_usa_retail_sales` (243),
//! `macro_usa_services_pmi` (649), `macro_usa_trade_balance` (430),
//! `macro_usa_unemployment_rate` (320).

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE: &str = "eastmoney";
const BASE: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";
const REPORT_NAME: &str = "RPT_INDUSTRY_INDEX";
const COLUMNS: &str =
    "REPORT_DATE,INDICATOR_VALUE,CHANGE_RATE,CHANGERATE_3M,CHANGERATE_6M,CHANGERATE_1Y,CHANGERATE_2Y,CHANGERATE_3Y";

/// One observation of a Baltic/Sina shipping freight index, as returned by
/// Eastmoney `datacenter-web` `RPT_INDUSTRY_INDEX`. Mirrors akshare's renamed
/// `_em_macro_1` columns.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShippingIndexRow {
    /// 日期 — report date (`REPORT_DATE`).
    pub date: String,
    /// 最新值 — latest indicator value (`INDICATOR_VALUE`).
    pub latest_value: Option<f64>,
    /// 涨跌幅 — change rate vs prior (`CHANGE_RATE`).
    pub change_rate: Option<f64>,
    /// 近3月涨跌幅 (`CHANGERATE_3M`).
    pub change_3m: Option<f64>,
    /// 近6月涨跌幅 (`CHANGERATE_6M`).
    pub change_6m: Option<f64>,
    /// 近1年涨跌幅 (`CHANGERATE_1Y`).
    pub change_1y: Option<f64>,
    /// 近2年涨跌幅 (`CHANGERATE_2Y`).
    pub change_2y: Option<f64>,
    /// 近3年涨跌幅 (`CHANGERATE_3Y`).
    pub change_3y: Option<f64>,
}

fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// Parse a single `RPT_INDUSTRY_INDEX` data object into a [`ShippingIndexRow`].
/// Rows missing a `REPORT_DATE` are skipped (return `None`).
pub(crate) fn parse_shipping_item(item: &Value) -> Option<ShippingIndexRow> {
    let date = item.get("REPORT_DATE").and_then(|v| v.as_str())?.to_string();
    Some(ShippingIndexRow {
        date,
        latest_value: item.get("INDICATOR_VALUE").and_then(num),
        change_rate: item.get("CHANGE_RATE").and_then(num),
        change_3m: item.get("CHANGERATE_3M").and_then(num),
        change_6m: item.get("CHANGERATE_6M").and_then(num),
        change_1y: item.get("CHANGERATE_1Y").and_then(num),
        change_2y: item.get("CHANGERATE_2Y").and_then(num),
        change_3y: item.get("CHANGERATE_3Y").and_then(num),
    })
}

/// Parse the `result.data` array of an `RPT_INDUSTRY_INDEX` response into rows.
pub(crate) fn parse_shipping(data: &[Value]) -> Vec<ShippingIndexRow> {
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        if let Some(row) = parse_shipping_item(item) {
            out.push(row);
        }
    }
    out
}

/// Pull the data array out of an Eastmoney `datacenter-web` `v1/get` envelope.
fn emg_data(resp: &Value) -> Result<Vec<Value>> {
    resp.get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .cloned()
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing result.data".into(),
        })
}

/// Read total page count from the envelope (`result.pages`).
fn emg_pages(resp: &Value) -> i64 {
    resp.get("result")
        .and_then(|r| r.get("pages"))
        .and_then(|p| p.as_i64())
        .unwrap_or(1)
}

/// Faithful re-implementation of akshare's `_em_macro_1`: paginate
/// `RPT_INDUSTRY_INDEX` filtered by `INDICATOR_ID`, accumulate `result.data`.
async fn em_macro_1(
    client: &Client,
    endpoint: &'static str,
    indicator_id: &str,
) -> Result<Vec<ShippingIndexRow>> {
    let filter = format!("(INDICATOR_ID=\"{indicator_id}\")");
    let mut data: Vec<Value> = Vec::new();

    // Page 1.
    let first = client
        .get_json(
            SOURCE,
            endpoint,
            BASE,
            &[
                ("sortColumns", "REPORT_DATE"),
                ("sortTypes", "-1"),
                ("pageSize", "500"),
                ("reportName", REPORT_NAME),
                ("columns", COLUMNS),
                ("filter", &filter),
                ("source", "WEB"),
                ("client", "WEB"),
            ],
        )
        .await?;
    data.extend(emg_data(&first)?);

    // Subsequent pages.
    let pages = emg_pages(&first);
    let mut page_strings: Vec<String> = Vec::new();
    for p in 2..=pages {
        page_strings.push(p.to_string());
        let page_ref = page_strings.last().expect("just pushed");
        let v = client
            .get_json(
                SOURCE,
                endpoint,
                BASE,
                &[
                    ("sortColumns", "REPORT_DATE"),
                    ("sortTypes", "-1"),
                    ("pageSize", "500"),
                    ("pageNumber", page_ref),
                    ("reportName", REPORT_NAME),
                    ("columns", COLUMNS),
                    ("filter", &filter),
                    ("source", "WEB"),
                    ("client", "WEB"),
                ],
            )
            .await?;
        data.extend(emg_data(&v)?);
    }

    Ok(parse_shipping(&data))
}

/// 海岬型运费指数 (BCI) — Eastmoney `EMI00107666` (`macro_china.py:2098`).
pub async fn macro_shipping_bci(client: &Client) -> Result<Vec<ShippingIndexRow>> {
    em_macro_1(client, "macro_shipping_bci", "EMI00107666").await
}

/// 波罗的海干散货指数 (BDI) — Eastmoney `EMI00107664` (`macro_china.py:2109`).
pub async fn macro_shipping_bdi(client: &Client) -> Result<Vec<ShippingIndexRow>> {
    em_macro_1(client, "macro_shipping_bdi", "EMI00107664").await
}

/// 巴拿马型运费指数 (BPI) — Eastmoney `EMI00107665` (`macro_china.py:2120`).
pub async fn macro_shipping_bpi(client: &Client) -> Result<Vec<ShippingIndexRow>> {
    em_macro_1(client, "macro_shipping_bpi", "EMI00107665").await
}

/// 成品油运输指数 (BCTI) — Eastmoney `EMI00107669` (`macro_china.py:2131`).
pub async fn macro_shipping_bcti(client: &Client) -> Result<Vec<ShippingIndexRow>> {
    em_macro_1(client, "macro_shipping_bcti", "EMI00107669").await
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

    fn sample() -> Vec<Value> {
        fixture("macro_shipping.json").as_array().unwrap().clone()
    }

    #[test]
    fn parses_shipping_rows() {
        let rows = parse_shipping(&sample());
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].date, "2024-01-02");
        assert!(approx(rows[0].latest_value, 2345.0));
        assert!(approx(rows[0].change_rate, 2.1));
        assert!(approx(rows[0].change_3y, 12.3));
        assert_eq!(rows[2].date, "2024-01-04");
        assert!(approx(rows[2].change_1y, -4.5));
    }

    #[test]
    fn macro_shipping_bci_parse() {
        let rows = parse_shipping(&sample());
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[1].date, "2024-01-03");
        assert!(approx(rows[1].latest_value, 1980.5));
    }

    #[test]
    fn macro_shipping_bdi_parse() {
        let rows = parse_shipping(&sample());
        assert_eq!(rows.len(), 3);
        assert!(approx(rows[0].change_6m, 5.5));
    }

    #[test]
    fn macro_shipping_bpi_parse() {
        let rows = parse_shipping(&sample());
        assert_eq!(rows.len(), 3);
        assert!(approx(rows[2].change_2y, 3.3));
    }

    #[test]
    fn macro_shipping_bcti_parse() {
        let rows = parse_shipping(&sample());
        assert_eq!(rows.len(), 3);
        assert!(approx(rows[1].change_3m, -1.2));
    }
}
