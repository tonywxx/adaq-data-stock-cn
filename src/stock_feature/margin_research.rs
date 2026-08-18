//! `stock_feature` **融资融券 (margin/short)** endpoints.
//!
//! Port of akshare `stock_margin_em.py`. Currently one tractable endpoint is
//! implemented; the remaining `stock_feature` margin/研报 families that need
//! HTML scraping or a `hexin-v` JS signature are tracked but left DEFERRED
//! (see `docs/MAPPING.md`).
//!
//! | akshare fn                  | source                                   | status   |
//! |-----------------------------|------------------------------------------|----------|
//! | `stock_margin_account_info` | Eastmoney `datacenter-web` `RPTA_WEB_MARGIN_DAILYTRADE` | DONE |
//!
//! `RPTA_WEB_MARGIN_DAILYTRADE` is a paginated `result.pages` / `result.data`
//! datacenter report. Real captured fixture carries `pages` for `pageSize=5`;
//! the live function requests `pageSize=500` (≈7 pages for the full history)
//! and concatenates every page.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};
use crate::core::json::*;

const BASE: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";
const REPORT: &str = "RPTA_WEB_MARGIN_DAILYTRADE";

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

/// Normalize an Eastmoney datetime `"YYYY-MM-DD HH:MM:SS"` (or `"YYYY-MM-DD"`)
/// to a plain `YYYY-MM-DD` date. `None` when null/empty.
fn norm_date(v: Option<&Value>) -> Option<String> {
    let s = match v {
        Some(Value::String(s)) => s.trim().to_string(),
        _ => return None,
    };
    if s.is_empty() {
        return None;
    }
    Some(s[..10].to_string())
}

fn dc_data(resp: &Value) -> Result<Vec<Value>> {
    resp.get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .cloned()
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data at stock_margin_account_info".into(),
        })
}

fn dc_pages(resp: &Value) -> usize {
    resp.get("result")
        .and_then(|r| r.get("pages"))
        .and_then(|p| p.as_i64())
        .unwrap_or(1)
        .max(1) as usize
}

// ---------------------------------------------------------------------------
// stock_margin_account_info — 两融账户信息
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct MarginAccountRow {
    /// `STATISTICS_DATE` 日期 (YYYY-MM-DD)
    pub date: String,
    /// `FIN_BALANCE` 融资余额 (亿元)
    pub fin_balance: Option<f64>,
    /// `LOAN_BALANCE` 融券余额 (亿元)
    pub loan_balance: Option<f64>,
    /// `FIN_BUY_AMT` 融资买入额 (亿元)
    pub fin_buy_amt: Option<f64>,
    /// `LOAN_SELL_AMT` 融券卖出额 (亿元)
    pub loan_sell_amt: Option<f64>,
    /// `SECURITY_ORG_NUM` 证券公司数量
    pub security_org_num: Option<f64>,
    /// `OPERATEDEPT_NUM` 营业部数量
    pub operate_dept_num: Option<f64>,
    /// `PERSONAL_INVESTOR_NUM` 个人投资者数量 (万户)
    pub personal_investor_num: Option<f64>,
    /// `ORG_INVESTOR_NUM` 机构投资者数量
    pub org_investor_num: Option<f64>,
    /// `INVESTOR_NUM` 参与交易的投资者数量
    pub investor_num: Option<f64>,
    /// `MARGINLIAB_INVESTOR_NUM` 有融资融券负债的投资者数量
    pub marginliab_investor_num: Option<f64>,
    /// `TOTAL_GUARANTEE` 担保物总价值 (亿元)
    pub total_guarantee: Option<f64>,
    /// `AVG_GUARANTEE_RATIO` 平均维持担保比例 (%)
    pub avg_guarantee_ratio: Option<f64>,
}

pub async fn stock_margin_account_info(client: &Client) -> Result<Vec<MarginAccountRow>> {
    let base: Vec<(&str, &str)> = vec![
        ("reportName", REPORT),
        ("columns", "ALL"),
        ("sortColumns", "STATISTICS_DATE"),
        ("sortTypes", "-1"),
        ("p", "1"),
        ("pageNo", "1"),
        ("pageNum", "1"),
    ];
    let first = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_margin_account_info",
            BASE,
            &base,
        )
        .await?;
    let pages = dc_pages(&first).min(200);
    let mut rows = parse_margin_account(&first)?;
    for page in 2..=pages {
        let mut p = base.clone();
        let pn = page.to_string();
        p.push(("pageNumber", pn.as_str()));
        p.push(("pageSize", "500"));
        let v = client
            .get_json(SOURCE_EASTMONEY, "stock_margin_account_info", BASE, &p)
            .await?;
        rows.extend(parse_margin_account(&v)?);
    }
    Ok(rows)
}

pub(crate) fn parse_margin_account(resp: &Value) -> Result<Vec<MarginAccountRow>> {
    let mut out = Vec::new();
    for item in dc_data(resp)? {
        let date = match norm_date(item.get("STATISTICS_DATE")) {
            Some(d) => d,
            None => continue,
        };
        out.push(MarginAccountRow {
            date,
            fin_balance: opt_f64(&item, "FIN_BALANCE"),
            loan_balance: opt_f64(&item, "LOAN_BALANCE"),
            fin_buy_amt: opt_f64(&item, "FIN_BUY_AMT"),
            loan_sell_amt: opt_f64(&item, "LOAN_SELL_AMT"),
            security_org_num: opt_f64(&item, "SECURITY_ORG_NUM"),
            operate_dept_num: opt_f64(&item, "OPERATEDEPT_NUM"),
            personal_investor_num: opt_f64(&item, "PERSONAL_INVESTOR_NUM"),
            org_investor_num: opt_f64(&item, "ORG_INVESTOR_NUM"),
            investor_num: opt_f64(&item, "INVESTOR_NUM"),
            marginliab_investor_num: opt_f64(&item, "MARGINLIAB_INVESTOR_NUM"),
            total_guarantee: opt_f64(&item, "TOTAL_GUARANTEE"),
            avg_guarantee_ratio: opt_f64(&item, "AVG_GUARANTEE_RATIO"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// tests
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

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    #[test]
    fn parses_margin_account() {
        let rows = parse_margin_account(&fixture("stock_margin_account_info.json")).unwrap();
        assert!(!rows.is_empty());
        let r = &rows[0];
        assert_eq!(r.date, "2026-08-13");
        assert!(approx(r.fin_balance, 26497.25100007));
        assert!(approx(r.loan_balance, 259.90134799));
        assert!(approx(r.fin_buy_amt, 2421.10328871));
        assert!(approx(r.loan_sell_amt, 12.05715155));
        assert_eq!(r.security_org_num, Some(97.0));
        assert_eq!(r.operate_dept_num, Some(11666.0));
        assert!(approx(r.total_guarantee, 86956.33058799));
        assert!(approx(r.avg_guarantee_ratio, 281.5776));
    }
}
