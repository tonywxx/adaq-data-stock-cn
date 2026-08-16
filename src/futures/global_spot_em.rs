//! Eastmoney international futures spot list (`futures_global_spot_em`).
//!
//! Ports `futures_global_spot_em` ← `futures_hf_em.py:87`.
//!
//! Pure JSON from `futsseapi.eastmoney.com/list/...` (no JS, no HTML scrape).
//! akshare pages through `total/20 - 1` pages; we replicate the loop and the
//! final Chinese column order.
//!
//! ## DEFERRED
//! None in this file.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

/// One international-futures spot row (`futures_global_spot_em`).
///
/// Field names follow akshare's final Chinese column order/semantics:
/// 序号, 代码, 名称, 最新价, 涨跌额, 涨跌幅, 今开, 最高, 最低, 昨结, 成交量, 买盘, 卖盘, 持仓量.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GlobalSpotEmRow {
    pub index: u32,
    pub code: String,
    pub name: String,
    pub price: Option<f64>,
    pub change: Option<f64>,
    pub change_pct: Option<f64>,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub pre_settle: Option<f64>,
    pub volume: Option<f64>,
    pub buy_vol: Option<f64>,
    pub sell_vol: Option<f64>,
    pub open_interest: Option<f64>,
}

fn to_f64_opt(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() || t == "-" {
                None
            } else {
                t.parse::<f64>().ok()
            }
        }
        _ => None,
    }
}

fn get_str<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(|x| x.as_str()).unwrap_or_default()
}

/// Parse a single page response (`{ "total", "list": [...] }`) into rows.
/// `index` is assigned 1-based within this page; the async wrapper renumbers
/// across pages to match akshare.
pub(crate) fn parse_global_spot_em(resp: &Value) -> Result<Vec<GlobalSpotEmRow>> {
    let list = resp
        .get("list")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing list".into(),
        })?;
    let mut out = Vec::with_capacity(list.len());
    for (i, item) in list.iter().enumerate() {
        out.push(GlobalSpotEmRow {
            index: (i + 1) as u32,
            code: get_str(item, "dm").to_string(),
            name: get_str(item, "name").to_string(),
            price: to_f64_opt(item.get("p").unwrap_or(&Value::Null)),
            change: to_f64_opt(item.get("zde").unwrap_or(&Value::Null)),
            change_pct: to_f64_opt(item.get("zdf").unwrap_or(&Value::Null)),
            open: to_f64_opt(item.get("o").unwrap_or(&Value::Null)),
            high: to_f64_opt(item.get("h").unwrap_or(&Value::Null)),
            low: to_f64_opt(item.get("l").unwrap_or(&Value::Null)),
            pre_settle: to_f64_opt(item.get("zjsj").unwrap_or(&Value::Null)),
            volume: to_f64_opt(item.get("vol").unwrap_or(&Value::Null)),
            buy_vol: to_f64_opt(item.get("wp").unwrap_or(&Value::Null)),
            sell_vol: to_f64_opt(item.get("np").unwrap_or(&Value::Null)),
            open_interest: to_f64_opt(item.get("ccl").unwrap_or(&Value::Null)),
        });
    }
    Ok(out)
}

/// Eastmoney international-futures spot list (`futures_global_spot_em`).
pub async fn futures_global_spot_em(client: &Client) -> Result<Vec<GlobalSpotEmRow>> {
    let base: &[(&str, &str)] = &[
        ("orderBy", "dm"),
        ("sort", "desc"),
        ("pageSize", "20"),
        ("pageIndex", "0"),
        ("token", "58b2fa8f54638b60b87d69b31969089c"),
        (
            "field",
            "dm,sc,name,p,zsjd,zde,zdf,f152,o,h,l,zjsj,vol,wp,np,ccl",
        ),
        ("blockName", "callback"),
    ];
    let url = "https://futsseapi.eastmoney.com/list/COMEX,NYMEX,COBOT,SGX,NYBOT,LME,MDEX,TOCOM,IPE";

    let first = client
        .get_json(SOURCE_EASTMONEY, "futures_global_spot_em", url, base)
        .await?;
    let total = first.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
    let total_page = if total == 0 {
        0
    } else {
        (total as f64 / 20.0).ceil() as u64 - 1
    };

    let mut rows = parse_global_spot_em(&first)?;
    for page in 1..=total_page {
        // Owned storage so the dynamic page index outlives the request.
        let mut owned: Vec<(String, String)> = base
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        for slot in owned.iter_mut() {
            if slot.0 == "pageIndex" {
                slot.1 = page.to_string();
            }
        }
        let params: Vec<(&str, &str)> = owned
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let resp = client
            .get_json(SOURCE_EASTMONEY, "futures_global_spot_em", url, &params)
            .await?;
        rows.extend(parse_global_spot_em(&resp)?);
    }
    // Renumber 1..n to match akshare's global reindex.
    for (i, r) in rows.iter_mut().enumerate() {
        r.index = (i + 1) as u32;
    }
    Ok(rows)
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
    fn parse_global_spot_em_ok() {
        let rows = parse_global_spot_em(&fixture("futures_global_spot_em.json")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].index, 1);
        assert_eq!(rows[0].code, "ZW29N");
        assert_eq!(rows[0].name, "小麦2907");
        assert!(rows[0].price.is_none()); // "-" -> None
        assert!(approx(rows[1].price, 709.0));
        assert!(approx(rows[1].change, -14.0));
        assert!(approx(rows[1].change_pct, -1.94));
        assert!(approx(rows[1].pre_settle, 723.0));
        assert!(approx(rows[2].volume, 2.0));
        assert!(approx(rows[2].open_interest, 8.0));
    }
}
