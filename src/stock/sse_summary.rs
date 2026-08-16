//! 上海证券交易所 (SSE) 市场总貌与每日成交概况。Ports `akshare/stock/stock_summary.py`.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `stock_sse_summary` | `stock_summary.py:207` | JSON via `query.sse.com.cn/commonQuery.do`, needs `Referer` header |
//! | `stock_sse_deal_daily` | `stock_summary.py:251` | JSON via `query.sse.com.cn/commonQuery.do`, needs `Referer` header |
//!
//! ## DEFERRED
//! None — both endpoints return JSON (`r.json()`) and only require the SSE
//! `Referer` header, so they are implemented as pure-JSON functions.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE: &str = "sse";
const BASE: &str = "https://query.sse.com.cn/commonQuery.do";
const REFERER: &[(&str, &str)] = &[("Referer", "https://www.sse.com.cn/")];

/// Best-effort string extraction from a JSON value (string or number).
fn field(obj: &Value, key: &str) -> String {
    match obj.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
        None => String::new(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// stock_sse_summary
// ─────────────────────────────────────────────────────────────────────────────

/// Row of `stock_sse_summary`: one market metric across 股票/主板/科创板.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SseSummaryRow {
    /// 项目 (metric Chinese name)
    pub project: String,
    /// 股票
    pub stock: String,
    /// 主板
    pub main_board: String,
    /// 科创板
    pub sci_tech_board: String,
}

pub(crate) fn parse_sse_summary(resp: &Value) -> Result<Vec<SseSummaryRow>> {
    let result = resp
        .get("result")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing result array".into(),
        })?;

    let mut by_product: std::collections::HashMap<String, &Value> = std::collections::HashMap::new();
    for item in result {
        let name = item
            .get("PRODUCT_NAME")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        by_product.insert(name, item);
    }

    let pick = |product: &str, key: &str| -> String {
        by_product
            .get(product)
            .map(|v| field(v, key))
            .unwrap_or_default()
    };

    // metric key -> Chinese name, in akshare output order (TOTAL_TRADE_AMT and
    // PRODUCT_NAME are dropped, matching the Python implementation).
    let metrics: &[(&str, &str)] = &[
        ("NEGO_ISSUE_VOL", "流通股本"),
        ("TOTAL_VALUE", "总市值"),
        ("AVG_PE_RATIO", "平均市盈率"),
        ("LIST_COM_NUM", "上市公司"),
        ("SECURITY_NUM", "上市股票"),
        ("NEGO_VALUE", "流通市值"),
        ("TRADE_DATE", "报告时间"),
        ("TOTAL_ISSUE_VOL", "总股本"),
    ];

    let mut out = Vec::with_capacity(metrics.len());
    for (key, name) in metrics {
        out.push(SseSummaryRow {
            project: name.to_string(),
            stock: pick("股票", key),
            main_board: pick("主板", key),
            sci_tech_board: pick("科创板", key),
        });
    }
    Ok(out)
}

/// 上海证券交易所-总貌。
pub async fn stock_sse_summary(client: &Client) -> Result<Vec<SseSummaryRow>> {
    let v = client
        .get_json_with_headers(
            SOURCE,
            "stock_sse_summary",
            BASE,
            &[
                ("sqlId", "COMMON_SSE_SJ_GPSJ_GPSJZM_TJSJ_L"),
                ("PRODUCT_NAME", "股票,主板,科创板"),
                ("type", "inParams"),
            ],
            Some(REFERER),
        )
        .await?;
    parse_sse_summary(&v)
}

// ─────────────────────────────────────────────────────────────────────────────
// stock_sse_deal_daily
// ─────────────────────────────────────────────────────────────────────────────

/// Row of `stock_sse_deal_daily`: one daily metric across the SSE product boards.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SseDealDailyRow {
    /// 单日情况 (metric Chinese name)
    pub item: String,
    /// 股票 (PRODUCT_CODE 17)
    pub stock: String,
    /// 主板A (PRODUCT_CODE 01)
    pub main_board_a: String,
    /// 主板B (PRODUCT_CODE 02)
    pub main_board_b: String,
    /// 科创板 (PRODUCT_CODE 03)
    pub sci_tech_board: String,
    /// 股票回购 (PRODUCT_CODE 11)
    pub stock_repurchase: String,
}

pub(crate) fn parse_sse_deal_daily(resp: &Value) -> Result<Vec<SseDealDailyRow>> {
    let result = resp
        .get("result")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing result array".into(),
        })?;

    let mut by_code: std::collections::HashMap<String, &Value> = std::collections::HashMap::new();
    for item in result {
        let code = item
            .get("PRODUCT_CODE")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        by_code.insert(code, item);
    }

    let pick = |code: &str, key: &str| -> String {
        by_code.get(code).map(|v| field(v, key)).unwrap_or_default()
    };

    // metric key -> Chinese name, in akshare desired output order
    // (TRADE_NUM, TRADE_DATE, PRODUCT_CODE rows are dropped).
    let metrics: &[(&str, &str)] = &[
        ("LIST_NUM", "挂牌数"),
        ("TOTAL_VALUE", "市价总值"),
        ("NEGO_VALUE", "流通市值"),
        ("TRADE_AMT", "成交金额"),
        ("TRADE_VOL", "成交量"),
        ("AVG_PE_RATE", "平均市盈率"),
        ("TOTAL_TO_RATE", "换手率"),
        ("NEGO_TO_RATE", "流通换手率"),
    ];

    let mut out = Vec::with_capacity(metrics.len());
    for (key, name) in metrics {
        out.push(SseDealDailyRow {
            item: name.to_string(),
            stock: pick("17", key),
            main_board_a: pick("01", key),
            main_board_b: pick("02", key),
            sci_tech_board: pick("03", key),
            stock_repurchase: pick("11", key),
        });
    }
    Ok(out)
}

/// 上海证券交易所-每日股票成交概况. `date` format `YYYYMMDD`.
pub async fn stock_sse_deal_daily(client: &Client, date: &str) -> Result<Vec<SseDealDailyRow>> {
    let search_date = format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..8]);
    let v = client
        .get_json_with_headers(
            SOURCE,
            "stock_sse_deal_daily",
            BASE,
            &[
                ("sqlId", "COMMON_SSE_SJ_GPSJ_CJGK_MRGK_C"),
                ("PRODUCT_CODE", "01,02,03,11,17"),
                ("type", "inParams"),
                ("SEARCH_DATE", &search_date),
            ],
            Some(REFERER),
        )
        .await?;
    parse_sse_deal_daily(&v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Value {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    #[test]
    fn parse_sse_summary_ok() {
        let rows = parse_sse_summary(&fixture("stock_sse_summary.json")).unwrap();
        assert_eq!(rows.len(), 8);
        assert_eq!(rows[0].project, "流通股本");
        // 流通股本 / 股票 = NEGO_ISSUE_VOL of 股票 product
        assert_eq!(rows[0].stock, "48603.54");
        assert_eq!(rows[0].main_board, "46390.1");
        assert_eq!(rows[0].sci_tech_board, "2213.44");
        // 报告时间 row carries the trade date string
        let report = rows.iter().find(|r| r.project == "报告时间").unwrap();
        assert_eq!(report.stock, "20260814");
        // 平均市盈率 / 股票 = AVG_PE_RATIO of 股票 product
        let pe = rows.iter().find(|r| r.project == "平均市盈率").unwrap();
        assert_eq!(pe.stock, "17.16");
        // 总股本 / 股票 = TOTAL_ISSUE_VOL of 股票 product
        let zgb = rows.iter().find(|r| r.project == "总股本").unwrap();
        assert_eq!(zgb.stock, "51770.96");
        assert_eq!(zgb.main_board, "48552.37");
        assert_eq!(zgb.sci_tech_board, "3218.59");
    }

    #[test]
    fn parse_sse_deal_daily_ok() {
        let rows = parse_sse_deal_daily(&fixture("stock_sse_deal_daily.json")).unwrap();
        assert_eq!(rows.len(), 8);
        assert_eq!(rows[0].item, "挂牌数");
        // 挂牌数 / 股票 (code 17) = LIST_NUM
        assert_eq!(rows[0].stock, "2316");
        assert_eq!(rows[0].main_board_a, "1692");
        assert_eq!(rows[0].main_board_b, "43");
        assert_eq!(rows[0].sci_tech_board, "581");
        assert_eq!(rows[0].stock_repurchase, "0");
        // 市价总值 / 股票 (code 17) = TOTAL_VALUE
        let mv = rows.iter().find(|r| r.item == "市价总值").unwrap();
        assert_eq!(mv.stock, "529411.2");
        // 平均市盈率 for 股票回购 (code 11) = "-" placeholder
        let pe = rows.iter().find(|r| r.item == "平均市盈率").unwrap();
        assert_eq!(pe.stock_repurchase, "-");
    }
}
