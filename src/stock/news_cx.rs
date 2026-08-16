//! 财新网-财新数据通-新闻 (Caixin news feed).
//!
//! Ports `akshare/stock/stock_news_cx.py:13`. JSON GET to
//! `cxdata.caixin.com/api/dataplus/sjtPc/news` with a `Referer` header; reads
//! `data.data` and keeps the `tag` / `summary` / `url` columns.
//!
//! | Rust fn | akshare fn | source |
//! |---|---|---|
//! | `stock_news_main_cx` | `stock_news_main_cx` | `akshare/stock/stock_news_cx.py:13` |
//!
//! ## DEFERRED
//! None.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE: &str = "caixin";
const URL: &str = "https://cxdata.caixin.com/api/dataplus/sjtPc/news";

fn str_of(v: Option<&Value>) -> Option<String> {
    match v {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct NewsMainCxRow {
    /// 标签 (`tag`).
    pub tag: Option<String>,
    /// 摘要 (`summary`).
    pub summary: Option<String>,
    /// 链接 (`url`).
    pub url: Option<String>,
}

/// Parse `stock_news_main_cx` rows from the already-fetched `Value`.
pub(crate) fn parse_news_main_cx(resp: &Value) -> Result<Vec<NewsMainCxRow>> {
    let arr = resp
        .get("data")
        .and_then(|d| d.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing data.data".into(),
        })?;
    Ok(arr
        .iter()
        .filter_map(|o| {
            let url = str_of(o.get("url"))?;
            Some(NewsMainCxRow {
                tag: str_of(o.get("tag")),
                summary: str_of(o.get("summary")),
                url: Some(url),
            })
        })
        .collect())
}

/// Port of `stock_news_main_cx()` — Caixin data-feed news.
pub async fn stock_news_main_cx(client: &Client) -> Result<Vec<NewsMainCxRow>> {
    let params = [
        ("pageNum", "1"),
        ("pageSize", "100"),
        ("showLabels", "true"),
    ];
    let headers = [
        ("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36"),
        ("referer", "https://cxdata.caixin.com/index/newsTab?tab=latest"),
    ];
    let v = client
        .get_json_with_headers(SOURCE, "stock_news_main_cx", URL, &params, Some(&headers))
        .await?;
    parse_news_main_cx(&v)
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

    #[test]
    fn parses_news_main_cx() {
        let rows = parse_news_main_cx(&fixture("stock_news_main_cx.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].tag.as_deref(), Some("宏观"));
        assert!(rows[0].summary.as_ref().unwrap().contains("PMI"));
        assert!(rows[0].url.as_ref().unwrap().starts_with("https://"));
    }
}
