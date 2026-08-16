//! SHMET news flash (`futures_news_shmet`).
//!
//! Ports akshare `futures_news_shmet`: Shanghai Metals Market (SHMET) publishes
//! a news-flash list via a JSON POST to
//! `https://www.shmet.com/api/rest/news/queryNewsflashList`. The `dataList`
//! items carry a publish timestamp (epoch millis) and content; akshare maps
//! the 4th and 6th columns to `发布时间` / `内容`.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// One SHMET news-flash item (`futures_news_shmet`).
///
/// akshare columns: 发布时间 (`publish_time`), 内容 (`content`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesNewsShmetRow {
    pub publish_time: Option<String>,
    pub content: Option<String>,
}

const FLASH_TAG_MAP: &[(&str, &str)] = &[
    ("要闻", "0"),
    ("VIP", "100"),
    ("财经", "999"),
    ("铜", "1002"),
    ("铝", "1003"),
    ("铅", "1005"),
    ("锌", "1004"),
    ("镍", "1006"),
    ("锡", "1007"),
    ("贵金属", "1008"),
    ("小金属", "1009"),
];

/// SHMET news flash (`futures_news_shmet`).
///
/// `symbol` is one of `{"全部", "要闻", "VIP", "财经", "铜", "铝", "铅", "锌",
/// "镍", "锡", "贵金属", "小金属"}` (akshare vocabulary). The `全部` branch
/// fetches a single page of 100 items; any other tag uses its `flashTag` code.
pub async fn futures_news_shmet(client: &Client, symbol: &str) -> Result<Vec<FuturesNewsShmetRow>> {
    let body: Value = if symbol == "全部" {
        serde_json::json!({"currentPage": 1, "pageSize": 100})
    } else {
        let tag = FLASH_TAG_MAP
            .iter()
            .find(|(k, _)| *k == symbol)
            .map(|(_, v)| *v)
            .ok_or_else(|| Error::InvalidParam(format!("unknown symbol: {symbol}")))?;
        serde_json::json!({"currentPage": 1, "pageSize": 2000, "content": "", "flashTag": tag})
    };
    let v = client
        .post_json(
            "shmet",
            "futures_news_shmet",
            "https://www.shmet.com/api/rest/news/queryNewsflashList",
            &body,
            None,
        )
        .await?;
    parse_news(&v)
}

/// Parse the SHMET `queryNewsflashList` JSON into rows.
pub(crate) fn parse_news(resp: &Value) -> Result<Vec<FuturesNewsShmetRow>> {
    let list = resp
        .get("data")
        .and_then(|d| d.get("dataList"))
        .and_then(|l| l.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: "shmet",
            message: "missing data.dataList".into(),
        })?;
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        out.push(FuturesNewsShmetRow {
            publish_time: pick_str(item, &["publishTime", "publishtime", "createTime", "time"]),
            content: pick_str(item, &["content", "contentText", "title"]),
        });
    }
    Ok(out)
}

fn pick_str(item: &Value, keys: &[&str]) -> Option<String> {
    for k in keys {
        if let Some(v) = item.get(k) {
            if let Some(s) = v.as_str() {
                return Some(s.to_string());
            }
            // Numeric publish time (epoch millis) -> keep as string.
            if let Some(n) = v.as_u64() {
                return Some(n.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_news_shmet_fixture() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/futures_news_shmet.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_news(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].publish_time.as_deref(), Some("1700000000000"));
        assert_eq!(rows[0].content.as_deref(), Some("铜价上涨"));
        assert_eq!(rows[1].content.as_deref(), Some("铝库存下降"));
    }
}
