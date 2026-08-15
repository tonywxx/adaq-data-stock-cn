//! CPCA (乘联会, China Passenger Car Association) car-sales data.
//!
//! Ports `akshare/other/other_car_cpca.py`. All six public functions hit the
//! JSON endpoint `http://data.cpcadata.com/api/chartlist` (a plain
//! `requests.get` + `r.json()` in the source — no HTML scraping, JS, token, or
//! Excel download), so every function is FEASIBLE and implemented with
//! `client.get_json`.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `car_market_total_cpca` | `other_car_cpca.py:13` | charttype=1; `symbol`∈{狭义乘用车,广义乘用车}, `indicator`∈{产量,批发,零售,出口} |
//! | `car_market_man_rank_cpca` | `other_car_cpca.py:391` | charttype=2; 批发 via `chartlist`, 零售 via `chartlist_2`; `symbol`∈{狭义乘用车-单月,狭义乘用车-累计,广义乘用车-单月,广义乘用车-累计} |
//! | `car_market_cate_cpca` | `other_car_cpca.py:646` | charttype=3; `symbol`∈{轿车,MPV,SUV}, `indicator`∈{批发,零售}; 占比 uses `car_market_cate_share_cpca` |
//! | `car_market_country_cpca` | `other_car_cpca.py:665` | charttype=4; 国别细分 (fixed column set) |
//! | `car_market_segment_cpca` | `other_car_cpca.py:685` | charttype=5; 级别细分 `symbol`∈{轿车,MPV,SUV} |
//! | `car_market_fuel_cpca` | `other_car_cpca.py:722` | charttype=6; 整体市场 / 销量占比-PHEV-BEV (`car_market_fuel_phev_bev_cpca`) / 销量占比-ICE-NEV (`car_market_fuel_ice_nev_cpca`) |
//!
//! ## DEFERRED
//!
//! None. Every public function in the source retrieves a JSON document over HTTP
//! (`data.cpcadata.com/api/chartlist`); there are no HTML/JS/token/Excel barriers.
//! (The source does not use the Eastmoney `datacenter-web` endpoint, so the
//! `emg_data_array` helper is not applicable here.)

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_CPCCA: &str = "cpcadata";
const BASE_CHARTLIST: &str = "http://data.cpcadata.com/api/chartlist";
const BASE_CHARTLIST_2: &str = "http://data.cpcadata.com/api/chartlist_2";

/// Index of each indicator inside the 4-element `[产量,批发,零售,出口]` arrays.
fn total_indicator_idx(indicator: &str) -> usize {
    match indicator {
        "批发" => 1,
        "零售" => 2,
        "出口" => 3,
        _ => 0, // 产量
    }
}

/// Index of each indicator inside the 2-element `[批发,零售]` arrays.
fn man_rank_indicator_idx(indicator: &str) -> usize {
    match indicator {
        "零售" => 1,
        _ => 0, // 批发
    }
}

fn total_symbol_idx(symbol: &str) -> usize {
    if symbol == "广义乘用车" { 1 } else { 0 }
}

fn man_rank_symbol_idx(symbol: &str) -> usize {
    match symbol {
        "狭义乘用车-累计" => 0,
        "广义乘用车-单月" => 3,
        "广义乘用车-累计" => 2,
        _ => 1, // 狭义乘用车-单月
    }
}

fn cate_symbol_idx(symbol: &str) -> usize {
    match symbol {
        "MPV" => 0,
        "SUV" => 1,
        "占比" => 3,
        _ => 2, // 轿车
    }
}

fn segment_symbol_idx(symbol: &str) -> usize {
    match symbol {
        "MPV" => 0,
        "SUV" => 1,
        _ => 2, // 轿车
    }
}

fn fuel_symbol_idx(symbol: &str) -> usize {
    match symbol {
        "销量占比-PHEV-BEV" => 1,
        "销量占比-ICE-NEV" => 2,
        _ => 0, // 整体市场
    }
}

/// Read a string field.
pub fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(str::to_string)
}

/// Read a numeric field.
pub fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| v.as_f64())
}

/// Read the `idx`-th element of an array-valued field, or `None`.
fn arr_num(item: &Value, key: &str, idx: usize) -> Option<f64> {
    item.get(key)
        .and_then(|v| v.as_array())
        .and_then(|a| a.get(idx))
        .and_then(|v| v.as_f64())
}

/// Return the label (month / 月份 / 厂商) for a dataList item, if present.
fn label(item: &Value) -> Option<String> {
    for k in ["month", "月份", "厂商"] {
        if let Some(s) = item.get(k).and_then(|v| v.as_str()) {
            return Some(s.to_string());
        }
    }
    None
}

/// Identify the current/previous year-array keys by the (4-digit) year embedded
/// in the key. `同比` and label columns carry no year digits and are ignored.
/// Returns `(current_key, previous_key)` sorted by descending year.
fn year_keys(item: &Value) -> Option<(String, String)> {
    let mut years: Vec<(i32, String)> = Vec::new();
    if let Some(obj) = item.as_object() {
        for (k, v) in obj {
            if v.as_array().is_some_and(|a| !a.is_empty()) {
                // Keys are like "2026年 1-7月": take only the leading 4-digit year,
                // not the month digits that follow.
                let digits: String = k.chars().filter(|c| c.is_ascii_digit()).collect();
                if digits.len() >= 4
                    && let Ok(y) = digits[..4].parse::<i32>()
                    && (2000..=2999).contains(&y)
                {
                    years.push((y, k.clone()));
                }
            }
        }
    }
    if years.len() < 2 {
        return None;
    }
    years.sort_by_key(|b| std::cmp::Reverse(b.0));
    Some((years[0].1.clone(), years[1].1.clone()))
}

/// Fetch a chartlist endpoint and return the full top-level JSON value (the
/// upstream returns a JSON array; `chart_datalist` indexes into it).
async fn fetch_chartlist(client: &Client, url: &str, charttype: &str) -> Result<Value> {
    client
        .get_json(
            SOURCE_CPCCA,
            "car_cpca_chartlist",
            url,
            &[("charttype", charttype)],
        )
        .await
}

/// Extract the `dataList` array at category `idx` within a chartlist response.
fn chart_datalist(resp: &Value, idx: usize) -> Result<&Vec<Value>> {
    resp.as_array()
        .and_then(|a| a.get(idx))
        .and_then(|x| x.get("dataList"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CPCCA,
            message: format!("missing dataList at index {idx}"),
        })
}

// ---------------------------------------------------------------------------
// car_market_total_cpca  (akshare other_car_cpca.py:13)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct CarMarketTotalRow {
    pub month: String,
    pub current_year: Option<f64>,
    pub previous_year: Option<f64>,
}

/// Parse `car_market_total_cpca` rows from a charttype=1 response.
///
/// NOTE: the akshare source takes `previous_year` from array index 0 for every
/// indicator (a quirk/bug); we align by `indicator_idx` so the previous-year
/// column matches the chosen indicator.
pub(crate) fn parse_car_market_total(
    resp: &Value,
    symbol_idx: usize,
    indicator_idx: usize,
) -> Result<Vec<CarMarketTotalRow>> {
    let dl = chart_datalist(resp, symbol_idx)?;
    let mut out = Vec::with_capacity(dl.len());
    for item in dl {
        let Some(month) = label(item) else { continue };
        let Some((cur_key, prev_key)) = year_keys(item) else {
            continue;
        };
        let current_year = arr_num(item, &cur_key, indicator_idx);
        let previous_year = arr_num(item, &prev_key, indicator_idx);
        out.push(CarMarketTotalRow {
            month,
            current_year,
            previous_year,
        });
    }
    Ok(out)
}

/// 乘联会-统计数据-总体市场 (charttype=1), defaults `symbol="狭义乘用车"`, `indicator="产量"`.
pub async fn car_market_total_cpca(client: &Client) -> Result<Vec<CarMarketTotalRow>> {
    car_market_total_cpca_opts(client, "狭义乘用车", "产量").await
}

/// 乘联会-统计数据-总体市场 (charttype=1) with explicit `symbol`/`indicator`.
pub async fn car_market_total_cpca_opts(
    client: &Client,
    symbol: &str,
    indicator: &str,
) -> Result<Vec<CarMarketTotalRow>> {
    let v = fetch_chartlist(client, BASE_CHARTLIST, "1").await?;
    parse_car_market_total(&v, total_symbol_idx(symbol), total_indicator_idx(indicator))
}

// ---------------------------------------------------------------------------
// car_market_man_rank_cpca  (akshare other_car_cpca.py:391)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct CarMarketManRankRow {
    pub manufacturer: String,
    pub current_year: Option<f64>,
    pub previous_year: Option<f64>,
}

/// Parse `car_market_man_rank_cpca` rows from a charttype=2 response.
pub(crate) fn parse_car_market_man_rank(
    resp: &Value,
    symbol_idx: usize,
    indicator_idx: usize,
) -> Result<Vec<CarMarketManRankRow>> {
    let dl = chart_datalist(resp, symbol_idx)?;
    let mut out = Vec::with_capacity(dl.len());
    for item in dl {
        let Some(mfr) = fstr(item, "厂商") else {
            continue;
        };
        let Some((cur_key, prev_key)) = year_keys(item) else {
            continue;
        };
        let current_year = arr_num(item, &cur_key, indicator_idx);
        let previous_year = arr_num(item, &prev_key, indicator_idx);
        out.push(CarMarketManRankRow {
            manufacturer: mfr,
            current_year,
            previous_year,
        });
    }
    Ok(out)
}

/// 乘联会-统计数据-厂商排名 (charttype=2), defaults `symbol="狭义乘用车-单月"`, `indicator="批发"`.
pub async fn car_market_man_rank_cpca(client: &Client) -> Result<Vec<CarMarketManRankRow>> {
    car_market_man_rank_cpca_opts(client, "狭义乘用车-单月", "批发").await
}

/// 乘联会-统计数据-厂商排名 (charttype=2) with explicit `symbol`/`indicator`.
/// `indicator="零售"` hits `chartlist_2`; `indicator="批发"` hits `chartlist`.
pub async fn car_market_man_rank_cpca_opts(
    client: &Client,
    symbol: &str,
    indicator: &str,
) -> Result<Vec<CarMarketManRankRow>> {
    let url = if indicator == "零售" {
        BASE_CHARTLIST_2
    } else {
        BASE_CHARTLIST
    };
    let v = fetch_chartlist(client, url, "2").await?;
    parse_car_market_man_rank(
        &v,
        man_rank_symbol_idx(symbol),
        man_rank_indicator_idx(indicator),
    )
}

// ---------------------------------------------------------------------------
// car_market_cate_cpca  (akshare other_car_cpca.py:646)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct CarMarketCateRow {
    pub month: String,
    pub current_year: Option<f64>,
    pub previous_year: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CarMarketCateShareRow {
    pub month: String,
    pub mpv: Option<f64>,
    pub suv: Option<f64>,
    pub jiaoche: Option<f64>,
}

/// Parse `car_market_cate_cpca` rows (轿车/MPV/SUV) from a charttype=3 response.
pub(crate) fn parse_car_market_cate(
    resp: &Value,
    symbol_idx: usize,
    indicator_idx: usize,
) -> Result<Vec<CarMarketCateRow>> {
    let dl = chart_datalist(resp, symbol_idx)?;
    let mut out = Vec::with_capacity(dl.len());
    for item in dl {
        let Some(month) = label(item) else { continue };
        let Some((cur_key, prev_key)) = year_keys(item) else {
            continue;
        };
        let current_year = arr_num(item, &cur_key, indicator_idx);
        let previous_year = arr_num(item, &prev_key, indicator_idx);
        out.push(CarMarketCateRow {
            month,
            current_year,
            previous_year,
        });
    }
    Ok(out)
}

/// Parse the `占比` (category share) variant from a charttype=3 response (index 3).
pub(crate) fn parse_car_market_cate_share(resp: &Value) -> Result<Vec<CarMarketCateShareRow>> {
    let dl = chart_datalist(resp, 3)?;
    let mut out = Vec::with_capacity(dl.len());
    for item in dl {
        let Some(month) = label(item) else { continue };
        let mpv = arr_num(item, "MPV", 2);
        let suv = arr_num(item, "SUV", 2);
        let jiaoche = arr_num(item, "轿车", 2);
        out.push(CarMarketCateShareRow {
            month,
            mpv,
            suv,
            jiaoche,
        });
    }
    Ok(out)
}

/// 乘联会-统计数据-车型大类 (charttype=3), defaults `symbol="轿车"`, `indicator="批发"`.
pub async fn car_market_cate_cpca(client: &Client) -> Result<Vec<CarMarketCateRow>> {
    car_market_cate_cpca_opts(client, "轿车", "批发").await
}

/// 乘联会-统计数据-车型大类 (charttype=3) with explicit `symbol`/`indicator`.
/// `symbol="占比"` is not a `CarMarketCateRow`; use `car_market_cate_share_cpca`.
pub async fn car_market_cate_cpca_opts(
    client: &Client,
    symbol: &str,
    indicator: &str,
) -> Result<Vec<CarMarketCateRow>> {
    if symbol == "占比" {
        return Err(Error::InvalidParam(
            "symbol \"占比\" is not a CarMarketCateRow; use car_market_cate_share_cpca".into(),
        ));
    }
    let v = fetch_chartlist(client, BASE_CHARTLIST, "3").await?;
    parse_car_market_cate(&v, cate_symbol_idx(symbol), total_indicator_idx(indicator))
}

/// 乘联会-统计数据-车型大类占比 (charttype=3, index 3).
pub async fn car_market_cate_share_cpca(client: &Client) -> Result<Vec<CarMarketCateShareRow>> {
    let v = fetch_chartlist(client, BASE_CHARTLIST, "3").await?;
    parse_car_market_cate_share(&v)
}

// ---------------------------------------------------------------------------
// car_market_country_cpca  (akshare other_car_cpca.py:665)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct CarMarketCountryRow {
    pub month: String,
    pub zi_zhu: Option<f64>,
    pub de_xi: Option<f64>,
    pub ri_xi: Option<f64>,
    pub mei_xi: Option<f64>,
    pub han_xi: Option<f64>,
    pub fa_xi: Option<f64>,
    pub qi_ta_ou_xi: Option<f64>,
}

/// Parse `car_market_country_cpca` rows (charttype=4, index 0). Each country
/// array yields element index 2 (per akshare, which takes `item_list[2]`).
pub(crate) fn parse_car_market_country(resp: &Value) -> Result<Vec<CarMarketCountryRow>> {
    let dl = chart_datalist(resp, 0)?;
    let mut out = Vec::with_capacity(dl.len());
    for item in dl {
        let Some(month) = label(item) else { continue };
        let zi_zhu = arr_num(item, "自主", 2);
        let de_xi = arr_num(item, "德系", 2);
        let ri_xi = arr_num(item, "日系", 2);
        let mei_xi = arr_num(item, "美系", 2);
        let han_xi = arr_num(item, "韩系", 2);
        let fa_xi = arr_num(item, "法系", 2);
        let qi_ta_ou_xi = arr_num(item, "其他欧系", 2);
        out.push(CarMarketCountryRow {
            month,
            zi_zhu,
            de_xi,
            ri_xi,
            mei_xi,
            han_xi,
            fa_xi,
            qi_ta_ou_xi,
        });
    }
    Ok(out)
}

/// 乘联会-统计数据-国别细分市场 (charttype=4).
pub async fn car_market_country_cpca(client: &Client) -> Result<Vec<CarMarketCountryRow>> {
    let v = fetch_chartlist(client, BASE_CHARTLIST, "4").await?;
    parse_car_market_country(&v)
}

// ---------------------------------------------------------------------------
// car_market_segment_cpca  (akshare other_car_cpca.py:685)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct CarMarketSegmentRow {
    pub month: String,
    pub a00: Option<f64>,
    pub a0: Option<f64>,
    pub a: Option<f64>,
    pub b: Option<f64>,
    pub c: Option<f64>,
}

/// Parse `car_market_segment_cpca` rows (charttype=5) for the given category.
/// Each segment array yields element index 2 (per akshare, `item_list[2]`).
pub(crate) fn parse_car_market_segment(
    resp: &Value,
    symbol_idx: usize,
) -> Result<Vec<CarMarketSegmentRow>> {
    let dl = chart_datalist(resp, symbol_idx)?;
    let mut out = Vec::with_capacity(dl.len());
    for item in dl {
        let Some(month) = label(item) else { continue };
        let a00 = arr_num(item, "A00", 2);
        let a0 = arr_num(item, "A0", 2);
        let a = arr_num(item, "A", 2);
        let b = arr_num(item, "B", 2);
        let c = arr_num(item, "C", 2);
        out.push(CarMarketSegmentRow {
            month,
            a00,
            a0,
            a,
            b,
            c,
        });
    }
    Ok(out)
}

/// 乘联会-统计数据-级别细分市场 (charttype=5), defaults `symbol="轿车"`.
pub async fn car_market_segment_cpca(client: &Client) -> Result<Vec<CarMarketSegmentRow>> {
    car_market_segment_cpca_opts(client, "轿车").await
}

/// 乘联会-统计数据-级别细分市场 (charttype=5) with explicit `symbol`.
pub async fn car_market_segment_cpca_opts(
    client: &Client,
    symbol: &str,
) -> Result<Vec<CarMarketSegmentRow>> {
    let v = fetch_chartlist(client, BASE_CHARTLIST, "5").await?;
    parse_car_market_segment(&v, segment_symbol_idx(symbol))
}

// ---------------------------------------------------------------------------
// car_market_fuel_cpca  (akshare other_car_cpca.py:722)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct CarMarketFuelTotalRow {
    pub month: String,
    pub current_year: Option<f64>,
    pub previous_year: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CarMarketFuelPhevBevRow {
    pub month: String,
    pub phev: Option<f64>,
    pub bev: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CarMarketFuelIceNevRow {
    pub month: String,
    pub nev: Option<f64>,
    pub ice: Option<f64>,
}

/// Parse the 整体市场 variant (charttype=6, index 0). akshare takes array index 2
/// (零售) for both current and previous year.
pub(crate) fn parse_car_market_fuel_total(resp: &Value) -> Result<Vec<CarMarketFuelTotalRow>> {
    let dl = chart_datalist(resp, 0)?;
    let mut out = Vec::with_capacity(dl.len());
    for item in dl {
        let Some(month) = label(item) else { continue };
        let Some((cur_key, prev_key)) = year_keys(item) else {
            continue;
        };
        let current_year = arr_num(item, &cur_key, 2);
        let previous_year = arr_num(item, &prev_key, 2);
        out.push(CarMarketFuelTotalRow {
            month,
            current_year,
            previous_year,
        });
    }
    Ok(out)
}

/// Parse the 销量占比-PHEV-BEV variant (charttype=6, index 1). akshare reorders
/// columns to `[月份, PHEV, BEV]`, so we surface `phev` then `bev`.
pub(crate) fn parse_car_market_fuel_phev_bev(resp: &Value) -> Result<Vec<CarMarketFuelPhevBevRow>> {
    let dl = chart_datalist(resp, 1)?;
    let mut out = Vec::with_capacity(dl.len());
    for item in dl {
        let Some(month) = label(item) else { continue };
        let bev = arr_num(item, "BEV", 2);
        let phev = arr_num(item, "PHEV", 2);
        out.push(CarMarketFuelPhevBevRow { month, phev, bev });
    }
    Ok(out)
}

/// Parse the 销量占比-ICE-NEV variant (charttype=6, index 2). akshare reorders
/// columns to `[月份, NEV, ICE]`, so we surface `nev` then `ice`.
pub(crate) fn parse_car_market_fuel_ice_nev(resp: &Value) -> Result<Vec<CarMarketFuelIceNevRow>> {
    let dl = chart_datalist(resp, 2)?;
    let mut out = Vec::with_capacity(dl.len());
    for item in dl {
        let Some(month) = label(item) else { continue };
        let ice = arr_num(item, "ICE", 2);
        let nev = arr_num(item, "NEV", 2);
        out.push(CarMarketFuelIceNevRow { month, nev, ice });
    }
    Ok(out)
}

/// 乘联会-统计数据-新能源细分市场 (charttype=6), defaults `symbol="整体市场"`.
pub async fn car_market_fuel_cpca(client: &Client) -> Result<Vec<CarMarketFuelTotalRow>> {
    car_market_fuel_cpca_opts(client, "整体市场").await
}

/// 乘联会-统计数据-新能源细分市场 (charttype=6) with explicit `symbol`.
/// `销量占比-PHEV-BEV`/`销量占比-ICE-NEV` use the dedicated share functions.
pub async fn car_market_fuel_cpca_opts(
    client: &Client,
    symbol: &str,
) -> Result<Vec<CarMarketFuelTotalRow>> {
    if fuel_symbol_idx(symbol) != 0 {
        return Err(Error::InvalidParam(
            "symbol must be \"整体市场\" for car_market_fuel_cpca; use car_market_fuel_phev_bev_cpca / car_market_fuel_ice_nev_cpca".into(),
        ));
    }
    let v = fetch_chartlist(client, BASE_CHARTLIST, "6").await?;
    parse_car_market_fuel_total(&v)
}

/// 乘联会-统计数据-新能源细分市场 销量占比-PHEV-BEV (charttype=6, index 1).
pub async fn car_market_fuel_phev_bev_cpca(
    client: &Client,
) -> Result<Vec<CarMarketFuelPhevBevRow>> {
    let v = fetch_chartlist(client, BASE_CHARTLIST, "6").await?;
    parse_car_market_fuel_phev_bev(&v)
}

/// 乘联会-统计数据-新能源细分市场 销量占比-ICE-NEV (charttype=6, index 2).
pub async fn car_market_fuel_ice_nev_cpca(client: &Client) -> Result<Vec<CarMarketFuelIceNevRow>> {
    let v = fetch_chartlist(client, BASE_CHARTLIST, "6").await?;
    parse_car_market_fuel_ice_nev(&v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Value {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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
    fn parse_total_narrow_output() {
        let v = fixture("car_market_total_cpca.json");
        let rows = parse_car_market_total(&v, 0, total_indicator_idx("产量")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].month, "1月");
        assert!(approx(rows[0].current_year, 200.364));
        assert!(approx(rows[0].previous_year, 209.2576));
    }

    #[test]
    fn parse_total_broad_wholesale() {
        let v = fixture("car_market_total_cpca.json");
        let rows = parse_car_market_total(&v, 1, total_indicator_idx("批发")).unwrap();
        assert_eq!(rows.len(), 3);
        assert!(approx(rows[0].current_year, 198.9483));
        assert!(approx(rows[0].previous_year, 211.9637));
    }

    #[test]
    fn parse_man_rank_pifa() {
        let v = fixture("car_market_man_rank_cpca_pifa.json");
        let rows = parse_car_market_man_rank(&v, 0, man_rank_indicator_idx("批发")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].manufacturer, "比亚迪汽车");
        assert!(approx(rows[0].current_year, 218.7987));
        assert!(approx(rows[0].previous_year, 245.4301));
    }

    #[test]
    fn parse_man_rank_lingshou() {
        let v = fixture("car_market_man_rank_cpca_lingshou.json");
        let rows = parse_car_market_man_rank(&v, 1, man_rank_indicator_idx("零售")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].manufacturer, "比亚迪汽车");
        assert!(approx(rows[0].current_year, 22.3461));
        assert!(approx(rows[0].previous_year, 27.4644));
    }

    #[test]
    fn parse_cate_sedan_wholesale() {
        let v = fixture("car_market_cate_cpca.json");
        let rows = parse_car_market_cate(&v, 2, total_indicator_idx("批发")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].month, "1月");
        assert!(approx(rows[0].current_year, 74.3614));
        assert!(approx(rows[0].previous_year, 86.8113));
    }

    #[test]
    fn parse_cate_share() {
        let v = fixture("car_market_cate_cpca.json");
        let rows = parse_car_market_cate_share(&v).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].month, "2026-1月");
        assert!(approx(rows[0].mpv, 3.7));
        assert!(approx(rows[0].suv, 58.6));
        assert!(approx(rows[0].jiaoche, 37.7));
    }

    #[test]
    fn parse_country() {
        let v = fixture("car_market_country_cpca.json");
        let rows = parse_car_market_country(&v).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].month, "2025-8月");
        assert!(approx(rows[0].zi_zhu, 69.6));
        assert!(approx(rows[0].de_xi, 12.6));
    }

    #[test]
    fn parse_segment_sedan() {
        let v = fixture("car_market_segment_cpca.json");
        let rows = parse_car_market_segment(&v, 2).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].month, "2025-8月");
        assert!(approx(rows[0].a00, 11.9));
        assert!(approx(rows[0].a0, 13.2));
        assert!(approx(rows[0].a, 35.6));
        assert!(approx(rows[0].b, 31.4));
        assert!(approx(rows[0].c, 8.0));
    }

    #[test]
    fn parse_fuel_total() {
        let v = fixture("car_market_fuel_cpca.json");
        let rows = parse_car_market_fuel_total(&v).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].month, "1月");
        assert!(approx(rows[0].current_year, 59.1814));
        assert!(approx(rows[0].previous_year, 74.4462));
    }

    #[test]
    fn parse_fuel_phev_bev() {
        let v = fixture("car_market_fuel_cpca.json");
        let rows = parse_car_market_fuel_phev_bev(&v).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].month, "2025-8月");
        assert!(approx(rows[0].bev, 68.5));
        assert!(approx(rows[0].phev, 31.5));
    }

    #[test]
    fn parse_fuel_ice_nev() {
        let v = fixture("car_market_fuel_cpca.json");
        let rows = parse_car_market_fuel_ice_nev(&v).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].month, "2025-8月");
        assert!(approx(rows[0].ice, 47.8));
        assert!(approx(rows[0].nev, 52.2));
    }
}
