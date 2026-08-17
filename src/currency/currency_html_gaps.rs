//! Currency / FX HTML-table endpoints — akshare `currency/*`.
//!
//! * [`currency_boc_safe`] — SAFE RMB central parity rates
//!   (`currency/currency_safe.py:18`); the live `RMBQuery.do` POST returns an
//!   HTML `<table>` (akshare `pd.read_html(...)[-1]`).
//! * [`currency_boc_sina`] — BOC (Bank of China) CNY quoted rates history
//!   (`currency/currency_china_bank_sina.py:57`); each page is an HTML
//!   `<table>` (akshare `pd.read_html(..., header=0)[0]`).

use scraper::{Html, Selector};
use std::collections::HashMap;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_SAFE: &str = "safe";
const SOURCE_SINA: &str = "sina";

/// Extract every `<table>` from an HTML document as a matrix of trimmed cell
/// strings (rows whose cells are all empty are dropped).
fn extract_tables(html: &str, endpoint: &'static str) -> Result<Vec<Vec<Vec<String>>>> {
    crate::core::html::tables(html, endpoint)
}

/// Find the table whose first header cell is exactly `日期` (matches both the
/// SAFE middle-rate table and the Sina BOC rate table regardless of document order).
fn find_date_table<'a>(
    tables: &'a [Vec<Vec<String>>],
    origin: &'static str,
) -> Result<&'a [Vec<String>]> {
    tables
        .iter()
        .find(|t| {
            t.first()
                .and_then(|r| r.first())
                .map_or(false, |c| c.trim() == "日期")
        })
        .map(|t| t.as_slice())
        .ok_or_else(|| Error::UpstreamChanged {
            origin,
            message: "no table with 日期 header found".into(),
        })
}

// ---------------------------------------------------------------------------
// currency_boc_safe — SAFE RMB central parity rates (wide table)
// ---------------------------------------------------------------------------

/// One date's SAFE RMB central-parity rates against every published currency.
///
/// SAFE publishes a *wide* table (one column per currency: 美元, 欧元, 日元,
/// 港元, 英镑, …). The column set is not fixed, so rates are kept as a map of
/// currency-name → rate.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CurrencyBocSafeRow {
    /// 日期 — rate date (`YYYY-MM-DD`).
    pub date: String,
    /// Currency → central parity rate (akshare columns after `日期`).
    pub rates: HashMap<String, Option<f64>>,
}

/// 人民币汇率中间价 (`currency_boc_safe`, akshare `currency/currency_safe.py:18`).
///
/// Mirrors akshare's live path: a `GET` to SAFE's `RMBQuery.do` (which returns
/// the HTML table; the frozen 2020 Excel snapshot akshare concatenates is a
/// static historical file and is omitted here).
pub async fn currency_boc_safe(client: &Client) -> Result<Vec<CurrencyBocSafeRow>> {
    let url = "https://www.safe.gov.cn/AppStructured/hlw/RMBQuery.do";
    let params: &[(&str, &str)] = &[
        ("startDate", "2024-01-01"),
        ("endDate", "2024-12-31"),
        ("queryYN", "true"),
    ];
    let html = client
        .get_text(SOURCE_SAFE, "currency_boc_safe", url, params, None)
        .await?;
    parse_currency_boc_safe(&html, "currency_boc_safe")
}

pub(crate) fn parse_currency_boc_safe(html: &str, endpoint: &'static str) -> Result<Vec<CurrencyBocSafeRow>> {
    let tables = extract_tables(html, endpoint)?;
    let table = find_date_table(&tables, SOURCE_SAFE)?;
    if table.len() < 2 {
        return Err(Error::UpstreamChanged {
            origin: SOURCE_SAFE,
            message: "RMB table has no data rows".into(),
        });
    }
    let header = &table[0];
    let mut out = Vec::with_capacity(table.len() - 1);
    for row in &table[1..] {
        if row.is_empty() {
            continue;
        }
        let date = row.first().cloned().unwrap_or_default();
        let mut rates = HashMap::new();
        for (i, cell) in row.iter().enumerate().skip(1) {
            let cur = match header.get(i) {
                Some(c) => c.trim(),
                None => continue,
            };
            if cur.is_empty() {
                continue;
            }
            rates.insert(cur.to_string(), cell.trim().parse::<f64>().ok());
        }
        out.push(CurrencyBocSafeRow { date, rates });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// currency_boc_sina — BOC CNY quoted rates history (fixed 6 columns)
// ---------------------------------------------------------------------------

/// One day's Bank-of-China CNY quoted rates (`currency_boc_sina`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CurrencyBocSinaRow {
    /// 日期 (`YYYY-MM-DD`).
    pub date: String,
    /// 中行汇买价 — BOC telegraphic-transfer buying rate.
    pub buy_rate: Option<f64>,
    /// 中行钞买价 — BOC cash buying rate.
    pub cash_buy: Option<f64>,
    /// 中行钞卖价/汇卖价 — BOC (cash/)telegraphic selling rate.
    pub sell_rate: Option<f64>,
    /// 央行中间价 — PBOC central parity.
    pub central: Option<f64>,
    /// 中行折算价 — BOC conversion rate.
    pub convert: Option<f64>,
}

/// Parse the Sina `<select id="money_code">` option list into a
/// `symbol → code` map (akshare `_currency_boc_sina_map`).
fn parse_sina_money_code_map(html: &str, endpoint: &'static str) -> Result<HashMap<String, String>> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse(r#"select#money_code option"#).map_err(|e| Error::Parse {
        endpoint,
        message: format!("money_code selector: {e}"),
    })?;
    let mut map = HashMap::new();
    for opt in doc.select(&sel) {
        let code = match opt.value().attr("value") {
            Some(v) => v.to_string(),
            None => continue,
        };
        let name: String = opt.text().collect::<String>().trim().to_string();
        if name.is_empty() || code.is_empty() {
            continue;
        }
        map.insert(name, code);
    }
    Ok(map)
}

/// 新浪财经-中行人民币牌价历史数据查询 (`currency_boc_sina`, akshare
/// `currency/currency_china_bank_sina.py:57`).
///
/// Mirrors akshare: fetch the forex page to resolve `symbol → money_code`,
/// then paginate `forex.php?call_type=ajax` and concat each page's table.
pub async fn currency_boc_sina(
    client: &Client,
    symbol: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<CurrencyBocSinaRow>> {
    let page_url = "http://biz.finance.sina.com.cn/forex/forex.php";
    let map_page = client
        .get_text(
            SOURCE_SINA,
            "currency_boc_sina",
            page_url,
            &[
                ("startdate", "2012-01-01"),
                ("enddate", "2021-06-14"),
                ("money_code", "EUR"),
                ("type", "0"),
            ],
            None,
        )
        .await?;
    let map = parse_sina_money_code_map(&map_page, "currency_boc_sina")?;
    let code = map.get(symbol).ok_or_else(|| Error::Parse {
        endpoint: "currency_boc_sina",
        message: format!("symbol {symbol} not in Sina money_code map"),
    })?;

    let fmt = |d: &str| [d, &d[4..6], &d[6..8] ].join("-");
    let mut out: Vec<CurrencyBocSinaRow> = Vec::new();
    for page in 1..=20 {
        let params: &[(&str, &str)] = &[
            ("money_code", code),
            ("type", "0"),
            ("startdate", &fmt(start_date)),
            ("enddate", &fmt(end_date)),
            ("page", &page.to_string()),
            ("call_type", "ajax"),
        ];
        let html = client
            .get_text(SOURCE_SINA, "currency_boc_sina", page_url, params, None)
            .await?;
        let rows = parse_currency_boc_sina(&html, "currency_boc_sina")?;
        if rows.is_empty() {
            break;
        }
        let done = rows.len() < 20;
        out.extend(rows);
        if done {
            break;
        }
    }
    Ok(out)
}

pub(crate) fn parse_currency_boc_sina(html: &str, endpoint: &'static str) -> Result<Vec<CurrencyBocSinaRow>> {
    let tables = extract_tables(html, endpoint)?;
    let table = find_date_table(&tables, SOURCE_SINA)?;
    if table.len() < 2 {
        return Ok(Vec::new());
    }
    let header = &table[0];
    // Sina's header order is fixed: [日期, 中行汇买价, 中行钞买价,
    // 中行钞卖价/汇卖价, 央行中间价, 中行折算价]; the raw header may carry a
    // "(元)" suffix, so map by position rather than exact text.
    if header.len() < 6 || header[0].trim() != "日期" {
        return Err(Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "unexpected Sina BOC header layout".into(),
        });
    }
    let num = |row: &[String], i: usize| -> Option<f64> {
        row.get(i).and_then(|c| c.trim().parse::<f64>().ok())
    };
    let mut out = Vec::with_capacity(table.len() - 1);
    for row in &table[1..] {
        if row.is_empty() {
            continue;
        }
        out.push(CurrencyBocSinaRow {
            date: row.first().cloned().unwrap_or_default(),
            buy_rate: num(row, 1),
            cash_buy: num(row, 2),
            sell_rate: num(row, 3),
            central: num(row, 4),
            convert: num(row, 5),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn load_html(name: &str) -> String {
        let bytes = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(name),
        )
        .unwrap();
        match std::str::from_utf8(&bytes) {
            Ok(s) => s.to_string(),
            Err(_) => encoding_rs::GBK
                .decode_without_bom_handling_and_without_replacement(&bytes)
                .unwrap_or_else(|| String::from_utf8_lossy(&bytes))
                .into_owned(),
        }
    }

    #[test]
    fn parses_currency_boc_safe() {
        let rows = parse_currency_boc_safe(&load_html("currency_boc_safe.html"), "currency_boc_safe").unwrap();
        assert!(!rows.is_empty(), "expected SAFE rows");
        // First row should carry the 美元 (USD) central parity.
        let first = &rows[0];
        assert!(!first.date.is_empty(), "empty date");
        let usd = first.rates.get("美元").expect("missing 美元 column").expect("NaN 美元");
        assert!(usd > 0.0, "bad 美元 rate");
    }

    #[test]
    fn parses_currency_boc_sina() {
        let rows = parse_currency_boc_sina(&load_html("currency_boc_sina.html"), "currency_boc_sina").unwrap();
        assert!(!rows.is_empty(), "expected Sina rows");
        let first = &rows[0];
        assert_eq!(first.date, "2021-06-14");
        // 美元 fixture: 汇买价 772.5800.
        assert!((first.buy_rate.unwrap() - 772.58).abs() < 1e-6);
        // 央行中间价 is '--' (missing) in the fixture → None.
        assert!(first.central.is_none(), "central should be None for '--'");
        assert!((first.convert.unwrap() - 777.29).abs() < 1e-6);
    }
}
