use serde::Serialize;
use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};
use crate::news::fstr;

const BASE: &str = "https://search-api-web.eastmoney.com/search/jsonp";
/// Fixed JSONP callback name; we strip the matching wrapper before parsing.
const CB: &str = "adaq_jsonp";

/// Individual-stock news from Eastmoney (`stock_news_em`).
///
/// Mirrors akshare: the upstream `search-api-web` endpoint returns JSONP, so we fetch
/// as text, strip the `cb(...)` wrapper, and decode the inner JSON.
#[derive(Debug, Clone, Serialize)]
pub struct NewsRow {
    pub date: String,
    pub title: String,
    pub url: String,
    pub content: String,
    /// Always `"eastmoney"`.
    pub source: &'static str,
}

/// Eastmoney individual-stock news (`stock_news_em`).
///
/// Returns up to `pageSize` (default 10) recent news items for `symbol`.
pub async fn stock_news_em(client: &Client, symbol: &str) -> Result<Vec<NewsRow>> {
    let inner = serde_json::json!({
        "uid": "",
        "keyword": symbol,
        "type": ["cmsArticleWebOld"],
        "client": "web",
        "clientType": "web",
        "clientVersion": "curr",
        "param": {
            "cmsArticleWebOld": {
                "searchScope": "default",
                "sort": "default",
                "pageIndex": 1,
                "pageSize": 10,
                "preTag": "<em>",
                "postTag": "</em>"
            }
        }
    });
    let param_s = serde_json::to_string(&inner).map_err(Error::Json)?;
    // `_` is just a cache-buster; a static value is sufficient.
    let params = [("cb", CB), ("param", param_s.as_str()), ("_", "1")];
    let text = client
        .get_text(SOURCE_EASTMONEY, "stock_news_em", BASE, &params, None)
        .await?;
    let json_text = strip_jsonp(&text, CB)?;
    let v: Value = serde_json::from_str(&json_text).map_err(Error::Json)?;
    parse(&v)
}

/// Remove the `cb(...)` JSONP wrapper from a response body.
fn strip_jsonp(text: &str, cb: &str) -> Result<String> {
    let s = text.trim();
    let prefix = format!("{cb}(");
    if !s.starts_with(&prefix) {
        return Err(Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: format!("expected JSONP wrapper `{prefix}...)`"),
        });
    }
    let body = &s[prefix.len()..];
    let body = body.strip_suffix(')').unwrap_or(body);
    Ok(body.to_string())
}

/// Map the decoded Eastmoney JSON to [`NewsRow`]s, skipping items without a `code`.
pub(crate) fn parse(resp: &Value) -> Result<Vec<NewsRow>> {
    let arr = resp
        .get("result")
        .and_then(|r| r.get("cmsArticleWebOld"))
        .and_then(|a| a.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.cmsArticleWebOld".into(),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        // akshare strips the `<em>` highlight tags; we do the same.
        let code = fstr(item, "code");
        if code.is_empty() {
            continue;
        }
        let date = fstr(item, "date");
        let title = strip_em(&fstr(item, "title"));
        let content = strip_em(&fstr(item, "content"));
        let url = format!("http://finance.eastmoney.com/a/{code}.html");
        out.push(NewsRow {
            date,
            title,
            url,
            content,
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

/// Drop the `<em>` / `</em>` highlight tags introduced by the search API.
fn strip_em(s: &str) -> String {
    s.replace("<em>", "").replace("</em>", "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_stock_news_em_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/stock_news_em.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2025-12-23 17:15:00");
        assert_eq!(rows[0].title, "某某公司利好公告");
        assert_eq!(rows[0].url, "http://finance.eastmoney.com/a/603777.html");
        assert_eq!(rows[0].content, "正文内容重点。");
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].title, "另一条新闻");
    }
}
