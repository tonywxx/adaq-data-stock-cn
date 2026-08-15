use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

const BASE: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";
const REPORT: &str = "RPT_IMP_INTRESTRATEN";

const MARKET_MAP: &[(&str, &str)] = &[
    ("上海银行同业拆借市场", "001"),
    ("中国银行同业拆借市场", "002"),
    ("伦敦银行同业拆借市场", "003"),
    ("欧洲银行同业拆借市场", "004"),
    ("香港银行同业拆借市场", "005"),
    ("新加坡银行同业拆借市场", "006"),
];

const SYMBOL_MAP: &[(&str, &str)] = &[
    ("Shibor人民币", "CNY"),
    ("Chibor人民币", "CNY"),
    ("Libor英镑", "GBP"),
    ("Libor欧元", "EUR"),
    ("Libor美元", "USD"),
    ("Libor日元", "JPY"),
    ("Euribor欧元", "EUR"),
    ("Hibor美元", "USD"),
    ("Hibor人民币", "CNH"),
    ("Hibor港币", "HKD"),
    ("Sibor星元", "SGD"),
    ("Sibor美元", "USD"),
];

const INDICATOR_MAP: &[(&str, &str)] = &[
    ("隔夜", "001"),
    ("1周", "101"),
    ("2周", "102"),
    ("3周", "103"),
    ("1月", "201"),
    ("2月", "202"),
    ("3月", "203"),
    ("4月", "204"),
    ("5月", "205"),
    ("6月", "206"),
    ("7月", "207"),
    ("8月", "208"),
    ("9月", "209"),
    ("10月", "210"),
    ("11月", "211"),
    ("1年", "301"),
];

/// Canonical interbank offered-rate row (Shibor/Chibor/Libor/Hibor/Sibor).
#[derive(Debug, Clone, serde::Serialize)]
pub struct InterbankRate {
    pub date: String,
    pub rate: Option<f64>,
    pub change: Option<f64>,
    pub source: &'static str,
}

/// Interbank offered rate from Eastmoney (`rate_interbank`, akshare `interest_rate` package).
///
/// `market`/`symbol`/`indicator` use akshare's Chinese label vocabulary; they are
/// mapped to Eastmoney codes via the ported `*_MAP` tables. Falls back across
/// paginated pages until all rows are collected.
pub async fn rate_interbank(
    client: &Client,
    market: &str,
    symbol: &str,
    indicator: &str,
) -> Result<Vec<InterbankRate>> {
    let market_code = map_lookup(MARKET_MAP, market, "market")?;
    let currency_code = map_lookup(SYMBOL_MAP, symbol, "symbol")?;
    let indicator_id = map_lookup(INDICATOR_MAP, indicator, "indicator")?;

    let mut out = Vec::new();
    let mut page: u32 = 1;
    loop {
        let page_s = page.to_string();
        let filter = format!(
            "(MARKET_CODE=\"{market_code}\")(CURRENCY_CODE=\"{currency_code}\")(INDICATOR_ID=\"{indicator_id}\")"
        );
        let params = [
            ("reportName", REPORT),
            (
                "columns",
                "REPORT_DATE,REPORT_PERIOD,IR_RATE,CHANGE_RATE,INDICATOR_ID,LATEST_RECORD,MARKET,MARKET_CODE,CURRENCY,CURRENCY_CODE",
            ),
            ("quoteColumns", ""),
            ("filter", filter.as_str()),
            ("pageNumber", page_s.as_str()),
            ("pageSize", "500"),
            ("sortTypes", "-1"),
            ("sortColumns", "REPORT_DATE"),
            ("source", "WEB"),
            ("client", "WEB"),
            ("p", page_s.as_str()),
            ("pageNo", page_s.as_str()),
            ("pageNum", page_s.as_str()),
        ];
        let v = client
            .get_json(
                SOURCE_EASTMONEY,
                "rate_interbank",
                BASE,
                &params,
            )
            .await?;
        let result = v
            .get("result")
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing result".into(),
            })?;
        let data = result
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing result.data".into(),
            })?;
        if data.is_empty() {
            break;
        }
        for item in data {
            out.push(InterbankRate {
                date: fstr(item, "REPORT_DATE"),
                rate: fnum(item, "IR_RATE"),
                change: fnum(item, "CHANGE_RATE"),
                source: SOURCE_EASTMONEY,
            });
        }
        let pages = result.get("pages").and_then(|p| p.as_u64()).unwrap_or(1);
        if page as u64 >= pages {
            break;
        }
        page += 1;
    }
    out.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(out)
}

fn map_lookup(map: &[(&str, &str)], key: &str, kind: &str) -> Result<String> {
    for (k, v) in map {
        if *k == key {
            return Ok((*v).to_string());
        }
    }
    Err(Error::InvalidParam(format!("unknown {kind}: {key}")))
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
    fn parses_interbank_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/rate_interbank_em.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        // Mirror the production parse path by reading result.data.
        let data = v.get("result").unwrap().get("data").unwrap().as_array().unwrap();
        let rows: Vec<InterbankRate> = data
            .iter()
            .map(|item| InterbankRate {
                date: fstr(item, "REPORT_DATE"),
                rate: fnum(item, "IR_RATE"),
                change: fnum(item, "CHANGE_RATE"),
                source: SOURCE_EASTMONEY,
            })
            .collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-03-01");
        assert_eq!(rows[0].rate, Some(1.85));
        assert_eq!(rows[0].change, Some(-0.05));
        assert_eq!(rows[1].date, "2024-03-04");
        assert_eq!(rows[1].rate, Some(1.83));
    }
}
