use std::time::Duration;

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

// ---------------------------------------------------------------------------
// bond_zh_us_rate — 东方财富网 中美国债收益率 (datacenter API)
// https://data.eastmoney.com/cjsj/zmgzsyl.html
// ---------------------------------------------------------------------------

const US_RATE_URL: &str = "https://datacenter.eastmoney.com/api/data/get";
/// Static datacenter token — no JS signing required (mirrors akshare).
const US_RATE_TOKEN: &str = "894050c76af8597a853f5b408b759f5d";
const US_RATE_TYPE: &str = "RPTA_WEB_TREASURYYIELD";
const US_RATE_PS: u32 = 500;

/// China/US treasury yield row (`bond_zh_us_rate`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondZhUsRate {
    pub date: String,
    pub cn_yield_2y: Option<f64>,
    pub cn_yield_5y: Option<f64>,
    pub cn_yield_10y: Option<f64>,
    pub cn_yield_30y: Option<f64>,
    pub cn_yield_10y_2y: Option<f64>,
    pub cn_gdp_yoy: Option<f64>,
    pub us_yield_2y: Option<f64>,
    pub us_yield_5y: Option<f64>,
    pub us_yield_10y: Option<f64>,
    pub us_yield_30y: Option<f64>,
    pub us_yield_10y_2y: Option<f64>,
    pub us_gdp_yoy: Option<f64>,
    pub source: &'static str,
}

impl BondZhUsRate {
    fn new(date: String) -> Self {
        Self {
            date,
            cn_yield_2y: None,
            cn_yield_5y: None,
            cn_yield_10y: None,
            cn_yield_30y: None,
            cn_yield_10y_2y: None,
            cn_gdp_yoy: None,
            us_yield_2y: None,
            us_yield_5y: None,
            us_yield_10y: None,
            us_yield_30y: None,
            us_yield_10y_2y: None,
            us_gdp_yoy: None,
            source: SOURCE_EASTMONEY,
        }
    }
}

/// China/US treasury yields from Eastmoney (`bond_zh_us_rate`).
///
/// Walks datacenter `result.pages`, accumulating `result.data`. The akshare
/// `start_date` (YYYYMMDD, default `19901219`) truncates the result to that date.
pub async fn bond_zh_us_rate(client: &Client, start_date: &str) -> Result<Vec<BondZhUsRate>> {
    let cutoff = fmt_date(start_date);
    let mut out = Vec::new();
    let mut page: u32 = 1;
    loop {
        let page_s = page.to_string();
        let ps_s = US_RATE_PS.to_string();
        let params = [
            ("type", US_RATE_TYPE),
            ("sty", "ALL"),
            ("st", "SOLAR_DATE"),
            ("sr", "-1"),
            ("token", US_RATE_TOKEN),
            ("p", page_s.as_str()),
            ("ps", ps_s.as_str()),
            ("pageNo", page_s.as_str()),
            ("pageNum", page_s.as_str()),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, "bond_zh_us_rate", US_RATE_URL, &params)
            .await?;
        let data = v
            .get("result")
            .and_then(|r| r.get("data"))
            .and_then(|d| d.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing result.data".into(),
            })?;
        if data.is_empty() {
            break;
        }
        let mut page_rows = parse_us_rate(&v)?;
        page_rows.retain(|r| r.date.as_str() >= cutoff.as_str());
        out.extend(page_rows);
        let total_pages = v
            .get("result")
            .and_then(|r| r.get("pages"))
            .and_then(|p| p.as_u64())
            .unwrap_or(1);
        if page >= total_pages as u32 {
            break;
        }
        page += 1;
        tokio::time::sleep(Duration::from_millis(800)).await;
    }
    Ok(out)
}

pub(crate) fn parse_us_rate(resp: &Value) -> Result<Vec<BondZhUsRate>> {
    let data = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data".into(),
        })?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        if let Some(row) = parse_us_rate_item(item) {
            out.push(row);
        }
    }
    Ok(out)
}

fn parse_us_rate_item(item: &Value) -> Option<BondZhUsRate> {
    let date = item.get("SOLAR_DATE").and_then(|v| v.as_str())?.to_string();
    let mut row = BondZhUsRate::new(date);
    row.cn_yield_2y = num(item.get("EMM00588704"));
    row.cn_yield_5y = num(item.get("EMM00166462"));
    row.cn_yield_10y = num(item.get("EMM00166466"));
    row.cn_yield_30y = num(item.get("EMM00166469"));
    row.cn_yield_10y_2y = num(item.get("EMM01276014"));
    row.cn_gdp_yoy = num(item.get("EMM00000024"));
    row.us_yield_2y = num(item.get("EMG00001306"));
    row.us_yield_5y = num(item.get("EMG00001308"));
    row.us_yield_10y = num(item.get("EMG00001310"));
    row.us_yield_30y = num(item.get("EMG00001312"));
    row.us_yield_10y_2y = num(item.get("EMG01339436"));
    row.us_gdp_yoy = num(item.get("EMG00159635"));
    Some(row)
}

// ---------------------------------------------------------------------------
// bond_cov_comparison — 东方财富网 可转债比价表 (push2 clist, `diff` pattern)
// https://quote.eastmoney.com/center/fullscreenlist.html#convertible_comparison
// ---------------------------------------------------------------------------

const COV_COMP_URL: &str = "https://16.push2.eastmoney.com/api/qt/clist/get";
/// Static Eastmoney `ut` token — no JS signing required (ADR-0005).
const COV_UT: &str = "bd1d9ddb04089700cf9c27f6f7426281";
const COV_FIELDS: &str = "f1,f152,f2,f3,f12,f13,f14,f227,f228,f229,f230,f231,f232,f233,f234,f235,f236,f237,f238,f239,f240,f241,f242,f26,f243";

/// Convertible-bond comparison row (`bond_cov_comparison`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondCovComparison {
    pub code: String,
    pub name: String,
    pub latest_price: Option<f64>,
    pub pct_change: Option<f64>,
    pub stock_code: String,
    pub stock_name: String,
    pub stock_price: Option<f64>,
    pub stock_pct_change: Option<f64>,
    pub transfer_price: Option<f64>,
    pub transfer_value: Option<f64>,
    pub transfer_premium_ratio: Option<f64>,
    pub pure_bond_premium_ratio: Option<f64>,
    pub resale_trigger_price: Option<f64>,
    pub redeem_trigger_price: Option<f64>,
    pub pure_bond_value: Option<f64>,
    pub maturity_redeem_price: Option<f64>,
    pub listing_date: String,
    pub start_transfer_date: String,
    pub source: &'static str,
}

impl BondCovComparison {
    fn new() -> Self {
        Self {
            code: String::new(),
            name: String::new(),
            latest_price: None,
            pct_change: None,
            stock_code: String::new(),
            stock_name: String::new(),
            stock_price: None,
            stock_pct_change: None,
            transfer_price: None,
            transfer_value: None,
            transfer_premium_ratio: None,
            pure_bond_premium_ratio: None,
            resale_trigger_price: None,
            redeem_trigger_price: None,
            pure_bond_value: None,
            maturity_redeem_price: None,
            listing_date: String::new(),
            start_transfer_date: String::new(),
            source: SOURCE_EASTMONEY,
        }
    }
}

/// Convertible-bond comparison table from Eastmoney (`bond_cov_comparison`).
///
/// Single push2 page (pz=1000). Mirrors the `stock_zh_a_spot_em` / `forex_spot_em`
/// `clist/get` + `data.diff` shape.
pub async fn bond_cov_comparison(client: &Client) -> Result<Vec<BondCovComparison>> {
    let pz_s = "1000".to_string();
    let params = [
        ("pn", "1"),
        ("pz", pz_s.as_str()),
        ("po", "1"),
        ("np", "1"),
        ("ut", COV_UT),
        ("fltt", "2"),
        ("invt", "2"),
        ("fid", "f243"),
        ("fs", "b:MK0354"),
        ("fields", COV_FIELDS),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "bond_cov_comparison",
            COV_COMP_URL,
            &params,
        )
        .await?;
    parse_cov_comparison(&v)
}

pub(crate) fn parse_cov_comparison(resp: &Value) -> Result<Vec<BondCovComparison>> {
    let diff = resp
        .get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff".into(),
        })?;
    let mut out = Vec::with_capacity(diff.len());
    for item in diff {
        out.push(parse_cov_item(item));
    }
    Ok(out)
}

fn parse_cov_item(item: &Value) -> BondCovComparison {
    let mut row = BondCovComparison::new();
    row.code = fstr(item, "f12");
    row.name = fstr(item, "f14");
    row.latest_price = fnum(item, "f2");
    row.pct_change = fnum(item, "f3");
    row.stock_code = fstr(item, "f234");
    row.stock_name = fstr(item, "f236");
    row.stock_price = fnum(item, "f231");
    row.stock_pct_change = fnum(item, "f232");
    row.transfer_price = fnum(item, "f237");
    row.transfer_value = fnum(item, "f238");
    row.transfer_premium_ratio = fnum(item, "f239");
    row.pure_bond_premium_ratio = fnum(item, "f240");
    row.resale_trigger_price = fnum(item, "f241");
    row.redeem_trigger_price = fnum(item, "f242");
    row.pure_bond_value = fnum(item, "f229");
    row.maturity_redeem_price = fnum(item, "f243");
    row.listing_date = fstr(item, "f227");
    row.start_transfer_date = fstr(item, "f26");
    row
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Convert akshare-style `YYYYMMDD` into `YYYY-MM-DD` (ISO sorts correctly).
pub(crate) fn fmt_date(s: &str) -> String {
    if s.len() == 8 {
        format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8])
    } else {
        s.to_string()
    }
}

fn fstr(item: &Value, k: &str) -> String {
    item.get(k)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    })
}

fn num(v: Option<&Value>) -> Option<f64> {
    v.and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    })
}

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
    fn parses_us_rate_fixture() {
        let v = fixture("bond_zh_us_rate.json");
        let rows = parse_us_rate(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02");
        assert_eq!(rows[0].cn_yield_10y, Some(2.60));
        assert_eq!(rows[0].us_yield_10y, Some(4.30));
        assert_eq!(rows[0].us_yield_10y_2y, Some(0.20));
        assert_eq!(rows[0].cn_gdp_yoy, Some(5.20));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].cn_yield_2y, Some(2.12));
    }

    #[test]
    fn parses_cov_comparison_fixture() {
        let v = fixture("bond_cov_comparison.json");
        let rows = parse_cov_comparison(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "123000");
        assert_eq!(rows[0].name, "普利转债");
        assert_eq!(rows[0].latest_price, Some(120.5));
        assert_eq!(rows[0].transfer_price, Some(40.0));
        assert_eq!(rows[0].transfer_value, Some(63.25));
        assert_eq!(rows[0].transfer_premium_ratio, Some(90.5));
        assert_eq!(rows[0].stock_code, "300630");
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].code, "113000");
    }
}
