//! Excel-backed SZSE fund scale reports (akshare `fund/fund_etf_szse.py`, `fund/fund_scale_szse.py`).

use calamine::{open_workbook_auto_from_rs, Reader, Sheets};

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use crate::core::json::*;

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/88.0.4324.150 Safari/537.36";

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

/// SZSE ETF fund-scale row (`fund_etf_scale_szse`, akshare `fund/fund_etf_szse.py:15`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundEtfScaleSzse {
    /// Fund code (akshare `基金代码`).
    pub fund_code: String,
    /// Fund abbreviation (akshare `基金简称`).
    pub fund_abbr: String,
    /// Fund category (akshare `基金类别`).
    pub fund_category: String,
    /// Investment category (akshare `投资类别`).
    pub invest_category: String,
    /// Listing date `YYYY-MM-DD` (akshare `上市日期`).
    pub list_date: String,
    /// Fund shares / scale (akshare `基金份额`).
    pub fund_scale: Option<f64>,
    /// Fund manager (akshare `基金管理人`).
    pub fund_manager: String,
    /// Fund sponsor (akshare `基金发起人`).
    pub fund_sponsor: String,
    /// Fund trustee (akshare `基金托管人`).
    pub fund_trustee: String,
    /// Net value (akshare `净值`).
    pub net_value: Option<f64>,
}

/// SZSE ETF fund scale (`fund_etf_scale_szse`, akshare `fund/fund_etf_szse.py:15`).
pub async fn fund_etf_scale_szse(_client: &Client) -> Result<Vec<FundEtfScaleSzse>> {
    let url = "https://fund.szse.cn/api/report/ShowReport";
    let params = &[
        ("SHOWTYPE", "xlsx"),
        ("CATALOGID", "1000_lf"),
        ("TABKEY", "tab1"),
        ("random", "0.07610353191740105"),
    ];
    let headers = &[("Referer", "https://fund.szse.cn/marketdata/fundslist/index.html")];
    let bytes = fetch_bytes(url, params, headers).await?;
    parse_fund_etf_scale_szse(&bytes)
}

pub(crate) fn parse_fund_etf_scale_szse(bytes: &[u8]) -> Result<Vec<FundEtfScaleSzse>> {
    let rows = read_rows(bytes, "fund_etf_scale_szse")?;
    let mut out = Vec::new();
    for r in rows.iter().skip(1) {
        if r.iter().all(|c| c.is_empty()) {
            continue;
        }
        out.push(FundEtfScaleSzse {
            fund_code: col(r, 0).to_string(),
            fund_abbr: col(r, 1).to_string(),
            fund_category: col(r, 2).to_string(),
            invest_category: col(r, 3).to_string(),
            list_date: col(r, 4).to_string(),
            fund_scale: parse_f64_str(col(r, 5)),
            fund_manager: col(r, 6).to_string(),
            fund_sponsor: col(r, 7).to_string(),
            fund_trustee: col(r, 8).to_string(),
            net_value: parse_f64_str(col(r, 9)),
        });
    }
    Ok(out)
}

/// SZSE daily fund scale row (`fund_scale_daily_szse`, akshare `fund/fund_scale_szse.py:27`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundScaleDailySzse {
    /// Date `YYYY-MM-DD` (akshare `日期`).
    pub date: String,
    /// Fund code (akshare `基金代码`).
    pub fund_code: String,
    /// Fund abbreviation (akshare `基金简称`).
    pub fund_abbr: String,
    /// Fund shares / scale (akshare `基金份额`).
    pub fund_scale: Option<f64>,
}

/// SZSE daily fund scale (`fund_scale_daily_szse`, akshare `fund/fund_scale_szse.py:27`).
pub async fn fund_scale_daily_szse(
    _client: &Client,
    start_date: &str,
    end_date: &str,
    symbol: &str,
) -> Result<Vec<FundScaleDailySzse>> {
    let jjlb = match symbol {
        "ETF" => "ETF",
        "LOF" => "LOF",
        "REITS" => "不动产基金",
        _ => return Err(Error::Parse {
            endpoint: "fund_scale_daily_szse",
            message: "symbol must be one of ETF/LOF/REITS".into(),
        }),
    };
    let start = format!("{}-{}-{}", &start_date[..4], &start_date[4..6], &start_date[6..]);
    let end = format!("{}-{}-{}", &end_date[..4], &end_date[4..6], &end_date[6..]);
    let url = "https://www.szse.cn/api/report/ShowReport";
    let params = &[
        ("SHOWTYPE", "xlsx"),
        ("CATALOGID", "scsj_fund_jjgm"),
        ("TABKEY", "tab1"),
        ("txtStart", &start),
        ("txtEnd", &end),
        ("jjlb", jjlb),
        ("random", "0.123456789"),
    ];
    let headers = &[("Referer", "https://www.szse.cn/market/fund/volume/etf/index.html")];
    let bytes = fetch_bytes(url, params, headers).await?;
    parse_fund_scale_daily_szse(&bytes)
}

pub(crate) fn parse_fund_scale_daily_szse(bytes: &[u8]) -> Result<Vec<FundScaleDailySzse>> {
    let rows = read_rows(bytes, "fund_scale_daily_szse")?;
    let mut out = Vec::new();
    for r in rows.iter().skip(1) {
        if r.iter().all(|c| c.is_empty()) {
            continue;
        }
        out.push(FundScaleDailySzse {
            date: col(r, 0).to_string(),
            fund_code: col(r, 1).to_string(),
            fund_abbr: col(r, 2).to_string(),
            fund_scale: parse_f64_str(col(r, 3)),
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
    fn parses_fund_etf_scale_szse() {
        let rows = parse_fund_etf_scale_szse(&fixture("fund_etf_scale_szse.xlsx")).unwrap();
        assert!(rows.len() > 100);
        assert_eq!(rows[0].fund_code, "158006");
        assert_eq!(rows[0].fund_abbr, "化工ETF博时");
        assert_eq!(rows[0].list_date, "2026-08-07");
        assert!((rows[0].fund_scale.unwrap() - 39_046_012.0).abs() < 1.0);
        assert!((rows[0].net_value.unwrap() - 1.0179).abs() < 1e-6);
    }

    #[test]
    fn parses_fund_scale_daily_szse() {
        let rows = parse_fund_scale_daily_szse(&fixture("fund_scale_daily_szse.xlsx")).unwrap();
        assert!(rows.len() > 10);
        assert_eq!(rows[0].date, "2026-04-01");
        assert_eq!(rows[0].fund_code, "159001");
        assert!((rows[0].fund_scale.unwrap() - 15_882_142.0).abs() < 1.0);
    }
}
