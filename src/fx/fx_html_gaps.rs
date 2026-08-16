//! FX HTML/HTML-fragment endpoints — akshare `fx/*`.
//!
//! * [`currency_pair_map`] — Investing.com currency-pair listing for a given
//!   base currency (`fx/currency_investing.py:16`). The upstream
//!   `Service/currency` XHR returns an HTML fragment of `<a>` links; akshare
//!   extracts each link's `href` (pair code) and `title` (pair name).

use scraper::{Html, Selector};

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_INVESTING: &str = "investing";

/// Headers required by Investing.com's `Service/*` XHR endpoints.
const INVESTING_HEADERS: &[(&str, &str)] = &[
    (
        "User-Agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/79.0.3945.130 Safari/537.36",
    ),
    ("Accept", "application/json, text/javascript, */*; q=0.01"),
    ("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8"),
    ("Cache-Control", "no-cache"),
    ("Pragma", "no-cache"),
    ("Referer", "https://cn.investing.com/currencies/single-currency-crosses"),
    ("X-Requested-With", "XMLHttpRequest"),
];

/// One investable currency pair for the chosen base currency.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CurrencyPairMapRow {
    /// Pair name (akshare `name`, `title` with spaces replaced by `-`).
    pub name: String,
    /// Pair code (akshare `code`, last path segment of the `href`).
    pub code: String,
}

/// 英为财情-外汇-指定货币的所有可获取货币对 (`currency_pair_map`, akshare
/// `fx/currency_investing.py:16`).
///
/// Mirrors akshare: query each region (`4,1,8,7,6`), build the
/// `symbol → continent-region` map, then fetch `Service/currency`. Investing
/// serves these behind a WAF, so the live call may be blocked; the parser is
/// verified against a captured fragment.
pub async fn currency_pair_map(client: &Client, symbol: &str) -> Result<Vec<CurrencyPairMapRow>> {
    let mut name_id_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for region_id in ["4", "1", "8", "7", "6"] {
        let url = "https://cn.investing.com/currencies/Service/region";
        let params: &[(&str, &str)] = &[("region_ID", region_id), ("currency_ID", "false")];
        let html = client
            .get_text(
                SOURCE_INVESTING,
                "currency_pair_map",
                url,
                params,
                Some(INVESTING_HEADERS),
            )
            .await?;
        let doc = Html::parse_document(&html);
        let sel = Selector::parse("[data-sml-id]:not([title])").map_err(|e| Error::Parse {
            endpoint: "currency_pair_map",
            message: format!("region selector: {e}"),
        })?;
        for item in doc.select(&sel) {
            let code = item
                .value()
                .attr("continentid")
                .map(|c| format!("{c}-{region_id}"));
            let name = item
                .select(&Selector::parse("i").unwrap())
                .next()
                .map(|i| i.text().collect::<String>().trim().to_string());
            if let (Some(c), Some(n)) = (code, name) {
                if !n.is_empty() {
                    name_id_map.insert(n, c);
                }
            }
        }
    }
    let key = name_id_map.get(symbol).ok_or_else(|| Error::Parse {
        endpoint: "currency_pair_map",
        message: format!("symbol {symbol} not found in Investing region map"),
    })?;
    let parts: Vec<&str> = key.split('-').collect();
    let url = "https://cn.investing.com/currencies/Service/currency";
    let params: &[(&str, &str)] = &[("region_ID", parts[1]), ("currency_ID", parts[0])];
    let html = client
        .get_text(
            SOURCE_INVESTING,
            "currency_pair_map",
            url,
            params,
            Some(INVESTING_HEADERS),
        )
        .await?;
    parse_currency_pair_map(&html, "currency_pair_map")
}

/// Parse the Investing.com `Service/currency` HTML fragment: every `<a>` yields
/// a pair `code` (last `href` path segment) and `name` (`title` with spaces → `-`).
pub(crate) fn parse_currency_pair_map(html: &str, endpoint: &'static str) -> Result<Vec<CurrencyPairMapRow>> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("a").map_err(|e| Error::Parse {
        endpoint,
        message: format!("a selector: {e}"),
    })?;
    let mut out = Vec::new();
    for a in doc.select(&sel) {
        let href = match a.value().attr("href") {
            Some(h) => h,
            None => continue,
        };
        let title = match a.value().attr("title") {
            Some(t) => t,
            None => continue,
        };
        let code = href
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_string();
        if code.is_empty() {
            continue;
        }
        let name = title.replace(' ', "-");
        out.push(CurrencyPairMapRow { name, code });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged {
            origin: SOURCE_INVESTING,
            message: "no currency-pair links found".into(),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn load_html(name: &str) -> String {
        let bytes = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(name),
        )
        .unwrap();
        match std::str::from_utf8(&bytes) {
            Ok(s) => s.to_string(),
            Err(_) => encoding_rs::GBK
                .decode_without_bom_handling_and_without_replacement(&bytes)
                .map(|c| c.into_owned())
                .unwrap_or_else(|| String::from_utf8_lossy(&bytes).into_owned()),
        }
    }

    #[test]
    fn parses_currency_pair_map() {
        let rows = parse_currency_pair_map(&load_html("currency_pair_map.html"), "currency_pair_map").unwrap();
        assert_eq!(rows.len(), 6);
        assert_eq!(rows[0].code, "cny-jmd");
        assert_eq!(rows[0].name, "人民币-牙买加元");
        assert_eq!(rows[1].code, "usd-cny");
        assert_eq!(rows[1].name, "美元-人民币");
        assert_eq!(rows[4].code, "100-jpy-cny");
        assert_eq!(rows[4].name, "100日元-人民币");
    }
}
