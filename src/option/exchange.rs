//! Exchange option quotes (akshare `option_czce.py`, `option_em.py`,
//! `option_finance.py`).
//!
//! Ports of exchange-level option functions that return either pure JSON or a
//! simple delimited text page (parseable with `get_json` / `get_text` + a
//! trivial parser). Mapping of each Rust fn to its akshare source line:
//!
//! | Rust fn | akshare fn | source line | status |
//! | --- | --- | --- | --- |
//! | `option_hist_yearly_czce` | `option_hist_yearly_czce` | `option_czce.py:37` | implemented |
//! | `option_finance_sse_underlying` | `option_finance_sse_underlying` | `option_finance.py:34` | implemented |
//! | `option_finance_board_sse` | `option_finance_board` (SSE king branch) | `option_finance.py:72` | implemented |
//!
//! ## ALREADY PORTED ELSEWHERE
//!
//! - `option_current_cffex_em` (`option_em.py:112`) — already ported in
//!   `src/option/extra.rs` (it shares the `option_current_cffex_em.json`
//!   fixture), so it is intentionally NOT re-implemented here to avoid
//!   duplication. Use `crate::option::extra::option_current_cffex_em`.
//!
//! ## DEFERRED
//!
//! - `option_hist_dce` (`option_commodity.py:32`) — DCE option daily history is
//!   fetched with a **JSON-body POST** (`requests.post(url, json=payload)`). The
//!   shared `Client` only exposes GET and form-encoded POST (`post_form_json`),
//!   so a faithful port is not possible without editing `client.rs`.
//! - `option_hist_czce` (`option_commodity.py:187`) — CZCE option *daily*
//!   history returns a pipe-`|`-delimited `OptionDataDaily.txt` page; akshare
//!   scrapes it with `pd.read_table(sep="|")`. It is already a deferred stub in
//!   `commodity.rs`. (The CZCE *yearly* variant IS ported here because its
//!   `OptionDataAllHistory/{symbol}OPTIONS{year}.txt` page is the same shape and
//!   is fetched as plain text.)
//! - `option_current_em` (`option_em.py:14`) — requires multi-page pagination via
//!   akshare's `fetch_paginated_data` (it walks all pages of a paged Eastmoney
//!   list); not a single request and the pager is not available client-side.
//! - `option_finance_board` (`option_finance.py:72`) — the SZSE branch is
//!   **paginated JSON** (`SHOWTYPE=JSON` paging) and the three CFFEX index-option
//!   branches parse comma-separated `.txt` files (`quote_MO.txt` / `quote_HO.txt`
//!   / `CFFEX_OPTION_URL_300`) whose CSV schema is not declared by akshare and
//!   would have to be guessed. The SSE "king" branch (clean positional JSON) is
//!   ported as `option_finance_board_sse`; the rest are deferred.

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use serde_json::Value;

const SOURCE_CZCE: &str = "czce";
const SOURCE_SSE: &str = "sse";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Get a numeric field by column index from a `|`-split text row.
fn field_num(fields: &[&str], i: Option<usize>) -> Option<f64> {
    i.and_then(|i| fields.get(i))
        .and_then(|s| s.replace(',', "").parse::<f64>().ok())
}

/// Get a string field by column index from a `|`-split text row.
fn field_str(fields: &[&str], i: Option<usize>) -> Option<String> {
    i.and_then(|i| fields.get(i)).map(|s| s.trim().to_string())
}

// ---------------------------------------------------------------------------
// 1. CZCE yearly option history (option_czce.py:37) — `|`-delimited text
// ---------------------------------------------------------------------------

/// A single CZCE option contract yearly-history row (`option_hist_yearly_czce`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CzceYearlyOptionRow {
    /// 合约代码 (contract code, e.g. `"SR311C6000"`)
    pub contract_code: String,
    /// 昨结算 (previous settlement)
    pub pre_settlement: Option<f64>,
    /// 今开盘 (open)
    pub open: Option<f64>,
    /// 最高价 (high)
    pub high: Option<f64>,
    /// 最低价 (low)
    pub low: Option<f64>,
    /// 今收盘 (close)
    pub close: Option<f64>,
    /// 今结算 (settlement)
    pub settlement: Option<f64>,
    /// 涨跌1 (change 1)
    pub chg1: Option<f64>,
    /// 涨跌2 (change 2)
    pub chg2: Option<f64>,
    /// 成交量(手) (volume, lots)
    pub volume: Option<f64>,
    /// 持仓量 (open interest)
    pub open_interest: Option<f64>,
    /// 增减量 (open-interest change)
    pub oi_chg: Option<f64>,
    /// 成交额(万元) (turnover, 10k CNY)
    pub turnover: Option<f64>,
    /// DELTA
    pub delta: Option<f64>,
    /// 隐含波动率 (implied volatility)
    pub implied_vol: Option<f64>,
    /// 行权量 (exercise volume)
    pub exec_volume: Option<f64>,
}

/// CZCE option listing year (akshare `symbol_year_dict`).
fn czce_symbol_listing_year(symbol: &str) -> Option<i32> {
    match symbol {
        "SR" => Some(2017),
        "CF" | "TA" | "MA" => Some(2019),
        "RM" | "ZC" => Some(2020),
        "OI" | "PK" => Some(2022),
        "PX" | "SH" | "SA" | "PF" | "SM" | "SF" | "UR" | "AP" => Some(2023),
        "CJ" | "FG" | "PR" => Some(2024),
        _ => None,
    }
}

/// Pure parser for CZCE yearly option history (pipe-`|`-delimited text).
///
/// Mirrors akshare `pd.read_table(sep="|", skiprows=1)`: skips the title row and
/// treats the next row as the column header, then maps known column names to
/// struct fields. Aggregate rows (`小计` / `合计` / `总计`) are dropped because
/// they are not contract quotes (akshare's yearly variant keeps them, but they
/// only carry totals and would pollute the result with a `合计` contract code).
pub fn parse_czce_yearly(text: &str) -> Vec<CzceYearlyOptionRow> {
    let mut lines = text
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.is_empty() && !l.chars().all(|c| c == '-'));
    // skiprows=1: first line is the page title; second line is the header.
    let _title = lines.next();
    let header = match lines.next() {
        Some(h) => h,
        None => return Vec::new(),
    };
    let headers: Vec<&str> = header.split('|').map(|s| s.trim()).collect();
    let pos = |name: &str| headers.iter().position(|h| *h == name);
    let c = pos("合约代码");
    let pre = pos("昨结算");
    let open = pos("今开盘");
    let high = pos("最高价");
    let low = pos("最低价");
    let close = pos("今收盘");
    let settle = pos("今结算");
    let chg1 = pos("涨跌1");
    let chg2 = pos("涨跌2");
    let vol = pos("成交量(手)");
    let oi = pos("持仓量");
    let oi_chg = pos("增减量");
    let turnover = pos("成交额(万元)");
    let delta = pos("DELTA");
    let iv = pos("隐含波动率");
    let exec = pos("行权量");

    let mut out = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
        let first = fields.first().map(|s| s.trim()).unwrap_or("");
        if first.is_empty() || first == "小计" || first == "合计" || first == "总计" {
            continue;
        }
        let code = match field_str(&fields, c) {
            Some(s) if !s.is_empty() => s,
            _ => continue,
        };
        out.push(CzceYearlyOptionRow {
            contract_code: code,
            pre_settlement: field_num(&fields, pre),
            open: field_num(&fields, open),
            high: field_num(&fields, high),
            low: field_num(&fields, low),
            close: field_num(&fields, close),
            settlement: field_num(&fields, settle),
            chg1: field_num(&fields, chg1),
            chg2: field_num(&fields, chg2),
            volume: field_num(&fields, vol),
            open_interest: field_num(&fields, oi),
            oi_chg: field_num(&fields, oi_chg),
            turnover: field_num(&fields, turnover),
            delta: field_num(&fields, delta),
            implied_vol: field_num(&fields, iv),
            exec_volume: field_num(&fields, exec),
        });
    }
    out
}

/// CZCE yearly option history (akshare `option_hist_yearly_czce`).
///
/// GETs `http://www.czce.com.cn/cn/DFSStaticFiles/Option/{year}/OptionDataAllHistory/{symbol}OPTIONS{year}.txt`
/// (pipe-delimited text) and parses it. Symbols not yet listed in `year` return
/// an empty vec, mirroring akshare's warning + empty frame.
pub async fn option_hist_yearly_czce(
    client: &Client,
    symbol: &str,
    year: &str,
) -> Result<Vec<CzceYearlyOptionRow>> {
    let year_i: i32 = year.parse().map_err(|_| {
        Error::InvalidParam(format!("year must be a 4-digit year, got {year}"))
    })?;
    if let Some(listed) = czce_symbol_listing_year(symbol)
        && listed > year_i
    {
        return Ok(Vec::new());
    }
    let url = format!(
        "http://www.czce.com.cn/cn/DFSStaticFiles/Option/{year}/OptionDataAllHistory/{symbol}OPTIONS{year}.txt"
    );
    let text = client
        .get_text(SOURCE_CZCE, "option_hist_yearly_czce", &url, &[], None)
        .await?;
    Ok(parse_czce_yearly(&text))
}

// ---------------------------------------------------------------------------
// 2. SSE option underlying ETF spot (option_finance.py:34) — positional JSON
// ---------------------------------------------------------------------------

/// SSE option underlying ETF spot row (`option_finance_sse_underlying`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SseUnderlyingOptionRow {
    /// 代码 (code)
    pub code: Option<String>,
    /// 名称 (name)
    pub name: Option<String>,
    /// 当前价 (last)
    pub last: Option<f64>,
    /// 涨跌 (change)
    pub change: Option<f64>,
    /// 涨跌幅 (chg_rate)
    pub chg_rate: Option<f64>,
    /// 振幅 (amp_rate)
    pub amp_rate: Option<f64>,
    /// 成交量(手) (volume)
    pub volume: Option<f64>,
    /// 成交额(万元) (amount)
    pub amount: Option<f64>,
    /// 昨收 (prev_close)
    pub prev_close: Option<f64>,
    /// 更新日期时间 (date+time snapshot, e.g. `"20240102153000"`)
    pub update_time: Option<String>,
}

/// Map an SSE underlying symbol to its `yunhq.sse.com.cn` URL.
fn sse_underlying_url(symbol: &str) -> Result<&'static str> {
    Ok(match symbol {
        "华夏上证50ETF期权" => "http://yunhq.sse.com.cn:32041/v1/sh1/list/self/510050",
        "华泰柏瑞沪深300ETF期权" => "http://yunhq.sse.com.cn:32041/v1/sh1/list/self/510300",
        "南方中证500ETF期权" => "http://yunhq.sse.com.cn:32041/v1/sh1/list/self/510500",
        "华夏科创50ETF期权" => "http://yunhq.sse.com.cn:32041/v1/sh1/list/self/588000",
        "易方达科创50ETF期权" => "http://yunhq.sse.com.cn:32041/v1/sh1/list/self/588080",
        _ => {
            return Err(Error::InvalidParam(format!(
                "unsupported SSE underlying symbol: {symbol}"
            )))
        }
    })
}

/// Pure parser for SSE option underlying spot (`list` of positional arrays).
///
/// The upstream `list` is positional (select `code,name,last,change,chg_rate,
/// amp_rate,volume,amount,prev_close`). Unlike akshare we keep the real `code`
/// from each row and do NOT overwrite row 0 with the hardcoded `"510300"`; we
/// also populate `update_time` for every row from the top-level `date`+`time`.
pub fn parse_sse_underlying(json: &Value) -> Vec<SseUnderlyingOptionRow> {
    let Some(list) = json.get("list").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    let update = match (
        json.get("date").and_then(|v| v.as_str()),
        json.get("time").and_then(|v| v.as_str()),
    ) {
        (Some(d), Some(t)) => Some(format!("{d}{t}")),
        _ => None,
    };
    let get_str = |arr: &[Value], i: usize| arr.get(i).and_then(|v| v.as_str()).map(str::to_string);
    let get_num = |arr: &[Value], i: usize| -> Option<f64> {
        match arr.get(i) {
            Some(Value::Number(n)) => n.as_f64(),
            Some(Value::String(s)) => s.parse::<f64>().ok(),
            _ => None,
        }
    };
    let mut out = Vec::with_capacity(list.len());
    for row in list {
        let Some(arr) = row.as_array() else { continue };
        out.push(SseUnderlyingOptionRow {
            code: get_str(arr, 0),
            name: get_str(arr, 1),
            last: get_num(arr, 2),
            change: get_num(arr, 3),
            chg_rate: get_num(arr, 4),
            amp_rate: get_num(arr, 5),
            volume: get_num(arr, 6),
            amount: get_num(arr, 7),
            prev_close: get_num(arr, 8),
            update_time: update.clone(),
        });
    }
    out
}

/// SSE option underlying ETF spot (akshare `option_finance_sse_underlying`).
///
/// GETs the `yunhq.sse.com.cn` self-list URL for `symbol` with the
/// `SH_OPTION_PAYLOAD` select and parses the positional `list`.
pub async fn option_finance_sse_underlying(
    client: &Client,
    symbol: &str,
) -> Result<Vec<SseUnderlyingOptionRow>> {
    let url = sse_underlying_url(symbol)?;
    let params = [(
        "select",
        "select: code,name,last,change,chg_rate,amp_rate,volume,amount,prev_close",
    )];
    let json = client
        .get_json(SOURCE_SSE, "option_finance_sse_underlying", url, &params)
        .await?;
    Ok(parse_sse_underlying(&json))
}

// ---------------------------------------------------------------------------
// 3. SSE option board (current trading day) — positional JSON
//    (option_finance.py:72 SSE "king" branches)
// ---------------------------------------------------------------------------

/// A single SSE option board row (`option_finance_board_sse`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SseOptionBoardRow {
    /// 日期 (date+time snapshot, e.g. `"20240102153000"`)
    pub date: Option<String>,
    /// 合约交易代码 (contractid)
    pub contract_code: Option<String>,
    /// 当前价 (last)
    pub last_price: Option<f64>,
    /// 涨跌幅 (chg_rate)
    pub chg_rate: Option<f64>,
    /// 前结价 (presetpx)
    pub pre_settle: Option<f64>,
    /// 行权价 (exepx)
    pub exercise_price: Option<f64>,
    /// 数量 (total count for the day)
    pub total: Option<i64>,
}

/// Map an SSE ETF-option board symbol to its underlying code.
fn sse_board_code(symbol: &str) -> Result<&'static str> {
    Ok(match symbol {
        "华夏上证50ETF期权" => "510050",
        "华泰柏瑞沪深300ETF期权" => "510300",
        "南方中证500ETF期权" => "510500",
        "华夏科创50ETF期权" => "588000",
        "易方达科创50ETF期权" => "588080",
        _ => {
            return Err(Error::InvalidParam(format!(
                "unsupported SSE board symbol: {symbol}"
            )))
        }
    })
}

/// Pure parser for the SSE option board (`list` of positional arrays).
///
/// Upstream `list` is positional (select `contractid,last,chg_rate,presetpx,
/// exepx`). `date`+`time` become the snapshot `date`; top-level `total` is the
/// contract count for the day.
pub fn parse_sse_option_board(json: &Value) -> Vec<SseOptionBoardRow> {
    let Some(list) = json.get("list").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    let date = match (
        json.get("date").and_then(|v| v.as_str()),
        json.get("time").and_then(|v| v.as_str()),
    ) {
        (Some(d), Some(t)) => Some(format!("{d}{t}")),
        _ => None,
    };
    let total = json.get("total").and_then(|v| v.as_i64());
    let get_str = |arr: &[Value], i: usize| arr.get(i).and_then(|v| v.as_str()).map(str::to_string);
    let get_num = |arr: &[Value], i: usize| -> Option<f64> {
        match arr.get(i) {
            Some(Value::Number(n)) => n.as_f64(),
            Some(Value::String(s)) => s.parse::<f64>().ok(),
            _ => None,
        }
    };
    let mut out = Vec::with_capacity(list.len());
    for row in list {
        let Some(arr) = row.as_array() else { continue };
        out.push(SseOptionBoardRow {
            date: date.clone(),
            contract_code: get_str(arr, 0),
            last_price: get_num(arr, 1),
            chg_rate: get_num(arr, 2),
            pre_settle: get_num(arr, 3),
            exercise_price: get_num(arr, 4),
            total,
        });
    }
    out
}

/// SSE option board (current trading day) for an ETF option
/// (akshare `option_finance_board` SSE "king" branches).
///
/// GETs `http://yunhq.sse.com.cn:32041/v1/sho/list/tstyle/{code}_{MM}` (the last
/// two digits of `end_month`) with the `SH_OPTION_PAYLOAD_OTHER` select.
pub async fn option_finance_board_sse(
    client: &Client,
    symbol: &str,
    end_month: &str,
) -> Result<Vec<SseOptionBoardRow>> {
    let code = sse_board_code(symbol)?;
    let two = &end_month[end_month.len().saturating_sub(2)..];
    let url = format!("http://yunhq.sse.com.cn:32041/v1/sho/list/tstyle/{code}_{two}");
    let params = [("select", "contractid,last,chg_rate,presetpx,exepx")];
    let json = client
        .get_json(SOURCE_SSE, "option_finance_board_sse", &url, &params)
        .await?;
    Ok(parse_sse_option_board(&json))
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

    fn fixture_text(name: &str) -> String {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        std::fs::read_to_string(p).unwrap()
    }

    /// Tolerance float compare; never unwraps the `Option`.
    fn approx(a: Option<f64>, b: f64) -> bool {
        a.is_some_and(|x| (x - b).abs() < 1e-6)
    }

    #[test]
    fn parse_czce_yearly_skips_aggregate_and_strips_commas() {
        let text = fixture_text("option_hist_yearly_czce.txt");
        let rows = parse_czce_yearly(&text);
        // 2 contract rows; the 合计 aggregate row is dropped.
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].contract_code, "SR311C6000");
        assert!(approx(rows[0].open, 101.0));
        // comma thousands-separators are stripped.
        assert!(approx(rows[0].volume, 1234.0));
        assert!(approx(rows[0].open_interest, 5678.0));
        assert!(approx(rows[0].turnover, 123456.0));
        assert!(approx(rows[0].delta, 0.5));
        assert!(approx(rows[0].implied_vol, 0.25));
        assert!(approx(rows[0].exec_volume, 50.0));
        assert_eq!(rows[1].contract_code, "SR311P6000");
        assert!(approx(rows[1].delta, -0.3));
    }

    #[test]
    fn parse_sse_underlying_maps_positional_list() {
        let json = fixture("option_finance_sse_underlying.json");
        let rows = parse_sse_underlying(&json);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code.as_deref(), Some("510300"));
        assert_eq!(rows[0].name.as_deref(), Some("沪深300ETF"));
        assert!(approx(rows[0].last, 3.980));
        assert!(approx(rows[0].chg_rate, 0.51));
        assert!(approx(rows[0].prev_close, 3.960));
        // snapshot date comes from top-level date+time.
        assert_eq!(rows[0].update_time.as_deref(), Some("20240102153000"));
        assert_eq!(rows[1].update_time.as_deref(), Some("20240102153000"));
    }

    #[test]
    fn parse_sse_option_board_maps_positional_list() {
        let json = fixture("option_finance_board_sse.json");
        let rows = parse_sse_option_board(&json);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].contract_code.as_deref(), Some("10003900"));
        assert!(approx(rows[0].last_price, 3.500));
        assert!(approx(rows[0].chg_rate, 0.012));
        assert!(approx(rows[0].pre_settle, 3.480));
        assert!(approx(rows[0].exercise_price, 3.500));
        assert_eq!(rows[0].total, Some(2));
        assert_eq!(rows[0].date.as_deref(), Some("20240102153000"));
    }

    #[test]
    fn czce_listing_guard_returns_empty_before_listing() {
        assert_eq!(czce_symbol_listing_year("SR"), Some(2017));
        assert_eq!(czce_symbol_listing_year("PR"), Some(2024));
        assert_eq!(czce_symbol_listing_year("ZZ"), None);
    }
}
