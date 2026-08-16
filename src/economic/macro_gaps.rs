//! Bridge gap functions for the economic domain.
//!
//! Ports a small set of akshare endpoints that did not fit the existing
//! `macro_*` modules:
//!
//! - `macro_china_swap_rate` — ChinaMoney FR007 interest-rate-swap curve
//!   (`bond/bond_china_money.py`). ChinaMoney POST; the live endpoint
//!   currently returns an empty `records` array / `500` for out-of-range or
//!   degraded requests, so the parser is written against the documented
//!   `records[].{日期, 曲线名称, data[11]}` shape and exercised against a
//!   structurally-faithful fixture.
//! - `macro_china_freight_index` — Sina CSV (GBK) freight-index report
//!   (`economic/macro_china.py`). NOTE: contrary to the porting brief the
//!   live endpoint at this line is a CSV download, not a jsonp callback.
//! - `macro_china_daily_energy` — Jin10 `.js` wrapper around a JSON document
//!   (`economic/macro_china.py`).
//! - `macro_euro_lme_holding` / `macro_euro_lme_stock` — Jin10 CDN JSON
//!   (`economic/macro_euro.py`).

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_CHINAMONEY: &str = "chinamoney";
const SOURCE_SINA: &str = "sina";
const SOURCE_JIN10: &str = "jin10";

/// Parse a JSON scalar into `f64`, tolerating string-encoded numbers and blanks.
fn as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                t.parse::<f64>().ok()
            }
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// macro_china_swap_rate — ChinaMoney FR007 IRS curve history
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct SwapRate {
    /// Quote date (`YYYY-MM-DD`).
    pub date: String,
    /// Curve name (akshare `曲线名称`, e.g. `FR007`).
    pub curve_name: String,
    /// 1M tenor.
    pub m1: Option<f64>,
    /// 3M tenor.
    pub m3: Option<f64>,
    /// 6M tenor.
    pub m6: Option<f64>,
    /// 9M tenor.
    pub m9: Option<f64>,
    /// 1Y tenor.
    pub y1: Option<f64>,
    /// 2Y tenor.
    pub y2: Option<f64>,
    /// 3Y tenor.
    pub y3: Option<f64>,
    /// 4Y tenor.
    pub y4: Option<f64>,
    /// 5Y tenor.
    pub y5: Option<f64>,
    /// 7Y tenor.
    pub y7: Option<f64>,
    /// 10Y tenor.
    pub y10: Option<f64>,
}

/// FR007 interest-rate-swap curve history from ChinaMoney
/// (`macro_china_swap_rate`, akshare `bond/bond_china_money.py:192`).
///
/// `start_date` / `end_date` use the `YYYYMMDD` form and must span at most one
/// month (ChinaMoney constraint); defaults match akshare.
pub async fn macro_china_swap_rate(
    client: &Client,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<SwapRate>> {
    let sd = format!(
        "{}-{}-{}",
        &start_date[..4],
        &start_date[4..6],
        &start_date[6..8]
    );
    let ed = format!(
        "{}-{}-{}",
        &end_date[..4],
        &end_date[4..6],
        &end_date[6..8]
    );
    let url = "https://www.chinamoney.com.cn/ags/ms/cm-u-bk-shibor/IfccHis";
    let params: &[(&str, &str)] = &[
        ("cfgItemType", "72"),
        ("interestRateType", "0"),
        ("startDate", &sd),
        ("endDate", &ed),
        ("bidAskType", ""),
        ("lang", "CN"),
        ("quoteTime", "全部"),
        ("pageSize", "5000"),
        ("pageNum", "1"),
    ];
    let headers: &[(&str, &str)] = &[
        ("X-Requested-With", "XMLHttpRequest"),
        (
            "Referer",
            "https://www.chinamoney.com.cn/chinese/bkcurvfxhis/?cfgItemType=72&curveType=FR007",
        ),
    ];
    let v = client
        .post_form_json(SOURCE_CHINAMONEY, "macro_china_swap_rate", url, params, Some(headers))
        .await?;
    parse_macro_china_swap_rate(&v)
}

pub(crate) fn parse_macro_china_swap_rate(resp: &Value) -> Result<Vec<SwapRate>> {
    let records = resp
        .get("records")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CHINAMONEY,
            message: "missing records".into(),
        })?;
    let mut out = Vec::with_capacity(records.len());
    for rec in records {
        let date = rec
            .get("日期")
            .or_else(|| rec.get("date"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let curve_name = rec
            .get("曲线名称")
            .or_else(|| rec.get("curveName"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let data = rec
            .get("data")
            .or_else(|| rec.get("items"))
            .or_else(|| rec.get("curveData"))
            .and_then(|v| v.as_array());
        let get = |i: usize| -> Option<f64> { data.and_then(|a| a.get(i)).and_then(as_f64) };
        out.push(SwapRate {
            date,
            curve_name,
            m1: get(0),
            m3: get(1),
            m6: get(2),
            m9: get(3),
            y1: get(4),
            y2: get(5),
            y3: get(6),
            y4: get(7),
            y5: get(8),
            y7: get(9),
            y10: get(10),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// macro_china_freight_index — Sina CSV freight index report
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct FreightIndex {
    /// Report date (`截止日期`, `YYYY-MM-DD`).
    pub date: String,
    /// 波罗的海好望角型船运价指数 BCI.
    pub bci: Option<f64>,
    /// 灵便型船综合运价指数 BHMI.
    pub bhmi: Option<f64>,
    /// 波罗的海超级大灵便型船 BSI 指数.
    pub bsi: Option<f64>,
    /// 波罗的海综合运价指数 BDI.
    pub bdi: Option<f64>,
    /// HRCI 国际集装箱租船指数.
    pub hrci: Option<f64>,
    /// 油轮运价指数成品油运价指数 BCTI.
    pub bcti: Option<f64>,
    /// 油轮运价指数原油运价指数 BDTI.
    pub bdti: Option<f64>,
}

/// China freight shipping indices from Sina
/// (`macro_china_freight_index`, akshare `economic/macro_china.py:3481`).
///
/// The live response is a GBK-encoded CSV. The client's `get_text` decodes as
/// UTF-8; in production a GBK-aware decode may be required — the parser here
/// operates on the already-decoded text.
pub async fn macro_china_freight_index(client: &Client) -> Result<Vec<FreightIndex>> {
    let url = "http://quotes.sina.cn/mac/view/vMacExcle.php";
    let params: &[(&str, &str)] = &[
        ("cate", "industry"),
        ("event", "22"),
        ("from", "0"),
        ("num", "5000"),
        ("condition", ""),
    ];
    let text = client
        .get_text(SOURCE_SINA, "macro_china_freight_index", url, params, None)
        .await?;
    parse_macro_china_freight_index(&text)
}

pub(crate) fn parse_macro_china_freight_index(resp: &str) -> Result<Vec<FreightIndex>> {
    // Header is the line that starts with the `截止日期` column label.
    let mut lines = resp.lines();
    let mut header: Option<&str> = None;
    for line in lines.by_ref() {
        if line.contains("截止日期") {
            header = Some(line);
            break;
        }
    }
    let _ = header.ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "freight csv header not found".into(),
    })?;

    let mut out = Vec::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if parts.last().map_or(false, |s| s.is_empty()) {
            parts.pop();
        }
        if parts.len() < 2 {
            continue;
        }
        let date = parts[0].to_string();
        let val = |i: usize| -> Option<f64> {
            parts
                .get(1 + i)
                .and_then(|s| as_f64(&Value::String(s.to_string())))
        };
        out.push(FreightIndex {
            date,
            bci: val(0),
            bhmi: val(1),
            bsi: val(2),
            bdi: val(3),
            hrci: val(4),
            bcti: val(5),
            bdti: val(6),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// macro_china_daily_energy — Jin10 embedded-JSON .js report
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct DailyEnergy {
    /// Report date (`YYYYMMDD`).
    pub date: String,
    /// 沿海六大电库存 — coastal six-utility inventory (万吨).
    pub inventory: Option<f64>,
    /// 日耗 — daily consumption.
    pub daily_consumption: Option<f64>,
    /// 存煤可用天数 — coal-availability days.
    pub available_days: Option<f64>,
}

/// China daily coastal six-utility energy inventory
/// (`macro_china_daily_energy`, akshare `economic/macro_china.py:750`).
pub async fn macro_china_daily_energy(client: &Client) -> Result<Vec<DailyEnergy>> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let url = format!(
        "https://cdn.jin10.com/dc/reports/dc_qihuo_energy_report_all.js?v{ts}&_{ts}"
    );
    let text = client
        .get_text(SOURCE_JIN10, "macro_china_daily_energy", &url, &[], None)
        .await?;
    parse_macro_china_daily_energy(&text)
}

pub(crate) fn parse_macro_china_daily_energy(resp: &str) -> Result<Vec<DailyEnergy>> {
    // The .js file wraps a JSON document: `var dataCenter_data = {...};`.
    let start = resp.find('{').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_JIN10,
        message: "energy js missing json".into(),
    })?;
    let end = resp.rfind('}').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_JIN10,
        message: "energy js missing json".into(),
    })?;
    let v: Value = serde_json::from_str(&resp[start..=end]).map_err(|e| Error::Parse {
        endpoint: "macro_china_daily_energy",
        message: e.to_string(),
    })?;
    let list = v
        .get("list")
        .and_then(|l| l.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_JIN10,
            message: "missing list".into(),
        })?;
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        let date = item
            .get("date")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();
        let datas = item.get("datas").and_then(|d| d.as_object());
        let series = datas
            .and_then(|o| o.values().next())
            .and_then(|a| a.as_array());
        let get = |i: usize| -> Option<f64> { series.and_then(|a| a.get(i)).and_then(as_f64) };
        out.push(DailyEnergy {
            date,
            inventory: get(0),
            daily_consumption: get(1),
            available_days: get(2),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// macro_euro_lme_holding / macro_euro_lme_stock — Jin10 CDN JSON
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct LmeHolding {
    /// Report date (`YYYY-MM-DD`).
    pub date: String,
    /// Metal name (e.g. `铜`).
    pub product: String,
    /// 多头仓位 — long position (手).
    pub long_position: Option<f64>,
    /// 空头仓位 — short position (手).
    pub short_position: Option<f64>,
    /// 净仓位 — net position (手).
    pub net_position: Option<f64>,
}

/// LME open-interest / positions report
/// (`macro_euro_lme_holding`, akshare `economic/macro_euro.py:839`).
pub async fn macro_euro_lme_holding(client: &Client) -> Result<Vec<LmeHolding>> {
    let url = "https://cdn.jin10.com/data_center/reports/lme_position.json";
    let v = client
        .get_json(SOURCE_JIN10, "macro_euro_lme_holding", url, &[("_", "1")])
        .await?;
    parse_macro_euro_lme_holding(&v)
}

pub(crate) fn parse_macro_euro_lme_holding(resp: &Value) -> Result<Vec<LmeHolding>> {
    let values = resp
        .get("values")
        .and_then(|v| v.as_object())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_JIN10,
            message: "missing values".into(),
        })?;
    let mut out = Vec::new();
    for (date, products) in values {
        // akshare drops the `1899-11-30` grand-total sentinel row.
        if date == "1899-11-30" {
            continue;
        }
        let Some(products) = products.as_object() else {
            continue;
        };
        for (product, arr) in products {
            let Some(arr) = arr.as_array() else {
                continue;
            };
            out.push(LmeHolding {
                date: date.clone(),
                product: product.clone(),
                long_position: arr.first().and_then(as_f64),
                short_position: arr.get(1).and_then(as_f64),
                net_position: arr.get(2).and_then(as_f64),
            });
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LmeStock {
    /// Report date (`YYYY-MM-DD`).
    pub date: String,
    /// Metal name (e.g. `铜`).
    pub product: String,
    /// 库存 — inventory (吨).
    pub inventory: Option<f64>,
    /// 注册仓单 — registered warrants (吨).
    pub registered_warrant: Option<f64>,
    /// 注销仓单 — cancelled warrants (吨).
    pub cancelled_warrant: Option<f64>,
}

/// LME warehouse-stock report
/// (`macro_euro_lme_stock`, akshare `economic/macro_euro.py:870`).
pub async fn macro_euro_lme_stock(client: &Client) -> Result<Vec<LmeStock>> {
    let url = "https://cdn.jin10.com/data_center/reports/lme_stock.json";
    let v = client
        .get_json(SOURCE_JIN10, "macro_euro_lme_stock", url, &[("_", "1")])
        .await?;
    parse_macro_euro_lme_stock(&v)
}

pub(crate) fn parse_macro_euro_lme_stock(resp: &Value) -> Result<Vec<LmeStock>> {
    let values = resp
        .get("values")
        .and_then(|v| v.as_object())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_JIN10,
            message: "missing values".into(),
        })?;
    let mut out = Vec::new();
    for (date, products) in values {
        let Some(products) = products.as_object() else {
            continue;
        };
        for (product, arr) in products {
            let Some(arr) = arr.as_array() else {
                continue;
            };
            out.push(LmeStock {
                date: date.clone(),
                product: product.clone(),
                inventory: arr.first().and_then(as_f64),
                registered_warrant: arr.get(1).and_then(as_f64),
                cancelled_warrant: arr.get(2).and_then(as_f64),
            });
        }
    }
    Ok(out)
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

    fn fixture_text(name: &str) -> String {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        std::fs::read_to_string(p).unwrap()
    }

    #[test]
    fn parses_macro_china_swap_rate() {
        let rows = parse_macro_china_swap_rate(&fixture("macro_china_swap_rate.json")).unwrap();
        assert!(!rows.is_empty(), "expected >0 rows");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2023-11-01");
        assert_eq!(rows[0].curve_name, "FR007");
        assert_eq!(rows[0].m1, Some(2.05));
        assert_eq!(rows[0].y10, Some(3.00));
        assert_eq!(rows[1].m3, Some(2.31));
    }

    #[test]
    fn parses_macro_china_freight_index() {
        let rows =
            parse_macro_china_freight_index(&fixture_text("macro_china_freight_index.txt")).unwrap();
        assert!(!rows.is_empty(), "expected >0 rows");
        // Latest date in fixture is 2026-08-13.
        let first = &rows[0];
        assert_eq!(first.date, "2026-08-13");
        assert_eq!(first.bci, Some(4469.00));
        assert_eq!(first.bhmi, None); // blank column
        assert_eq!(first.bsi, Some(1613.00));
        assert_eq!(first.bdi, Some(2844.00));
        assert_eq!(first.bcti, Some(1313.00));
        assert_eq!(first.bdti, Some(2693.00));
    }

    #[test]
    fn parses_macro_china_daily_energy() {
        let rows =
            parse_macro_china_daily_energy(&fixture_text("macro_china_daily_energy.js")).unwrap();
        assert!(!rows.is_empty(), "expected >0 rows");
        assert_eq!(rows[0].date, "20160101");
        assert_eq!(rows[0].inventory, Some(1167.60));
        assert_eq!(rows[0].daily_consumption, Some(64.20));
        assert_eq!(rows[0].available_days, Some(18.19));
    }

    #[test]
    fn parses_macro_euro_lme_holding() {
        let rows = parse_macro_euro_lme_holding(&fixture("macro_euro_lme_holding.json")).unwrap();
        assert!(!rows.is_empty(), "expected >0 rows");
        // Sentinel 1899-11-30 must be dropped.
        assert!(!rows.iter().any(|r| r.date == "1899-11-30"));
        let r = rows
            .iter()
            .find(|r| r.date == "2021-09-03" && r.product == "铜")
            .expect("2021-09-03 铜 row");
        assert_eq!(r.long_position, Some(7453.93));
        assert_eq!(r.short_position, Some(10574.36));
        assert_eq!(r.net_position, Some(-3120.43));
    }

    #[test]
    fn parses_macro_euro_lme_stock() {
        let rows = parse_macro_euro_lme_stock(&fixture("macro_euro_lme_stock.json")).unwrap();
        assert!(!rows.is_empty(), "expected >0 rows");
        let r = rows
            .iter()
            .find(|r| r.date == "2026-07-17" && r.product == "镍")
            .expect("2026-07-17 镍 row");
        assert_eq!(r.inventory, Some(274284.0));
        assert_eq!(r.registered_warrant, Some(258084.0));
        assert_eq!(r.cancelled_warrant, Some(16200.0));
    }
}
