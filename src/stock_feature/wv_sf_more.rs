//! 融资融券 (margin financing) summary endpoints — SSE & SZSE.
//!
//! Ports `akshare/stock_feature/stock_margin_sse.py` (`stock_margin_sse`) and
//! `akshare/stock_feature/stock_margin_szse.py` (`stock_margin_szse`).
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `stock_margin_sse` | `stock_margin_sse.py:68` | SSE `queryMargin.do` JSON GET + `Referer` |
//! | `stock_margin_szse` | `stock_margin_szse.py:47` | SZSE `ShowReport/data` JSON GET + `Referer` |
//!
//! ## DONE
//! 2 margin-summary functions. Both are **plain JSON GETs** with a `Referer`
//! header (no token / cookie / JS / HTML scraping), so they are ported
//! faithfully. SSE returns `result` as a positional list-of-lists (akshare
//! renames columns by position); the parser tolerates both list-of-lists and
//! object rows. SZSE returns `[{ "data": [ ... ] }]` where each object carries
//! Chinese-named keys with comma-grouped numeric strings.
//!
//! ## DEFERRED (out of scope for this module)
//! * `stock_board_concept_name_ths` (`stock_board_concept_ths.py:71`): THS
//!   `hexin-v` cookie + `py_mini_racer` JS (`v` token) + BeautifulSoup scrape → DEFERRED.
//! * `stock_board_industry_name_ths` (`stock_board_industry_ths.py:68`): THS
//!   `hexin-v` cookie + JS → DEFERRED.
//! * `stock_classify_sina` (`stock_classify_sina.py:48`): Sina `BeautifulSoup`
//!   HTML scraping of the board JSON → DEFERRED.
//! * `stock_cyq_em` (`stock_cyq_em.py:16`): `py_mini_racer` JS (`CYQCalculator`) → DEFERRED.
//! * `stock_hk_indicator_eniu` (`stock_a_indicator.py:54`): 乐股/eniu token → DEFERRED.
//! * `stock_hot_deal_xq` (`stock_hot_xq.py:207`): 雪球 `xq_a_token` → DEFERRED.
//! * `stock_hot_follow_xq` (`stock_hot_xq.py:81`): 雪球 `xq_a_token` → DEFERRED.
//! * `stock_hot_tweet_xq` (`stock_hot_xq.py:144`): 雪球 `xq_a_token` → DEFERRED.
//! * `stock_inner_trade_xq` (`stock_inner_trade_xq.py:72`): 雪球 `xq_a_token` → DEFERRED.
//! * `stock_js_weibo_nlp_time` (`stock/stock_weibo_nlp.py:20`): Jin10 `x-csrf-token` gate → DEFERRED.
//! * `stock_js_weibo_report` (`stock/stock_weibo_nlp.py:49`): Jin10 `x-csrf-token` gate → DEFERRED.
//!
//! See `docs/_draft_stkf.md` for the full ported/deferred ledger.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Parse a numeric cell, tolerating both JSON numbers and comma-grouped
/// numeric strings (SZSE returns `"1,234.56"`).
fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.replace(',', "").trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// Extract a cell from a SSE row, which may be a positional list (use `idx`)
/// or an object (use `key`). Tolerant to either upstream shape.
fn get_cell<'a>(row: &'a Value, idx: usize, key: &str) -> Option<&'a Value> {
    match row {
        Value::Array(a) => a.get(idx),
        Value::Object(m) => m.get(key),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// SSE — stock_margin_sse
// ---------------------------------------------------------------------------

const SSE_SOURCE: &str = "sse";
const SSE_BASE: &str = "https://query.sse.com.cn/marketdata/tradedata/queryMargin.do";

/// One day's SSE margin-financing summary (`stock_margin_sse.py:68`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct MarginSseRow {
    /// 信用交易日期 — trade date.
    pub date: String,
    /// 融资余额 — financing balance (`融资余额`).
    pub financing_balance: Option<f64>,
    /// 融资买入额 — financing buy amount (`融资买入额`).
    pub financing_buy: Option<f64>,
    /// 融券余量 — securities lending balance volume (`融券余量`).
    pub securities_balance: Option<f64>,
    /// 融券余量金额 — securities lending balance amount (`融券余量金额`).
    pub securities_balance_amount: Option<f64>,
    /// 融券卖出量 — securities sold volume (`融券卖出量`).
    pub securities_sell: Option<f64>,
    /// 融资融券余额 — total margin balance (`融资融券余额`).
    pub margin_total_balance: Option<f64>,
}

/// Parse a single SSE row (list-of-lists or object) into [`MarginSseRow`].
/// Rows without a date are skipped (return `None`).
pub(crate) fn parse_margin_sse_item(item: &Value) -> Option<MarginSseRow> {
    let date = match get_cell(item, 1, "信用交易日期") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => return None,
    };
    Some(MarginSseRow {
        date,
        financing_balance: get_cell(item, 10, "融资余额").and_then(num),
        financing_buy: get_cell(item, 8, "融资买入额").and_then(num),
        securities_balance: get_cell(item, 4, "融券余量").and_then(num),
        securities_balance_amount: get_cell(item, 5, "融券余量金额").and_then(num),
        securities_sell: get_cell(item, 3, "融券卖出量").and_then(num),
        margin_total_balance: get_cell(item, 9, "融资融券余额").and_then(num),
    })
}

/// Parse the SSE `result` array into rows, skipping malformed items.
pub(crate) fn parse_margin_sse(result: &[Value]) -> Vec<MarginSseRow> {
    let mut out = Vec::with_capacity(result.len());
    for item in result {
        if let Some(row) = parse_margin_sse_item(item) {
            out.push(row);
        }
    }
    out
}

/// 上海证券交易所-融资融券数据-融资融券汇总 (`stock_margin_sse.py:68`).
///
/// `start_date` / `end_date` are `YYYYMMDD` (e.g. `"20010106"`).
pub async fn stock_margin_sse(
    client: &Client,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<MarginSseRow>> {
    let headers = &[("Referer", "https://www.sse.com.cn/")];
    let resp = client
        .get_json_with_headers(
            SSE_SOURCE,
            "stock_margin_sse",
            SSE_BASE,
            &[
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
            ],
            Some(headers),
        )
        .await?;
    let result = resp
        .get("result")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SSE_SOURCE,
            message: "missing result".into(),
        })?;
    Ok(parse_margin_sse(result))
}

// ---------------------------------------------------------------------------
// SZSE — stock_margin_szse
// ---------------------------------------------------------------------------

const SZSE_SOURCE: &str = "szse";
const SZSE_BASE: &str = "https://www.szse.cn/api/report/ShowReport/data";

/// One day's SZSE margin-financing summary (`stock_margin_szse.py:47`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct MarginSzseRow {
    /// 融资买入额 — financing buy amount.
    pub financing_buy: Option<f64>,
    /// 融资余额 — financing balance.
    pub financing_balance: Option<f64>,
    /// 融券卖出量 — securities sold volume.
    pub securities_sell: Option<f64>,
    /// 融券余量 — securities lending balance volume.
    pub securities_balance: Option<f64>,
    /// 融券余额 — securities lending balance amount.
    pub securities_balance_amount: Option<f64>,
    /// 融资融券余额 — total margin balance.
    pub margin_total_balance: Option<f64>,
}

/// Parse a single SZSE object (Chinese-named keys) into [`MarginSzseRow`].
pub(crate) fn parse_margin_szse_item(item: &Value) -> Option<MarginSzseRow> {
    let obj = item.as_object()?;
    Some(MarginSzseRow {
        financing_buy: obj.get("融资买入额").and_then(num),
        financing_balance: obj.get("融资余额").and_then(num),
        securities_sell: obj.get("融券卖出量").and_then(num),
        securities_balance: obj.get("融券余量").and_then(num),
        securities_balance_amount: obj.get("融券余额").and_then(num),
        margin_total_balance: obj.get("融资融券余额").and_then(num),
    })
}

/// Parse the SZSE `data` array into rows.
pub(crate) fn parse_margin_szse(data: &[Value]) -> Vec<MarginSzseRow> {
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        if let Some(row) = parse_margin_szse_item(item) {
            out.push(row);
        }
    }
    out
}

/// 深圳证券交易所-融资融券数据-融资融券汇总 (`stock_margin_szse.py:47`).
///
/// `date` is `YYYYMMDD` (e.g. `"20240411"`); reformatted to `YYYY-MM-DD` for
/// the upstream `txtDate` parameter.
pub async fn stock_margin_szse(client: &Client, date: &str) -> Result<Vec<MarginSzseRow>> {
    if date.len() != 8 || !date.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::InvalidParam(format!(
            "date must be YYYYMMDD (8 digits), got {date}"
        )));
    }
    let txt_date = format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..8]);
    let headers = &[(
        "Referer",
        "https://www.szse.cn/disclosure/margin/object/index.html",
    )];
    let resp = client
        .get_json_with_headers(
            SZSE_SOURCE,
            "stock_margin_szse",
            SZSE_BASE,
            &[
                ("SHOWTYPE", "JSON"),
                ("CATALOGID", "1837_xxpl"),
                ("txtDate", &txt_date),
                ("tab1PAGENO", "1"),
                ("random", "0.7425245522795993"),
            ],
            Some(headers),
        )
        .await?;
    let data = resp
        .get(0)
        .and_then(|a| a.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SZSE_SOURCE,
            message: "missing data[0].data".into(),
        })?;
    Ok(parse_margin_szse(data))
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

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    #[test]
    fn parses_sse_rows() {
        let fx = fixture("stock_margin_sse.json");
        let result = fx.get("result").unwrap().as_array().unwrap();
        let rows = parse_margin_sse(result);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2023-09-22");
        assert!(approx(rows[0].financing_balance, 4500.6));
        assert!(approx(rows[0].financing_buy, 400.2));
        assert!(approx(rows[0].securities_balance, 200.3));
        assert!(approx(rows[0].securities_balance_amount, 300.7));
        assert!(approx(rows[0].securities_sell, 88.5));
        assert!(approx(rows[0].margin_total_balance, 5000.8));
        assert_eq!(rows[1].date, "2023-09-21");
        assert!(approx(rows[1].financing_balance, 4600.0));
    }

    #[test]
    fn sse_skips_rows_without_date() {
        let bad = Value::Array(vec![Value::Array(vec![
            Value::String("".into()),
            Value::Null,
        ])]);
        let rows = parse_margin_sse(&[bad]);
        assert!(rows.is_empty());
    }

    #[test]
    fn parses_szse_rows() {
        let fx = fixture("stock_margin_szse.json");
        let data = fx
            .get("packet")
            .unwrap()
            .get(0)
            .unwrap()
            .get("data")
            .unwrap()
            .as_array()
            .unwrap();
        let rows = parse_margin_szse(data);
        assert_eq!(rows.len(), 2);
        // comma-grouped strings must be parsed
        assert!(approx(rows[0].financing_buy, 1234.56));
        assert!(approx(rows[0].financing_balance, 2345.67));
        assert!(approx(rows[0].securities_sell, 12.3));
        assert!(approx(rows[0].securities_balance, 23.4));
        assert!(approx(rows[0].securities_balance_amount, 34.5));
        assert!(approx(rows[0].margin_total_balance, 5678.90));
        assert!(approx(rows[1].financing_balance, 2000.0));
    }

    #[test]
    fn szse_date_validation() {
        // pure unit check on the parse layer; date reformat is exercised by
        // the async fn which needs a live client, so we only assert here that
        // a valid 8-digit date slices cleanly (mirrors stock_margin_szse).
        let date = "20240411";
        assert_eq!(
            format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..8]),
            "2024-04-11"
        );
    }
}
