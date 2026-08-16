//! 东方财富网-数据中心-限售股解禁 (akshare `akshare/stock_fundamental/stock_restricted_em.py`).
//!
//! Ported public functions (all pure Eastmoney JSON via the `datacenter-web`
//! API; no JS execution, token, signature, `execjs`/`MiniRacer`, cookie, HTML or
//! Excel scraping):
//!
//! | Rust fn                              | akshare fn                         | akshare file:line            | Report (`reportName`)   |
//! |--------------------------------------|-----------------------------------|------------------------------|-------------------------|
//! | `stock_restricted_release_summary_em`  | `stock_restricted_release_summary_em`  | `stock_restricted_em.py:14`  | `RPT_LIFTDAY_STA`       |
//! | `stock_restricted_release_detail_em`  | `stock_restricted_release_detail_em`  | `stock_restricted_em.py:106` | `RPT_LIFT_STAGE`        |
//! | `stock_restricted_release_queue_em`   | `stock_restricted_release_queue_em`   | `stock_restricted_em.py:209` | `RPT_LIFT_STAGE`        |
//! | `stock_restricted_release_stockholder_em` | `stock_restricted_release_stockholder_em` | `stock_restricted_em.py:301` | `RPT_LIFT_GD`       |
//!
//! ## Field-name fidelity
//!
//! For `detail_em` / `queue_em` / `stockholder_em`, akshare passes an explicit
//! `columns=` list to the datacenter API, so the upstream JSON object keys are
//! the **real** Eastmoney field names. akshare then discards them with a
//! positional `df.columns = [...]` relabel; we drop that relabel and keep the
//! real keys, ported exactly (the mapping is documented inline on each struct
//! field as `akshare column -> upstream key`).
//!
//! For `summary_em`, akshare requests `columns="ALL"` and relabels positionally,
//! so the real upstream keys are **not** recoverable. Its field names are
//! **INFERRED** from the report name `RPT_LIFTDAY_STA` and column semantics, and
//! are flagged `inferred` in the doc comments. They must be verified against a
//! live sample before production use.
//!
//! ## Unit scaling (parity with akshare)
//!
//! akshare multiplies every share-count (`*_SHARES`) and market-cap (`*_CAP`)
//! field by `10000` (Eastmoney returns them in 万股 / 万元). To match akshare's
//! documented units (股 / 元) we replicate that `× 10000` in the parsers below.
//! `stockholder_em` is exempt — akshare does **not** scale it.
//!
//! ## DEFERRED
//!
//! - `stock_circulate_em` — no such Eastmoney fn exists in this akshare checkout;
//!   the expected file `akshare/stock_feature/stock_circulate_em.py` is absent.
//! - `stock_circulate_stock_holder` — defined in `akshare/stock_fundamental/stock_finance_sina.py`
//!   and implemented with `pd.read_html` (HTML table scraping), outside pure-HTTP scope.
//! - `stock_restricted_release_queue_sina` / `*_sina` — defined in the broad
//!   `stock_finance_sina.py`, outside the scoped topic file; not Eastmoney JSON.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_EASTMONEY: &str = "eastmoney";
const DATACENTER: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";

// ---------------------------------------------------------------------------
// Helpers (local, per porting brief)
// ---------------------------------------------------------------------------

/// Read a string field.
fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Read a numeric field that may be a JSON number or a plain numeric string.
fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

/// Read an integer field that may be a JSON number or a plain integer string.
#[allow(dead_code)]
fn inum(item: &Value, k: &str) -> Option<i64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    })
}

/// Extract `result.data` (the row array) from a datacenter-web response.
fn result_data(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data".into(),
        })
}

/// Validate an `YYYYMMDD` date and return it dashed as `YYYY-MM-DD`
/// (Eastmoney's expected `FREE_DATE` form).
fn fmt_date8(date: &str) -> Result<String> {
    if date.len() == 8 && date.bytes().all(|b| b.is_ascii_digit()) {
        Ok(format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8]))
    } else {
        Err(Error::InvalidParam(format!(
            "date must be 8 ASCII digits YYYYMMDD, got {date:?}"
        )))
    }
}

// ===========================================================================
// stock_restricted_release_summary_em — 限售股解禁-每日解禁汇总
// ===========================================================================

/// Eastmoney index-code values for each `symbol` (mirrors akshare `symbol_map`).
fn summary_index_code(symbol: &str) -> Result<&'static str> {
    match symbol {
        "全部股票" => Ok("000300"),
        "沪市A股" => Ok("000001"),
        "科创板" => Ok("000688"),
        "深市A股" => Ok("399001"),
        "创业板" => Ok("399001"),
        "京市A股" => Ok("999999"),
        other => Err(Error::InvalidParam(format!(
            "stock_restricted_release_summary_em: symbol must be one of \
             {{\"全部股票\", \"沪市A股\", \"科创板\", \"深市A股\", \"创业板\", \"京市A股\"}}, got {other:?}"
        ))),
    }
}

/// One daily restricted-share-lifting summary row, port of
/// `stock_restricted_release_summary_em` (Eastmoney `RPT_LIFTDAY_STA`).
///
/// **Field names are INFERRED** — akshare requests `columns="ALL"` and relabels
/// positionally, so the real upstream keys are unrecoverable. Verify against a
/// live sample before production use.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockRestrictedReleaseSummary {
    /// 解禁时间 (inferred `FREE_DATE`)
    pub free_date: Option<String>,
    /// 当日解禁股票家数 (inferred `LIFT_STOCKS_NUM`)
    pub lift_stocks_num: Option<f64>,
    /// 解禁数量 (inferred `LIFT_SHARES`, ×10000)
    pub lift_shares: Option<f64>,
    /// 实际解禁数量 (inferred `ACTUAL_LIFT_SHARES`, ×10000)
    pub actual_lift_shares: Option<f64>,
    /// 实际解禁市值 (inferred `ACTUAL_LIFT_MARKET_CAP`, ×10000)
    pub actual_lift_market_cap: Option<f64>,
    /// 沪深300指数 (inferred `INDEX_VALUE`)
    pub index_value: Option<f64>,
    /// 沪深300指数涨跌幅 (inferred `INDEX_CHANGE_PCT`)
    pub index_change_pct: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_restricted_release_summary_em(symbol, start_date, end_date)`.
pub async fn stock_restricted_release_summary_em(
    client: &Client,
    symbol: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<StockRestrictedReleaseSummary>> {
    let code = summary_index_code(symbol)?;
    let s = fmt_date8(start_date)?;
    let e = fmt_date8(end_date)?;
    let filter = format!("(INDEX_CODE=\"{code}\")(FREE_DATE>='{s}')(FREE_DATE<='{e}')");
    let params = [
        ("sortColumns", "FREE_DATE"),
        ("sortTypes", "1"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("columns", "ALL"),
        (
            "quoteColumns",
            "f2~03~INDEX_CODE,f3~03~INDEX_CODE,f124~03~INDEX_CODE",
        ),
        ("quoteType", "0"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
        ("reportName", "RPT_LIFTDAY_STA"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_restricted_release_summary_em",
            DATACENTER,
            &params,
        )
        .await?;
    parse_release_summary(&v)
}

pub(crate) fn parse_release_summary(resp: &Value) -> Result<Vec<StockRestrictedReleaseSummary>> {
    let data = result_data(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(StockRestrictedReleaseSummary {
            free_date: fstr(item, "FREE_DATE"),
            lift_stocks_num: fnum(item, "LIFT_STOCKS_NUM"),
            lift_shares: fnum(item, "LIFT_SHARES").map(|x| x * 10000.0),
            actual_lift_shares: fnum(item, "ACTUAL_LIFT_SHARES").map(|x| x * 10000.0),
            actual_lift_market_cap: fnum(item, "ACTUAL_LIFT_MARKET_CAP").map(|x| x * 10000.0),
            index_value: fnum(item, "INDEX_VALUE"),
            index_change_pct: fnum(item, "INDEX_CHANGE_PCT"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_restricted_release_detail_em — 限售股解禁-解禁详情一览
// ===========================================================================

/// Explicit `columns` list requested from `RPT_LIFT_STAGE` (the real upstream keys).
const LIFT_STAGE_COLUMNS: &str = "SECURITY_CODE,SECURITY_NAME_ABBR,FREE_DATE,CURRENT_FREE_SHARES,\
ABLE_FREE_SHARES,LIFT_MARKET_CAP,FREE_RATIO,NEW,B20_ADJCHRATE,A20_ADJCHRATE,FREE_SHARES_TYPE,\
TOTAL_RATIO,NON_FREE_SHARES,BATCH_HOLDER_NUM";

/// One restricted-share-lifting detail row, port of
/// `stock_restricted_release_detail_em` (Eastmoney `RPT_LIFT_STAGE`).
///
/// Field names are the **real** upstream keys (akshare passes `columns=` to the
/// API; only its positional df relabel is discarded).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockRestrictedReleaseDetail {
    /// 股票代码 (`SECURITY_CODE`)
    pub security_code: Option<String>,
    /// 股票简称 (`SECURITY_NAME_ABBR`)
    pub security_name: Option<String>,
    /// 解禁时间 (`FREE_DATE`)
    pub free_date: Option<String>,
    /// 实际解禁数量 (`CURRENT_FREE_SHARES`, ×10000)
    pub actual_lift_shares: Option<f64>,
    /// 解禁数量 (`ABLE_FREE_SHARES`, ×10000)
    pub lift_shares: Option<f64>,
    /// 实际解禁市值 (`LIFT_MARKET_CAP`, ×10000)
    pub lift_market_cap: Option<f64>,
    /// 占解禁前流通市值比例 (`FREE_RATIO`)
    pub free_ratio: Option<f64>,
    /// 解禁前一交易日收盘价 (`NEW`)
    pub prev_close: Option<f64>,
    /// 解禁前20日涨跌幅 (`B20_ADJCHRATE`)
    pub b20_adjchrate: Option<f64>,
    /// 解禁后20日涨跌幅 (`A20_ADJCHRATE`)
    pub a20_adjchrate: Option<f64>,
    /// 限售股类型 (`FREE_SHARES_TYPE`)
    pub free_shares_type: Option<String>,
    pub source: &'static str,
}

/// Port of `stock_restricted_release_detail_em(start_date, end_date)`.
///
/// Fetches a single page (`pageSize=500`). akshare loops all `result.pages`,
/// but the brief's single-`get_json` contract is honored here; callers needing
/// the full history should page the `FREE_DATE` window.
pub async fn stock_restricted_release_detail_em(
    client: &Client,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<StockRestrictedReleaseDetail>> {
    let s = fmt_date8(start_date)?;
    let e = fmt_date8(end_date)?;
    let filter = format!("(FREE_DATE>='{s}')(FREE_DATE<='{e}')");
    let params = [
        ("sortColumns", "FREE_DATE,CURRENT_FREE_SHARES"),
        ("sortTypes", "1,1"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("reportName", "RPT_LIFT_STAGE"),
        ("columns", LIFT_STAGE_COLUMNS),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_restricted_release_detail_em",
            DATACENTER,
            &params,
        )
        .await?;
    parse_release_detail(&v)
}

pub(crate) fn parse_release_detail(resp: &Value) -> Result<Vec<StockRestrictedReleaseDetail>> {
    let data = result_data(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(StockRestrictedReleaseDetail {
            security_code: fstr(item, "SECURITY_CODE"),
            security_name: fstr(item, "SECURITY_NAME_ABBR"),
            free_date: fstr(item, "FREE_DATE"),
            actual_lift_shares: fnum(item, "CURRENT_FREE_SHARES").map(|x| x * 10000.0),
            lift_shares: fnum(item, "ABLE_FREE_SHARES").map(|x| x * 10000.0),
            lift_market_cap: fnum(item, "LIFT_MARKET_CAP").map(|x| x * 10000.0),
            free_ratio: fnum(item, "FREE_RATIO"),
            prev_close: fnum(item, "NEW"),
            b20_adjchrate: fnum(item, "B20_ADJCHRATE"),
            a20_adjchrate: fnum(item, "A20_ADJCHRATE"),
            free_shares_type: fstr(item, "FREE_SHARES_TYPE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_restricted_release_queue_em — 个股限售解禁-解禁批次
// ===========================================================================

/// One per-stock lifting batch row, port of `stock_restricted_release_queue_em`
/// (Eastmoney `RPT_LIFT_STAGE`, same columns as the detail endpoint).
///
/// Field names are the **real** upstream keys (akshare passes `columns=` to the
/// API; only its positional df relabel is discarded).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockRestrictedReleaseQueue {
    /// 解禁时间 (`FREE_DATE`)
    pub free_date: Option<String>,
    /// 解禁股东数 (`BATCH_HOLDER_NUM`)
    pub batch_holder_num: Option<f64>,
    /// 解禁数量 (`ABLE_FREE_SHARES`, ×10000)
    pub lift_shares: Option<f64>,
    /// 实际解禁数量 (`CURRENT_FREE_SHARES`, ×10000)
    pub actual_lift_shares: Option<f64>,
    /// 未解禁数量 (`NON_FREE_SHARES`, ×10000)
    pub non_free_shares: Option<f64>,
    /// 实际解禁数量市值 (`LIFT_MARKET_CAP`, ×10000)
    pub lift_market_cap: Option<f64>,
    /// 占总市值比例 (`TOTAL_RATIO`)
    pub total_ratio: Option<f64>,
    /// 占流通市值比例 (`FREE_RATIO`)
    pub free_ratio: Option<f64>,
    /// 解禁前一交易日收盘价 (`NEW`)
    pub prev_close: Option<f64>,
    /// 限售股类型 (`FREE_SHARES_TYPE`)
    pub free_shares_type: Option<String>,
    /// 解禁前20日涨跌幅 (`B20_ADJCHRATE`)
    pub b20_adjchrate: Option<f64>,
    /// 解禁后20日涨跌幅 (`A20_ADJCHRATE`)
    pub a20_adjchrate: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_restricted_release_queue_em(symbol)`.
///
/// Mirrors akshare's empty-result handling: when Eastmoney returns
/// `result: null` (no lifting batches for the symbol) an empty `Vec` is
/// returned (akshare returns an empty frame).
pub async fn stock_restricted_release_queue_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<StockRestrictedReleaseQueue>> {
    let filter = format!("(SECURITY_CODE=\"{symbol}\")");
    let params = [
        ("sortColumns", "FREE_DATE"),
        ("sortTypes", "-1"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("reportName", "RPT_LIFT_STAGE"),
        ("columns", LIFT_STAGE_COLUMNS),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_restricted_release_queue_em",
            DATACENTER,
            &params,
        )
        .await?;
    parse_release_queue(&v)
}

pub(crate) fn parse_release_queue(resp: &Value) -> Result<Vec<StockRestrictedReleaseQueue>> {
    // akshare: `if not data_json["result"]: return pd.DataFrame()`
    match resp.get("result") {
        None | Some(Value::Null) => return Ok(Vec::new()),
        Some(_) => {}
    }
    let data = result_data(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(StockRestrictedReleaseQueue {
            free_date: fstr(item, "FREE_DATE"),
            batch_holder_num: fnum(item, "BATCH_HOLDER_NUM"),
            lift_shares: fnum(item, "ABLE_FREE_SHARES").map(|x| x * 10000.0),
            actual_lift_shares: fnum(item, "CURRENT_FREE_SHARES").map(|x| x * 10000.0),
            non_free_shares: fnum(item, "NON_FREE_SHARES").map(|x| x * 10000.0),
            lift_market_cap: fnum(item, "LIFT_MARKET_CAP").map(|x| x * 10000.0),
            total_ratio: fnum(item, "TOTAL_RATIO"),
            free_ratio: fnum(item, "FREE_RATIO"),
            prev_close: fnum(item, "NEW"),
            free_shares_type: fstr(item, "FREE_SHARES_TYPE"),
            b20_adjchrate: fnum(item, "B20_ADJCHRATE"),
            a20_adjchrate: fnum(item, "A20_ADJCHRATE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_restricted_release_stockholder_em — 个股限售解禁-解禁股东
// ===========================================================================

/// Explicit `columns` list requested from `RPT_LIFT_GD` (the real upstream keys).
const LIFT_GD_COLUMNS: &str = "LIMITED_HOLDER_NAME,ADD_LISTING_SHARES,ACTUAL_LISTED_SHARES,\
ADD_LISTING_CAP,LOCK_MONTH,RESIDUAL_LIMITED_SHARES,FREE_SHARES_TYPE,PLAN_FEATURE";

/// One lifting shareholder row, port of `stock_restricted_release_stockholder_em`
/// (Eastmoney `RPT_LIFT_GD`).
///
/// Field names are the **real** upstream keys (akshare passes `columns=` to the
/// API; only its positional df relabel is discarded). Note akshare does **not**
/// scale these fields (no `× 10000`), so values are kept as returned.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockRestrictedReleaseStockholder {
    /// 股东名称 (`LIMITED_HOLDER_NAME`)
    pub holder_name: Option<String>,
    /// 解禁数量 (`ADD_LISTING_SHARES`)
    pub add_listing_shares: Option<f64>,
    /// 实际解禁数量 (`ACTUAL_LISTED_SHARES`)
    pub actual_listed_shares: Option<f64>,
    /// 解禁市值 (`ADD_LISTING_CAP`)
    pub add_listing_cap: Option<f64>,
    /// 锁定期 (`LOCK_MONTH`)
    pub lock_month: Option<f64>,
    /// 剩余未解禁数量 (`RESIDUAL_LIMITED_SHARES`)
    pub residual_limited_shares: Option<f64>,
    /// 限售股类型 (`FREE_SHARES_TYPE`)
    pub free_shares_type: Option<String>,
    /// 进度 (`PLAN_FEATURE`)
    pub plan_feature: Option<String>,
    pub source: &'static str,
}

/// Port of `stock_restricted_release_stockholder_em(symbol, date)`.
pub async fn stock_restricted_release_stockholder_em(
    client: &Client,
    symbol: &str,
    date: &str,
) -> Result<Vec<StockRestrictedReleaseStockholder>> {
    let d = fmt_date8(date)?;
    let filter = format!("(SECURITY_CODE=\"{symbol}\")(FREE_DATE='{d}')");
    let params = [
        ("sortColumns", "ADD_LISTING_SHARES"),
        ("sortTypes", "-1"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("reportName", "RPT_LIFT_GD"),
        ("columns", LIFT_GD_COLUMNS),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_restricted_release_stockholder_em",
            DATACENTER,
            &params,
        )
        .await?;
    parse_release_stockholder(&v)
}

pub(crate) fn parse_release_stockholder(
    resp: &Value,
) -> Result<Vec<StockRestrictedReleaseStockholder>> {
    let data = result_data(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(StockRestrictedReleaseStockholder {
            holder_name: fstr(item, "LIMITED_HOLDER_NAME"),
            add_listing_shares: fnum(item, "ADD_LISTING_SHARES"),
            actual_listed_shares: fnum(item, "ACTUAL_LISTED_SHARES"),
            add_listing_cap: fnum(item, "ADD_LISTING_CAP"),
            lock_month: fnum(item, "LOCK_MONTH"),
            residual_limited_shares: fnum(item, "RESIDUAL_LIMITED_SHARES"),
            free_shares_type: fstr(item, "FREE_SHARES_TYPE"),
            plan_feature: fstr(item, "PLAN_FEATURE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// Tests — offline, against fixtures in tests/fixtures/
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
    fn parses_release_summary() {
        let rows =
            parse_release_summary(&fixture("stock_restricted_release_summary_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].free_date, Some("2022-11-08".to_string()));
        assert_eq!(rows[0].lift_stocks_num, Some(12.0));
        // ×10000 scaling applied (akshare parity)
        assert_eq!(rows[0].lift_shares, Some(100.0 * 10000.0));
        assert_eq!(rows[0].actual_lift_shares, Some(80.0 * 10000.0));
        assert_eq!(rows[0].actual_lift_market_cap, Some(5000.0 * 10000.0));
        assert_eq!(rows[0].index_value, Some(3700.0));
        assert_eq!(rows[0].index_change_pct, Some(1.2));
        assert_eq!(rows[1].index_change_pct, Some(-0.5));
    }

    #[test]
    fn parses_release_detail() {
        let rows =
            parse_release_detail(&fixture("stock_restricted_release_detail_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].security_code, Some("600000".to_string()));
        assert_eq!(rows[0].security_name, Some("浦发银行".to_string()));
        assert_eq!(rows[0].free_date, Some("2022-12-02".to_string()));
        assert_eq!(rows[0].actual_lift_shares, Some(10.0 * 10000.0));
        assert_eq!(rows[0].lift_shares, Some(9.0 * 10000.0));
        assert_eq!(rows[0].lift_market_cap, Some(100.0 * 10000.0));
        assert_eq!(rows[0].free_ratio, Some(1.5));
        assert_eq!(
            rows[0].free_shares_type,
            Some("首发原股东限售股份".to_string())
        );
        assert_eq!(rows[1].b20_adjchrate, Some(-1.0));
    }

    #[test]
    fn parses_release_queue() {
        let rows = parse_release_queue(&fixture("stock_restricted_release_queue_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].free_date, Some("2022-12-02".to_string()));
        assert_eq!(rows[0].batch_holder_num, Some(3.0));
        assert_eq!(rows[0].lift_shares, Some(9.0 * 10000.0));
        assert_eq!(rows[0].actual_lift_shares, Some(10.0 * 10000.0));
        assert_eq!(rows[0].non_free_shares, Some(20.0 * 10000.0));
        assert_eq!(rows[0].lift_market_cap, Some(100.0 * 10000.0));
        assert_eq!(rows[1].total_ratio, Some(0.2));
    }

    #[test]
    fn parses_release_queue_empty() {
        // akshare returns an empty frame when result is null
        let rows =
            parse_release_queue(&fixture("stock_restricted_release_queue_em_empty.json")).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn parses_release_stockholder() {
        let rows =
            parse_release_stockholder(&fixture("stock_restricted_release_stockholder_em.json"))
                .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].holder_name, Some("张三".to_string()));
        assert_eq!(rows[0].add_listing_shares, Some(100.0));
        assert_eq!(rows[0].actual_listed_shares, Some(90.0));
        assert_eq!(rows[0].add_listing_cap, Some(800.0));
        assert_eq!(rows[0].lock_month, Some(12.0));
        assert_eq!(rows[0].residual_limited_shares, Some(10.0));
        assert_eq!(
            rows[0].free_shares_type,
            Some("首发原股东限售股份".to_string())
        );
        assert_eq!(rows[0].plan_feature, Some("已实施".to_string()));
        assert_eq!(rows[1].lock_month, Some(24.0));
    }
}
