//! Sina main-continuous futures contracts (`futures_main` / `futures_display`).
//!
//! Ports the Sina-backed helpers from akshare's `futures_derivative/futures_index_sina.py`:
//! - `futures_main`    ← `futures_main_sina` (主力连续日数据, JSONP via `InnerFuturesNewService.getDailyKLine`)
//! - `futures_display` ← `futures_display_main_sina` (主力连续合约品种一览表, `Market_Center.getHQFuturesData`)
//!
//! Both endpoints are plain HTTP/JSONP (no JS signing), so they are source-resilient.
//! `futures_rule` (an HTML table) is intentionally **skipped** — it is not portable
//! offline and cannot be parsed without a DOM/HTML engine.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_SINA};
use crate::core::error::{Error, Result};

/// `Referer` expected by Sina's futures endpoints.
const SINA_HEADERS: &[(&str, &str)] = &[("Referer", "https://finance.sina.com.cn/futuremarket/")];

// ---------------------------------------------------------------------------
// futures_main  (akshare `futures_main_sina`)
// ---------------------------------------------------------------------------

/// Sina JSONP endpoint for a main-continuous daily kline.
const MAIN_KLINE_URL: &str = "https://stock2.finance.sina.com.cn/futures/api/jsonp.php/var%20_SYMBOL_DATE=/InnerFuturesNewService.getDailyKLine";

/// Upstream hardcodes a cache-busting `trade_date` (`20210817`, formatted as
/// `2021_08_17` in the callback). Kept faithful to akshare rather than guessing
/// "today" — the value is only used to build the JSONP callback name.
const MAIN_TRADE_DATE: &str = "20210817";

/// One day of a Sina main-continuous futures kline (`futures_main_sina`).
///
/// Mirrors akshare's columns: 日期, 开盘价, 最高价, 最低价, 收盘价, 成交量,
/// 持仓量, 动态结算价. The queried `symbol` is *not* part of the upstream
/// payload (akshare's output has no symbol column either), so it is omitted here.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesMainRow {
    /// 日期 (akshare `日期`, ISO `YYYY-MM-DD`)
    pub date: String,
    /// 开盘价 (akshare `开盘价`)
    pub open: Option<f64>,
    /// 最高价 (akshare `最高价`)
    pub high: Option<f64>,
    /// 最低价 (akshare `最低价`)
    pub low: Option<f64>,
    /// 收盘价 (akshare `收盘价`)
    pub close: Option<f64>,
    /// 成交量 (akshare `成交量`)
    pub volume: Option<f64>,
    /// 持仓量 (akshare `持仓量`)
    pub open_interest: Option<f64>,
    /// 动态结算价 (akshare `动态结算价`)
    pub settle_price: Option<f64>,
    /// Data origin (`sina`).
    pub source: &'static str,
}

/// Main-continuous daily kline for a Chinese futures contract (Sina `futures_main_sina`).
///
/// `symbol` is the Sina main-continuous code (e.g. `"V0"`, `"CF0"`) — obtainable
/// from [`futures_display`]. `start_date` / `end_date` are `YYYYMMDD` bounds and
/// are applied by slicing on the ISO date string, matching akshare's `df[start:end]`.
pub async fn futures_main(
    client: &Client,
    symbol: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<FuturesMainRow>> {
    let start = norm_date(start_date)?;
    let end = norm_date(end_date)?;

    // Build the JSONP callback name exactly as akshare does: `var _<symbol><YYYY_MM_DD>=`.
    let td = MAIN_TRADE_DATE;
    let td_fmt = format!("{}_{}_{}", &td[..4], &td[4..6], &td[6..]);
    let url = MAIN_KLINE_URL.replace("SYMBOL_DATE", &format!("{}{}", symbol, td_fmt));

    let params = [("symbol", symbol), ("_", td_fmt.as_str())];
    let text = client
        .get_text(
            SOURCE_SINA,
            "futures_main",
            url.as_str(),
            &params,
            Some(SINA_HEADERS),
        )
        .await?;
    let json = strip_jsonp(&text).ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "JSONP response contained no array body".into(),
    })?;
    let v: Value = serde_json::from_str(&json).map_err(Error::Json)?;
    let mut rows = parse_kline(&v)?;
    // Slice by date bounds (ISO strings sort lexicographically).
    rows.retain(|r| r.date.as_str() >= start.as_str() && r.date.as_str() <= end.as_str());
    Ok(rows)
}

/// Parse a Sina main-continuous kline payload (JSON array of 8-field rows).
pub(crate) fn parse_kline(resp: &Value) -> Result<Vec<FuturesMainRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "expected a JSON array of daily klines".into(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for row in arr {
        let a = row.as_array().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "kline entry is not an array".into(),
        })?;
        // Fields: 日期, 开盘价, 最高价, 最低价, 收盘价, 成交量, 持仓量, 动态结算价
        if a.len() < 8 {
            continue;
        }
        out.push(FuturesMainRow {
            date: fstr(&a[0]).unwrap_or_default(),
            open: fnum(&a[1]),
            high: fnum(&a[2]),
            low: fnum(&a[3]),
            close: fnum(&a[4]),
            volume: fnum(&a[5]),
            open_interest: fnum(&a[6]),
            settle_price: fnum(&a[7]),
            source: SOURCE_SINA,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// futures_display  (akshare `futures_display_main_sina`)
// ---------------------------------------------------------------------------

/// Sina market-center endpoint listing contracts for a given product node.
const DISPLAY_NODE_URL: &str = "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQFuturesData";

/// Sina JS that enumerates each exchange's tradable product nodes.
const SUBSCRIBE_URL: &str =
    "http://vip.stock.finance.sina.com.cn/quotes_service/view/js/qihuohangqing.js";

/// Exchanges iterated by `futures_display_main_sina`.
const EXCHANGES: &[&str] = &["dce", "czce", "shfe", "cffex", "gfex"];

/// One main-continuous contract in the Sina product list (`futures_display_main_sina`).
///
/// Mirrors the first three columns akshare keeps (`iloc[0, :3]`): 名称, 合约代码, 代码.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesDisplayRow {
    /// 名称 (akshare `name`, e.g. `PVC连续`)
    pub name: Option<String>,
    /// 合约代码 (akshare `symbol`, e.g. `V0`)
    pub symbol: Option<String>,
    /// 代码 (akshare `code`)
    pub code: Option<String>,
    /// Data origin (`sina`).
    pub source: &'static str,
}

/// Sina main-continuous contract product list (`futures_display_main_sina`).
///
/// Iterates every exchange, resolves its product nodes via the subscribe JS, then
/// lists each node's contracts and keeps only the main-continuous ones (name
/// contains `连续` and the symbol's trailing digit is `0`), matching akshare.
pub async fn futures_display(client: &Client) -> Result<Vec<FuturesDisplayRow>> {
    let mut out = Vec::new();
    for ex in EXCHANGES {
        let nodes = subscribe_exchange_symbol(client, ex).await?;
        for node in nodes {
            let params = [
                ("page", "1"),
                ("num", "5"),
                ("sort", "position"),
                ("asc", "0"),
                ("node", node.as_str()),
                ("base", "futures"),
            ];
            let text = client
                .get_text(
                    SOURCE_SINA,
                    "futures_display",
                    DISPLAY_NODE_URL,
                    &params,
                    Some(SINA_HEADERS),
                )
                .await?;
            let v: Value = serde_json::from_str(&text).map_err(Error::Json)?;
            out.extend(parse_display(&v)?);
        }
    }
    Ok(out)
}

/// Parse a Sina `getHQFuturesData` payload, keeping only main-continuous contracts.
pub(crate) fn parse_display(resp: &Value) -> Result<Vec<FuturesDisplayRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "expected a JSON array of contracts".into(),
    })?;
    let mut out = Vec::new();
    for item in arr {
        let name = item.get("name").and_then(|v| v.as_str());
        let symbol = item.get("symbol").and_then(|v| v.as_str());
        // akshare: name contains "连续" AND symbol matches ([\\w])(\\d) with the
        // captured digit containing "0" (i.e. the trailing digit is 0).
        let is_main = match (name, symbol) {
            (Some(n), Some(s)) => n.contains("连续") && symbol_has_zero(s),
            _ => false,
        };
        if !is_main {
            continue;
        }
        out.push(FuturesDisplayRow {
            name: name.map(str::to_string),
            symbol: symbol.map(str::to_string),
            code: item
                .get("code")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            source: SOURCE_SINA,
        });
    }
    Ok(out)
}

/// Resolve an exchange's product nodes from the Sina subscribe JS.
async fn subscribe_exchange_symbol(client: &Client, exchange: &str) -> Result<Vec<String>> {
    let text = client
        .get_text(
            SOURCE_SINA,
            "futures_display_subscribe",
            SUBSCRIBE_URL,
            &[],
            Some(SINA_HEADERS),
        )
        .await?;
    let obj = extract_object(&text).ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "subscribe response contained no JSON object".into(),
    })?;
    let v: Value = serde_json::from_str(obj).map_err(Error::Json)?;
    parse_subscribe(&v, exchange)
}

/// Parse the Sina subscribe object: column 1 (name/node) of each product entry.
///
/// The object is `{ exchange: [ <exchange-name>, [mark, name], [mark, name], ... ] }`;
/// akshare uses `.iloc[:, 1]` (the `name`) as the `node` passed to `getHQFuturesData`.
pub(crate) fn parse_subscribe(resp: &Value, exchange: &str) -> Result<Vec<String>> {
    let arr = resp
        .get(exchange)
        .and_then(|x| x.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: format!("missing product list for exchange `{exchange}`"),
        })?;
    let mut nodes = Vec::new();
    // Skip element 0 (the exchange display name), keep column 1 of each entry.
    for item in arr.iter().skip(1) {
        if let Some(lst) = item.as_array()
            && let Some(n) = lst.get(1).and_then(|v| v.as_str())
        {
            nodes.push(n.to_string());
        }
    }
    Ok(nodes)
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Strip a Sina JSONP wrapper (`var ...=(...);`) down to the inner JSON array.
pub(crate) fn strip_jsonp(text: &str) -> Option<String> {
    // akshare: text[text.find("([")+1 : text.rfind("])")+1]
    let start = text.find("([")? + 1;
    let end = text.rfind("])")? + 1;
    if end <= start {
        return None;
    }
    Some(text[start..end].to_string())
}

/// Extract the first `{...}` object from a possibly-JS-wrapped response.
fn extract_object(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(&text[start..=end])
}

/// Normalize `YYYYMMDD` to `YYYY-MM-DD`; reject anything else.
fn norm_date(s: &str) -> Result<String> {
    let t = s.trim();
    if t.len() == 8 && t.bytes().all(|b| b.is_ascii_digit()) {
        Ok(format!("{}-{}-{}", &t[..4], &t[4..6], &t[6..]))
    } else {
        Err(Error::InvalidParam(format!(
            "date must be YYYYMMDD, got: {s}"
        )))
    }
}

/// Faithful std-only reimplementation of akshare's `symbol.str.extract(r"([\w])(\d)").iloc[:,1].str.contains("0")`.
///
/// Returns true iff a word char is immediately followed by a digit, and that
/// digit equals `0`.
fn symbol_has_zero(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();
    for w in 0..chars.len().saturating_sub(1) {
        if is_word_char(chars[w]) && chars[w + 1].is_ascii_digit() {
            return chars[w + 1] == '0';
        }
    }
    false
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn fnum(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn fstr(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_text(name: &str) -> String {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        std::fs::read_to_string(path).unwrap()
    }

    #[test]
    fn parses_futures_main_kline_fixture() {
        let text = fixture_text("futures_main.txt");
        let json = strip_jsonp(&text).expect("JSONP body");
        let v: Value = serde_json::from_str(&json).unwrap();
        let rows = parse_kline(&v).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].date, "2024-01-02");
        assert_eq!(rows[0].open, Some(3000.0));
        assert_eq!(rows[0].high, Some(3080.0));
        assert_eq!(rows[0].low, Some(2980.0));
        assert_eq!(rows[0].close, Some(3050.0));
        assert_eq!(rows[0].volume, Some(120000.0));
        assert_eq!(rows[0].open_interest, Some(150000.0));
        assert_eq!(rows[0].settle_price, Some(3040.0));
        assert_eq!(rows[0].source, "sina");
        assert_eq!(rows[2].date, "2023-12-29");
    }

    #[test]
    fn keeps_only_main_continuous_contracts() {
        let text = fixture_text("futures_display.json");
        let v: Value = serde_json::from_str(&text).unwrap();
        let rows = parse_display(&v).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].name.as_deref(), Some("PVC连续"));
        assert_eq!(rows[0].symbol.as_deref(), Some("V0"));
        assert_eq!(rows[0].code.as_deref(), Some("V0"));
        assert_eq!(rows[1].symbol.as_deref(), Some("M0"));
        assert_eq!(rows[2].symbol.as_deref(), Some("PP0"));
        assert_eq!(rows[0].source, "sina");
    }

    #[test]
    fn parses_futures_display_subscribe_fixture() {
        let text = fixture_text("futures_display_subscribe.json");
        let v: Value = serde_json::from_str(&text).unwrap();
        let nodes = parse_subscribe(&v, "dce").unwrap();
        assert_eq!(nodes, vec!["PVC", "豆粕", "聚丙烯"]);
    }

    #[test]
    fn rejects_bad_date_format() {
        assert!(norm_date("2024010").is_err());
        assert!(norm_date("not-a-date").is_err());
        assert!(norm_date("20240101").is_ok());
    }
}
