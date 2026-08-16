//! Coin / metal / commodity futures data (akshare `futures` commodity endpoints).
//!
//! New top-level domain `coin`, reimplementing akshare's metal/commodity futures
//! functions that are pure HTTP/JSON (or Sina text):
//!
//! | Rust fn                    | akshare source                       | transport            |
//! | -------------------------- | ------------------------------------ | -------------------- |
//! | `coin_lme_realtime`        | `futures_foreign_commodity_realtime`| Sina text (`hq.sinajs.cn`) |
//! | `coin_shfe_rank`           | `get_shfe_rank_table`                | SHFE JSON (`o_cursor`) |
//! | `coin_foreign_hist`        | `futures_global_hist_em`             | Eastmoney `push2his` kline |
//! | `coin_futures_hist`        | `futures_hist_em`                    | Eastmoney `push2his` kline |
//! | `coin_futures_symbol_map`  | `futures_hist_table_em`              | Eastmoney `futsse-static` redis |
//!
//! ## Skips (not portable / already covered)
//! - `get_roll_yield` — computed from daily bars + calendar math, not a direct HTTP call.
//! - `get_czce_rank_table` / `get_dce_rank_table` — Excel / HTML downloads (CZCE `.xls`,
//!   DCE `.xlsx`/HTML scrape).
//! - `futures_settlement_price_sgx` — downloads a ZIP of CSV from `links.sgx.com`.
//! - `futures_spot_price` / `futures_spot_stock` — 100ppi HTML + embedded-JS (`demjson`) scrape.
//! - `futures_comm_ctp` / `futures_comm_js` / `futures_comm_qihuo` — HTML scrape / JS signing.
//! - `futures_inventory_em` / `futures_comex_inventory` — already ported under `futures`.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

/// SHFE daily position-rank endpoint (JSON, `o_cursor`).
const SOURCE_SHFE: &str = "shfe";
/// Sina foreign-futures realtime quote endpoint (text).
const SOURCE_SINA: &str = "sina";

const SHFE_RANK_URL: &str = "https://www.shfe.com.cn/data/tradedata/future/dailydata/pm{date}.dat";
const SINA_FOREIGN_URL: &str = "https://hq.sinajs.cn/";
/// `Referer` required by Sina for the foreign-futures realtime endpoint.
const SINA_HEADERS: &[(&str, &str)] = &[("Referer", "https://finance.sina.com.cn/")];
const EM_KLINE_URL: &str = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
const EM_REDIS_URL: &str = "https://futsse-static.eastmoney.com/redis";

// ---------------------------------------------------------------------------
// coin_lme_realtime — Sina LME + London-metal realtime quotes
// ---------------------------------------------------------------------------

/// One realtime foreign/metal quote from Sina (`futures_foreign_commodity_realtime`).
///
/// akshare columns: 名称, 最新价, 人民币报价, 买价, 卖价, 最高, 最低, 时间,
/// 昨结, 开盘, 持仓, 日期, 代码.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CoinLmeRealtimeRow {
    /// Sina subscribe code, e.g. `CAD` (LME copper), `XAU` (London gold).
    pub code: String,
    /// Chinese name, e.g. `伦敦铜`. akshare column `名称`.
    pub name: String,
    /// Latest price. akshare column `最新价`.
    pub current_price: Option<f64>,
    /// RMB-quoted price. akshare column `人民币报价`.
    pub current_price_rmb: Option<f64>,
    /// Bid price. akshare column `买价`.
    pub bid: Option<f64>,
    /// Ask price. akshare column `卖价`.
    pub ask: Option<f64>,
    /// Session high. akshare column `最高`.
    pub high: Option<f64>,
    /// Session low. akshare column `最低`.
    pub low: Option<f64>,
    /// Quote time `HH:MM:SS`. akshare column `时间`.
    pub time: String,
    /// Last settlement price. akshare column `昨结`.
    pub last_settle_price: Option<f64>,
    /// Open. akshare column `开盘`.
    pub open: Option<f64>,
    /// Open interest / holdings. akshare column `持仓`.
    pub hold: Option<f64>,
    /// Trade date `YYYY-MM-DD`. akshare column `日期`.
    pub date: String,
}

/// Realtime LME + London-metal quotes from Sina (`futures_foreign_commodity_realtime`).
///
/// `symbols` are Sina subscribe codes such as `CAD` (LME copper), `AHD` (LME aluminium),
/// `XAU` (London gold), `XAG` (London silver). A `Referer` header is required by Sina.
pub async fn coin_lme_realtime(
    client: &Client,
    symbols: &[&str],
) -> Result<Vec<CoinLmeRealtimeRow>> {
    if symbols.is_empty() {
        return Err(Error::InvalidParam("symbols must not be empty".into()));
    }
    let list = symbols
        .iter()
        .map(|s| format!("hf_{s}"))
        .collect::<Vec<_>>()
        .join(",");
    let params = [("list", list.as_str())];
    let text = client
        .get_text(
            SOURCE_SINA,
            "coin_lme_realtime",
            SINA_FOREIGN_URL,
            &params,
            Some(SINA_HEADERS),
        )
        .await?;
    parse_coin_lme_realtime(&text)
}

/// Parse Sina `hq.sinajs.cn` foreign-futures quote text into rows.
pub(crate) fn parse_coin_lme_realtime(text: &str) -> Result<Vec<CoinLmeRealtimeRow>> {
    let mut out = Vec::new();
    for line in text.split(';') {
        let line = line.trim();
        if line.is_empty() || !line.contains("hq_str_hf_") {
            continue;
        }
        // var hq_str_hf_CAD="伦敦铜,7321,...,2024-01-02,CAD,52780.00";
        let (prefix, value) = match line.split_once('=') {
            Some(p) => p,
            None => continue,
        };
        let code = match prefix.find("hf_") {
            Some(i) => prefix[i + 3..].to_string(),
            None => continue,
        };
        let value = value.trim().trim_matches('"').to_string();
        if value.is_empty() {
            continue;
        }
        let mut parts: Vec<&str> = value.split(',').collect();
        // London gold (XAU) omits the RMB column -> 14 fields; pad to 15.
        if parts.len() == 14 {
            parts.push("");
        }
        if parts.len() < 15 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_SINA,
                message: format!("unexpected field count {}", parts.len()),
            });
        }
        out.push(CoinLmeRealtimeRow {
            code,
            name: parts[0].to_string(),
            current_price: fnum_str(parts[1]),
            current_price_rmb: fnum_str(parts[14]),
            bid: fnum_str(parts[2]),
            ask: fnum_str(parts[3]),
            high: fnum_str(parts[4]),
            low: fnum_str(parts[5]),
            time: parts[6].to_string(),
            last_settle_price: fnum_str(parts[7]),
            open: fnum_str(parts[8]),
            hold: fnum_str(parts[9]),
            date: parts[12].to_string(),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// coin_shfe_rank — SHFE member position ranking (JSON)
// ---------------------------------------------------------------------------

/// One SHFE member position-ranking row (`get_shfe_rank_table`).
///
/// akshare columns: 排名, 成交量会员, 成交量, 成交量变化, 持多单会员, 持多单,
/// 持多单变化, 持空单会员, 持空单, 持空单变化, 合约, 品种, 日期.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CoinShfeRankRow {
    /// Rank. akshare column `RANK`.
    pub rank: Option<i64>,
    /// Volume member name. akshare column `PARTICIPANTABBR1`.
    pub vol_party_name: String,
    /// Volume. akshare column `CJ1`.
    pub vol: Option<f64>,
    /// Volume change. akshare column `CJ1_CHG`.
    pub vol_chg: Option<f64>,
    /// Long-position member name. akshare column `PARTICIPANTABBR2`.
    pub long_party_name: String,
    /// Long open interest. akshare column `CJ2`.
    pub long_open_interest: Option<f64>,
    /// Long open-interest change. akshare column `CJ2_CHG`.
    pub long_open_interest_chg: Option<f64>,
    /// Short-position member name. akshare column `PARTICIPANTABBR3`.
    pub short_party_name: String,
    /// Short open interest. akshare column `CJ3`.
    pub short_open_interest: Option<f64>,
    /// Short open-interest change. akshare column `CJ3_CHG`.
    pub short_open_interest_chg: Option<f64>,
    /// Instrument id, e.g. `cu2410`. akshare column `INSTRUMENTID`.
    pub symbol: String,
    /// Product name, e.g. `铜`. akshare column `PRODUCTNAME`.
    pub product: String,
    /// Variety (alphabetic prefix of `symbol`). akshare derives `variety`.
    pub variety: String,
    /// Trade date `YYYYMMDD`.
    pub date: String,
}

/// SHFE member position ranking (`get_shfe_rank_table`).
///
/// `date` is `YYYYMMDD`. Returns the combined ranking table for all contracts traded
/// that day (akshare filters by `vars_list` afterwards; we return everything and let
/// callers filter).
pub async fn coin_shfe_rank(client: &Client, date: &str) -> Result<Vec<CoinShfeRankRow>> {
    if date.len() != 8 || !date.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::InvalidParam("date must be YYYYMMDD".into()));
    }
    let url = SHFE_RANK_URL.replace("{date}", date);
    let v = client
        .get_json(SOURCE_SHFE, "coin_shfe_rank", &url, &[])
        .await?;
    parse_coin_shfe_rank(&v, date)
}

/// Parse SHFE `o_cursor` JSON into ranking rows.
pub(crate) fn parse_coin_shfe_rank(resp: &Value, date: &str) -> Result<Vec<CoinShfeRankRow>> {
    let cursor = resp
        .get("o_cursor")
        .and_then(|c| c.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SHFE,
            message: "missing o_cursor".into(),
        })?;
    let mut out = Vec::with_capacity(cursor.len());
    for item in cursor {
        let symbol = fstr(item, "INSTRUMENTID");
        let variety = symbol
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect::<String>();
        out.push(CoinShfeRankRow {
            rank: inum(item, "RANK"),
            vol_party_name: fstr(item, "PARTICIPANTABBR1"),
            vol: fnum(item, "CJ1"),
            vol_chg: fnum(item, "CJ1_CHG"),
            long_party_name: fstr(item, "PARTICIPANTABBR2"),
            long_open_interest: fnum(item, "CJ2"),
            long_open_interest_chg: fnum(item, "CJ2_CHG"),
            short_party_name: fstr(item, "PARTICIPANTABBR3"),
            short_open_interest: fnum(item, "CJ3"),
            short_open_interest_chg: fnum(item, "CJ3_CHG"),
            symbol,
            product: fstr(item, "PRODUCTNAME"),
            variety,
            date: date.to_string(),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// coin_foreign_hist / coin_futures_hist — Eastmoney kline history
// ---------------------------------------------------------------------------

/// One K-line row for a foreign or domestic futures contract (Eastmoney `push2his`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CoinFuturesKlineRow {
    /// Trade date `YYYY-MM-DD`. akshare column `日期`.
    pub date: String,
    /// Contract code, e.g. `HG00Y`. akshare column `代码`.
    pub code: String,
    /// Contract name, e.g. `COMEX铜`. akshare column `名称`.
    pub name: String,
    /// Open. akshare column `开盘`.
    pub open: Option<f64>,
    /// Close / latest. akshare column `最新价`/`收盘`.
    pub close: Option<f64>,
    /// High. akshare column `最高`.
    pub high: Option<f64>,
    /// Low. akshare column `最低`.
    pub low: Option<f64>,
    /// Volume. akshare column `总量`/`成交量`.
    pub volume: Option<f64>,
    /// Turnover. akshare column `成交额`.
    pub amount: Option<f64>,
    /// Change (absolute). akshare column `涨跌`.
    pub change: Option<f64>,
    /// Change percent. akshare column `涨幅`/`涨跌幅`.
    pub change_pct: Option<f64>,
    /// Open interest / holdings. akshare column `持仓`/`持仓量`.
    pub open_interest: Option<f64>,
    /// Daily open-interest change. akshare column `日增`.
    pub position_chg: Option<f64>,
}

/// Foreign/metal futures K-line history from Eastmoney (`futures_global_hist_em`).
///
/// `symbol` is a Sina/Eastmoney foreign symbol such as `HG00Y` (COMEX copper),
/// `GC00Y` (COMEX gold), `SI00Y` (COMEX silver). The Eastmoney market code is resolved
/// via akshare's `__futures_global_hist_market_code` table.
pub async fn coin_foreign_hist(client: &Client, symbol: &str) -> Result<Vec<CoinFuturesKlineRow>> {
    let market = foreign_market_code(symbol)
        .ok_or_else(|| Error::InvalidParam(format!("unsupported foreign symbol: {symbol}")))?;
    let secid = format!("{market}.{symbol}");
    kline(client, &secid, "coin_foreign_hist").await
}

/// Domestic futures K-line history from Eastmoney (`futures_hist_em`).
///
/// `secid` is the full Eastmoney security id, e.g. `114.al2505` (Shanghai aluminium)
/// or `114.cu2505` (Shanghai copper). Resolve Chinese names via
/// [`coin_futures_symbol_map`].
pub async fn coin_futures_hist(client: &Client, secid: &str) -> Result<Vec<CoinFuturesKlineRow>> {
    if secid.is_empty() {
        return Err(Error::InvalidParam("secid must not be empty".into()));
    }
    kline(client, secid, "coin_futures_hist").await
}

async fn kline(
    client: &Client,
    secid: &str,
    endpoint: &'static str,
) -> Result<Vec<CoinFuturesKlineRow>> {
    let params = [
        ("secid", secid),
        ("klt", "101"),
        ("fqt", "1"),
        ("lmt", "10000"),
        ("end", "20500000"),
        ("iscca", "1"),
        ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8"),
        (
            "fields2",
            "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f62,f63,f64",
        ),
        ("ut", "f057cbcbce2a86e2866ab8877db1d059"),
        ("forcect", "1"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, endpoint, EM_KLINE_URL, &params)
        .await?;
    parse_kline(&v)
}

/// Parse an Eastmoney `push2his` kline response into rows.
pub(crate) fn parse_kline(resp: &Value) -> Result<Vec<CoinFuturesKlineRow>> {
    let data = resp.get("data").filter(|v| v.is_object());
    let code = data
        .and_then(|d| d.get("code"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let name = data
        .and_then(|d| d.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let klines = match data
        .and_then(|d| d.get("klines"))
        .and_then(|k| k.as_array())
    {
        Some(k) => k,
        None => return Ok(Vec::new()),
    };
    let mut out = Vec::with_capacity(klines.len());
    for k in klines {
        let s = match k.as_str() {
            Some(s) => s,
            None => continue,
        };
        let p: Vec<&str> = s.split(',').collect();
        if p.len() < 14 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: format!("kline field count {}", p.len()),
            });
        }
        out.push(CoinFuturesKlineRow {
            date: p[0].to_string(),
            code: code.clone(),
            name: name.clone(),
            open: fnum_str(p[1]),
            close: fnum_str(p[2]),
            high: fnum_str(p[3]),
            low: fnum_str(p[4]),
            volume: fnum_str(p[5]),
            amount: fnum_str(p[6]),
            change_pct: fnum_str(p[8]),
            change: fnum_str(p[9]),
            open_interest: fnum_str(p[12]),
            position_chg: fnum_str(p[13]),
        });
    }
    Ok(out)
}

/// Resolve an Eastmoney market code for a foreign-futures symbol (akshare
/// `__futures_global_hist_market_code`). Returns `None` for unsupported symbols
/// (notably LME 3-month names such as `CAD`/`AHD`, which akshare also rejects —
/// use [`coin_lme_realtime`] for those).
fn foreign_market_code(symbol: &str) -> Option<i64> {
    let base: String = symbol
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    let has = |arr: &[&str]| arr.contains(&base.as_str());
    if has(&["HG", "GC", "SI", "QI", "QO", "MGC", "LTH"]) {
        return Some(101);
    }
    if has(&["CL", "NG", "RB", "HO", "PA", "PL", "QM"]) {
        return Some(102);
    }
    if has(&[
        "ZW", "ZM", "ZS", "ZC", "XC", "XK", "XW", "YM", "TY", "US", "EH", "ZL", "ZR", "ZO", "FV",
        "TU", "UL", "NQ", "ES",
    ]) {
        return Some(103);
    }
    if has(&["TF", "RT", "CN"]) {
        return Some(104);
    }
    if has(&["SB", "CT", "SF"]) {
        return Some(108);
    }
    if has(&["LCPT", "LZNT", "LALT", "LTNT", "LLDT", "LNKT"]) {
        return Some(109);
    }
    if base == "MPM" {
        return Some(110);
    }
    if base.starts_with('J') {
        return Some(111);
    }
    if has(&["M", "B", "G"]) {
        return Some(112);
    }
    None
}

// ---------------------------------------------------------------------------
// coin_futures_symbol_map — Eastmoney exchange symbol map (name -> secid)
// ---------------------------------------------------------------------------

/// One exchange symbol entry (`futures_hist_table_em`).
///
/// akshare columns: 市场简称, 合约中文代码, 合约代码.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CoinFuturesSymbolRow {
    /// Market id, e.g. `114`. akshare key `mktid`.
    pub mktid: String,
    /// Market name, e.g. `上海期货交易所`. akshare key `mktname`.
    pub mktname: String,
    /// Chinese contract name, e.g. `沪铜2505`. akshare key `name`.
    pub name: String,
    /// Contract code, e.g. `cu2505`. akshare key `code`.
    pub code: String,
    /// English/variety code, e.g. `CU2505`. akshare key `vcode`.
    pub vcode: String,
    /// Variety Chinese name, e.g. `铜`. akshare key `vname`.
    pub vname: String,
}

/// Eastmoney futures exchange symbol map (`futures_hist_table_em`), used to resolve a
/// Chinese contract name to its `secid` for [`coin_futures_hist`].
///
/// Mirrors akshare's nested `futsse-static.eastmoney.com/redis` walk (gnweb -> per-market
/// -> per-chunk) and flattens every entry into a single list.
pub async fn coin_futures_symbol_map(client: &Client) -> Result<Vec<CoinFuturesSymbolRow>> {
    let base = client
        .get_json(
            SOURCE_EASTMONEY,
            "coin_futures_symbol_map",
            EM_REDIS_URL,
            &[("msgid", "gnweb")],
        )
        .await?;
    let mut all: Vec<Value> = Vec::new();
    if let Some(markets) = base.as_array() {
        for mkt in markets {
            let mktid = match mkt.get("mktid").and_then(|v| v.as_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            let inner = client
                .get_json(
                    SOURCE_EASTMONEY,
                    "coin_futures_symbol_map",
                    EM_REDIS_URL,
                    &[("msgid", mktid.as_str())],
                )
                .await?;
            let n = inner.as_array().map(|a| a.len()).unwrap_or(0);
            for num in 1..=n {
                let msgid = format!("{mktid}_{num}");
                let chunk = client
                    .get_json(
                        SOURCE_EASTMONEY,
                        "coin_futures_symbol_map",
                        EM_REDIS_URL,
                        &[("msgid", msgid.as_str())],
                    )
                    .await?;
                if let Some(c) = chunk.as_array() {
                    all.extend(c.iter().cloned());
                }
            }
        }
    }
    parse_symbol_map(&Value::Array(all))
}

/// Parse the flattened Eastmoney symbol-map array into rows.
pub(crate) fn parse_symbol_map(resp: &Value) -> Result<Vec<CoinFuturesSymbolRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "symbol map is not an array".into(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(CoinFuturesSymbolRow {
            mktid: fstr(item, "mktid"),
            mktname: fstr(item, "mktname"),
            name: fstr(item, "name"),
            code: fstr(item, "code"),
            vcode: fstr(item, "vcode"),
            vname: fstr(item, "vname"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn fstr(item: &Value, k: &str) -> String {
    fstr_opt(item, k).unwrap_or_default()
}

fn fstr_opt(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.replace(',', "").parse::<f64>().ok(),
        _ => None,
    })
}

fn fnum_str(s: &str) -> Option<f64> {
    let t = s.replace(',', "");
    if t.is_empty() || t == "-" {
        None
    } else {
        t.parse::<f64>().ok()
    }
}

fn inum(item: &Value, k: &str) -> Option<i64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.replace(',', "").parse::<i64>().ok(),
        _ => None,
    })
}

// ---------------------------------------------------------------------------
// tests (offline fixtures)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let txt = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    fn fixture_text(name: &str) -> String {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        std::fs::read_to_string(path).unwrap()
    }

    #[test]
    fn parses_coin_lme_realtime_fixture() {
        let txt = fixture_text("coin_lme_realtime.txt");
        let rows = parse_coin_lme_realtime(&txt).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].code, "CAD");
        assert_eq!(rows[0].name, "伦敦铜");
        assert_eq!(rows[0].current_price, Some(7321.0));
        assert_eq!(rows[0].current_price_rmb, Some(52780.0));
        assert_eq!(rows[0].high, Some(7345.0));
        assert_eq!(rows[0].date, "2024-01-02");
        // XAU (London gold) has 14 fields -> RMB column padded to None.
        assert_eq!(rows[2].code, "XAU");
        assert_eq!(rows[2].current_price_rmb, None);
        assert_eq!(rows[2].current_price, Some(2045.30));
    }

    #[test]
    fn parses_coin_shfe_rank_fixture() {
        let v = fixture("coin_shfe_rank.json");
        let rows = parse_coin_shfe_rank(&v, "20240509").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].rank, Some(1));
        assert_eq!(rows[0].vol_party_name, "永安期货");
        assert_eq!(rows[0].vol, Some(12345.0));
        assert_eq!(rows[0].long_open_interest, Some(23456.0));
        assert_eq!(rows[0].short_open_interest_chg, Some(200.0));
        assert_eq!(rows[0].symbol, "cu2410");
        assert_eq!(rows[0].variety, "cu");
        assert_eq!(rows[0].product, "铜");
        assert_eq!(rows[0].date, "20240509");
        // "-" parses to None.
        assert_eq!(rows[0].long_open_interest_chg, Some(-50.0));
    }

    #[test]
    fn parses_coin_foreign_hist_fixture() {
        let v = fixture("coin_foreign_hist.json");
        let rows = parse_kline(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "HG00Y");
        assert_eq!(rows[0].name, "COMEX铜");
        assert_eq!(rows[0].date, "2024-01-02");
        assert_eq!(rows[0].open, Some(3.85));
        assert_eq!(rows[0].close, Some(3.92));
        assert_eq!(rows[0].high, Some(3.95));
        assert_eq!(rows[0].volume, Some(120000.0));
        assert_eq!(rows[0].open_interest, Some(220000.0));
        assert_eq!(rows[0].position_chg, Some(1500.0));
        assert_eq!(rows[0].change_pct, Some(1.83));
    }

    #[test]
    fn parses_coin_futures_hist_fixture() {
        let v = fixture("coin_futures_hist.json");
        let rows = parse_kline(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "al2505");
        assert_eq!(rows[0].name, "沪铝2505");
        assert_eq!(rows[0].open, Some(19000.0));
        assert_eq!(rows[0].close, Some(19100.0));
        assert_eq!(rows[0].open_interest, Some(300000.0));
        // Domestic kline keeps position_chg at index 13 (0.0 for the fixture row).
        assert_eq!(rows[0].position_chg, Some(0.0));
        assert_eq!(rows[1].change, Some(-50.0));
    }

    #[test]
    fn parses_coin_futures_symbol_map_fixture() {
        let v = fixture("coin_futures_symbol_map.json");
        let rows = parse_symbol_map(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].mktid, "114");
        assert_eq!(rows[0].mktname, "上海期货交易所");
        assert_eq!(rows[0].name, "沪铜2505");
        assert_eq!(rows[0].code, "cu2505");
        assert_eq!(rows[0].vcode, "CU2505");
        assert_eq!(rows[0].vname, "铜");
    }

    #[test]
    fn foreign_market_code_table() {
        assert_eq!(foreign_market_code("HG00Y"), Some(101));
        assert_eq!(foreign_market_code("GC00Y"), Some(101));
        assert_eq!(foreign_market_code("CL00Y"), Some(102));
        // LME 3-month names are not supported by akshare's history endpoint.
        assert_eq!(foreign_market_code("CAD"), None);
    }
}
