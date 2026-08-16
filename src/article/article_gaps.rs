//! Article-domain HTML/JS-data ports that are reachable without a JS engine.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `article_ff_crr` | `article/ff_factor.py:17` | Fama/French factor table on the Dartmouth data-library page |
//! | `article_rlab_rv` | `article/risk_rv.py:117` | Dacheng Xiu individual-stock realized volatility |
//!
//! ## DEFERRED (no code below — see report)
//!
//! - `article_oman_rv` (`article/risk_rv.py:18`) and `article_oman_rv_short`
//!   (`article/risk_rv.py:78`) — both read JSON embedded in JS files served
//!   from `realized.oxford-man.ox.ac.uk`, which is currently **unreachable**
//!   (connection timeout). No captured fixture is possible.

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use scraper::{Html, Selector};

/// Source bucket for the Dartmouth Fama/French library.
const SOURCE_FRENCH: &str = "french";
/// Source bucket for the Dacheng Xiu realized-volatility site.
const SOURCE_RLAB: &str = "rlab";

/// One Fama/French research-factor group and its recent return figures.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FfCrrRow {
    /// Factor group label (akshare group row, e.g. `Fama/French 3 Research Factors`).
    pub group: String,
    /// Returns for the current month (akshare `June 2026` column header).
    pub june_2026: String,
    /// Returns over the last 3 months.
    pub last_3_months: String,
    /// Returns over the last 12 months.
    pub last_12_months: String,
}

/// Fama/French factor summary (`article_ff_crr`, akshare `article/ff_factor.py:17`).
///
/// The live Dartmouth page consolidated the formerly separate factor tables
/// into a single table whose first cell of each group row names the factor set
/// and whose remaining cells hold space-separated returns. We surface one row
/// per factor group.
pub async fn article_ff_crr(client: &Client) -> Result<Vec<FfCrrRow>> {
    let url = "https://mba.tuck.dartmouth.edu/pages/faculty/ken.french/data_library.html";
    let html = client
        .get_text(SOURCE_FRENCH, "article_ff_crr", url, &[], None)
        .await?;
    parse_article_ff_crr(&html)
}

/// Parse `article_ff_crr` from captured HTML.
pub(crate) fn parse_article_ff_crr(html: &str) -> Result<Vec<FfCrrRow>> {
    let fragment = Html::parse_document(html);
    let table_sel = Selector::parse("table").unwrap();
    let tr_sel = Selector::parse("tr").unwrap();
    let cell_sel = Selector::parse("th, td").unwrap();

    // The factor-summary table is headed by the return-period columns
    // (`Last 3 Months`). On the live page this table is nested inside an outer
    // layout `<table>`, so `scraper` flattens the outer row + the 4 inner rows
    // together. We therefore collect every table that carries the period header
    // and keep the *smallest* one — the genuine factor table (4 rows) rather
    // than the outer wrapper (5 rows, whose first cell is the entire inner
    // table concatenated).
    let mut best: Option<(usize, Vec<Vec<String>>)> = None;
    for table in fragment.select(&table_sel) {
        let trs: Vec<Vec<String>> = table
            .select(&tr_sel)
            .map(|tr| {
                tr.select(&cell_sel)
                    .map(|e| collapse_ws(&e.text().collect::<String>()))
                    .collect()
            })
            .collect();
        let is_factor = trs
            .iter()
            .any(|cells| cells.len() >= 3 && cells.iter().any(|c| c == "Last 3 Months"));
        if is_factor {
            let score = trs.len();
            if best.as_ref().map_or(true, |(s, _)| score < *s) {
                best = Some((score, trs));
            }
        }
    }
    let rows = best
        .map(|(_, trs)| trs)
        .ok_or_else(|| Error::Parse {
            endpoint: SOURCE_FRENCH,
            message: "Fama/French factor table not found".into(),
        })?;

    // The factor groups are exactly the rows whose first cell names a
    // "Fama/French" set. This is robust to stray header/footer rows that the
    // HTML parser may surface (the page has malformed nested markup).
    let groups: Vec<&Vec<String>> = rows
        .iter()
        .filter(|cells| cells.first().map_or(false, |c| c.starts_with("Fama/French")))
        .collect();
    if groups.is_empty() {
        return Err(Error::Parse {
            endpoint: SOURCE_FRENCH,
            message: "factor table has no data rows".into(),
        });
    }
    let mut out = Vec::new();
    for cells in groups {
        let raw = cells.first().cloned().unwrap_or_default();
        // Drop the trailing factor-name tail after the group title.
        let group = raw
            .split("Rm-Rf")
            .next()
            .unwrap_or(&raw)
            .trim()
            .to_string();
        let june_2026 = cells.get(1).cloned().unwrap_or_default();
        let last_3_months = cells.get(2).cloned().unwrap_or_default();
        let last_12_months = cells.get(3).cloned().unwrap_or_default();
        out.push(FfCrrRow {
            group,
            june_2026,
            last_3_months,
            last_12_months,
        });
    }
    Ok(out)
}

/// One daily realized-volatility observation for an individual stock/ETF/future.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RlabRvRow {
    /// Trading date `YYYYMMDD` (akshare index).
    pub date: String,
    /// Annualized realized volatility (akshare `RV`, last field).
    pub rv: Option<f64>,
}

/// Individual-stock realized volatility (`article_rlab_rv`, akshare `article/risk_rv.py:117`).
///
/// Fetches `data.php?ticker=39693` (Dacheng Xiu). The response is a plain-text
/// block of `ticker date v0..vN` lines; we keep the date and the trailing `RV`
/// field (akshare's `iloc[:, 1]`).
pub async fn article_rlab_rv(client: &Client) -> Result<Vec<RlabRvRow>> {
    let url = "https://dachxiu.chicagobooth.edu/data.php?ticker=39693";
    let html = client
        .get_text(SOURCE_RLAB, "article_rlab_rv", url, &[], None)
        .await?;
    parse_article_rlab_rv(&html)
}

/// Parse `article_rlab_rv` from captured HTML.
pub(crate) fn parse_article_rlab_rv(html: &str) -> Result<Vec<RlabRvRow>> {
    let mut out = Vec::new();
    for line in html.split('\n') {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        // Data lines: `ticker date values…` with >=3 tokens, ticker all digits,
        // and an 8-digit `YYYYMMDD` date in the second position.
        if tokens.len() < 3 {
            continue;
        }
        if !tokens[0].bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let date = tokens[1];
        if date.len() != 8 || !date.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let rv = tokens.last().and_then(|v| v.parse::<f64>().ok());
        out.push(RlabRvRow {
            date: date.to_string(),
            rv,
        });
    }
    if out.is_empty() {
        return Err(Error::Parse {
            endpoint: SOURCE_RLAB,
            message: "no realized-volatility data lines found".into(),
        });
    }
    Ok(out)
}

/// Collapse runs of ASCII whitespace (incl. the page's `\t` padding) to a single space.
fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_ws = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_ws {
                out.push(' ');
            }
            prev_ws = true;
        } else {
            out.push(c);
            prev_ws = false;
        }
    }
    out.trim().to_string()
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
    fn parses_article_ff_crr() {
        let rows = parse_article_ff_crr(&fixture("article_ff_crr.html")).unwrap();
        for r in &rows {
            eprintln!("DBG group={:?} june={:?}", r.group, r.june_2026);
        }
        assert!(!rows.is_empty(), "expected >0 rows");
        assert_eq!(rows.len(), 3);
        assert!(rows[0].group.contains("3 Research Factors"));
        // June 2026 column holds the Rm-Rf value `3.58` for the 3-factor set.
        assert!(rows[0].june_2026.contains("3.58"));
        assert!(rows[1].group.contains("5 Research Factors"));
        assert!(rows[2].group.contains("Research Portfolios"));
    }

    #[test]
    fn parses_article_rlab_rv() {
        let rows = parse_article_rlab_rv(&fixture("article_rlab_rv.html")).unwrap();
        assert!(rows.len() > 1000, "expected many observations, got {}", rows.len());
        assert_eq!(rows[0].date, "19960102");
        assert_eq!(rows[0].rv, Some(0.0931612));
        // Spot-check a later row still parses as f64.
        assert!(rows[1].rv.is_some());
    }
}
