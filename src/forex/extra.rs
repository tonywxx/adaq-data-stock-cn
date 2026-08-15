//! Extra currency / foreign-exchange endpoints.
//!
//! This module ports a batch of akshare currency & forex functions that are
//! pure-HTTP (no JS rendering, no encryption) and that are not already covered
//! by `src/forex/eastmoney.rs` (Eastmoney spot/history) or `src/alt/fx.rs`
//! (ChinaMoney spot/pair quotes):
//!
//! - `currency_boc` — Bank of China current FX spot rates (HTML table, manual parse).
//!   Port of the akshare `currency_boc` intent (BOC `sourcedb/whpj` table).
//! - `currency_hist` — Bank of China historical RMB price per currency, via Sina
//!   (HTML table, manual parse). Port of akshare `currency_boc_sina`.
//! - `fx_swap_quote` — ChinaMoney RMB FX swap-point quotes (POST/JSON).
//!   Port of akshare `fx_swap_quote` (`fx/fx_quote.py`).
//!
//! HTML responses are parsed with a small dependency-free table extractor
//! (no `scraper`/`beautifulsoup` needed) so no new crates are required.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_BOC: &str = "boc";
const SOURCE_SINA: &str = "sina";
const SOURCE_CHINAMONEY: &str = "chinamoney";

const BOC_SPOT_URL: &str = "https://www.boc.cn/sourcedb/whpj/index.html";
const SINA_FX_URL: &str = "http://biz.finance.sina.com.cn/forex/forex.php";
const CHINAMONEY_FX_SWAP_URL: &str =
    "http://www.chinamoney.com.cn/r/cms/www/chinamoney/data/fx/rfx-sw-quot.json";

const BOC_HEADERS: &[(&str, &str)] = &[(
    "User-Agent",
    "Mozilla/5.0 (compatible; adaq-data-stock-cn/0.1)",
)];
const CHINAMONEY_HEADERS: &[(&str, &str)] = &[(
    "User-Agent",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
     Chrome/61.0.3163.91 Safari/537.36",
)];

/// Current epoch time in milliseconds (the `t` form param ChinaMoney expects).
fn now_ms() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

// ---------------------------------------------------------------------------
// currency_boc — Bank of China current FX spot rates (HTML table)
// ---------------------------------------------------------------------------

/// Bank of China current foreign-exchange spot rate for one currency.
///
/// Mirrors the akshare `currency_boc` columns from the BOC `sourcedb/whpj` table:
/// 货币名称 / 现汇买入价 / 现钞买入价 / 现汇卖出价 / 现钞卖出价 / 中行折算价 / 发布时间.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BocSpotRow {
    /// 货币名称 — currency name (e.g. 英镑).
    pub currency_name: String,
    /// 现汇买入价 — BOC spot (wire) buying rate.
    pub spot_buy: Option<f64>,
    /// 现钞买入价 — BOC cash buying rate.
    pub cash_buy: Option<f64>,
    /// 现汇卖出价 — BOC spot selling rate.
    pub spot_sell: Option<f64>,
    /// 现钞卖出价 — BOC cash selling rate.
    pub cash_sell: Option<f64>,
    /// 中行折算价 — BOC conversion/reference rate.
    pub mid_rate: Option<f64>,
    /// 发布时间 — publish time of the quote.
    pub publish_time: String,
    /// Data source identifier.
    pub source: &'static str,
}

/// Bank of China current FX spot rates (`currency_boc`).
///
/// Fetches the BOC foreign-exchange rate table and parses it with a small
/// dependency-free HTML table extractor (no `scraper` needed).
pub async fn currency_boc(client: &Client) -> Result<Vec<BocSpotRow>> {
    let text = client
        .get_text(SOURCE_BOC, "currency_boc", BOC_SPOT_URL, &[], Some(BOC_HEADERS))
        .await?;
    parse_boc_spot(&text)
}

/// Parse a Bank of China spot-rate HTML page into [`BocSpotRow`]s.
pub(crate) fn parse_boc_spot(html: &str) -> Result<Vec<BocSpotRow>> {
    let rows = parse_table_rows(html);
    let mut out = Vec::new();
    for cells in rows {
        if cells.len() < 7 {
            continue; // header or malformed
        }
        if cells[0].contains("货币名称") {
            continue; // header row
        }
        out.push(BocSpotRow {
            currency_name: cells[0].clone(),
            spot_buy: parse_rate(&cells[1]),
            cash_buy: parse_rate(&cells[2]),
            spot_sell: parse_rate(&cells[3]),
            cash_sell: parse_rate(&cells[4]),
            mid_rate: parse_rate(&cells[5]),
            publish_time: cells.get(6).cloned().unwrap_or_default(),
            source: SOURCE_BOC,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// currency_hist — Bank of China historical RMB price per currency (Sina, HTML)
// ---------------------------------------------------------------------------

/// Bank of China historical RMB price for one currency on one day.
///
/// Mirrors the akshare `currency_boc_sina` columns:
/// 日期 / 中行汇买价 / 中行钞买价 / 中行钞卖价/汇卖价 / 央行中间价 / 中行折算价.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BocHistRow {
    /// 日期 — date of the quote (YYYY-MM-DD).
    pub date: String,
    /// 中行汇买价 — BOC spot buying rate.
    pub spot_buy: Option<f64>,
    /// 中行钞买价 — BOC cash buying rate.
    pub cash_buy: Option<f64>,
    /// 中行钞卖价/汇卖价 — BOC spot/cash selling rate.
    pub spot_sell: Option<f64>,
    /// 央行中间价 — PBOC central parity rate.
    pub central_parity: Option<f64>,
    /// 中行折算价 — BOC conversion rate.
    pub conversion_rate: Option<f64>,
    /// Data source identifier.
    pub source: &'static str,
}

/// Bank of China historical RMB price per currency (`currency_hist`).
///
/// `code` is the Sina `money_code` (e.g. `"USD"`, `"EUR"`, `"JPY"`).
/// `start_date` / `end_date` are `YYYYMMDD` (e.g. `"20230304"`).
/// The Sina page returns an HTML table parsed with a dependency-free extractor.
pub async fn currency_hist(
    client: &Client,
    code: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<BocHistRow>> {
    if !is_ymd(start_date) || !is_ymd(end_date) {
        return Err(Error::InvalidParam(
            "start_date and end_date must be in YYYYMMDD format".into(),
        ));
    }
    let sd = format!(
        "{}-{}-{}",
        &start_date[0..4], &start_date[4..6], &start_date[6..8]
    );
    let ed = format!(
        "{}-{}-{}",
        &end_date[0..4], &end_date[4..6], &end_date[6..8]
    );
    let params = [
        ("money_code", code),
        ("type", "0"),
        ("startdate", sd.as_str()),
        ("enddate", ed.as_str()),
        ("page", "1"),
        ("call_type", "ajax"),
    ];
    let text = client
        .get_text(SOURCE_SINA, "currency_hist", SINA_FX_URL, &params, None)
        .await?;
    parse_boc_hist(&text)
}

/// Parse a Sina BOC historical-price HTML table into [`BocHistRow`]s.
pub(crate) fn parse_boc_hist(html: &str) -> Result<Vec<BocHistRow>> {
    let rows = parse_table_rows(html);
    let mut out = Vec::new();
    for cells in rows {
        if cells.len() < 6 {
            continue; // header or malformed
        }
        if cells[0].contains("日期") {
            continue; // header row
        }
        out.push(BocHistRow {
            date: cells[0].clone(),
            spot_buy: parse_rate(&cells[1]),
            cash_buy: parse_rate(&cells[2]),
            spot_sell: parse_rate(&cells[3]),
            central_parity: parse_rate(&cells[4]),
            conversion_rate: parse_rate(&cells[5]),
            source: SOURCE_SINA,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// fx_swap_quote — ChinaMoney RMB FX swap-point quotes (POST/JSON)
// ---------------------------------------------------------------------------

/// ChinaMoney RMB FX swap-point quote for one currency pair.
///
/// Mirrors the akshare `fx_swap_quote` columns:
/// 货币对 / 1周 / 1月 / 3月 / 6月 / 9月 / 1年.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FxSwapQuoteRow {
    /// 货币对 — currency pair code (e.g. USD/CNY).
    pub ccy_pair: String,
    /// 1周 — 1-week swap points.
    pub swap_1w: Option<f64>,
    /// 1月 — 1-month swap points.
    pub swap_1m: Option<f64>,
    /// 3月 — 3-month swap points.
    pub swap_3m: Option<f64>,
    /// 6月 — 6-month swap points.
    pub swap_6m: Option<f64>,
    /// 9月 — 9-month swap points.
    pub swap_9m: Option<f64>,
    /// 1年 — 1-year swap points.
    pub swap_1y: Option<f64>,
    /// Data source identifier.
    pub source: &'static str,
}

/// ChinaMoney RMB FX swap-point quotes (`fx_swap_quote`).
///
/// POSTs a `t` (epoch-millis) form param to the ChinaMoney FX swap endpoint and
/// parses the `records` array.
pub async fn fx_swap_quote(client: &Client) -> Result<Vec<FxSwapQuoteRow>> {
    let t = now_ms();
    let params: [(&str, &str); 1] = [("t", &t)];
    let v = client
        .post_form_json(
            SOURCE_CHINAMONEY,
            "fx_swap_quote",
            CHINAMONEY_FX_SWAP_URL,
            &params,
            Some(CHINAMONEY_HEADERS),
        )
        .await?;
    parse_fx_swap_quote(&v)
}

/// Parse a ChinaMoney FX swap JSON response into [`FxSwapQuoteRow`]s.
pub(crate) fn parse_fx_swap_quote(resp: &Value) -> Result<Vec<FxSwapQuoteRow>> {
    let records = resp
        .get("records")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CHINAMONEY,
            message: "missing records".into(),
        })?;
    let mut out = Vec::with_capacity(records.len());
    for r in records {
        out.push(FxSwapQuoteRow {
            ccy_pair: fstr(r, "ccyPair"),
            swap_1w: fnum(r, "label_1W"),
            swap_1m: fnum(r, "label_1M"),
            swap_3m: fnum(r, "label_3M"),
            swap_6m: fnum(r, "label_6M"),
            swap_9m: fnum(r, "label_9M"),
            swap_1y: fnum(r, "label_1Y"),
            source: SOURCE_CHINAMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// HTML table extraction (dependency-free)
// ---------------------------------------------------------------------------

/// Split an HTML document into table rows, each row a list of cell texts
/// (both `<td>` and `<th>` cells, in document order).
fn parse_table_rows(html: &str) -> Vec<Vec<String>> {
    let lower = html.to_ascii_lowercase();
    let mut rows = Vec::new();
    let mut i = 0;
    while let Some(tr_rel) = lower[i..].find("<tr") {
        let abs = i + tr_rel;
        let after_tag = match lower[abs..].find('>') {
            Some(p) => abs + p + 1,
            None => break,
        };
        let end_rel = match lower[after_tag..].find("</tr>") {
            Some(p) => after_tag + p,
            None => break,
        };
        let row_html = &html[after_tag..end_rel];
        rows.push(extract_cells(row_html));
        i = end_rel + 5; // skip past "</tr>"
    }
    rows
}

/// Extract cell texts from a single `<tr>...</tr>` fragment.
fn extract_cells(row_html: &str) -> Vec<String> {
    let lower = row_html.to_ascii_lowercase();
    let mut cells = Vec::new();
    let mut i = 0;
    while i < row_html.len() {
        let rest_lower = &lower[i..];
        if rest_lower.starts_with("<td") || rest_lower.starts_with("<th") {
            let tag_open = match row_html[i..].find('>') {
                Some(p) => i + p,
                None => break,
            };
            let content_start = tag_open + 1;
            let rest = &lower[content_start..];
            let close_td = rest.find("</td>");
            let close_th = rest.find("</th>");
            let (close_rel, close_len) = match (close_td, close_th) {
                (Some(a), Some(b)) if a <= b => (a, 5),
                (Some(_), Some(b)) => (b, 5),
                (Some(a), None) => (a, 5),
                (None, Some(b)) => (b, 5),
                (None, None) => break,
            };
            let content_end = content_start + close_rel;
            cells.push(decode_cell(&row_html[content_start..content_end]));
            i = content_end + close_len;
        } else {
            i += 1;
        }
    }
    cells
}

/// Decode a single table-cell's inner HTML into plain text: strip tags,
/// normalize whitespace, and decode the few HTML entities we expect.
fn decode_cell(s: &str) -> String {
    let with_br = s.replace("<br>", " ").replace("<br/>", " ").replace("<br />", " ");
    let stripped = strip_tags(&with_br);
    let decoded = stripped
        .replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    decoded
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

/// Remove all `<...>` tags from a string.
fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// Parse a numeric cell, treating empty / `-` / `--` as `None`.
fn parse_rate(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() || s == "-" || s == "--" {
        return None;
    }
    s.replace(',', "").parse::<f64>().ok()
}

/// Validate a `YYYYMMDD` date string.
fn is_ymd(s: &str) -> bool {
    s.len() == 8 && s.bytes().all(|b| b.is_ascii_digit())
}

// ---------------------------------------------------------------------------
// small JSON helpers
// ---------------------------------------------------------------------------

fn fstr(v: &Value, k: &str) -> String {
    v.get(k)
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string()
}

fn fnum(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(|x| match x {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    })
}

// ---------------------------------------------------------------------------
// offline tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_text(name: &str) -> String {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        std::fs::read_to_string(path).unwrap()
    }

    fn fixture_json(name: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let txt = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    #[test]
    fn parses_currency_boc_fixture() {
        let html = fixture_text("currency_boc.html");
        let rows = parse_boc_spot(&html).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].currency_name, "英镑");
        assert_eq!(rows[0].spot_buy, Some(914.52));
        assert_eq!(rows[0].cash_buy, Some(885.64));
        assert_eq!(rows[0].spot_sell, Some(921.24));
        assert_eq!(rows[0].cash_sell, Some(921.24));
        assert_eq!(rows[0].mid_rate, Some(914.93));
        assert_eq!(rows[0].publish_time, "2024-01-02 10:30:00");
        assert_eq!(rows[0].source, "boc");
        assert_eq!(rows[2].currency_name, "欧元");
        assert_eq!(rows[2].spot_buy, Some(783.45));
    }

    #[test]
    fn parses_currency_hist_fixture() {
        let html = fixture_text("currency_hist.html");
        let rows = parse_boc_hist(&html).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2023-03-06");
        assert_eq!(rows[0].spot_buy, Some(687.61));
        assert_eq!(rows[0].cash_buy, Some(681.95));
        assert_eq!(rows[0].spot_sell, Some(690.53));
        assert_eq!(rows[0].central_parity, Some(689.12));
        assert_eq!(rows[0].conversion_rate, Some(689.12));
        assert_eq!(rows[0].source, "sina");
        assert_eq!(rows[1].date, "2023-03-07");
        assert_eq!(rows[1].spot_buy, Some(688.20));
    }

    #[test]
    fn parses_fx_swap_quote_fixture() {
        let v = fixture_json("fx_swap_quote.json");
        let rows = parse_fx_swap_quote(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].ccy_pair, "USD/CNY");
        assert_eq!(rows[0].swap_1w, Some(12.5));
        assert_eq!(rows[0].swap_1m, Some(50.3));
        assert_eq!(rows[0].swap_3m, Some(145.2));
        assert_eq!(rows[0].swap_6m, Some(280.4));
        assert_eq!(rows[0].swap_9m, Some(410.1));
        assert_eq!(rows[0].swap_1y, Some(535.6));
        assert_eq!(rows[0].source, "chinamoney");
        assert_eq!(rows[1].ccy_pair, "EUR/CNY");
        assert_eq!(rows[1].swap_1y, Some(658.2));
    }

    #[test]
    fn parses_dash_and_entities_as_none() {
        // Synthetic table: header row skipped, a "-" cell becomes None,
        // `&nbsp;` collapses to a space, and a nested tag is stripped.
        let html = "\
<table>\
<tr><th>货币名称</th><th>现汇买入价</th><th>现钞买入价</th><th>现汇卖出价</th><th>现钞卖出价</th><th>中行折算价</th><th>发布时间</th></tr>\
<tr><td>美元</td><td>-</td><td>-</td><td>-</td><td>-</td><td>716.83&nbsp;</td><td>2024-01-02</td></tr>\
<tr><td>欧元</td><td><span>783.45</span></td><td>-</td><td>-</td><td>-</td><td>784.20</td><td>2024-01-02</td></tr>\
</table>";
        let rows = parse_boc_spot(html).unwrap();
        assert_eq!(rows.len(), 2);
        // header skipped, "-" -> None
        assert_eq!(rows[0].currency_name, "美元");
        assert_eq!(rows[0].spot_buy, None);
        assert_eq!(rows[0].mid_rate, Some(716.83));
        // nested <span> stripped
        assert_eq!(rows[1].spot_buy, Some(783.45));
        assert_eq!(rows[1].mid_rate, Some(784.20));
    }

    #[test]
    fn is_ymd_validates_format() {
        assert!(is_ymd("20230304"));
        assert!(!is_ymd("2023-03-04"));
        assert!(!is_ymd("202304"));
        assert!(!is_ymd("2023abcd"));
    }
}
