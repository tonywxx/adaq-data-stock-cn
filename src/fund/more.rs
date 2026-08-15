//! Additional Eastmoney fund endpoints (akshare `fund` package), not yet in the
//! other `fund/*` modules.
//!
//! Ports six pure-HTTP fund functions that return JSON (or a JSON payload
//! wrapped in a thin `var NAME = ...;` assignment, which we unwrap without
//! evaluating JS — ADR-0005). All functions are async and take `&Client`.
//!
//! Ported here:
//! - [`fund_aum_trend_em`]     — fund-market AUM trend (`fund_aum_em.fund_aum_trend_em`)
//! - [`fund_name_em`]          — full fund code/name/type directory (`fund_em.fund_name_em`)
//! - [`fund_fh_em`]            — fund dividend history (`fund_fhsp_em.fund_fh_em`)
//! - [`fund_scale_change_em`]  — market-wide scale/Share change (`fund_scale_em.fund_scale_change_em`)
//! - [`fund_hold_structure_em`]- holder structure (`fund_scale_em.fund_hold_structure_em`)
//! - [`fund_manager_em`]       — fund-manager directory (`fund_manager.fund_manager_em`)
//!
//! Skipped (require HTML table parsing / JS signing, out of scope per task):
//! - `fund_portfolio_hold_em`, `fund_portfolio_bond_hold_em`, `fund_portfolio_change_em`
//!   — `FundArchivesDatas.aspx` returns an HTML fragment inside a JS object;
//!   parsing needs an HTML `<table>` reader (BeautifulSoup `read_html`).
//! - `fund_fee_em`, `fund_overview_em`, `fund_aum_em`, `fund_aum_hist_em`
//!   — pure HTML pages (`pd.read_html`), no JSON.
//! - `fund_portfolio_industry_allocation_em` — JSON but wrapped in a jQuery
//!   callback (`callback=jQuery...({...})`); needs callback-stripping + `demjson`.
//! - `fund_open_fund_rank_em` / `fund_exchange_rank_em` — `rankhandler.aspx`
//!   requires a per-request random `v` token and positional CSV columns.
//! - `fund_value_estimation_em` — static HTML scraping (`py_mini_racer` JS).

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};
use crate::fund::{fnum, fstr};

// ---------------------------------------------------------------------------
// URLs + fixed upstream params (replicated from akshare)
// ---------------------------------------------------------------------------

const AUM_TREND_URL: &str = "https://fund.eastmoney.com/Company/home/GetFundTotalScaleForChart";
const NAME_URL: &str = "https://fund.eastmoney.com/js/fundcode_search.js";
const FH_URL: &str = "https://fund.eastmoney.com/Data/funddataIndex_Interface.aspx";
const PORTFOLIO_URL: &str =
    "https://fund.eastmoney.com/data/FundDataPortfolio_Interface.aspx";
const MANAGER_URL: &str = "https://fund.eastmoney.com/Data/FundDataPortfolio_Interface.aspx";

// ---------------------------------------------------------------------------
// fund_aum_trend_em
// ---------------------------------------------------------------------------

/// Fund-market total AUM trend point (akshare `fund_aum_trend_em`).
///
/// Maps Eastmoney `GetFundTotalScaleForChart` `{x:[date], y:[scale]}` payload.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundAumTrendRow {
    /// akshare column `x`: period label, e.g. `2023-01` (YYYY-MM).
    pub date: String,
    /// akshare column `y`: total fund-market AUM (亿元 / 100M CNY).
    pub value: Option<f64>,
    pub source: &'static str,
}

/// Fund-market AUM trend from Eastmoney (`fund_aum_trend_em`).
///
/// Upstream is a plain JSON object `{x:[...], y:[...]}` (no JS wrapping).
pub async fn fund_aum_trend_em(client: &Client) -> Result<Vec<FundAumTrendRow>> {
    let params = [("fundType", "0")];
    let v = client
        .get_json(SOURCE_EASTMONEY, "fund_aum_trend_em", AUM_TREND_URL, &params)
        .await?;
    parse_aum_trend(&v)
}

pub(crate) fn parse_aum_trend(resp: &Value) -> Result<Vec<FundAumTrendRow>> {
    let x = resp
        .get("x")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "fund_aum_trend_em: missing x array".into(),
        })?;
    let y = resp
        .get("y")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "fund_aum_trend_em: missing y array".into(),
        })?;
    let n = x.len().min(y.len());
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(FundAumTrendRow {
            date: x[i].as_str().unwrap_or_default().to_string(),
            value: y[i].as_f64(),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// fund_name_em
// ---------------------------------------------------------------------------

/// Fund directory entry (akshare `fund_name_em`).
///
/// Maps Eastmoney `fundcode_search.js` `var r = [[...]]` array; each row is
/// `[基金代码, 拼音缩写, 基金简称, 基金类型, 拼音全称]`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundNameRow {
    /// akshare column `基金代码`.
    pub code: String,
    /// akshare column `拼音缩写`.
    pub pinyin_abbr: Option<String>,
    /// akshare column `基金简称`.
    pub name: String,
    /// akshare column `基金类型`.
    pub fund_type: Option<String>,
    /// akshare column `拼音全称`.
    pub pinyin_full: Option<String>,
    pub source: &'static str,
}

/// Full fund code/name/type directory from Eastmoney (`fund_name_em`).
///
/// Upstream is `var r = [[...]];` — a JSON array wrapped in a `var` assignment.
pub async fn fund_name_em(client: &Client) -> Result<Vec<FundNameRow>> {
    let text = client
        .get_text(SOURCE_EASTMONEY, "fund_name_em", NAME_URL, &[], None)
        .await?;
    let v: Value = serde_json::from_str(unwrap_json(&text)?).map_err(Error::Json)?;
    parse_name(&v)
}

pub(crate) fn parse_name(resp: &Value) -> Result<Vec<FundNameRow>> {
    let arr = resp
        .as_array()
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "fund_name_em: expected a JSON array".into(),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let cells = match item.as_array() {
            Some(c) => c,
            None => continue, // skip malformed rows
        };
        if cells.len() < 5 {
            continue; // skip malformed rows
        }
        out.push(FundNameRow {
            code: cell_str(cells, 0),
            pinyin_abbr: cell_opt_str(cells, 1),
            name: cell_str(cells, 2),
            fund_type: cell_opt_str(cells, 3),
            pinyin_full: cell_opt_str(cells, 4),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// fund_fh_em (基金分红 / dividend history)
// ---------------------------------------------------------------------------

/// Fund dividend record (akshare `fund_fh_em`).
///
/// Maps Eastmoney `funddataIndex_Interface.aspx` `dt=8` response. Each row of
/// the extracted `[[...]]` array is
/// `[序号, 基金代码, 基金简称, 权益登记日, 除息日期, 分红, 分红发放日, -]`;
/// the trailing 8th column is dropped.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundDividendRow {
    /// akshare column `序号`.
    pub seq: Option<f64>,
    /// akshare column `基金代码`.
    pub code: String,
    /// akshare column `基金简称`.
    pub name: String,
    /// akshare column `权益登记日` (record date, YYYY-MM-DD).
    pub register_date: String,
    /// akshare column `除息日期` (ex-dividend date, YYYY-MM-DD).
    pub ex_date: String,
    /// akshare column `分红` (dividend per unit, 元/份).
    pub dividend: Option<f64>,
    /// akshare column `分红发放日` (pay date, YYYY-MM-DD).
    pub pay_date: String,
    pub source: &'static str,
}

/// Fund dividend history from Eastmoney (`fund_fh_em`).
///
/// Upstream `funddataIndex_Interface.aspx?dt=8` returns a JS blob
/// `var jjfh_data=[[...]];var jjfh_jjgs=...;`; we slice out the `[[...]]`
/// array (no JS evaluation) and parse it positionally.
pub async fn fund_fh_em(client: &Client, year: &str, typ: &str) -> Result<Vec<FundDividendRow>> {
    let params = [
        ("dt", "8"),
        ("page", "1"),
        ("rank", "BZDM"),
        ("sort", "asc"),
        ("gs", ""),
        ("ftype", typ),
        ("year", year),
    ];
    let text = client
        .get_text(SOURCE_EASTMONEY, "fund_fh_em", FH_URL, &params, None)
        .await?;
    let v = extract_fh_array(&text)?;
    parse_fh(&v)
}

pub(crate) fn parse_fh(resp: &Value) -> Result<Vec<FundDividendRow>> {
    let arr = resp
        .as_array()
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "fund_fh_em: expected a JSON array".into(),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let cells = match item.as_array() {
            Some(c) => c,
            None => continue, // skip malformed rows
        };
        if cells.len() < 7 {
            continue; // skip malformed rows
        }
        out.push(FundDividendRow {
            seq: cell_f64(cells, 0),
            code: cell_str(cells, 1),
            name: cell_str(cells, 2),
            register_date: cell_str(cells, 3),
            ex_date: cell_str(cells, 4),
            dividend: cell_f64(cells, 5),
            pay_date: cell_str(cells, 6),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// fund_scale_change_em
// ---------------------------------------------------------------------------

/// Market-wide scale / share change record (akshare `fund_scale_change_em`).
///
/// Maps Eastmoney `FundDataPortfolio_Interface.aspx` `dt=9` `data[]` objects,
/// keyed by Chinese column names.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundScaleChangeRow {
    /// akshare column `截止日期` (report date, YYYY-MM-DD).
    pub report_date: String,
    /// akshare column `基金家数` (number of funds).
    pub fund_count: Option<f64>,
    /// akshare column `期间申购` (subscriptions over period, 亿份).
    pub subscribe: Option<f64>,
    /// akshare column `期间赎回` (redemptions over period, 亿份).
    pub redeem: Option<f64>,
    /// akshare column `期末总份额` (end-of-period total shares, 亿份).
    pub end_shares: Option<f64>,
    /// akshare column `期末净资产` (end-of-period net assets, 亿元).
    pub end_net_asset: Option<f64>,
    pub source: &'static str,
}

/// Market-wide scale/share change from Eastmoney (`fund_scale_change_em`).
///
/// Upstream is `var <name> = {"data":[...], "pages":N};`. Only the first page
/// (`pi=1`) is fetched; pagination is intentionally not implemented.
pub async fn fund_scale_change_em(client: &Client) -> Result<Vec<FundScaleChangeRow>> {
    let params = [
        ("dt", "9"),
        ("pi", "1"),
        ("pn", "50"),
        ("mc", "hypzDetail"),
        ("st", "desc"),
        ("sc", "reportdate"),
    ];
    let text = client
        .get_text(
            SOURCE_EASTMONEY,
            "fund_scale_change_em",
            PORTFOLIO_URL,
            &params,
            None,
        )
        .await?;
    let v: Value = serde_json::from_str(unwrap_json(&text)?).map_err(Error::Json)?;
    parse_scale_change(&v)
}

pub(crate) fn parse_scale_change(resp: &Value) -> Result<Vec<FundScaleChangeRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(FundScaleChangeRow {
            report_date: fstr(item, "截止日期"),
            fund_count: fnum(item, "基金家数"),
            subscribe: fnum(item, "期间申购"),
            redeem: fnum(item, "期间赎回"),
            end_shares: fnum(item, "期末总份额"),
            end_net_asset: fnum(item, "期末净资产"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// fund_hold_structure_em
// ---------------------------------------------------------------------------

/// Holder-structure record (akshare `fund_hold_structure_em`).
///
/// Maps Eastmoney `FundDataPortfolio_Interface.aspx` `dt=11` `data[]` objects,
/// keyed by Chinese column names.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundHoldStructureRow {
    /// akshare column `截止日期` (report date, YYYY-MM-DD).
    pub report_date: String,
    /// akshare column `基金家数` (number of funds).
    pub fund_count: Option<f64>,
    /// akshare column `机构持有比列` (institution holding ratio, %).
    pub institution_ratio: Option<f64>,
    /// akshare column `个人持有比列` (individual holding ratio, %).
    pub individual_ratio: Option<f64>,
    /// akshare column `内部持有比列` (internal holding ratio, %).
    pub internal_ratio: Option<f64>,
    /// akshare column `总份额` (total shares, 亿份).
    pub total_shares: Option<f64>,
    pub source: &'static str,
}

/// Holder structure from Eastmoney (`fund_hold_structure_em`).
///
/// Upstream is `var <name> = {"data":[...], "pages":N};`. Only the first page
/// (`pi=1`) is fetched; pagination is intentionally not implemented.
pub async fn fund_hold_structure_em(client: &Client) -> Result<Vec<FundHoldStructureRow>> {
    let params = [
        ("dt", "11"),
        ("pi", "1"),
        ("pn", "50"),
        ("mc", "hypzDetail"),
        ("st", "desc"),
        ("sc", "reportdate"),
    ];
    let text = client
        .get_text(
            SOURCE_EASTMONEY,
            "fund_hold_structure_em",
            PORTFOLIO_URL,
            &params,
            None,
        )
        .await?;
    let v: Value = serde_json::from_str(unwrap_json(&text)?).map_err(Error::Json)?;
    parse_hold_structure(&v)
}

pub(crate) fn parse_hold_structure(resp: &Value) -> Result<Vec<FundHoldStructureRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(FundHoldStructureRow {
            report_date: fstr(item, "截止日期"),
            fund_count: fnum(item, "基金家数"),
            institution_ratio: fnum(item, "机构持有比列"),
            individual_ratio: fnum(item, "个人持有比列"),
            internal_ratio: fnum(item, "内部持有比列"),
            total_shares: fnum(item, "总份额"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// fund_manager_em
// ---------------------------------------------------------------------------

/// Fund-manager directory entry (akshare `fund_manager_em`).
///
/// Maps Eastmoney `FundDataPortfolio_Interface.aspx` `dt=14` `data[]` objects,
/// keyed by Chinese column names.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundManagerRow {
    /// akshare column `姓名`.
    pub name: String,
    /// akshare column `所属公司`.
    pub company: String,
    /// akshare column `现任基金代码` (comma-joined fund codes).
    pub current_fund_codes: Option<String>,
    /// akshare column `现任基金` (comma-joined fund names).
    pub current_funds: Option<String>,
    /// akshare column `累计从业时间` (years in the industry).
    pub career_years: Option<f64>,
    /// akshare column `现任基金资产总规模` (current AUM managed, 亿元).
    pub current_scale: Option<f64>,
    /// akshare column `现任基金最佳回报` (best return on current funds, %).
    pub best_return: Option<f64>,
    pub source: &'static str,
}

/// Fund-manager directory from Eastmoney (`fund_manager_em`).
///
/// Upstream is `var returnjson = {"data":[...], "pages":N};`. Only the first
/// page (`pi=1`, `pn=500`) is fetched; pagination is intentionally not implemented.
pub async fn fund_manager_em(client: &Client) -> Result<Vec<FundManagerRow>> {
    let params = [
        ("dt", "14"),
        ("mc", "returnjson"),
        ("ft", "all"),
        ("pn", "500"),
        ("pi", "1"),
        ("sc", "abbname"),
        ("st", "asc"),
    ];
    let text = client
        .get_text(
            SOURCE_EASTMONEY,
            "fund_manager_em",
            MANAGER_URL,
            &params,
            None,
        )
        .await?;
    let v: Value = serde_json::from_str(unwrap_json(&text)?).map_err(Error::Json)?;
    parse_manager(&v)
}

pub(crate) fn parse_manager(resp: &Value) -> Result<Vec<FundManagerRow>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(FundManagerRow {
            name: fstr(item, "姓名"),
            company: fstr(item, "所属公司"),
            current_fund_codes: fstr_opt(item, "现任基金代码"),
            current_funds: fstr_opt(item, "现任基金"),
            career_years: fnum(item, "累计从业时间"),
            current_scale: fnum(item, "现任基金资产总规模"),
            best_return: fnum(item, "现任基金最佳回报"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

/// Extract the `data` array from an Eastmoney `{"data":[...], ...}` payload.
fn data_array(resp: &Value) -> Result<Vec<&Value>> {
    resp.get("data")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().collect())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data array".into(),
        })
}

/// Strip an Eastmoney `var NAME = <json>;` wrapper and return the inner JSON
/// Extract an optional string field from an Eastmoney item.
fn fstr_opt(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// object/array substring. Robust to any variable name; pure-JSON (unwrapped)
/// input is returned unchanged. No JS is evaluated.
fn unwrap_json(text: &str) -> Result<&str> {
    let s = text.trim_start();
    // Drop a leading `var <ident> = ` assignment if one is present (the `=`
    // must come before the first `{`/`[`, otherwise it is JSON-internal).
    let s = if let Some(eq) = s.find('=') {
        let bracket = s
            .find(['{', '['])
            .unwrap_or(usize::MAX);
        if eq < bracket {
            &s[eq + 1..]
        } else {
            s
        }
    } else {
        s
    };
    let s = s.trim_start();
    // Drop a single trailing `;`.
    let s = s.strip_suffix(';').unwrap_or(s).trim();
    let start = s
        .find(['{', '['])
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "expected JSON object/array".into(),
        })?;
    let end = s
        .rfind(['}', ']'])
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "expected JSON object/array".into(),
        })?;
    Ok(&s[start..=end])
}

/// Extract the dividend-data JSON array from a
/// `var jjfh_data=[[...]];var jjfh_jjgs=...` response (no JS evaluation).
fn extract_fh_array(text: &str) -> Result<Value> {
    let start = text.find("[[")
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "fund_fh_em: missing '[['".into(),
        })?;
    let end = text
        .find(";var jjfh_jjgs")
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "fund_fh_em: missing end marker".into(),
        })?;
    serde_json::from_str(&text[start..end]).map_err(Error::Json)
}

/// Cell accessor: string value (numbers rendered as their decimal text).
fn cell_str(cells: &[Value], i: usize) -> String {
    match cells.get(i) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

/// Cell accessor: optional string value (None when missing / null).
fn cell_opt_str(cells: &[Value], i: usize) -> Option<String> {
    match cells.get(i) {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    }
}

/// Cell accessor: numeric value (number, or numeric string).
fn cell_f64(cells: &[Value], i: usize) -> Option<f64> {
    cells.get(i).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Load a fixture as a parsed `Value` (fixtures store the parse-ready JSON
    /// payload, i.e. already unwrapped from any `var ... =` wrapper).
    fn fixture(name: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let txt = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    #[test]
    fn parses_aum_trend_fixture() {
        let v = fixture("fund_aum_trend_em.json");
        let rows = parse_aum_trend(&v).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].date, "2023-01");
        assert_eq!(rows[0].value, Some(260_000.0));
        assert_eq!(rows[2].date, "2023-03");
        assert_eq!(rows[2].value, Some(263_000.0));
        assert_eq!(rows[0].source, "eastmoney");
    }

    #[test]
    fn parses_name_fixture() {
        let v = fixture("fund_name_em.json");
        let rows = parse_name(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "000001");
        assert_eq!(rows[0].name, "华夏成长");
        assert_eq!(rows[0].fund_type.as_deref(), Some("混合型"));
        assert_eq!(rows[0].pinyin_abbr.as_deref(), Some("HXCZ"));
        assert_eq!(rows[1].code, "000002");
        assert_eq!(rows[1].name, "国泰金龙债券");
    }

    #[test]
    fn parses_fh_fixture() {
        let v = fixture("fund_fh_em.json");
        let rows = parse_fh(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].seq, Some(1.0));
        assert_eq!(rows[0].code, "000001");
        assert_eq!(rows[0].name, "华夏成长");
        assert_eq!(rows[0].register_date, "2025-01-10");
        assert_eq!(rows[0].ex_date, "2025-01-13");
        assert_eq!(rows[0].dividend, Some(0.5));
        assert_eq!(rows[0].pay_date, "2025-01-15");
        assert_eq!(rows[1].dividend, Some(0.3));
        assert_eq!(rows[1].source, "eastmoney");
    }

    #[test]
    fn parses_scale_change_fixture() {
        let v = fixture("fund_scale_change_em.json");
        let rows = parse_scale_change(&v).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].report_date, "2024-12-31");
        assert_eq!(rows[0].fund_count, Some(200.0));
        assert_eq!(rows[0].subscribe, Some(5000.2));
        assert_eq!(rows[0].redeem, Some(4800.1));
        assert_eq!(rows[0].end_shares, Some(30_000.5));
        assert_eq!(rows[0].end_net_asset, Some(45_000.3));
    }

    #[test]
    fn parses_hold_structure_fixture() {
        let v = fixture("fund_hold_structure_em.json");
        let rows = parse_hold_structure(&v).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].report_date, "2024-12-31");
        assert_eq!(rows[0].fund_count, Some(200.0));
        assert_eq!(rows[0].institution_ratio, Some(35.2));
        assert_eq!(rows[0].individual_ratio, Some(60.1));
        assert_eq!(rows[0].internal_ratio, Some(4.7));
        assert_eq!(rows[0].total_shares, Some(30_000.5));
    }

    #[test]
    fn parses_manager_fixture() {
        let v = fixture("fund_manager_em.json");
        let rows = parse_manager(&v).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "张三");
        assert_eq!(rows[0].company, "华夏基金");
        assert_eq!(rows[0].current_fund_codes.as_deref(), Some("000001,000002"));
        assert_eq!(rows[0].current_funds.as_deref(), Some("华夏成长,华夏收益"));
        assert_eq!(rows[0].career_years, Some(12.0));
        assert_eq!(rows[0].current_scale, Some(350.6));
        assert_eq!(rows[0].best_return, Some(45.3));
    }

    #[test]
    fn unwrap_json_strips_var_wrappers() {
        // object wrapper
        let obj = unwrap_json("var x = {\"a\":1};").unwrap();
        assert_eq!(obj, "{\"a\":1}");
        // array wrapper
        let arr = unwrap_json("var r = [[1,2]];").unwrap();
        assert_eq!(arr, "[[1,2]]");
        // pure JSON passthrough
        let pure = unwrap_json("{\"x\":[1],\"y\":[2]}").unwrap();
        assert_eq!(pure, "{\"x\":[1],\"y\":[2]}");
    }

    #[test]
    fn extract_fh_array_slices_correctly() {
        let raw = "var jjfh_data=[[1,\"000001\",\"华夏成长\",\"2025-01-10\",\"2025-01-13\",0.5,\"2025-01-15\"]];var jjfh_jjgs=[\"华夏成长\"];var jjfh_pages=1;";
        let v = extract_fh_array(raw).unwrap();
        let rows = parse_fh(&v).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].code, "000001");
        assert_eq!(rows[0].dividend, Some(0.5));
    }
}
