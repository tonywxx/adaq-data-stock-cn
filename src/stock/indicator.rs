//! A股 估值指标 / 市盈率 / 估值分析 (valuation & indicator endpoints) ported
//! from `akshare`.
//!
//! | Rust fn                          | akshare fn                        | Source     | File:line                                            | Notes                              |
//! |----------------------------------|-----------------------------------|------------|------------------------------------------------------|------------------------------------|
//! | `stock_zh_valuation_baidu`       | `stock_zh_valuation_baidu`        | 百度股市通 | `akshare/stock_feature/stock_zh_valuation_baidu.py:13` | A股 估值数据 (总市值/市盈率/市净率/市现率) |
//! | `stock_zh_valuation_comparison_em` | `stock_zh_valuation_comparison_em` | 东方财富   | `akshare/stock/stock_zh_comparison_em.py:72`          | 同行比较-估值比较 (PE/PB/PS/PEG/EV-EBITDA) |
//! | `stock_value_em`                 | `stock_value_em`                  | 东方财富   | `akshare/stock_feature/stock_value_em.py:14`          | 估值分析 (每日 PE(TTM)/PE(静)/PB/PEG/PS/PCF) |
//!
//! All three ports are pure-JSON HTTP (Eastmoney datacenter / Baidu opendata).
//! No JS signing, token, `execjs`/`MiniRacer`, cookie, HTML or Excel scraping.
//!
//! ## DEFERRED (out of scope — require JS / token / HTML)
//!
//! - `stock_a_pe_and_pb.py` → `stock_market_pe_lg`, `stock_index_pe_lg`,
//!   `stock_market_pb_lg`, `stock_index_pb_lg`: all call `py_mini_racer.MiniRacer()`
//!   (JS `hex` hash) + `get_cookie_csrf()` (BeautifulSoup CSRF) — JS engine / HTML
//!   scraping required.
//! - `stock_industry_pe_ratio_cninfo` (`akshare/stock/stock_industry_pe_cninfo.py`):
//!   requires `py_mini_racer` (`Accept-Enckey` header) — JS signing.
//! - `stock_a_indicator`, `stock_a_pe`, `stock_a_pb`, `stock_zh_val_em`,
//!   `stock_hs_daily`, `stock_zh_a_hist_pre` (daily): **do not exist** as pure-HTTP
//!   functions in this akshare checkout. The A股 PE/PB moved into the JS-based
//!   `stock_a_pe_and_pb.py`; `stock_zh_a_hist_pre` (前复权 daily) was merged into
//!   `stock_zh_a_hist(adjust="qfq")` (already ported in `hist.rs`); the closest
//!   pure-HTTP Eastmoney valuation fn is the ported `stock_value_em` below.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// Eastmoney source bucket (matches `crate::core::client::SOURCE_EASTMONEY`).
const SOURCE_EASTMONEY: &str = "eastmoney";
/// 百度股市通 (Baidu gushitong) source bucket for rate-limiting / error context.
const SOURCE_BAIDU: &str = "baidu";

// ---------------------------------------------------------------------------
// Shared helpers (mirror src/stock/gdfx.rs / src/stock/more.rs)
// ---------------------------------------------------------------------------

/// Read a string field, returning `None` when missing/null.
fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Read a numeric field that may be a JSON number or a plain numeric string.
fn fnum(item: &Value, k: &str) -> Option<f64> {
    match item.get(k) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// Read an integer field (`i64`). Unused in this module (kept per porting brief);
/// prefixed to silence the dead-code lint until a caller needs it.
#[allow(dead_code)]
fn inum(item: &Value, k: &str) -> Option<i64> {
    match item.get(k) {
        Some(Value::Number(n)) => n.as_i64(),
        Some(Value::String(s)) => s.trim().parse::<i64>().ok(),
        _ => None,
    }
}

// ===========================================================================
// stock_zh_valuation_baidu — 百度股市通-A股-财务报表-估值数据
// ===========================================================================

/// One valuation point (date + value) from `stock_zh_valuation_baidu`.
///
/// Baidu returns a nested JSON whose `body` is an array of `[date, value]`
/// pairs. The `value` column is whatever `indicator` selected
/// (总市值 / 市盈率(TTM) / 市盈率(静) / 市净率 / 市现率), kept generic here.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockZhValuationBaiduRow {
    /// `date` 日期 (ISO date string from Baidu)
    pub date: String,
    /// `value` 估值数值 (selected `indicator`)
    pub value: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_zh_valuation_baidu(symbol, indicator, period)`.
///
/// `symbol` is a 6-digit A-share code (e.g. `"002044"`); `indicator` ∈
/// {"总市值","市盈率(TTM)","市盈率(静)","市净率","市现率"}; `period` ∈
/// {"近一年","近三年","近五年","近十年","全部"}. All three are passed straight
/// through to Baidu (akshare does not validate them).
pub async fn stock_zh_valuation_baidu(
    client: &Client,
    symbol: &str,
    indicator: &str,
    period: &str,
) -> Result<Vec<StockZhValuationBaiduRow>> {
    if symbol.is_empty() {
        return Err(Error::InvalidParam(
            "stock_zh_valuation_baidu: symbol must be a non-empty A-share code".into(),
        ));
    }
    let params = [
        ("openapi", "1"),
        ("dspName", "iphone"),
        ("tn", "tangram"),
        ("client", "app"),
        ("query", indicator),
        ("code", symbol),
        ("word", ""),
        ("resource_id", "51171"),
        ("market", "ab"),
        ("tag", indicator),
        ("chart_select", period),
        ("industry_select", ""),
        ("skip_industry", "1"),
        ("finClientType", "pc"),
    ];
    let v = client
        .get_json(
            SOURCE_BAIDU,
            "stock_zh_valuation_baidu",
            "https://gushitong.baidu.com/opendata",
            &params,
        )
        .await?;
    parse_valuation_baidu(&v)
}

/// Parse a Baidu `opendata` response into [`StockZhValuationBaiduRow`]s.
///
/// Navigates `Result[0].DisplayData.resultData.tplData.result.chartInfo[0].body`,
/// each element being a `["date", "value"]` pair.
pub(crate) fn parse_valuation_baidu(resp: &Value) -> Result<Vec<StockZhValuationBaiduRow>> {
    let body = resp
        .get("Result")
        .and_then(|r| r.as_array())
        .and_then(|a| a.first())
        .and_then(|r| r.get("DisplayData"))
        .and_then(|d| d.get("resultData"))
        .and_then(|d| d.get("tplData"))
        .and_then(|d| d.get("result"))
        .and_then(|d| d.get("chartInfo"))
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("body"))
        .and_then(|b| b.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_BAIDU,
            message: "missing Result[0].DisplayData...chartInfo[0].body".into(),
        })?;

    let mut out = Vec::with_capacity(body.len());
    for item in body {
        let pair = item.as_array().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_BAIDU,
            message: "valuation body entry is not a [date, value] array".into(),
        })?;
        if pair.len() < 2 {
            continue;
        }
        let date = pair[0].as_str().unwrap_or_default().to_string();
        let value = match &pair[1] {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.trim().parse::<f64>().ok(),
            _ => None,
        };
        out.push(StockZhValuationBaiduRow {
            date,
            value,
            source: SOURCE_BAIDU,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_zh_valuation_comparison_em — 东方财富-行情中心-同行比较-估值比较
// ===========================================================================

/// One peer valuation-comparison row, port of `stock_zh_valuation_comparison_em`
/// (Eastmoney datacenter `RPT_PCF10_INDUSTRY_CVALUE`).
///
/// Field names are the real upstream keys (akshare renames them positionally via
/// `.rename`, so these are the actual API columns).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockZhValuationComparisonRow {
    /// `PAIMING` 排名
    pub ranking: Option<String>,
    /// `CORRE_SECURITY_CODE` 代码
    pub code: Option<String>,
    /// `CORRE_SECURITY_NAME` 简称
    pub name: Option<String>,
    /// `PEG`
    pub peg: Option<f64>,
    /// `PE_TTM` 市盈率-TTM
    pub pe_ttm: Option<f64>,
    /// `PE_1Y` 市盈率-25E
    pub pe_1y: Option<f64>,
    /// `PE_2Y` 市盈率-26E
    pub pe_2y: Option<f64>,
    /// `PE_3Y` 市盈率-27E
    pub pe_3y: Option<f64>,
    /// `PS` 市销率-24A
    pub ps: Option<f64>,
    /// `PS_TTM` 市销率-TTM
    pub ps_ttm: Option<f64>,
    /// `PS_1Y` 市销率-25E
    pub ps_1y: Option<f64>,
    /// `PS_2Y` 市销率-26E
    pub ps_2y: Option<f64>,
    /// `PS_3Y` 市销率-27E
    pub ps_3y: Option<f64>,
    /// `PB` 市净率-24A
    pub pb: Option<f64>,
    /// `PB_MRQ` 市净率-MRQ
    pub pb_mrq: Option<f64>,
    /// `PCE` 市现率1-24A
    pub pce: Option<f64>,
    /// `PCE_TTM` 市现率1-TTM
    pub pce_ttm: Option<f64>,
    /// `PCF` 市现率2-24A
    pub pcf: Option<f64>,
    /// `PCF_TTM` 市现率2-TTM
    pub pcf_ttm: Option<f64>,
    /// `QYBS` EV/EBITDA-24A
    pub ev_ebitda: Option<f64>,
    /// `REPORT_DATE` 报告日期
    pub report_date: Option<String>,
    /// `SECUCODE` 证券代码
    pub secucode: Option<String>,
    /// `TOTAL_COUNT` 证券数量
    pub total_count: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_zh_valuation_comparison_em(symbol)`.
///
/// `symbol` is a market-prefixed code like `"SZ000895"`; akshare slices
/// `symbol[2:]` (code) and `symbol[:2]` (market) into the `SECUCODE` filter.
pub async fn stock_zh_valuation_comparison_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<StockZhValuationComparisonRow>> {
    if symbol.len() < 2 {
        return Err(Error::InvalidParam(
            "stock_zh_valuation_comparison_em: symbol must be market-prefixed, e.g. \"SZ000895\""
                .into(),
        ));
    }
    let code = &symbol[2..];
    let market = &symbol[..2];
    let filter = format!("(SECUCODE=\"{code}.{market}\")");
    let params = [
        ("reportName", "RPT_PCF10_INDUSTRY_CVALUE"),
        ("columns", "ALL"),
        ("quoteColumns", ""),
        ("filter", filter.as_str()),
        ("pageNumber", ""),
        ("pageSize", ""),
        ("sortTypes", "1"),
        ("sortColumns", "PAIMING"),
        ("source", "HSF10"),
        ("client", "PC"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_zh_valuation_comparison_em",
            "https://datacenter.eastmoney.com/securities/api/data/v1/get",
            &params,
        )
        .await?;
    parse_valuation_comparison(&v)
}

/// Parse an Eastmoney datacenter `result.data` array into
/// [`StockZhValuationComparisonRow`]s. `result: null` / `data: null` → empty.
pub(crate) fn parse_valuation_comparison(
    resp: &Value,
) -> Result<Vec<StockZhValuationComparisonRow>> {
    let data = match resp.get("result").and_then(|r| r.get("data")) {
        Some(Value::Array(a)) => a,
        Some(Value::Null) | None => return Ok(Vec::new()),
        Some(_) => {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "result.data is not an array".into(),
            });
        }
    };
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(StockZhValuationComparisonRow {
            ranking: fstr(item, "PAIMING"),
            code: fstr(item, "CORRE_SECURITY_CODE"),
            name: fstr(item, "CORRE_SECURITY_NAME"),
            peg: fnum(item, "PEG"),
            pe_ttm: fnum(item, "PE_TTM"),
            pe_1y: fnum(item, "PE_1Y"),
            pe_2y: fnum(item, "PE_2Y"),
            pe_3y: fnum(item, "PE_3Y"),
            ps: fnum(item, "PS"),
            ps_ttm: fnum(item, "PS_TTM"),
            ps_1y: fnum(item, "PS_1Y"),
            ps_2y: fnum(item, "PS_2Y"),
            ps_3y: fnum(item, "PS_3Y"),
            pb: fnum(item, "PB"),
            pb_mrq: fnum(item, "PB_MRQ"),
            pce: fnum(item, "PCE"),
            pce_ttm: fnum(item, "PCE_TTM"),
            pcf: fnum(item, "PCF"),
            pcf_ttm: fnum(item, "PCF_TTM"),
            ev_ebitda: fnum(item, "QYBS"),
            report_date: fstr(item, "REPORT_DATE"),
            secucode: fstr(item, "SECUCODE"),
            total_count: fnum(item, "TOTAL_COUNT"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_value_em — 东方财富网-数据中心-估值分析 (每日估值)
// ===========================================================================

/// One daily valuation-analysis row, port of `stock_value_em`
/// (Eastmoney datacenter `RPT_VALUEANALYSIS_DET`).
///
/// Field names are the real upstream keys (akshare renames them positionally via
/// `.rename`, so these are the actual API columns).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockValueEmRow {
    /// `TRADE_DATE` 数据日期
    pub trade_date: Option<String>,
    /// `CLOSE_PRICE` 当日收盘价
    pub close_price: Option<f64>,
    /// `CHANGE_RATE` 当日涨跌幅
    pub change_rate: Option<f64>,
    /// `TOTAL_MARKET_CAP` 总市值
    pub total_market_cap: Option<f64>,
    /// `NOTLIMITED_MARKETCAP_A` 流通市值
    pub float_market_cap: Option<f64>,
    /// `TOTAL_SHARES` 总股本
    pub total_shares: Option<f64>,
    /// `FREE_SHARES_A` 流通股本
    pub float_shares: Option<f64>,
    /// `PE_TTM` PE(TTM)
    pub pe_ttm: Option<f64>,
    /// `PE_LAR` PE(静)
    pub pe_lar: Option<f64>,
    /// `PB_MRQ` 市净率
    pub pb_mrq: Option<f64>,
    /// `PEG_CAR` PEG值
    pub peg: Option<f64>,
    /// `PCF_OCF_TTM` 市现率
    pub pcf_ocf_ttm: Option<f64>,
    /// `PS_TTM` 市销率
    pub ps_ttm: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_value_em(symbol)`.
///
/// `symbol` is a 6-digit A-share code (e.g. `"300766"`), embedded in the
/// `SECURITY_CODE` filter. Returns up to 5000 daily valuation rows, newest
/// first (akshare then sorts ascending; we keep the upstream descending order).
pub async fn stock_value_em(client: &Client, symbol: &str) -> Result<Vec<StockValueEmRow>> {
    if symbol.is_empty() {
        return Err(Error::InvalidParam(
            "stock_value_em: symbol must be a non-empty A-share code".into(),
        ));
    }
    let filter = format!("(SECURITY_CODE=\"{symbol}\")");
    let params = [
        ("sortColumns", "TRADE_DATE"),
        ("sortTypes", "-1"),
        ("pageSize", "5000"),
        ("pageNumber", "1"),
        ("reportName", "RPT_VALUEANALYSIS_DET"),
        ("columns", "ALL"),
        ("quoteColumns", ""),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_value_em",
            "https://datacenter-web.eastmoney.com/api/data/v1/get",
            &params,
        )
        .await?;
    parse_value_em(&v)
}

/// Parse an Eastmoney datacenter `result.data` array into [`StockValueEmRow`]s.
/// `result: null` / `data: null` → empty.
pub(crate) fn parse_value_em(resp: &Value) -> Result<Vec<StockValueEmRow>> {
    let data = match resp.get("result").and_then(|r| r.get("data")) {
        Some(Value::Array(a)) => a,
        Some(Value::Null) | None => return Ok(Vec::new()),
        Some(_) => {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "result.data is not an array".into(),
            });
        }
    };
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(StockValueEmRow {
            trade_date: fstr(item, "TRADE_DATE"),
            close_price: fnum(item, "CLOSE_PRICE"),
            change_rate: fnum(item, "CHANGE_RATE"),
            total_market_cap: fnum(item, "TOTAL_MARKET_CAP"),
            float_market_cap: fnum(item, "NOTLIMITED_MARKETCAP_A"),
            total_shares: fnum(item, "TOTAL_SHARES"),
            float_shares: fnum(item, "FREE_SHARES_A"),
            pe_ttm: fnum(item, "PE_TTM"),
            pe_lar: fnum(item, "PE_LAR"),
            pb_mrq: fnum(item, "PB_MRQ"),
            peg: fnum(item, "PEG_CAR"),
            pcf_ocf_ttm: fnum(item, "PCF_OCF_TTM"),
            ps_ttm: fnum(item, "PS_TTM"),
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
    fn parses_valuation_baidu() {
        let rows = parse_valuation_baidu(&fixture("stock_zh_valuation_baidu.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02");
        assert_eq!(rows[0].value, Some(1234567890.12));
        assert_eq!(rows[0].source, "baidu");
        assert_eq!(rows[1].date, "2024-01-03");
        assert_eq!(rows[1].value, Some(1240000000.00));
    }

    #[test]
    fn parses_valuation_comparison() {
        let rows =
            parse_valuation_comparison(&fixture("stock_zh_valuation_comparison_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].ranking, Some("1/20".to_string()));
        assert_eq!(rows[0].code, Some("000895".to_string()));
        assert_eq!(rows[0].name, Some("双汇发展".to_string()));
        assert_eq!(rows[0].peg, Some(1.23));
        assert_eq!(rows[0].pe_ttm, Some(15.6));
        assert_eq!(rows[0].pe_1y, Some(14.2));
        assert_eq!(rows[0].pe_3y, Some(12.0));
        assert_eq!(rows[0].ps_ttm, Some(1.7));
        assert_eq!(rows[0].pb, Some(4.5));
        assert_eq!(rows[0].pb_mrq, Some(4.4));
        assert_eq!(rows[0].pce_ttm, Some(10.9));
        assert_eq!(rows[0].pcf_ttm, Some(12.0));
        assert_eq!(rows[0].ev_ebitda, Some(9.8));
        assert_eq!(rows[0].report_date, Some("2024-09-30".to_string()));
        assert_eq!(rows[0].secucode, Some("000895.SZ".to_string()));
        assert_eq!(rows[0].total_count, Some(20.0));
        assert_eq!(rows[0].source, "eastmoney");

        assert_eq!(rows[1].code, Some("603288".to_string()));
        assert_eq!(rows[1].name, Some("海天味业".to_string()));
        assert_eq!(rows[1].pe_ttm, Some(30.1));
        assert_eq!(rows[1].total_count, Some(20.0));
    }

    #[test]
    fn parses_value_em() {
        let rows = parse_value_em(&fixture("stock_value_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].trade_date, Some("2024-09-30T00:00:00".to_string()));
        assert_eq!(rows[0].close_price, Some(25.30));
        assert_eq!(rows[0].change_rate, Some(-1.20));
        assert_eq!(rows[0].total_market_cap, Some(1234567890.12));
        assert_eq!(rows[0].float_market_cap, Some(1200000000.00));
        assert_eq!(rows[0].total_shares, Some(500000000.0));
        assert_eq!(rows[0].float_shares, Some(480000000.0));
        assert_eq!(rows[0].pe_ttm, Some(30.5));
        assert_eq!(rows[0].pe_lar, Some(32.1));
        assert_eq!(rows[0].pb_mrq, Some(5.2));
        assert_eq!(rows[0].peg, Some(1.05));
        assert_eq!(rows[0].pcf_ocf_ttm, Some(22.3));
        assert_eq!(rows[0].ps_ttm, Some(6.1));
        assert_eq!(rows[0].source, "eastmoney");

        assert_eq!(rows[1].trade_date, Some("2024-09-27T00:00:00".to_string()));
        assert_eq!(rows[1].close_price, Some(25.60));
        assert_eq!(rows[1].pe_ttm, Some(30.9));
    }

    #[test]
    fn value_em_null_data_is_empty() {
        let v = serde_json::json!({ "result": null });
        let rows = parse_value_em(&v).unwrap();
        assert!(rows.is_empty());
    }
}
