//! Excel-backed SSE bond summary reports (akshare `bond/bond_summary.py`).

use calamine::{open_workbook_auto_from_rs, Reader, Sheets};

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/87.0.4280.88 Safari/537.36";

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

/// SSE cash-bond market overview (`bond_cash_summary_sse`, akshare `bond/bond_summary.py:15`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondCashSummarySse {
    /// Bond category (akshare `债券现货`).
    pub bond_type: String,
    /// Custody count (akshare `托管只数`).
    pub custody_count: Option<f64>,
    /// Custody market value in 100M CNY (akshare `托管市值`).
    pub custody_market_value: Option<f64>,
    /// Custody face value in 100M CNY (akshare `托管面值`).
    pub custody_face_value: Option<f64>,
    /// Trade date `YYYY-MM-DD` (akshare `数据日期`).
    pub date: String,
}

/// SSE cash-bond market overview (akshare `bond/bond_summary.py:15`).
pub async fn bond_cash_summary_sse(_client: &Client, date: &str) -> Result<Vec<BondCashSummarySse>> {
    let d = format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..]);
    let url = "http://query.sse.com.cn/commonExcelDd.do";
    let params = &[
        ("sqlId", "COMMON_SSEBOND_SCSJ_SCTJ_SCGL_ZQXQSCGL_CX_L"),
        ("TRADE_DATE", &d),
    ];
    let headers = &[("Referer", "http://bond.sse.com.cn/")];
    let bytes = fetch_bytes(url, params, headers).await?;
    parse_bond_cash_summary_sse(&bytes)
}

pub(crate) fn parse_bond_cash_summary_sse(bytes: &[u8]) -> Result<Vec<BondCashSummarySse>> {
    let rows = read_rows(bytes, "bond_cash_summary_sse")?;
    let mut out = Vec::new();
    for r in rows.iter().skip(1) {
        if r.iter().all(|c| c.is_empty()) {
            continue;
        }
        out.push(BondCashSummarySse {
            bond_type: col(r, 0).to_string(),
            custody_count: parse_f64(col(r, 1)),
            custody_market_value: parse_f64(col(r, 2)),
            custody_face_value: parse_f64(col(r, 3)),
            date: col(r, 4).to_string(),
        });
    }
    Ok(out)
}

/// SSE bond dealing overview (`bond_deal_summary_sse`, akshare `bond/bond_summary.py:50`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondDealSummarySse {
    /// Bond type (akshare `债券类型`).
    pub bond_type: String,
    /// Deals today (akshare `当日成交笔数`).
    pub deal_count_today: Option<f64>,
    /// Deal amount today in 10k CNY (akshare `当日成交金额`).
    pub deal_amount_today: Option<f64>,
    /// Deals YTD (akshare `当年成交笔数`).
    pub deal_count_year: Option<f64>,
    /// Deal amount YTD in 10k CNY (akshare `当年成交金额`).
    pub deal_amount_year: Option<f64>,
    /// Trade date `YYYY-MM-DD` (akshare `数据日期`).
    pub date: String,
}

/// SSE bond dealing overview (akshare `bond/bond_summary.py:50`).
pub async fn bond_deal_summary_sse(_client: &Client, date: &str) -> Result<Vec<BondDealSummarySse>> {
    let d = format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..]);
    let url = "http://query.sse.com.cn/commonExcelDd.do";
    let params = &[
        ("sqlId", "COMMON_SSEBOND_SCSJ_SCTJ_SCGL_ZQCJGL_CX_L"),
        ("TRADE_DATE", &d),
    ];
    let headers = &[("Referer", "http://bond.sse.com.cn/")];
    let bytes = fetch_bytes(url, params, headers).await?;
    parse_bond_deal_summary_sse(&bytes)
}

pub(crate) fn parse_bond_deal_summary_sse(bytes: &[u8]) -> Result<Vec<BondDealSummarySse>> {
    let rows = read_rows(bytes, "bond_deal_summary_sse")?;
    let mut out = Vec::new();
    for r in rows.iter().skip(1) {
        if r.iter().all(|c| c.is_empty()) {
            continue;
        }
        out.push(BondDealSummarySse {
            bond_type: col(r, 0).to_string(),
            deal_count_today: parse_f64(col(r, 1)),
            deal_amount_today: parse_f64(col(r, 2)),
            deal_count_year: parse_f64(col(r, 3)),
            deal_amount_year: parse_f64(col(r, 4)),
            date: col(r, 5).to_string(),
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
    fn parses_bond_cash_summary_sse() {
        let rows = parse_bond_cash_summary_sse(&fixture("bond_cash_summary_sse.xlsx")).unwrap();
        assert!(rows.len() >= 3);
        assert_eq!(rows[0].bond_type, "国债");
        assert_eq!(rows[0].custody_count, Some(193.0));
        assert!((rows[0].custody_market_value.unwrap() - 6815.47).abs() < 1e-6);
        assert_eq!(rows[0].date, "2021-01-11");
    }

    #[test]
    fn parses_bond_deal_summary_sse() {
        let rows = parse_bond_deal_summary_sse(&fixture("bond_deal_summary_sse.xlsx")).unwrap();
        assert!(rows.len() >= 3);
        assert_eq!(rows[0].bond_type, "记账式国债");
        assert_eq!(rows[0].deal_count_today, Some(3685.0));
        assert!((rows[0].deal_amount_today.unwrap() - 363349.44).abs() < 1e-6);
        assert_eq!(rows[0].date, "2021-01-04");
    }
}
