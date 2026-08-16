//! HTML-scraping ports for the `stock` domain that akshare exposes via
//! `pd.read_html` / BeautifulSoup. Pure-Rust reimplementation (no JS engine).

use scraper::{Html, Selector};
use std::collections::HashMap;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

/// Extract every `<table>` in the document as `Vec<Vec<Vec<String>>>`
/// (table → row → cells, text joined). Skips empty rows.
fn extract_tables(html: &str, endpoint: &'static str) -> Result<Vec<Vec<Vec<String>>>> {
    let doc = Html::parse_document(html);
    let table_sel = Selector::parse("table")
        .map_err(|e| Error::Parse { endpoint, message: format!("table selector: {e}") })?;
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
            message: "no <table> found in HTML".into(),
        });
    }
    Ok(tables)
}

/// Parse a numeric cell, tolerating thousands separators / surrounding spaces.
fn num(s: &str) -> Option<f64> {
    s.replace(',', "").trim().parse::<f64>().ok()
}

#[cfg(test)]
fn load_html(name: &str) -> String {
    use std::path::PathBuf;
    let bytes =
        std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name))
            .unwrap();
    match std::str::from_utf8(&bytes) {
        Ok(s) => s.to_string(),
        Err(_) => match encoding_rs::GBK.decode_without_bom_handling_and_without_replacement(&bytes)
        {
            Some(cow) => cow.into_owned(),
            None => String::from_utf8_lossy(&bytes).into_owned(),
        },
    }
}

// ---------------------------------------------------------------------------
// stock_hk_fhpx_detail_ths — 同花顺港股分红派息 (THS bonus HTML table)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct HkFhpxRow {
    pub announce_date: Option<String>,
    pub plan: String,
    pub ex_dividend_date: Option<String>,
    pub pay_date: Option<String>,
    pub transfer_start: Option<String>,
    pub transfer_end: Option<String>,
    pub type_: String,
    pub progress: String,
    pub stock_dividend: String,
}

/// Original akshare `stock_hk_fhpx_detail_ths(symbol="0700")`.
pub async fn stock_hk_fhpx_detail_ths(
    client: &Client,
    symbol: &str,
) -> Result<Vec<HkFhpxRow>> {
    let url = format!("https://basic.10jqka.com.cn/176/HK{symbol}/bonus.html");
    let html = client
        .get_text("ths", "stock_hk_fhpx_detail_ths", &url, &[], None)
        .await?;
    parse_stock_hk_fhpx_detail_ths(&html, "stock_hk_fhpx_detail_ths")
}

pub(crate) fn parse_stock_hk_fhpx_detail_ths(
    html: &str,
    endpoint: &'static str,
) -> Result<Vec<HkFhpxRow>> {
    let tables = extract_tables(html, endpoint)?;
    // table whose header row contains 公告日期
    let table = tables
        .iter()
        .find(|t| t.first().map_or(false, |h| h.iter().any(|c| c.contains("公告日期"))))
        .ok_or_else(|| Error::UpstreamChanged {
            origin: endpoint,
            message: "bonus table not found".into(),
        })?;
    let mut out = Vec::new();
    for row in table.iter().skip(1) {
        if row.len() < 9 {
            continue;
        }
        let ex = row[2].trim();
        let pay = row[3].trim();
        // akshare drops rows without 除净日 / 派息日
        if ex.is_empty() || pay.is_empty() {
            continue;
        }
        out.push(HkFhpxRow {
            announce_date: Some(row[0].trim().to_string()).filter(|s| !s.is_empty()),
            plan: row[1].trim().to_string(),
            ex_dividend_date: Some(ex.to_string()),
            pay_date: Some(pay.to_string()),
            transfer_start: Some(row[4].trim().to_string()).filter(|s| !s.is_empty()),
            transfer_end: Some(row[5].trim().to_string()).filter(|s| !s.is_empty()),
            type_: row[6].trim().to_string(),
            progress: row[7].trim().to_string(),
            stock_dividend: row[8].trim().to_string(),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: "no bonus rows parsed".into(),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// stock_info_change_name — 新浪财经股票曾用名 (sina company-info table[3])
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct NameChangeRow {
    pub index: usize,
    pub name: String,
}

/// Original akshare `stock_info_change_name(symbol="000503")`.
pub async fn stock_info_change_name(client: &Client, symbol: &str) -> Result<Vec<NameChangeRow>> {
    let url = format!(
        "https://vip.stock.finance.sina.com.cn/corp/go.php/vCI_CorpInfo/stockid/{symbol}.phtml"
    );
    let html = client
        .get_text("sina", "stock_info_change_name", &url, &[], None)
        .await?;
    parse_stock_info_change_name(&html, "stock_info_change_name")
}

pub(crate) fn parse_stock_info_change_name(
    html: &str,
    endpoint: &'static str,
) -> Result<Vec<NameChangeRow>> {
    let tables = extract_tables(html, endpoint)?;
    // the company-info table (akshare reads table index 3) contains 证券简称更名历史
    let table = tables
        .iter()
        .find(|t| {
            t.iter()
                .any(|r| r.first().map_or(false, |c| c.contains("证券简称更名历史")))
        })
        .ok_or_else(|| Error::UpstreamChanged {
            origin: endpoint,
            message: "company-info table not found".into(),
        })?;
    // locate the row whose first cell is the 更名历史 label, take 2nd cell
    let value = table
        .iter()
        .find(|r| r.first().map_or(false, |c| c.contains("证券简称更名历史")))
        .and_then(|r| r.get(1))
        .map(|s| s.trim().to_string())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: endpoint,
            message: "name-history cell not found".into(),
        })?;
    let names: Vec<&str> = value.split_whitespace().collect();
    if names.is_empty() {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: "no historical names".into(),
        });
    }
    Ok(names
        .iter()
        .enumerate()
        .map(|(i, n)| NameChangeRow {
            index: i + 1,
            name: n.to_string(),
        })
        .collect())
}

// ---------------------------------------------------------------------------
// stock_szse_sector_summary — 深交所统计月报股票行业成交 (script→xls link→HTML)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct SzseSectorRow {
    pub name: String,
    pub name_en: String,
    pub trading_days: Option<f64>,
    pub turnover_value: Option<f64>,
    pub turnover_value_pct: Option<f64>,
    pub turnover_volume: Option<f64>,
    pub turnover_volume_pct: Option<f64>,
    pub deals: Option<f64>,
    pub deals_pct: Option<f64>,
}

/// Original akshare `stock_szse_sector_summary(symbol="当月", date="202501")`.
/// Resolves the month report via the inline JS date→url map, follows the
/// "股票行业成交数据" xls link, and parses the (gbk) HTML table.
pub async fn stock_szse_sector_summary(
    client: &Client,
    symbol: &str,
    date: &str,
) -> Result<Vec<SzseSectorRow>> {
    let index_url = "https://www.szse.cn/market/periodical/month/index.html";
    let index_html = client
        .get_text("szse", "stock_szse_sector_summary", index_url, &[], None)
        .await?;
    let map = parse_szse_date_map(&index_html, "stock_szse_sector_summary")?;
    let key = if map.contains_key(date) {
        date.to_string()
    } else {
        // akshare raises when the date is unavailable; fall back to the latest.
        map.keys().next().cloned().unwrap_or_default()
    };
    let value = map.get(&key).ok_or_else(|| Error::UpstreamChanged {
        origin: "stock_szse_sector_summary",
        message: "no month report url".into(),
    })?;
    // value looks like "./t20260806_622028.html" → strip leading "./"
    let rel = value.trim_start_matches("./");
    let month_url = format!("https://www.szse.cn/market/periodical/month/{rel}");
    let month_html = client
        .get_text("szse", "stock_szse_sector_summary", &month_url, &[], None)
        .await?;
    let xls_url = find_szse_xls_link(&month_html, "stock_szse_sector_summary")?;
    let xls_html = client
        .get_text("szse", "stock_szse_sector_summary", &xls_url, &[], None)
        .await?;
    parse_stock_szse_sector_summary(&xls_html, symbol, "stock_szse_sector_summary")
}

/// Parse the inline `{ value:'...', text:'YYYY-MM' }` JS objects from the index
/// page into a date-label → relative-url map.
fn parse_szse_date_map(html: &str, endpoint: &'static str) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    let mut i = 0;
    while let Some(start) = html[i..].find('{') {
        let abs = i + start;
        let Some(end_rel) = html[abs..].find('}') else { break };
        let end = abs + end_rel;
        let obj = &html[abs..=end];
        if obj.contains("value") && obj.contains("text") {
            if let (Some(v), Some(t)) = (kv(obj, "value"), kv(obj, "text")) {
                map.insert(t, v);
            }
        }
        i = end + 1;
    }
    if map.is_empty() {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: "no date→url map in szse index".into(),
        });
    }
    Ok(map)
}

/// Extract `key:'...'` from a minimal JS object literal.
fn kv(obj: &str, key: &str) -> Option<String> {
    let pat = format!("{key}:");
    let idx = obj.find(&pat)?;
    let rest = obj[idx + pat.len()..].trim_start();
    if rest.starts_with('\'') {
        let close = rest[1..].find('\'')?;
        Some(rest[1..1 + close].to_string())
    } else {
        None
    }
}

/// Find the href of the `<a>` whose text is "股票行业成交数据".
fn find_szse_xls_link(html: &str, endpoint: &'static str) -> Result<String> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("a")
        .map_err(|e| Error::Parse { endpoint, message: format!("a selector: {e}") })?;
    for a in doc.select(&sel) {
        let text: String = a.text().collect::<String>();
        if text.contains("股票行业成交数据") {
            if let Some(href) = a.value().attr("href") {
                return Ok(href.to_string());
            }
        }
    }
    Err(Error::UpstreamChanged {
        origin: endpoint,
        message: "xls link not found".into(),
    })
}

pub(crate) fn parse_stock_szse_sector_summary(
    html: &str,
    symbol: &str,
    endpoint: &'static str,
) -> Result<Vec<SzseSectorRow>> {
    let tables = extract_tables(html, endpoint)?;
    // akshare reads table[0] for "当月" and table[1] for "当年".
    let matches: Vec<&Vec<Vec<String>>> = tables
        .iter()
        .filter(|t| t.iter().any(|r| r.iter().any(|c| c.contains("成交金额"))))
        .collect();
    let table = if symbol == "当年" {
        matches.get(1)
    } else {
        matches.first()
    }
    .ok_or_else(|| Error::UpstreamChanged {
        origin: endpoint,
        message: "sector table not found".into(),
    })?;
    // data rows have 9 cells (header rows have 5 / 8); skip non-data rows.
    let mut out = Vec::new();
    for row in table.iter() {
        if row.len() != 9 {
            continue;
        }
        let first = row[0].trim();
        if first.is_empty() || first.contains("2026年") || first.contains("July") {
            continue;
        }
        out.push(SzseSectorRow {
            name: first.to_string(),
            name_en: row[1].trim().to_string(),
            trading_days: num(&row[2]),
            turnover_value: num(&row[3]),
            turnover_value_pct: num(&row[4]),
            turnover_volume: num(&row[5]),
            turnover_volume_pct: num(&row[6]),
            deals: num(&row[7]),
            deals_pct: num(&row[8]),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: "no sector rows parsed".into(),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stock_hk_fhpx_detail_ths() {
        let rows =
            parse_stock_hk_fhpx_detail_ths(&load_html("stock_hk_fhpx_detail_ths.html"), "x")
                .unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].pay_date.is_some());
    }

    #[test]
    fn parses_stock_info_change_name() {
        let rows = parse_stock_info_change_name(&load_html("stock_info_change_name.html"), "x")
            .unwrap();
        assert!(!rows.is_empty());
        // 000503 history ends with 国新健康
        assert_eq!(rows.last().unwrap().name, "国新健康");
    }

    #[test]
    fn parses_stock_szse_sector_summary() {
        let rows =
            parse_stock_szse_sector_summary(&load_html("stock_szse_sector_summary.xls"), "当月", "x")
                .unwrap();
        assert!(rows.len() >= 10);
        // 合计 / Total is the first data row
        assert_eq!(rows[0].name, "合计");
        assert!(rows[0].turnover_value.is_some());
    }
}
