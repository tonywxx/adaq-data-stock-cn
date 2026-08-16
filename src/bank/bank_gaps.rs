//! Bank regulatory administrative-penalty ports (`akshare/bank/bank_cbirc_2020.py`).
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `bank_fjcf_table_detail` | `bank_cbirc_2020.py:111` | NFRA penalty disclosure tables |
//!
//! The NFRA list endpoint and the per-document `docClob` JSON are reachable
//! **without any token or JS** (a plain `GET`), so this function is portable.
//! The live site now renders each disclosure as a horizontal 5-column table
//! (`序号 / 当事人名称 / 主要违法违规行为 / 行政处罚内容 / 作出决定机关`); we
//! surface one row per party per document.
//!
//! ## DEFERRED (no code below — see report)
//!
//! - `bank_fjcf_page_url` / `bank_fjcf_total_num` / `bank_fjcf_total_page`
//!   (`bank_cbirc_2020.py:76/:22/:47`) — pagination helpers; the task scopes
//!   them out as token/JS-gated list enumeration. `bank_fjcf_table_detail`
//!   performs its own enumeration via the same list endpoint.

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use scraper::{Html, Selector};
use serde_json::Value;

/// Source bucket for the NFRA (formerly CBIRC) penalty site.
const SOURCE_NFRA: &str = "nfra";

/// One party named in an NFRA administrative-penalty disclosure.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BankFjcfRow {
    /// Disclosure serial within the document (akshare `序号`).
    pub serial: Option<i64>,
    /// Named party / institution (akshare `当事人名称`).
    pub party: String,
    /// Principal violation (akshare `主要违法违规行为`).
    pub violation: String,
    /// Penalty content (akshare `行政处罚内容`).
    pub penalty: String,
    /// Authority that issued the penalty (akshare `作出决定机关`).
    pub authority: String,
    /// Source disclosure id (from the list endpoint).
    pub doc_id: String,
    /// Disclosure publish date (from the list endpoint).
    pub publish_date: String,
}

/// NFRA administrative-penalty disclosures (`bank_fjcf_table_detail`,
/// akshare `bank/bank_cbirc_2020.py:111`).
///
/// Enumerates document ids for `分局本级` (item 4115) via the public list
/// endpoint, then scrapes each document's `docClob` HTML table.
pub async fn bank_fjcf_table_detail(client: &Client) -> Result<Vec<BankFjcfRow>> {
    let list_url = "https://www.nfra.gov.cn/cbircweb/DocInfo/SelectDocByItemIdAndChild";
    let params: &[(&str, &str)] = &[("itemId", "4115"), ("pageSize", "18"), ("pageIndex", "1")];
    let list = client
        .get_json(SOURCE_NFRA, "bank_fjcf_table_detail", list_url, params)
        .await?;
    let rows = list
        .get("data")
        .and_then(|d| d.get("rows"))
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_NFRA,
            message: "list response missing data.rows".into(),
        })?;

    let mut out = Vec::new();
    for r in rows {
        let doc_id = r
            .get("docId")
            .and_then(|v| v.as_u64())
            .map(|x| x.to_string())
            .unwrap_or_default();
        let publish_date = r
            .get("publishDate")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if doc_id.is_empty() {
            continue;
        }
        let url = format!(
            "https://www.nfra.gov.cn/cn/static/data/DocInfo/SelectByDocId/data_docId={doc_id}.json"
        );
        // A single bad document shouldn't abort the whole batch.
        let doc: Value = match client
            .get_json(SOURCE_NFRA, "bank_fjcf_table_detail", &url, &[])
            .await
        {
            Ok(d) => d,
            Err(_) => continue,
        };
        let clob = doc
            .get("data")
            .and_then(|d| d.get("docClob"))
            .and_then(|c| c.as_str())
            .unwrap_or("");
        if clob.is_empty() {
            continue;
        }
        match parse_bank_fjcf_table_detail(clob) {
            Ok(mut parsed) => {
                for row in parsed.iter_mut() {
                    row.doc_id = doc_id.clone();
                    row.publish_date = publish_date.clone();
                }
                out.extend(parsed);
            }
            Err(_) => continue,
        }
    }
    Ok(out)
}

/// Parse an NFRA `docClob` HTML table into [`BankFjcfRow`]s.
pub(crate) fn parse_bank_fjcf_table_detail(html: &str) -> Result<Vec<BankFjcfRow>> {
    let fragment = Html::parse_document(html);
    let table_sel = Selector::parse("table").unwrap();
    let tr_sel = Selector::parse("tr").unwrap();
    let cell_sel = Selector::parse("th, td").unwrap();

    let table = fragment.select(&table_sel).next().ok_or_else(|| Error::Parse {
        endpoint: SOURCE_NFRA,
        message: "no <table> in docClob".into(),
    })?;

    let mut rows: Vec<Vec<String>> = Vec::new();
    for tr in table.select(&tr_sel) {
        let cells: Vec<String> = tr
            .select(&cell_sel)
            .map(|e| e.text().collect::<String>().trim().to_string())
            .collect();
        if !cells.is_empty() {
            rows.push(cells);
        }
    }
    if rows.is_empty() {
        return Err(Error::Parse {
            endpoint: SOURCE_NFRA,
            message: "docClob table has no rows".into(),
        });
    }

    // The first row is the column header; map known labels to column indexes.
    // Some documents use <td> for the header, some <th> — both land in `rows[0]`.
    let header = &rows[0];
    let idx = |label: &str| header.iter().position(|c| c == label);
    let i_serial = idx("序号");
    let i_party = idx("当事人名称");
    let i_violation = idx("主要违法违规行为");
    let i_penalty = idx("行政处罚内容");
    let i_authority = idx("作出决定机关");

    let mut out = Vec::new();
    for cells in &rows[1..] {
        if cells.iter().all(|c| c.is_empty()) {
            continue;
        }
        let serial = i_serial
            .and_then(|i| cells.get(i))
            .and_then(|c| c.trim().parse::<i64>().ok());
        let party = i_party
            .and_then(|i| cells.get(i))
            .cloned()
            .unwrap_or_default();
        let violation = i_violation
            .and_then(|i| cells.get(i))
            .cloned()
            .unwrap_or_default();
        let penalty = i_penalty
            .and_then(|i| cells.get(i))
            .cloned()
            .unwrap_or_default();
        let authority = i_authority
            .and_then(|i| cells.get(i))
            .cloned()
            .unwrap_or_default();
        out.push(BankFjcfRow {
            serial,
            party,
            violation,
            penalty,
            authority,
            doc_id: String::new(),
            publish_date: String::new(),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> String {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        std::fs::read_to_string(p).unwrap()
    }

    #[test]
    fn parses_bank_fjcf_table_detail() {
        let rows = parse_bank_fjcf_table_detail(&fixture("bank_fjcf_table_detail.html")).unwrap();
        assert!(!rows.is_empty(), "expected >0 rows");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].serial, Some(1));
        assert_eq!(
            rows[0].party,
            "萍乡农村商业银行股份有限公司湘东支行及相关责任人"
        );
        assert_eq!(rows[0].violation, "违规发放按揭贷款");
        assert!(rows[0].penalty.contains("罚款50万元"));
        assert_eq!(rows[0].authority, "国家金融监督管理总局萍乡监管分局");
        // doc_id / publish_date are filled by the async caller, not the fixture.
        assert_eq!(rows[0].doc_id, "");
    }
}
