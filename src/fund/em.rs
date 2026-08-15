//! Eastmoney (天天基金网 / 东方财富) fund endpoints — port of the feasible public
//! functions from `akshare/fund/fund_em.py`.
//!
//! **Important deviation from the generic datacenter pattern:** none of the
//! functions below use the `datacenter-web.eastmoney.com/api/data/v1/get`
//! `reportName` API. They hit a variety of other Eastmoney JSON endpoints
//! (`Fund_JJJZ_Data.aspx`, `api.fund.eastmoney.com/f10/lsjz`, `GetLCJJJZ`,
//! `GetFundGZList`, `FundTradeRank/GetRankList`, `overseasapi/...`). The
//! `emg_data_array` helper (datacenter `result.data` envelope) is kept for parity
//! with `macro_china2.rs` but is unused by these endpoints, which have their own
//! response shapes.
//!
//! All functions are PURE HTTP (no JS / token / signature / `execjs` / cookie /
//! xls). Custom `Referer`/headers are supplied via `get_json_with_headers`.
//!
//! ## Ported functions
//!
//! | Rust fn | akshare line | upstream endpoint |
//! | --- | --- | --- |
//! | `fund_purchase_em` | fund_em.py:151 | `fund.eastmoney.com/Data/Fund_JJJZ_Data.aspx` (t=8) |
//! | `fund_info_index_em` | fund_em.py:234 | `api.fund.eastmoney.com/FundTradeRank/GetRankList` |
//! | `fund_open_fund_daily_em` | fund_em.py:386 | `fund.eastmoney.com/Data/Fund_JJJZ_Data.aspx` (t=1) |
//! | `fund_money_fund_info_em` | fund_em.py:741 | `api.fund.eastmoney.com/f10/lsjz` |
//! | `fund_financial_fund_daily_em` | fund_em.py:800 | `api.fund.eastmoney.com/FundNetValue/GetLCJJJZ` |
//! | `fund_financial_fund_info_em` | fund_em.py:873 | `api.fund.eastmoney.com/f10/lsjz` |
//! | `fund_graded_fund_daily_em` | fund_em.py:938 | `fund.eastmoney.com/Data/Fund_JJJZ_Data.aspx` (t=1,lx=9) |
//! | `fund_graded_fund_info_em` | fund_em.py:1008 | `api.fund.eastmoney.com/f10/lsjz` |
//! | `fund_etf_fund_info_em` | fund_em.py:1097 | `api.fund.eastmoney.com/f10/lsjz` |
//! | `fund_value_estimation_em` | fund_em.py:1161 | `api.fund.eastmoney.com/FundGuZhi/GetFundGZList` |
//! | `fund_hk_fund_hist_em` | fund_em.py:1260 | `overseas.1234567.com.cn/overseasapi/OpenApiHander.ashx` |
//!
//! ## DEFERRED (not ported)
//!
//! * **`fund_money_fund_daily_em`** (fund_em.py:707) — HTML scraping via
//!   `pd.read_html` (gb2312) of `HBJJ_pjsyl.html`. No JSON API.
//! * **`fund_etf_fund_daily_em`** (fund_em.py:1064) — HTML scraping via
//!   `pd.read_html` (gb2312) of `cnjy_dwjz.html`. No JSON API.
//! * **`fund_open_fund_info_em`** (fund_em.py:452) — already ported elsewhere in
//!   the crate (as `fund_open_fund_info`); also relies on `py_mini_racer` JS eval
//!   of `pingzhongdata/<symbol>.js` for most indicators. Skipped per instructions.
//! * **`fund_value_estimation_em` index-page fallback** — for `symbol` in
//!   `{"全部","指数型"}` akshare first scrapes static HTML pages
//!   (`lof_fundguzhiN.html`) via BeautifulSoup. That HTML branch is DEFERRED; the
//!   `GetFundGZList` JSON API path is implemented for every symbol (including
//!   全部/指数型, mapped to type=1/5).

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

const JJJZ_URL: &str = "https://fund.eastmoney.com/Data/Fund_JJJZ_Data.aspx";
const RANK_URL: &str = "https://api.fund.eastmoney.com/FundTradeRank/GetRankList";
const LSJZ_URL: &str = "https://api.fund.eastmoney.com/f10/lsjz";
const LCJJZ_URL: &str = "https://api.fund.eastmoney.com/FundNetValue/GetLCJJJZ";
const GUZHI_URL: &str = "https://api.fund.eastmoney.com/FundGuZhi/GetFundGZList";
const HK_URL: &str = "https://overseas.1234567.com.cn/overseasapi/OpenApiHander.ashx";

/// Extract `result.data` (the row array) from a datacenter-web response.
///
/// Kept for parity with `macro_china2.rs`; the endpoints in this module use
/// other envelopes, so it is currently unused here.
#[allow(dead_code)]
fn emg_data_array(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data".into(),
        })
}

/// Object field as `String` (also stringifies numbers, like akshare does).
fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(val_str)
}

/// Object field as `f64` (handles `Number` and numeric `String`).
fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(val_num)
}

fn val_str(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn val_num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().trim_end_matches('%').trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// Positional string from an array row (also stringifies numbers).
fn str_at(item: &Value, idx: usize) -> Option<String> {
    item.get(idx).and_then(val_str)
}

/// Positional numeric from an array row (handles `Number` / numeric `String`).
fn num_at(item: &Value, idx: usize) -> Option<f64> {
    item.get(idx).and_then(val_num)
}

/// Pick the first present key from a candidate list (order-insensitive; works
/// around serde_json sorting object keys and unknown upstream key casing).
fn pick<'a>(item: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    for k in keys {
        if let Some(v) = item.get(*k) {
            return Some(v);
        }
    }
    None
}

// ===========================================================================
// fund_purchase_em (fund_em.py:151) — Fund_JJJZ_Data.aspx t=8
// ===========================================================================

/// 东方财富-天天基金网-基金申购状态 (`Fund_JJJZ_Data.aspx`, t=8).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundPurchaseRow {
    /// 序号
    pub seq: u32,
    /// 基金代码
    pub fund_code: String,
    /// 基金简称
    pub fund_name: String,
    /// 基金类型
    pub fund_type: Option<String>,
    /// 最新净值/万份收益
    pub nav: Option<f64>,
    /// 最新净值/万份收益-报告时间
    pub report_time: Option<String>,
    /// 申购状态
    pub purchase_status: Option<String>,
    /// 赎回状态
    pub redeem_status: Option<String>,
    /// 下一开放日
    pub next_open: Option<String>,
    /// 购买起点
    pub min_buy: Option<f64>,
    /// 日累计限定金额
    pub daily_limit: Option<f64>,
    /// 手续费 (%)
    pub fee: Option<f64>,
}

fn parse_purchase(datas: &[Value]) -> Vec<FundPurchaseRow> {
    let mut out = Vec::with_capacity(datas.len());
    for (i, item) in datas.iter().enumerate() {
        out.push(FundPurchaseRow {
            seq: (i + 1) as u32,
            fund_code: str_at(item, 0).unwrap_or_default(),
            fund_name: str_at(item, 1).unwrap_or_default(),
            fund_type: str_at(item, 2),
            nav: num_at(item, 3),
            report_time: str_at(item, 4),
            purchase_status: str_at(item, 5),
            redeem_status: str_at(item, 6),
            next_open: str_at(item, 7),
            min_buy: num_at(item, 8),
            daily_limit: num_at(item, 9),
            fee: num_at(item, 12),
        });
    }
    out
}

/// 东方财富网站-天天基金网-基金数据-基金申购状态 (akshare/fund/fund_em.py:151).
pub async fn fund_purchase_em(client: &Client) -> Result<Vec<FundPurchaseRow>> {
    let params = [
        ("t", "8"),
        ("page", "1,50000"),
        ("js", "reData"),
        ("sort", "fcode,asc"),
    ];
    let text = client
        .get_text(SOURCE_EASTMONEY, "fund_purchase_em", JJJZ_URL, &params, None)
        .await?;
    let json = parse_jjjz_json(&text)?;
    let datas = jjjz_datas(&json)?;
    Ok(parse_purchase(datas))
}

// ===========================================================================
// fund_info_index_em (fund_em.py:234) — FundTradeRank/GetRankList
// ===========================================================================

/// 东方财富-天天基金网-基金信息-指数型 (`GetRankList`, ft=zs).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundInfoIndexRow {
    /// 基金代码
    pub fund_code: String,
    /// 基金名称
    pub fund_name: String,
    /// 单位净值
    pub nav: Option<f64>,
    /// 日期
    pub date: Option<String>,
    /// 日增长率
    pub daily_growth: Option<f64>,
    /// 近1周
    pub w1: Option<f64>,
    /// 近1月
    pub w1m: Option<f64>,
    /// 近3月
    pub w3m: Option<f64>,
    /// 近6月
    pub w6m: Option<f64>,
    /// 近1年
    pub w1y: Option<f64>,
    /// 近2年
    pub w2y: Option<f64>,
    /// 近3年
    pub w3y: Option<f64>,
    /// 今年来
    pub ytd: Option<f64>,
    /// 成立来
    pub total: Option<f64>,
    /// 手续费
    pub fee: Option<f64>,
    /// 起购金额
    pub min_buy: Option<String>,
    /// 跟踪标的 (the `symbol` argument)
    pub track_symbol: String,
    /// 跟踪方式 (the `indicator` argument)
    pub track_indicator: String,
}

fn parse_info_index(datas: &[Value], symbol: &str, indicator: &str) -> Vec<FundInfoIndexRow> {
    let mut out = Vec::with_capacity(datas.len());
    for item in datas {
        let Some(s) = item.as_str() else { continue };
        let f: Vec<&str> = s.split('|').collect();
        let at = |i: usize| f.get(i).map(|x| x.trim()).filter(|x| !x.is_empty());
        let num = |i: usize| at(i).and_then(|x| x.parse::<f64>().ok());
        out.push(FundInfoIndexRow {
            fund_code: at(0).unwrap_or("").to_string(),
            fund_name: at(1).unwrap_or("").to_string(),
            nav: num(4),
            date: at(3).map(|x| x.to_string()),
            daily_growth: num(5),
            w1: num(6),
            w1m: num(7),
            w3m: num(8),
            w6m: num(9),
            w1y: num(10),
            w2y: num(11),
            w3y: num(12),
            ytd: num(13),
            total: num(14),
            fee: num(18),
            min_buy: at(25).map(|x| x.to_string()),
            track_symbol: symbol.to_string(),
            track_indicator: indicator.to_string(),
        });
    }
    out
}

/// 东方财富网站-天天基金网-基金数据-基金信息-指数型 (akshare/fund/fund_em.py:234).
pub async fn fund_info_index_em(
    client: &Client,
    symbol: &str,
    indicator: &str,
) -> Result<Vec<FundInfoIndexRow>> {
    let fr = match symbol {
        "全部" => "050|001", // treated as special below
        "沪深指数" => "053",
        "行业主题" => "054",
        "大盘指数" => "01",
        "中盘指数" => "02",
        "小盘指数" => "03",
        "股票指数" => "050|001",
        "债券指数" => "050|003",
        _ => return Err(Error::InvalidParam(format!("unknown symbol: {symbol}"))),
    };
    let fr1 = match indicator {
        "全部" => "",
        "被动指数型" => "051",
        "增强指数型" => "052",
        _ => return Err(Error::InvalidParam(format!("unknown indicator: {indicator}"))),
    };
    let (fr_main, ftype, fr1_val) = if symbol == "股票指数" || symbol == "债券指数" {
        let parts: Vec<&str> = fr.split('|').collect();
        (parts[0], parts.get(1).copied().unwrap_or(""), fr1)
    } else if symbol == "全部" {
        ("050", "001", fr1)
    } else {
        (fr, "", fr1)
    };
    let params = [
        ("ft", "zs"),
        ("sc", "1n"),
        ("st", "desc"),
        ("pi", "1"),
        ("pn", "10000"),
        ("cp", ""),
        ("ct", ""),
        ("cd", ""),
        ("ms", ""),
        ("fr", fr_main),
        ("plevel", ""),
        ("fst", ""),
        ("ftype", ftype),
        ("fr1", fr1_val),
        ("fl", "0"),
        ("isab", "1"),
    ];
    let headers = [
        ("Referer", "https://fund.eastmoney.com/"),
        ("Host", "api.fund.eastmoney.com"),
        (
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/103.0.0.0 Safari/537.36",
        ),
    ];
    let v = client
        .get_json_with_headers(
            SOURCE_EASTMONEY,
            "fund_info_index_em",
            RANK_URL,
            &params,
            Some(&headers),
        )
        .await?;
    let data_str = fstr(&v, "Data").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "missing Data".into(),
    })?;
    let inner: Value = serde_json::from_str(&data_str).map_err(Error::Json)?;
    let datas = inner.get("datas").and_then(|d| d.as_array()).ok_or_else(|| {
        Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing datas".into(),
        }
    })?;
    Ok(parse_info_index(datas, symbol, indicator))
}

// ===========================================================================
// fund_open_fund_daily_em (fund_em.py:386) — Fund_JJJZ_Data.aspx t=1
// ===========================================================================

/// 东方财富网-天天基金网-开放式基金净值 (`Fund_JJJZ_Data.aspx`, t=1).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundOpenFundDailyRow {
    /// 基金代码
    pub fund_code: String,
    /// 基金简称
    pub fund_name: String,
    /// 日期1 (`showday[0]`)
    pub date_1: Option<String>,
    /// 日期1-单位净值
    pub nav_1: Option<f64>,
    /// 日期1-累计净值
    pub cum_nav_1: Option<f64>,
    /// 日期2 (`showday[1]`)
    pub date_2: Option<String>,
    /// 日期2-单位净值
    pub nav_2: Option<f64>,
    /// 日期2-累计净值
    pub cum_nav_2: Option<f64>,
    /// 日增长值
    pub day_increase: Option<f64>,
    /// 日增长率
    pub day_growth_rate: Option<f64>,
    /// 申购状态
    pub purchase_status: Option<String>,
    /// 赎回状态
    pub redeem_status: Option<String>,
    /// 手续费 (%)
    pub fee: Option<f64>,
}

fn parse_open_fund_daily(datas: &[Value], showday: &[String]) -> Vec<FundOpenFundDailyRow> {
    let date_1 = showday.first().cloned();
    let date_2 = showday.get(1).cloned();
    let mut out = Vec::with_capacity(datas.len());
    for item in datas {
        out.push(FundOpenFundDailyRow {
            fund_code: str_at(item, 0).unwrap_or_default(),
            fund_name: str_at(item, 1).unwrap_or_default(),
            date_1: date_1.clone(),
            nav_1: num_at(item, 3),
            cum_nav_1: num_at(item, 4),
            date_2: date_2.clone(),
            nav_2: num_at(item, 5),
            cum_nav_2: num_at(item, 6),
            day_increase: num_at(item, 7),
            day_growth_rate: num_at(item, 8),
            purchase_status: str_at(item, 9),
            redeem_status: str_at(item, 10),
            fee: num_at(item, 18),
        });
    }
    out
}

/// 东方财富网-天天基金网-基金数据-开放式基金净值 (akshare/fund/fund_em.py:386).
pub async fn fund_open_fund_daily_em(client: &Client) -> Result<Vec<FundOpenFundDailyRow>> {
    let params = [
        ("t", "1"),
        ("lx", "1"),
        ("letter", ""),
        ("gsid", ""),
        ("text", ""),
        ("sort", "zdf,desc"),
        ("page", "1,50000"),
        ("dt", "1580914040623"),
        ("atfc", ""),
        ("onlySale", "0"),
    ];
    let text = client
        .get_text(
            SOURCE_EASTMONEY,
            "fund_open_fund_daily_em",
            JJJZ_URL,
            &params,
            None,
        )
        .await?;
    let json = parse_jjjz_json(&text)?;
    let datas = jjjz_datas(&json)?;
    let showday = showday_of(&json);
    Ok(parse_open_fund_daily(datas, &showday))
}

// ===========================================================================
// fund_money_fund_info_em (fund_em.py:741) — f10/lsjz
// ===========================================================================

/// 东方财富网-天天基金网-货币型基金-历史净值 (`f10/lsjz`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundMoneyFundInfoRow {
    /// 净值日期 (FSRQ)
    pub date: Option<String>,
    /// 每万份收益 (DWJZ)
    pub per_million: Option<f64>,
    /// 7日年化收益率 (LJJZ)
    pub annual_7d: Option<f64>,
    /// 申购状态 (SGZT)
    pub purchase_status: Option<String>,
    /// 赎回状态 (SHZT)
    pub redeem_status: Option<String>,
}

fn parse_money_fund_info(items: &[Value]) -> Vec<FundMoneyFundInfoRow> {
    items
        .iter()
        .map(|item| FundMoneyFundInfoRow {
            date: fstr(item, "FSRQ"),
            per_million: fnum(item, "DWJZ"),
            annual_7d: fnum(item, "LJJZ"),
            purchase_status: fstr(item, "SGZT"),
            redeem_status: fstr(item, "SHZT"),
        })
        .collect()
}

/// 东方财富网-天天基金网-基金数据-货币型基金收益-历史净值数据 (akshare/fund/fund_em.py:741).
pub async fn fund_money_fund_info_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<FundMoneyFundInfoRow>> {
    let items = lsjz_all(client, "fund_money_fund_info_em", symbol, "", "").await?;
    Ok(parse_money_fund_info(&items))
}

// ===========================================================================
// fund_financial_fund_daily_em (fund_em.py:800) — GetLCJJJZ
// ===========================================================================

/// 东方财富网站-天天基金网-理财型基金收益 (`GetLCJJJZ`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundFinancialDailyRow {
    /// 序号 (Id)
    pub seq: Option<String>,
    /// 基金代码 (fcode)
    pub fund_code: Option<String>,
    /// 基金简称 (shortname)
    pub fund_name: Option<String>,
    /// 上一期年化收益率 (actualsyi)
    pub prev_annual_yield: Option<f64>,
    /// 日期1 (`showday[0]`)
    pub date_1: Option<String>,
    /// 日期1-万份收益 (mui)
    pub mui_1: Option<f64>,
    /// 日期1-7日年华 (syi)
    pub syi_1: Option<f64>,
    /// 日期2 (`showday[1]`)
    pub date_2: Option<String>,
    /// 日期2-万份收益 (zrmui)
    pub mui_2: Option<f64>,
    /// 日期2-7日年华 (zrsyi)
    pub syi_2: Option<f64>,
    /// 封闭期 (cycle)
    pub cycle: Option<String>,
    /// 申购状态 (kfr)
    pub purchase_status: Option<String>,
}

fn parse_financial_daily(list: &[Value], showday: &[String]) -> Vec<FundFinancialDailyRow> {
    let date_1 = showday.first().cloned();
    let date_2 = showday.get(1).cloned();
    list.iter()
        .map(|item| FundFinancialDailyRow {
            seq: fstr(item, "Id"),
            fund_code: fstr(item, "fcode"),
            fund_name: fstr(item, "shortname"),
            prev_annual_yield: fnum(item, "actualsyi"),
            date_1: date_1.clone(),
            mui_1: fnum(item, "mui"),
            syi_1: fnum(item, "syi"),
            date_2: date_2.clone(),
            mui_2: fnum(item, "zrmui"),
            syi_2: fnum(item, "zrsyi"),
            cycle: fstr(item, "cycle"),
            purchase_status: fstr(item, "kfr"),
        })
        .collect()
}

/// 东方财富网站-天天基金网-基金数据-理财型基金收益 (akshare/fund/fund_em.py:800).
pub async fn fund_financial_fund_daily_em(client: &Client) -> Result<Vec<FundFinancialDailyRow>> {
    let headers = [
        ("Referer", "https://fund.eastmoney.com/lcjj.html"),
        (
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/80.0.3987.149 Safari/537.36",
        ),
    ];
    let params = [
        ("letter", ""),
        ("jjgsid", "0"),
        ("searchtext", ""),
        ("sort", "ljjz,desc"),
        ("page", "1,100"),
        ("AttentionCodes", ""),
        ("cycle", ""),
        ("OnlySale", "1"),
    ];
    let v = client
        .get_json_with_headers(
            SOURCE_EASTMONEY,
            "fund_financial_fund_daily_em",
            LCJJZ_URL,
            &params,
            Some(&headers),
        )
        .await?;
    let data = v.get("Data").and_then(|d| d.as_object()).ok_or_else(|| {
        Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing Data".into(),
        }
    })?;
    let list = data.get("List").and_then(|l| l.as_array()).ok_or_else(|| {
        Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing Data.List".into(),
        }
    })?;
    let showday = data
        .get("showday")
        .and_then(|s| s.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|x| x.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(parse_financial_daily(list, &showday))
}

// ===========================================================================
// fund_financial_fund_info_em (fund_em.py:873) — f10/lsjz (+ FHSP)
// ===========================================================================

/// 东方财富网站-天天基金网-理财型基金-历史净值明细 (`f10/lsjz`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundFinancialFundInfoRow {
    /// 净值日期 (FSRQ)
    pub date: Option<String>,
    /// 单位净值 (DWJZ)
    pub nav: Option<f64>,
    /// 累计净值 (LJJZ)
    pub cum_nav: Option<f64>,
    /// 日增长率 (JZZZL)
    pub daily_growth: Option<f64>,
    /// 申购状态 (SGZT)
    pub purchase_status: Option<String>,
    /// 赎回状态 (SHZT)
    pub redeem_status: Option<String>,
    /// 分红送配 (FHSP)
    pub dividend: Option<String>,
}

fn parse_financial_fund_info(items: &[Value]) -> Vec<FundFinancialFundInfoRow> {
    items
        .iter()
        .map(|item| FundFinancialFundInfoRow {
            date: fstr(item, "FSRQ"),
            nav: fnum(item, "DWJZ"),
            cum_nav: fnum(item, "LJJZ"),
            daily_growth: fnum(item, "JZZZL"),
            purchase_status: fstr(item, "SGZT"),
            redeem_status: fstr(item, "SHZT"),
            dividend: fstr(item, "FHSP"),
        })
        .collect()
}

/// 东方财富网站-天天基金网-基金数据-理财型基金收益-历史净值明细 (akshare/fund/fund_em.py:873).
pub async fn fund_financial_fund_info_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<FundFinancialFundInfoRow>> {
    let items = lsjz_all(client, "fund_financial_fund_info_em", symbol, "", "").await?;
    Ok(parse_financial_fund_info(&items))
}

// ===========================================================================
// fund_graded_fund_daily_em (fund_em.py:938) — Fund_JJJZ_Data.aspx t=1 lx=9
// ===========================================================================

/// 东方财富网站-天天基金网-分级基金净值 (`Fund_JJJZ_Data.aspx`, t=1 lx=9).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundGradedDailyRow {
    /// 基金代码
    pub fund_code: String,
    /// 基金简称
    pub fund_name: String,
    /// 日期1 (`showday[0]`)
    pub date_1: Option<String>,
    /// 日期1-单位净值
    pub nav_1: Option<f64>,
    /// 日期1-累计净值
    pub cum_nav_1: Option<f64>,
    /// 日期2 (`showday[1]`)
    pub date_2: Option<String>,
    /// 日期2-单位净值
    pub nav_2: Option<f64>,
    /// 日期2-累计净值
    pub cum_nav_2: Option<f64>,
    /// 日增长值
    pub day_increase: Option<f64>,
    /// 日增长率
    pub day_growth_rate: Option<f64>,
    /// 市价
    pub market_price: Option<f64>,
    /// 折价率
    pub discount_rate: Option<f64>,
    /// 手续费 (%)
    pub fee: Option<f64>,
}

fn parse_graded_daily(datas: &[Value], showday: &[String]) -> Vec<FundGradedDailyRow> {
    let date_1 = showday.first().cloned();
    let date_2 = showday.get(1).cloned();
    let mut out = Vec::with_capacity(datas.len());
    for item in datas {
        out.push(FundGradedDailyRow {
            fund_code: str_at(item, 0).unwrap_or_default(),
            fund_name: str_at(item, 1).unwrap_or_default(),
            date_1: date_1.clone(),
            nav_1: num_at(item, 3),
            cum_nav_1: num_at(item, 4),
            date_2: date_2.clone(),
            nav_2: num_at(item, 5),
            cum_nav_2: num_at(item, 6),
            day_increase: num_at(item, 7),
            day_growth_rate: num_at(item, 8),
            market_price: num_at(item, 9),
            discount_rate: num_at(item, 10),
            fee: num_at(item, 18),
        });
    }
    out
}

/// 东方财富网站-天天基金网-基金数据-分级基金净值 (akshare/fund/fund_em.py:938).
pub async fn fund_graded_fund_daily_em(client: &Client) -> Result<Vec<FundGradedDailyRow>> {
    let params = [
        ("t", "1"),
        ("lx", "9"),
        ("letter", ""),
        ("gsid", "0"),
        ("text", ""),
        ("sort", "zdf,desc"),
        ("page", "1,10000"),
        ("dt", "1580914040623"),
        ("atfc", ""),
    ];
    let headers = [
        (
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/80.0.3987.149 Safari/537.36",
        ),
        ("Referer", "https://fund.eastmoney.com/fjjj.html"),
    ];
    let text = client
        .get_text(
            SOURCE_EASTMONEY,
            "fund_graded_fund_daily_em",
            JJJZ_URL,
            &params,
            Some(&headers),
        )
        .await?;
    let json = parse_jjjz_json(&text)?;
    let datas = jjjz_datas(&json)?;
    let showday = showday_of(&json);
    Ok(parse_graded_daily(datas, &showday))
}

// ===========================================================================
// fund_graded_fund_info_em (fund_em.py:1008) & fund_etf_fund_info_em (1097)
// — f10/lsjz, shared NAV-history shape
// ===========================================================================

/// 东方财富-分级/场内交易基金-历史净值明细 (`f10/lsjz`): 净值日期/单位净值/累计净值/日增长率/申购/赎回.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundNavHistoryRow {
    /// 净值日期 (FSRQ)
    pub date: Option<String>,
    /// 单位净值 (DWJZ)
    pub nav: Option<f64>,
    /// 累计净值 (LJJZ)
    pub cum_nav: Option<f64>,
    /// 日增长率 (JZZZL)
    pub daily_growth: Option<f64>,
    /// 申购状态 (SGZT)
    pub purchase_status: Option<String>,
    /// 赎回状态 (SHZT)
    pub redeem_status: Option<String>,
}

fn parse_nav_history(items: &[Value]) -> Vec<FundNavHistoryRow> {
    items
        .iter()
        .map(|item| FundNavHistoryRow {
            date: fstr(item, "FSRQ"),
            nav: fnum(item, "DWJZ"),
            cum_nav: fnum(item, "LJJZ"),
            daily_growth: fnum(item, "JZZZL"),
            purchase_status: fstr(item, "SGZT"),
            redeem_status: fstr(item, "SHZT"),
        })
        .collect()
}

/// 东方财富网站-天天基金网-分级基金净值-历史净值明细 (akshare/fund/fund_em.py:1008).
pub async fn fund_graded_fund_info_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<FundNavHistoryRow>> {
    let items = lsjz_all(client, "fund_graded_fund_info_em", symbol, "", "").await?;
    Ok(parse_nav_history(&items))
}

/// 东方财富网站-天天基金网-场内交易基金-历史净值明细 (akshare/fund/fund_em.py:1097).
pub async fn fund_etf_fund_info_em(
    client: &Client,
    symbol: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<FundNavHistoryRow>> {
    let items = lsjz_all(
        client,
        "fund_etf_fund_info_em",
        symbol,
        &fmt_date(start_date),
        &fmt_date(end_date),
    )
    .await?;
    Ok(parse_nav_history(&items))
}

// ===========================================================================
// fund_value_estimation_em (fund_em.py:1161) — GetFundGZList
// ===========================================================================

/// 东方财富网-数据中心-净值估算 (`GetFundGZList`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundValueEstimationRow {
    /// 基金代码
    pub fund_code: Option<String>,
    /// 基金名称
    pub fund_name: Option<String>,
    /// 估算值
    pub est_value: Option<f64>,
    /// 估算增长率
    pub est_growth: Option<f64>,
    /// 公布数据-单位净值
    pub pub_nav: Option<f64>,
    /// 公布数据-日增长率
    pub pub_growth: Option<f64>,
    /// 估算偏差
    pub deviation: Option<f64>,
    /// 单位净值 (估值日)
    pub nav: Option<f64>,
    /// 估算日期 (gxrq)
    pub estimate_date: Option<String>,
    /// 公布日期 (gzrq)
    pub publish_date: Option<String>,
}

fn parse_value_estimation(
    list: &[Value],
    gxrq: Option<String>,
    gzrq: Option<String>,
) -> Vec<FundValueEstimationRow> {
    list
        .iter()
        .map(|item| FundValueEstimationRow {
            fund_code: fstr_keys(item, &["fcode", "fundcode"]),
            fund_name: fstr_keys(item, &["shortname", "fundname", "jjjc", "name"]),
            est_value: fnum_keys(item, &["gz", "gsz", "gzvalue"]),
            est_growth: fnum_keys(item, &["gzr", "gszzl"]),
            pub_nav: fnum_keys(item, &["dwjz", "pubnav"]),
            pub_growth: fnum_keys(item, &["rzdf", "pubgrowth"]),
            deviation: fnum_keys(item, &["deviation", "pj"]),
            nav: fnum_keys(item, &["jjjz", "nav"]),
            estimate_date: gxrq.clone(),
            publish_date: gzrq.clone(),
        })
        .collect()
}

fn fstr_keys(item: &Value, keys: &[&str]) -> Option<String> {
    pick(item, keys).and_then(val_str)
}

fn fnum_keys(item: &Value, keys: &[&str]) -> Option<f64> {
    pick(item, keys).and_then(val_num)
}

/// 东方财富网-数据中心-净值估算 (akshare/fund/fund_em.py:1161).
///
/// The `GetFundGZList` JSON path is implemented for every `symbol`. The akshare
/// HTML index-page fallback (used for `symbol` in `{"全部","指数型"}`) is
/// DEFERRED (BeautifulSoup scraping of static `lof_fundguzhiN.html` pages).
pub async fn fund_value_estimation_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<FundValueEstimationRow>> {
    let type_id = match symbol {
        "全部" => 1,
        "股票型" => 2,
        "混合型" => 3,
        "债券型" => 4,
        "指数型" => 5,
        "QDII" => 6,
        "ETF联接" => 7,
        "LOF" => 8,
        "场内交易基金" => 9,
        _ => return Err(Error::InvalidParam(format!("unknown symbol: {symbol}"))),
    };
    let headers = [("Referer", "https://fund.eastmoney.com/")];
    let t = type_id.to_string();
    let params = [
        ("type", t.as_str()),
        ("sort", "3"),
        ("orderType", "desc"),
        ("canbuy", "0"),
        ("pageIndex", "1"),
        ("pageSize", "20000"),
    ];
    let v = client
        .get_json_with_headers(
            SOURCE_EASTMONEY,
            "fund_value_estimation_em",
            GUZHI_URL,
            &params,
            Some(&headers),
        )
        .await?;
    let data = v.get("Data").and_then(|d| d.as_object()).ok_or_else(|| {
        Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing Data".into(),
        }
    })?;
    let gxrq = data.get("gxrq").and_then(val_str);
    let gzrq = data.get("gzrq").and_then(val_str);
    let list = data.get("list").and_then(|l| l.as_array()).ok_or_else(|| {
        Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing Data.list".into(),
        }
    })?;
    Ok(parse_value_estimation(list, gxrq, gzrq))
}

// ===========================================================================
// fund_hk_fund_hist_em (fund_em.py:1260) — overseasapi/OpenApiHander.ashx
// ===========================================================================

/// 东方财富网-天天基金网-香港基金-历史净值明细/分红送配详情 (`OpenApiHander.ashx`, HKFDApi/MethodJZ).
///
/// Unified row: NAV-branch fields and dividend-branch fields are all optional and
/// populated according to `indicator`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundHkHistRow {
    // NAV branch (indicator = "历史净值明细", action=2)
    /// 净值日期
    pub date: Option<String>,
    /// 单位净值
    pub nav: Option<f64>,
    /// 日增长值
    pub day_increase: Option<f64>,
    /// 日增长率
    pub day_growth: Option<f64>,
    /// 单位
    pub unit: Option<String>,
    // Dividend branch (indicator = "分红送配详情", action=3)
    /// 年份
    pub year: Option<String>,
    /// 权益登记日
    pub register_date: Option<String>,
    /// 除息日
    pub ex_date: Option<String>,
    /// 分红发放日
    pub pay_date: Option<String>,
    /// 分红金额
    pub dividend_amount: Option<String>,
}

fn parse_hk_hist(items: &[Value], dividend: bool) -> Vec<FundHkHistRow> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        if dividend {
            // action=3: 年份(5), 权益登记日(8), 除息日(7), 分红发放日(9), 分红金额(6), 单位(11)
            out.push(FundHkHistRow {
                date: None,
                nav: None,
                day_increase: None,
                day_growth: None,
                unit: hk_field_str(item, 11, &["DW"]),
                year: hk_field_str(item, 5, &["NDATE", "YEAR"]),
                register_date: hk_field_str(item, 8, &["REGDATE"]),
                ex_date: hk_field_str(item, 7, &["EXDATE"]),
                pay_date: hk_field_str(item, 9, &["PAYDATE"]),
                dividend_amount: hk_field_str(item, 6, &["FHJE", "AMOUNT"]),
            });
        } else {
            // action=2: 净值日期(3), 单位净值(4), 日增长值(6), 日增长率(7), 单位(9)
            out.push(FundHkHistRow {
                date: hk_field_str(item, 3, &["FSRQ"]),
                nav: hk_field_num(item, 4, &["DWJZ"]),
                day_increase: hk_field_num(item, 6, &["DAYINC"]),
                day_growth: hk_field_num(item, 7, &["DAYRATE"]),
                unit: hk_field_str(item, 9, &["DW"]),
                year: None,
                register_date: None,
                ex_date: None,
                pay_date: None,
                dividend_amount: None,
            });
        }
    }
    out
}

/// Read a field positionally (array) or by candidate key (object).
fn hk_field<'a>(item: &'a Value, idx: usize, keys: &[&str]) -> Option<&'a Value> {
    if let Some(arr) = item.as_array() {
        arr.get(idx)
    } else {
        pick(item, keys)
    }
}

fn hk_field_str(item: &Value, idx: usize, keys: &[&str]) -> Option<String> {
    hk_field(item, idx, keys).and_then(val_str)
}

fn hk_field_num(item: &Value, idx: usize, keys: &[&str]) -> Option<f64> {
    hk_field(item, idx, keys).and_then(val_num)
}

/// 东方财富网-天天基金网-香港基金-历史净值明细(分红送配详情) (akshare/fund/fund_em.py:1260).
///
/// `symbol` is the HK fund code (e.g. `1002200683`); `indicator` is
/// `"历史净值明细"` (default) or `"分红送配详情"`.
pub async fn fund_hk_fund_hist_em(
    client: &Client,
    symbol: &str,
    indicator: &str,
) -> Result<Vec<FundHkHistRow>> {
    let action = if indicator == "分红送配详情" { "3" } else { "2" };
    let params = [
        ("api", "HKFDApi"),
        ("m", "MethodJZ"),
        ("hkfcode", symbol),
        ("action", action),
        ("pageindex", "0"),
        ("pagesize", "1000"),
        ("date1", ""),
        ("date2", ""),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "fund_hk_fund_hist_em", HK_URL, &params)
        .await?;
    let data = v.get("Data").and_then(|d| d.as_array()).ok_or_else(|| {
        Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing Data".into(),
        }
    })?;
    Ok(parse_hk_hist(data, indicator == "分红送配详情"))
}

// ===========================================================================
// Shared helpers
// ===========================================================================

/// Parse the `var <ident>= {...}` text body returned by `Fund_JJJZ_Data.aspx`.
fn parse_jjjz_json(text: &str) -> Result<Value> {
    let trimmed = text.trim();
    let json_text = if let Some(pos) = trimmed.find("var ") {
        if let Some(eq) = trimmed[pos..].find('=') {
            &trimmed[pos + eq + 1..]
        } else {
            trimmed
        }
    } else {
        trimmed
    };
    serde_json::from_str(json_text.trim()).map_err(Error::Json)
}

/// Extract the `datas` array (list of lists) from a `Fund_JJJZ_Data.aspx` response.
fn jjjz_datas(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("datas")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing datas".into(),
        })
}

/// Extract the `showday` array (list of date strings) from a `Fund_JJJZ_Data.aspx` response.
fn showday_of(resp: &Value) -> Vec<String> {
    resp.get("showday")
        .and_then(|s| s.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|x| x.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Fetch all `LSJZList` pages from `f10/lsjz` (paginated by `TotalCount`).
async fn lsjz_all(
    client: &Client,
    fn_name: &'static str,
    symbol: &str,
    start: &str,
    end: &str,
) -> Result<Vec<Value>> {
    let referer = format!("https://fundf10.eastmoney.com/jjjz_{symbol}.html");
    let headers = [("Referer", referer.as_str())];
    let page_size = 20u32;
    let mut all: Vec<Value> = Vec::new();
    let mut page = 1u32;
    loop {
        let pidx = page.to_string();
        let psz = page_size.to_string();
        let params = [
            ("fundCode", symbol),
            ("pageIndex", pidx.as_str()),
            ("pageSize", psz.as_str()),
            ("startDate", start),
            ("endDate", end),
        ];
        let v = client
            .get_json_with_headers(SOURCE_EASTMONEY, fn_name, LSJZ_URL, &params, Some(&headers))
            .await?;
        let data = v.get("Data").and_then(|d| d.as_object()).ok_or_else(|| {
            Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing Data".into(),
            }
        })?;
        let list = data.get("LSJZList").and_then(|l| l.as_array()).ok_or_else(|| {
            Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing Data.LSJZList".into(),
            }
        })?;
        if list.is_empty() {
            break;
        }
        all.extend(list.iter().cloned());
        let total: u32 = v
            .get("TotalCount")
            .and_then(|t| t.as_u64())
            .unwrap_or(0)
            .try_into()
            .unwrap_or(0);
        let pages = total.div_ceil(page_size);
        if page >= pages {
            break;
        }
        page += 1;
    }
    Ok(all)
}

/// Convert `YYYYMMDD` to `YYYY-MM-DD` (Eastmoney `f10/lsjz` date format); pass through otherwise.
fn fmt_date(s: &str) -> String {
    if s.len() == 8 && s.chars().all(|c| c.is_ascii_digit()) {
        format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8])
    } else {
        s.to_string()
    }
}

// ===========================================================================
// Offline golden tests
// ===========================================================================

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

    #[test]
    fn parses_fund_purchase_em() {
        let json = parse_jjjz_json(
            &std::fs::read_to_string(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures")
                    .join("fund_purchase_em.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let rows = parse_purchase(jjjz_datas(&json).unwrap());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].seq, 1);
        assert_eq!(rows[0].fund_code, "000001");
        assert_eq!(rows[0].fund_name, "华夏成长");
        assert_eq!(rows[0].nav, Some(1.234));
        assert_eq!(rows[0].purchase_status.as_deref(), Some("开放"));
        assert_eq!(rows[0].fee, Some(1.5));
    }

    #[test]
    fn parses_fund_info_index_em() {
        let v = fixture("fund_info_index_em.json");
        let data_str = fstr(&v, "Data").unwrap();
        let inner: Value = serde_json::from_str(&data_str).unwrap();
        let datas = inner.get("datas").unwrap().as_array().unwrap();
        let rows = parse_info_index(datas, "沪深指数", "被动指数型");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].fund_code, "000001");
        assert_eq!(rows[0].nav, Some(1.234));
        assert_eq!(rows[0].date.as_deref(), Some("2024-01-01"));
        assert_eq!(rows[0].w1y, Some(10.0));
        assert_eq!(rows[0].fee, Some(1.5));
        assert_eq!(rows[0].min_buy.as_deref(), Some("100"));
        assert_eq!(rows[0].track_symbol, "沪深指数");
    }

    #[test]
    fn parses_fund_open_fund_daily_em() {
        let json = parse_jjjz_json(
            &std::fs::read_to_string(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures")
                    .join("fund_open_fund_daily_em.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let rows = parse_open_fund_daily(jjjz_datas(&json).unwrap(), &showday_of(&json));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].fund_code, "000001");
        assert_eq!(rows[0].date_1.as_deref(), Some("2024-01-01"));
        assert_eq!(rows[0].nav_1, Some(1.0));
        assert_eq!(rows[0].cum_nav_2, Some(1.9));
        assert_eq!(rows[0].purchase_status.as_deref(), Some("开放"));
        assert_eq!(rows[0].fee, Some(0.15));
    }

    #[test]
    fn parses_fund_money_fund_info_em() {
        let v = fixture("fund_money_fund_info_em.json");
        let items = v
            .get("Data")
            .unwrap()
            .get("LSJZList")
            .unwrap()
            .as_array()
            .unwrap();
        let rows = parse_money_fund_info(items);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].date.as_deref(), Some("2024-01-01"));
        assert_eq!(rows[0].per_million, Some(1.5));
        assert_eq!(rows[0].annual_7d, Some(2.5));
        assert_eq!(rows[0].purchase_status.as_deref(), Some("开放"));
    }

    #[test]
    fn parses_fund_financial_fund_daily_em() {
        let v = fixture("fund_financial_fund_daily_em.json");
        let data = v.get("Data").unwrap().as_object().unwrap();
        let list = data.get("List").unwrap().as_array().unwrap();
        let showday = data
            .get("showday")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        let rows = parse_financial_daily(list, &showday);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].fund_code.as_deref(), Some("000134"));
        assert_eq!(rows[0].prev_annual_yield, Some(3.2));
        assert_eq!(rows[0].date_1.as_deref(), Some("2024-01-01"));
        assert_eq!(rows[0].mui_1, Some(1.1));
        assert_eq!(rows[0].syi_2, Some(2.2));
        assert_eq!(rows[0].purchase_status.as_deref(), Some("开放"));
    }

    #[test]
    fn parses_fund_financial_fund_info_em() {
        let v = fixture("fund_financial_fund_info_em.json");
        let items = v
            .get("Data")
            .unwrap()
            .get("LSJZList")
            .unwrap()
            .as_array()
            .unwrap();
        let rows = parse_financial_fund_info(items);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].date.as_deref(), Some("2024-01-01"));
        assert_eq!(rows[0].nav, Some(1.234));
        assert_eq!(rows[0].cum_nav, Some(2.345));
        assert_eq!(rows[0].daily_growth, Some(1.1));
        assert_eq!(rows[0].dividend.as_deref(), Some("每份派0.1"));
    }

    #[test]
    fn parses_fund_graded_fund_daily_em() {
        let json = parse_jjjz_json(
            &std::fs::read_to_string(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures")
                    .join("fund_graded_fund_daily_em.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let rows = parse_graded_daily(jjjz_datas(&json).unwrap(), &showday_of(&json));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].fund_code, "150001");
        assert_eq!(rows[0].nav_1, Some(1.0));
        assert_eq!(rows[0].market_price, Some(1.05));
        assert_eq!(rows[0].discount_rate, Some(-3.0));
        assert_eq!(rows[0].fee, Some(0.15));
    }

    #[test]
    fn parses_fund_graded_fund_info_em() {
        let v = fixture("fund_graded_fund_info_em.json");
        let items = v
            .get("Data")
            .unwrap()
            .get("LSJZList")
            .unwrap()
            .as_array()
            .unwrap();
        let rows = parse_nav_history(items);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].date.as_deref(), Some("2024-01-01"));
        assert_eq!(rows[0].nav, Some(1.234));
        assert_eq!(rows[0].cum_nav, Some(2.345));
        assert_eq!(rows[0].daily_growth, Some(1.1));
    }

    #[test]
    fn parses_fund_etf_fund_info_em() {
        let v = fixture("fund_etf_fund_info_em.json");
        let items = v
            .get("Data")
            .unwrap()
            .get("LSJZList")
            .unwrap()
            .as_array()
            .unwrap();
        let rows = parse_nav_history(items);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].date.as_deref(), Some("2024-01-01"));
        assert_eq!(rows[0].nav, Some(1.234));
        assert_eq!(rows[0].cum_nav, Some(2.345));
        assert_eq!(rows[0].purchase_status.as_deref(), Some("开放"));
    }

    #[test]
    fn parses_fund_value_estimation_em() {
        let v = fixture("fund_value_estimation_em.json");
        let data = v.get("Data").unwrap().as_object().unwrap();
        let gxrq = data.get("gxrq").and_then(val_str);
        let gzrq = data.get("gzrq").and_then(val_str);
        let list = data.get("list").unwrap().as_array().unwrap();
        let rows = parse_value_estimation(list, gxrq, gzrq);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].fund_code.as_deref(), Some("000001"));
        assert_eq!(rows[0].fund_name.as_deref(), Some("华夏成长"));
        assert_eq!(rows[0].est_value, Some(1.23));
        assert_eq!(rows[0].est_growth, Some(0.5));
        assert_eq!(rows[0].pub_nav, Some(1.20));
        assert_eq!(rows[0].nav, Some(1.22));
        assert_eq!(rows[0].estimate_date.as_deref(), Some("2024-01-02"));
        assert_eq!(rows[0].publish_date.as_deref(), Some("2024-01-01"));
    }

    #[test]
    fn parses_fund_hk_fund_hist_em_nav() {
        let v = fixture("fund_hk_fund_hist_em.json");
        let data = v.get("Data").unwrap().as_array().unwrap();
        let rows = parse_hk_hist(data, false);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date.as_deref(), Some("2024-01-01"));
        assert_eq!(rows[0].nav, Some(1.234));
        assert_eq!(rows[0].day_growth, Some(1.0));
        assert_eq!(rows[0].unit.as_deref(), Some("HKD"));
        assert_eq!(rows[1].nav, Some(1.240));
    }

    #[test]
    fn parses_fund_hk_fund_hist_em_dividend() {
        let v = fixture("fund_hk_fund_hist_em_dividend.json");
        let data = v.get("Data").unwrap().as_array().unwrap();
        let rows = parse_hk_hist(data, true);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].year.as_deref(), Some("2024"));
        assert_eq!(rows[0].dividend_amount.as_deref(), Some("0.50"));
        assert_eq!(rows[0].ex_date.as_deref(), Some("2024-03-01"));
        assert_eq!(rows[0].register_date.as_deref(), Some("2024-02-28"));
        assert_eq!(rows[0].pay_date.as_deref(), Some("2024-03-02"));
    }
}
