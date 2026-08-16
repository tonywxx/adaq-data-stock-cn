//! Currency & FX data. Ports `akshare/currency/currency.py`,
//! `akshare/currency/currency_china_bank_sina.py`, `akshare/currency/currency_safe.py`,
//! `akshare/fx/currency_investing.py`, `akshare/fx/fx_c_swap_cm.py` and
//! `akshare/fx/fx_quote_baidu.py`.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `currency_latest` | `currency/currency.py:14` | currencyscoop v1/latest; needs `api_key` (paid account). |
//! | `currency_history` | `currency/currency.py:39` | currencyscoop v1/historical; needs `api_key`. |
//! | `currency_time_series` | `currency/currency.py:66` | currencyscoop v1/timeseries; needs `api_key`. |
//! | `currency_currencies` | `currency/currency.py:107` | currencyscoop v1/currencies; needs `api_key`. |
//! | `currency_convert` | `currency/currency.py:126` | currencyscoop v1/convert; needs `api_key`. |
//! | `fx_c_swap_cm` | `fx/fx_c_swap_cm.py:25` | ChinaMoney C-Swap fixing curve (JSON POST). |
//!
//! ## DEFERRED
//!
//! * `currency_boc_sina` (`currency/currency_china_bank_sina.py:57`) — HTML
//!   scraping: the Sina endpoint returns HTML tables parsed via `pd.read_html`
//!   plus a paginated loop and an initial `<select>` option scrape
//!   (`_currency_boc_sina_map`). No JSON API; requires HTML-table scraping.
//! * `currency_boc_safe` (`currency/currency_safe.py:18`) — HTML scraping +
//!   Excel download: BeautifulSoup to locate an `<a href>` then
//!   `pd.read_excel(url)` (remote `.xls` download) and `pd.read_html` on a
//!   POSTed query. Requires Excel/ZIP download + HTML scrape.
//! * `currency_pair_map` (`fx/currency_investing.py:16`) — HTML scraping: the
//!   cn.investing.com `Service/region` & `Service/currency` endpoints return
//!   HTML parsed with BeautifulSoup (`soup.find_all("a")`), and the site is
//!   anti-bot protected. Requires HTML scraping.
//! * `fx_quote_baidu` (`fx/fx_quote_baidu.py:13`) — requires an `acs-token`
//!   anti-bot header (`headers["acs-token"]`). A live `curl` of the endpoint
//!   returns `{"ResultCode":"403","Result":[]}` without the token, so it cannot
//!   be fetched without a browser-derived token (third-party JS-signed auth).

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_CURRENCYSCOOP: &str = "currencyscoop";
const SOURCE_CHINAMONEY: &str = "chinamoney";

const BASE_CURRENCYSCOOP: &str = "https://api.currencyscoop.com/v1";
const CHINAMONEY_C_SWAP_URL: &str =
    "https://www.chinamoney.org.cn/r/cms/www/chinamoney/data/fx/fx-c-sw-curv-USD.CNY.json";

fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(str::to_string)
}

/// Render any JSON scalar/value as a plain string for the `currency_convert`
/// (item, value) table (mirrors akshare's `pd.Series(...).reset_index()`).
fn val_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

// ===========================================================================
// currencyscoop — latest / history / currencies / convert / time_series
// ===========================================================================

/// A single currency rate observation (used by `currency_latest` & `currency_history`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CurrencyRateRow {
    /// Base currency (currencyscoop `base`).
    pub base: String,
    /// Quote date (currencyscoop `response.date`).
    pub date: String,
    /// Currency code, e.g. `CNY`.
    pub currency: String,
    /// Exchange rate (1 unit of `base` = `rate` units of `currency`).
    pub rate: Option<f64>,
}

/// Parse `response.{date,base,rates}` into typed rows.
/// `response.rates` maps currency code -> rate (mirrors akshare's intent:
/// `from_dict(response)` + `rename(index="currency")`).
pub(crate) fn parse_currency_rates(resp: &Value) -> Result<Vec<CurrencyRateRow>> {
    let response = resp.get("response").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_CURRENCYSCOOP,
        message: "missing response".into(),
    })?;
    let date = fstr(response, "date").unwrap_or_default();
    let base = fstr(response, "base").unwrap_or_default();
    let rates = response
        .get("rates")
        .and_then(|v| v.as_object())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CURRENCYSCOOP,
            message: "missing response.rates".into(),
        })?;
    let mut out = Vec::with_capacity(rates.len());
    for (currency, rate) in rates {
        out.push(CurrencyRateRow {
            base: base.clone(),
            date: date.clone(),
            currency: currency.clone(),
            rate: rate.as_f64(),
        });
    }
    Ok(out)
}

/// 最新外汇牌价 (currencyscoop v1/latest). `api_key` is required by the upstream
/// (paid CurrencyBeacon account). Defaults `base="USD"`.
pub async fn currency_latest(
    client: &Client,
    base: &str,
    symbols: &str,
    api_key: &str,
) -> Result<Vec<CurrencyRateRow>> {
    let url = format!("{BASE_CURRENCYSCOOP}/latest");
    let v = client
        .get_json(
            SOURCE_CURRENCYSCOOP,
            "currency_latest",
            &url,
            &[("base", base), ("symbols", symbols), ("api_key", api_key)],
        )
        .await?;
    parse_currency_rates(&v)
}

/// 历史外汇牌价 (currencyscoop v1/historical) for a single `date`, e.g. `2023-02-03`.
pub async fn currency_history(
    client: &Client,
    base: &str,
    date: &str,
    symbols: &str,
    api_key: &str,
) -> Result<Vec<CurrencyRateRow>> {
    let url = format!("{BASE_CURRENCYSCOOP}/historical");
    let v = client
        .get_json(
            SOURCE_CURRENCYSCOOP,
            "currency_history",
            &url,
            &[
                ("base", base),
                ("date", date),
                ("symbols", symbols),
                ("api_key", api_key),
            ],
        )
        .await?;
    parse_currency_rates(&v)
}

/// A single point in a currency time series (used by `currency_time_series`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CurrencyTimeSeriesRow {
    /// Observation date, e.g. `2023-02-03`.
    pub date: String,
    /// Currency code.
    pub currency: String,
    /// Exchange rate on that date.
    pub rate: Option<f64>,
}

/// Parse `response` keyed by date -> {currency -> rate} (mirrors akshare's
/// `from_dict(response).T` producing one row per (date, currency)).
pub(crate) fn parse_currency_time_series(resp: &Value) -> Result<Vec<CurrencyTimeSeriesRow>> {
    let response = resp
        .get("response")
        .and_then(|v| v.as_object())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CURRENCYSCOOP,
            message: "missing response".into(),
        })?;
    let mut out = Vec::new();
    for (date, inner) in response {
        let Some(rates) = inner.as_object() else {
            continue;
        };
        for (currency, rate) in rates {
            out.push(CurrencyTimeSeriesRow {
                date: date.clone(),
                currency: currency.clone(),
                rate: rate.as_f64(),
            });
        }
    }
    Ok(out)
}

/// 外汇时间序列 (currencyscoop v1/timeseries) between `start_date` and `end_date`.
pub async fn currency_time_series(
    client: &Client,
    base: &str,
    start_date: &str,
    end_date: &str,
    symbols: &str,
    api_key: &str,
) -> Result<Vec<CurrencyTimeSeriesRow>> {
    let url = format!("{BASE_CURRENCYSCOOP}/timeseries");
    let v = client
        .get_json(
            SOURCE_CURRENCYSCOOP,
            "currency_time_series",
            &url,
            &[
                ("base", base),
                ("start_date", start_date),
                ("end_date", end_date),
                ("symbols", symbols),
                ("api_key", api_key),
            ],
        )
        .await?;
    parse_currency_time_series(&v)
}

/// A single supported currency (used by `currency_currencies`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CurrencyInfoRow {
    /// ISO code, e.g. `USD`.
    pub code: String,
    /// English name.
    pub name: String,
    /// Symbol, e.g. `$`.
    pub symbol: Option<String>,
    /// Decimal units.
    pub decimal_units: Option<f64>,
}

/// Parse `response` (array of currency objects) into typed rows.
pub(crate) fn parse_currency_currencies(resp: &Value) -> Result<Vec<CurrencyInfoRow>> {
    let arr = resp
        .get("response")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CURRENCYSCOOP,
            message: "missing response array".into(),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let Some(code) = fstr(item, "code") else {
            continue;
        };
        out.push(CurrencyInfoRow {
            code,
            name: fstr(item, "name").unwrap_or_default(),
            symbol: fstr(item, "symbol"),
            decimal_units: item.get("decimal_units").and_then(|v| v.as_f64()),
        });
    }
    Ok(out)
}

/// 支持的货币列表 (currencyscoop v1/currencies). `c_type` is `fiat` (only one returning data).
pub async fn currency_currencies(
    client: &Client,
    c_type: &str,
    api_key: &str,
) -> Result<Vec<CurrencyInfoRow>> {
    let url = format!("{BASE_CURRENCYSCOOP}/currencies");
    let v = client
        .get_json(
            SOURCE_CURRENCYSCOOP,
            "currency_currencies",
            &url,
            &[("type", c_type), ("api_key", api_key)],
        )
        .await?;
    parse_currency_currencies(&v)
}

/// A single (field, value) pair of a convert result (mirrors akshare's
/// `pd.Series(response).reset_index()` -> columns [item, value]).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CurrencyConvertRow {
    /// Field name, e.g. `amount`, `rate`, `value`.
    pub item: String,
    /// Field value rendered as a string.
    pub value: String,
}

/// Parse the `response` object of `v1/convert` into (item, value) rows.
pub(crate) fn parse_currency_convert(resp: &Value) -> Result<Vec<CurrencyConvertRow>> {
    let response = resp
        .get("response")
        .and_then(|v| v.as_object())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CURRENCYSCOOP,
            message: "missing response".into(),
        })?;
    let mut out = Vec::with_capacity(response.len());
    for (item, value) in response {
        out.push(CurrencyConvertRow {
            item: item.clone(),
            value: val_to_string(value),
        });
    }
    Ok(out)
}

/// 货币兑换 (currencyscoop v1/convert): convert `amount` of `base` into `to`.
pub async fn currency_convert(
    client: &Client,
    base: &str,
    to: &str,
    amount: &str,
    api_key: &str,
) -> Result<Vec<CurrencyConvertRow>> {
    let url = format!("{BASE_CURRENCYSCOOP}/convert");
    let v = client
        .get_json(
            SOURCE_CURRENCYSCOOP,
            "currency_convert",
            &url,
            &[
                ("from", base),
                ("to", to),
                ("amount", amount),
                ("api_key", api_key),
            ],
        )
        .await?;
    parse_currency_convert(&v)
}

// ===========================================================================
// ChinaMoney — C-Swap fixing curve (fx_c_swap_cm.py)
// ===========================================================================

/// A single tenor on the ChinaMoney USD.CNY C-Swap fixing curve.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FxCSwapCmRow {
    /// 日期时间 (curveTime), e.g. `2026-08-14 16:30:00.0`.
    pub curve_time: String,
    /// 期限品种 (tenor), e.g. `ON`, `1W`.
    pub tenor: String,
    /// 掉期点(Pips) (swapPnt).
    pub swap_pnt: Option<f64>,
    /// 全价汇率 (swapAllPrc).
    pub swap_all_prc: Option<f64>,
    /// 掉期点数据源 (dataSource).
    pub data_source: String,
}

/// Parse the `records` array of the ChinaMoney C-Swap curve response.
pub(crate) fn parse_fx_c_swap_cm(resp: &Value) -> Result<Vec<FxCSwapCmRow>> {
    let arr = resp
        .get("records")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CHINAMONEY,
            message: "missing records".into(),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(FxCSwapCmRow {
            curve_time: fstr(item, "curveTime").unwrap_or_default(),
            tenor: fstr(item, "tenor").unwrap_or_default(),
            swap_pnt: item.get("swapPnt").and_then(|v| v.as_f64()),
            swap_all_prc: item.get("swapAllPrc").and_then(|v| v.as_f64()),
            data_source: fstr(item, "dataSource").unwrap_or_default(),
        });
    }
    Ok(out)
}

/// 中国外汇交易中心-外汇掉期 C-Swap 定盘曲线 (ChinaMoney `fx-c-sw-curv-USD.CNY.json`,
/// `fx/fx_c_swap_cm.py:25`). Pure JSON POST; `t` is a cache-buster millisecond
/// timestamp and does not affect the returned curve.
pub async fn fx_c_swap_cm(client: &Client) -> Result<Vec<FxCSwapCmRow>> {
    let v = client
        .post_form_json(
            SOURCE_CHINAMONEY,
            "fx_c_swap_cm",
            CHINAMONEY_C_SWAP_URL,
            &[("t", "0")],
            None,
        )
        .await?;
    parse_fx_c_swap_cm(&v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Value {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    #[test]
    fn parse_currency_latest_ok() {
        let rows = parse_currency_rates(&fixture("currency_latest.json")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].base, "USD");
        assert_eq!(rows[0].date, "2023-02-03");
        assert_eq!(rows[0].currency, "CNY");
        assert!(approx(rows[0].rate, 6.85));
        assert!(approx(rows[2].rate, 130.5));
    }

    #[test]
    fn parse_currency_history_ok() {
        let rows = parse_currency_rates(&fixture("currency_history.json")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[1].currency, "GBP");
        assert!(approx(rows[1].rate, 0.81));
    }

    #[test]
    fn parse_currency_time_series_ok() {
        let rows = parse_currency_time_series(&fixture("currency_time_series.json")).unwrap();
        assert_eq!(rows.len(), 6);
        assert_eq!(rows[0].date, "2023-02-03");
        assert_eq!(rows[0].currency, "CNY");
        assert!(approx(rows[0].rate, 6.85));
        assert_eq!(rows[5].date, "2023-02-05");
        assert!(approx(rows[5].rate, 0.94));
    }

    #[test]
    fn parse_currency_currencies_ok() {
        let rows = parse_currency_currencies(&fixture("currency_currencies.json")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].code, "USD");
        assert_eq!(rows[0].name, "US Dollar");
        assert_eq!(rows[0].symbol, Some("$".into()));
        assert!(approx(rows[0].decimal_units, 2.0));
        assert_eq!(rows[2].code, "EUR");
    }

    #[test]
    fn parse_currency_convert_ok() {
        let rows = parse_currency_convert(&fixture("currency_convert.json")).unwrap();
        assert_eq!(rows.len(), 6);
        let rate = rows.iter().find(|r| r.item == "rate").unwrap();
        assert_eq!(rate.value, "6.85");
        let to = rows.iter().find(|r| r.item == "to").unwrap();
        assert_eq!(to.value, "CNY");
    }

    #[test]
    fn parse_fx_c_swap_cm_ok() {
        let rows = parse_fx_c_swap_cm(&fixture("fx_c_swap_cm.json")).unwrap();
        assert_eq!(rows.len(), 12);
        assert_eq!(rows[0].tenor, "ON");
        assert!(approx(rows[0].swap_pnt, -14.1));
        assert!(approx(rows[0].swap_all_prc, 6.7439));
        assert_eq!(rows[0].data_source, "报价数据");
        assert_eq!(rows[3].tenor, "1W");
    }
}
