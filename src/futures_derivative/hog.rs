//! 玄田生猪数据 (zhujia.zhuwang.com.cn / xt.yangzhu.vip), ported from
//! `akshare/futures_derivative/futures_hog.py`.
//!
//! All three functions hit `https://xt.yangzhu.vip/data/getzhujiahitsdata` or
//! `https://xt.yangzhu.vip/data/getmapdata` (pure JSON POST; `data` is a
//! row-oriented array of arrays). No HTML / JS / Excel.
//!
//! | Rust fn               | akshare source        | notes                                  |
//! | --------------------- | --------------------- | -------------------------------------- |
//! | `futures_hog_core`    | `futures_hog.py:13`   | 核心数据: 外三元/内三元/土杂猪           |
//! | `futures_hog_cost`    | `futures_hog.py:57`   | 成本维度: 玉米/豆粕/二元母猪价格/仔猪价格 |
//! | `futures_hog_supply`  | `futures_hog.py:116`  | 供应维度: 8 sub-symbols                 |
//!
//! ## DEFERRED
//! None. Every function in the source retrieves a JSON document over HTTP.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_HOG: &str = "zhuwang";
const HITS_URL: &str = "https://xt.yangzhu.vip/data/getzhujiahitsdata";
const MAP_URL: &str = "https://xt.yangzhu.vip/data/getmapdata";

// ---------------------------------------------------------------------------
// core & cost — both reduce to (date, value) rows
// ---------------------------------------------------------------------------

/// One hog core/cost point (`futures_hog_core` / `futures_hog_cost`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct HogPointRow {
    /// Date `YYYY-MM-DD` (or period label for some cost series).
    pub date: Option<String>,
    /// The metric value.
    pub value: Option<f64>,
}

/// 玄田数据-核心数据 (`futures_hog_core`). `symbol` ∈ {外三元, 内三元, 土杂猪}.
pub async fn futures_hog_core(client: &Client, symbol: &str) -> Result<Vec<HogPointRow>> {
    let ptype = match symbol {
        "外三元" => "1",
        "内三元" => "2",
        "土杂猪" => "3",
        _ => return Err(Error::InvalidParam("symbol must be 外三元/内三元/土杂猪".into())),
    };
    let v = client
        .post_form_json(
            SOURCE_HOG,
            "futures_hog_core",
            HITS_URL,
            &[("ptype", ptype), ("areano", "-1"), ("datetype", "0")],
            None,
        )
        .await?;
    parse_hog_hits(&v, symbol)
}

/// 玄田数据-成本维度 (`futures_hog_cost`). `symbol` ∈ {玉米, 豆粕, 二元母猪价格, 仔猪价格}.
pub async fn futures_hog_cost(client: &Client, symbol: &str) -> Result<Vec<HogPointRow>> {
    let (url, ptype) = match symbol {
        "玉米" => (HITS_URL, "4"),
        "豆粕" => (HITS_URL, "5"),
        "二元母猪价格" => (MAP_URL, "1"),
        "仔猪价格" => (MAP_URL, "2"),
        _ => return Err(Error::InvalidParam(
            "symbol must be 玉米/豆粕/二元母猪价格/仔猪价格".into(),
        )),
    };
    let v = client
        .post_form_json(
            SOURCE_HOG,
            "futures_hog_cost",
            url,
            &[("ptype", ptype), ("areano", "-1")],
            None,
        )
        .await?;
    parse_hog_points(&v, symbol)
}

/// Parse `getzhujiahitsdata` rows: upstream shape `[value, date]` → (date, value).
pub(crate) fn parse_hog_hits(resp: &Value, _symbol: &str) -> Result<Vec<HogPointRow>> {
    parse_rows(resp, &["value", "date"])
}

/// Parse cost rows: `getzhujiahitsdata` gives `[value, date]`;
/// `getmapdata` (二元母猪价格/仔猪价格) gives `[date, value]`. Both collapse to (date, value).
pub(crate) fn parse_hog_points(resp: &Value, symbol: &str) -> Result<Vec<HogPointRow>> {
    let cols = if matches!(symbol, "二元母猪价格" | "仔猪价格") {
        &["date", "value"][..]
    } else {
        &["value", "date"][..]
    };
    parse_rows(resp, cols)
}

// ---------------------------------------------------------------------------
// supply — mixed (date, value) and period-based shapes
// ---------------------------------------------------------------------------

/// One hog supply point (`futures_hog_supply`).
///
/// Date-based series (e.g. 猪肉批发价) fill `date` + `value`. Period-based
/// series (e.g. 生猪产能) fill `period` and the metric map `extra`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HogSupplyRow {
    /// Date `YYYY-MM-DD` for date-based series.
    pub date: Option<String>,
    /// Period label (e.g. `2024-01`) for period-based series.
    pub period: Option<String>,
    /// The single metric for date-based series.
    pub value: Option<f64>,
    /// Metric map for period-based series (key = akshare Chinese column name).
    pub extra: std::collections::HashMap<String, f64>,
}

/// 玄田数据-供应维度 (`futures_hog_supply`). `symbol` ∈ {猪肉批发价, 储备冻猪肉,
/// 饲料原料数据, 白条肉, 生猪产能, 育肥猪, 肉类价格指数, 猪粮比价}.
pub async fn futures_hog_supply(client: &Client, symbol: &str) -> Result<Vec<HogSupplyRow>> {
    let ptype = match symbol {
        "猪肉批发价" => "3",
        "储备冻猪肉" => "4",
        "饲料原料数据" => "5",
        "白条肉" => "6",
        "生猪产能" => "7",
        "育肥猪" => "9",
        "肉类价格指数" => "10",
        "猪粮比价" => "11",
        _ => return Err(Error::InvalidParam(
            "symbol must be one of 猪肉批发价/储备冻猪肉/饲料原料数据/白条肉/生猪产能/育肥猪/肉类价格指数/猪粮比价".into(),
        )),
    };
    let v = client
        .post_form_json(
            SOURCE_HOG,
            "futures_hog_supply",
            MAP_URL,
            &[("ptype", ptype), ("areano", "-1")],
            None,
        )
        .await?;
    parse_hog_supply(&v, symbol)
}

/// Parse `futures_hog_supply`. Date-based series drop the `item` column (猪肉批发价 /
/// 肉类价格指数) and keep (date, value); period-based series key metrics by column.
pub(crate) fn parse_hog_supply(resp: &Value, symbol: &str) -> Result<Vec<HogSupplyRow>> {
    let arr = data_array(resp)?;
    match symbol {
        "猪肉批发价" | "肉类价格指数" => Ok(arr
            .iter()
            .map(|r| HogSupplyRow {
                date: cell_str(idx(r, 0)),
                period: None,
                value: cell_num(idx(r, 2)),
                extra: std::collections::HashMap::new(),
            })
            .collect()),
        "储备冻猪肉" | "猪粮比价" => Ok(arr
            .iter()
            .map(|r| HogSupplyRow {
                date: cell_str(idx(r, 0)),
                period: None,
                value: cell_num(idx(r, 1)),
                extra: std::collections::HashMap::new(),
            })
            .collect()),
        "育肥猪" => Ok(arr
            .iter()
            .map(|r| HogSupplyRow {
                date: cell_str(idx(r, 0)),
                period: None,
                value: cell_num(idx(r, 1)),
                extra: std::collections::HashMap::new(),
            })
            .collect()),
        "饲料原料数据" => Ok(arr
            .iter()
            .map(|r| {
                let mut extra = std::collections::HashMap::new();
                extra.insert("大豆进口金额".into(), cell_num(idx(r, 1)).unwrap_or(f64::NAN));
                extra.insert("大豆播种面积".into(), cell_num(idx(r, 2)).unwrap_or(f64::NAN));
                extra.insert("玉米进口金额".into(), cell_num(idx(r, 3)).unwrap_or(f64::NAN));
                extra.insert("玉米播种面积".into(), cell_num(idx(r, 4)).unwrap_or(f64::NAN));
                HogSupplyRow { period: cell_str(idx(r, 0)), date: None, value: None, extra }
            })
            .collect()),
        "白条肉" => Ok(arr
            .iter()
            .map(|r| {
                let mut extra = std::collections::HashMap::new();
                extra.insert("白条肉平均出厂价格".into(), cell_num(idx(r, 1)).unwrap_or(f64::NAN));
                extra.insert("环比".into(), cell_num(idx(r, 2)).unwrap_or(f64::NAN));
                extra.insert("同比".into(), cell_num(idx(r, 3)).unwrap_or(f64::NAN));
                HogSupplyRow { period: cell_str(idx(r, 0)), date: None, value: None, extra }
            })
            .collect()),
        "生猪产能" => Ok(arr
            .iter()
            .map(|r| {
                let mut extra = std::collections::HashMap::new();
                extra.insert("能繁母猪存栏".into(), cell_num(idx(r, 1)).unwrap_or(f64::NAN));
                extra.insert("猪肉产量".into(), cell_num(idx(r, 2)).unwrap_or(f64::NAN));
                extra.insert("生猪存栏".into(), cell_num(idx(r, 3)).unwrap_or(f64::NAN));
                extra.insert("生猪出栏".into(), cell_num(idx(r, 4)).unwrap_or(f64::NAN));
                HogSupplyRow { period: cell_str(idx(r, 0)), date: None, value: None, extra }
            })
            .collect()),
        _ => Err(Error::InvalidParam(format!("unknown supply symbol: {symbol}"))),
    }
}

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

/// Extract the `data` array from a hog JSON response.
fn data_array(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_HOG,
            message: "missing data array".into(),
        })
}

/// Generic (date, value) parse for `[a, b]` shaped rows given column order.
fn parse_rows(resp: &Value, cols: &[&str]) -> Result<Vec<HogPointRow>> {
    let arr = data_array(resp)?;
    let date_idx = cols.iter().position(|c| *c == "date").unwrap();
    let val_idx = cols.iter().position(|c| *c == "value").unwrap();
    let mut out = Vec::with_capacity(arr.len());
    for r in arr {
        out.push(HogPointRow {
            date: cell_str(idx(r, date_idx)),
            value: cell_num(idx(r, val_idx)),
        });
    }
    Ok(out)
}

/// Safe array element access.
fn idx(row: &Value, i: usize) -> &Value {
    row.get(i).unwrap_or(&Value::Null)
}

/// Extract a string cell.
fn cell_str(v: &Value) -> Option<String> {
    v.as_str().map(str::to_string)
}

/// Extract a numeric cell, tolerating numeric strings.
fn cell_num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
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
    fn parse_hog_core_ok() {
        let rows = parse_hog_hits(&fixture("futures_hog_core.json"), "外三元").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, Some("2024-01-01".into()));
        assert!(approx(rows[0].value, 14.5));
        assert_eq!(rows[1].date, Some("2024-01-02".into()));
        assert!(approx(rows[1].value, 14.8));
    }

    #[test]
    fn parse_hog_cost_ok() {
        // 玉米 uses getzhujiahitsdata [value, date]; 仔猪价格 uses getmapdata [date, value].
        let hits = parse_hog_points(&fixture("futures_hog_cost.json"), "玉米").unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].date, Some("2024-01-01".into()));
        assert!(approx(hits[0].value, 2450.0));

        let map = parse_hog_points(&fixture("futures_hog_cost_map.json"), "仔猪价格").unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map[0].date, Some("2024-01-01".into()));
        assert!(approx(map[0].value, 320.0));
    }

    #[test]
    fn parse_hog_supply_date_based_ok() {
        let rows = parse_hog_supply(&fixture("futures_hog_supply.json"), "猪肉批发价").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, Some("2024-01-01".into()));
        assert!(approx(rows[0].value, 20.5));
        assert!(rows[0].extra.is_empty());
        assert_eq!(rows[1].date, Some("2024-01-08".into()));
    }

    #[test]
    fn parse_hog_supply_period_based_ok() {
        let rows = parse_hog_supply(&fixture("futures_hog_supply_period.json"), "生猪产能").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].period, Some("2024-01".into()));
        assert!(approx(rows[0].extra.get("能繁母猪存栏").copied(), 4150.0));
        assert!(approx(rows[0].extra.get("猪肉产量").copied(), 5500.0));
        assert!(approx(rows[0].extra.get("生猪存栏").copied(), 43000.0));
        assert!(approx(rows[0].extra.get("生猪出栏").copied(), 65000.0));
        assert!(rows[0].date.is_none());
        assert_eq!(rows[1].period, Some("2024-02".into()));
    }
}
