use scraper::{Html, Selector};

use crate::core::error::{Error, Result};

/// Shared table walker. `table_sel` lets a caller restrict to a specific table
/// (e.g. `"table.tablesort"`); most callers pass `"table"` via [`tables`].
pub fn tables_with(
    html: &str,
    endpoint: &'static str,
    table_sel: &str,
) -> Result<Vec<Vec<Vec<String>>>> {
    let doc = Html::parse_document(html);
    let table_sel = Selector::parse(table_sel)
        .map_err(|e| Error::Parse {
            endpoint,
            message: format!("table selector: {e}"),
        })?;
    let tr_sel = Selector::parse("tr").unwrap();
    let cell_sel = Selector::parse("td,th").unwrap();
    let mut tables = Vec::new();
    for table in doc.select(&table_sel) {
        let mut rows = Vec::new();
        for tr in table.select(&tr_sel) {
            let cells: Vec<String> = tr
                .select(&cell_sel)
                .map(|c| c.text().collect::<Vec<_>>().join(" ").trim().to_string())
                .collect();
            if !cells.is_empty() {
                rows.push(cells);
            }
        }
        if !rows.is_empty() {
            tables.push(rows);
        }
    }
    if tables.is_empty() {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: "no <table> found".into(),
        });
    }
    Ok(tables)
}

/// Extract every `<table>` from an HTML document as a list of row-to-cell
/// strings: `tables[table][row][cell]`.
///
/// This is the shared table walker that the `*_html_gaps` endpoint modules used
/// to copy-paste privately. Centralizing it here means the row→cell traversal
/// has one place to test and change (locality); per-source field mapping stays
/// in each endpoint module. The first row of each table is treated as the
/// header, matching akshare's `pd.read_html` enumeration closely enough.
pub fn tables(html: &str, endpoint: &'static str) -> Result<Vec<Vec<Vec<String>>>> {
    tables_with(html, endpoint, "table")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_rows_and_cells() {
        let html = r#"
            <table>
              <tr><th>日期</th><th>收盘</th></tr>
              <tr><td>2024-01-02</td><td>10.5</td></tr>
              <tr><td>2024-01-03</td><td>11.0</td></tr>
            </table>"#;
        let got = tables(html, "test").unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].len(), 3);
        assert_eq!(got[0][0], vec!["日期", "收盘"]);
        assert_eq!(got[0][1], vec!["2024-01-02", "10.5"]);
    }

    #[test]
    fn errors_when_no_table() {
        let err = tables("<div>no tables here</div>", "test").unwrap_err();
        match err {
            Error::UpstreamChanged { origin, .. } => assert_eq!(origin, "test"),
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
