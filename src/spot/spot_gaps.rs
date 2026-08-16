//! Spot "gaps" port: 99qh spot-vs-futures trend (`spot_price_qh`).
//!
//! akshare `spot/spot_price_qh.py:79` queries fx168 `centerapi.fx168api.com`
//! `app/qh/api/spot/trend`. That endpoint requires a dynamic `_pcc` token that
//! fx168 returns **only in the response header** of `app/common/v.js`, which
//! the `Client` API does not expose. The token is therefore passed in by the
//! caller (the parser is fully tested against a captured fixture).

use std::collections::HashMap;

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_FX168: &str = "fx168";

const SPOT_QH_PAGE_URL: &str = "https://www.99qh.com/data/spotTrend";
const SPOT_QH_API: &str = "https://centerapi.fx168api.com/app/qh/api/spot/trend";

/// One 99qh spot-vs-futures observation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SpotPriceQhRow {
    /// Trade date (`YYYY-MM-DD`, akshare `日期`).
    pub date: String,
    /// Futures settlement close (akshare `期货收盘价`, field `fp`).
    pub futures_price: Option<f64>,
    /// Spot price (akshare `现货价格`, field `sp`).
    pub spot_price: Option<f64>,
    pub source: &'static str,
}

/// 99qh spot-vs-futures trend (`spot_price_qh`, akshare `spot_price_qh.py:79`).
///
/// `symbol` is the Chinese variety name (e.g. `螺纹钢`). `token` is the `_pcc`
/// value from the `centerapi.fx168api.com/app/common/v.js` response header.
pub async fn spot_price_qh(client: &Client, symbol: &str, token: &str) -> Result<Vec<SpotPriceQhRow>> {
    let html = client
        .get_text(SOURCE_FX168, "spot_price_qh", SPOT_QH_PAGE_URL, &[], None)
        .await?;
    let product_id = spot_qh_product_id(&html, symbol)?;
    let params: &[(&str, &str)] = &[
        ("productId", product_id.as_str()),
        ("pageNo", "1"),
        ("pageSize", "50000"),
        ("startDate", ""),
        ("endDate", "2050-01-01"),
        ("appCategory", "web"),
    ];
    let headers: &[(&str, &str)] = &[
        (
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
        ),
        ("_pcc", token),
        ("Origin", "https://www.99qh.com"),
        ("Referer", "https://www.99qh.com"),
    ];
    let v = client
        .get_json_with_headers(SOURCE_FX168, "spot_price_qh", SPOT_QH_API, params, Some(headers))
        .await?;
    parse_spot_price_qh(&v)
}

/// Resolve the `productId` for a variety name from the 99qh `spotTrend` page.
fn spot_qh_product_id(html: &str, symbol: &str) -> Result<String> {
    use scraper::{Html, Selector};
    let doc = Html::parse_document(html);
    let sel = Selector::parse("script#__NEXT_DATA__").map_err(|e| Error::Parse {
        endpoint: "spot_price_qh",
        message: e.to_string(),
    })?;
    let script = doc.select(&sel).next().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_FX168,
        message: "missing __NEXT_DATA__".into(),
    })?;
    let text: String = script.text().collect();
    let v: Value = serde_json::from_str(&text).map_err(|e| Error::Parse {
        endpoint: "spot_price_qh",
        message: e.to_string(),
    })?;
    let mut map: HashMap<String, String> = HashMap::new();
    if let Some(list) = v
        .get("props")
        .and_then(|p| p.get("pageProps"))
        .and_then(|p| p.get("data"))
        .and_then(|d| d.get("varietyListData"))
        .and_then(|x| x.as_array())
    {
        for item in list {
            if let Some(pl) = item.get("productList").and_then(|x| x.as_array()) {
                for p in pl {
                    let name = p.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    let pid = p
                        .get("productId")
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| {
                            p.get("productId")
                                .and_then(|x| x.as_i64())
                                .map(|n| n.to_string())
                        })
                        .unwrap_or_default();
                    if !name.is_empty() {
                        map.insert(name, pid);
                    }
                }
            }
        }
    }
    map.get(symbol)
        .cloned()
        .ok_or_else(|| Error::NotFound {
            endpoint: "spot_price_qh",
            message: format!("unknown symbol {symbol}"),
        })
}

/// Parse `data.list` (`[{date, fp, sp}, ...]`) from the 99qh `spot/trend` response.
pub(crate) fn parse_spot_price_qh(resp: &Value) -> Result<Vec<SpotPriceQhRow>> {
    let list = resp
        .get("data")
        .and_then(|d| d.get("list"))
        .and_then(|x| x.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_FX168,
            message: "missing data.list".into(),
        })?;
    let mut out = Vec::with_capacity(list.len());
    for row in list {
        let date = row
            .get("date")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let fp = row.get("fp").and_then(num_from_val);
        let sp = row.get("sp").and_then(num_from_val);
        out.push(SpotPriceQhRow {
            date,
            futures_price: fp,
            spot_price: sp,
            source: SOURCE_FX168,
        });
    }
    Ok(out)
}

/// Read a JSON scalar (number or numeric string) as `f64`.
fn num_from_val(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
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

    #[test]
    fn parses_spot_price_qh() {
        let rows = parse_spot_price_qh(&fixture("spot_price_qh.json")).unwrap();
        assert_eq!(rows.len(), 3273);
        assert_eq!(rows[0].date, "2026-08-14");
        assert_eq!(rows[0].futures_price, Some(3015.0));
        assert_eq!(rows[0].spot_price, Some(3022.0));
        assert_eq!(rows[0].source, SOURCE_FX168);
    }
}
