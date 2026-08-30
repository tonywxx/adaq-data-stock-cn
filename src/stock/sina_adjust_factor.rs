//! 新浪复权因子 — ports `sina_adjust_factor` from the `simonlin1212/a-stock-data`
//! skill (Layer 1 Quote Layer).
//!
//! GET `https://finance.sina.com.cn/realstock/company/{prefix}{digits}/{kind}.js`
//! returns a JS assignment like:
//!
//! ```text
//! var sh600519qfq={"total":33,"data":[{"d":"2026-06-26","f":"1.0"}, ...]}; /* ... */
//! ```
//!
//! We extract the JSON object (between the first `{` and the trailing `/*`
//! comment), then map each `data[]` entry to a `{date, factor}` row.
//! `kind` is `"qfq"` (forward) or `"hfq"` (backward). 北交所 (920*) returns 404.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_SINA: &str = "sina";
const BASE: &str = "https://finance.sina.com.cn/realstock/company";

/// One adjust-factor row (`sina_adjust_factor`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SinaAdjustFactorRow {
    /// 除权除息日 `YYYY-MM-DD`（最新在前）
    pub date: Option<String>,
    /// 复权因子
    pub factor: Option<f64>,
    pub source: &'static str,
}

/// Map a 6-digit code to Sina's market prefix (`sh`/`sz`/`bj`).
fn sina_prefix(code: &str) -> &'static str {
    let c = code.trim();
    if c.starts_with("920") || c.starts_with("83") || c.starts_with("43") || c.starts_with('8') || c.starts_with('4') {
        "bj"
    } else if c.starts_with('6') || c.starts_with('5') || c.starts_with('9') {
        "sh"
    } else {
        "sz"
    }
}

/// Port of `sina_adjust_factor(code, kind)` — 个股前/后复权因子序列。
///
/// `kind` is `"qfq"` (forward) or `"hfq"` (backward). Returns rows newest-first.
pub async fn sina_adjust_factor(
    client: &Client,
    code: &str,
    kind: &str,
) -> Result<Vec<SinaAdjustFactorRow>> {
    if code.trim().starts_with("920") {
        return Err(Error::NotFound {
            endpoint: "sina_adjust_factor",
            message: format!("北交所(920*) 无复权因子 (code {code})"),
        });
    }
    if kind != "qfq" && kind != "hfq" {
        return Err(Error::InvalidParam(format!(
            "sina_adjust_factor: kind must be qfq|hfq, got {kind}"
        )));
    }
    let digits: String = code.trim().chars().filter(|ch| ch.is_ascii_digit()).collect();
    let url = format!("{BASE}/{}{digits}/{kind}.js", sina_prefix(code));
    let text = client
        .get_text(
            SOURCE_SINA,
            "sina_adjust_factor",
            &url,
            &[],
            Some(&[
                ("User-Agent", "Mozilla/5.0"),
                ("Referer", "https://finance.sina.com.cn/"),
            ]),
        )
        .await?;
    parse_sina_factor(&text)
}

/// Parse the Sina `var xxxqfq={...}` JS payload into factor rows.
pub(crate) fn parse_sina_factor(text: &str) -> Result<Vec<SinaAdjustFactorRow>> {
    let start = text
        .find('{')
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "sina_adjust_factor: no JSON object in response".into(),
        })?;
    // Cut at the trailing `/* ... */` comment if present; otherwise take the rest.
    // Also drop any trailing `;` between the object and that comment.
    let end = text.find("/*").unwrap_or(text.len());
    let json = text[start..end].trim_end().trim_end_matches(';').trim_end();
    let v: Value = serde_json::from_str(json).map_err(Error::Json)?;
    let arr = v
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "sina_adjust_factor: missing data array".into(),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for x in arr {
        out.push(SinaAdjustFactorRow {
            date: x.get("d").and_then(|d| d.as_str()).map(|s| s.to_string()),
            factor: match x.get("f") {
                Some(Value::Number(n)) => n.as_f64(),
                Some(Value::String(s)) => s.trim().parse::<f64>().ok(),
                _ => None,
            },
            source: SOURCE_SINA,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_factor() {
        let text = "var sh600519qfq={\"total\":2,\"data\":[{\"d\":\"2026-06-26\",\"f\":\"1.0000000000000000\"},{\"d\":\"2025-12-19\",\"f\":\"1.0236675985693000\"}]}; /* base64 */";
        let rows = parse_sina_factor(text).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date.as_deref(), Some("2026-06-26"));
        assert_eq!(rows[0].factor, Some(1.0));
        assert!((rows[1].factor.unwrap() - 1.0236675985693).abs() < 1e-9);
    }

    #[test]
    fn prefix_maps() {
        assert_eq!(sina_prefix("600519"), "sh");
        assert_eq!(sina_prefix("000001"), "sz");
        assert_eq!(sina_prefix("920575"), "bj");
    }

    #[test]
    fn parses_captured_fixture() {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sina_adjust_factor.txt");
        let text = std::fs::read_to_string(p).unwrap();
        let rows = parse_sina_factor(&text).unwrap();
        assert_eq!(rows.len(), 33);
        assert_eq!(rows[0].date.as_deref(), Some("2026-06-26"));
        assert_eq!(rows[0].factor, Some(1.0));
    }
}
