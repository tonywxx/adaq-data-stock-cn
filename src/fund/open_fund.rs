use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

const BASE: &str = "https://fund.eastmoney.com/pingzhongdata";

/// Which JS variable (and value shape) backs a given akshare indicator.
pub(crate) enum NavKind {
    /// `Data_netWorthTrend`: objects `{x, y, equityReturn, ...}`.
    NetWorthTrend,
    /// `Data_ACWorthTrend` / `Data_millionCopiesIncome` / `Data_sevenDaysYearIncome`:
    /// `[x, y]` tuples. No per-row percentage.
    Tuple,
}

/// Canonical open-end fund NAV history (akshare `fund_open_fund_info`), Eastmoney.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OpenFundNavRow {
    pub symbol: String,
    /// NAV date, `YYYY-MM-DD` (derived from the upstream millisecond timestamp).
    pub date: String,
    /// 单位净值 / 累计净值 / 每万份收益 / 7日年化 (per indicator).
    pub nav: Option<f64>,
    /// 日增长率 — only present for the `单位净值走势` indicator.
    pub pct_change: Option<f64>,
    pub source: &'static str,
}

/// Open-end fund NAV history from Eastmoney (`fund_open_fund_info`).
///
/// The upstream is a JS file (`pingzhongdata/{symbol}.js`) that embeds several
/// JSON arrays. We fetch it as text and extract the requested array **without**
/// executing JS, then normalize. `indicator` selects the series:
/// `"单位净值走势"` (default), `"累计净值走势"`, `"每万份收益"`, `"7日年化收益率"`.
/// `period` is accepted for API parity but does not change the fetch (the JS
/// already carries the full history).
pub async fn fund_open_fund_info(
    client: &Client,
    symbol: &str,
    indicator: &str,
    _period: &str,
) -> Result<Vec<OpenFundNavRow>> {
    let (var, kind) = indicator_kind(indicator)?;
    let url = format!("{BASE}/{symbol}.js");
    let text = client
        .get_text(SOURCE_EASTMONEY, "fund_open_fund_info", &url, &[], None)
        .await?;
    let arr_text = extract_js_array(&text, var)?;
    let v: Value = serde_json::from_str(&arr_text).map_err(|e| Error::Parse {
        endpoint: "fund_open_fund_info",
        message: format!("array parse failed: {e}"),
    })?;
    parse_nav(&v, symbol, kind)
}

pub(crate) fn parse_nav(resp: &Value, symbol: &str, kind: NavKind) -> Result<Vec<OpenFundNavRow>> {
    let arr = resp
        .as_array()
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "expected a JSON array".into(),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        // Object form (net-worth trend): {x, y, equityReturn, ...}
        if let Some(obj) = item.as_object() {
            let x = match obj.get("x").and_then(|v| v.as_i64()) {
                Some(x) => x,
                None => continue, // skip malformed rows
            };
            let nav = obj.get("y").and_then(|v| v.as_f64());
            let pct = match kind {
                NavKind::NetWorthTrend => obj.get("equityReturn").and_then(|v| v.as_f64()),
                NavKind::Tuple => None,
            };
            out.push(OpenFundNavRow {
                symbol: symbol.to_string(),
                date: ms_to_date(x),
                nav,
                pct_change: pct,
                source: SOURCE_EASTMONEY,
            });
        } else if let Some(tuple) = item.as_array() {
            // Tuple form (acc-worth / million-copies / 7-day): [x, y]
            let x = match tuple.first().and_then(|v| v.as_i64()) {
                Some(x) => x,
                None => continue,
            };
            let nav = tuple.get(1).and_then(|v| v.as_f64());
            out.push(OpenFundNavRow {
                symbol: symbol.to_string(),
                date: ms_to_date(x),
                nav,
                pct_change: None,
                source: SOURCE_EASTMONEY,
            });
        }
        // skip anything else
    }
    Ok(out)
}

/// Map an akshare indicator name to its embedded JS variable + value shape.
fn indicator_kind(indicator: &str) -> Result<(&'static str, NavKind)> {
    match indicator {
        "单位净值走势" => Ok(("Data_netWorthTrend", NavKind::NetWorthTrend)),
        "累计净值走势" => Ok(("Data_ACWorthTrend", NavKind::Tuple)),
        "每万份收益" => Ok(("Data_millionCopiesIncome", NavKind::Tuple)),
        "7日年化收益率" => Ok(("Data_sevenDaysYearIncome", NavKind::Tuple)),
        other => Err(Error::InvalidParam(format!(
            "unsupported fund_open_fund_info indicator: {other}"
        ))),
    }
}

/// Extract the JSON array assigned to `var <name> = [...]` from a JS text blob.
fn extract_js_array(text: &str, var: &str) -> Result<String> {
    let start = text.find(var).ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: format!("js variable {var} not found"),
    })?;
    let after = &text[start + var.len()..];
    let eq = after.find('=').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: format!("'=' not found after {var}"),
    })?;
    let bracket = after[eq + 1..].find('[').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: format!("'[' not found after {var} ="),
    })?;
    let open = start + var.len() + eq + 1 + bracket;
    let mut depth: i64 = 0;
    let mut end = None;
    for (i, c) in text[open..].char_indices() {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(open + i + 1);
                    break;
                }
            }
            _ => {}
        }
    }
    let end = end.ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: format!("unbalanced brackets in {var}"),
    })?;
    Ok(text[open..end].to_string())
}

/// Convert a Unix millisecond timestamp to a `YYYY-MM-DD` UTC date string.
fn ms_to_date(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_open_fund_nav_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/fund_open_fund_info.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_nav(&v, "710001", NavKind::NetWorthTrend).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "710001");
        assert_eq!(rows[0].date, "2020-01-01");
        assert_eq!(rows[0].nav, Some(1.0));
        assert_eq!(rows[0].pct_change, None);
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].date, "2020-01-02");
        assert_eq!(rows[1].nav, Some(1.0123));
        assert_eq!(rows[1].pct_change, Some(1.23));
    }
}
