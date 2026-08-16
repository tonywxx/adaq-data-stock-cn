use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};
use crate::fund::{fnum, fstr};

const UT: &str = "bd1d9ddb04089700cf9c27f6f7426281";
const SPOT_URL: &str = "https://88.push2.eastmoney.com/api/qt/clist/get";
/// LOF boards: b:MK0404 (LOF-沪), b:MK0405, b:MK0406, b:MK0407.
const SPOT_FS: &str = "b:MK0404,b:MK0405,b:MK0406,b:MK0407";
const SPOT_FIELDS: &str = "f1,f2,f3,f4,f5,f6,f12,f13,f14,f15,f16,f17,f18";
const SPOT_PAGE_SIZE: u32 = 1000;

/// Canonical LOF real-time spot quote (akshare `fund_lof_spot_em`), Eastmoney.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LofSpotRow {
    pub code: String,
    pub name: String,
    pub price: Option<f64>,
    pub pct_change: Option<f64>,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub pre_close: Option<f64>,
    pub volume: Option<f64>,
    pub amount: Option<f64>,
    pub source: &'static str,
}

/// Real-time LOF spot quotes from Eastmoney (`fund_lof_spot_em`).
///
/// Replicates akshare's `clist/get` request (static `ut`, no JS signing).
/// Eastmoney paginates; we walk pages until `data.total` is covered.
pub async fn fund_lof_spot_em(client: &Client) -> Result<Vec<LofSpotRow>> {
    let mut out = Vec::new();
    let mut pn: u32 = 1;
    loop {
        let pn_s = pn.to_string();
        let pz_s = SPOT_PAGE_SIZE.to_string();
        let params = [
            ("pn", pn_s.as_str()),
            ("pz", pz_s.as_str()),
            ("po", "1"),
            ("np", "1"),
            ("ut", UT),
            ("fltt", "2"),
            ("invt", "2"),
            ("wbp2u", "|0|0|0|web"),
            ("fid", "f3"),
            ("fs", SPOT_FS),
            ("fields", SPOT_FIELDS),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, "fund_lof_spot_em", SPOT_URL, &params)
            .await?;
        let diff = v
            .get("data")
            .and_then(|d| d.get("diff"))
            .and_then(|d| d.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing data.diff".into(),
            })?;
        if diff.is_empty() {
            break;
        }
        out.extend(parse_spot(&v)?);
        let total = v
            .get("data")
            .and_then(|d| d.get("total"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        if (pn as u64) * SPOT_PAGE_SIZE as u64 >= total {
            break;
        }
        pn += 1;
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    }
    Ok(out)
}

pub(crate) fn parse_spot(resp: &Value) -> Result<Vec<LofSpotRow>> {
    let diff = resp
        .get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff".into(),
        })?;
    let mut out = Vec::with_capacity(diff.len());
    for item in diff {
        let code = fstr(item, "f12");
        if code.is_empty() {
            continue; // skip malformed rows
        }
        out.push(LofSpotRow {
            code,
            name: fstr(item, "f14"),
            price: fnum(item, "f2"),
            pct_change: fnum(item, "f3"),
            open: fnum(item, "f17"),
            high: fnum(item, "f15"),
            low: fnum(item, "f16"),
            pre_close: fnum(item, "f18"),
            volume: fnum(item, "f5"),
            amount: fnum(item, "f6"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_lof_spot_fixture() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fund_lof_spot_em.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_spot(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "166009");
        assert_eq!(rows[0].name, "中欧增强回报LOF");
        assert_eq!(rows[0].price, Some(1.23));
        assert_eq!(rows[0].pct_change, Some(0.82));
        assert_eq!(rows[0].open, Some(1.21));
        assert_eq!(rows[0].high, Some(1.25));
        assert_eq!(rows[0].low, Some(1.20));
        assert_eq!(rows[0].pre_close, Some(1.22));
        assert_eq!(rows[0].volume, Some(1_234_567.0));
        assert_eq!(rows[0].amount, Some(1_519_843.0));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].code, "160505");
        assert_eq!(rows[1].pct_change, Some(-1.10));
    }
}
