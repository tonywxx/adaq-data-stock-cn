//! 申万宏源研究-基金指数实时/历史行情 (akshare `index/index_research_fund_sw.py`).
//!
//! Both endpoints POST a **JSON request body** (not form-encoded), so they use
//! [`Client::post_json`] rather than `post_form_json`. They were previously
//! DEFERRED in `src/index/stock_hk_us_zh.rs` before `post_json` existed; they
//! now port cleanly.
//!
//! | Rust function | akshare source | note |
//! !---|---|---|
//! | `index_realtime_fund_sw` | `index_research_fund_sw.py:15` | POST JSON `fundIndex/pageList`; `data.list` |
//! | `index_hist_fund_sw` | `index_research_fund_sw.py:61` | POST JSON `getFundKChartData`; `data` array |

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_SWSRESEARCH: &str = "swsresearch";

const FUND_SW_REALTIME_URL: &str =
    "https://www.swsresearch.com/insWechatSw/fundIndex/pageList";
const FUND_SW_HIST_URL: &str =
    "https://www.swsresearch.com/insWechatSw/fundIndex/getFundKChartData";

// ---------------------------------------------------------------------------
// shared row + parse cores
// ---------------------------------------------------------------------------

/// One 申万基金指数 realtime row (`index_realtime_fund_sw`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundSwRealtimeRow {
    /// 指数代码 (swIndexCode)
    pub index_code: String,
    /// 指数名称 (swIndexName)
    pub index_name: String,
    /// 昨收盘 (lastCloseIndex)
    pub last_close: Option<f64>,
    /// 日涨跌幅 (lastMarkup)
    pub daily_change_pct: Option<f64>,
    /// 年涨跌幅 (yearMarkup)
    pub year_change_pct: Option<f64>,
}

/// One 申万基金指数 history row (`index_hist_fund_sw`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundSwHistRow {
    /// 日期 (bargaindate)
    pub date: Option<String>,
    /// 收盘指数 (closeindex)
    pub close_index: Option<f64>,
    /// 开盘指数 (openindex)
    pub open_index: Option<f64>,
    /// 最高指数 (maxindex)
    pub max_index: Option<f64>,
    /// 最低指数 (minindex)
    pub min_index: Option<f64>,
    /// 涨跌幅 (markup)
    pub markup: Option<f64>,
}

/// Parse `index_realtime_fund_sw` rows from the already-fetched JSON value.
pub(crate) fn parse_index_realtime_fund_sw(resp: &Value) -> Result<Vec<FundSwRealtimeRow>> {
    let list = resp
        .get("data")
        .and_then(|d| d.get("list"))
        .and_then(|l| l.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SWSRESEARCH,
            message: "missing data.list".into(),
        })?;
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        out.push(FundSwRealtimeRow {
            index_code: fstr(item, "swIndexCode").unwrap_or_default(),
            index_name: fstr(item, "swIndexName").unwrap_or_default(),
            last_close: fnum(item, "lastCloseIndex"),
            daily_change_pct: fnum(item, "lastMarkup"),
            year_change_pct: fnum(item, "yearMarkup"),
        });
    }
    Ok(out)
}

/// Parse `index_hist_fund_sw` rows from the already-fetched JSON value.
pub(crate) fn parse_index_hist_fund_sw(resp: &Value) -> Result<Vec<FundSwHistRow>> {
    let arr = resp
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SWSRESEARCH,
            message: "missing data array".into(),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(FundSwHistRow {
            date: fstr(item, "bargaindate"),
            close_index: fnum(item, "closeindex"),
            open_index: fnum(item, "openindex"),
            max_index: fnum(item, "maxindex"),
            min_index: fnum(item, "minindex"),
            markup: fnum(item, "markup"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// public functions
// ---------------------------------------------------------------------------

/// 申万宏源研究-基金指数-实时行情 (akshare `index_realtime_fund_sw`).
///
/// `symbol` is one of `{"基础一级","基础二级","基础三级","特色指数"}` and is sent
/// verbatim as the `indexTypeName` request field.
pub async fn index_realtime_fund_sw(client: &Client, symbol: &str) -> Result<Vec<FundSwRealtimeRow>> {
    let body = serde_json::json!({
        "pageNo": 1,
        "pageSize": 50,
        "indexTypeName": symbol,
        "sortField": "",
        "rule": "",
        "indexType": 1,
    });
    let v = client
        .post_json(
            SOURCE_SWSRESEARCH,
            "index_realtime_fund_sw",
            FUND_SW_REALTIME_URL,
            &body,
            None,
        )
        .await?;
    parse_index_realtime_fund_sw(&v)
}

/// 申万宏源研究-基金指数-历史行情 (akshare `index_hist_fund_sw`).
///
/// `period` is one of `{"day","week","month"}` (mapped to the upstream
/// `DAY`/`WEEK`/`MONTH` `type`); `symbol` is the fund-index code (e.g. `807200`).
pub async fn index_hist_fund_sw(
    client: &Client,
    symbol: &str,
    period: &str,
) -> Result<Vec<FundSwHistRow>> {
    let type_code = match period {
        "day" => "DAY",
        "week" => "WEEK",
        "month" => "MONTH",
        other => {
            return Err(Error::InvalidParam(format!(
                "unknown period: {other} (expected day/week/month)"
            )))
        }
    };
    let body = serde_json::json!({
        "swIndexCode": symbol,
        "type": type_code,
    });
    let v = client
        .post_json(
            SOURCE_SWSRESEARCH,
            "index_hist_fund_sw",
            FUND_SW_HIST_URL,
            &body,
            None,
        )
        .await?;
    parse_index_hist_fund_sw(&v)
}

// ---------------------------------------------------------------------------
// private helpers (verbatim per task instructions)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(str::to_string)
}

fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

// ---------------------------------------------------------------------------
// offline parse tests
// ---------------------------------------------------------------------------

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
    fn test_parse_index_realtime_fund_sw() {
        let rows = parse_index_realtime_fund_sw(&fixture("index_realtime_fund_sw.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].index_code, "807100");
        assert_eq!(rows[0].index_name, "申万指数");
        assert_eq!(rows[0].last_close, Some(3456.78));
        assert_eq!(rows[0].daily_change_pct, Some(-0.0123));
        assert_eq!(rows[0].year_change_pct, Some(0.0456));
        assert_eq!(rows[1].index_code, "807200");
        assert_eq!(rows[1].last_close, Some(1234.56));
    }

    #[test]
    fn test_parse_index_hist_fund_sw() {
        let rows = parse_index_hist_fund_sw(&fixture("index_hist_fund_sw.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date.as_deref(), Some("2024-01-02"));
        assert_eq!(rows[0].close_index, Some(3456.78));
        assert_eq!(rows[0].open_index, Some(3440.0));
        assert_eq!(rows[0].max_index, Some(3470.12));
        assert_eq!(rows[0].min_index, Some(3430.5));
        assert_eq!(rows[0].markup, Some(0.0049));
        assert_eq!(rows[1].close_index, Some(3460.0));
    }

    #[test]
    fn test_index_hist_fund_sw_bad_period() {
        // Build a no-op client path by calling the period mapping via the public
        // signature is async; instead assert the InvalidParam branch directly.
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // A fake client is unnecessary: we only validate `period` before any
            // network call, so construct a Client and expect an error shape.
            let client = Client::new();
            let err = index_hist_fund_sw(&client, "807200", "quarter").await;
            assert!(matches!(err, Err(Error::InvalidParam(_))));
        });
    }
}
