//! 百度股市通-外汇-行情榜单 (Baidu Finance FX quote ranking).
//!
//! Ports `akshare/fx/fx_quote_baidu.py` (`fx_quote_baidu`). Plain JSON GET
//! against `https://finance.pae.baidu.com/api/getforeignrank`.
//!
//! The upstream wraps the payload in a `Result` array; each element carries
//! `code` / `name` plus a `list` of `{name, value}` field dicts (akshare
//! reshapes these via a `pd.DataFrame` transpose into the 最新价 / 涨跌额 /
//! 涨跌幅 columns). We paginate with `pn` (start 0, step 20) / `rn=20`,
//! mirroring akshare's `while` loop, and stop when a page returns fewer than
//! 20 rows or `ResultCode != "0"`.
//!
//! ## Caveat — anti-bot `acs-token`
//! The upstream enforces a Baidu anti-bot `acs-token` (akshare's `token`
//! parameter, copied from the browser by the caller). We forward it as the
//! `acs-token` request header, empty by default to match akshare's
//! `token=""`. A live probe from this environment returned HTTP 403 without a
//! valid browser token; the port is structurally faithful (plain JSON GET +
//! optional token header) but the token must be supplied at call time to get
//! data in production.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE: &str = "baidu";
const BASE: &str = "https://finance.pae.baidu.com/api/getforeignrank";

/// One FX quote row from the Baidu Finance ranking board.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FxQuoteRow {
    /// 代码 (`code`).
    pub code: String,
    /// 名称 (`name`).
    pub name: String,
    /// 最新价 (`最新价`).
    pub latest_price: Option<f64>,
    /// 涨跌额 (`涨跌额`).
    pub change_amount: Option<f64>,
    /// 涨跌幅 (`涨跌幅`), stored as a fraction (e.g. `0.17%` → `0.0017`),
    /// matching akshare's `str.strip("%") / 100` normalization.
    pub change_pct: Option<f64>,
}

fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// Parse an optional JSON value into a `f64` (number or numeric string).
fn num_opt(v: Option<&Value>) -> Option<f64> {
    v.and_then(num)
}

/// Parse a percent string like `"0.17%"` into a fraction `0.0017`.
/// Non-percent numeric strings are parsed as-is; unparseable → `None`.
fn pct(v: &Value) -> Option<f64> {
    match v {
        Value::String(s) => {
            let t = s.trim().trim_end_matches('%').trim();
            t.parse::<f64>().ok().map(|x| x / 100.0)
        }
        other => num(other),
    }
}

/// Parse an optional percent JSON value into a fraction `f64`.
fn pct_opt(v: Option<&Value>) -> Option<f64> {
    v.and_then(pct)
}

/// Parse a single Baidu FX rank item (one `Result` element) into [`FxQuoteRow`].
pub(crate) fn parse_fx_quote_item(item: &Value) -> FxQuoteRow {
    let code = item
        .get("code")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let name = item
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let mut latest_price = None;
    let mut change_amount = None;
    let mut change_pct = None;
    if let Some(list) = item.get("list").and_then(|l| l.as_array()) {
        for field in list {
            let key = field.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let val = field.get("value");
            match key {
                "最新价" => latest_price = num_opt(val),
                "涨跌额" => change_amount = num_opt(val),
                "涨跌幅" => change_pct = pct_opt(val),
                _ => {}
            }
        }
    }
    FxQuoteRow {
        code,
        name,
        latest_price,
        change_amount,
        change_pct,
    }
}

/// Parse the full `Result` array of a `getforeignrank` response into rows.
pub(crate) fn parse_fx_quote(resp: &Value) -> Result<Vec<FxQuoteRow>> {
    let result = resp
        .get("Result")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing Result array".into(),
        })?;
    Ok(result.iter().map(parse_fx_quote_item).collect())
}

/// 百度股市通-外汇-行情榜单 (`https://finance.pae.baidu.com/api/getforeignrank`).
///
/// `symbol` maps `"美元"` → `dollar`, everything else (incl. `"人民币"`) →
/// `rmb`, mirroring akshare's `symbol_map`. `token` is the optional Baidu
/// anti-bot `acs-token` (empty by default).
pub async fn fx_quote_baidu(
    client: &Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<FxQuoteRow>> {
    let type_arg = if symbol == "美元" { "dollar" } else { "rmb" };
    let headers = [
        ("Referer", "https://finance.baidu.com/"),
        ("acs-token", token),
    ];

    let mut out: Vec<FxQuoteRow> = Vec::new();
    let mut pn: u32 = 0;
    loop {
        let resp = client
            .get_json_with_headers(
                SOURCE,
                "fx_quote_baidu",
                BASE,
                &[
                    ("type", type_arg),
                    ("pn", &pn.to_string()),
                    ("rn", "20"),
                    ("finClientType", "pc"),
                ],
                Some(&headers),
            )
            .await?;

        // akshare breaks the loop when ResultCode != "0".
        if resp.get("ResultCode").and_then(|v| v.as_str()) != Some("0") {
            break;
        }
        let rows = parse_fx_quote(&resp)?;
        let n = rows.len();
        out.extend(rows);
        // akshare stops once a page returns fewer than 20 rows.
        if n < 20 {
            break;
        }
        pn += 20;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-9,
            None => false,
        }
    }

    #[test]
    fn parses_fx_quote_rows() {
        let rows = parse_fx_quote(&fixture("fx_quote_baidu.json")).unwrap();
        assert_eq!(rows.len(), 2);

        assert_eq!(rows[0].code, "USD");
        assert_eq!(rows[0].name, "美元");
        assert!(approx(rows[0].latest_price, 7.2310));
        assert!(approx(rows[0].change_amount, 0.0123));
        assert!(approx(rows[0].change_pct, 0.0017));

        assert_eq!(rows[1].code, "HKD");
        assert!(approx(rows[1].change_amount, -0.0010));
        assert!(approx(rows[1].change_pct, -0.0011));
    }

    #[test]
    fn missing_result_array_is_error() {
        let bad = serde_json::json!({"ResultCode": "0", "Result": null});
        assert!(parse_fx_quote(&bad).is_err());
    }
}
