use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// Jin10 (金十数据) data center — public `X-App-Id`, no signing/secret required.
const SOURCE_JIN10: &str = "jin10";

const HOLD_URL: &str = "https://datacenter-api.jin10.com/bitcoin_treasuries/list";

const REPORT_HEADERS: &[(&str, &str)] = &[("x-app-id", "lnFP5lxse24wPgtY"), ("x-version", "1.0.0")];

/// Bitcoin treasury-holdings report (akshare `crypto_bitcoin_hold_report`).
///
/// One row per holder (company / entity). Numeric fields may be `null` upstream
/// (e.g. `market_cap`); these map to `None`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CryptoHold {
    pub code: String,
    pub company: String,
    pub country: String,
    pub market_cap: Option<f64>,
    pub btc_pct_market_cap: Option<f64>,
    pub cost_basis: Option<f64>,
    pub hold_pct: Option<f64>,
    pub hold_amount: Option<f64>,
    pub hold_value: Option<f64>,
    pub report_date: String,
    pub announcement: String,
    pub filings_url: String,
    pub category: String,
    pub multiple: Option<f64>,
    pub cost_basis_prefix: String,
    pub company_cn: String,
    pub source: &'static str,
}

/// Fetch the Bitcoin holdings report from Jin10 (`crypto_bitcoin_hold_report`).
pub async fn crypto_bitcoin_hold_report(client: &Client) -> Result<Vec<CryptoHold>> {
    let text = client
        .get_text(
            SOURCE_JIN10,
            "crypto_bitcoin_hold_report",
            HOLD_URL,
            &[],
            Some(REPORT_HEADERS),
        )
        .await?;
    let v: Value = serde_json::from_str(&text).map_err(Error::Json)?;
    parse(&v)
}

/// Parse a `crypto_bitcoin_hold_report` response. `data.keys` is an array of
/// column-name strings and `data.values` a parallel array of row arrays. Rows
/// without a `代码` (code) are skipped.
pub(crate) fn parse(resp: &Value) -> Result<Vec<CryptoHold>> {
    let data = resp.get("data").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_JIN10,
        message: "missing data".into(),
    })?;
    let names =
        data.get("keys")
            .and_then(|k| k.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_JIN10,
                message: "missing data.keys".into(),
            })?;
    let col: Vec<String> = names
        .iter()
        .filter_map(|n| n.as_str().map(|s| s.to_string()))
        .collect();
    let values = data
        .get("values")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_JIN10,
            message: "missing data.values".into(),
        })?;

    let idx = |name: &str| col.iter().position(|c| c == name);
    let i_code = idx("代码");
    let i_company = idx("公司");
    let i_country = idx("国家地区");
    let i_mcap = idx("市值");
    let i_btc_pct = idx("比特币占比公司市值");
    let i_cost = idx("持仓成本");
    let i_hold_pct = idx("持仓占比");
    let i_hold_amt = idx("持仓量");
    let i_hold_val = idx("当日持仓市值");
    let i_report_date = idx("报告日期");
    let i_announcement = idx("购买公告/文件");
    let i_filings = idx("filings_url");
    let i_category = idx("分类");
    let i_multiple = idx("倍数");
    let i_cost_prefix = idx("cost_basis_prefix");
    let i_company_cn = idx("公司中文名");

    let mut out = Vec::with_capacity(values.len());
    for row in values {
        let cells = match row.as_array() {
            Some(c) => c,
            None => continue,
        };
        let at = |i: Option<usize>| i.and_then(|p| cells.get(p));
        let code = match at(i_code).and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => continue,
        };
        out.push(CryptoHold {
            code,
            company: at(i_company).and_then(str_val).unwrap_or_default(),
            country: at(i_country).and_then(str_val).unwrap_or_default(),
            market_cap: at(i_mcap).and_then(num),
            btc_pct_market_cap: at(i_btc_pct).and_then(num),
            cost_basis: at(i_cost).and_then(num),
            hold_pct: at(i_hold_pct).and_then(num),
            hold_amount: at(i_hold_amt).and_then(num),
            hold_value: at(i_hold_val).and_then(num),
            report_date: at(i_report_date).and_then(str_val).unwrap_or_default(),
            announcement: at(i_announcement).and_then(str_val).unwrap_or_default(),
            filings_url: at(i_filings).and_then(str_val).unwrap_or_default(),
            category: at(i_category).and_then(str_val).unwrap_or_default(),
            multiple: at(i_multiple).and_then(num),
            cost_basis_prefix: at(i_cost_prefix).and_then(str_val).unwrap_or_default(),
            company_cn: at(i_company_cn).and_then(str_val).unwrap_or_default(),
            source: SOURCE_JIN10,
        });
    }
    Ok(out)
}

fn str_val(v: &Value) -> Option<String> {
    v.as_str().map(|s| s.to_string())
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
    fn parses_bitcoin_hold_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/crypto_bitcoin_hold_report.json");
        let v: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let rows = parse(&v).unwrap();
        // fixture includes one malformed row (empty code) that must be skipped
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "MSTR:NADQ");
        assert_eq!(rows[0].company, "MicroStrategy");
        assert_eq!(rows[0].country, "美国");
        assert_eq!(rows[0].hold_amount, Some(152333.0));
        assert_eq!(rows[0].hold_value, Some(4624823786.68));
        assert_eq!(rows[0].hold_pct, Some(0.725));
        assert_eq!(rows[0].btc_pct_market_cap, None);
        assert_eq!(rows[0].market_cap, None);
        assert_eq!(rows[0].filings_url, "https://example.com/mstr.pdf");
        assert_eq!(rows[0].company_cn, "");
        assert_eq!(rows[0].source, "jin10");
        assert_eq!(rows[1].code, "MARA:NADQ");
        assert_eq!(rows[1].company, "Marathon Digital Holdings Inc");
        assert_eq!(rows[1].company_cn, "美国加密货币挖矿公司");
    }
}
