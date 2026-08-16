//! Futures exchange settlement parameters (`futures_settle*`).
//!
//! Ports akshare `futures_settle.py`: per-exchange margin / fee / settlement
//! parameters published by each Chinese exchange. Response formats vary:
//! CFFEX is a CSV (GBK), CZCE is a pipe-delimited text file, GFEX is a JSON
//! POST, SHFE/INE are JSON (`o_cursor`), and `futures_settle` normalizes them
//! onto a single column layout (`SETTLE_OUTPUT_COLUMNS` in akshare).

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

// ---------------------------------------------------------------------------
// Per-exchange row types (akshare native column names)
// ---------------------------------------------------------------------------

/// CFFEX settlement parameters (`futures_settle_cffex`).
///
/// akshare columns: date, symbol, variety, long_margin_ratio,
/// short_margin_ratio, trade_fee_ratio, delivery_fee_ratio,
/// close_today_fee_ratio.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CffexSettleRow {
    pub date: String,
    pub symbol: String,
    pub variety: String,
    pub long_margin_ratio: Option<f64>,
    pub short_margin_ratio: Option<f64>,
    pub trade_fee_ratio: Option<f64>,
    pub delivery_fee_ratio: Option<f64>,
    pub close_today_fee_ratio: Option<f64>,
}

/// CZCE settlement parameters (`futures_settle_czce`).
///
/// akshare columns: date, symbol, variety, settle_price, is_single_market,
/// single_market_days, margin_ratio, limit_ratio, trade_fee, fee_type,
/// delivery_fee, close_today_fee, position_limit, trade_limit.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CzceSettleRow {
    pub date: String,
    pub symbol: String,
    pub variety: String,
    pub settle_price: Option<f64>,
    pub is_single_market: Option<String>,
    pub single_market_days: Option<String>,
    pub margin_ratio: Option<f64>,
    pub limit_ratio: Option<f64>,
    pub trade_fee: Option<String>,
    pub fee_type: Option<String>,
    pub delivery_fee: Option<String>,
    pub close_today_fee: Option<String>,
    pub position_limit: Option<f64>,
    pub trade_limit: Option<f64>,
}

/// GFEX settlement parameters (`futures_settle_gfex`).
///
/// akshare columns: date, symbol, variety, spec_buy_rate, spec_buy,
/// hedge_buy_rate, hedge_buy, rise_limit_rate, rise_limit, fall_limit,
/// agent_tot_buy_posi_quota, self_tot_buy_posi_quota, client_buy_posi_quota.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GfexSettleRow {
    pub date: String,
    pub symbol: String,
    pub variety: String,
    pub spec_buy_rate: Option<f64>,
    pub spec_buy: Option<f64>,
    pub hedge_buy_rate: Option<f64>,
    pub hedge_buy: Option<f64>,
    pub rise_limit_rate: Option<f64>,
    pub rise_limit: Option<f64>,
    pub fall_limit: Option<f64>,
    pub agent_tot_buy_posi_quota: Option<f64>,
    pub self_tot_buy_posi_quota: Option<f64>,
    pub client_buy_posi_quota: Option<f64>,
}

/// SHFE / INE settlement parameters (`futures_settle_shfe` / `futures_settle_ine`).
///
/// akshare columns: date, symbol, variety, settle_price,
/// spec_long_margin_ratio, hedge_long_margin_ratio, spec_short_margin_ratio,
/// hedge_short_margin_ratio, trade_fee_ratio, close_today_fee_ratio,
/// is_close_today.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShfeSettleRow {
    pub date: String,
    pub symbol: String,
    pub variety: String,
    pub settle_price: Option<f64>,
    pub spec_long_margin_ratio: Option<f64>,
    pub hedge_long_margin_ratio: Option<f64>,
    pub spec_short_margin_ratio: Option<f64>,
    pub hedge_short_margin_ratio: Option<f64>,
    pub trade_fee_ratio: Option<f64>,
    pub close_today_fee_ratio: Option<f64>,
    pub is_close_today: Option<String>,
}

/// Unified settlement parameters (`futures_settle`), mirroring akshare's
/// `SETTLE_OUTPUT_COLUMNS`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesSettleRow {
    pub date: String,
    pub symbol: String,
    pub variety: String,
    pub settle_price: Option<f64>,
    pub long_margin_ratio: Option<f64>,
    pub short_margin_ratio: Option<f64>,
    pub spec_long_margin_ratio: Option<f64>,
    pub spec_short_margin_ratio: Option<f64>,
    pub hedge_long_margin_ratio: Option<f64>,
    pub hedge_short_margin_ratio: Option<f64>,
    pub trade_fee_ratio: Option<f64>,
    pub close_today_fee_ratio: Option<f64>,
    pub delivery_fee_ratio: Option<f64>,
    pub is_single_market: Option<String>,
    pub single_market_days: Option<String>,
    pub limit_ratio: Option<f64>,
    pub position_limit: Option<f64>,
    pub trade_limit: Option<f64>,
    pub rise_limit_rate: Option<f64>,
    pub fall_limit_rate: Option<f64>,
}

// ---------------------------------------------------------------------------
// Public endpoints
// ---------------------------------------------------------------------------

/// CFFEX settlement parameters (`futures_settle_cffex`).
///
/// `date` is `YYYY-MM-DD` / `YYYYMMDD` (akshare default `20260119`).
pub async fn futures_settle_cffex(client: &Client, date: &str) -> Result<Vec<CffexSettleRow>> {
    let d = norm_date(date)?;
    let ym = &d[0..6];
    let day = &d[6..8];
    let url = format!("http://www.cffex.com.cn/sj/jscs/{ym}/{day}/{d}_1.csv");
    let text = client
        .get_text("cffex", "futures_settle_cffex", &url, &[], None)
        .await?;
    parse_cffex_settle(&text, &d)
}

/// CZCE settlement parameters (`futures_settle_czce`).
pub async fn futures_settle_czce(client: &Client, date: &str) -> Result<Vec<CzceSettleRow>> {
    let d = norm_date(date)?;
    let url = format!(
        "http://www.czce.com.cn/cn/DFSStaticFiles/Future/{}/{}/FutureDataClearParams.txt",
        &d[0..4], d
    );
    let text = client
        .get_text("czce", "futures_settle_czce", &url, &[], None)
        .await?;
    parse_czce_settle(&text, &d)
}

/// GFEX settlement parameters (`futures_settle_gfex`).
pub async fn futures_settle_gfex(client: &Client, date: &str) -> Result<Vec<GfexSettleRow>> {
    let d = norm_date(date)?;
    let v = client
        .post_form_json(
            "gfex",
            "futures_settle_gfex",
            "http://www.gfex.com.cn/u/interfacesWebTtQueryTradPara/loadDayList",
            &[("trade_type", "0")],
            Some(GFEX_HEADERS),
        )
        .await?;
    parse_gfex_settle(&v, &d)
}

/// SHFE settlement parameters (`futures_settle_shfe`).
pub async fn futures_settle_shfe(client: &Client, date: &str) -> Result<Vec<ShfeSettleRow>> {
    let d = norm_date(date)?;
    let url = format!("https://www.shfe.com.cn/data/tradedata/future/dailydata/js{d}.dat");
    let v = client
        .get_json("shfe", "futures_settle_shfe", &url, &[])
        .await?;
    parse_shfe_settle(&v, &d)
}

/// INE settlement parameters (`futures_settle_ine`).
pub async fn futures_settle_ine(client: &Client, date: &str) -> Result<Vec<ShfeSettleRow>> {
    let d = norm_date(date)?;
    let url = format!("https://www.ine.cn/data/tradedata/future/dailydata/js{d}.dat");
    let v = client
        .get_json("ine", "futures_settle_ine", &url, &[])
        .await?;
    parse_shfe_settle(&v, &d)
}

/// Unified settlement parameters across exchanges (`futures_settle`).
///
/// `market` is one of `CFFEX`, `CZCE`, `SHFE`, `GFEX`, `INE` (case-insensitive).
/// `DCE` is unsupported upstream (anti-bot 412) and returns an empty set.
pub async fn futures_settle(
    client: &Client,
    date: &str,
    market: &str,
) -> Result<Vec<FuturesSettleRow>> {
    let rows = match market.to_uppercase().as_str() {
        "CFFEX" => futures_settle_cffex(client, date)
            .await?
            .into_iter()
            .map(FuturesSettleRow::from)
            .collect(),
        "CZCE" => futures_settle_czce(client, date)
            .await?
            .into_iter()
            .map(FuturesSettleRow::from)
            .collect(),
        "GFEX" => futures_settle_gfex(client, date)
            .await?
            .into_iter()
            .map(FuturesSettleRow::from)
            .collect(),
        "SHFE" => futures_settle_shfe(client, date)
            .await?
            .into_iter()
            .map(FuturesSettleRow::from)
            .collect(),
        "INE" => futures_settle_ine(client, date)
            .await?
            .into_iter()
            .map(FuturesSettleRow::from)
            .collect(),
        _ => Vec::new(),
    };
    Ok(rows)
}

// ---------------------------------------------------------------------------
// Parsers
// ---------------------------------------------------------------------------

/// Normalize a date arg (`YYYY-MM-DD` or `YYYYMMDD`) to `YYYYMMDD`.
fn norm_date(date: &str) -> Result<String> {
    let d = date.replace('-', "");
    if d.len() != 8 || !d.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::InvalidParam("date must be YYYY-MM-DD or YYYYMMDD".into()));
    }
    Ok(d)
}

fn variety_of(symbol: &str) -> String {
    symbol
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect()
}

fn parse_f64(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        t.replace(',', "").parse::<f64>().ok()
    }
}

fn parse_f64_val(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => parse_f64(s),
        _ => None,
    }
}

/// Parse CFFEX `jscs` CSV (skip first line; 6 columns after rename).
pub(crate) fn parse_cffex_settle(text: &str, date: &str) -> Result<Vec<CffexSettleRow>> {
    let lines: Vec<&str> = text.lines().map(|l| l.trim_end()).collect();
    let mut out = Vec::new();
    // lines[0] is a title; lines[1] is the header; data from lines[2].
    for line in lines.iter().skip(2) {
        if line.trim().is_empty() {
            continue;
        }
        let p: Vec<&str> = line.split(',').collect();
        if p.len() < 6 {
            continue;
        }
        let symbol = p[0].trim().to_string();
        if symbol.is_empty() || !symbol.starts_with(|c: char| c.is_ascii_uppercase()) {
            continue;
        }
        out.push(CffexSettleRow {
            date: date.to_string(),
            variety: variety_of(&symbol),
            symbol,
            long_margin_ratio: parse_f64(p[1]),
            short_margin_ratio: parse_f64(p[2]),
            trade_fee_ratio: parse_f64(p[3]),
            delivery_fee_ratio: parse_f64(p[4]),
            close_today_fee_ratio: parse_f64(p[5]),
        });
    }
    Ok(out)
}

/// Parse CZCE pipe-delimited `FutureDataClearParams.txt` (skip first line).
pub(crate) fn parse_czce_settle(text: &str, date: &str) -> Result<Vec<CzceSettleRow>> {
    let lines: Vec<&str> = text.lines().map(|l| l.trim_end()).collect();
    let mut out = Vec::new();
    // lines[0] is a title; lines[1] is the header; data from lines[2].
    for line in lines.iter().skip(2) {
        if line.trim().is_empty() {
            continue;
        }
        let p: Vec<&str> = line.split('|').collect();
        if p.len() < 12 {
            continue;
        }
        let symbol = p[0].trim().to_string();
        if symbol.is_empty()
            || symbol.contains("小计")
            || symbol.contains("合计")
            || symbol.contains("总计")
        {
            continue;
        }
        out.push(CzceSettleRow {
            date: date.to_string(),
            variety: variety_of(&symbol),
            symbol,
            settle_price: parse_f64(p[1]),
            is_single_market: Some(p[2].trim().to_string()),
            single_market_days: Some(p[3].trim().to_string()),
            margin_ratio: parse_f64(p[4]),
            limit_ratio: parse_f64(p[5]),
            trade_fee: Some(p[6].trim().to_string()),
            fee_type: Some(p[7].trim().to_string()),
            delivery_fee: Some(p[8].trim().to_string()),
            close_today_fee: Some(p[9].trim().to_string()),
            position_limit: parse_f64(p[10]),
            trade_limit: parse_f64(p[11]),
        });
    }
    Ok(out)
}

/// Parse GFEX `loadDayList` JSON, dropping option contracts (`-` in id).
pub(crate) fn parse_gfex_settle(resp: &Value, date: &str) -> Result<Vec<GfexSettleRow>> {
    let code = resp.get("code").and_then(|c| c.as_str()).unwrap_or("");
    if code != "0" {
        return Ok(Vec::new());
    }
    let list = resp
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: "gfex",
            message: "missing data".into(),
        })?;
    let mut out = Vec::new();
    for item in list {
        let symbol = item
            .get("contractId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if symbol.is_empty() || symbol.contains('-') {
            continue;
        }
        out.push(GfexSettleRow {
            date: date.to_string(),
            variety: variety_of(&symbol),
            symbol,
            spec_buy_rate: parse_f64_val(item.get("specBuyRate").unwrap_or(&Value::Null)),
            spec_buy: parse_f64_val(item.get("specBuy").unwrap_or(&Value::Null)),
            hedge_buy_rate: parse_f64_val(item.get("hedgeBuyRate").unwrap_or(&Value::Null)),
            hedge_buy: parse_f64_val(item.get("hedgeBuy").unwrap_or(&Value::Null)),
            rise_limit_rate: parse_f64_val(item.get("riseLimitRate").unwrap_or(&Value::Null)),
            rise_limit: parse_f64_val(item.get("riseLimit").unwrap_or(&Value::Null)),
            fall_limit: parse_f64_val(item.get("fallLimit").unwrap_or(&Value::Null)),
            agent_tot_buy_posi_quota: parse_f64_val(
                item.get("agentTotBuyPosiQuota").unwrap_or(&Value::Null),
            ),
            self_tot_buy_posi_quota: parse_f64_val(
                item.get("selfTotBuyPosiQuota").unwrap_or(&Value::Null),
            ),
            client_buy_posi_quota: parse_f64_val(
                item.get("clientBuyPosiQuota").unwrap_or(&Value::Null),
            ),
        });
    }
    Ok(out)
}

/// Parse SHFE / INE `js{date}.dat` JSON (`o_cursor`).
pub(crate) fn parse_shfe_settle(resp: &Value, date: &str) -> Result<Vec<ShfeSettleRow>> {
    let list = resp
        .get("o_cursor")
        .and_then(|c| c.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: "shfe",
            message: "missing o_cursor".into(),
        })?;
    let mut out = Vec::new();
    for item in list {
        let symbol = item
            .get("PRODUCTID")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if symbol.is_empty() {
            continue;
        }
        out.push(ShfeSettleRow {
            date: date.to_string(),
            variety: variety_of(&symbol),
            symbol,
            settle_price: parse_f64_val(item.get("SETTLEMENTPRICE").unwrap_or(&Value::Null)),
            spec_long_margin_ratio: parse_f64_val(
                item.get("SPECLONGMARGINRATIO").unwrap_or(&Value::Null),
            ),
            hedge_long_margin_ratio: parse_f64_val(
                item.get("HEDGLONGMARGINRATIO").unwrap_or(&Value::Null),
            ),
            spec_short_margin_ratio: parse_f64_val(
                item.get("SPECSHORTMARGINRATIO").unwrap_or(&Value::Null),
            ),
            hedge_short_margin_ratio: parse_f64_val(
                item.get("HEDGSHORTMARGINRATIO").unwrap_or(&Value::Null),
            ),
            trade_fee_ratio: parse_f64_val(item.get("TRADEFEERATIO").unwrap_or(&Value::Null)),
            close_today_fee_ratio: parse_f64_val(item.get("TTRADEFEERATIO").unwrap_or(&Value::Null)),
            is_close_today: item
                .get("ISCLOSETODAY")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        });
    }
    Ok(out)
}

const GFEX_HEADERS: &[(&str, &str)] = &[
    ("Accept", "application/json, text/javascript, */*; q=0.01"),
    ("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8"),
    ("Content-Type", "application/x-www-form-urlencoded; charset=UTF-8"),
    ("Origin", "http://www.gfex.com.cn"),
    ("Referer", "http://www.gfex.com.cn/gfex/rjycs/ywcs.shtml"),
    (
        "User-Agent",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36",
    ),
    ("X-Requested-With", "XMLHttpRequest"),
];

// ---------------------------------------------------------------------------
// Normalize to unified row
// ---------------------------------------------------------------------------

impl From<CffexSettleRow> for FuturesSettleRow {
    fn from(r: CffexSettleRow) -> Self {
        FuturesSettleRow {
            date: r.date,
            symbol: r.symbol,
            variety: r.variety,
            settle_price: None,
            long_margin_ratio: r.long_margin_ratio,
            short_margin_ratio: r.short_margin_ratio,
            spec_long_margin_ratio: None,
            spec_short_margin_ratio: None,
            hedge_long_margin_ratio: None,
            hedge_short_margin_ratio: None,
            trade_fee_ratio: r.trade_fee_ratio,
            close_today_fee_ratio: r.close_today_fee_ratio,
            delivery_fee_ratio: r.delivery_fee_ratio,
            is_single_market: None,
            single_market_days: None,
            limit_ratio: None,
            position_limit: None,
            trade_limit: None,
            rise_limit_rate: None,
            fall_limit_rate: None,
        }
    }
}

impl From<CzceSettleRow> for FuturesSettleRow {
    fn from(r: CzceSettleRow) -> Self {
        FuturesSettleRow {
            date: r.date,
            symbol: r.symbol,
            variety: r.variety,
            settle_price: r.settle_price,
            long_margin_ratio: r.margin_ratio,
            short_margin_ratio: None,
            spec_long_margin_ratio: None,
            spec_short_margin_ratio: None,
            hedge_long_margin_ratio: None,
            hedge_short_margin_ratio: None,
            trade_fee_ratio: r.trade_fee.and_then(|s| s.parse::<f64>().ok()),
            close_today_fee_ratio: r.close_today_fee.and_then(|s| s.parse::<f64>().ok()),
            delivery_fee_ratio: r.delivery_fee.and_then(|s| s.parse::<f64>().ok()),
            is_single_market: r.is_single_market,
            single_market_days: r.single_market_days,
            limit_ratio: r.limit_ratio,
            position_limit: r.position_limit,
            trade_limit: r.trade_limit,
            rise_limit_rate: None,
            fall_limit_rate: None,
        }
    }
}

impl From<GfexSettleRow> for FuturesSettleRow {
    fn from(r: GfexSettleRow) -> Self {
        FuturesSettleRow {
            date: r.date,
            symbol: r.symbol,
            variety: r.variety,
            settle_price: None,
            long_margin_ratio: r.spec_buy_rate,
            short_margin_ratio: r.hedge_buy_rate,
            spec_long_margin_ratio: r.spec_buy_rate,
            spec_short_margin_ratio: r.spec_buy_rate,
            hedge_long_margin_ratio: r.hedge_buy_rate,
            hedge_short_margin_ratio: r.spec_buy_rate,
            trade_fee_ratio: None,
            close_today_fee_ratio: None,
            delivery_fee_ratio: None,
            is_single_market: None,
            single_market_days: None,
            limit_ratio: None,
            position_limit: None,
            trade_limit: None,
            rise_limit_rate: r.rise_limit_rate,
            fall_limit_rate: r.fall_limit,
        }
    }
}

impl From<ShfeSettleRow> for FuturesSettleRow {
    fn from(r: ShfeSettleRow) -> Self {
        FuturesSettleRow {
            date: r.date,
            symbol: r.symbol,
            variety: r.variety,
            settle_price: r.settle_price,
            long_margin_ratio: None,
            short_margin_ratio: None,
            spec_long_margin_ratio: r.spec_long_margin_ratio,
            spec_short_margin_ratio: r.spec_short_margin_ratio,
            hedge_long_margin_ratio: r.hedge_long_margin_ratio,
            hedge_short_margin_ratio: r.hedge_short_margin_ratio,
            trade_fee_ratio: r.trade_fee_ratio,
            close_today_fee_ratio: r.close_today_fee_ratio,
            delivery_fee_ratio: None,
            is_single_market: None,
            single_market_days: None,
            limit_ratio: None,
            position_limit: None,
            trade_limit: None,
            rise_limit_rate: None,
            fall_limit_rate: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests (offline fixtures)
// ---------------------------------------------------------------------------

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

    fn fixture_json(name: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn parses_cffex_settle_fixture() {
        let txt = fixture_text("futures_settle_cffex.csv");
        let rows = parse_cffex_settle(&txt, "20260119").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "IF2501");
        assert_eq!(rows[0].variety, "IF");
        assert_eq!(rows[0].long_margin_ratio, Some(10.0));
        assert_eq!(rows[0].short_margin_ratio, Some(10.0));
        assert_eq!(rows[0].trade_fee_ratio, Some(0.0023));
        assert_eq!(rows[0].delivery_fee_ratio, Some(0.002));
    }

    #[test]
    fn parses_czce_settle_fixture() {
        let txt = fixture_text("futures_settle_czce.txt");
        let rows = parse_czce_settle(&txt, "20260119").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "SR2505");
        assert_eq!(rows[0].variety, "SR");
        assert_eq!(rows[0].settle_price, Some(6000.0));
        assert_eq!(rows[0].margin_ratio, Some(5.0));
        assert_eq!(rows[0].limit_ratio, Some(4.0));
        assert_eq!(rows[0].fee_type.as_deref(), Some("定值"));
        assert_eq!(rows[0].position_limit, Some(10000.0));
    }

    #[test]
    fn parses_gfex_settle_fixture() {
        let v = fixture_json("futures_settle_gfex.json");
        let rows = parse_gfex_settle(&v, "20260119").unwrap();
        // Option contract (lc2505-C-...) is dropped.
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "lc2505");
        assert_eq!(rows[0].variety, "lc");
        assert_eq!(rows[0].spec_buy_rate, Some(5.0));
        assert_eq!(rows[0].hedge_buy_rate, Some(5.0));
        assert_eq!(rows[0].rise_limit_rate, Some(4.0));
        assert_eq!(rows[0].client_buy_posi_quota, Some(10000.0));
    }

    #[test]
    fn parses_shfe_settle_fixture() {
        let v = fixture_json("futures_settle_shfe.json");
        let rows = parse_shfe_settle(&v, "20260119").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "cu");
        assert_eq!(rows[0].variety, "cu");
        assert_eq!(rows[0].settle_price, Some(69000.0));
        assert_eq!(rows[0].spec_long_margin_ratio, Some(5.0));
        assert_eq!(rows[0].hedge_long_margin_ratio, Some(5.0));
        assert_eq!(rows[0].trade_fee_ratio, Some(0.0001));
        assert_eq!(rows[0].close_today_fee_ratio, Some(0.0));
    }

    #[test]
    fn parses_ine_settle_fixture() {
        let v = fixture_json("futures_settle_ine.json");
        let rows = parse_shfe_settle(&v, "20250117").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "sc");
        assert_eq!(rows[0].variety, "sc");
        assert_eq!(rows[0].settle_price, Some(600.0));
        assert_eq!(rows[0].spec_long_margin_ratio, Some(10.0));
        assert_eq!(rows[0].hedge_short_margin_ratio, Some(10.0));
    }

    #[test]
    fn gfex_code_zero_required() {
        let v = serde_json::json!({"code": "1", "data": [{"contractId": "lc2505"}]});
        let rows = parse_gfex_settle(&v, "20260119").unwrap();
        assert!(rows.is_empty());
    }
}
