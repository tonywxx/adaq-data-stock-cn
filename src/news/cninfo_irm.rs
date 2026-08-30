//! 互动易问答（深沪统一走巨潮 irm.cninfo.com.cn）— ports `cninfo_irm` from the
//! `simonlin1212/a-stock-data` skill.
//!
//! Two-step call (per the upstream contract):
//! 1. `queryKeyboardInfo` resolves the org id (`secid`) for a 6-digit `code`.
//! 2. `company/question` lists the Q&A. Its parameters must travel in the query
//!    string with an *empty* body, otherwise the server answers HTTP 400.
//!
//! `Client::post_form_json` issues a POST whose params are serialized into the
//! query string (not a form body), which is exactly what step 2 requires.

use chrono::DateTime;
use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use crate::core::json::opt_str;

const SOURCE_CNINFO: &str = "cninfo";
const CNINFO_KB_URL: &str = "https://irm.cninfo.com.cn/newircs/index/queryKeyboardInfo";
const CNINFO_QA_URL: &str = "https://irm.cninfo.com.cn/newircs/company/question";

/// One Q&A row from 互动易.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CninfoIrmRow {
    /// `stockCode`
    pub code: Option<String>,
    /// `companyShortName`
    pub company: Option<String>,
    /// investor question (`mainContent`)
    pub question: Option<String>,
    /// company answer (`attachedContent`); `None` when unanswered
    pub answer: Option<String>,
    /// `attachedAuthor` (who answered)
    pub answerer: Option<String>,
    /// `pubDate` rendered as `YYYY-MM-DD HH:MM` (ms timestamp → local-naive UTC)
    pub ask_time: Option<String>,
    pub source: &'static str,
}

/// Port of `cninfo_irm(code, page_size, page_num)`.
///
/// `code` is a 6-digit A-share code (e.g. `"002594"`). Returns the latest
/// `page_size` Q&A on the given `page_num` (1-based).
pub async fn cninfo_irm(
    client: &Client,
    code: &str,
    page_size: u32,
    page_num: u32,
) -> Result<Vec<CninfoIrmRow>> {
    // Step 1: resolve the org id (secid) for this code.
    let kb = client
        .post_form_json(
            SOURCE_CNINFO,
            "cninfo_irm",
            CNINFO_KB_URL,
            &[("keyWord", code)],
            None,
        )
        .await?;
    let org_id = kb
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|a| a.first())
        .and_then(|x| x.get("secid"))
        .and_then(|s| s.as_str())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CNINFO,
            message: format!("no org id resolved for code {code}"),
        })?;

    // Step 2: list Q&A. Params go in the query string; body stays empty.
    let ps = page_size.to_string();
    let pn = page_num.to_string();
    let qa = client
        .post_form_json(
            SOURCE_CNINFO,
            "cninfo_irm",
            CNINFO_QA_URL,
            &[
                ("_t", "1"),
                ("stockcode", code),
                ("orgId", org_id),
                ("pageSize", ps.as_str()),
                ("pageNum", pn.as_str()),
                ("keyWord", ""),
                ("startDay", ""),
                ("endDay", ""),
            ],
            None,
        )
        .await?;
    parse_cninfo_irm(&qa)
}

/// Parse a `company/question` JSON envelope into [`CninfoIrmRow`]s.
pub(crate) fn parse_cninfo_irm(resp: &Value) -> Result<Vec<CninfoIrmRow>> {
    let rows = resp
        .get("rows")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CNINFO,
            message: "missing rows".into(),
        })?;
    let mut out = Vec::with_capacity(rows.len());
    for it in rows {
        let ask_time = it
            .get("pubDate")
            .and_then(|v| v.as_i64())
            .filter(|t| *t > 0)
            .and_then(|t| {
                let secs = t / 1000;
                let nanos = ((t % 1000) * 1_000_000) as u32;
                DateTime::from_timestamp(secs, nanos).map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            });
        out.push(CninfoIrmRow {
            code: opt_str(it, "stockCode"),
            company: opt_str(it, "companyShortName"),
            question: opt_str(it, "mainContent"),
            answer: opt_str(it, "attachedContent"),
            answerer: opt_str(it, "attachedAuthor"),
            ask_time,
            source: SOURCE_CNINFO,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_cninfo_irm_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/cninfo_irm.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_cninfo_irm(&v).unwrap();
        assert_eq!(rows.len(), 14);
        let r0 = &rows[0];
        assert_eq!(r0.code.as_deref(), Some("000001"));
        assert_eq!(r0.company.as_deref(), Some("平安银行"));
        assert!(r0.question.as_ref().map(|s| s.len()).unwrap_or(0) > 0);
        // ask_time is rendered from a millisecond timestamp
        assert!(r0.ask_time.as_ref().map(|s| s.len()).unwrap_or(0) >= 11);
    }
}
