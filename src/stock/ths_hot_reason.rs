//! 同花顺热点题材归因 — ports `ths_hot_reason` from the `simonlin1212/a-stock-data`
//! skill (Layer 3 Signal Layer).
//!
//! GET `http://zx.10jqka.com.cn/event/api/getharden/date/{date}/orderby/date/
//! orderway/desc/charset/GBK/` → `{"errocode":0,"errormsg":"","data":[...]}`.
//!
//! The upstream serves GBK; the impersonate backend decodes it (UTF-8 → GBK
//! fallback), so [`Client::get_text`] returns valid UTF-8 here.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use crate::core::json::{opt_f64, opt_i64, opt_str};

const SOURCE_THS: &str = "ths";
const BASE: &str = "http://zx.10jqka.com.cn/event/api/getharden/date";
const UA: (&str, &str) = (
    "User-Agent",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/117.0.0.0 Safari/537.36",
);

/// One hot-stock + theme-attribution row (`ths_hot_reason`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ThsHotReasonRow {
    /// 6-digit code
    pub code: Option<String>,
    /// 股票简称
    pub name: Option<String>,
    /// 题材归因（核心字段）
    pub reason: Option<String>,
    /// 交易日 `YYYY-MM-DD`
    pub date: Option<String>,
    /// 收盘价
    pub close: Option<f64>,
    /// 涨跌额
    pub change: Option<f64>,
    /// 涨幅%
    pub change_pct: Option<f64>,
    /// 换手率%
    pub turnover: Option<f64>,
    /// 成交额
    pub amount: Option<f64>,
    /// 成交量
    pub volume: Option<f64>,
    /// 大单净量
    pub dde: Option<f64>,
    /// 原始市场码（同花顺，如 33）
    pub market_code: Option<i64>,
    /// 交易所: SH / SZ / BJ（由代码号段推导）
    pub market: Option<String>,
    pub source: &'static str,
}

/// Map a 6-digit code to its exchange (SH / SZ / BJ).
fn exchange(code: &str) -> Option<String> {
    let c = code.trim();
    if c.starts_with("920") || c.starts_with("83") || c.starts_with("43") || c.starts_with('8') || c.starts_with('4') {
        Some("BJ".into())
    } else if c.starts_with('6') || c.starts_with('5') || c.starts_with('9') {
        Some("SH".into())
    } else if c.starts_with('0') || c.starts_with('3') || c.starts_with('2') {
        Some("SZ".into())
    } else {
        None
    }
}

/// Port of `ths_hot_reason(date)` — 当日涨停/异动股题材归因。
///
/// `date` is `YYYY-MM-DD`; pass `None` for today. The upstream lists the
/// harden (涨停) board for that trading day with per-stock theme tags.
pub async fn ths_hot_reason(client: &Client, date: Option<&str>) -> Result<Vec<ThsHotReasonRow>> {
    let day = match date {
        Some(d) => d.to_string(),
        None => chrono::Local::now().format("%Y-%m-%d").to_string(),
    };
    let url = format!("{BASE}/{day}/orderby/date/orderway/desc/charset/GBK/");
    let text = client
        .get_text(SOURCE_THS, "ths_hot_reason", &url, &[], Some(&[UA]))
        .await?;
    let v: Value = serde_json::from_str(&text).map_err(Error::Json)?;
    parse_ths_hot_reason(&v)
}

/// Parse the `getharden` envelope (`data` is a JSON array of rows).
pub(crate) fn parse_ths_hot_reason(resp: &Value) -> Result<Vec<ThsHotReasonRow>> {
    if resp.get("errocode").and_then(|e| e.as_i64()) == Some(1) {
        return Err(Error::UpstreamChanged {
            origin: SOURCE_THS,
            message: format!("ths_hot_reason error: {}", resp.get("errormsg").and_then(|m| m.as_str()).unwrap_or("")),
        });
    }
    let arr = resp
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_THS,
            message: "ths_hot_reason: missing data array".into(),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for x in arr {
        let code = opt_str(x, "code");
        out.push(ThsHotReasonRow {
            market_code: opt_i64(x, "market"),
            market: code.as_deref().and_then(exchange),
            code,
            name: opt_str(x, "name"),
            reason: opt_str(x, "reason"),
            date: opt_str(x, "date"),
            close: opt_f64(x, "close"),
            change: opt_f64(x, "zhangdie"),
            change_pct: opt_f64(x, "zhangfu"),
            turnover: opt_f64(x, "huanshou"),
            amount: opt_f64(x, "chengjiaoe"),
            volume: opt_f64(x, "chengjiaoliang"),
            dde: opt_f64(x, "ddejingliang"),
            source: SOURCE_THS,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fx(name: &str) -> Value {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/{name}.json"));
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    #[test]
    fn parses_hot_reason() {
        let rows = parse_ths_hot_reason(&fx("ths_hot_reason")).unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].code.as_deref(), Some("000712"));
        assert_eq!(rows[0].name.as_deref(), Some("锦龙股份"));
        assert!(rows[0].reason.as_deref().unwrap().contains("证券"));
        assert_eq!(rows[0].market.as_deref(), Some("SZ"));
        assert!(rows[0].change_pct.is_some());
    }

    #[test]
    fn exchange_maps() {
        assert_eq!(exchange("600519"), Some("SH".into()));
        assert_eq!(exchange("000001"), Some("SZ".into()));
        assert_eq!(exchange("920575"), Some("BJ".into()));
    }
}
