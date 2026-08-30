//! 财联社电报 (Cailianpress telegraph) — ports `cls_telegraph` from the
//! `simonlin1212/a-stock-data` skill (v3.7.1).
//!
//! Zero-key local signature: `sign = md5(sha1(sorted-query-string))`. No API
//! key or server-side secret is involved — the query string is signed locally.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use crate::core::json::*;
use sha1::Digest;

const SOURCE_CLS: &str = "cls";
const ROLL_URL: &str = "https://www.cls.cn/v1/roll/get_roll_list";

/// One 财联社 telegraph (电报) item.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ClsTelegraphRow {
    /// `id`
    pub id: Option<i64>,
    /// `ctime` — unix seconds
    pub ctime: Option<i64>,
    /// `title`
    pub title: String,
    /// `brief`
    pub brief: String,
    /// `content`
    pub content: String,
    /// `level` (e.g. `C`)
    pub level: String,
    /// `reading_num`
    pub reading_num: Option<i64>,
    /// `comment_num`
    pub comment_num: Option<i64>,
    /// `share_num`
    pub share_num: Option<i64>,
    pub source: &'static str,
}

/// Fetch the latest 财联社 telegraph (电报) items.
///
/// `page_size` controls how many items to return (the upstream caps per page).
pub async fn telegraph(client: &Client, page_size: u32) -> Result<Vec<ClsTelegraphRow>> {
    let rn = page_size.to_string();
    let params = [
        ("appName", "CailianpressWeb"),
        ("last_time", ""),
        ("os", "web"),
        ("refresh_type", "1"),
        ("rn", rn.as_str()),
        ("sv", "7.7.5"),
    ];
    let sign = cls_sign(&params);
    let mut all: Vec<(&str, &str)> = Vec::with_capacity(params.len() + 1);
    all.extend_from_slice(&params);
    all.push(("sign", &sign));
    let v = client
        .get_json_with_headers(
            SOURCE_CLS,
            "cls_telegraph",
            ROLL_URL,
            &all,
            Some(&[("Referer", "https://www.cls.cn/")]),
        )
        .await?;
    parse_telegraph(&v)
}

/// Build the zero-key 财联社 `sign = md5(sha1(sorted-query-string))`.
fn cls_sign(params: &[(&str, &str)]) -> String {
    let mut items: Vec<(&str, &str)> = params.to_vec();
    items.sort_by(|a, b| a.0.cmp(b.0));
    let qs: String = items
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");
    let sha_hex = format!("{:x}", sha1::Sha1::digest(qs.as_bytes()));
    format!("{:x}", md5::Md5::digest(sha_hex.as_bytes()))
}

/// Parse an `data.roll_data` array into [`ClsTelegraphRow`]s.
pub(crate) fn parse_telegraph(resp: &Value) -> Result<Vec<ClsTelegraphRow>> {
    let roll = resp
        .get("data")
        .and_then(|d| d.get("roll_data"))
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CLS,
            message: "missing data.roll_data".into(),
        })?;
    let mut out = Vec::with_capacity(roll.len());
    for item in roll {
        out.push(ClsTelegraphRow {
            id: opt_i64(item, "id"),
            ctime: opt_i64(item, "ctime"),
            title: opt_str_or(item, "title", ""),
            brief: opt_str_or(item, "brief", ""),
            content: opt_str_or(item, "content", ""),
            level: opt_str_or(item, "level", ""),
            reading_num: opt_i64(item, "reading_num"),
            comment_num: opt_i64(item, "comment_num"),
            share_num: opt_i64(item, "share_num"),
            source: SOURCE_CLS,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_cls_telegraph_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/cls_telegraph.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_telegraph(&v).unwrap();
        assert_eq!(rows.len(), 10);
        assert_eq!(rows[0].ctime, Some(1788055968));
        assert!(rows[0].content.contains("财联社"));
        assert_eq!(rows[0].source, "cls");
        assert_eq!(rows[0].reading_num, Some(72481));
    }

    #[test]
    fn sign_is_deterministic() {
        let p = [
            ("appName", "CailianpressWeb"),
            ("last_time", ""),
            ("os", "web"),
            ("refresh_type", "1"),
            ("rn", "10"),
            ("sv", "7.7.5"),
        ];
        assert_eq!(cls_sign(&p), "c5aa50381fc8b5af4355b75dd16ca7fc");
    }
}
