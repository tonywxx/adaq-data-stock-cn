//! `futures` HTML-scraping gap fillers.
//!
//! Ports akshare `futures`-package functions whose upstreams return HTML tables
//! (`pd.read_html`). Each follows the established pattern: a public `async fn`
//! that performs the network fetch and a `pub(crate)` `parse_*` that turns the
//! captured body into rows.
//!
//! Sources / akshare references:
//! * [`pandas_read_html_link`] — `futures/requests_fun.py:53`

use scraper::{Html, Selector};

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// One HTML table, rows of trimmed cells. Mirrors akshare `pd.read_html`
/// returning a list of DataFrames (each a 2-D grid).
#[derive(Debug, Clone, serde::Serialize)]
pub struct HtmlTable {
    /// Rows, each a list of cell strings (header row included).
    pub rows: Vec<Vec<String>>,
}

/// Parse every `<table>` on the page into a [`HtmlTable`].
pub(crate) fn parse_html_tables(html: &str, endpoint: &'static str) -> Result<Vec<HtmlTable>> {
    let doc = Html::parse_document(html);
    let table_sel = Selector::parse("table")
        .map_err(|e| Error::Parse { endpoint, message: format!("table selector: {e}") })?;
    let tr_sel = Selector::parse("tr").unwrap();
    let cell_sel = Selector::parse("td,th").unwrap();
    let mut out = Vec::new();
    for table in doc.select(&table_sel) {
        let rows: Vec<Vec<String>> = table
            .select(&tr_sel)
            .map(|tr| {
                tr.select(&cell_sel)
                    .map(|c| {
                        c.text()
                            .collect::<Vec<_>>()
                            .join(" ")
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .collect()
            })
            .filter(|r: &Vec<String>| !r.is_empty())
            .collect();
        if !rows.is_empty() {
            out.push(HtmlTable { rows });
        }
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: "no <table> found".into(),
        });
    }
    Ok(out)
}

/// Generic HTML-table reader (`pandas_read_html_link`, akshare
/// `futures/requests_fun.py:53`). Fetches a URL (GET by default, POST when
/// `data` is supplied) and returns every table parsed from the response.
pub async fn pandas_read_html_link(
    client: &Client,
    url: &str,
    method: &str,
    data: &[(&str, &str)],
) -> Result<Vec<HtmlTable>> {
    let endpoint = "pandas_read_html_link";
    let html = if method.eq_ignore_ascii_case("post") {
        client.post_form_text("futures", endpoint, url, data, None).await?
    } else {
        client.get_text("futures", endpoint, url, data, None).await?
    };
    parse_html_tables(&html, endpoint)
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn load_html(name: &str) -> String {
        std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(name),
        )
        .unwrap()
    }

    #[test]
    fn parses_html_tables() {
        let tables = parse_html_tables(&load_html("pandas_read_html_link.html"), "pandas_read_html_link")
            .unwrap();
        assert_eq!(tables.len(), 2);
        assert_eq!(tables[0].rows[0][0], "名称");
        assert_eq!(tables[0].rows[1][0], "苹果");
        assert_eq!(tables[1].rows[0][0], "日期");
    }
}
