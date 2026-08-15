//! Margin financing / securities lending (融资融券) and earnings reports (业绩报表).
//!
//! Ports akshare functions that return pure-HTTP JSON (no JS signing, no
//! HTML scrape):
//!
//! | Rust fn                 | akshare fn            | Source | Notes                                  |
//! | ----------------------- | --------------------- | ------ | -------------------------------------- |
//! | `stock_margin_sh`       | `stock_margin_sse`    | SSE    | 上海证券交易所-融资融券汇总             |
//! | `stock_margin_sz`       | `stock_margin_szse`   | SZSE   | 深圳证券交易所-融资融券汇总             |
//! | `stock_yjbb_em`         | `stock_yjbb_em`       | Eastmoney | 东方财富-业绩报表                   |
//!
//! `stock_lh_data` does not exist in the current akshare tree (no function or
//! file by that name), so it is skipped. `stock_zh_a_gdhs` is ported elsewhere,
//! so it is skipped per the task brief.
//!
//! Source identifiers: Eastmoney rows use [`SOURCE_EASTMONEY`]; SSE/SZSE rows
//! use the module-local [`SOURCE_SSE`] / [`SOURCE_SZSE`] constants (the crate's
//! `client` only predefines Eastmoney/Sina/Tencent buckets).

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

/// SSE (Shanghai) source bucket, for rate limiting / error context.
const SOURCE_SSE: &str = "sse";
/// SZSE (Shenzhen) source bucket, for rate limiting / error context.
const SOURCE_SZSE: &str = "szse";

/// SSE margin summary endpoint (融资融券汇总).
const SSE_MARGIN_URL: &str = "https://query.sse.com.cn/marketdata/tradedata/queryMargin.do";
/// SZSE margin summary endpoint (融资融券汇总).
const SZSE_MARGIN_URL: &str = "https://www.szse.cn/api/report/ShowReport/data";
/// Eastmoney datacenter-web endpoint for the earnings report.
const YJBB_URL: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Read a string field, defaulting to `""` when missing/null (matches akshare,
/// which keeps such columns as empty strings rather than dropping the row).
fn fstr(item: &Value, k: &str) -> String {
    item.get(k)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

/// Read a numeric field that may be a JSON number or a plain numeric string.
fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

/// Read a numeric field that is a string with thousands separators (e.g. SZSE
/// returns `"7,077.67"`). Strips commas before parsing.
fn fnum_c(item: &Value, k: &str) -> Option<f64> {
    item.get(k)
        .and_then(|v| v.as_str())
        .and_then(|s| s.replace(',', "").trim().parse::<f64>().ok())
}

/// Validate an `YYYYMMDD` date string used as a request parameter.
fn check_date8(date: &str, what: &str) -> Result<()> {
    if date.len() == 8 && date.bytes().all(|b| b.is_ascii_digit()) {
        Ok(())
    } else {
        Err(Error::InvalidParam(format!(
            "{what} must be 8 ASCII digits YYYYMMDD, got {date:?}"
        )))
    }
}

/// Format `YYYYMMDD` as `YYYY-MM-DD` (the form SZSE/Eastmoney filters expect).
fn dashed(date: &str) -> String {
    format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..])
}

// ===========================================================================
// Shanghai margin summary (stock_margin_sse -> stock_margin_sh)
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct MarginShRow {
    /// 信用交易日期 (opDate), e.g. "20230922".
    #[serde(rename = "opDate")]
    pub credit_trade_date: String,
    /// 融资余额 (rzye), in 元.
    #[serde(rename = "rzye")]
    pub financing_balance: Option<f64>,
    /// 融资买入额 (rzmre), in 元.
    #[serde(rename = "rzmre")]
    pub financing_buy_amount: Option<f64>,
    /// 融券余量 (rqyl), in 股.
    #[serde(rename = "rqyl")]
    pub securities_balance_volume: Option<f64>,
    /// 融券余量金额 (rqylje), in 元.
    #[serde(rename = "rqylje")]
    pub securities_balance_amount: Option<f64>,
    /// 融券卖出量 (rqmcl), in 股.
    #[serde(rename = "rqmcl")]
    pub securities_sell_volume: Option<f64>,
    /// 融资融券余额 (rzrqjyzl), in 元.
    #[serde(rename = "rzrqjyzl")]
    pub margin_balance: Option<f64>,
    pub source: &'static str,
}

/// Port of akshare `stock_margin_sse(start_date, end_date)` — Shanghai Stock
/// Exchange margin-trading summary between two dates (akshare column
/// 信用交易日期 / 融资余额 / 融资买入额 / 融券余量 / 融券余量金额 / 融券卖出量 /
/// 融资融券余额).
pub async fn stock_margin_sh(
    client: &Client,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<MarginShRow>> {
    check_date8(start_date, "start_date")?;
    check_date8(end_date, "end_date")?;
    let params = [
        ("isPagination", "true"),
        ("beginDate", start_date),
        ("endDate", end_date),
        ("tabType", ""),
        ("stockCode", ""),
        ("pageHelp.pageSize", "5000"),
        ("pageHelp.pageNo", "1"),
        ("pageHelp.beginPage", "1"),
        ("pageHelp.cacheSize", "1"),
        ("pageHelp.endPage", "5"),
    ];
    let v = client
        .get_json(SOURCE_SSE, "stock_margin_sh", SSE_MARGIN_URL, &params)
        .await?;
    parse_margin_sh(&v)
}

/// Parse an SSE margin-summary response (the `result` array).
pub(crate) fn parse_margin_sh(resp: &Value) -> Result<Vec<MarginShRow>> {
    let data = resp
        .get("result")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SSE,
            message: "missing result array at stock_margin_sh".into(),
        })?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(MarginShRow {
            credit_trade_date: fstr(item, "opDate"),
            financing_balance: fnum(item, "rzye"),
            financing_buy_amount: fnum(item, "rzmre"),
            securities_balance_volume: fnum(item, "rqyl"),
            securities_balance_amount: fnum(item, "rqylje"),
            securities_sell_volume: fnum(item, "rqmcl"),
            margin_balance: fnum(item, "rzrqjyzl"),
            source: SOURCE_SSE,
        });
    }
    Ok(out)
}

// ===========================================================================
// Shenzhen margin summary (stock_margin_szse -> stock_margin_sz)
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct MarginSzRow {
    /// 融资买入额 (jrrzmr), in 亿元.
    #[serde(rename = "jrrzmr")]
    pub financing_buy_amount: Option<f64>,
    /// 融资余额 (jrrzye), in 亿元.
    #[serde(rename = "jrrzye")]
    pub financing_balance: Option<f64>,
    /// 融券卖出量 (jrrjmc), in 亿股/亿份.
    #[serde(rename = "jrrjmc")]
    pub securities_sell_volume: Option<f64>,
    /// 融券余量 (jrrjyl), in 亿股/亿份.
    #[serde(rename = "jrrjyl")]
    pub securities_balance_volume: Option<f64>,
    /// 融券余额 (jrrjye), in 亿元.
    #[serde(rename = "jrrjye")]
    pub securities_balance_amount: Option<f64>,
    /// 融资融券余额 (jrrzrjye), in 亿元.
    #[serde(rename = "jrrzrjye")]
    pub margin_balance: Option<f64>,
    pub source: &'static str,
}

/// Port of akshare `stock_margin_szse(date)` — Shenzhen Stock Exchange
/// margin-trading summary for a single trading date (akshare column
/// 融资买入额 / 融资余额 / 融券卖出量 / 融券余量 / 融券余额 / 融资融券余额).
/// Values are reported by SZSE in 亿 (hundred-million) units.
pub async fn stock_margin_sz(client: &Client, date: &str) -> Result<Vec<MarginSzRow>> {
    check_date8(date, "date")?;
    let txt = dashed(date);
    let params = [
        ("SHOWTYPE", "JSON"),
        ("CATALOGID", "1837_xxpl"),
        ("txtDate", txt.as_str()),
        ("tab1PAGENO", "1"),
        ("random", "0.7425245522795993"),
    ];
    let v = client
        .get_json(SOURCE_SZSE, "stock_margin_sz", SZSE_MARGIN_URL, &params)
        .await?;
    parse_margin_sz(&v)
}

/// Parse an SZSE margin-summary response (top-level array, first element's
/// `data` array).
pub(crate) fn parse_margin_sz(resp: &Value) -> Result<Vec<MarginSzRow>> {
    let data = resp
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|o| o.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SZSE,
            message: "missing data array at stock_margin_sz".into(),
        })?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(MarginSzRow {
            financing_buy_amount: fnum_c(item, "jrrzmr"),
            financing_balance: fnum_c(item, "jrrzye"),
            securities_sell_volume: fnum_c(item, "jrrjmc"),
            securities_balance_volume: fnum_c(item, "jrrjyl"),
            securities_balance_amount: fnum_c(item, "jrrjye"),
            margin_balance: fnum_c(item, "jrrzrjye"),
            source: SOURCE_SZSE,
        });
    }
    Ok(out)
}

// ===========================================================================
// Eastmoney earnings report (stock_yjbb_em)
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct YjbbRow {
    /// SECUCODE, e.g. "000637.SZ".
    #[serde(rename = "SECUCODE")]
    pub secucode: String,
    /// 股票代码 (SECURITY_CODE).
    #[serde(rename = "SECURITY_CODE")]
    pub security_code: String,
    /// 股票简称 (SECURITY_NAME_ABBR).
    #[serde(rename = "SECURITY_NAME_ABBR")]
    pub security_name: String,
    /// ORG_CODE.
    #[serde(rename = "ORG_CODE")]
    pub org_code: String,
    /// 报告期 (REPORTDATE), e.g. "2022-03-31 00:00:00".
    #[serde(rename = "REPORTDATE")]
    pub report_date: String,
    /// 最新公告日期 (NOTICE_DATE).
    #[serde(rename = "NOTICE_DATE")]
    pub notice_date: String,
    /// 所处行业 (PUBLISHNAME).
    #[serde(rename = "PUBLISHNAME")]
    pub industry: String,
    /// 每股收益 (BASIC_EPS).
    #[serde(rename = "BASIC_EPS")]
    pub basic_eps: Option<f64>,
    /// 营业总收入-营业总收入 (TOTAL_OPERATE_INCOME).
    #[serde(rename = "TOTAL_OPERATE_INCOME")]
    pub total_operate_income: Option<f64>,
    /// 营业总收入-同比增长 (YSTZ).
    #[serde(rename = "YSTZ")]
    pub total_income_yoy: Option<f64>,
    /// 营业总收入-季度环比增长 (YSHZ).
    #[serde(rename = "YSHZ")]
    pub total_income_qoq: Option<f64>,
    /// 净利润-净利润 (PARENT_NETPROFIT).
    #[serde(rename = "PARENT_NETPROFIT")]
    pub parent_netprofit: Option<f64>,
    /// 净利润-同比增长 (SJLTZ).
    #[serde(rename = "SJLTZ")]
    pub netprofit_yoy: Option<f64>,
    /// 净利润-季度环比增长 (SJLHZ).
    #[serde(rename = "SJLHZ")]
    pub netprofit_qoq: Option<f64>,
    /// 每股净资产 (BPS).
    #[serde(rename = "BPS")]
    pub bps: Option<f64>,
    /// 净资产收益率 (WEIGHTAVG_ROE).
    #[serde(rename = "WEIGHTAVG_ROE")]
    pub weightavg_roe: Option<f64>,
    /// 每股经营现金流量 (MGJYXJJE).
    #[serde(rename = "MGJYXJJE")]
    pub mgjyxjje: Option<f64>,
    /// 销售毛利率 (XSMLL).
    #[serde(rename = "XSMLL")]
    pub xsmll: Option<f64>,
    pub source: &'static str,
}

/// Port of akshare `stock_yjbb_em(date)` — Eastmoney earnings report
/// (业绩报表) for the reporting period `date` (format `YYYYMMDD`, e.g.
/// `20220331`). Returns up to 500 rows of the first page.
pub async fn stock_yjbb_em(client: &Client, date: &str) -> Result<Vec<YjbbRow>> {
    check_date8(date, "date")?;
    let report = dashed(date);
    let filter = format!("(REPORTDATE='{report}')");
    let params = [
        ("sortColumns", "UPDATE_DATE,SECURITY_CODE"),
        ("sortTypes", "-1,-1"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("reportName", "RPT_LICO_FN_CPD"),
        ("columns", "ALL"),
        ("filter", filter.as_str()),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_yjbb_em", YJBB_URL, &params)
        .await?;
    parse_yjbb(&v)
}

/// Parse an Eastmoney earnings-report response (`result.data`).
pub(crate) fn parse_yjbb(resp: &Value) -> Result<Vec<YjbbRow>> {
    let data = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data at stock_yjbb_em".into(),
        })?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(YjbbRow {
            secucode: fstr(item, "SECUCODE"),
            security_code: fstr(item, "SECURITY_CODE"),
            security_name: fstr(item, "SECURITY_NAME_ABBR"),
            org_code: fstr(item, "ORG_CODE"),
            report_date: fstr(item, "REPORTDATE"),
            notice_date: fstr(item, "NOTICE_DATE"),
            industry: fstr(item, "PUBLISHNAME"),
            basic_eps: fnum(item, "BASIC_EPS"),
            total_operate_income: fnum(item, "TOTAL_OPERATE_INCOME"),
            total_income_yoy: fnum(item, "YSTZ"),
            total_income_qoq: fnum(item, "YSHZ"),
            parent_netprofit: fnum(item, "PARENT_NETPROFIT"),
            netprofit_yoy: fnum(item, "SJLTZ"),
            netprofit_qoq: fnum(item, "SJLHZ"),
            bps: fnum(item, "BPS"),
            weightavg_roe: fnum(item, "WEIGHTAVG_ROE"),
            mgjyxjje: fnum(item, "MGJYXJJE"),
            xsmll: fnum(item, "XSMLL"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

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

    #[test]
    fn parses_margin_sh_fixture() {
        let v = fixture("stock_margin_sh.json");
        let rows = parse_margin_sh(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].credit_trade_date, "20230922");
        assert_eq!(rows[0].source, "sse");
        assert_eq!(rows[0].financing_balance, Some(801_602_446_146.0));
        assert_eq!(rows[0].financing_buy_amount, Some(30_926_857_859.0));
        assert_eq!(rows[0].securities_balance_volume, Some(7_214_266_710.0));
        assert_eq!(rows[0].securities_balance_amount, Some(56_778_317_878.0));
        assert_eq!(rows[0].securities_sell_volume, Some(735_951_864.0));
        assert_eq!(rows[0].margin_balance, Some(858_380_764_024.0));
        assert_eq!(rows[1].credit_trade_date, "20230921");
    }

    #[test]
    fn parses_margin_sz_fixture() {
        let v = fixture("stock_margin_sz.json");
        let rows = parse_margin_sz(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].source, "szse");
        // SZSE reports in 亿 (hundred-million) units, with thousands separators.
        assert_eq!(rows[0].financing_buy_amount, Some(321.08));
        assert_eq!(rows[0].financing_balance, Some(7_077.67));
        assert_eq!(rows[0].securities_sell_volume, Some(0.28));
        assert_eq!(rows[0].securities_balance_volume, Some(24.34));
        assert_eq!(rows[0].securities_balance_amount, Some(157.30));
        assert_eq!(rows[0].margin_balance, Some(7_234.97));
        assert_eq!(rows[1].financing_balance, Some(7_050.12));
    }

    #[test]
    fn parses_yjbb_em_fixture() {
        let v = fixture("stock_yjbb_em.json");
        let rows = parse_yjbb(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[0].secucode, "000637.SZ");
        assert_eq!(rows[0].security_code, "000637");
        assert_eq!(rows[0].security_name, "茂化实华");
        assert_eq!(rows[0].industry, "炼化及贸易");
        assert_eq!(rows[0].report_date, "2022-03-31 00:00:00");
        assert_eq!(rows[0].notice_date, "2022-04-29 00:00:00");
        assert_eq!(rows[0].basic_eps, Some(-0.02));
        assert_eq!(rows[0].total_operate_income, Some(1_912_733_727.04));
        assert_eq!(rows[0].parent_netprofit, Some(-7_538_271.02));
        assert_eq!(rows[0].weightavg_roe, Some(-0.73));
        assert_eq!(rows[1].secucode, "603843.SH");
        assert_eq!(rows[1].security_name, "*ST正平");
        assert_eq!(rows[1].bps, Some(2.663397610591));
    }

    #[test]
    fn rejects_malformed_margin_sh() {
        let bad = serde_json::json!({ "result": [ { "opDate": "20230922" } ] });
        let rows = parse_margin_sh(&bad).unwrap();
        // All fields optional -> row still produced, missing numbers are None.
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].financing_balance, None);
    }
}
