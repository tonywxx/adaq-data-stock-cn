//! Economic macro indicators scraped from 同花顺 (THS) HTML tables.
//!
//! All three endpoints are `pd.read_html` scrapes of `data.10jqka.com.cn`
//! macro pages. The pages are `gbk`-encoded, so the [`load_html`] test helper
//! decodes them before parsing. THS uses a two-level header for the loan and
//! deposit tables (a group row + a sub-row of `总额/同比/环比`), which akshare
//! merges; we mirror the merged column layout.
//!
//! * [`macro_stock_finance`] — `economic/macro_finance_ths.py:15`
//! * [`macro_rmb_loan`] — `economic/macro_finance_ths.py:50`
//! * [`macro_rmb_deposit`] — `economic/macro_finance_ths.py:82`

use scraper::{Html, Selector};

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// Parse a numeric cell into `f64`, tolerating thousands separators.
fn as_f64(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', "");
    t.parse::<f64>().ok()
}

/// Extract every `<table>` as a list of rows-of-cells (mirrors `pd.read_html`).
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
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no <table> found".into() });
    }
    Ok(tables)
}

/// THS request headers (the upstream rejects the default client UA).
const THS_HEADERS: &[(&str, &str)] = &[
    (
        "user-agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36",
    ),
    ("accept-language", "zh-CN,zh;q=0.9"),
];

// ---------------------------------------------------------------------------
// macro_stock_finance
// ---------------------------------------------------------------------------

/// One month of A-share fundraising totals (THS `macro/finance`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct MacroStockFinance {
    /// Month (akshare `月份`).
    pub month: String,
    /// Total funds raised, 亿元 (akshare `募集资金`).
    pub total_funds: Option<f64>,
    /// IPO funds raised, 亿元 (akshare `首发募集资金`).
    pub ipo_funds: Option<f64>,
    /// Seasoned (SEO) funds raised, 亿元 (akshare `增发募集资金`).
    pub seo_funds: Option<f64>,
    /// Rights-issue funds raised, 亿元 (akshare `配股募集资金`).
    pub rights_funds: Option<f64>,
}

/// 同花顺-数据中心-宏观数据-股票筹资 (`macro_stock_finance`, akshare `macro_finance_ths.py:15`).
pub async fn macro_stock_finance(client: &Client) -> Result<Vec<MacroStockFinance>> {
    let url = "https://data.10jqka.com.cn/macro/finance/";
    let html = client
        .get_text("ths", "macro_stock_finance", url, &[], Some(THS_HEADERS))
        .await?;
    parse_macro_stock_finance(&html, "macro_stock_finance")
}

pub(crate) fn parse_macro_stock_finance(html: &str, endpoint: &'static str) -> Result<Vec<MacroStockFinance>> {
    let tables = extract_tables(html, endpoint)?;
    let rows = tables
        .into_iter()
        .next()
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "missing table".into() })?;
    // Single header row; data starts at row 1.
    let mut out = Vec::new();
    for cells in rows.into_iter().skip(1) {
        if cells.len() < 5 {
            continue;
        }
        out.push(MacroStockFinance {
            month: cells[0].clone(),
            total_funds: as_f64(&cells[1]),
            ipo_funds: as_f64(&cells[2]),
            seo_funds: as_f64(&cells[3]),
            rights_funds: as_f64(&cells[4]),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no rows".into() });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// macro_rmb_loan
// ---------------------------------------------------------------------------

/// One month of RMB loan data (THS `macro/loan`). `yoy`/`mom` are kept as the
/// raw percentage strings (`-580.00%`) exactly as akshare stores them.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MacroRmbLoan {
    /// Month (akshare `月份`).
    pub month: String,
    /// New RMB loans — total, 亿元 (akshare `新增人民币贷款-总额`).
    pub new_loan_total: Option<f64>,
    /// New RMB loans — YoY (akshare `新增人民币贷款-同比`).
    pub new_loan_yoy: Option<String>,
    /// New RMB loans — MoM (akshare `新增人民币贷款-环比`).
    pub new_loan_mom: Option<String>,
    /// Cumulative RMB loans — total, 亿元 (akshare `累计人民币贷款-总额`).
    pub cum_loan_total: Option<f64>,
    /// Cumulative RMB loans — YoY (akshare `累计人民币贷款-同比`).
    pub cum_loan_yoy: Option<String>,
}

/// 同花顺-数据中心-宏观数据-新增人民币贷款 (`macro_rmb_loan`, akshare `macro_finance_ths.py:50`).
pub async fn macro_rmb_loan(client: &Client) -> Result<Vec<MacroRmbLoan>> {
    let url = "https://data.10jqka.com.cn/macro/loan/";
    let html = client
        .get_text("ths", "macro_rmb_loan", url, &[], Some(THS_HEADERS))
        .await?;
    parse_macro_rmb_loan(&html, "macro_rmb_loan")
}

pub(crate) fn parse_macro_rmb_loan(html: &str, endpoint: &'static str) -> Result<Vec<MacroRmbLoan>> {
    let tables = extract_tables(html, endpoint)?;
    let rows = tables
        .into_iter()
        .next()
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "missing table".into() })?;
    // Two-level header (group row + 总额/同比/环比 sub-row); data starts at row 2.
    let mut out = Vec::new();
    for cells in rows.into_iter().skip(2) {
        if cells.len() < 6 {
            continue;
        }
        out.push(MacroRmbLoan {
            month: cells[0].clone(),
            new_loan_total: as_f64(&cells[1]),
            new_loan_yoy: Some(cells[2].clone()),
            new_loan_mom: Some(cells[3].clone()),
            cum_loan_total: as_f64(&cells[4]),
            cum_loan_yoy: Some(cells[5].clone()),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no rows".into() });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// macro_rmb_deposit
// ---------------------------------------------------------------------------

/// One month of RMB deposit data (THS `macro/rmb`). `yoy`/`mom` kept as raw
/// percentage strings, exactly as akshare stores them.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MacroRmbDeposit {
    /// Month (akshare `月份`).
    pub month: String,
    /// New deposits — amount, 亿元.
    pub new_deposit_amount: Option<f64>,
    /// New deposits — YoY.
    pub new_deposit_yoy: Option<String>,
    /// New deposits — MoM.
    pub new_deposit_mom: Option<String>,
    /// New corporate deposits — amount, 亿元.
    pub new_corp_amount: Option<f64>,
    /// New corporate deposits — YoY.
    pub new_corp_yoy: Option<String>,
    /// New corporate deposits — MoM.
    pub new_corp_mom: Option<String>,
    /// New savings deposits — amount, 亿元.
    pub new_savings_amount: Option<f64>,
    /// New savings deposits — YoY.
    pub new_savings_yoy: Option<String>,
    /// New savings deposits — MoM.
    pub new_savings_mom: Option<String>,
    /// New other deposits — amount, 亿元.
    pub new_other_amount: Option<f64>,
    /// New other deposits — YoY.
    pub new_other_yoy: Option<String>,
    /// New other deposits — MoM.
    pub new_other_mom: Option<String>,
}

/// 同花顺-数据中心-宏观数据-人民币存款余额 (`macro_rmb_deposit`, akshare `macro_finance_ths.py:82`).
pub async fn macro_rmb_deposit(client: &Client) -> Result<Vec<MacroRmbDeposit>> {
    let url = "https://data.10jqka.com.cn/macro/rmb/";
    let html = client
        .get_text("ths", "macro_rmb_deposit", url, &[], Some(THS_HEADERS))
        .await?;
    parse_macro_rmb_deposit(&html, "macro_rmb_deposit")
}

pub(crate) fn parse_macro_rmb_deposit(html: &str, endpoint: &'static str) -> Result<Vec<MacroRmbDeposit>> {
    let tables = extract_tables(html, endpoint)?;
    let rows = tables
        .into_iter()
        .next()
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "missing table".into() })?;
    // Two-level header (group row + 数量/同比/环比 sub-row); data starts at row 2.
    let mut out = Vec::new();
    for cells in rows.into_iter().skip(2) {
        if cells.len() < 13 {
            continue;
        }
        out.push(MacroRmbDeposit {
            month: cells[0].clone(),
            new_deposit_amount: as_f64(&cells[1]),
            new_deposit_yoy: Some(cells[2].clone()),
            new_deposit_mom: Some(cells[3].clone()),
            new_corp_amount: as_f64(&cells[4]),
            new_corp_yoy: Some(cells[5].clone()),
            new_corp_mom: Some(cells[6].clone()),
            new_savings_amount: as_f64(&cells[7]),
            new_savings_yoy: Some(cells[8].clone()),
            new_savings_mom: Some(cells[9].clone()),
            new_other_amount: as_f64(&cells[10]),
            new_other_yoy: Some(cells[11].clone()),
            new_other_mom: Some(cells[12].clone()),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no rows".into() });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

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
            Err(_) => match encoding_rs::GBK
                .decode_without_bom_handling_and_without_replacement(&bytes)
            {
                Some(cow) => cow.into_owned(),
                None => String::from_utf8_lossy(&bytes).into_owned(),
            },
        }
    }

    #[test]
    fn parses_macro_stock_finance() {
        let rows = parse_macro_stock_finance(&load_html("macro_stock_finance.html"), "macro_stock_finance").unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].total_funds.is_some());
    }

    #[test]
    fn parses_macro_rmb_loan() {
        let rows = parse_macro_rmb_loan(&load_html("macro_rmb_loan.html"), "macro_rmb_loan").unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].new_loan_total.is_some());
        assert!(rows[0].new_loan_yoy.as_ref().unwrap().contains('%'));
        assert!(rows[0].cum_loan_total.is_some());
    }

    #[test]
    fn parses_macro_rmb_deposit() {
        let rows = parse_macro_rmb_deposit(&load_html("macro_rmb_deposit.html"), "macro_rmb_deposit").unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].new_deposit_amount.is_some());
        assert!(rows[0].new_corp_mom.as_ref().unwrap().contains('%'));
        assert!(rows[0].new_other_amount.is_some());
    }
}
