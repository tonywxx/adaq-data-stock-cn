//! Port of the Shenwan (SW) industry-index functions from
//! `akshare/index/index_research_sw.py` (akshare lines cited per function below).
//!
//! **Note on endpoints**: unlike most of this crate's Eastmoney ports, every
//! function here hits the **SWS Research** HTTP API (`www.swsresearch.com`,
//! `institute-sw/api/...`) — not Eastmoney `datacenter-web`/`push2`. The URLs,
//! params and output field lists below are taken verbatim from the akshare
//! source so they match the live API exactly. Two response envelopes appear:
//!
//! * `{"data": [ ...rows... ]}` — a bare array (`index_hist_sw`, `index_min_sw`,
//!   `index_analysis_week_month_sw`).
//! * `{"data": {"count": N, "results": [ ...rows... ]}}` — a paginated wrapper
//!   (`index_component_sw`, `index_realtime_sw`, `index_analysis_*`). The source
//!   paginates these with `math.ceil(count / page_size)`; the helpers below
//!   replicate that loop (1 request when a single fixture page is enough).
//!
//! ## Ported functions
//!
//! | Rust fn | akshare line | endpoint |
//! | --- | --- | --- |
//! | `index_hist_sw` | index_research_sw.py:29 | `institute-sw/api/index_publish/trend/` |
//! | `index_min_sw` | index_research_sw.py:93 | `institute-sw/api/index_publish/details/timelines/` |
//! | `index_component_sw` | index_research_sw.py:139 | `institute-sw/api/index_publish/details/component_stocks/` |
//! | `index_realtime_sw` | index_research_sw.py:241 | `institute-sw/api/index_publish/current/` |
//! | `index_analysis_daily_sw` | index_research_sw.py:319 | `institute-sw/api/index_analysis/index_analysis_report/` |
//! | `index_analysis_week_month_sw` | index_research_sw.py:397 | `institute-sw/api/index_analysis/week_month_datetime/` |
//! | `index_analysis_weekly_sw` | index_research_sw.py:423 | `institute-sw/api/index_analysis/index_analysis_reports/` |
//! | `index_analysis_monthly_sw` | index_research_sw.py:498 | `institute-sw/api/index_analysis/index_analysis_reports/` |
//!
//! ## DEFERRED
//!
//! * `index_realtime_sw` for `symbol` ∈ {"大类风格指数", "金创指数"}: akshare
//!   routes these to a **JSON-body POST** (`insWechatSw/dflgOrJcIndex/pageList`,
//!   see `__index_realtime_sw` at index_research_sw.py:183). `Client` only offers
//!   form-encoded POST (`post_form_json`) and GET, so the JSON-body path is
//!   DEFERRED and those symbols return `Error::InvalidParam`. The four primary
//!   symbols (`市场表征`, `一级行业`, `二级行业`, `风格指数`) use the GET `current/`
//!   endpoint and are fully implemented.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_SWS: &str = "swsresearch";

const HIST_URL: &str = "https://www.swsresearch.com/institute-sw/api/index_publish/trend/";
const MIN_URL: &str =
    "https://www.swsresearch.com/institute-sw/api/index_publish/details/timelines/";
const COMPONENT_URL: &str =
    "https://www.swsresearch.com/institute-sw/api/index_publish/details/component_stocks/";
const CURRENT_URL: &str = "https://www.swsresearch.com/institute-sw/api/index_publish/current/";
const ANALYSIS_DAILY_URL: &str =
    "https://www.swsresearch.com/institute-sw/api/index_analysis/index_analysis_report/";
const ANALYSIS_REPORTS_URL: &str =
    "https://www.swsresearch.com/institute-sw/api/index_analysis/index_analysis_reports/";
const WEEK_MONTH_URL: &str =
    "https://www.swsresearch.com/institute-sw/api/index_analysis/week_month_datetime/";

/// Extract `data` (a bare row array) from a SWS response.
fn sw_data_array(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SWS,
            message: "missing data array".into(),
        })
}

fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

/// Positional-array variant of [`fstr`] for endpoints that return rows as lists
/// (not objects) — e.g. the `current/` realtime feed.
fn arr_fstr(item: &Value, idx: usize) -> Option<String> {
    item.get(idx)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Positional-array variant of [`fnum`].
fn arr_fnum(item: &Value, idx: usize) -> Option<f64> {
    item.get(idx).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

/// Fetch a `{"data": [...]}` (bare-array) endpoint.
async fn sw_fetch_array(
    client: &Client,
    fn_name: &'static str,
    url: &str,
    params: &[(&str, &str)],
) -> Result<Vec<Value>> {
    let v = client.get_json(SOURCE_SWS, fn_name, url, params).await?;
    sw_data_array(&v).cloned()
}

/// Append/replace the `page` query param on `base`.
fn with_page<'a, 'b: 'a>(base: &[(&'b str, &'b str)], page: &'a str) -> Vec<(&'a str, &'a str)> {
    let mut v: Vec<(&'a str, &'a str)> = base.iter().map(|(k, val)| (*k, *val)).collect();
    v.push(("page", page));
    v
}

/// Fetch a paginated `{"data": {"count", "results": [...]}}` endpoint, replicating
/// akshare's `math.ceil(count / page_size)` page loop. Returns the concatenated
/// `results` rows. A single fixture page (count <= page_size) triggers exactly one
/// request.
async fn sw_fetch_results(
    client: &Client,
    fn_name: &'static str,
    url: &str,
    base: &[(&str, &str)],
    page_size: u64,
) -> Result<Vec<Value>> {
    let first = with_page(base, "1");
    let resp = client.get_json(SOURCE_SWS, fn_name, url, &first).await?;
    let data = resp.get("data").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SWS,
        message: "missing data".into(),
    })?;
    let count = data.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
    let total_page = count.div_ceil(page_size);
    let mut rows: Vec<Value> = Vec::new();
    if let Some(results) = data.get("results").and_then(|v| v.as_array()) {
        rows.extend(results.iter().cloned());
    }
    let mut page_buf;
    for page in 2..=total_page {
        page_buf = page.to_string();
        let params = with_page(base, page_buf.as_str());
        let resp = client.get_json(SOURCE_SWS, fn_name, url, &params).await?;
        if let Some(results) = resp
            .get("data")
            .and_then(|d| d.get("results"))
            .and_then(|v| v.as_array())
        {
            rows.extend(results.iter().cloned());
        }
    }
    Ok(rows)
}

/// Convert akshare-style `YYYYMMDD` to `YYYY-MM-DD` (matching the API's date param).
fn fmt_date(d: &str) -> String {
    if d.len() >= 8 {
        format!("{}-{}-{}", &d[0..4], &d[4..6], &d[6..8])
    } else {
        d.to_string()
    }
}

/// Map akshare `period` ("day"/"week"/"month") to the API's `DAY`/`WEEK`/`MONTH`.
fn hist_period(p: &str) -> &str {
    match p {
        "day" => "DAY",
        "week" => "WEEK",
        "month" => "MONTH",
        _ => p,
    }
}

// ===========================================================================
// index_hist_sw (index_research_sw.py:29)
// ===========================================================================

/// A single historical bar of a Shenwan (SW) industry index.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HistSwRow {
    /// 代码 (SWS `swindexcode`)
    pub code: String,
    /// 日期 (SWS `bargaindate`)
    pub date: Option<String>,
    /// 收盘 (SWS `closeindex`)
    pub close: Option<f64>,
    /// 开盘 (SWS `openindex`)
    pub open: Option<f64>,
    /// 最高 (SWS `maxindex`)
    pub high: Option<f64>,
    /// 最低 (SWS `minindex`)
    pub low: Option<f64>,
    /// 成交量 (SWS `bargainamount`)
    pub volume: Option<f64>,
    /// 成交额 (SWS `bargainsum`)
    pub amount: Option<f64>,
}

/// 申万宏源研究-指数发布-指数详情-指数历史数据 (`institute-sw/api/index_publish/trend/`, index_research_sw.py:29).
pub async fn index_hist_sw(client: &Client, symbol: &str, period: &str) -> Result<Vec<HistSwRow>> {
    let p = hist_period(period);
    let params: [(&str, &str); 2] = [("swindexcode", symbol), ("period", p)];
    let rows = sw_fetch_array(client, "index_hist_sw", HIST_URL, &params).await?;
    Ok(parse_hist_sw(&rows))
}

fn parse_hist_sw(items: &[Value]) -> Vec<HistSwRow> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(code) = fstr(item, "swindexcode") else {
            continue;
        };
        out.push(HistSwRow {
            code,
            date: fstr(item, "bargaindate"),
            close: fnum(item, "closeindex"),
            open: fnum(item, "openindex"),
            high: fnum(item, "maxindex"),
            low: fnum(item, "minindex"),
            volume: fnum(item, "bargainamount"),
            amount: fnum(item, "bargainsum"),
        });
    }
    out
}

// ===========================================================================
// index_min_sw (index_research_sw.py:93)
// ===========================================================================

/// A single intraday timeline point of a Shenwan (SW) industry index.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MinSwRow {
    /// 代码 (SWS `l1`)
    pub code: Option<String>,
    /// 名称 (SWS `l2`)
    pub name: Option<String>,
    /// 价格 (SWS `l8`)
    pub price: Option<f64>,
    /// 日期 (SWS `trading_date`)
    pub date: Option<String>,
    /// 时间 (SWS `trading_time`)
    pub time: Option<String>,
}

/// 申万宏源研究-指数发布-指数详情-指数分时数据 (`institute-sw/api/index_publish/details/timelines/`, index_research_sw.py:93).
pub async fn index_min_sw(client: &Client, symbol: &str) -> Result<Vec<MinSwRow>> {
    let params: [(&str, &str); 1] = [("swindexcode", symbol)];
    let rows = sw_fetch_array(client, "index_min_sw", MIN_URL, &params).await?;
    Ok(parse_min_sw(&rows))
}

fn parse_min_sw(items: &[Value]) -> Vec<MinSwRow> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(MinSwRow {
            code: fstr(item, "l1"),
            name: fstr(item, "l2"),
            price: fnum(item, "l8"),
            date: fstr(item, "trading_date"),
            time: fstr(item, "trading_time"),
        });
    }
    out
}

// ===========================================================================
// index_component_sw (index_research_sw.py:139)
// ===========================================================================

/// A constituent stock of a Shenwan (SW) industry index.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ComponentSwRow {
    /// 序号 (1-based position in the response)
    pub sequence: u64,
    /// 证券代码 (SWS `stockcode`)
    pub stock_code: String,
    /// 证券名称 (SWS `stockname`)
    pub stock_name: String,
    /// 最新权重 (SWS `newweight`)
    pub weight: Option<f64>,
    /// 计入日期 (SWS `beginningdate`)
    pub begin_date: Option<String>,
}

/// 申万宏源研究-指数发布-指数详情-成分股 (`institute-sw/api/index_publish/details/component_stocks/`, index_research_sw.py:139).
pub async fn index_component_sw(client: &Client, symbol: &str) -> Result<Vec<ComponentSwRow>> {
    let params: [(&str, &str); 2] = [("swindexcode", symbol), ("page_size", "10000")];
    let rows =
        sw_fetch_results(client, "index_component_sw", COMPONENT_URL, &params, 10000).await?;
    Ok(parse_component_sw(&rows))
}

fn parse_component_sw(items: &[Value]) -> Vec<ComponentSwRow> {
    let mut out = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let Some(stock_code) = fstr(item, "stockcode") else {
            continue;
        };
        let Some(stock_name) = fstr(item, "stockname") else {
            continue;
        };
        out.push(ComponentSwRow {
            sequence: (i + 1) as u64,
            stock_code,
            stock_name,
            weight: fnum(item, "newweight"),
            begin_date: fstr(item, "beginningdate"),
        });
    }
    out
}

// ===========================================================================
// index_realtime_sw (index_research_sw.py:241)
// ===========================================================================

/// A realtime quote row for a Shenwan (SW) index series (GET `current/` path).
#[derive(Debug, Clone, serde::Serialize)]
pub struct RealtimeSwRow {
    /// 指数代码 (positional field 0)
    pub index_code: String,
    /// 指数名称 (positional field 1)
    pub index_name: String,
    /// 昨收盘 (positional field 2)
    pub prev_close: Option<f64>,
    /// 今开盘 (positional field 3)
    pub open: Option<f64>,
    /// 成交额 (positional field 4)
    pub amount: Option<f64>,
    /// 最高价 (positional field 5)
    pub high: Option<f64>,
    /// 最低价 (positional field 6)
    pub low: Option<f64>,
    /// 最新价 (positional field 7)
    pub latest: Option<f64>,
    /// 成交量 (positional field 8)
    pub volume: Option<f64>,
}

/// 申万宏源研究-指数系列实时行情 (`institute-sw/api/index_publish/current/`, index_research_sw.py:241).
///
/// `symbol` is one of {"市场表征", "一级行业", "二级行业", "风格指数"}. The
/// `大类风格指数` / `金创指数` variants use a JSON-body POST and are DEFERRED
/// (see module docs).
pub async fn index_realtime_sw(client: &Client, symbol: &str) -> Result<Vec<RealtimeSwRow>> {
    if matches!(symbol, "大类风格指数" | "金创指数") {
        return Err(Error::InvalidParam(format!(
            "index_realtime_sw: symbol '{}' requires JSON-body POST (DEFERRED: unsupported by Client)",
            symbol
        )));
    }
    let params: [(&str, &str); 2] = [("page_size", "50"), ("indextype", symbol)];
    let rows = sw_fetch_results(client, "index_realtime_sw", CURRENT_URL, &params, 50).await?;
    Ok(parse_realtime_sw(&rows))
}

fn parse_realtime_sw(items: &[Value]) -> Vec<RealtimeSwRow> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(index_code) = arr_fstr(item, 0) else {
            continue;
        };
        let Some(index_name) = arr_fstr(item, 1) else {
            continue;
        };
        out.push(RealtimeSwRow {
            index_code,
            index_name,
            prev_close: arr_fnum(item, 2),
            open: arr_fnum(item, 3),
            amount: arr_fnum(item, 4),
            high: arr_fnum(item, 5),
            low: arr_fnum(item, 6),
            latest: arr_fnum(item, 7),
            volume: arr_fnum(item, 8),
        });
    }
    out
}

// ===========================================================================
// index_analysis_* (index_research_sw.py:319, 397, 423, 498)
// ===========================================================================

/// A daily/weekly/monthly analysis observation of a Shenwan (SW) index.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AnalysisSwRow {
    /// 指数代码 (SWS `swindexcode`)
    pub index_code: String,
    /// 指数名称 (SWS `swindexname`)
    pub index_name: String,
    /// 发布日期 (SWS `bargaindate`)
    pub date: Option<String>,
    /// 收盘指数 (SWS `closeindex`)
    pub close_index: Option<f64>,
    /// 成交量 (SWS `bargainamount`)
    pub volume: Option<f64>,
    /// 涨跌幅 (SWS `markup`)
    pub change_pct: Option<f64>,
    /// 换手率 (SWS `turnoverrate`)
    pub turnover_rate: Option<f64>,
    /// 市盈率 (SWS `pe`)
    pub pe: Option<f64>,
    /// 市净率 (SWS `pb`)
    pub pb: Option<f64>,
    /// 均价 (SWS `meanprice`)
    pub mean_price: Option<f64>,
    /// 成交额占比 (SWS `bargainsumrate`)
    pub amount_rate: Option<f64>,
    /// 流通市值 (SWS `negotiablessharesum1`)
    pub float_market_cap: Option<f64>,
    /// 平均流通市值 (SWS `negotiablessharesum2`)
    pub avg_float_market_cap: Option<f64>,
    /// 股息率 (SWS `dp`)
    pub dp: Option<f64>,
}

/// 申万宏源研究-指数分析-日报表 (`institute-sw/api/index_analysis/index_analysis_report/`, index_research_sw.py:319).
pub async fn index_analysis_daily_sw(
    client: &Client,
    symbol: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<AnalysisSwRow>> {
    let start = fmt_date(start_date);
    let end = fmt_date(end_date);
    let params: Vec<(&str, &str)> = vec![
        ("page_size", "50"),
        ("index_type", symbol),
        ("start_date", &start),
        ("end_date", &end),
        ("type", "DAY"),
        ("swindexcode", "all"),
    ];
    let rows = sw_fetch_results(
        client,
        "index_analysis_daily_sw",
        ANALYSIS_DAILY_URL,
        &params,
        50,
    )
    .await?;
    Ok(parse_analysis_sw(&rows))
}

/// 申万宏源研究-周/月报表-日期序列 (`institute-sw/api/index_analysis/week_month_datetime/`, index_research_sw.py:397).
pub async fn index_analysis_week_month_sw(
    client: &Client,
    symbol: &str,
) -> Result<Vec<WeekMonthSwRow>> {
    let t = symbol.to_uppercase();
    let params: [(&str, &str); 1] = [("type", t.as_str())];
    let rows = sw_fetch_array(
        client,
        "index_analysis_week_month_sw",
        WEEK_MONTH_URL,
        &params,
    )
    .await?;
    Ok(parse_week_month_sw(&rows))
}

/// 申万宏源研究-指数分析-周报告 (`institute-sw/api/index_analysis/index_analysis_reports/`, index_research_sw.py:423).
pub async fn index_analysis_weekly_sw(
    client: &Client,
    symbol: &str,
    date: &str,
) -> Result<Vec<AnalysisSwRow>> {
    let d = fmt_date(date);
    let params: Vec<(&str, &str)> = vec![
        ("page_size", "50"),
        ("index_type", symbol),
        ("bargaindate", &d),
        ("type", "WEEK"),
        ("swindexcode", "all"),
    ];
    let rows = sw_fetch_results(
        client,
        "index_analysis_weekly_sw",
        ANALYSIS_REPORTS_URL,
        &params,
        50,
    )
    .await?;
    Ok(parse_analysis_sw(&rows))
}

/// 申万宏源研究-指数分析-月报告 (`institute-sw/api/index_analysis/index_analysis_reports/`, index_research_sw.py:498).
pub async fn index_analysis_monthly_sw(
    client: &Client,
    symbol: &str,
    date: &str,
) -> Result<Vec<AnalysisSwRow>> {
    let d = fmt_date(date);
    let params: Vec<(&str, &str)> = vec![
        ("page_size", "50"),
        ("index_type", symbol),
        ("bargaindate", &d),
        ("type", "MONTH"),
        ("swindexcode", "all"),
    ];
    let rows = sw_fetch_results(
        client,
        "index_analysis_monthly_sw",
        ANALYSIS_REPORTS_URL,
        &params,
        50,
    )
    .await?;
    Ok(parse_analysis_sw(&rows))
}

fn parse_analysis_sw(items: &[Value]) -> Vec<AnalysisSwRow> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(index_code) = fstr(item, "swindexcode") else {
            continue;
        };
        let Some(index_name) = fstr(item, "swindexname") else {
            continue;
        };
        out.push(AnalysisSwRow {
            index_code,
            index_name,
            date: fstr(item, "bargaindate"),
            close_index: fnum(item, "closeindex"),
            volume: fnum(item, "bargainamount"),
            change_pct: fnum(item, "markup"),
            turnover_rate: fnum(item, "turnoverrate"),
            pe: fnum(item, "pe"),
            pb: fnum(item, "pb"),
            mean_price: fnum(item, "meanprice"),
            amount_rate: fnum(item, "bargainsumrate"),
            float_market_cap: fnum(item, "negotiablessharesum1"),
            avg_float_market_cap: fnum(item, "negotiablessharesum2"),
            dp: fnum(item, "dp"),
        });
    }
    out
}

/// A single date in the weekly/monthly report date series
/// (`index_analysis_week_month_sw`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct WeekMonthSwRow {
    /// 日期 (SWS `bargaindate`)
    pub date: Option<String>,
}

fn parse_week_month_sw(items: &[Value]) -> Vec<WeekMonthSwRow> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(WeekMonthSwRow {
            date: fstr(item, "bargaindate"),
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

    /// Extract the bare `data` array from a fixture response.
    fn array_of(name: &str) -> Vec<Value> {
        sw_data_array(&fixture(name)).unwrap().clone()
    }

    /// Extract the `data.results` array from a paginated fixture response.
    fn results_of(name: &str) -> Vec<Value> {
        fixture(name)
            .get("data")
            .and_then(|d| d.get("results"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap()
    }

    #[test]
    fn parses_index_hist_sw() {
        let rows = parse_hist_sw(&array_of("index_hist_sw.json"));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "801030");
        assert_eq!(rows[0].date, Some("2024-10-25".to_string()));
        assert_eq!(rows[0].close, Some(3456.78));
        assert_eq!(rows[0].amount, None);
        assert_eq!(rows[1].close, Some(3400.0));
    }

    #[test]
    fn parses_index_min_sw() {
        let rows = parse_min_sw(&array_of("index_min_sw.json"));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, Some("801001".to_string()));
        assert_eq!(rows[0].name, Some("申万50".to_string()));
        assert_eq!(rows[0].price, Some(3200.5));
        assert_eq!(rows[0].date, Some("2024-10-25".to_string()));
        assert_eq!(rows[1].price, None);
    }

    #[test]
    fn parses_index_component_sw() {
        let rows = parse_component_sw(&results_of("index_component_sw.json"));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].sequence, 1);
        assert_eq!(rows[0].stock_code, "600000");
        assert_eq!(rows[0].stock_name, "浦发银行");
        assert_eq!(rows[0].weight, Some(5.5));
        assert_eq!(rows[0].begin_date, Some("2024-01-01".to_string()));
        assert_eq!(rows[1].sequence, 2);
        assert_eq!(rows[1].weight, None);
    }

    #[test]
    fn parses_index_realtime_sw() {
        let rows = parse_realtime_sw(&results_of("index_realtime_sw.json"));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].index_code, "801010");
        assert_eq!(rows[0].index_name, "农林牧渔");
        assert_eq!(rows[0].prev_close, Some(3000.0));
        assert_eq!(rows[0].latest, Some(3020.0));
        assert_eq!(rows[0].volume, Some(98765.0));
        assert_eq!(rows[1].latest, Some(4100.0));
        assert_eq!(rows[1].volume, None);
    }

    #[test]
    fn parses_index_analysis_daily_sw() {
        let rows = parse_analysis_sw(&results_of("index_analysis_daily_sw.json"));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].index_code, "801010");
        assert_eq!(rows[0].index_name, "农林牧渔");
        assert_eq!(rows[0].date, Some("2024-10-25".to_string()));
        assert_eq!(rows[0].close_index, Some(3020.0));
        assert_eq!(rows[0].dp, None);
        assert_eq!(rows[1].dp, Some(1.5));
    }

    #[test]
    fn parses_index_analysis_week_month_sw() {
        let rows = parse_week_month_sw(&array_of("index_analysis_week_month_sw.json"));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, Some("2024-10-25".to_string()));
        assert_eq!(rows[1].date, Some("2024-10-18".to_string()));
    }

    #[test]
    fn parses_index_analysis_weekly_sw() {
        let rows = parse_analysis_sw(&results_of("index_analysis_weekly_sw.json"));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].index_code, "801010");
        assert_eq!(rows[0].close_index, Some(3020.0));
        assert_eq!(rows[1].close_index, Some(2980.0));
    }

    #[test]
    fn parses_index_analysis_monthly_sw() {
        let rows = parse_analysis_sw(&results_of("index_analysis_monthly_sw.json"));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].index_code, "801010");
        assert_eq!(rows[0].close_index, Some(3050.0));
        assert_eq!(rows[1].close_index, Some(3000.0));
    }
}
