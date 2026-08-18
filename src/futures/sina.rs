//! Sina (新浪) domestic futures realtime spot & minute bars.
//!
//! Ports the two text-based Sina futures helpers from akshare's
//! `futures/futures_zh_sina.py`:
//! - `futures_zh_spot_sina`   ← `futures_zh_spot` (实时行情, `hq.sinajs.cn`)
//! - `futures_zh_minute_sina` ← `futures_zh_minute_sina` (分钟数据, `getFewMinLine`)
//!
//! Both endpoints are plain HTTP (no JS signing, no HTML scrape, no Excel/ZIP),
//! so they are source-resilient and fully portable. `futures_zh_spot_sina` is the
//! only Sina futures spot helper that does NOT need `futures_contract_detail`
//! enrichment — the upstream `hq.sinajs.cn` payload already carries every field.
//!
//! ## Functions ported in THIS file
//!
//! | Rust fn | akshare | status |
//! |---|---|---|
//! | `futures_zh_spot_sina` | `futures_zh_spot` (`futures_zh_sina.py:205`) | DONE |
//! | `futures_zh_minute_sina` | `futures_zh_minute_sina` (`futures_zh_sina.py:615`) | DONE |
//!
//! ## SKIPPED — already ported elsewhere in this crate
//!
//! These akshare Sina futures names were already implemented in sibling modules
//! (verified by `grep` before porting), so they are intentionally NOT re-ported
//! here to avoid duplication:
//!
//! * `futures_main_sina` — already ported as
//!   `crate::futures_derivative::sina::futures_main_sina`
//!   (`futures_index_sina.py:103`, `InnerFuturesNewService.getDailyKLine`).
//! * `futures_zh_daily_sina` — already ported as
//!   `crate::futures::extra::futures_zh_daily_sina`
//!   (`futures_zh_sina.py:651`, `InnerFuturesNewService.getDailyKLine`).
//!
//! ## DEFERRED (not ported here)
//!
//! * `futures_display_main_sina` — akshare uses `demjson` to leniently decode a
//!   JS-wrapped object, and the upstream `qihuohangqing.js` subscribe list is a
//!   JS document, not pure JSON. **Deferred**: not fakeable without a JS engine /
//!   `demjson`. NOTE: the equivalent list is already available as
//!   `crate::futures::main::futures_display` (ported without `demjson`).

use crate::core::client::{Client, SOURCE_SINA};
use crate::core::error::Result;
use crate::core::json::*;

/// `Referer` Sina expects on the realtime-quote and minute endpoints.
const SINA_HEADERS: &[(&str, &str)] = &[("Referer", "https://vip.stock.finance.sina.com.cn/")];

/// Realtime-quote host used by akshare's `futures_zh_spot`.
const SPOT_URL: &str = "https://hq.sinajs.cn/rn=";

/// Minute-K-line JSONP endpoint used by akshare's `futures_zh_minute_sina`.
const MINUTE_URL: &str =
    "https://stock2.finance.sina.com.cn/futures/api/jsonp.php/=/InnerFuturesNewService.getFewMinLine";

// ---------------------------------------------------------------------------
// futures_zh_spot_sina  (akshare `futures_zh_spot`, `hq.sinajs.cn`)
// ---------------------------------------------------------------------------

/// One realtime futures quote from Sina (`futures_zh_spot`).
///
/// Mirrors the commodity-futures (`market="CF"`) column layout akshare keeps:
/// 合约, 时间, 开盘, 最高, 最低, 昨收, 买价, 卖价, 最新价, 均价, 昨结算, 买量,
/// 卖量, 持仓量, 成交量. Non-commodity layouts share these leading fields, so the
/// same mapping is applied; per-contract `adjust="1"` enrichment
/// (`futures_contract_detail`) is intentionally **not** ported (see module doc).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesSpotSinaRow {
    /// 合约 (akshare `symbol`, e.g. `V2405`)
    pub symbol: String,
    /// 时间 (akshare `time`, `YYYY-MM-DD HH:MM:SS`)
    pub time: String,
    /// 开盘 (akshare `open`)
    pub open: Option<f64>,
    /// 最高 (akshare `high`)
    pub high: Option<f64>,
    /// 最低 (akshare `low`)
    pub low: Option<f64>,
    /// 昨收 (akshare `last_close`)
    pub last_close: Option<f64>,
    /// 买价 (akshare `bid_price`)
    pub bid_price: Option<f64>,
    /// 卖价 (akshare `ask_price`)
    pub ask_price: Option<f64>,
    /// 最新价 (akshare `current_price`)
    pub current_price: Option<f64>,
    /// 均价 (akshare `avg_price`)
    pub avg_price: Option<f64>,
    /// 昨结算 (akshare `last_settle_price`)
    pub last_settle_price: Option<f64>,
    /// 买量 (akshare `buy_vol`)
    pub buy_vol: Option<f64>,
    /// 卖量 (akshare `sell_vol`)
    pub sell_vol: Option<f64>,
    /// 持仓量 (akshare `hold`)
    pub hold: Option<f64>,
    /// 成交量 (akshare `volume`)
    pub volume: Option<f64>,
    /// Data origin (`sina`).
    pub source: &'static str,
}

/// Realtime futures spot quotes from Sina (`futures_zh_spot`).
///
/// `symbol` is a comma-separated list of contract codes (e.g. `"V2405"` or
/// `"V2405,V2409"`). `market` is accepted for API parity (`"CF"` commodity, the
/// akshare default) — only the commodity column layout is ported here.
pub async fn futures_zh_spot_sina(
    client: &Client,
    symbol: &str,
    market: &str,
) -> Result<Vec<FuturesSpotSinaRow>> {
    let _ = market; // commodity layout is the only one ported (see module doc)
    let subscribe: Vec<String> = symbol
        .split(',')
        .map(|s| format!("nf_{}", s.trim()))
        .collect();
    let list = subscribe.join(",");
    let rn = make_rn();
    let url = format!("{SPOT_URL}{rn}&list={list}");
    let text = client
        .get_text(SOURCE_SINA, "futures_zh_spot_sina", &url, &[], Some(SINA_HEADERS))
        .await?;
    parse_spot(&text)
}

/// Parse a raw `hq.sinajs.cn` response (`var hq_str_nf_X="...";` lines) into rows.
pub(crate) fn parse_spot(text: &str) -> Result<Vec<FuturesSpotSinaRow>> {
    let mut out = Vec::new();
    for line in text.split(';') {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // form: var hq_str_nf_SYMBOL="<csv>"
        let rhs = match line.split_once('=') {
            Some((_, r)) => r.trim(),
            None => continue,
        };
        let rhs = rhs.strip_prefix('"').unwrap_or(rhs);
        let rhs = rhs.strip_suffix('"').unwrap_or(rhs);
        let f: Vec<&str> = rhs.split(',').collect();
        if f.len() < 15 {
            continue;
        }
        out.push(FuturesSpotSinaRow {
            symbol: f[0].to_string(),
            time: f[1].to_string(),
            open: parse_f64_str(f[2]),
            high: parse_f64_str(f[3]),
            low: parse_f64_str(f[4]),
            last_close: parse_f64_str(f[5]),
            bid_price: parse_f64_str(f[6]),
            ask_price: parse_f64_str(f[7]),
            current_price: parse_f64_str(f[8]),
            avg_price: parse_f64_str(f[9]),
            last_settle_price: parse_f64_str(f[10]),
            buy_vol: parse_f64_str(f[11]),
            sell_vol: parse_f64_str(f[12]),
            hold: parse_f64_str(f[13]),
            volume: parse_f64_str(f[14]),
            source: SOURCE_SINA,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// futures_zh_minute_sina  (akshare `futures_zh_minute_sina`, `getFewMinLine`)
// ---------------------------------------------------------------------------

/// One minute bar of a Chinese futures contract from Sina (`futures_zh_minute_sina`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesMinRow {
    /// 时间 (akshare `datetime`, `YYYY-MM-DD HH:MM:SS`)
    pub datetime: String,
    /// 开盘 (akshare `open`)
    pub open: Option<f64>,
    /// 最高 (akshare `high`)
    pub high: Option<f64>,
    /// 最低 (akshare `low`)
    pub low: Option<f64>,
    /// 收盘 (akshare `close`)
    pub close: Option<f64>,
    /// 成交量 (akshare `volume`)
    pub volume: Option<f64>,
    /// 成交额 (akshare `amount`)
    pub amount: Option<f64>,
    /// Data origin (`sina`).
    pub source: &'static str,
}

/// Minute-K-line for a Chinese futures contract from Sina (`futures_zh_minute_sina`).
///
/// `symbol` is a Sina futures code such as `"IF2008"` or `"RB0"`; `period` is one
/// of `{"1","5","15","30","60"}` minutes. The upstream payload is parsed
/// line-by-line as CSV (`datetime,open,high,low,close,volume,amount`), matching
/// the crate contract requested for this port.
pub async fn futures_zh_minute_sina(
    client: &Client,
    symbol: &str,
    period: &str,
) -> Result<Vec<FuturesMinRow>> {
    let params = [("symbol", symbol), ("type", period)];
    let text = client
        .get_text(
            SOURCE_SINA,
            "futures_zh_minute_sina",
            MINUTE_URL,
            &params,
            Some(SINA_HEADERS),
        )
        .await?;
    parse_minute(&text)
}

/// Parse a raw line-by-line CSV (`datetime,open,high,low,close,volume,amount`)
/// Sina minute payload into [`FuturesMinRow`]s. A leading header row is skipped.
pub(crate) fn parse_minute(text: &str) -> Result<Vec<FuturesMinRow>> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let p: Vec<&str> = line.split(',').collect();
        if p.len() < 7 {
            continue;
        }
        if p[0].eq_ignore_ascii_case("datetime") {
            continue; // header row
        }
        out.push(FuturesMinRow {
            datetime: p[0].to_string(),
            open: parse_f64_str(p[1]),
            high: parse_f64_str(p[2]),
            low: parse_f64_str(p[3]),
            close: parse_f64_str(p[4]),
            volume: parse_f64_str(p[5]),
            amount: parse_f64_str(p[6]),
            source: SOURCE_SINA,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Generate the `rn` cache-buster akshare computes via `py_mini_racer`
/// (`Math.round(Math.random()*2147483648).toString(16)`). Any hex value works as
/// a cache-buster, so we derive one from the current nanosecond timestamp.
fn make_rn() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}", nanos % 0x8000_0000)
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_text(name: &str) -> String {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let path = dir.join("tests/fixtures").join(name);
        std::fs::read_to_string(path).unwrap()
    }

    fn approx(a: Option<f64>, b: f64) -> bool {
        a.is_some_and(|x| (x - b).abs() < 1e-9)
    }

    #[test]
    fn parses_futures_zh_spot_sina_fixture() {
        let text = fixture_text("futures_zh_spot_sina.txt");
        let rows = parse_spot(&text).unwrap();
        assert_eq!(rows.len(), 2);

        let v2405 = rows.iter().find(|r| r.symbol == "V2405").unwrap();
        assert_eq!(v2405.time, "2023-08-15 14:00:00");
        assert!(approx(v2405.open, 6280.0));
        assert!(approx(v2405.high, 6300.0));
        assert!(approx(v2405.low, 6250.0));
        assert!(approx(v2405.current_price, 6275.0));
        assert!(approx(v2405.last_close, 6200.0));
        assert!(approx(v2405.bid_price, 6270.0));
        assert!(approx(v2405.ask_price, 6280.0));
        assert!(approx(v2405.avg_price, 6260.0));
        assert!(approx(v2405.last_settle_price, 6250.0));
        assert!(approx(v2405.buy_vol, 100.0));
        assert!(approx(v2405.sell_vol, 200.0));
        assert!(approx(v2405.hold, 50000.0));
        assert!(approx(v2405.volume, 120000.0));
        assert_eq!(v2405.source, "sina");

        let v2409 = rows.iter().find(|r| r.symbol == "V2409").unwrap();
        assert!(approx(v2409.current_price, 6295.0));
        assert!(approx(v2409.volume, 130000.0));
    }

    #[test]
    fn parses_futures_zh_minute_sina_fixture() {
        let text = fixture_text("futures_zh_minute_sina.txt");
        let rows = parse_minute(&text).unwrap();
        assert_eq!(rows.len(), 3);

        let first = rows
            .iter()
            .find(|r| r.datetime == "2024-01-02 09:01:00")
            .unwrap();
        assert!(approx(first.open, 3000.0));
        assert!(approx(first.high, 3080.0));
        assert!(approx(first.low, 2980.0));
        assert!(approx(first.close, 3050.0));
        assert!(approx(first.volume, 120000.0));
        assert!(approx(first.amount, 1234567.0));
        assert_eq!(first.source, "sina");

        let third = rows
            .iter()
            .find(|r| r.datetime == "2024-01-02 09:03:00")
            .unwrap();
        assert!(approx(third.close, 3085.0));
    }
}
