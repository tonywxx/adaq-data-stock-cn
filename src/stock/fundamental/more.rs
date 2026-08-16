//! Long-tail of `stock_fundamental` — tractable JSON endpoints.
//!
//! Ports the remaining akshare fundamental/financial functions that hit plain
//! HTTP JSON APIs (no JS signing, no HTML scrape):
//!
//! - Sina `CompanyFinanceService.getFinanceReport2022` (财务报表三大报表 +
//!   关键指标) — [`stock_financial_report_sina`], [`stock_financial_abstract`].
//! - Eastmoney `datacenter` A-share equity structure (股本结构) —
//!   [`stock_zh_a_gbjg_em`].
//! - Eastmoney fund/institution holdings (基金持仓) —
//!   [`stock_report_fund_hold`], [`stock_report_fund_hold_detail`].
//!
//! All field names were verified against the live upstream responses. Raw API
//! values are stored as-is (units preserved, e.g. 万股/万元); akshare's
//! `* 10000` post-processing is intentionally NOT replicated, matching the
//! raw-value convention used by `eastmoney.rs`.
//!
//! | Rust fn                                  | akshare fn                                | source    | akshare file:line                                  |
//! |------------------------------------------|-------------------------------------------|-----------|----------------------------------------------------|
//! | `stock_financial_report_sina`            | `stock_financial_report_sina`            | sina      | `akshare/stock_fundamental/stock_finance_sina.py:24` |
//! | `stock_financial_abstract`               | `stock_financial_abstract`               | sina      | `akshare/stock_fundamental/stock_finance_sina.py:94` |
//! | `stock_zh_a_gbjg_em`                     | `stock_zh_a_gbjg_em`                     | eastmoney | `akshare/stock_fundamental/stock_gbjg_em.py:62` |
//! | `stock_report_fund_hold`                 | `stock_report_fund_hold`                 | eastmoney | `akshare/stock/stock_fund_hold.py:13` |
//! | `stock_report_fund_hold_detail`          | `stock_report_fund_hold_detail`          | eastmoney | `akshare/stock/stock_fund_hold.py:110` |
//!
//! ## DEFERRED (HTML scrape / jsonp / emweb / not in akshare — do NOT fake)
//!
//! - `stock_financial_analysis_indicator` (sina) — `read_html` HTML table scrape.
//! - `stock_history_dividend` / `stock_history_dividend_detail` — `read_html`.
//! - `stock_ipo_info` / `stock_add_stock` — `read_html`.
//! - `stock_circulate_stock_holder` / `stock_main_stock_holder` / `stock_fund_stock_holder` —
//!   `read_html` HTML table scrape.
//! - `stock_institute_hold` — `read_html`; `stock_institute_hold_detail` — Sina
//!   `jsonp.php` + `demjson` (non-strict JSON wrapper).
//! - `stock_profit_forecast_hk_etnet` — HK etnet HTML/JS.
//! - `stock_zyjs_ths` — 同花顺 HTML.
//! - `stock_yjbb_xq` — does not exist in akshare.
//!
//! ## Skipped — already ported elsewhere in this crate
//!
//! - `stock_profit_sheet_by_report_em` / `stock_balance_sheet_by_report_em` /
//!   `stock_cash_flow_sheet_by_report_em` / `stock_financial_analysis_indicator_em`
//!   → `eastmoney.rs`.
//! - `stock_yjbb_em` → `src/stock/margin.rs`.
//! - `stock_zygc_em` → `src/stock/holder.rs`.
//! - `stock_zcfz_em` / `stock_lrb_em` / `stock_xjll_em` → `src/stock/financial.rs`.
//! - `stock_financial_debt_ths` / `*_benefit_ths` / `*_cash_ths` / `*_new_ths` /
//!   `*_hk_*` / `*_us_*` / `stock_profit_forecast_em` / registration fns →
//!   `finance_more.rs` / `registration.rs`.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY, SOURCE_SINA};
use crate::core::error::{Error, Result};

/// Eastmoney `datacenter-web` (WEB) REST endpoint — no JS signing.
const EM_WEB: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";
/// Eastmoney `datacenter` securities REST endpoint — no JS signing.
const EM_SEC: &str = "https://datacenter.eastmoney.com/securities/api/data/v1/get";
/// Sina financial-report JSON API.
const SINA_REPORT_URL: &str =
    "https://quotes.sina.cn/cn/api/openapi.php/CompanyFinanceService.getFinanceReport2022";
/// Eastmoney fund-holding `zlsj/list` API (proxies to datacenter-web).
const FUND_HOLD_URL: &str = "https://data.eastmoney.com/dataapi/zlsj/list";

/// Empty array used when an optional JSON array is missing.
static EMPTY_ARR: Vec<Value> = Vec::new();

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Read a numeric field that may be a JSON number or a (comma-grouped) string.
fn num_of(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => {
            let t = s.trim().replace(',', "");
            if t.is_empty() {
                None
            } else {
                t.parse::<f64>().ok()
            }
        }
        _ => None,
    }
}

/// Read an optional string field (missing / null / empty → `None`).
fn str_of(v: Option<&Value>) -> Option<String> {
    match v {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

/// Read a string-or-number field as `String` (missing → empty).
fn fstr(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

/// Read an `update_time`-style timestamp as its raw string form.
fn ts_str(v: Option<&Value>) -> Option<String> {
    match v {
        Some(Value::Number(n)) => Some(n.to_string()),
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

/// Extract the `result.data` array shared by every `datacenter*` response.
fn web_data<'a>(resp: &'a Value, endpoint: &'static str) -> Result<&'a [Value]> {
    resp.get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .map(|a| a.as_slice())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: format!("missing result.data at {endpoint}"),
        })
}

/// Format an `yyyymmdd` date as `yyyy-mm-dd` (Eastmoney filter form).
fn fmt_date(s: &str) -> Result<String> {
    if s.len() != 8 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::InvalidParam(format!(
            "expected yyyymmdd date, got {s:?}"
        )));
    }
    Ok(format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8]))
}

// ---------------------------------------------------------------------------
// Sina financial report (财务报表三大报表)
// ---------------------------------------------------------------------------

/// One (报告期, 指标) cell of a Sina `getFinanceReport2022` statement.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SinaFinanceReportRow {
    /// 报告期 date key (e.g. `2024-03-31`).
    pub report_date: String,
    /// 指标名称 (`item_title`).
    pub item_title: String,
    /// 指标值 (`item_value`); comma-grouped strings parsed to f64.
    pub item_value: Option<f64>,
    /// 数据源 (`data_source`).
    pub data_source: Option<String>,
    /// 是否审计 (`is_audit`).
    pub is_audit: Option<String>,
    /// 公告日期 (`publish_date`).
    pub publish_date: Option<String>,
    /// 币种 (`rCurrency`).
    pub currency: Option<String>,
    /// 类型 (`rType`).
    pub report_type: Option<String>,
    /// 更新时间戳 (`update_time`, raw).
    pub update_time: Option<String>,
    /// Data source (`sina`).
    pub source: &'static str,
}

/// Parse a Sina financial-report response into [`SinaFinanceReportRow`]s.
pub(crate) fn parse_sina_finance_report(resp: &Value) -> Result<Vec<SinaFinanceReportRow>> {
    let list = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.get("report_list"))
        .and_then(|m| m.as_object())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "missing result.data.report_list".into(),
        })?;
    let mut out = Vec::new();
    for (date, block) in list {
        let data = match block.get("data").and_then(|v| v.as_array()) {
            Some(a) => a,
            None => &EMPTY_ARR,
        };
        let data_source = str_of(block.get("data_source"));
        let is_audit = str_of(block.get("is_audit"));
        let publish_date = str_of(block.get("publish_date"));
        let currency = str_of(block.get("rCurrency"));
        let report_type = str_of(block.get("rType"));
        let update_time = ts_str(block.get("update_time"));
        for item in data {
            out.push(SinaFinanceReportRow {
                report_date: date.clone(),
                item_title: fstr(item.get("item_title")),
                item_value: num_of(item.get("item_value")),
                data_source: data_source.clone(),
                is_audit: is_audit.clone(),
                publish_date: publish_date.clone(),
                currency: currency.clone(),
                report_type: report_type.clone(),
                update_time: update_time.clone(),
                source: SOURCE_SINA,
            });
        }
    }
    Ok(out)
}

/// Port of `stock_financial_report_sina(stock, symbol)`.
///
/// `stock` is the Sina code, e.g. `sh600600`; `symbol` ∈ {"资产负债表",
/// "利润表", "现金流量表"}.
pub async fn stock_financial_report_sina(
    client: &Client,
    stock: &str,
    symbol: &str,
) -> Result<Vec<SinaFinanceReportRow>> {
    let source = match symbol {
        "资产负债表" => "fzb",
        "利润表" => "lrb",
        "现金流量表" => "llb",
        other => {
            return Err(Error::InvalidParam(format!(
                "stock_financial_report_sina: unknown symbol {other:?}"
            )));
        }
    };
    let params = [
        ("paperCode", stock),
        ("source", source),
        ("type", "0"),
        ("page", "1"),
        ("num", "1000"),
    ];
    let v = client
        .get_json(
            SOURCE_SINA,
            "stock_financial_report_sina",
            SINA_REPORT_URL,
            &params,
        )
        .await?;
    parse_sina_finance_report(&v)
}

// ---------------------------------------------------------------------------
// Sina financial abstract (关键指标)
// ---------------------------------------------------------------------------

/// One (报告期, 指标) cell of a Sina `getFinanceReport2022` abstract (关键指标).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SinaFinanceAbstractRow {
    /// 报告期 date key (e.g. `2024-03-31`).
    pub report_date: String,
    /// 指标名称 (`item_title`).
    pub item_title: String,
    /// 指标值 (`item_value`).
    pub item_value: Option<f64>,
    /// Data source (`sina`).
    pub source: &'static str,
}

/// Parse a Sina financial-abstract response into [`SinaFinanceAbstractRow`]s.
pub(crate) fn parse_sina_finance_abstract(resp: &Value) -> Result<Vec<SinaFinanceAbstractRow>> {
    let list = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.get("report_list"))
        .and_then(|m| m.as_object())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "missing result.data.report_list".into(),
        })?;
    let mut out = Vec::new();
    for (date, block) in list {
        let data = match block.get("data").and_then(|v| v.as_array()) {
            Some(a) => a,
            None => &EMPTY_ARR,
        };
        for item in data {
            out.push(SinaFinanceAbstractRow {
                report_date: date.clone(),
                item_title: fstr(item.get("item_title")),
                item_value: num_of(item.get("item_value")),
                source: SOURCE_SINA,
            });
        }
    }
    Ok(out)
}

/// Port of `stock_financial_abstract(symbol)` — Sina 关键指标.
///
/// `symbol` is a 6-digit code; it is sent to Sina as `sh{symbol}`.
pub async fn stock_financial_abstract(
    client: &Client,
    symbol: &str,
) -> Result<Vec<SinaFinanceAbstractRow>> {
    let paper_code = format!("sh{symbol}");
    let params = [
        ("paperCode", paper_code.as_str()),
        ("source", "gjzb"),
        ("type", "0"),
        ("page", "1"),
        ("num", "1000"),
    ];
    let v = client
        .get_json(
            SOURCE_SINA,
            "stock_financial_abstract",
            SINA_REPORT_URL,
            &params,
        )
        .await?;
    parse_sina_finance_abstract(&v)
}


/// Port of `stock_zh_a_gbjg_em(symbol)` — Eastmoney A股 股本结构.
///
/// `symbol` accepts `603392.SH`, `SH603392` or `603392`.
/// One row of the A-share equity-structure report (股本结构).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockZhAGbjgRow {
    /// `SECUCODE` (e.g. `603392.SH`).
    pub secucode: String,
    /// `SECURITY_CODE`.
    pub security_code: String,
    /// `END_DATE`.
    pub end_date: String,
    /// 总股本 (`TOTAL_SHARES`).
    pub total_shares: Option<f64>,
    /// 流通A股受限 (`LIMITED_A_SHARES`).
    pub limited_a_shares: Option<f64>,
    /// 其他限售 (`LIMITED_OTHARS`).
    pub limited_othars: Option<f64>,
    /// 境内自然人限售 (`LIMITED_DOMESTIC_NATURAL`).
    pub limited_domestic_natural: Option<f64>,
    /// 境内非国有限售 (`LIMITED_DOMESTIC_NOSTATE`).
    pub limited_domestic_nostate: Option<f64>,
    /// 流通股本 (`FREE_SHARES`).
    pub free_shares: Option<f64>,
    /// 已上市流通A股 (`LISTED_A_SHARES`).
    pub listed_a_shares: Option<f64>,
    /// 变动原因 (`CHANGE_REASON`).
    pub change_reason: Option<String>,
}

/// Normalize a stock code to Eastmoney `SECUCODE` form (`603392.SH`).
///
/// Accepts `603392.SH`, `SH603392` or `603392`.
fn gbjg_symbol(symbol: &str) -> Result<String> {
    if let Some((code, ex)) = symbol.split_once('.') {
        return Ok(format!("{}.{}", code, ex.to_uppercase()));
    }
    if symbol.len() > 2 && symbol[..2].chars().all(|c| c.is_ascii_alphabetic()) {
        let ex = &symbol[..2];
        let code = &symbol[2..];
        return Ok(format!("{}.{}", code, ex.to_uppercase()));
    }
    Ok(format!("{symbol}.SH"))
}

/// Parse an `RPT_F10_EH_EQUITY` response into [`StockZhAGbjgRow`]s.
pub(crate) fn parse_stock_zh_a_gbjg_em(resp: &Value) -> Result<Vec<StockZhAGbjgRow>> {
    let data = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .map(|a| a.as_slice())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data at parse_stock_zh_a_gbjg_em".into(),
        })?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(StockZhAGbjgRow {
            secucode: fstr(item.get("SECUCODE")),
            security_code: fstr(item.get("SECURITY_CODE")),
            end_date: fstr(item.get("END_DATE")),
            total_shares: num_of(item.get("TOTAL_SHARES")),
            limited_a_shares: num_of(item.get("LIMITED_A_SHARES")),
            limited_othars: num_of(item.get("LIMITED_OTHARS")),
            limited_domestic_natural: num_of(item.get("LIMITED_DOMESTIC_NATURAL")),
            limited_domestic_nostate: num_of(item.get("LIMITED_DOMESTIC_NOSTATE")),
            free_shares: num_of(item.get("FREE_SHARES")),
            listed_a_shares: num_of(item.get("LISTED_A_SHARES")),
            change_reason: str_of(item.get("CHANGE_REASON")),
        });
    }
    Ok(out)
}

/// Port of `stock_zh_a_gbjg_em(symbol)` — Eastmoney A股 股本结构.
///
/// `symbol` accepts `603392.SH`, `SH603392` or `603392`.
pub async fn stock_zh_a_gbjg_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<StockZhAGbjgRow>> {
    let secucode = gbjg_symbol(symbol)?;
    let filter = format!("(SECUCODE=\"{secucode}\")");
    let params = [
        ("reportName", "RPT_F10_EH_EQUITY"),
        (
            "columns",
            "SECUCODE,SECURITY_CODE,END_DATE,TOTAL_SHARES,LIMITED_SHARES,LIMITED_OTHARS,\
LIMITED_DOMESTIC_NATURAL,LIMITED_STATE_LEGAL,LIMITED_OVERSEAS_NOSTATE,LIMITED_OVERSEAS_NATURAL,\
UNLIMITED_SHARES,LISTED_A_SHARES,B_FREE_SHARE,H_FREE_SHARE,FREE_SHARES,LIMITED_A_SHARES,\
NON_FREE_SHARES,LIMITED_B_SHARES,OTHER_FREE_SHARES,LIMITED_STATE_SHARES,\
LIMITED_DOMESTIC_NOSTATE,LOCK_SHARES,LIMITED_FOREIGN_SHARES,LIMITED_H_SHARES,\
SPONSOR_SHARES,STATE_SPONSOR_SHARES,SPONSOR_SOCIAL_SHARES,RAISE_SHARES,\
RAISE_STATE_SHARES,RAISE_DOMESTIC_SHARES,RAISE_OVERSEAS_SHARES,CHANGE_REASON",
        ),
        ("quoteColumns", ""),
        ("filter", filter.as_str()),
        ("pageNumber", "1"),
        ("pageSize", "500"),
        ("sortTypes", "-1"),
        ("sortColumns", "END_DATE"),
        ("source", "HSF10"),
        ("client", "PC"),
        ("v", "047483522105257925"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_zh_a_gbjg_em",
            EM_SEC,
            &params,
        )
        .await?;
    parse_stock_zh_a_gbjg_em(&v)
}

// ---------------------------------------------------------------------------
// Eastmoney fund/institution holdings (基金持仓)
// ---------------------------------------------------------------------------

/// One row of the market-wide fund/institution holding report (基金持仓).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundHoldRow {
    /// `SECUCODE` (e.g. `300750.SZ`).
    pub secucode: String,
    /// `SECURITY_CODE`.
    pub security_code: String,
    /// `SECURITY_NAME_ABBR`.
    pub security_name: String,
    /// `REPORT_DATE`.
    pub report_date: String,
    /// 机构类型 (`ORG_TYPE_NAME`).
    pub org_type_name: Option<String>,
    /// 持有基金家数 (`HOULD_NUM`).
    pub hould_num: Option<f64>,
    /// 持股总数 (`TOTAL_SHARES`).
    pub total_shares: Option<f64>,
    /// 持股市值 (`HOLD_VALUE`).
    pub hold_value: Option<f64>,
    /// 占总股本比例 (`TOTALSHARES_RATIO`).
    pub total_shares_ratio: Option<f64>,
    /// 占流通股比例 (`FREESHARES_RATIO`).
    pub freeshares_ratio: Option<f64>,
    /// 持股变化 (`HOLDCHA`).
    pub holdcha: Option<String>,
    /// 持股变动数值 (`HOLDCHA_NUM`).
    pub holdcha_num: Option<f64>,
    /// 持股变动比例 (`HOLDCHA_RATIO`).
    pub holdcha_ratio: Option<f64>,
    /// Data source (`eastmoney`).
    pub source: &'static str,
}

/// Parse a `zlsj/list` response (top-level `data`/`pages`) into [`FundHoldRow`]s.
pub(crate) fn parse_stock_report_fund_hold(resp: &Value) -> Result<Vec<FundHoldRow>> {
    let data = resp
        .get("data")
        .and_then(|d| d.as_array())
        .map(|a| a.as_slice())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing top-level data at stock_report_fund_hold".into(),
        })?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(FundHoldRow {
            secucode: fstr(item.get("SECUCODE")),
            security_code: fstr(item.get("SECURITY_CODE")),
            security_name: fstr(item.get("SECURITY_NAME_ABBR")),
            report_date: fstr(item.get("REPORT_DATE")),
            org_type_name: str_of(item.get("ORG_TYPE_NAME")),
            hould_num: num_of(item.get("HOULD_NUM")),
            total_shares: num_of(item.get("TOTAL_SHARES")),
            hold_value: num_of(item.get("HOLD_VALUE")),
            total_shares_ratio: num_of(item.get("TOTALSHARES_RATIO")),
            freeshares_ratio: num_of(item.get("FREESHARES_RATIO")),
            holdcha: str_of(item.get("HOLDCHA")),
            holdcha_num: num_of(item.get("HOLDCHA_NUM")),
            holdcha_ratio: num_of(item.get("HOLDCHA_RATIO")),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

/// Port of `stock_report_fund_hold(symbol, date)` — Eastmoney 基金持仓.
///
/// `symbol` ∈ {"基金持仓","QFII持仓","社保持仓","券商持仓","保险持仓","信托持仓"};
/// `date` is `yyyymmdd`.
pub async fn stock_report_fund_hold(
    client: &Client,
    symbol: &str,
    date: &str,
) -> Result<Vec<FundHoldRow>> {
    let typ = match symbol {
        "基金持仓" => "1",
        "QFII持仓" => "2",
        "社保持仓" => "3",
        "券商持仓" => "4",
        "保险持仓" => "5",
        "信托持仓" => "6",
        other => {
            return Err(Error::InvalidParam(format!(
                "stock_report_fund_hold: unknown symbol {other:?}"
            )));
        }
    };
    let date_str = fmt_date(date)?;
    let mut out = Vec::new();
    let mut page: u32 = 1;
    loop {
        let pn = page.to_string();
        let params = [
            ("date", date_str.as_str()),
            ("type", typ),
            ("zjc", "0"),
            ("sortField", "HOULD_NUM"),
            ("sortDirec", "1"),
            ("pageNum", pn.as_str()),
            ("pageSize", "500"),
            ("p", pn.as_str()),
            ("pageNo", pn.as_str()),
        ];
        let v = client
            .get_json(
                SOURCE_EASTMONEY,
                "stock_report_fund_hold",
                FUND_HOLD_URL,
                &params,
            )
            .await?;
        out.extend(parse_stock_report_fund_hold(&v)?);
        let pages = v.get("pages").and_then(|p| p.as_u64()).unwrap_or(1);
        if u64::from(page) >= pages {
            break;
        }
        page += 1;
    }
    Ok(out)
}

/// One row of a single fund's holdings detail (基金持仓-明细).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundHoldDetailRow {
    /// `SECUCODE` (e.g. `601012.SH`).
    pub secucode: String,
    /// `SECURITY_CODE`.
    pub security_code: String,
    /// `SECURITY_NAME_ABBR`.
    pub security_name: String,
    /// `REPORT_DATE`.
    pub report_date: String,
    /// `HOLDER_CODE` (fund code).
    pub holder_code: String,
    /// `HOLDER_NAME`.
    pub holder_name: String,
    /// 机构类型 (`ORG_TYPE`).
    pub org_type: Option<String>,
    /// 持股数 (`TOTAL_SHARES`).
    pub total_shares: Option<f64>,
    /// 持股市值 (`HOLD_MARKET_CAP`).
    pub hold_market_cap: Option<f64>,
    /// 占总股本比例 (`TOTAL_SHARES_RATIO`).
    pub total_shares_ratio: Option<f64>,
    /// 占流通股本比例 (`FREE_SHARES_RATIO`).
    pub free_shares_ratio: Option<f64>,
    /// 占净值比例 (`NETASSET_RATIO`).
    pub netasset_ratio: Option<f64>,
    /// Data source (`eastmoney`).
    pub source: &'static str,
}

/// Parse a `RPT_MAINDATA_MAIN_POSITIONDETAILS` response into [`FundHoldDetailRow`]s.
pub(crate) fn parse_stock_report_fund_hold_detail(
    resp: &Value,
) -> Result<Vec<FundHoldDetailRow>> {
    let data = web_data(resp, "stock_report_fund_hold_detail")?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(FundHoldDetailRow {
            secucode: fstr(item.get("SECUCODE")),
            security_code: fstr(item.get("SECURITY_CODE")),
            security_name: fstr(item.get("SECURITY_NAME_ABBR")),
            report_date: fstr(item.get("REPORT_DATE")),
            holder_code: fstr(item.get("HOLDER_CODE")),
            holder_name: fstr(item.get("HOLDER_NAME")),
            org_type: str_of(item.get("ORG_TYPE")),
            total_shares: num_of(item.get("TOTAL_SHARES")),
            hold_market_cap: num_of(item.get("HOLD_MARKET_CAP")),
            total_shares_ratio: num_of(item.get("TOTAL_SHARES_RATIO")),
            free_shares_ratio: num_of(item.get("FREE_SHARES_RATIO")),
            netasset_ratio: num_of(item.get("NETASSET_RATIO")),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

/// Port of `stock_report_fund_hold_detail(symbol, date)` — 基金持仓-明细.
///
/// `symbol` is a fund code (e.g. `008286`); `date` is `yyyymmdd`.
pub async fn stock_report_fund_hold_detail(
    client: &Client,
    symbol: &str,
    date: &str,
) -> Result<Vec<FundHoldDetailRow>> {
    let date_str = fmt_date(date)?;
    let filter = format!("(HOLDER_CODE=\"{symbol}\")(REPORT_DATE='{date_str}')");
    let params = [
        ("sortColumns", "SECURITY_CODE"),
        ("sortTypes", "-1"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("reportName", "RPT_MAINDATA_MAIN_POSITIONDETAILS"),
        ("columns", "ALL"),
        ("quoteColumns", ""),
        ("filter", filter.as_str()),
        ("source", "WEB"),
        ("client", "WEB"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_report_fund_hold_detail",
            EM_WEB,
            &params,
        )
        .await?;
    parse_stock_report_fund_hold_detail(&v)
}

// ===========================================================================
// Tests — offline, against fixtures in tests/fixtures/
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let p = p.join("tests/fixtures").join(name);
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    /// Approximate float comparison for `Option<f64>` fields (never `.unwrap()`).
    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    #[test]
    fn parses_stock_financial_report_sina() {
        let rows = parse_sina_finance_report(&fixture("stock_financial_report_sina.json")).unwrap();
        assert_eq!(rows.len(), 3);
        // serde_json::Map iterates in sorted key order, so key by (date, title).
        let r = rows
            .iter()
            .find(|r| r.report_date == "2024-03-31" && r.item_title == "货币资金")
            .unwrap();
        assert!(approx(r.item_value, 1_234_567.89));
        assert_eq!(r.data_source.as_deref(), Some("新浪财经"));
        assert_eq!(r.update_time.as_deref(), Some("1714521600"));
        // empty item_value -> None
        let empty = rows
            .iter()
            .find(|r| r.report_date == "2024-03-31" && r.item_title == "应收账款")
            .unwrap();
        assert_eq!(empty.item_value, None);
        let older = rows
            .iter()
            .find(|r| r.report_date == "2023-12-31" && r.item_title == "货币资金")
            .unwrap();
        assert!(approx(older.item_value, 987_654.32));
    }

    #[test]
    fn parses_stock_financial_abstract() {
        let rows = parse_sina_finance_abstract(&fixture("stock_financial_abstract.json")).unwrap();
        assert_eq!(rows.len(), 3);
        let r = rows
            .iter()
            .find(|r| r.report_date == "2024-03-31" && r.item_title == "每股收益")
            .unwrap();
        assert!(approx(r.item_value, 1.23));
        let none = rows
            .iter()
            .find(|r| r.report_date == "2024-03-31" && r.item_title == "净资产收益率")
            .unwrap();
        assert_eq!(none.item_value, None);
        let older = rows
            .iter()
            .find(|r| r.report_date == "2023-12-31" && r.item_title == "每股收益")
            .unwrap();
        assert!(approx(older.item_value, 1.10));
    }


    #[test]
    fn parses_stock_zh_a_gbjg_em() {
        let rows = parse_stock_zh_a_gbjg_em(&fixture("stock_zh_a_gbjg_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].secucode, "603392.SH");
        assert!(approx(rows[0].total_shares, 500_000_000.0));
        assert!(approx(rows[0].limited_a_shares, 100_000_000.0));
        assert!(approx(rows[0].listed_a_shares, 390_000_000.0));
        assert_eq!(rows[0].change_reason.as_deref(), Some("增发"));
        assert_eq!(rows[1].total_shares, None);
    }

    #[test]
    fn parses_stock_report_fund_hold() {
        let rows = parse_stock_report_fund_hold(&fixture("stock_report_fund_hold.json")).unwrap();
        assert_eq!(rows.len(), 2);
        // keyed lookup (Map key order is not stable; find by code)
        let maotai = rows.iter().find(|r| r.security_code == "600519").unwrap();
        assert_eq!(maotai.security_name, "贵州茅台");
        assert!(approx(maotai.hould_num, 1796.0));
        assert_eq!(maotai.holdcha.as_deref(), Some("减仓"));
        assert!(approx(maotai.holdcha_num, -41406.0));
        assert!(approx(maotai.holdcha_ratio, -0.04));
    }

    #[test]
    fn parses_stock_report_fund_hold_detail() {
        let rows = parse_stock_report_fund_hold_detail(&fixture(
            "stock_report_fund_hold_detail.json",
        ))
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].holder_name, "易方达研究精选股票");
        assert!(approx(rows[0].total_shares, 12_287_846.0));
        assert!(approx(rows[0].total_shares_ratio, 0.22700819));
        assert!(approx(rows[0].free_shares_ratio, 0.22701322));
        assert_eq!(rows[1].total_shares, None);
    }
}
