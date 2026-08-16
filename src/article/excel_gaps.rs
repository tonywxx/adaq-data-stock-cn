//! Excel-backed Economic Policy Uncertainty index (akshare `article/epu_index.py`).

use std::collections::HashMap;

use calamine::{open_workbook_auto_from_rs, Reader, Sheets};

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const BASE: &str = "http://www.policyuncertainty.com/media";
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

/// One Economic Policy Uncertainty observation (akshare `article_epu_index`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct EpuIndexRow {
    /// Calendar year (akshare `Year`).
    pub year: String,
    /// Calendar month (akshare `Month`).
    pub month: String,
    /// Remaining policy-uncertainty index columns keyed by their header name.
    pub values: HashMap<String, Option<f64>>,
}

/// Map an akshare `symbol` to the policyuncertainty.com media URL.
///
/// NOTE: the akshare default `"China"` URL
/// (`SCMP_China_EPU_Data_Annotated.xlsx`) now returns 404 upstream; the
/// documented xlsx files for the other symbols are still served, so we build
/// xlsx URLs for every branch (akshare falls back to CSV for the generic
/// branch).
fn epu_url(symbol: &str) -> String {
    let mapped = match symbol {
        "China" | "China New" => "SCMP_China",
        "Hong Kong" => "HK",
        "USA" => "US",
        "Germany" | "France" | "Italy" => "Europe",
        "South Korea" => "Korea",
        "Spain New" => "Spain",
        "Greece" => "FKT_Greece",
        other => other,
    };
    match mapped {
        "FKT_Greece" => format!("{BASE}/FKT_Greece_Policy_Uncertainty_Data.xlsx"),
        "SCMP_China" | "HK" => format!("{BASE}/{mapped}_EPU_Data_Annotated.xlsx"),
        other => format!("{BASE}/{other}_Policy_Uncertainty_Data.xlsx"),
    }
}

/// Economic Policy Uncertainty index (`article_epu_index`, akshare `article/epu_index.py:12`).
pub async fn article_epu_index(_client: &Client, symbol: &str) -> Result<Vec<EpuIndexRow>> {
    let url = epu_url(symbol);
    let bytes = fetch_bytes(&url, &[], &[]).await?;
    parse_article_epu_index(&bytes)
}

pub(crate) fn parse_article_epu_index(bytes: &[u8]) -> Result<Vec<EpuIndexRow>> {
    let rows = read_rows(bytes, "article_epu_index")?;
    if rows.len() < 2 {
        return Ok(Vec::new());
    }
    let header = &rows[0];
    let yi = header
        .iter()
        .position(|c| c == "Year")
        .ok_or_else(|| Error::Parse {
            endpoint: "article_epu_index".into(),
            message: "missing Year column".into(),
        })?;
    let mi = header
        .iter()
        .position(|c| c == "Month")
        .ok_or_else(|| Error::Parse {
            endpoint: "article_epu_index".into(),
            message: "missing Month column".into(),
        })?;
    let mut out = Vec::new();
    for r in &rows[1..] {
        if r.iter().all(|c| c.is_empty()) {
            continue;
        }
        let mut values = HashMap::new();
        for (i, h) in header.iter().enumerate() {
            if i == yi || i == mi || h.is_empty() {
                continue;
            }
            values.insert(h.clone(), parse_f64(&r[i]));
        }
        out.push(EpuIndexRow {
            year: r[yi].clone(),
            month: r[mi].clone(),
            values,
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
    fn parses_article_epu_index() {
        let rows = parse_article_epu_index(&fixture("article_epu_index.xlsx")).unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].year, "2026");
        assert_eq!(rows[0].month, "7");
        let v = rows[0].values.get("News_Based_Policy_Uncert_Index").unwrap();
        assert!((v.unwrap() - 219.32374070681453).abs() < 1e-6);
        // ensure no empty-key entries leak in
        assert!(!rows[0].values.contains_key(""));
    }

    #[test]
    fn epu_url_is_excel() {
        assert_eq!(
            epu_url("USA"),
            "http://www.policyuncertainty.com/media/US_Policy_Uncertainty_Data.xlsx"
        );
        assert_eq!(
            epu_url("China"),
            "http://www.policyuncertainty.com/media/SCMP_China_EPU_Data_Annotated.xlsx"
        );
    }
}
