use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

use super::SOURCE_CHINAMONEY;

/// Canonical ChinaMoney repo fixing-rate row (FR* / FDR* series).
///
/// Both the historical (`repo_rate_hist`, returns all six series) and the
/// full-series CSV (`repo_rate_query`, returns three per `symbol`) normalizers
/// produce this type, so callers get a stable shape regardless of endpoint.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RepoRate {
    pub date: String,
    pub fr001: Option<f64>,
    pub fr007: Option<f64>,
    pub fr014: Option<f64>,
    pub fdr001: Option<f64>,
    pub fdr007: Option<f64>,
    pub fdr014: Option<f64>,
    pub source: &'static str,
}

impl RepoRate {
    fn new(date: String, source: &'static str) -> Self {
        Self {
            date,
            fr001: None,
            fr007: None,
            fr014: None,
            fdr001: None,
            fdr007: None,
            fdr014: None,
            source,
        }
    }
}

/// Convert akshare-style `YYYYMMDD` into `YYYY-MM-DD`.
pub(crate) fn fmt_date(s: &str) -> String {
    if s.len() == 8 {
        format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8])
    } else {
        s.to_string()
    }
}

/// Repo fixing-rate history from ChinaMoney (`rate.repo_rate_hist`).
///
/// POSTs to the `FrrHis` endpoint; akshare requires `start_date`/`end_date`
/// to sit within the same month.
pub async fn repo_rate_hist(client: &Client, start_date: &str, end_date: &str) -> Result<Vec<RepoRate>> {
    let url = "https://www.chinamoney.com.cn/ags/ms/cm-u-bk-currency/FrrHis";
    let sd = fmt_date(start_date);
    let ed = fmt_date(end_date);
    let params = [("lang", "CN"), ("startDate", sd.as_str()), ("endDate", ed.as_str())];
    let v = client
        .post_form_json(SOURCE_CHINAMONEY, "repo_rate_hist", url, &params, None)
        .await?;
    parse_repo_rate_hist(&v)
}

/// Full-series repo fixing-rate CSV from ChinaMoney (`rate.repo_rate_query`).
///
/// `symbol` selects the series: `"回购定盘利率"` → FR001/FR007/FR014,
/// `"银银间回购定盘利率"` → FDR001/FDR007/FDR014.
pub async fn repo_rate_query(client: &Client, symbol: &str) -> Result<Vec<RepoRate>> {
    let (url, fdr) = if symbol == "回购定盘利率" {
        (
            "https://www.chinamoney.com.cn/r/cms/www/chinamoney/data/currency/frr-chrt.csv",
            false,
        )
    } else if symbol == "银银间回购定盘利率" {
        (
            "https://www.chinamoney.com.cn/r/cms/www/chinamoney/data/currency/fdr-chrt.csv",
            true,
        )
    } else {
        return Err(Error::InvalidParam(format!("unknown symbol: {symbol}")));
    };
    let text = client
        .get_text(SOURCE_CHINAMONEY, "repo_rate_query", url, &[], None)
        .await?;
    parse_repo_rate_query_csv(&text, fdr)
}

pub(crate) fn parse_repo_rate_hist(resp: &Value) -> Result<Vec<RepoRate>> {
    let records = resp
        .get("records")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CHINAMONEY,
            message: "missing records".into(),
        })?;
    let mut out = Vec::with_capacity(records.len());
    for rec in records {
        let map = rec
            .get("frValueMap")
            .and_then(|m| m.as_object())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_CHINAMONEY,
                message: "missing frValueMap".into(),
            })?;
        let date = map
            .get("date")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let mut row = RepoRate::new(date, SOURCE_CHINAMONEY);
        row.fr001 = num(map.get("FR001"));
        row.fr007 = num(map.get("FR007"));
        row.fr014 = num(map.get("FR014"));
        row.fdr001 = num(map.get("FDR001"));
        row.fdr007 = num(map.get("FDR007"));
        row.fdr014 = num(map.get("FDR014"));
        out.push(row);
    }
    Ok(out)
}

pub(crate) fn parse_repo_rate_query_csv(text: &str, fdr: bool) -> Result<Vec<RepoRate>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(text.as_bytes());
    let mut out = Vec::new();
    for rec in rdr.records() {
        let rec = rec.map_err(|e| Error::Parse {
            endpoint: "repo_rate_query",
            message: e.to_string(),
        })?;
        if rec.len() < 4 {
            continue;
        }
        let date = rec.get(0).unwrap_or("").to_string();
        let a = rec.get(1).and_then(|s| s.parse::<f64>().ok());
        let b = rec.get(2).and_then(|s| s.parse::<f64>().ok());
        let c = rec.get(3).and_then(|s| s.parse::<f64>().ok());
        let mut row = RepoRate::new(date, SOURCE_CHINAMONEY);
        if fdr {
            row.fdr001 = a;
            row.fdr007 = b;
            row.fdr014 = c;
        } else {
            row.fr001 = a;
            row.fr007 = b;
            row.fr014 = c;
        }
        out.push(row);
    }
    Ok(out)
}


fn num(v: Option<&Value>) -> Option<f64> {
    v.and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> String {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
        std::fs::read_to_string(p).unwrap()
    }

    #[test]
    fn parses_repo_rate_hist_fixture() {
        let v: Value = serde_json::from_str(&fixture("rate_repo_rate_hist.json")).unwrap();
        let rows = parse_repo_rate_hist(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2023-10-09");
        assert_eq!(rows[0].fr001, Some(1.85));
        assert_eq!(rows[0].fr014, Some(2.20));
        assert_eq!(rows[0].fdr001, Some(1.83));
        assert_eq!(rows[0].fdr014, Some(2.18));
        assert_eq!(rows[0].source, "chinamoney");
        assert_eq!(rows[1].fr007, Some(2.05));
    }

    #[test]
    fn parses_repo_rate_query_csv_fixture() {
        let text = fixture("rate_repo_rate_query.csv");
        let rows = parse_repo_rate_query_csv(&text, false).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2023-10-09");
        assert_eq!(rows[0].fr001, Some(1.85));
        assert_eq!(rows[0].fr007, Some(2.10));
        assert_eq!(rows[0].fr014, Some(2.20));
        assert_eq!(rows[0].fdr001, None);
        assert_eq!(rows[1].fr001, Some(1.83));
    }

    #[test]
    fn parses_repo_rate_query_csv_fdr_variant() {
        let text = "2023-10-09,1.83,2.08,2.18\n2023-10-10,1.81,2.03,2.13\n";
        let rows = parse_repo_rate_query_csv(text, true).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].fdr001, Some(1.83));
        assert_eq!(rows[0].fdr014, Some(2.18));
        assert_eq!(rows[0].fr001, None);
    }

    #[test]
    fn fmt_date_pads() {
        assert_eq!(fmt_date("20230930"), "2023-09-30");
        assert_eq!(fmt_date("2023-09-30"), "2023-09-30");
    }
}
