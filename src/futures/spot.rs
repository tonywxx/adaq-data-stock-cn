//! Eastmoney real-time futures spot quotes (`futures_global_spot_em`).
//!
//! Ports akshare `futures_global_spot_em`: international/global futures quotes
//! via Eastmoney's `futsseapi` list endpoint (a `clist`-style paginated API,
//! no JS signing). akshare's Sina-based `futures_zh_spot` requires JS signing
//! (`py_mini_racer`) and is not portable here, so we expose the Eastmoney
//! global-spot endpoint under the `futures_zh_spot` name (source-resilient).

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};
use crate::core::json::*;

/// Static Eastmoney list token (from akshare `futures_global_spot_em`).
const TOKEN: &str = "58b2fa8f54638b60b87d69b31969089c";
const BASE: &str =
    "https://futsseapi.eastmoney.com/list/COMEX,NYMEX,COBOT,SGX,NYBOT,LME,MDEX,TOCOM,IPE";
const FIELDS: &str = "dm,sc,name,p,zsjd,zde,zdf,f152,o,h,l,zjsj,vol,wp,np,ccl";
const PAGE_SIZE: u32 = 20;

/// One real-time futures quote from Eastmoney (`futures_global_spot_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesSpotRow {
    pub code: String,
    pub name: String,
    pub price: Option<f64>,
    pub change: Option<f64>,
    pub pct_change: Option<f64>,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub pre_settle: Option<f64>,
    pub volume: Option<f64>,
    pub buy_vol: Option<f64>,
    pub sell_vol: Option<f64>,
    pub open_interest: Option<f64>,
    pub source: &'static str,
}

/// Real-time global futures spot quotes from Eastmoney (`futures_global_spot_em`).
///
/// Paginates the `futsseapi` list endpoint until `total` is covered.
pub async fn futures_zh_spot(client: &Client) -> Result<Vec<FuturesSpotRow>> {
    let ps = PAGE_SIZE.to_string();
    let mut out = Vec::new();
    let mut page: u32 = 0;
    loop {
        let page_s = page.to_string();
        let params = [
            ("orderBy", "dm"),
            ("sort", "desc"),
            ("pageSize", ps.as_str()),
            ("pageIndex", page_s.as_str()),
            ("token", TOKEN),
            ("field", FIELDS),
            ("blockName", "callback"),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, "futures_zh_spot", BASE, &params)
            .await?;
        let page_rows = parse_spot(&v)?;
        if page_rows.is_empty() {
            break;
        }
        out.extend(page_rows);
        let total = v.get("total").and_then(|t| t.as_u64()).unwrap_or(0);
        if (page as u64 + 1) * PAGE_SIZE as u64 >= total {
            break;
        }
        page += 1;
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    }
    Ok(out)
}

pub(crate) fn parse_spot(resp: &Value) -> Result<Vec<FuturesSpotRow>> {
    let list =
        resp.get("list")
            .and_then(|l| l.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing list".into(),
            })?;
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        let obj = match item.as_object() {
            Some(o) => o,
            None => continue,
        };
        if !obj.contains_key("dm") {
            continue;
        }
        out.push(parse_item(item));
    }
    Ok(out)
}

fn parse_item(item: &Value) -> FuturesSpotRow {
    FuturesSpotRow {
        code: opt_str_or(item, "dm", ""),
        name: opt_str_or(item, "name", ""),
        price: opt_f64(item, "p"),
        change: opt_f64(item, "zde"),
        pct_change: opt_f64(item, "zdf"),
        open: opt_f64(item, "o"),
        high: opt_f64(item, "h"),
        low: opt_f64(item, "l"),
        pre_settle: opt_f64(item, "zjsj"),
        volume: opt_f64(item, "vol"),
        buy_vol: opt_f64(item, "wp"),
        sell_vol: opt_f64(item, "np"),
        open_interest: opt_f64(item, "ccl"),
        source: SOURCE_EASTMONEY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_futures_zh_spot_fixture() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/futures_zh_spot.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_spot(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "HG00Y");
        assert_eq!(rows[0].name, "Comex铜");
        assert_eq!(rows[0].price, Some(4.12));
        assert_eq!(rows[0].pct_change, Some(1.23));
        assert_eq!(rows[0].open_interest, Some(220000.0));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].code, "GC00Y");
        assert_eq!(rows[1].price, Some(2015.3));
    }
}
