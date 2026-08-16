//! US (美股) stock data. Ports four akshare sources.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `stock_us_famous_spot_em` | `stock/stock_us_js.py:13` (`stock_price_js`) | 美股/港股目标价, ushknews JSON API (feasible) |
//! | `stock_us_pink_spot_em` | `stock/stock_us_pink.py:15` | 东方财富粉单市场, push2 `clist` (feasible, hardcoded `ut`) |
//! | `stock_us_daily` | `stock/stock_us_sina.py:117` | DEFERRED — JS decryption (`zh_js_decode`) |
//! | `stock_us_spot` | `stock/stock_us_sina.py:86` | DEFERRED — JS-signed hash (`js_hash_text`) |
//! | `get_us_stock_name` | `stock/stock_us_sina.py:55` | DEFERRED — JS-signed hash (`js_hash_text`) |
//! | `stock_us_valuation_baidu` | `stock_feature/stock_us_valuation_baidu.py:16` | DEFERRED per task — Baidu HTML/nested scrape |
//!
//! ## DEFERRED
//!
//! * **`stock_us_daily`** (`stock/stock_us_sina.py:117`): fetches
//!   `https://finance.sina.com.cn/staticdata/us/{symbol}` then runs the upstream
//!   response through `zh_js_decode` executed by `py_mini_racer.MiniRacer()`
//!   (`js_code.call("d", ...)`). The historical bars are JS-encrypted; pure-Rust
//!   replication needs a JS engine, so this is deferred (JS-signed, per
//!   PORTING_GUIDE rule 4).
//! * **`stock_us_spot`** (`stock/stock_us_sina.py:86`): paginates
//!   `US_CategoryService.getList`, but the request URL embeds a hash computed by
//!   `js_hash_text` in `py_mini_racer` (`js_code.call("d", ...)`). The hash is
//!   JS-signed, so deferred.
//! * **`get_us_stock_name`** (`stock/stock_us_sina.py:55`): same JS-signed
//!   `js_hash_text` hash as `stock_us_spot`; deferred.
//! * **`stock_us_valuation_baidu`** (`stock_feature/stock_us_valuation_baidu.py:16`):
//!   DEFERRED per task — the upstream `gushitong.baidu.com/opendata` response is
//!   scraped from a deep nested path
//!   `Result[0].DisplayData.resultData.tplData.result.chartInfo[0].body` whose
//!   schema is unstable and frequently returns HTML error pages.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_USHKNEWS: &str = "ushknews";
const SOURCE_EASTMONEY: &str = "eastmoney";

const FAMOUS_URL: &str = "https://calendar-api.ushknews.com/getWebTargetPriceList";
const FAMOUS_HEADERS: &[(&str, &str)] = &[
    ("Referer", "https://www.ushknews.com/"),
    ("x-app-id", "BNsiR9uq7yfW0LVz"),
    ("x-version", "1.0.0"),
];

const PINK_URL: &str = "https://23.push2.eastmoney.com/api/qt/clist/get";
/// Hardcoded Eastmoney push2 token (akshare `stock_us_pink.py:35`). Not
/// dynamically signed for this endpoint, so safe to reuse.
const PINK_UT: &str = "bd1d9ddb04089700cf9c27f6f7426281";
const PINK_PAGE_SIZE: u32 = 100;

fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(str::to_string)
}

fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

// ===========================================================================
// stock_us_famous_spot_em  (akshare stock_us_js.py:13, stock_price_js)
// ===========================================================================

/// US/HK stock target-price (目标价) rows from the ushknews API.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UsFamousSpotRow {
    /// 日期 (inner-list index 7)
    pub date: String,
    /// 个股名称 (inner-list index 9)
    pub stock_name: String,
    /// 评级 (inner-list index 2)
    pub rating: String,
    /// 先前目标价 (inner-list index 5)
    pub prev_target_price: Option<f64>,
    /// 最新目标价 (inner-list index 4)
    pub latest_target_price: Option<f64>,
    /// 机构名称 (inner-list index 6)
    pub institution: String,
}

fn arr_str(a: &[Value], idx: usize) -> Option<String> {
    a.get(idx).and_then(|v| v.as_str()).map(str::to_string)
}

fn arr_num(a: &[Value], idx: usize) -> Option<f64> {
    a.get(idx).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

/// Parse `stock_us_famous_spot_em` rows from the full ushknews response.
///
/// The upstream returns `{"data": {"list": [[...], ...]}}` where every element
/// is a 12-element array. We map the meaningful indices (akshare reorders the
/// final DataFrame to 日期, 个股名称, 评级, 先前目标价, 最新目标价, 机构名称).
pub(crate) fn parse_stock_us_famous_spot_em(resp: &Value) -> Result<Vec<UsFamousSpotRow>> {
    let list = resp
        .get("data")
        .and_then(|d| d.get("list"))
        .and_then(|l| l.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_USHKNEWS,
            message: "missing data.list".into(),
        })?;
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        let Some(arr) = item.as_array() else {
            continue;
        };
        out.push(UsFamousSpotRow {
            date: arr_str(arr, 7).unwrap_or_default(),
            stock_name: arr_str(arr, 9).unwrap_or_default(),
            rating: arr_str(arr, 2).unwrap_or_default(),
            prev_target_price: arr_num(arr, 5),
            latest_target_price: arr_num(arr, 4),
            institution: arr_str(arr, 6).unwrap_or_default(),
        });
    }
    Ok(out)
}

/// 美股目标价 (ushknews `getWebTargetPriceList`, category `us`).
pub async fn stock_us_famous_spot_em(client: &Client) -> Result<Vec<UsFamousSpotRow>> {
    stock_us_famous_spot_em_opts(client, "us").await
}

/// 美股/港股目标价 with explicit `symbol` ∈ {"us", "hk"}.
pub async fn stock_us_famous_spot_em_opts(
    client: &Client,
    symbol: &str,
) -> Result<Vec<UsFamousSpotRow>> {
    let v = client
        .get_json_with_headers(
            SOURCE_USHKNEWS,
            "stock_us_famous_spot_em",
            FAMOUS_URL,
            &[("limit", "20"), ("category", symbol)],
            Some(FAMOUS_HEADERS),
        )
        .await?;
    parse_stock_us_famous_spot_em(&v)
}

// ===========================================================================
// stock_us_pink_spot_em  (akshare stock_us_pink.py:15)
// ===========================================================================

/// 东方财富-粉单市场 (US pink-sheet) real-time spot quotes.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UsPinkSpotRow {
    /// 代码 (Eastmoney `f14` + "." + `f13`)
    pub code: String,
    /// 名称 (Eastmoney `f15`)
    pub name: String,
    /// 最新价 (Eastmoney `f2`)
    pub latest_price: Option<f64>,
    /// 涨跌额 (Eastmoney `f4`)
    pub change: Option<f64>,
    /// 涨跌幅 (Eastmoney `f3`)
    pub pct_change: Option<f64>,
    /// 开盘价 (Eastmoney `f18`)
    pub open: Option<f64>,
    /// 最高价 (Eastmoney `f16`)
    pub high: Option<f64>,
    /// 最低价 (Eastmoney `f17`)
    pub low: Option<f64>,
    /// 昨收价 (Eastmoney `f20`)
    pub pre_close: Option<f64>,
    /// 总市值 (Eastmoney `f21`)
    pub total_mv: Option<f64>,
    /// 市盈率 (Eastmoney `f128`)
    pub pe: Option<f64>,
}

/// Extract the `data.diff` array (one page) from a push2 clist response.
fn pink_diff(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff".into(),
        })
}

/// Parse one page of `stock_us_pink_spot_em` rows from a push2 `clist` response.
pub(crate) fn parse_stock_us_pink_spot_em(resp: &Value) -> Result<Vec<UsPinkSpotRow>> {
    let diff = pink_diff(resp)?;
    let mut out = Vec::with_capacity(diff.len());
    for item in diff {
        let code = format!(
            "{}.{}",
            fstr(item, "f14").unwrap_or_default(),
            fstr(item, "f13").unwrap_or_default()
        );
        out.push(UsPinkSpotRow {
            code,
            name: fstr(item, "f15").unwrap_or_default(),
            latest_price: fnum(item, "f2"),
            change: fnum(item, "f4"),
            pct_change: fnum(item, "f3"),
            open: fnum(item, "f18"),
            high: fnum(item, "f16"),
            low: fnum(item, "f17"),
            pre_close: fnum(item, "f20"),
            total_mv: fnum(item, "f21"),
            pe: fnum(item, "f128"),
        });
    }
    Ok(out)
}

/// Build the push2 `clist` query params for a given 1-based `page` (owned so the
/// dynamic `pn`/`pz` values can be borrowed without leaking).
fn pink_params_raw(page: u32) -> Vec<(String, String)> {
    vec![
        ("np".into(), "1".into()),
        ("fltt".into(), "1".into()),
        ("invt".into(), "1".into()),
        ("fs".into(), "m:153".into()),
        (
            "fields".into(),
            "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f12,f13,f14,f15,f16,f17,f18,f20,f21,f23,f24,f25,f26,f22,f33,f11,f62,f128,f136,f115,f152".into(),
        ),
        ("fid".into(), "f3".into()),
        ("pn".into(), page.to_string()),
        ("pz".into(), PINK_PAGE_SIZE.to_string()),
        ("po".into(), "1".into()),
        ("dect".into(), "1".into()),
        ("ut".into(), PINK_UT.into()),
    ]
}

/// 东方财富网-行情中心-美股市场-粉单市场 (push2 `clist`, `fs=m:153`).
///
/// Walks every page (100 rows/page) like akshare; aggregates all pink-sheet rows.
pub async fn stock_us_pink_spot_em(client: &Client) -> Result<Vec<UsPinkSpotRow>> {
    let raw = pink_params_raw(1);
    let params: Vec<(&str, &str)> = raw.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let first = client
        .get_json(SOURCE_EASTMONEY, "stock_us_pink_spot_em", PINK_URL, &params)
        .await?;
    let total = first
        .get("data")
        .and_then(|d| d.get("total"))
        .and_then(|t| t.as_u64())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.total".into(),
        })?;
    let pages = total.div_ceil(u64::from(PINK_PAGE_SIZE));

    let mut out = parse_stock_us_pink_spot_em(&first)?;
    for page in 2..=pages {
        let raw = pink_params_raw(page as u32);
        let params: Vec<(&str, &str)> = raw.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let v = client
            .get_json(SOURCE_EASTMONEY, "stock_us_pink_spot_em", PINK_URL, &params)
            .await?;
        out.extend(parse_stock_us_pink_spot_em(&v)?);
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
    fn parse_stock_us_famous_spot_em_ok() {
        let rows = parse_stock_us_famous_spot_em(&fixture("stock_us_famous_spot_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2025-01-15");
        assert_eq!(rows[0].stock_name, "苹果");
        assert_eq!(rows[0].rating, "买入");
        assert!(approx(rows[0].latest_target_price, 200.5));
        assert!(approx(rows[0].prev_target_price, 180.0));
        assert_eq!(rows[0].institution, "高盛");
        assert_eq!(rows[1].stock_name, "特斯拉");
        assert_eq!(rows[1].institution, "摩根士丹利");
    }

    #[test]
    fn parse_stock_us_pink_spot_em_ok() {
        let rows = parse_stock_us_pink_spot_em(&fixture("stock_us_pink_spot_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "ABCDE.PINK");
        assert_eq!(rows[0].name, "Example Corp");
        assert!(approx(rows[0].latest_price, 1.23));
        assert!(approx(rows[0].pct_change, 2.5));
        assert!(approx(rows[0].change, 0.03));
        assert!(approx(rows[0].open, 1.2));
        assert!(approx(rows[0].high, 1.3));
        assert!(approx(rows[0].low, 1.1));
        assert!(approx(rows[0].pre_close, 1.2));
        assert!(approx(rows[0].total_mv, 5000000.0));
        assert!(approx(rows[0].pe, 15.2));
        assert_eq!(rows[1].code, "XYZ.PINK");
    }
}
