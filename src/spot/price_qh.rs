//! 99 期货 (99qh) 期现数据. Ports `akshare/spot/spot_price_qh.py`.
//!
//! `spot_price_table_qh` reads the server-rendered Next.js `__NEXT_DATA__`
//! JSON embedded in `https://www.99qh.com/data/spotTrend` (no client JS
//! execution needed — the payload is static HTML text we extract by locating
//! the `<script id="__NEXT_DATA__">` block). We then walk
//! `props.pageProps.data.varietyListData[].productList` and keep the exchange
//! name + variety name, matching akshare's final column select.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `spot_price_table_qh` | `spot_price_qh.py:55` | 交易所与品种对照表 |
//!
//! ## DEFERRED
//! - `spot_price_qh` (`spot_price_qh.py:79`): requires a dynamic anti-bot
//!   `_pcc` token from `https://centerapi.fx168api.com/app/common/v.js`
//!   (response header) plus the `__NEXT_DATA__` variety list; the `_pcc` token
//!   is the same class of anti-bot credential as an x-csrf-token, so defer.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE: &str = "99qh";
const URL: &str = "https://www.99qh.com/data/spotTrend";
const NEXT_DATA_MARKER: &str = r#"<script id="__NEXT_DATA__">"#;

/// 99 期货-数据-期现-交易所与品种对照表.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SpotPriceTableRow {
    /// 交易所名称 (`qhExchangeName`).
    pub exchange_name: String,
    /// 品种名称 (`name`).
    pub variety_name: String,
}

/// Extract the embedded `__NEXT_DATA__` JSON text from the spotTrend HTML page.
fn extract_next_data(html: &str) -> Result<String> {
    let start = html
        .find(NEXT_DATA_MARKER)
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing __NEXT_DATA__ script".into(),
        })?
        + NEXT_DATA_MARKER.len();
    let rest = &html[start..];
    let end = rest
        .find("</script>")
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "unterminated __NEXT_DATA__ script".into(),
        })?;
    Ok(rest[..end].to_string())
}

/// Parse `spot_price_table_qh` rows from the already-extracted `__NEXT_DATA__` JSON.
pub(crate) fn parse_spot_price_table(resp: &Value) -> Result<Vec<SpotPriceTableRow>> {
    let variety = resp
        .get("props")
        .and_then(|p| p.get("pageProps"))
        .and_then(|p| p.get("data"))
        .and_then(|d| d.get("varietyListData"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing props.pageProps.data.varietyListData".into(),
        })?;

    let mut out = Vec::new();
    for group in variety {
        let Some(products) = group.get("productList").and_then(|p| p.as_array()) else {
            continue;
        };
        for item in products {
            let exchange_name = item
                .get("qhExchangeName")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let variety_name = item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            out.push(SpotPriceTableRow {
                exchange_name,
                variety_name,
            });
        }
    }
    Ok(out)
}

/// 99 期货-数据-期现-交易所与品种对照表 (`https://www.99qh.com/data/spotTrend`).
pub async fn spot_price_table_qh(client: &Client) -> Result<Vec<SpotPriceTableRow>> {
    let html = client
        .get_text(SOURCE, "spot_price_table_qh", URL, &[], None)
        .await?;
    let json_text = extract_next_data(&html)?;
    let v: Value = serde_json::from_str(&json_text).map_err(|e| Error::UpstreamChanged {
        origin: SOURCE,
        message: format!("invalid __NEXT_DATA__ json: {e}"),
    })?;
    parse_spot_price_table(&v)
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

    #[test]
    fn parse_spot_price_table_ok() {
        let rows = parse_spot_price_table(&fixture("spot_price_table_qh.json")).unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].exchange_name, "上海期货交易所");
        assert_eq!(rows[0].variety_name, "螺纹钢");
        assert_eq!(rows[1].exchange_name, "上海期货交易所");
        assert_eq!(rows[1].variety_name, "铜");
        assert_eq!(rows[2].exchange_name, "大连商品交易所");
        assert_eq!(rows[2].variety_name, "豆粕");
        assert_eq!(rows[3].variety_name, "玉米");
    }

    #[test]
    fn extract_next_data_ok() {
        let html = r#"<html><head><script id="__NEXT_DATA__">{"props":{"pageProps":{"data":{"varietyListData":[]}}}}</script></head></html>"#;
        let json = extract_next_data(html).unwrap();
        assert!(json.contains("varietyListData"));
        let _: Value = serde_json::from_str(&json).unwrap();
    }
}
