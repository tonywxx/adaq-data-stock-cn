//! Futures "gaps" ports: Sina main-continuous listing + 99qh inventory.
//!
//! - `futures_display_main_sina` — akshare `futures_derivative/futures_index_sina.py:89`
//!   (Sina `Market_Center.getHQFuturesData`). The listing endpoint is UTF-8 JSON
//!   (unicode-escaped Chinese), so the `Client`'s `get_json` works directly.
//! - `futures_inventory_99` — akshare `futures/futures_inventory_99.py:47`
//!   (fx168 `centerapi.fx168api.com` with akshare's hardcoded `_pcc` token).
//!
//! `futures_dce_position_rank` (akshare `futures/cot.py:818`) is intentionally
//! NOT ported here: that endpoint returns a **zip** of TSV files, which requires a
//! zip/deflate decompressor crate that is not in `Cargo.toml` (and may not be
//! added). See the agent report for details.

use std::collections::HashMap;

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_SINA: &str = "sina";
const SOURCE_FX168: &str = "fx168";

// ---------------------------------------------------------------------------
// futures_display_main_sina — Sina main continuous contract listing
// ---------------------------------------------------------------------------

const SINA_NODES_URL: &str =
    "http://vip.stock.finance.sina.com.cn/quotes_service/view/js/qihuohangqing.js";
const SINA_HQ_URL: &str = "http://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQFuturesData";

/// One Sina main-continuous (主力连续) contract.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesDisplayMainSinaRow {
    /// Contract code, e.g. `FU0` (akshare `symbol`).
    pub symbol: String,
    /// Exchange code, e.g. `shfe` (akshare `exchange`).
    pub exchange: String,
    /// Contract name, e.g. `燃料油连续` (akshare `name`).
    pub name: String,
    pub source: &'static str,
}

/// Sina main-continuous contract listing across all exchanges
/// (`futures_display_main_sina`, akshare `futures_index_sina.py:89`).
///
/// Mirrors akshare: read the exchange node list from `qihuohangqing.js`, then
/// for every `_qh` node query `getHQFuturesData` and keep rows whose `name`
/// contains `连续` and whose first digit is `0` (the main-continuous symbol).
pub async fn futures_display_main_sina(client: &Client) -> Result<Vec<FuturesDisplayMainSinaRow>> {
    let js = client
        .get_text(
            SOURCE_SINA,
            "futures_display_main_sina",
            SINA_NODES_URL,
            &[],
            None,
        )
        .await?;
    let nodes = sina_node_codes(&js);
    let mut out = Vec::new();
    for node in nodes {
        let params: &[(&str, &str)] = &[
            ("page", "1"),
            ("num", "50"),
            ("sort", "position"),
            ("asc", "0"),
            ("node", node.as_str()),
            ("base", "futures"),
        ];
        let headers: &[(&str, &str)] =
            &[("Referer", "https://finance.sina.com.cn/futuremarket/")];
        let resp = match client
            .get_json_with_headers(SOURCE_SINA, "futures_display_main_sina", SINA_HQ_URL, params, Some(headers))
            .await
        {
            Ok(v) => v,
            // A node with no data returns `[]` (or errors); skip it.
            Err(_) => continue,
        };
        match parse_futures_display_main_sina(&resp) {
            Ok(rows) => out.extend(rows),
            Err(_) => continue,
        }
    }
    Ok(out)
}

/// Extract every `'<code>_qh'` node code from the Sina `qihuohangqing.js`
/// document. The JS is gb2312 elsewhere, but the ASCII node codes survive a
/// UTF-8 decode, so a quote-delimited suffix scan is sufficient.
fn sina_node_codes(js: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = js.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\'' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] != b'\'' {
                j += 1;
            }
            if j < bytes.len() {
                let tok = &js[i + 1..j];
                if tok.ends_with("_qh") {
                    out.push(tok.to_string());
                }
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    out
}

/// Parse a single `getHQFuturesData` response array, keeping main-continuous
/// contracts (akshare keeps rows where `name` contains `连续` and the first
/// digit of `symbol` is `0`).
pub(crate) fn parse_futures_display_main_sina(resp: &Value) -> Result<Vec<FuturesDisplayMainSinaRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "expected JSON array".into(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let symbol = item.get("symbol").and_then(|v| v.as_str()).unwrap_or("");
        if !name.contains("连续") {
            continue;
        }
        // akshare: `symbol.str.extract(r"([\w])(\d)")` group 2 is the first digit.
        if symbol.chars().find(|c| c.is_ascii_digit()) != Some('0') {
            continue;
        }
        out.push(FuturesDisplayMainSinaRow {
            symbol: symbol.to_string(),
            exchange: item
                .get("exchange")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            name: name.to_string(),
            source: SOURCE_SINA,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// futures_inventory_99 — 99qh commodity inventory (fx168)
// ---------------------------------------------------------------------------

const INV_99_PAGE_URL: &str = "https://www.99qh.com/data/stockIn";
const INV_99_API: &str = "https://centerapi.fx168api.com/app/qh/api/stock/trend";
/// Hardcoded `_pcc` token copied verbatim from akshare `futures_inventory_99.py`.
const INV_99_TOKEN: &str = "DJKijwhimCjFLvYe7p2Evo5OnkSZ/sohOcXWRKQiwxhWKtezlhkQwqkaFeAVaF8h/H8Qx7u6Ew80tAI2ph2bQEQwUP1y+6m8tEecTQSZtLbjtgtqg1FijxNIwgzGaIn9vVfujlOTDFCLkUJWSKuCcTm/diD9X/lhoFSaqJxB56E=";

/// One 99qh inventory observation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesInventory99Row {
    /// Trade date (`YYYY-MM-DD`, akshare `日期`).
    pub date: String,
    /// Settlement close price (akshare `收盘价`).
    pub close: Option<f64>,
    /// Exchange warehouse inventory (akshare `库存`).
    pub inventory: Option<f64>,
    pub source: &'static str,
}

/// 99qh commodity inventory (`futures_inventory_99`, akshare `futures_inventory_99.py:47`).
///
/// `symbol` is the Chinese variety name, e.g. `豆一`. The product id is resolved
/// from the `stockIn` page's `__NEXT_DATA__`, then the `stock/trend` API is
/// queried with akshare's hardcoded `_pcc` token.
pub async fn futures_inventory_99(client: &Client, symbol: &str) -> Result<Vec<FuturesInventory99Row>> {
    let html = client
        .get_text(SOURCE_FX168, "futures_inventory_99", INV_99_PAGE_URL, &[], None)
        .await?;
    let product_id = inv_99_product_id(&html, symbol)?;
    let params: &[(&str, &str)] = &[
        ("productId", product_id.as_str()),
        ("type", "1"),
        ("pageNo", "1"),
        ("pageSize", "5000"),
        ("startDate", ""),
        ("endDate", "2050-01-01"),
        ("appCategory", "web"),
    ];
    let headers: &[(&str, &str)] = &[
        ("Content-Type", "application/json;charset=UTF-8"),
        ("_pcc", INV_99_TOKEN),
        ("Referer", "https://www.99qh.com"),
        ("Origin", "https://www.99qh.com"),
    ];
    let v = client
        .get_json_with_headers(SOURCE_FX168, "futures_inventory_99", INV_99_API, params, Some(headers))
        .await?;
    parse_futures_inventory_99(&v)
}

/// Parse `productId` for a variety name out of the 99qh `stockIn` page.
fn inv_99_product_id(html: &str, symbol: &str) -> Result<String> {
    use scraper::{Html, Selector};
    let doc = Html::parse_document(html);
    let sel = Selector::parse("script#__NEXT_DATA__").map_err(|e| Error::Parse {
        endpoint: "futures_inventory_99",
        message: e.to_string(),
    })?;
    let script = doc.select(&sel).next().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_FX168,
        message: "missing __NEXT_DATA__".into(),
    })?;
    let text: String = script.text().collect();
    let v: Value = serde_json::from_str(&text).map_err(|e| Error::Parse {
        endpoint: "futures_inventory_99",
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
                    let pid: String = match p.get("productId").and_then(|x| x.as_str()) {
                        Some(s) => s.to_string(),
                        None => p
                            .get("productId")
                            .and_then(|x| x.as_i64())
                            .map(|n| n.to_string())
                            .unwrap_or_default(),
                    };
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
            endpoint: "futures_inventory_99",
            message: format!("unknown symbol {symbol}"),
        })
}

/// Parse `data.list` (a list of `[date, close, inventory]` rows) from the
/// 99qh `stock/trend` response.
pub(crate) fn parse_futures_inventory_99(resp: &Value) -> Result<Vec<FuturesInventory99Row>> {
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
        let arr = row.as_array().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_FX168,
            message: "expected array row".into(),
        })?;
        let date = arr
            .first()
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let close = arr.get(1).and_then(num_from_val);
        let inventory = arr.get(2).and_then(num_from_val);
        out.push(FuturesInventory99Row {
            date,
            close,
            inventory,
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
    fn parses_futures_display_main_sina() {
        let rows = parse_futures_display_main_sina(&fixture("futures_display_main_sina.json")).unwrap();
        // Fixture (shfe `ry_qh`) has two 连续 rows; only FU0 keeps the main-continuous
        // filter (first digit == '0'); FU2610 is excluded.
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "FU0");
        assert_eq!(rows[0].exchange, "shfe");
        assert_eq!(rows[0].name, "燃料油连续");
        assert_eq!(rows[0].source, SOURCE_SINA);
    }

    #[test]
    fn parses_futures_inventory_99() {
        let rows = parse_futures_inventory_99(&fixture("futures_inventory_99.json")).unwrap();
        assert_eq!(rows.len(), 4349);
        assert_eq!(rows[0].date, "2026-08-14");
        assert_eq!(rows[0].close, Some(4998.0));
        assert_eq!(rows[0].inventory, Some(44401.0));
        // Some rows carry null/empty numerics -> parsed as None.
        assert!(rows.iter().any(|r| r.close.is_none() || r.inventory.is_none()));
    }

    #[test]
    fn sina_node_codes_extracts_qh_nodes() {
        let js = "dce : ['大连商品交易所', ['PTA', 'pta_qh', '16'], ['豆一', 'a_qh', '3']]";
        let nodes = sina_node_codes(js);
        assert_eq!(nodes, vec!["pta_qh".to_string(), "a_qh".to_string()]);
    }
}
