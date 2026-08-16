//! Excel-backed SZSE stock reports and SW industry classification
//! (akshare `stock/stock_industry_sw.py`, `stock/stock_info.py`, `stock/stock_summary.py`).

use calamine::{open_workbook_auto_from_rs, Reader, Sheets};

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36";

async fn fetch_bytes(
    url: &str,
    params: &[(&str, &str)],
    headers: &[(&str, &str)],
) -> Result<Vec<u8>> {
    let http = reqwest::Client::builder()
        .user_agent(UA)
        .build()
        .map_err(Error::Http)?;
    let mut req = http.get(url);
    if !params.is_empty() {
        req = req.query(params);
    }
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let resp = req.send().await.map_err(Error::Http)?;
    let bytes = resp.bytes().await.map_err(Error::Http)?;
    Ok(bytes.to_vec())
}

fn read_rows(bytes: &[u8], endpoint: &'static str) -> Result<Vec<Vec<String>>> {
    let mut wb: Sheets<std::io::Cursor<Vec<u8>>> =
        open_workbook_auto_from_rs(std::io::Cursor::new(bytes.to_vec())).map_err(|e| {
            Error::Parse {
                endpoint,
                message: e.to_string(),
            }
        })?;
    let range = wb
        .worksheet_range_at(0)
        .ok_or_else(|| Error::Parse {
            endpoint,
            message: "no sheet".into(),
        })?
        .map_err(|e| Error::Parse {
            endpoint,
            message: e.to_string(),
        })?;
    Ok(range
        .rows()
        .map(|r| r.iter().map(cell_to_string).collect())
        .collect())
}

fn parse_f64(s: &str) -> Option<f64> {
    let t: String = s.chars().filter(|c| *c != ',').collect();
    let t = t.trim();
    if t.is_empty() {
        return None;
    }
    t.parse::<f64>().ok()
}

fn cell_to_string(c: &calamine::Data) -> String {
    match c {
        calamine::Data::Empty => String::new(),
        calamine::Data::String(s) => s.trim().to_string(),
        calamine::Data::Int(i) => i.to_string(),
        calamine::Data::Float(f) => {
            if f.is_finite() && f.fract() == 0.0 && f.abs() < 1e15 {
                format!("{}", *f as i64)
            } else {
                format!("{}", f)
            }
        }
        calamine::Data::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}

fn col<'a>(row: &'a [String], i: usize) -> &'a str {
    row.get(i).map(|s| s.as_str()).unwrap_or("")
}

/// Convert an Excel serial date (1900 date system) to `YYYY-MM-DD`.
fn excel_serial_to_date(serial: f64) -> Option<String> {
    if !serial.is_finite() || serial <= 0.0 {
        return None;
    }
    let days = serial.floor() as i64;
    let base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30)?;
    base.checked_add_days(chrono::Days::new(days as u64))
        .map(|d| d.format("%Y-%m-%d").to_string())
}

/// SW industry classification history row (`stock_industry_clf_hist_sw`, akshare `stock/stock_industry_sw.py:17`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockIndustryClfHistSw {
    /// Stock code (akshare `symbol` / `股票代码`).
    pub symbol: String,
    /// Inclusion date `YYYY-MM-DD` (akshare `start_date` / `计入日期`).
    pub start_date: Option<String>,
    /// Industry code (akshare `industry_code` / `行业代码`).
    pub industry_code: String,
    /// Update time `YYYY-MM-DD` (akshare `update_time` / `更新日期`).
    pub update_time: Option<String>,
}

/// SW industry classification history (`stock_industry_clf_hist_sw`, akshare `stock/stock_industry_sw.py:17`).
pub async fn stock_industry_clf_hist_sw(_client: &Client) -> Result<Vec<StockIndustryClfHistSw>> {
    let url = "https://www.swsresearch.com/swindex/pdf/SwClass2021/StockClassifyUse_stock.xls";
    let headers = &[("Referer", "https://www.swsresearch.com/")];
    let bytes = fetch_bytes(url, &[], headers).await?;
    parse_stock_industry_clf_hist_sw(&bytes)
}

pub(crate) fn parse_stock_industry_clf_hist_sw(
    bytes: &[u8],
) -> Result<Vec<StockIndustryClfHistSw>> {
    // Dates are stored as Excel serial numbers (dates feature is off in calamine).
    let rows = read_rows(bytes, "stock_industry_clf_hist_sw")?;
    let mut out = Vec::new();
    for r in rows.iter().skip(1) {
        if r.iter().all(|c| c.is_empty()) {
            continue;
        }
        out.push(StockIndustryClfHistSw {
            symbol: col(r, 0).to_string(),
            start_date: parse_f64(col(r, 1)).and_then(excel_serial_to_date),
            industry_code: col(r, 2).to_string(),
            update_time: parse_f64(col(r, 3)).and_then(excel_serial_to_date),
        });
    }
    Ok(out)
}

/// SZSE name-change row (`stock_info_sz_change_name`, akshare `stock/stock_info.py:384`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockInfoSzChangeName {
    /// Change date `YYYY-MM-DD` (akshare `变更日期`).
    pub change_date: String,
    /// Security code (akshare `证券代码`).
    pub security_code: String,
    /// Security abbreviation (akshare `证券简称`).
    pub security_abbr: String,
    /// Previous full name (akshare `变更前全称`).
    pub old_full_name: String,
    /// New full name (akshare `变更后全称`).
    pub new_full_name: String,
}

/// SZSE name changes (`stock_info_sz_change_name`, akshare `stock/stock_info.py:384`).
pub async fn stock_info_sz_change_name(
    _client: &Client,
    symbol: &str,
) -> Result<Vec<StockInfoSzChangeName>> {
    let tab = match symbol {
        "全称变更" => "tab1",
        "简称变更" => "tab2",
        _ => "tab1",
    };
    let url = "https://www.szse.cn/api/report/ShowReport";
    let params = &[
        ("SHOWTYPE", "xlsx"),
        ("CATALOGID", "SSGSGMXX"),
        ("TABKEY", tab),
        ("random", "0.6935816432433362"),
    ];
    let bytes = fetch_bytes(url, params, &[]).await?;
    parse_stock_info_sz_change_name(&bytes)
}

pub(crate) fn parse_stock_info_sz_change_name(
    bytes: &[u8],
) -> Result<Vec<StockInfoSzChangeName>> {
    let rows = read_rows(bytes, "stock_info_sz_change_name")?;
    let mut out = Vec::new();
    for r in rows.iter().skip(1) {
        if r.iter().all(|c| c.is_empty()) {
            continue;
        }
        out.push(StockInfoSzChangeName {
            change_date: col(r, 0).to_string(),
            security_code: col(r, 1).to_string(),
            security_abbr: col(r, 2).to_string(),
            old_full_name: col(r, 3).to_string(),
            new_full_name: col(r, 4).to_string(),
        });
    }
    Ok(out)
}

/// SZSE area trading ranking row (`stock_szse_area_summary`, akshare `stock/stock_summary.py:53`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockSzseAreaSummary {
    /// Sequence number (akshare `序号`).
    pub seq: Option<f64>,
    /// Region (akshare `地区`).
    pub region: String,
    /// Total trading amount in CNY (akshare `总交易额`).
    pub total_amount: Option<f64>,
    /// Market share percentage (akshare `占市场`).
    pub market_share: Option<f64>,
    /// Stock trading amount in CNY (akshare `股票交易额`).
    pub stock_amount: Option<f64>,
    /// Fund trading amount in CNY (akshare `基金交易额`).
    pub fund_amount: Option<f64>,
    /// Bond trading amount in CNY (akshare `债券交易额`).
    pub bond_amount: Option<f64>,
}

/// SZSE area trading ranking (`stock_szse_area_summary`, akshare `stock/stock_summary.py:53`).
pub async fn stock_szse_area_summary(_client: &Client, date: &str) -> Result<Vec<StockSzseAreaSummary>> {
    let dt = format!("{}-{}", &date[..4], &date[4..6]);
    let url = "https://www.szse.cn/api/report/ShowReport";
    let params = &[
        ("SHOWTYPE", "xlsx"),
        ("CATALOGID", "1803_sczm"),
        ("TABKEY", "tab2"),
        ("DATETIME", &dt),
        ("random", "0.39349437497296137"),
    ];
    let bytes = fetch_bytes(url, params, &[]).await?;
    parse_stock_szse_area_summary(&bytes)
}

pub(crate) fn parse_stock_szse_area_summary(bytes: &[u8]) -> Result<Vec<StockSzseAreaSummary>> {
    let rows = read_rows(bytes, "stock_szse_area_summary")?;
    let mut out = Vec::new();
    for r in rows.iter().skip(1) {
        if r.iter().all(|c| c.is_empty()) {
            continue;
        }
        out.push(StockSzseAreaSummary {
            seq: parse_f64(col(r, 0)),
            region: col(r, 1).to_string(),
            total_amount: parse_f64(col(r, 2)),
            market_share: parse_f64(col(r, 3)),
            stock_amount: parse_f64(col(r, 4)),
            fund_amount: parse_f64(col(r, 5)),
            bond_amount: parse_f64(col(r, 6)),
        });
    }
    Ok(out)
}

/// SZSE market overview row (`stock_szse_summary`, akshare `stock/stock_summary.py:22`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockSzseSummary {
    /// Security category (akshare `证券类别`).
    pub security_category: String,
    /// Count (akshare `数量`).
    pub count: Option<f64>,
    /// Deal amount in CNY (akshare `成交金额`).
    pub deal_amount: Option<f64>,
    /// Total market value in CNY (akshare `总市值`).
    pub total_market_value: Option<f64>,
    /// Free-float market value in CNY (akshare `流通市值`).
    pub float_market_value: Option<f64>,
}

/// SZSE market overview (`stock_szse_summary`, akshare `stock/stock_summary.py:22`).
pub async fn stock_szse_summary(_client: &Client, date: &str) -> Result<Vec<StockSzseSummary>> {
    let d = format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..]);
    let url = "http://www.szse.cn/api/report/ShowReport";
    let params = &[
        ("SHOWTYPE", "xlsx"),
        ("CATALOGID", "1803_sczm"),
        ("TABKEY", "tab1"),
        ("txtQueryDate", &d),
        ("random", "0.39339437497296137"),
    ];
    let bytes = fetch_bytes(url, params, &[]).await?;
    parse_stock_szse_summary(&bytes)
}

pub(crate) fn parse_stock_szse_summary(bytes: &[u8]) -> Result<Vec<StockSzseSummary>> {
    let rows = read_rows(bytes, "stock_szse_summary")?;
    let mut out = Vec::new();
    for r in rows.iter().skip(1) {
        if r.iter().all(|c| c.is_empty()) {
            continue;
        }
        out.push(StockSzseSummary {
            security_category: col(r, 0).trim().to_string(),
            count: parse_f64(col(r, 1)),
            deal_amount: parse_f64(col(r, 2)),
            total_market_value: parse_f64(col(r, 3)),
            float_market_value: parse_f64(col(r, 4)),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(name),
        )
        .unwrap()
    }

    #[test]
    fn parses_stock_industry_clf_hist_sw() {
        let rows = parse_stock_industry_clf_hist_sw(&fixture("stock_industry_clf_hist_sw.xls")).unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].symbol, "000001");
        assert!(rows[0].start_date.is_some());
        assert_eq!(rows[0].industry_code, "440101");
        assert!(rows[0].update_time.is_some());
        // serial 33331 -> 1991-04-18
        assert_eq!(rows[0].start_date.as_deref().unwrap().len(), 10);
    }

    #[test]
    fn parses_stock_info_sz_change_name() {
        let rows = parse_stock_info_sz_change_name(&fixture("stock_info_sz_change_name.xlsx")).unwrap();
        assert!(rows.len() > 10);
        assert_eq!(rows[0].security_code, "002662");
        assert_eq!(rows[0].security_abbr, "峰璟股份");
        assert_eq!(rows[0].change_date, "2026-08-11");
        assert!(rows[0].new_full_name.contains("峰璟新能源"));
    }

    #[test]
    fn parses_stock_szse_area_summary() {
        let rows = parse_stock_szse_area_summary(&fixture("stock_szse_area_summary.xlsx")).unwrap();
        assert!(rows.len() > 5);
        assert_eq!(rows[0].region, "上海");
        assert!((rows[0].market_share.unwrap() - 17.144).abs() < 1e-6);
        assert!(rows[0].total_amount.unwrap() > 1e11);
    }

    #[test]
    fn parses_stock_szse_summary() {
        let rows = parse_stock_szse_summary(&fixture("stock_szse_summary.xlsx")).unwrap();
        assert!(rows.len() > 3);
        assert_eq!(rows[0].security_category, "股票");
        assert_eq!(rows[0].count, Some(2874.0));
        assert!(rows[0].deal_amount.unwrap() > 1e11);
        // sub-row leading spaces are trimmed
        assert!(rows.iter().any(|r| r.security_category == "主板A股"));
    }
}
