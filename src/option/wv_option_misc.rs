//! Miscellaneous option endpoints (akshare `option` package).
//!
//! Rust ports assigned from akshare's `option` package that did not fit the
//! existing `eastmoney`/`sina`/`commodity`/`exchange` modules:
//!
//! | Rust fn | akshare fn | source line | status |
//! | --- | --- | --- | --- |
//! | `option_premium_analysis_em` | `option_premium_analysis_em` | `option_premium_analysis_em.py:14` | implemented |
//! | `option_finance_board` | `option_finance_board` | `option_finance.py:72` | implemented |
//! | `option_risk_analysis_em` | `option_risk_analysis_em` | `option_risk_analysis_em.py:14` | implemented |
//! | `option_contract_info_ctp` | `option_contract_info_ctp` | `option_contract_info_ctp.py:13` | implemented |
//! | `option_hist_czce` | `option_hist_czce` | `option_commodity.py:187` | implemented (real; see note) |
//! | `option_lhb_em` | `option_lhb_em` | `option_lhb_em.py:13` | implemented |
//! | `option_daily_stats_szse` | `option_daily_stats_szse` | `option_daily_stats_sse_szse.py:85` | implemented |
//! | `option_value_analysis_em` | `option_value_analysis_em` | `option_value_analysis_em.py:14` | implemented |
//!
//! ## Note on `option_hist_czce`
//!
//! `commodity.rs` ships a DEFERRED stub of the same name (it predates the
//! `get_text` + pipe-delimited parse path used here). This module provides the
//! real implementation; both live in separate modules so there is no symbol
//! collision. Prefer this one.
//!
//! ## Source-agnostic helpers
//!
//! `fstr`/`fnum`/`fdate` mirror the conventions in `commodity.rs` (tolerate
//! string-encoded numbers and comma thousands-separators). Eastmoney option
//! analysis endpoints share the `clist` pagination shape and are driven by
//! [`fetch_em_clist`].

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

const SOURCE_SSE: &str = "sse";
const SOURCE_SZSE: &str = "szse";
const SOURCE_CZCE: &str = "czce";
const SOURCE_CFFEX: &str = "cffex";
const SOURCE_OPENCTP: &str = "openctp";

const EM_CLIST_URL: &str = "https://push2.eastmoney.com/api/qt/clist/get";
const EM_UT: &str = "b2884a393a59ad64002292a3e90d46a5";

// ---------------------------------------------------------------------------
// Shared parse helpers
// ---------------------------------------------------------------------------

fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| match v {
        Value::String(s) => Some(s.to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    })
}

fn fnum(item: &Value, k: &str) -> Option<f64> {
    match item.get(k) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.replace(',', "").parse::<f64>().ok(),
        _ => None,
    }
}

/// Parse an Eastmoney `YYYYMMDD` (number or string) into `YYYY-MM-DD`.
fn fdate(item: &Value, k: &str) -> Option<String> {
    let raw = match item.get(k) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => return None,
    };
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() == 8 {
        Some(format!(
            "{}-{}-{}",
            &digits[0..4],
            &digits[4..6],
            &digits[6..8]
        ))
    } else {
        Some(raw)
    }
}

/// Fetch all pages of an Eastmoney `clist` report (akshare `fetch_paginated_data`).
///
/// The first page yields `data.total` and the per-page count `len(data.diff)`;
/// subsequent pages are fetched until exhausted.
async fn fetch_em_clist(
    client: &Client,
    endpoint: &'static str,
    fields: &str,
    fid: &str,
) -> Result<Vec<Value>> {
    let mut all = Vec::new();
    let mut page: u32 = 1;
    loop {
        let pn = page.to_string();
        let params = [
            ("fid", fid),
            ("po", "1"),
            ("pz", "100"),
            ("pn", pn.as_str()),
            ("np", "1"),
            ("fltt", "2"),
            ("invt", "2"),
            ("ut", EM_UT),
            ("fields", fields),
            ("fs", "m:10"),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, endpoint, EM_CLIST_URL, &params)
            .await?;
        let data = v.get("data").ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data".into(),
        })?;
        let diff = data
            .get("diff")
            .and_then(|d| d.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing data.diff".into(),
            })?;
        if diff.is_empty() {
            break;
        }
        for d in diff {
            all.push(d.clone());
        }
        let total = data.get("total").and_then(|t| t.as_u64()).unwrap_or(0);
        let per = diff.len() as u64;
        if per == 0 || page as u64 >= total.div_ceil(per) {
            break;
        }
        page += 1;
    }
    Ok(all)
}

// ---------------------------------------------------------------------------
// 1. option_premium_analysis_em (option_premium_analysis_em.py:14)
// ---------------------------------------------------------------------------

/// A single option premium/discount analysis row (`option_premium_analysis_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct OptionPremiumRow {
    pub code: Option<String>,
    pub name: Option<String>,
    pub price: Option<f64>,
    pub change_pct: Option<f64>,
    pub exercise_price: Option<f64>,
    pub premium_rate: Option<f64>,
    pub underlying_name: Option<String>,
    pub underlying_price: Option<f64>,
    pub underlying_change_pct: Option<f64>,
    pub breakeven: Option<f64>,
    pub expiry_date: Option<String>,
}

/// Option premium/discount analysis from Eastmoney (`option_premium_analysis_em`).
///
/// `fs=m:10` selects the option market; `fid=f250` sorts by exercise price.
pub async fn option_premium_analysis_em(client: &Client) -> Result<Vec<OptionPremiumRow>> {
    let fields = "f1,f2,f3,f12,f13,f14,f161,f250,f330,f331,f332,f333,f334,f335,f337,f301,f152";
    let diff = fetch_em_clist(client, "option_premium_analysis_em", fields, "f250").await?;
    Ok(diff.iter().map(parse_premium).collect())
}

fn parse_premium(d: &Value) -> OptionPremiumRow {
    OptionPremiumRow {
        code: fstr(d, "f12"),
        name: fstr(d, "f14"),
        price: fnum(d, "f2"),
        change_pct: fnum(d, "f3"),
        exercise_price: fnum(d, "f250"),
        premium_rate: fnum(d, "f330"),
        underlying_name: fstr(d, "f161"),
        underlying_price: fnum(d, "f301"),
        underlying_change_pct: fnum(d, "f152"),
        breakeven: fnum(d, "f335"),
        expiry_date: fdate(d, "f333"),
    }
}

// ---------------------------------------------------------------------------
// 2. option_risk_analysis_em (option_risk_analysis_em.py:14)
// ---------------------------------------------------------------------------

/// A single option risk-analysis row (`option_risk_analysis_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct OptionRiskRow {
    pub code: Option<String>,
    pub name: Option<String>,
    pub price: Option<f64>,
    pub change_pct: Option<f64>,
    pub leverage: Option<f64>,
    pub actual_leverage: Option<f64>,
    pub delta: Option<f64>,
    pub gamma: Option<f64>,
    pub vega: Option<f64>,
    pub rho: Option<f64>,
    pub theta: Option<f64>,
    pub expiry_date: Option<String>,
}

/// Option risk analysis (Greeks etc.) from Eastmoney (`option_risk_analysis_em`).
pub async fn option_risk_analysis_em(client: &Client) -> Result<Vec<OptionRiskRow>> {
    let fields = "f1,f2,f3,f12,f13,f14,f302,f303,f325,f326,f327,f329,f328,f301,f152,f154";
    let diff = fetch_em_clist(client, "option_risk_analysis_em", fields, "f12").await?;
    Ok(diff.iter().map(parse_risk).collect())
}

fn parse_risk(d: &Value) -> OptionRiskRow {
    OptionRiskRow {
        code: fstr(d, "f12"),
        name: fstr(d, "f14"),
        price: fnum(d, "f2"),
        change_pct: fnum(d, "f3"),
        leverage: fnum(d, "f302"),
        actual_leverage: fnum(d, "f303"),
        delta: fnum(d, "f325"),
        gamma: fnum(d, "f326"),
        vega: fnum(d, "f327"),
        rho: fnum(d, "f329"),
        theta: fnum(d, "f328"),
        expiry_date: fdate(d, "f154"),
    }
}

// ---------------------------------------------------------------------------
// 3. option_value_analysis_em (option_value_analysis_em.py:14)
// ---------------------------------------------------------------------------

/// A single option value-analysis row (`option_value_analysis_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct OptionValueRow {
    pub code: Option<String>,
    pub name: Option<String>,
    pub price: Option<f64>,
    pub time_value: Option<f64>,
    pub intrinsic_value: Option<f64>,
    pub implied_vol: Option<f64>,
    pub theoretical_price: Option<f64>,
    pub underlying_name: Option<String>,
    pub underlying_price: Option<f64>,
    pub underlying_yr_vol: Option<f64>,
    pub expiry_date: Option<String>,
}

/// Option value analysis (time/intrinsic value, IV, theoretical price) from
/// Eastmoney (`option_value_analysis_em`).
pub async fn option_value_analysis_em(client: &Client) -> Result<Vec<OptionValueRow>> {
    let fields = "f1,f2,f3,f12,f13,f14,f298,f299,f249,f300,f330,f331,f332,f333,f334,f335,f336,f301,f152";
    let diff = fetch_em_clist(client, "option_value_analysis_em", fields, "f301").await?;
    Ok(diff.iter().map(parse_value).collect())
}

fn parse_value(d: &Value) -> OptionValueRow {
    OptionValueRow {
        code: fstr(d, "f12"),
        name: fstr(d, "f14"),
        price: fnum(d, "f2"),
        time_value: fnum(d, "f298"),
        intrinsic_value: fnum(d, "f299"),
        implied_vol: fnum(d, "f249"),
        theoretical_price: fnum(d, "f300"),
        underlying_name: fstr(d, "f161"),
        underlying_price: fnum(d, "f301"),
        underlying_yr_vol: fnum(d, "f336"),
        expiry_date: fdate(d, "f333"),
    }
}

// ---------------------------------------------------------------------------
// 4. option_contract_info_ctp (option_contract_info_ctp.py:13)
// ---------------------------------------------------------------------------

/// A single CTP option contract row (`option_contract_info_ctp`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct OptionContractCtpRow {
    pub exchange_id: Option<String>,
    pub instrument_id: Option<String>,
    pub instrument_name: Option<String>,
    pub product_class: Option<String>,
    pub product_id: Option<String>,
    pub volume_multiple: Option<f64>,
    pub price_tick: Option<f64>,
    pub long_margin_ratio_by_money: Option<f64>,
    pub short_margin_ratio_by_money: Option<f64>,
    pub long_margin_ratio_by_volume: Option<f64>,
    pub short_margin_ratio_by_volume: Option<f64>,
    pub open_ratio_by_money: Option<f64>,
    pub open_ratio_by_volume: Option<f64>,
    pub close_ratio_by_money: Option<f64>,
    pub close_ratio_by_volume: Option<f64>,
    pub close_today_ratio_by_money: Option<f64>,
    pub close_today_ratio_by_volume: Option<f64>,
    pub delivery_year: Option<String>,
    pub delivery_month: Option<String>,
    pub open_date: Option<String>,
    pub expire_date: Option<String>,
    pub delivery_date: Option<String>,
    pub underlying_instr_id: Option<String>,
    pub underlying_multiple: Option<f64>,
    pub options_type: Option<String>,
    pub strike_price: Option<f64>,
    pub inst_life_phase: Option<String>,
}

/// Option contract metadata from openctp (`option_contract_info_ctp`).
///
/// GETs `http://dict.openctp.cn/instruments?types=option` and maps the raw
/// English field names to a typed row.
pub async fn option_contract_info_ctp(client: &Client) -> Result<Vec<OptionContractCtpRow>> {
    let url = "http://dict.openctp.cn/instruments?types=option";
    let v = client
        .get_json(SOURCE_OPENCTP, "option_contract_info_ctp", url, &[])
        .await?;
    let data = v
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_OPENCTP,
            message: "missing data".into(),
        })?;
    Ok(data.iter().map(parse_ctp).collect())
}

fn parse_ctp(d: &Value) -> OptionContractCtpRow {
    OptionContractCtpRow {
        exchange_id: fstr(d, "ExchangeID"),
        instrument_id: fstr(d, "InstrumentID"),
        instrument_name: fstr(d, "InstrumentName"),
        product_class: fstr(d, "ProductClass"),
        product_id: fstr(d, "ProductID"),
        volume_multiple: fnum(d, "VolumeMultiple"),
        price_tick: fnum(d, "PriceTick"),
        long_margin_ratio_by_money: fnum(d, "LongMarginRatioByMoney"),
        short_margin_ratio_by_money: fnum(d, "ShortMarginRatioByMoney"),
        long_margin_ratio_by_volume: fnum(d, "LongMarginRatioByVolume"),
        short_margin_ratio_by_volume: fnum(d, "ShortMarginRatioByVolume"),
        open_ratio_by_money: fnum(d, "OpenRatioByMoney"),
        open_ratio_by_volume: fnum(d, "OpenRatioByVolume"),
        close_ratio_by_money: fnum(d, "CloseRatioByMoney"),
        close_ratio_by_volume: fnum(d, "CloseRatioByVolume"),
        close_today_ratio_by_money: fnum(d, "CloseTodayRatioByMoney"),
        close_today_ratio_by_volume: fnum(d, "CloseTodayRatioByVolume"),
        delivery_year: fstr(d, "DeliveryYear"),
        delivery_month: fstr(d, "DeliveryMonth"),
        open_date: fstr(d, "OpenDate"),
        expire_date: fstr(d, "ExpireDate"),
        delivery_date: fstr(d, "DeliveryDate"),
        underlying_instr_id: fstr(d, "UnderlyingInstrID"),
        underlying_multiple: fnum(d, "UnderlyingMultiple"),
        options_type: fstr(d, "OptionsType"),
        strike_price: fnum(d, "StrikePrice"),
        inst_life_phase: fstr(d, "InstLifePhase"),
    }
}

// ---------------------------------------------------------------------------
// 5. option_hist_czce (option_commodity.py:187)
// ---------------------------------------------------------------------------

/// A single CZCE option daily-history row (`option_hist_czce`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct OptionCzceHistRow {
    pub contract_code: Option<String>,
    pub pre_settle: Option<f64>,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: Option<f64>,
    pub settle: Option<f64>,
    pub chg1: Option<f64>,
    pub chg2: Option<f64>,
    pub volume: Option<f64>,
    pub open_interest: Option<f64>,
    pub oi_chg: Option<f64>,
    pub turnover: Option<f64>,
    pub delta: Option<f64>,
    pub implied_vol: Option<f64>,
    pub exec_volume: Option<f64>,
}

/// CZCE option daily history (`option_hist_czce`).
///
/// Fetches the pipe-delimited `OptionDataDaily.txt` page and filters to the
/// chosen underlying's contracts (by ticker prefix), dropping the per-symbol
/// subtotal row — mirroring `pd.read_table(sep="|")` + the `iloc[:-1]` filter.
pub async fn option_hist_czce(
    client: &Client,
    symbol: &str,
    trade_date: &str,
) -> Result<Vec<OptionCzceHistRow>> {
    let prefix = czce_symbol_prefix(symbol)?;
    let digits: String = trade_date.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 8 {
        return Err(Error::InvalidParam(format!(
            "trade_date must be YYYYMMDD, got {trade_date}"
        )));
    }
    let year = &digits[0..4];
    let url = format!(
        "http://www.czce.com.cn/cn/DFSStaticFiles/Option/{year}/{digits}/OptionDataDaily.txt"
    );
    let text = client
        .get_text(SOURCE_CZCE, "option_hist_czce", &url, &[], None)
        .await?;
    Ok(parse_czce_hist(&text, prefix))
}

fn czce_symbol_prefix(symbol: &str) -> Result<&'static str> {
    Ok(match symbol {
        "白糖期权" => "SR",
        "棉花期权" => "CF",
        "甲醇期权" => "MA",
        "PTA期权" => "TA",
        "动力煤期权" => "ZC",
        "菜籽粕期权" => "RM",
        "菜籽油期权" => "OI",
        "花生期权" => "PK",
        "对二甲苯期权" => "PX",
        "烧碱期权" => "SH",
        "纯碱期权" => "SA",
        "短纤期权" => "PF",
        "锰硅期权" => "SM",
        "硅铁期权" => "SF",
        "尿素期权" => "UR",
        "苹果期权" => "AP",
        "红枣期权" => "CJ",
        "玻璃期权" => "FG",
        "瓶片期权" => "PR",
        "丙烯期权" => "PL",
        _ => return Err(Error::InvalidParam(format!("unknown CZCE symbol: {symbol}"))),
    })
}

fn parse_czce_hist(text: &str, prefix: &str) -> Vec<OptionCzceHistRow> {
    let mut lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    // First line is the header; remaining lines are data + a trailing subtotal.
    if lines.len() > 1 {
        lines.remove(0);
    }
    // Keep only the chosen underlying's contracts; drop the trailing subtotal row.
    let mut data: Vec<&str> = lines
        .into_iter()
        .filter(|l| {
            l.split('|')
                .next()
                .map(|c| c.contains(prefix))
                .unwrap_or(false)
        })
        .collect();
    if data.len() > 1 {
        data.pop();
    }
    let mut out = Vec::with_capacity(data.len());
    for line in data {
        let f: Vec<&str> = line.split('|').map(str::trim).collect();
        if f.len() < 16 {
            continue;
        }
        let num = |i: usize| f[i].parse::<f64>().ok();
        out.push(OptionCzceHistRow {
            contract_code: Some(f[0].to_string()),
            pre_settle: num(1),
            open: num(2),
            high: num(3),
            low: num(4),
            close: num(5),
            settle: num(6),
            chg1: num(7),
            chg2: num(8),
            volume: num(9),
            open_interest: num(10),
            oi_chg: num(11),
            turnover: num(12),
            delta: num(13),
            implied_vol: num(14),
            exec_volume: num(15),
        });
    }
    out
}

// ---------------------------------------------------------------------------
// 6. option_lhb_em (option_lhb_em.py:13)
// ---------------------------------------------------------------------------

/// A single option leaderboard (龙虎榜) row (`option_lhb_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct OptionLhbRow {
    pub trade_type: Option<String>,
    pub trade_date: Option<String>,
    pub security_code: Option<String>,
    pub target_name: Option<String>,
    pub rank: Option<i64>,
    pub member: Option<String>,
    pub volume: Option<f64>,
    pub position: Option<f64>,
    pub change: Option<f64>,
    pub net: Option<f64>,
    pub ratio: Option<f64>,
}

/// Option leaderboard (龙虎榜) from Eastmoney (`option_lhb_em`).
///
/// `indicator` selects the metric block (认沽/认购 x 交易量/持仓量). Eastmoney
/// returns named fields (not positional), so we map the relevant keys per block.
pub async fn option_lhb_em(
    client: &Client,
    symbol: &str,
    indicator: &str,
    trade_date: &str,
) -> Result<Vec<OptionLhbRow>> {
    let date = format!(
        "{}-{}-{}",
        &trade_date[0..4],
        &trade_date[4..6],
        &trade_date[6..8]
    );
    let filter = format!("(SECURITY_CODE=\"{symbol}\")(TRADE_DATE='{date}')");
    let params = [
        ("type", "RPT_IF_BILLBOARD_TD"),
        ("sty", "ALL"),
        ("filter", &filter),
        ("p", "1"),
        ("pss", "200"),
        ("source", "IFBILLBOARD"),
        ("client", "WEB"),
        ("ut", EM_UT),
    ];
    let url = "https://datacenter-web.eastmoney.com/api/data/get";
    let v = client
        .get_json(SOURCE_EASTMONEY, "option_lhb_em", url, &params)
        .await?;
    let data = v
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data".into(),
        })?;
    Ok(data.iter().map(|d| parse_lhb(d, indicator)).collect())
}

fn parse_lhb(d: &Value, indicator: &str) -> OptionLhbRow {
    let i64f = |k: &str| d.get(k).and_then(|v| v.as_i64());
    let f64f = |k: &str| fnum(d, k);
    let (vol_k, vol_chg_k, net_k, ratio_k, pos_k, pos_chg_k, net_pos_k, pos_ratio_k) = (
        "SELL_VOLUME",
        "SELL_VOLUME_CHANGE",
        "NET_SELL_VOLUME",
        "SELL_VOLUME_RATIO",
        "SELL_POSITION",
        "SELL_POSITION_CHANGE",
        "NET_SELL_POSITION",
        "SELL_POSITION_RATIO",
    );
    let (buy_vol_k, buy_vol_chg_k, net_buy_k, buy_ratio_k, buy_pos_k, buy_pos_chg_k, net_buy_pos_k, buy_pos_ratio_k) = (
        "BUY_VOLUME",
        "BUY_VOLUME_CHANGE",
        "NET_BUY_VOLUME",
        "BUY_VOLUME_RATIO",
        "BUY_POSITION",
        "BUY_POSITION_CHANGE",
        "NET_BUY_POSITION",
        "BUY_POSITION_RATIO",
    );
    let (volume, position, change, net, ratio) = match indicator {
        "期权交易情况-认沽交易量" => (f64f(vol_k), None, f64f(vol_chg_k), f64f(net_k), f64f(ratio_k)),
        "期权持仓情况-认沽持仓量" => (None, f64f(pos_k), f64f(pos_chg_k), f64f(net_pos_k), f64f(pos_ratio_k)),
        "期权交易情况-认购交易量" => (f64f(buy_vol_k), None, f64f(buy_vol_chg_k), f64f(net_buy_k), f64f(buy_ratio_k)),
        "期权持仓情况-认购持仓量" => (None, f64f(buy_pos_k), f64f(buy_pos_chg_k), f64f(net_buy_pos_k), f64f(buy_pos_ratio_k)),
        _ => (None, None, None, None, None),
    };
    OptionLhbRow {
        trade_type: fstr(d, "TRADE_TYPE"),
        trade_date: fstr(d, "TRADE_DATE"),
        security_code: fstr(d, "SECURITY_CODE"),
        target_name: fstr(d, "TARGET_NAME"),
        rank: i64f("MEMBER_RANK"),
        member: fstr(d, "MEMBER_NAME_ABBR"),
        volume,
        position,
        change,
        net,
        ratio,
    }
}

// ---------------------------------------------------------------------------
// 7. option_daily_stats_szse (option_daily_stats_sse_szse.py:85)
// ---------------------------------------------------------------------------

/// A single SZSE option daily-statistics row (`option_daily_stats_szse`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct OptionDailyStatsSzseRow {
    pub trade_date: Option<String>,
    pub target_code: Option<String>,
    pub target_name: Option<String>,
    pub volume: Option<f64>,
    pub call_volume: Option<f64>,
    pub put_volume: Option<f64>,
    pub put_call_oi_ratio: Option<f64>,
    pub total_oi: Option<f64>,
    pub call_oi: Option<f64>,
    pub put_oi: Option<f64>,
}

/// SZSE option daily statistics (`option_daily_stats_szse`).
///
/// GETs the SZSE `ShowReport` JSON (catalog `ysprdzb`) for the given date and
/// maps the renamed Chinese columns; numeric columns have comma separators
/// stripped, matching akshare.
pub async fn option_daily_stats_szse(
    client: &Client,
    date: &str,
) -> Result<Vec<OptionDailyStatsSzseRow>> {
    let trade_date = format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8]);
    let params = [
        ("SHOWTYPE", "JSON"),
        ("CATALOGID", "ysprdzb"),
        ("TABKEY", "tab1"),
        ("txtQueryDate", &trade_date),
        ("random", "0.0652692406565949"),
    ];
    let url = "https://investor.szse.cn/api/report/ShowReport/data";
    let v = client
        .get_json(SOURCE_SZSE, "option_daily_stats_szse", url, &params)
        .await?;
    let arr = v
        .as_array()
        .and_then(|a| a.first())
        .and_then(|o| o.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SZSE,
            message: "missing [0].data".into(),
        })?;
    Ok(arr
        .iter()
        .map(|d| OptionDailyStatsSzseRow {
            trade_date: Some(trade_date.clone()),
            target_code: fstr(d, "bddm"),
            target_name: fstr(d, "bdmc"),
            volume: fnum(d, "cjl"),
            call_volume: fnum(d, "rccjl"),
            put_volume: fnum(d, "rpcjl"),
            put_call_oi_ratio: fnum(d, "rcrpccb"),
            total_oi: fnum(d, "wpchyzs"),
            call_oi: fnum(d, "wpcrchys"),
            put_oi: fnum(d, "wpcrphys"),
        })
        .collect())
}

// ---------------------------------------------------------------------------
// 8. option_finance_board (option_finance.py:72)
// ---------------------------------------------------------------------------

/// A single option board row (`option_finance_board`).
///
/// Unified superset across the SSE "king" ETF boards, the SZSE ETF board, and
/// the CFFEX index-option boards. Branches populate only the fields relevant to
/// their upstream schema.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OptionFinanceBoardRow {
    pub symbol: Option<String>,
    pub date: Option<String>,
    pub contract_code: Option<String>,
    pub contract_name: Option<String>,
    pub underlying_name: Option<String>,
    pub option_type: Option<String>,
    pub last_price: Option<f64>,
    pub change: Option<f64>,
    pub pre_settle: Option<f64>,
    pub exercise_price: Option<f64>,
    pub unit: Option<f64>,
    pub volume: Option<f64>,
    pub open_interest: Option<f64>,
    pub total: Option<i64>,
    pub exercise_date: Option<String>,
    pub delivery_date: Option<String>,
}

/// Option board (current trading day) for an ETF or index option
/// (`option_finance_board`). Dispatches on `symbol` across SSE / SZSE / CFFEX.
pub async fn option_finance_board(
    client: &Client,
    symbol: &str,
    end_month: &str,
) -> Result<Vec<OptionFinanceBoardRow>> {
    let mm = &end_month[end_month.len().saturating_sub(2)..];
    match symbol {
        "华夏上证50ETF期权" => sse_board(client, "510050", mm, symbol).await,
        "华泰柏瑞沪深300ETF期权" => sse_board(client, "510300", mm, symbol).await,
        "南方中证500ETF期权" => sse_board(client, "510500", mm, symbol).await,
        "华夏科创50ETF期权" => sse_board(client, "588000", mm, symbol).await,
        "易方达科创50ETF期权" => sse_board(client, "588080", mm, symbol).await,
        "嘉实沪深300ETF期权" => szse_board(client, mm, symbol).await,
        "沪深300股指期权" => cffex_board(client, "http://www.cffex.com.cn/quote_IO.txt", mm, symbol).await,
        "中证1000股指期权" => cffex_board(client, "http://www.cffex.com.cn/quote_MO.txt", mm, symbol).await,
        "上证50股指期权" => cffex_board(client, "http://www.cffex.com.cn/quote_HO.txt", mm, symbol).await,
        _ => Ok(Vec::new()),
    }
}

async fn sse_board(
    client: &Client,
    code: &str,
    mm: &str,
    symbol: &str,
) -> Result<Vec<OptionFinanceBoardRow>> {
    let url = format!("http://yunhq.sse.com.cn:32041/v1/sho/list/tstyle/{code}_{mm}");
    let params = [("select", "contractid,last,chg_rate,presetpx,exepx")];
    let json = client
        .get_json(SOURCE_SSE, "option_finance_board", &url, &params)
        .await?;
    Ok(parse_sse_board(&json, symbol))
}

fn parse_sse_board(json: &Value, symbol: &str) -> Vec<OptionFinanceBoardRow> {
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
        out.push(OptionFinanceBoardRow {
            symbol: Some(symbol.to_string()),
            date: date.clone(),
            contract_code: get_str(arr, 0),
            contract_name: None,
            underlying_name: None,
            option_type: None,
            last_price: get_num(arr, 1),
            change: get_num(arr, 2),
            pre_settle: get_num(arr, 3),
            exercise_price: get_num(arr, 4),
            unit: None,
            volume: None,
            open_interest: None,
            total,
            exercise_date: None,
            delivery_date: None,
        });
    }
    out
}

async fn szse_board(
    client: &Client,
    mm: &str,
    symbol: &str,
) -> Result<Vec<OptionFinanceBoardRow>> {
    let url = "https://investor.szse.cn/api/report/ShowReport/data";
    let mut out = Vec::new();
    let mut page = 1u32;
    loop {
        let p = page.to_string();
        let params = [
            ("SHOWTYPE", "JSON"),
            ("CATALOGID", "ysplbrb"),
            ("TABKEY", "tab1"),
            ("PAGENO", p.as_str()),
            ("random", "0.10642298535346595"),
        ];
        let v = client
            .get_json(SOURCE_SZSE, "option_finance_board", url, &params)
            .await?;
        let Some(obj) = v.as_array().and_then(|a| a.first()) else {
            break;
        };
        let data: &[Value] = match obj.get("data").and_then(|d| d.as_array()) {
            Some(a) => a.as_slice(),
            None => &[],
        };
        if data.is_empty() {
            break;
        }
        for d in data {
            let ex = fstr(d, "xqrq");
            let month = ex.as_ref().and_then(|s| s.get(5..7).map(str::to_string));
            if month.as_deref() != Some(mm) {
                continue;
            }
            out.push(OptionFinanceBoardRow {
                symbol: Some(symbol.to_string()),
                date: None,
                contract_code: fstr(d, "hydm"),
                contract_name: fstr(d, "hymc"),
                underlying_name: fstr(d, "bdmc"),
                option_type: fstr(d, "hylx"),
                last_price: None,
                change: None,
                pre_settle: None,
                exercise_price: fnum(d, "xqj"),
                unit: fnum(d, "hydw"),
                volume: None,
                open_interest: None,
                total: None,
                exercise_date: ex,
                delivery_date: fstr(d, "jsrq"),
            });
        }
        let pagecount = obj
            .get("metadata")
            .and_then(|m| m.get("pagecount"))
            .and_then(|p| p.as_u64())
            .unwrap_or(1);
        if page as u64 >= pagecount {
            break;
        }
        page += 1;
    }
    Ok(out)
}

async fn cffex_board(
    client: &Client,
    url: &str,
    mm: &str,
    symbol: &str,
) -> Result<Vec<OptionFinanceBoardRow>> {
    let text = client
        .get_text(SOURCE_CFFEX, "option_finance_board", url, &[], None)
        .await?;
    Ok(parse_cffex_board(&text, mm, symbol))
}

fn parse_cffex_board(text: &str, mm: &str, symbol: &str) -> Vec<OptionFinanceBoardRow> {
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
    let header = match lines.next() {
        Some(h) => h,
        None => return Vec::new(),
    };
    let cols: Vec<&str> = header.split(',').map(str::trim).collect();
    let idx = |name: &str| cols.iter().position(|c| *c == name);
    let (i_inst, i_pos, i_vol, i_last, i_chg) = (
        idx("instrument"),
        idx("position"),
        idx("volume"),
        idx("lastprice"),
        idx("updown"),
    );
    let mut out = Vec::new();
    for line in lines {
        let f: Vec<&str> = line.split(',').map(str::trim).collect();
        let inst = match i_inst.and_then(|i| f.get(i)) {
            Some(s) => *s,
            None => continue,
        };
        // instrument like "IO2306-C-4000" -> month = chars [4..6].
        let month = inst.get(4..6).unwrap_or("");
        if month != mm {
            continue;
        }
        let num = |i: Option<usize>| i.and_then(|i| f.get(i)).and_then(|s| s.parse::<f64>().ok());
        out.push(OptionFinanceBoardRow {
            symbol: Some(symbol.to_string()),
            date: None,
            contract_code: Some(inst.to_string()),
            contract_name: None,
            underlying_name: None,
            option_type: None,
            last_price: num(i_last),
            change: num(i_chg),
            pre_settle: None,
            exercise_price: None,
            unit: None,
            volume: num(i_vol),
            open_interest: num(i_pos),
            total: None,
            exercise_date: None,
            delivery_date: None,
        });
    }
    out
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

    fn approx(a: Option<f64>, b: f64) -> bool {
        a.is_some_and(|x| (x - b).abs() < 1e-6)
    }

    #[test]
    fn parses_premium_fixture() {
        let v = fixture("option_premium_analysis_em.json");
        let diff = v.get("data").unwrap().get("diff").unwrap().as_array().unwrap();
        let rows: Vec<OptionPremiumRow> = diff.iter().map(parse_premium).collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].code.as_deref(), Some("10003720"));
        assert!(approx(rows[0].price, 0.1234));
        assert!(approx(rows[0].exercise_price, 3.1));
        assert!(approx(rows[0].premium_rate, -0.05));
        assert_eq!(rows[0].underlying_name.as_deref(), Some("华夏上证50ETF"));
        assert_eq!(rows[0].expiry_date.as_deref(), Some("2026-03-20"));
    }

    #[test]
    fn parses_risk_fixture() {
        let v = fixture("option_risk_analysis_em.json");
        let diff = v.get("data").unwrap().get("diff").unwrap().as_array().unwrap();
        let rows: Vec<OptionRiskRow> = diff.iter().map(parse_risk).collect();
        assert_eq!(rows.len(), 1);
        assert!(approx(rows[0].leverage, 12.5));
        assert!(approx(rows[0].delta, 0.55));
        assert!(approx(rows[0].gamma, 0.02));
        assert!(approx(rows[0].vega, 0.03));
        assert!(approx(rows[0].rho, 0.01));
        assert!(approx(rows[0].theta, -0.02));
        assert_eq!(rows[0].expiry_date.as_deref(), Some("2026-03-20"));
    }

    #[test]
    fn parses_value_fixture() {
        let v = fixture("option_value_analysis_em.json");
        let diff = v.get("data").unwrap().get("diff").unwrap().as_array().unwrap();
        let rows: Vec<OptionValueRow> = diff.iter().map(parse_value).collect();
        assert_eq!(rows.len(), 1);
        assert!(approx(rows[0].time_value, 0.01));
        assert!(approx(rows[0].intrinsic_value, 0.11));
        assert!(approx(rows[0].implied_vol, 0.25));
        assert!(approx(rows[0].theoretical_price, 0.12));
        assert!(approx(rows[0].underlying_yr_vol, 0.20));
    }

    #[test]
    fn parses_ctp_fixture() {
        let v = fixture("option_contract_info_ctp.json");
        let data = v.get("data").unwrap().as_array().unwrap();
        let rows: Vec<OptionContractCtpRow> = data.iter().map(parse_ctp).collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].exchange_id.as_deref(), Some("CFFEX"));
        assert_eq!(rows[0].instrument_id.as_deref(), Some("IO2306-C-4000"));
        assert!(approx(rows[0].volume_multiple, 100.0));
        assert!(approx(rows[0].strike_price, 4000.0));
        assert_eq!(rows[0].options_type.as_deref(), Some("C"));
    }

    #[test]
    fn parses_czce_hist_fixture() {
        let text = fixture_text("option_hist_czce.txt");
        let rows = parse_czce_hist(&text, "SR");
        // 2 SR contracts; the "SR合计" subtotal row is dropped.
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].contract_code.as_deref(), Some("SR311C6000"));
        assert!(approx(rows[0].open, 101.0));
        assert!(approx(rows[0].close, 103.0));
        assert!(approx(rows[0].volume, 1234.0));
        assert!(approx(rows[0].implied_vol, 0.25));
        assert_eq!(rows[1].contract_code.as_deref(), Some("SR311P6000"));
    }

    #[test]
    fn parses_lhb_fixture() {
        let v = fixture("option_lhb_em.json");
        let data = v.get("result").unwrap().get("data").unwrap().as_array().unwrap();
        let put_vol = data.iter().map(|d| parse_lhb(d, "期权交易情况-认沽交易量")).collect::<Vec<_>>();
        assert_eq!(put_vol.len(), 1);
        assert_eq!(put_vol[0].security_code.as_deref(), Some("510050"));
        assert!(approx(put_vol[0].volume, 800.0));
        assert!(approx(put_vol[0].net, -30.0));
        let call_pos = data.iter().map(|d| parse_lhb(d, "期权持仓情况-认购持仓量")).collect::<Vec<_>>();
        assert!(approx(call_pos[0].position, 5000.0));
        assert!(approx(call_pos[0].net, 150.0));
    }

    #[test]
    fn parses_szse_stats_fixture() {
        let v = fixture("option_daily_stats_szse.json");
        let arr = v.as_array().unwrap().first().unwrap().get("data").unwrap().as_array().unwrap();
        let rows: Vec<OptionDailyStatsSzseRow> = arr
            .iter()
            .map(|d| OptionDailyStatsSzseRow {
                trade_date: Some("2024-06-26".to_string()),
                target_code: fstr(d, "bddm"),
                target_name: fstr(d, "bdmc"),
                volume: fnum(d, "cjl"),
                call_volume: fnum(d, "rccjl"),
                put_volume: fnum(d, "rpcjl"),
                put_call_oi_ratio: fnum(d, "rcrpccb"),
                total_oi: fnum(d, "wpchyzs"),
                call_oi: fnum(d, "wpcrchys"),
                put_oi: fnum(d, "wpcrphys"),
            })
            .collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].target_code.as_deref(), Some("159919"));
        assert!(approx(rows[0].volume, 1234.0));
        assert!(approx(rows[0].call_volume, 700.0));
        assert!(approx(rows[0].put_volume, 534.0));
        assert!(approx(rows[0].put_call_oi_ratio, 0.76));
    }

    #[test]
    fn parses_finance_board_sse_fixture() {
        let json = fixture("option_finance_board_sse.json");
        let rows = parse_sse_board(&json, "华夏上证50ETF期权");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].contract_code.as_deref(), Some("10003900"));
        assert!(approx(rows[0].last_price, 3.500));
        assert!(approx(rows[0].exercise_price, 3.500));
        assert_eq!(rows[0].total, Some(2));
        assert_eq!(rows[0].date.as_deref(), Some("20240102153000"));
    }

    #[test]
    fn parses_finance_board_szse_fixture() {
        let v = fixture("option_finance_board_szse.json");
        let obj = v.as_array().unwrap().first().unwrap();
        let data = obj.get("data").unwrap().as_array().unwrap();
        let mut rows = Vec::new();
        for d in data {
            let ex = fstr(d, "xqrq");
            if ex.as_deref() != Some("2026-09-25") {
                continue;
            }
            rows.push(OptionFinanceBoardRow {
                symbol: Some("嘉实沪深300ETF期权".to_string()),
                date: None,
                contract_code: fstr(d, "hydm"),
                contract_name: fstr(d, "hymc"),
                underlying_name: fstr(d, "bdmc"),
                option_type: fstr(d, "hylx"),
                last_price: None,
                change: None,
                pre_settle: None,
                exercise_price: fnum(d, "xqj"),
                unit: fnum(d, "hydw"),
                volume: None,
                open_interest: None,
                total: None,
                exercise_date: ex,
                delivery_date: fstr(d, "jsrq"),
            });
        }
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].contract_code.as_deref(), Some("90007051"));
        assert_eq!(rows[0].underlying_name.as_deref(), Some("深证100ETF"));
        assert!(approx(rows[0].exercise_price, 3.1));
    }

    #[test]
    fn parses_finance_board_cffex_fixture() {
        let text = fixture_text("option_finance_board_cffex.txt");
        let rows = parse_cffex_board(&text, "06", "沪深300股指期权");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].contract_code.as_deref(), Some("IO2306-C-4000"));
        assert!(approx(rows[0].last_price, 607.20));
        assert!(approx(rows[0].change, -10.60));
        assert!(approx(rows[0].open_interest, 34.0));
    }

    #[test]
    fn czce_unknown_symbol_errors() {
        assert!(czce_symbol_prefix("不存在期权").is_err());
    }
}
