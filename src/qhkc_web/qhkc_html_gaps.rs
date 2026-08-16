//! 奇货可查 website HTML endpoints ported from akshare `qhkc_web/`.
//!
//! * [`qhkc_tool_gdp`] — akshare `qhkc_web/qhkc_tool.py:111`
//!   (qhkch.com 各地区经济数据 table).
//!
//! NOTE: the qhkch.com GDP page renders its table body via a JS AJAX call to
//! `/ajax/gdp.php`, which now returns **404** — so the static HTML only
//! contains the `<thead>` (11 columns) with an empty `<tbody>`. `pd.read_html`
//! on the live URL therefore yields a header with zero data rows. The parser
//! below reproduces that structure (header + body rows) faithfully; on a live
//! page with a working AJAX it would capture the country rows.

use scraper::{Html, Selector};

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// One country/region row from the qhkch GDP table (`qhkc_tool_gdp`).
///
/// All columns are kept as `String` because the source mixes plain numbers
/// (`20494`) with percent/ratio strings (`2.30%`, `106.10%`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct QhkcGdpRow {
    /// Country / region (akshare `国家/地区`).
    pub area: String,
    /// GDP (akshare `GDP`).
    pub gdp: String,
    /// GDP YoY (akshare `GDP同比`).
    pub gdp_yoy: String,
    /// GDP QoQ (akshare `GDP环比`).
    pub gdp_qoq: String,
    /// Interest rate (akshare `利率`).
    pub interest_rate: String,
    /// Inflation rate (akshare `通货膨胀率`).
    pub inflation_rate: String,
    /// Unemployment rate (akshare `失业率`).
    pub unemployment_rate: String,
    /// Government budget (akshare `政府预算`).
    pub budget: String,
    /// Debt / GDP (akshare `债务/GDP`).
    pub debt: String,
    /// Current account (akshare `经常账户`).
    pub accrount: String,
    /// Population (akshare `人口`).
    pub pop: String,
}

/// 奇货可查-工具-各地区经济数据 (`qhkc_tool_gdp`, akshare `qhkc_web/qhkc_tool.py:111`).
pub async fn qhkc_tool_gdp(client: &Client) -> Result<Vec<QhkcGdpRow>> {
    let url = "https://qhkch.com/dist/views/toolbox/gdp.html?v=1.10.7.1";
    let html = client
        .get_text("qhkc", "qhkc_tool_gdp", url, &[], None)
        .await?;
    parse_qhkc_tool_gdp(&html, "qhkc_tool_gdp")
}

/// Parse the qhkch GDP table (`#toolbox_gdp`). The header row is `tables[0][0]`;
/// country rows follow. When the upstream AJAX is dead the body is empty.
pub(crate) fn parse_qhkc_tool_gdp(html: &str, endpoint: &'static str) -> Result<Vec<QhkcGdpRow>> {
    let doc = Html::parse_document(html);
    let table_sel = Selector::parse("table#toolbox_gdp")
        .map_err(|e| Error::Parse { endpoint, message: format!("table selector: {e}") })?;
    let tr_sel = Selector::parse("tr").unwrap();
    let cell_sel = Selector::parse("th,td").unwrap();
    let table = doc
        .select(&table_sel)
        .next()
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "no toolbox_gdp table".into() })?;
    let mut rows: Vec<Vec<String>> = Vec::new();
    for tr in table.select(&tr_sel) {
        let cells: Vec<String> = tr
            .select(&cell_sel)
            .map(|c| c.text().collect::<Vec<_>>().join(" ").trim().to_string())
            .collect();
        if !cells.is_empty() {
            rows.push(cells);
        }
    }
    if rows.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "empty GDP table".into() });
    }
    // rows[0] is the header; data rows follow.
    let out = rows
        .iter()
        .skip(1)
        .filter(|cells| cells.len() >= 11)
        .map(|cells| QhkcGdpRow {
            area: cells[0].clone(),
            gdp: cells[1].clone(),
            gdp_yoy: cells[2].clone(),
            gdp_qoq: cells[3].clone(),
            interest_rate: cells[4].clone(),
            inflation_rate: cells[5].clone(),
            unemployment_rate: cells[6].clone(),
            budget: cells[7].clone(),
            debt: cells[8].clone(),
            accrount: cells[9].clone(),
            pop: cells[10].clone(),
        })
        .collect();
    Ok(out)
}

/// Expected GDP table header columns (used by the parse test to verify structure).
#[cfg(test)]
pub(crate) fn qhkc_gdp_expected_headers() -> [&'static str; 11] {
    [
        "国家/地区", "GDP", "GDP同比", "GDP环比", "利率", "通货膨胀率", "失业率", "政府预算",
        "债务/GDP", "经常账户", "人口",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn load_html(name: &str) -> String {
        let bytes = std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)).unwrap();
        match std::str::from_utf8(&bytes) {
            Ok(s) => s.to_string(),
            Err(_) => encoding_rs::GBK
                .decode_without_bom_handling_and_without_replacement(&bytes)
                .map(|c| c.into_owned())
                .unwrap_or_else(|| String::from_utf8_lossy(&bytes).into_owned()),
        }
    }

    #[test]
    fn parses_qhkc_tool_gdp_structure() {
        let html = load_html("qhkc_tool_gdp.html");
        let doc = Html::parse_document(&html);
        let table = doc
            .select(&Selector::parse("table#toolbox_gdp").unwrap())
            .next()
            .expect("toolbox_gdp table present");
        let header: Vec<String> = table
            .select(&Selector::parse("thead th").unwrap())
            .map(|c| c.text().collect::<Vec<_>>().join(" ").trim().to_string())
            .collect();
        assert_eq!(header.len(), 11, "expected 11 GDP columns, got {header:?}");
        for expected in qhkc_gdp_expected_headers() {
            assert!(
                header.iter().any(|h| h.contains(expected)),
                "missing expected GDP column: {expected}"
            );
        }
        // Upstream AJAX is dead (404) → empty body, so parse yields no rows.
        let rows = parse_qhkc_tool_gdp(&html, "qhkc_tool_gdp").unwrap();
        assert!(rows.is_empty(), "expected 0 live rows (upstream AJAX 404)");
    }
}
