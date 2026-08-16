//! 沪港通/深港通 港股通 参考汇率与结算汇率 (SSE `commonSoaQuery.do`), plus
//! DEFERRED records for two assigned functions that cannot be implemented under
//! the porting rules.
//!
//! | Rust fn | akshare source | endpoint |
//! |---|---|---|
//! | `stock_sgt_reference_exchange_rate_sse` | `stock_hsgt_exchange_rate.py:76` | `query.sse.com.cn` `FW_HGT_GGTHL` |
//! | `stock_sgt_settlement_exchange_rate_sse` | `stock_hsgt_exchange_rate.py:134` | `query.sse.com.cn` `FW_HGT_JSHDBL` |
//!
//! The two live functions below were the only ones in the assigned brief not
//! already present in the sibling `wv_sf_misc1/2/3.rs` modules. The other 21
//! assigned functions (`stock_ggcg_em`, `stock_hk_valuation_baidu`,
//! `stock_info_cjzc_em`, `stock_info_global_em`, `stock_info_global_sina`,
//! `stock_info_global_futu`, `stock_jgdy_tj_em`, `stock_margin_bse`,
//! `stock_margin_detail_bse`, `stock_margin_detail_sse`, `stock_margin_ratio_pa`,
//! `stock_margin_underlying_info_bse`, `stock_pg_em`, `stock_research_report_em`,
//! `stock_tfp_em`, `stock_us_valuation_baidu`, `stock_xgsglb_em`, `stock_yjkb_em`,
//! `stock_yjyg_em`, `stock_zh_vote_baidu`) are already implemented there and are
//! not re-implemented here to avoid duplication.
//!
//! ## DEFERRED
//! - `stock_info_global_ths` (`stock_info.py:162`): function name contains `_ths`
//!   → 同花顺/hexin-v. Per the porting rules, `_ths` endpoints are deferred.
//! - `stock_info_global_cls` (`stock_info.py:195`): requires an
//!   `md5(sha1(urlencode(params)))` request signature (`sign` param). The crate
//!   only provides `sha2` (SHA-256); MD5/SHA-1 are unavailable and new
//!   dependencies are forbidden, so the signed request cannot be produced →
//!   live fetch would fail. Deferred (no fake).

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE: &str = "sse";
const BASE: &str = "https://query.sse.com.cn/commonSoaQuery.do";
const REFERER: &str = "https://www.sse.com.cn/";

/// A single 参考汇率 row (沪港通-港股通信息披露-参考汇率).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SgtReferenceRateRow {
    #[serde(rename = "适用日期")]
    pub valid_date: String,
    #[serde(rename = "参考汇率买入价")]
    pub buy_price: Option<f64>,
    #[serde(rename = "参考汇率卖出价")]
    pub sell_price: Option<f64>,
    #[serde(rename = "货币种类")]
    pub currency_type: String,
}

/// A single 结算汇兑比率 row (沪港通-港股通信息披露-结算汇兑).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SgtSettlementRateRow {
    #[serde(rename = "适用日期")]
    pub valid_date: String,
    #[serde(rename = "买入结算汇兑比率")]
    pub buy_price: Option<f64>,
    #[serde(rename = "卖出结算汇兑比率")]
    pub sell_price: Option<f64>,
    #[serde(rename = "货币种类")]
    pub currency_type: String,
}

fn str_field(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn num_field(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// Format an SSE date (`"20251231"`) as `"2025-12-31"`; pass through anything
/// already formatted. Mirrors akshare's `pd.to_datetime(...).dt.date`.
fn fmt_date(v: Option<&Value>) -> Result<String> {
    let s = match v {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(Value::Number(n)) => n.to_string(),
        _ => return Ok(String::new()),
    };
    if s.is_empty() {
        return Ok(String::new());
    }
    if s.len() == 8 && s.bytes().all(|b| b.is_ascii_digit()) {
        return Ok(format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8]));
    }
    Ok(s)
}

fn result_array(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("result")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing `result` array".into(),
        })
}

pub(crate) fn parse_sgt_reference_rate(resp: &Value) -> Result<Vec<SgtReferenceRateRow>> {
    let arr = result_array(resp)?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(SgtReferenceRateRow {
            valid_date: fmt_date(item.get("validDate"))?,
            buy_price: num_field(item.get("buyPrice")),
            sell_price: num_field(item.get("sellPrice")),
            currency_type: str_field(item.get("currencyType")),
        });
    }
    out.sort_by(|a, b| a.valid_date.cmp(&b.valid_date));
    Ok(out)
}

pub(crate) fn parse_sgt_settlement_rate(resp: &Value) -> Result<Vec<SgtSettlementRateRow>> {
    let arr = result_array(resp)?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(SgtSettlementRateRow {
            valid_date: fmt_date(item.get("validDate"))?,
            buy_price: num_field(item.get("buyPrice")),
            sell_price: num_field(item.get("sellPrice")),
            currency_type: str_field(item.get("currencyType")),
        });
    }
    out.sort_by(|a, b| a.valid_date.cmp(&b.valid_date));
    Ok(out)
}

/// 沪港通-港股通信息披露-参考汇率.
pub async fn stock_sgt_reference_exchange_rate_sse(
    client: &Client,
) -> Result<Vec<SgtReferenceRateRow>> {
    let params: &[(&str, &str)] = &[
        ("isPagination", "true"),
        ("updateDate", "20120601"),
        ("updateDateEnd", "20300101"),
        ("sqlId", "FW_HGT_GGTHL"),
        ("pageHelp.cacheSize", "1"),
        ("pageHelp.pageSize", "10000"),
        ("pageHelp.pageNo", "1"),
        ("pageHelp.beginPage", "1"),
        ("pageHelp.endPage", "1"),
    ];
    let v = client
        .get_json_with_headers(SOURCE, "sgt_reference_exchange_rate_sse", BASE, params, Some(&[("Referer", REFERER)]))
        .await?;
    parse_sgt_reference_rate(&v)
}

/// 沪港通-港股通信息披露-结算汇兑.
pub async fn stock_sgt_settlement_exchange_rate_sse(
    client: &Client,
) -> Result<Vec<SgtSettlementRateRow>> {
    let params: &[(&str, &str)] = &[
        ("isPagination", "true"),
        ("updateDate", "20120601"),
        ("updateDateEnd", "20300101"),
        ("sqlId", "FW_HGT_JSHDBL"),
        ("pageHelp.cacheSize", "1"),
        ("pageHelp.pageSize", "10000"),
        ("pageHelp.pageNo", "1"),
        ("pageHelp.beginPage", "1"),
        ("pageHelp.endPage", "1"),
    ];
    let v = client
        .get_json_with_headers(
            SOURCE,
            "sgt_settlement_exchange_rate_sse",
            BASE,
            params,
            Some(&[("Referer", REFERER)]),
        )
        .await?;
    parse_sgt_settlement_rate(&v)
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
    fn parse_sgt_reference_ok() {
        let rows = parse_sgt_reference_rate(&fixture("stock_sgt_reference_exchange_rate_sse.json")).unwrap();
        assert_eq!(rows.len(), 5);
        // sorted ascending by 适用日期: earliest first
        assert_eq!(rows[0].valid_date, "2025-12-29");
        assert_eq!(rows[0].currency_type, "HKD");
        // last row is the newest date
        assert_eq!(rows[4].valid_date, "2025-12-31");
        assert!((rows[4].buy_price.unwrap() - 0.8705).abs() < 1e-9);
        assert!((rows[4].sell_price.unwrap() - 0.9243).abs() < 1e-9);
    }

    #[test]
    fn parse_sgt_settlement_ok() {
        let rows = parse_sgt_settlement_rate(&fixture("stock_sgt_settlement_exchange_rate_sse.json")).unwrap();
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].valid_date, "2025-12-23");
        assert_eq!(rows[0].currency_type, "HKD");
        assert_eq!(rows[4].valid_date, "2025-12-31");
        assert!((rows[4].buy_price.unwrap() - 0.89731).abs() < 1e-9);
        assert!((rows[4].sell_price.unwrap() - 0.89749).abs() < 1e-9);
    }
}
