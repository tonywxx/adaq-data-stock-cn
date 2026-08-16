use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};
use crate::stock::spot::SpotQuote;

/// Static, well-known Eastmoney `ut` token — no JS signing required (ADR-0005).
const UT: &str = "bd1d9ddb04089700cf9c27f6f7426281";
const FIELDS: &str = "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f12,f13,f14,f15,f16,f17,f18,f20,f21";
/// Exchange filter: SH/SZ main boards + select boards (mirrors akshare `stock_zh_a_spot_em`).
const FS: &str = "m:0 t:6,m:0 t:80,m:1 t:2,m:1 t:23,m:0 t:81 s:2048";
const BASE: &str = "https://push2.eastmoney.com/api/qt/clist/get";
const PAGE_SIZE: u32 = 1000;

/// A-share real-time spot quotes from Eastmoney (`stock_zh_a_spot_em`).
///
/// Eastmoney paginates `clist/get`; we walk pages until `total` is covered.
pub async fn spot(client: &Client) -> Result<Vec<SpotQuote>> {
    let mut out = Vec::new();
    let mut pn: u32 = 1;
    loop {
        let pn_s = pn.to_string();
        let pz_s = PAGE_SIZE.to_string();
        let params = [
            ("pn", pn_s.as_str()),
            ("pz", pz_s.as_str()),
            ("po", "1"),
            ("np", "1"),
            ("ut", UT),
            ("fltt", "2"),
            ("invt", "2"),
            ("fid", "f12"),
            ("fs", FS),
            ("fields", FIELDS),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, "stock_zh_a_spot_em", BASE, &params)
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
        out.extend(parse_diff(&v)?);
        let total = v
            .get("data")
            .and_then(|d| d.get("total"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        if (pn as u64) * PAGE_SIZE as u64 >= total {
            break;
        }
        pn += 1;
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    }
    Ok(out)
}

pub(crate) fn parse_diff(resp: &Value) -> Result<Vec<SpotQuote>> {
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
        out.push(parse_item(item));
    }
    Ok(out)
}

fn parse_item(item: &Value) -> SpotQuote {
    SpotQuote {
        code: fstr(item, "f12"),
        name: fstr(item, "f14"),
        price: fnum(item, "f2"),
        pct_change: fnum(item, "f3"),
        change: fnum(item, "f4"),
        volume: fnum(item, "f5"),
        amount: fnum(item, "f6"),
        turnover_rate: fnum(item, "f8"),
        pe: fnum(item, "f9"),
        high: fnum(item, "f15"),
        low: fnum(item, "f16"),
        open: fnum(item, "f17"),
        pre_close: fnum(item, "f18"),
        total_mv: fnum(item, "f20"),
        float_mv: fnum(item, "f21"),
        source: SOURCE_EASTMONEY,
    }
}

fn fstr(item: &Value, k: &str) -> String {
    item.get(k)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_eastmoney_spot_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/stock_zh_a_spot_em.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_diff(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "600000");
        assert_eq!(rows[0].name, "浦发银行");
        assert_eq!(rows[0].price, Some(13.45));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].code, "000001");
        assert_eq!(rows[1].name, "平安银行");
        assert_eq!(rows[1].pct_change, Some(-1.20));
    }
}
