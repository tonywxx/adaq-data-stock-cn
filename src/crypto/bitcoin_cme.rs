use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// Jin10 (金十数据) data center — public `X-App-Id`, no signing/secret required.
const SOURCE_JIN10: &str = "jin10";

const CME_URL: &str = "https://datacenter-api.jin10.com/reports/list";

/// Headers Jin10's public report API expects (no secret — `x-app-id` is the
/// same public token akshare ships).
const REPORT_HEADERS: &[(&str, &str)] = &[
    ("x-app-id", "rU6QIu7JHe2gOUeR"),
    ("x-version", "1.0.0"),
    ("referer", "https://datacenter.jin10.com/"),
    (
        "user-agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.106 Safari/537.36",
    ),
];

/// CME Bitcoin volume/open-interest report (akshare `crypto_bitcoin_cme`).
///
/// One row per product/contract type (期货/期权/看涨/看跌). `date` is the
/// report date in `YYYYMMDD` form; the upstream `data.date` is also captured.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CryptoCme {
    pub date: String,
    pub product: String,
    pub type_: String,
    pub electronic_contracts: Option<f64>,
    pub floor_contracts: Option<f64>,
    pub block_contracts: Option<f64>,
    pub volume: Option<f64>,
    pub open_interest: Option<f64>,
    pub oi_change: Option<f64>,
    pub source: &'static str,
}

/// Fetch the CME Bitcoin report for `date` (format `YYYYMMDD`, e.g. `20230830`).
pub async fn crypto_bitcoin_cme(client: &Client, date: &str) -> Result<Vec<CryptoCme>> {
    if date.len() != 8 || !date.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::InvalidParam(format!(
            "date must be YYYYMMDD, got: {date}"
        )));
    }
    let formatted = format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8]);
    let params = [
        ("category", "cme"),
        ("date", formatted.as_str()),
        ("attr_id", "4"),
    ];
    let text = client
        .get_text(
            SOURCE_JIN10,
            "crypto_bitcoin_cme",
            CME_URL,
            &params,
            Some(REPORT_HEADERS),
        )
        .await?;
    let v: Value = serde_json::from_str(&text).map_err(Error::Json)?;
    parse(&v)
}

/// Parse a `crypto_bitcoin_cme` response. `data.keys` is an array of
/// `{"name": ...}` column descriptors and `data.values` a parallel array of
/// row arrays. Columns are resolved by name so reordering upstream is safe;
/// rows without a product are skipped.
pub(crate) fn parse(resp: &Value) -> Result<Vec<CryptoCme>> {
    let data = resp.get("data").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_JIN10,
        message: "missing data".into(),
    })?;
    let date = data
        .get("date")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let names =
        data.get("keys")
            .and_then(|k| k.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_JIN10,
                message: "missing data.keys".into(),
            })?;
    let col: Vec<String> = names
        .iter()
        .filter_map(|n| {
            n.get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .collect();
    let values = data
        .get("values")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_JIN10,
            message: "missing data.values".into(),
        })?;

    let idx = |name: &str| col.iter().position(|c| c == name);
    let i_product = idx("商品");
    let i_type = idx("类型");
    let i_elec = idx("电子交易合约");
    let i_floor = idx("场内成交合约");
    let i_block = idx("场外成交合约");
    let i_vol = idx("成交量");
    let i_oi = idx("未平仓合约");
    let i_chg = idx("持仓变化");

    let mut out = Vec::with_capacity(values.len());
    for row in values {
        let cells = match row.as_array() {
            Some(c) => c,
            None => continue,
        };
        let at = |i: Option<usize>| i.and_then(|p| cells.get(p));
        let product = match at(i_product).and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => continue,
        };
        let type_ = at(i_type)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        out.push(CryptoCme {
            date: date.clone(),
            product,
            type_,
            electronic_contracts: at(i_elec).and_then(num),
            floor_contracts: at(i_floor).and_then(num),
            block_contracts: at(i_block).and_then(num),
            volume: at(i_vol).and_then(num),
            open_interest: at(i_oi).and_then(num),
            oi_change: at(i_chg).and_then(num),
            source: SOURCE_JIN10,
        });
    }
    Ok(out)
}

fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_bitcoin_cme_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/crypto_bitcoin_cme.json");
        let v: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let rows = parse(&v).unwrap();
        // fixture includes one malformed row (empty product) that must be skipped
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2023-08-30");
        assert_eq!(rows[0].product, "比特币");
        assert_eq!(rows[0].type_, "期货");
        assert_eq!(rows[0].electronic_contracts, Some(7895.0));
        assert_eq!(rows[0].volume, Some(8261.0));
        assert_eq!(rows[0].open_interest, Some(15408.0));
        assert_eq!(rows[0].oi_change, Some(-764.0));
        assert_eq!(rows[0].block_contracts, Some(366.0));
        assert_eq!(rows[0].source, "jin10");
        assert_eq!(rows[1].product, "微型比特币");
        assert_eq!(rows[1].electronic_contracts, Some(7818.0));
        assert_eq!(rows[1].oi_change, Some(-425.0));
    }
}
