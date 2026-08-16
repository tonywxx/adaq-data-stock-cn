//! LPR (Loan Prime Rate) — Eastmoney datacenter report `RPTA_WEB_RATE`.
//!
//! Port of akshare's `macro_china_lpr` (in `economic/macro_china.py`). The
//! Eastmoney `datacenter-web` endpoint returns plain JSON paged over
//! `result.pages`; no JS signing is required (static report token).

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

const URL: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";
const TOKEN: &str = "894050c76af8597a853f5b408b759f5d";
const PAGE_SIZE: u32 = 500;

/// One LPR quote: trade date plus the 1Y / 5Y rates and the legacy `RATE_1` / `RATE_2`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LprRow {
    pub date: Option<String>,
    pub lpr_1y: Option<f64>,
    pub lpr_5y: Option<f64>,
    pub rate_1: Option<f64>,
    pub rate_2: Option<f64>,
}

/// Fetch the full LPR history, walking all pages of the Eastmoney report.
pub async fn lpr(client: &Client) -> Result<Vec<LprRow>> {
    let mut out = Vec::new();
    let mut page: u32 = 1;
    loop {
        let page_s = page.to_string();
        let pz_s = PAGE_SIZE.to_string();
        let params = [
            ("reportName", "RPTA_WEB_RATE"),
            ("columns", "ALL"),
            ("sortColumns", "TRADE_DATE"),
            ("sortTypes", "-1"),
            ("token", TOKEN),
            ("pageNumber", page_s.as_str()),
            ("pageSize", pz_s.as_str()),
            ("p", "1"),
            ("pageNo", "1"),
            ("pageNum", "1"),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, "lpr", URL, &params)
            .await?;
        let data = v
            .get("result")
            .and_then(|r| r.get("data"))
            .and_then(|d| d.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing result.data".into(),
            })?;
        if data.is_empty() {
            break;
        }
        out.extend(parse(&v)?);
        let pages = v
            .get("result")
            .and_then(|r| r.get("pages"))
            .and_then(|p| p.as_u64())
            .unwrap_or(1);
        if page as u64 >= pages {
            break;
        }
        page += 1;
    }
    Ok(out)
}

/// Map an Eastmoney `RPTA_WEB_RATE` response to [`LprRow`]s. Malformed rows are skipped.
pub(crate) fn parse(resp: &Value) -> Result<Vec<LprRow>> {
    let data = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data".into(),
        })?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let date = item
            .get("TRADE_DATE")
            .and_then(|v| v.as_str())
            .filter(|s| s.len() >= 10)
            .map(|s| s[..10].to_string());
        let row = LprRow {
            date,
            lpr_1y: num_opt(item, "LPR1Y"),
            lpr_5y: num_opt(item, "LPR5Y"),
            rate_1: num_opt(item, "RATE_1"),
            rate_2: num_opt(item, "RATE_2"),
        };
        // Skip rows that carry no usable data at all.
        if row.date.is_none()
            && row.lpr_1y.is_none()
            && row.lpr_5y.is_none()
            && row.rate_1.is_none()
            && row.rate_2.is_none()
        {
            continue;
        }
        out.push(row);
    }
    Ok(out)
}

fn num_opt(item: &Value, k: &str) -> Option<f64> {
    match item.get(k) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_lpr_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lpr.json");
        let v: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let rows = parse(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date.as_deref(), Some("2024-01-22"));
        assert_eq!(rows[0].lpr_1y, Some(3.45));
        assert_eq!(rows[0].lpr_5y, Some(4.20));
        assert_eq!(rows[1].date.as_deref(), Some("2024-01-15"));
    }
}
