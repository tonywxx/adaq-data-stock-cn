//! Excel-backed China macro leverage ratio (akshare `economic/marco_cnbs.py`).

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

fn read_rows_named(bytes: &[u8], name: &str, endpoint: &'static str) -> Result<Vec<Vec<String>>> {
    let mut wb: Sheets<std::io::Cursor<Vec<u8>>> =
        open_workbook_auto_from_rs(std::io::Cursor::new(bytes.to_vec())).map_err(|e| {
            Error::Parse {
                endpoint,
                message: e.to_string(),
            }
        })?;
    let range = wb.worksheet_range(name).map_err(|e| Error::Parse {
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

/// Convert an Excel serial date (1900 date system) to `YYYY-MM`.
fn excel_serial_to_month(serial: f64) -> Option<String> {
    if !serial.is_finite() || serial <= 0.0 {
        return None;
    }
    let days = serial.floor() as i64;
    let base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30)?;
    base.checked_add_days(chrono::Days::new(days as u64))
        .map(|d| d.format("%Y-%m").to_string())
}

/// China macro leverage-ratio row (akshare `macro_cnbs`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct MacroCnbs {
    /// Period as `YYYY-MM` (akshare `年份`).
    pub year_month: String,
    /// Household sector (akshare `居民部门`).
    pub household: Option<f64>,
    /// Non-financial corporations (akshare `非金融企业部门`).
    pub non_financial_corporations: Option<f64>,
    /// Central government (akshare `中央政府`).
    pub central_government: Option<f64>,
    /// Local government (akshare `地方政府`).
    pub local_government: Option<f64>,
    /// General government (akshare `政府部门`).
    pub general_government: Option<f64>,
    /// Non-financial sector (akshare `实体经济部门`).
    pub non_financial_sector: Option<f64>,
    /// Financial sector, asset side (akshare `金融部门资产方`).
    pub financial_sector_asset: Option<f64>,
    /// Financial sector, liability side (akshare `金融部门负债方`).
    pub financial_sector_liability: Option<f64>,
}

/// China macro leverage ratio (`macro_cnbs`, akshare `economic/marco_cnbs.py:12`).
pub async fn macro_cnbs(_client: &Client) -> Result<Vec<MacroCnbs>> {
    let url = "http://114.115.232.154:8080/handler/download.ashx";
    let bytes = fetch_bytes(url, &[], &[]).await?;
    parse_macro_cnbs(&bytes)
}

pub(crate) fn parse_macro_cnbs(bytes: &[u8]) -> Result<Vec<MacroCnbs>> {
    // akshare reads sheet "Data" with skiprows=1, header=0, so the second
    // row (English headers) becomes the column names and data follows.
    let rows = read_rows_named(bytes, "Data", "macro_cnbs")?;
    if rows.len() < 3 {
        return Ok(Vec::new());
    }
    let header = &rows[1];
    let find = |name: &str| {
        header
            .iter()
            .position(|c| c.trim() == name)
            .ok_or_else(|| Error::Parse {
                endpoint: "macro_cnbs".into(),
                message: format!("missing column {name}"),
            })
    };
    let i_period = find("Period")?;
    let i_hh = find("Household")?;
    let i_nfc = find("Non-financial corporations")?;
    let i_cg = find("Central government")?;
    let i_lg = find("Local government")?;
    let i_gg = find("General government")?;
    let i_nfs = find("Non financial sector")?;
    let i_fa = find("Financial sector(asset side)")?;
    let i_fl = find("Financial sector(liability side)")?;

    let mut out = Vec::new();
    for r in &rows[2..] {
        if r.iter().all(|c| c.is_empty()) {
            continue;
        }
        let serial = parse_f64(r[i_period].trim());
        out.push(MacroCnbs {
            year_month: serial
                .and_then(excel_serial_to_month)
                .unwrap_or_default(),
            household: parse_f64(r[i_hh].trim()),
            non_financial_corporations: parse_f64(r[i_nfc].trim()),
            central_government: parse_f64(r[i_cg].trim()),
            local_government: parse_f64(r[i_lg].trim()),
            general_government: parse_f64(r[i_gg].trim()),
            non_financial_sector: parse_f64(r[i_nfs].trim()),
            financial_sector_asset: parse_f64(r[i_fa].trim()),
            financial_sector_liability: parse_f64(r[i_fl].trim()),
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
    fn parses_macro_cnbs() {
        let rows = parse_macro_cnbs(&fixture("macro_cnbs.xlsx")).unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].household, Some(17.6));
        assert!((rows[0].non_financial_corporations.unwrap() - 104.6).abs() < 1e-6);
        // Period serial 38442 -> 2005-03 (YYYY-MM)
        assert_eq!(rows[0].year_month.len(), 7);
        assert!(rows[0].year_month.starts_with("2005"));
    }
}
