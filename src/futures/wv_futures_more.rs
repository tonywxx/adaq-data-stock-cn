//! Futures market data ports (akshare `futures` package) — second wave.
//!
//! Ports three Eastmoney / Sina helpers whose real request is a plain JSON or
//! JSONP GET (no JS signing, token, cookie, HTML scrape or ZIP/Excel):
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `futures_hist_em` | `futures_hist_em.py:91` | Eastmoney `push2his` kline (`secid` resolved via `futsse-static` symbol map) |
//! | `futures_foreign_hist` | `futures_foreign.py:20` | Sina JSONP `GlobalFuturesService.getGlobalFuturesDailyKLine` |
//! | `futures_rule_em` | `futures_rule_em.py:14` | Eastmoney `GetPZJYInfo` (`data.Data`, dynamic columns) |
//!
//! ## DEFERRED (see `docs/_draft_fut.md` and table below)
//! * **`futures_inventory_99`** (`futures_inventory_99.py:47`) — the symbol →
//!   `productId` map is scraped from `99qh.com` `__NEXT_DATA__` via
//!   BeautifulSoup (HTML scrape → deferral trigger). The actual inventory
//!   endpoint (`centerapi.fx168api.com`) is a clean JSON GET, but it is
//!   unreachable without the scraped map.
//! * **`futures_dce_position_rank`** (`cot.py:818`) — POSTs to DCE and downloads
//!   a **ZIP** of TSV position-rank tables; the result is a `dict` of
//!   DataFrames (not a `Vec<Row>`), and would need a `zip` dependency plus
//!   complex table-slicing logic. Outside the Eastmoney/Sina plain-JSON scope.
//!
//! ### Deferred table
//! | akshare fn | source | reason |
//! |---|---|---|
//! | `futures_inventory_99` | `futures_inventory_99.py:47` | symbol map via BeautifulSoup HTML scrape |
//! | `futures_dce_position_rank` | `cot.py:818` | DCE ZIP+TSV binary; returns dict of DataFrames, not `Vec<Row>` |

#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap};

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE: &str = "eastmoney";
const SOURCE_SINA: &str = "sina";

// ---------------------------------------------------------------------------
// 1. futures_hist_em — Eastmoney futures kline history
// ---------------------------------------------------------------------------

/// One Eastmoney futures daily/weekly/monthly kline bar (`futures_hist_em`).
///
/// Mirrors akshare's reordered output columns: 时间 / 开盘 / 最高 / 最低 / 收盘 /
/// 涨跌 / 涨跌幅 / 成交量 / 成交额 / 持仓量 (derived from the upstream 14-field
/// `f51..f64` kline string).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesHistEmRow {
    /// 时间 (date, `YYYY-MM-DD`).
    pub date: String,
    /// 开盘 (open).
    pub open: Option<f64>,
    /// 最高 (high).
    pub high: Option<f64>,
    /// 最低 (low).
    pub low: Option<f64>,
    /// 收盘 (close).
    pub close: Option<f64>,
    /// 涨跌 (absolute change).
    pub change: Option<f64>,
    /// 涨跌幅 (percent change).
    pub change_pct: Option<f64>,
    /// 成交量 (volume).
    pub volume: Option<f64>,
    /// 成交额 (turnover).
    pub amount: Option<f64>,
    /// 持仓量 (open interest).
    pub open_interest: Option<f64>,
}

/// Parse one `f51..f64` kline CSV string into a [`FuturesHistEmRow`].
///
/// Column layout (after split on `,`), matching akshare's `temp_df.columns`:
/// `0=时间 1=开盘 2=收盘 3=最高 4=最低 5=成交量 6=成交额 7=- 8=涨跌幅 9=涨跌
/// 10=_ 11=_ 12=持仓量 13=_`. Rows with fewer than 13 fields are skipped.
pub(crate) fn parse_kline(line: &str) -> Option<FuturesHistEmRow> {
    let p: Vec<&str> = line.split(',').collect();
    if p.len() < 13 {
        return None;
    }
    let num = |i: usize| p.get(i).and_then(|s| s.trim().parse::<f64>().ok());
    Some(FuturesHistEmRow {
        date: p[0].to_string(),
        open: num(1),
        close: num(2),
        high: num(3),
        low: num(4),
        volume: num(5),
        amount: num(6),
        change_pct: num(8),
        change: num(9),
        open_interest: num(12),
    })
}

/// Normalise a `YYYYMMDD` (or already `YYYY-MM-DD`) date string to `YYYY-MM-DD`
/// for lexicographic range filtering.
pub(crate) fn normalize_date(d: &str) -> String {
    if d.len() == 8 && d.bytes().all(|b| b.is_ascii_digit()) {
        format!("{}-{}-{}", &d[0..4], &d[4..6], &d[6..8])
    } else {
        d.to_string()
    }
}

/// Eastmoney kline period code (`futures_hist_em` `period_dict`).
fn period_code(period: &str) -> Result<&'static str> {
    match period {
        "daily" => Ok("101"),
        "weekly" => Ok("102"),
        "monthly" => Ok("103"),
        _ => Err(Error::InvalidParam(format!("unknown period: {period}"))),
    }
}

/// Split a futures symbol into its alphabetic/`CJK` prefix and its numeric
/// suffix (akshare `__futures_hist_separate_char_and_numbers_em`, no regex).
pub(crate) fn separate_char_and_numbers(symbol: &str) -> (String, String) {
    let chars: String = symbol
        .chars()
        .filter(|c| c.is_alphabetic() || (0x4e00..=0x9fa5).contains(&(*c as u32)))
        .collect();
    let numbers: String = symbol.chars().filter(|c| c.is_ascii_digit()).collect();
    (chars, numbers)
}

/// Four-way symbol→market/code maps (akshare `__get_exchange_symbol_map`).
#[derive(Default)]
pub(crate) struct ExchangeSymbolMap {
    c_contract_mkt: HashMap<String, String>,
    c_contract_to_e_contract: HashMap<String, String>,
    e_symbol_mkt: HashMap<String, String>,
    c_symbol_mkt: HashMap<String, String>,
}

/// Build the four symbol maps from the raw `futsse-static` contract list.
pub(crate) fn build_exchange_symbol_map(raw: &[Value]) -> ExchangeSymbolMap {
    let mut m = ExchangeSymbolMap::default();
    for item in raw {
        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let mktid = item.get("mktid").and_then(|v| v.as_str()).unwrap_or("");
        let code = item.get("code").and_then(|v| v.as_str()).unwrap_or("");
        let vcode = item.get("vcode").and_then(|v| v.as_str()).unwrap_or("");
        let vname = item.get("vname").and_then(|v| v.as_str()).unwrap_or("");
        m.c_contract_mkt.insert(name.to_string(), mktid.to_string());
        m.c_contract_to_e_contract
            .insert(name.to_string(), code.to_string());
        m.e_symbol_mkt.insert(vcode.to_string(), mktid.to_string());
        m.c_symbol_mkt.insert(vname.to_string(), mktid.to_string());
    }
    m
}

/// Fetch the full exchange-symbol raw list from `futsse-static.eastmoney.com`
/// (akshare `__fetch_exchange_symbol_raw_em`: a 3-layer `msgid` walk).
async fn fetch_exchange_symbol_raw_em(client: &Client) -> Result<Vec<Value>> {
    let base = "https://futsse-static.eastmoney.com/redis";
    let first = client
        .get_json(SOURCE, "futures_hist_em_map", base, &[("msgid", "gnweb")])
        .await?;
    let items = first.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE,
        message: "symbol-map root is not an array".into(),
    })?;
    let mut all: Vec<Value> = Vec::new();
    for item in items {
        let mktid = item.get("mktid").and_then(|v| v.as_str()).ok_or_else(|| {
            Error::UpstreamChanged {
                origin: SOURCE,
                message: "symbol-map item missing mktid".into(),
            }
        })?;
        let inner = client
            .get_json(SOURCE, "futures_hist_em_map", base, &[("msgid", mktid)])
            .await?;
        let inner_arr = inner.as_array().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "symbol-map inner is not an array".into(),
        })?;
        for n in 1..=inner_arr.len() {
            let msgid = format!("{mktid}_{n}");
            let sub = client
                .get_json(SOURCE, "futures_hist_em_map", base, &[("msgid", &msgid)])
                .await?;
            if let Some(arr) = sub.as_array() {
                all.extend(arr.iter().cloned());
            }
        }
    }
    Ok(all)
}

/// Resolve an akshare futures symbol to an Eastmoney `secid` (`mktid.code`).
async fn resolve_secid(client: &Client, symbol: &str) -> Result<String> {
    let raw = fetch_exchange_symbol_raw_em(client).await?;
    let m = build_exchange_symbol_map(&raw);
    if let (Some(mkt), Some(code)) = (
        m.c_contract_mkt.get(symbol),
        m.c_contract_to_e_contract.get(symbol),
    ) {
        return Ok(format!("{mkt}.{code}"));
    }
    // Fallback: separate leading chars from trailing digits.
    let (chars, _) = separate_char_and_numbers(symbol);
    let is_cjk = !chars.is_empty()
        && chars.chars().all(|c| (0x4e00..=0x9fa5).contains(&(c as u32)));
    let mkt = if is_cjk {
        m.c_symbol_mkt.get(&chars)
    } else {
        m.e_symbol_mkt.get(&chars)
    };
    match mkt {
        Some(mkt) => Ok(format!("{mkt}.{symbol}")),
        None => Err(Error::InvalidParam(format!(
            "futures_hist_em: unknown symbol {symbol}"
        ))),
    }
}

/// 东方财富网-期货行情-行情数据 (`futures_hist_em`, `futures_hist_em.py:91`).
///
/// `symbol` is an akshare futures code (e.g. `"热卷主连"`); `period` is one of
/// `daily` / `weekly` / `monthly`; `start_date` / `end_date` are `YYYYMMDD`.
pub async fn futures_hist_em(
    client: &Client,
    symbol: &str,
    period: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<FuturesHistEmRow>> {
    let klt = period_code(period)?;
    let secid = resolve_secid(client, symbol).await?;
    let url = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
    let params = [
        ("secid", secid.as_str()),
        ("klt", klt),
        ("fqt", "1"),
        ("lmt", "10000"),
        ("end", "20500000"),
        ("iscca", "1"),
        ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8"),
        (
            "fields2",
            "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f62,f63,f64",
        ),
        ("ut", "7eea3edcaed734bea9cbfc24409ed989"),
        ("forcect", "1"),
    ];
    let v = client
        .get_json(SOURCE, "futures_hist_em", url, &params)
        .await?;
    let klines = v
        .get("data")
        .and_then(|d| d.get("klines"))
        .and_then(|k| k.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing data.klines".into(),
        })?;
    let start = normalize_date(start_date);
    let end = normalize_date(end_date);
    let mut out = Vec::with_capacity(klines.len());
    for item in klines {
        let line = item.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "kline entry is not a string".into(),
        })?;
        if let Some(row) = parse_kline(line) {
            if row.date.as_str() >= start.as_str() && row.date.as_str() <= end.as_str() {
                out.push(row);
            }
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// 2. futures_foreign_hist — Sina global futures daily history (JSONP)
// ---------------------------------------------------------------------------

/// One Sina global-futures daily bar (`futures_foreign_hist`).
///
/// akshare returns the upstream array with default positional columns; the
/// Sina `getGlobalFuturesDailyKLine` payload is `[date, open, high, low, close,
/// volume, ...]`, mapped here to the OHLCV core fields.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesForeignHistRow {
    /// 日期 (date).
    pub date: String,
    /// 开盘 (open).
    pub open: Option<f64>,
    /// 最高 (high).
    pub high: Option<f64>,
    /// 最低 (low).
    pub low: Option<f64>,
    /// 收盘 (close).
    pub close: Option<f64>,
    /// 成交量 (volume).
    pub volume: Option<f64>,
}

fn vnum(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// Strip the Sina JSONP wrapper (`var _S...=([...]);`) and parse the inner
/// daily-K-line array into [`FuturesForeignHistRow`]s.
pub(crate) fn parse_foreign_hist(text: &str) -> Result<Vec<FuturesForeignHistRow>> {
    let start = text.find('[').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "foreign-hist jsonp missing '['".into(),
    })?;
    let end = text.rfind(']').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "foreign-hist jsonp missing ']'".into(),
    })? + 1;
    let v: Value = serde_json::from_slice(text[start..end].as_bytes()).map_err(Error::Json)?;
    let arr = v.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "foreign-hist payload is not an array".into(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for row in arr {
        let r = row.as_array().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "foreign-hist row is not an array".into(),
        })?;
        out.push(FuturesForeignHistRow {
            date: r
                .first()
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            open: r.get(1).and_then(vnum),
            high: r.get(2).and_then(vnum),
            low: r.get(3).and_then(vnum),
            close: r.get(4).and_then(vnum),
            volume: r.get(5).and_then(vnum),
        });
    }
    Ok(out)
}

/// 外盘期货-历史行情数据-日频率 (`futures_foreign_hist`, `futures_foreign.py:20`).
///
/// `symbol` is a Sina global-futures code (e.g. `"ZSD"`).
pub async fn futures_foreign_hist(client: &Client, symbol: &str) -> Result<Vec<FuturesForeignHistRow>> {
    let today = chrono::Local::now().format("%Y_%m_%d").to_string();
    let url = format!(
        "https://stock2.finance.sina.com.cn/futures/api/jsonp.php/var%20_S{today}=/\
         GlobalFuturesService.getGlobalFuturesDailyKLine"
    );
    let params = [("symbol", symbol), ("_", today.as_str()), ("source", "web")];
    let text = client
        .get_text(SOURCE_SINA, "futures_foreign_hist", &url, &params, None)
        .await?;
    parse_foreign_hist(&text)
}

// ---------------------------------------------------------------------------
// 3. futures_rule_em — Eastmoney futures variety & trading rules
// ---------------------------------------------------------------------------

/// One Eastmoney futures variety/trading-rule record (`futures_rule_em`).
///
/// akshare returns `pd.DataFrame(data_json["Data"])` with **whatever columns
/// the upstream emits** (no fixed schema). We faithfully preserve every field
/// via a flattened map rather than guessing a fixed column set.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesRuleEmRow {
    #[serde(flatten)]
    pub fields: BTreeMap<String, Value>,
}

/// Parse the `Data` array of an Eastmoney `GetPZJYInfo` response into rows.
pub(crate) fn parse_rule_em(resp: &Value) -> Result<Vec<FuturesRuleEmRow>> {
    let data = resp
        .get("Data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing Data".into(),
        })?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let fields = match item {
            Value::Object(o) => o
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect::<BTreeMap<_, _>>(),
            _ => BTreeMap::new(),
        };
        out.push(FuturesRuleEmRow { fields });
    }
    Ok(out)
}

/// 东方财富网-期货行情-品种及交易规则 (`futures_rule_em`, `futures_rule_em.py:14`).
pub async fn futures_rule_em(client: &Client) -> Result<Vec<FuturesRuleEmRow>> {
    let url = "https://eastmoneyfutures.com/api/ComManage/GetPZJYInfo";
    let v = client.get_json(SOURCE, "futures_rule_em", url, &[]).await?;
    parse_rule_em(&v)
}

// ---------------------------------------------------------------------------
// Deferred set (recorded, not implemented) — see module doc.
// ---------------------------------------------------------------------------

/// Deferred akshare futures fns in this module (see module doc for reasons).
pub const DEFERRED_FNS: &[&str] = &["futures_inventory_99", "futures_dce_position_rank"];

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

    fn fixture_text(name: &str) -> String {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        std::fs::read_to_string(p).unwrap()
    }

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    #[test]
    fn parses_kline_row() {
        let row = parse_kline(
            "2024-01-02,3500.0,3520.0,3530.0,3490.0,1000,3500000.0,,1.2,42.0,,,-12.0,",
        )
        .expect("row");
        assert_eq!(row.date, "2024-01-02");
        assert!(approx(row.open, 3500.0));
        assert!(approx(row.high, 3530.0));
        assert!(approx(row.low, 3490.0));
        assert!(approx(row.close, 3520.0));
        assert!(approx(row.change, 42.0));
        assert!(approx(row.change_pct, 1.2));
        assert!(approx(row.volume, 1000.0));
        assert!(approx(row.amount, 3500000.0));
        assert!(approx(row.open_interest, -12.0));
    }

    #[test]
    fn kline_skips_short_rows() {
        assert!(parse_kline("2024-01-02,1,2,3").is_none());
    }

    #[test]
    fn normalize_date_works() {
        assert_eq!(normalize_date("19900101"), "1990-01-01");
        assert_eq!(normalize_date("2024-01-02"), "2024-01-02");
    }

    #[test]
    fn separate_char_and_numbers_works() {
        assert_eq!(separate_char_and_numbers("焦煤2506"), ("焦煤".to_string(), "2506".to_string()));
        assert_eq!(
            separate_char_and_numbers("热卷主连"),
            ("热卷主连".to_string(), "".to_string())
        );
    }

    #[test]
    fn builds_symbol_map() {
        let raw = vec![
            serde_json::json!({"name":"热卷主连","mktid":"114","code":"hc","vcode":"HCM","vname":"热卷"}),
            serde_json::json!({"name":"焦煤","mktid":"114","code":"jm","vcode":"JMM","vname":"焦煤"}),
        ];
        let m = build_exchange_symbol_map(&raw);
        assert_eq!(m.c_contract_mkt.get("热卷主连"), Some(&"114".to_string()));
        assert_eq!(m.c_contract_to_e_contract.get("热卷主连"), Some(&"hc".to_string()));
        assert_eq!(m.e_symbol_mkt.get("HCM"), Some(&"114".to_string()));
        assert_eq!(m.c_symbol_mkt.get("焦煤"), Some(&"114".to_string()));
    }

    #[test]
    fn hist_em_klines_fixture_filters_by_date() {
        let v = fixture("futures_hist_em.json");
        let klines = v
            .get("data")
            .and_then(|d| d.get("klines"))
            .and_then(|k| k.as_array())
            .unwrap();
        let start = normalize_date("20240103");
        let end = normalize_date("20240104");
        let mut count = 0;
        for item in klines {
            if let Some(row) = parse_kline(item.as_str().unwrap()) {
                if row.date.as_str() >= start.as_str() && row.date.as_str() <= end.as_str() {
                    count += 1;
                }
            }
        }
        assert_eq!(count, 2);
    }

    #[test]
    fn parses_foreign_hist_jsonp() {
        let t = fixture_text("futures_foreign_hist.txt");
        let rows = parse_foreign_hist(&t).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02");
        assert!(approx(rows[0].open, 3500.0));
        assert!(approx(rows[0].high, 3520.0));
        assert!(approx(rows[0].low, 3490.0));
        assert!(approx(rows[0].close, 3510.0));
        assert!(approx(rows[0].volume, 12345.0));
        assert_eq!(rows[1].date, "2024-01-03");
    }

    #[test]
    fn parses_rule_em_data() {
        let v = fixture("futures_rule_em.json");
        let rows = parse_rule_em(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].fields.get("品种代码").and_then(|x| x.as_str()),
            Some("AU")
        );
        assert_eq!(
            rows[0].fields.get("品种名称").and_then(|x| x.as_str()),
            Some("黄金")
        );
        assert_eq!(
            rows[1].fields.get("品种代码").and_then(|x| x.as_str()),
            Some("AG")
        );
    }
}
