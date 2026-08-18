//! China / government-bond spot and registration endpoints.
//!
//! Port of akshare `bond/bond_gb_sina.py` (Sina 中国/美国国债收益率行情) and
//! `bond/bond_nafmii.py` (NAFMII 非金融企业债务融资工具注册信息). All functions
//! hit pure-JSON endpoints (no JS decode, token, HTML scrape, or Excel).
//!
//! Ported public functions:
//! - [`bond_gb_zh_sina`] — `akshare/bond/bond_gb_sina.py:13`  (中国国债收益率行情)
//! - [`bond_gb_us_sina`] — `akshare/bond/bond_gb_sina.py:54`  (美国国债收益率行情)
//! - [`bond_debt_nafmii`] — `akshare/bond/bond_nafmii.py:13`  (NAFMII 注册信息)
//!
//! Already ported elsewhere (NOT re-implemented here):
//! - `bond_zh_us_rate`   — `src/bond/eastmoney.rs` (akshare `bond/bond_em.py:14`).
//! - `bond_spot_quote` / `bond_spot_deal` — `src/bond/chinamoney.rs`.
//! - `bond_zh_hs_cov_spot/daily/min/...` — `src/bond/cov.rs`.
//!
//! DEFERRED (with reason):
//! - `bond_zh_us_stock` / `bond_china_money` — no function with these names exists
//!   in the akshare tree (verified via `grep -rn` across `akshare/`); the matching
//!   China/US gov-bond feeds are `bond_gb_zh_sina` / `bond_gb_us_sina`, ported below.
//! - `bond_china_yield` (`bond/bond_china.py:142`) — returns `pd.read_html` over a
//!   `&nbsp`-stripped HTML page: DEFERRED (HTML scrape, no JSON).
//! - `bond_cash_summary_sse` / `bond_deal_summary_sse` (`bond/bond_summary.py`) —
//!   both `pd.read_excel(BytesIO(...))`; DEFERRED (Excel binary, needs an xls engine).
//! - `bond_china_close_return` / `macro_china_swap_rate` (`bond/bond_china_money.py`)
//!   — require a ChinaMoney service-registration bootstrap (`bond_china_close_return_map`)
//!   that fetches a hardcoded token and depends on cookies: DEFERRED (third-party token).
//! - `bond_zh_hs_spot` (`bond/bond_zh_sina.py:45`) — Sina feed decoded via `demjson`
//!   (non-strict JSON) with fragile positional columns; overlaps `bond_zh_hs_cov_spot`
//!   (`src/bond/cov.rs`): DEFERRED.
//! - `bond_cb_profile_sina` / `bond_cb_summary_sina` (`bond/bond_cb_sina.py`) —
//!   `pd.read_html`: DEFERRED.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_SINA};
use crate::core::error::{Error, Result};
use crate::core::json::*;

/// NAFMII source id (local, not in `core::client`).
const SOURCE_NAFMII: &str = "nafmii";

// ---------------------------------------------------------------------------
// bond_gb_zh_sina / bond_gb_us_sina — 新浪财经 中国/美国国债收益率行情
// https://stock.finance.sina.com.cn/forex/globalbd/cn10yt.html
// ---------------------------------------------------------------------------

const GB_SINA_URL: &str = "https://bond.finance.sina.com.cn/hq/gb/daily";

/// 中国国债期限 choice → Sina symbol code.
const ZH_SYMBOL_MAP: &[(&str, &str)] = &[
    ("中国1年期国债", "CN1YT"),
    ("中国2年期国债", "CN2YT"),
    ("中国3年期国债", "CN3YT"),
    ("中国5年期国债", "CN5YT"),
    ("中国7年期国债", "CN7YT"),
    ("中国10年期国债", "CN10YT"),
    ("中国15年期国债", "CN15YT"),
    ("中国20年期国债", "CN20YT"),
    ("中国30年期国债", "CN30YT"),
];

/// 美国国债期限 choice → Sina symbol code.
const US_SYMBOL_MAP: &[(&str, &str)] = &[
    ("美国1月期国债", "US1MT"),
    ("美国2月期国债", "US2MT"),
    ("美国3月期国债", "US3MT"),
    ("美国4月期国债", "US4MT"),
    ("美国6月期国债", "US6MT"),
    ("美国1年期国债", "US1YT"),
    ("美国2年期国债", "US2YT"),
    ("美国3年期国债", "US3YT"),
    ("美国5年期国债", "US5YT"),
    ("美国7年期国债", "US7YT"),
    ("美国10年期国债", "US10YT"),
    ("美国20年期国债", "US20YT"),
    ("美国30年期国债", "US30YT"),
];

/// 中国/美国国债收益率行情 row (`bond_gb_zh_sina` / `bond_gb_us_sina`).
///
/// Mirrors akshare's columns: `date, open, high, low, close, volume`. The upstream
/// `result.data` is a list of 6-element positional rows.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondGbSina {
    /// 日期 — row[0]
    pub date: String,
    /// 开盘 — row[1]
    pub open: Option<f64>,
    /// 最高 — row[2]
    pub high: Option<f64>,
    /// 最低 — row[3]
    pub low: Option<f64>,
    /// 收盘 — row[4]
    pub close: Option<f64>,
    /// 成交量 — row[5]
    pub volume: Option<f64>,
    pub source: &'static str,
}

/// 新浪财经-中国国债收益率行情 (`bond_gb_zh_sina`, `bond_gb_sina.py:13`).
///
/// `symbol` ∈ `ZH_SYMBOL_MAP` keys, e.g. `"中国10年期国债"`.
pub async fn bond_gb_zh_sina(client: &Client, symbol: &str) -> Result<Vec<BondGbSina>> {
    let code = lookup(ZH_SYMBOL_MAP, symbol, "unknown zh symbol")?;
    fetch_gb_sina(client, code, "bond_gb_zh_sina").await
}

/// 新浪财经-美国国债收益率行情 (`bond_gb_us_sina`, `bond_gb_sina.py:54`).
///
/// `symbol` ∈ `US_SYMBOL_MAP` keys, e.g. `"美国10年期国债"`.
pub async fn bond_gb_us_sina(client: &Client, symbol: &str) -> Result<Vec<BondGbSina>> {
    let code = lookup(US_SYMBOL_MAP, symbol, "unknown us symbol")?;
    fetch_gb_sina(client, code, "bond_gb_us_sina").await
}

async fn fetch_gb_sina(client: &Client, code: &str, endpoint: &'static str) -> Result<Vec<BondGbSina>> {
    let url = format!("{GB_SINA_URL}?symbol={code}");
    let v = client.get_json(SOURCE_SINA, endpoint, &url, &[]).await?;
    let items = v
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "missing result.data".into(),
        })?;
    parse_gb_sina(items)
}

/// Parse Sina gov-bond rows (`result.data` array) into [`BondGbSina`].
pub(crate) fn parse_gb_sina(items: &[Value]) -> Result<Vec<BondGbSina>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        if let Some(arr) = item.as_array() {
            out.push(parse_gb_row(arr));
        }
    }
    Ok(out)
}

/// Parse one positional 6-element row (`&[Value]`) into a [`BondGbSina`].
fn parse_gb_row(arr: &[Value]) -> BondGbSina {
    BondGbSina {
        date: at_str(arr, 0),
        open: at_num(arr, 1),
        high: at_num(arr, 2),
        low: at_num(arr, 3),
        close: at_num(arr, 4),
        volume: at_num(arr, 5),
        source: SOURCE_SINA,
    }
}

// ---------------------------------------------------------------------------
// bond_debt_nafmii — 交易商协会 非金融企业债务融资工具注册信息
// http://zhuce.nafmii.org.cn/fans/publicQuery/releFileProjDataGrid
// ---------------------------------------------------------------------------

const NAFMII_URL: &str = "http://zhuce.nafmii.org.cn/fans/publicQuery/releFileProjDataGrid";

/// 非金融企业债务融资工具注册信息 row (`bond_debt_nafmii`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondDebtNafmii {
    /// 债券名称 — `regFileName`
    pub bond_name: String,
    /// 品种 — `regPrdtType`
    pub product_type: String,
    /// 注册或备案 — `isReg`
    pub is_reg: String,
    /// 金额(亿元) — `firstIssueAmount`
    pub amount: Option<f64>,
    /// 注册通知书文号 — `regNoticeNo` (empty → `None`)
    pub reg_notice_no: Option<String>,
    /// 更新日期 — `releaseTime`
    pub release_time: Option<String>,
    /// 项目状态 — `projPhase`
    pub proj_phase: String,
    pub source: &'static str,
}

/// 交易商协会-非金融企业债务融资工具注册信息 (`bond_debt_nafmii`, `bond_nafmii.py:13`).
///
/// `page` is the 1-based page number (default `"1"`); 50 rows/page.
pub async fn bond_debt_nafmii(client: &Client, page: &str) -> Result<Vec<BondDebtNafmii>> {
    let params = [
        ("regFileName", ""),
        ("itemType", ""),
        ("startTime", ""),
        ("endTime", ""),
        ("entityName", ""),
        ("leadManager", ""),
        ("regPrdtType", ""),
        ("page", page),
        ("rows", "50"),
    ];
    let v = client
        .post_form_json(SOURCE_NAFMII, "bond_debt_nafmii", NAFMII_URL, &params, None)
        .await?;
    let rows = v.get("rows").and_then(|r| r.as_array()).ok_or_else(|| {
        Error::UpstreamChanged {
            origin: SOURCE_NAFMII,
            message: "missing rows".into(),
        }
    })?;
    parse_bond_debt_nafmii(rows)
}

/// Parse NAFMII `rows` array into [`BondDebtNafmii`].
pub(crate) fn parse_bond_debt_nafmii(items: &[Value]) -> Result<Vec<BondDebtNafmii>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        if let Some(row) = parse_nafmii_item(item) {
            out.push(row);
        }
    }
    Ok(out)
}

/// Parse one NAFMII `rows` object (`&Value`) into a [`BondDebtNafmii`].
fn parse_nafmii_item(item: &Value) -> Option<BondDebtNafmii> {
    let bond_name = opt_str_or(item, "regFileName", "");
    if bond_name.is_empty() {
        return None;
    }
    let notice = opt_str_or(item, "regNoticeNo", "");
    let reg_notice_no = if notice.is_empty() {
        None
    } else {
        Some(notice)
    };
    Some(BondDebtNafmii {
        bond_name,
        product_type: opt_str_or(item, "regPrdtType", ""),
        is_reg: opt_str_or(item, "isReg", ""),
        amount: opt_f64(item, "firstIssueAmount"),
        reg_notice_no,
        release_time: fstr_opt(item, "releaseTime"),
        proj_phase: opt_str_or(item, "projPhase", ""),
        source: SOURCE_NAFMII,
    })
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn lookup(m: &'static [(&str, &str)], s: &str, what: &str) -> Result<&'static str> {
    m.iter()
        .find(|(k, _)| *k == s)
        .map(|(_, v)| *v)
        .ok_or_else(|| Error::InvalidParam(format!("{what}: {s}")))
}

fn at_str(arr: &[Value], idx: usize) -> String {
    arr.get(idx)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn at_num(arr: &[Value], idx: usize) -> Option<f64> {
    arr.get(idx).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    })
}


fn fstr_opt(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
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
        let txt = std::fs::read_to_string(p).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    /// Compare an `Option<f64>` field against an expected value within 1e-9.
    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-9,
            None => false,
        }
    }

    #[test]
    fn parses_bond_gb_zh_sina() {
        let v = fixture("bond_gb_zh_sina.json");
        let items = v
            .get("result")
            .and_then(|r| r.get("data"))
            .and_then(|d| d.as_array())
            .unwrap();
        let rows = parse_gb_sina(items).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02");
        assert!(approx(rows[0].open, 2.50));
        assert!(approx(rows[0].high, 2.70));
        assert!(approx(rows[0].low, 2.40));
        assert!(approx(rows[0].close, 2.60));
        assert!(approx(rows[0].volume, 12345.0));
        assert_eq!(rows[0].source, "sina");
        assert_eq!(rows[1].date, "2024-01-03");
        assert!(approx(rows[1].close, 2.70));
    }

    #[test]
    fn parses_bond_gb_us_sina() {
        let v = fixture("bond_gb_us_sina.json");
        let items = v
            .get("result")
            .and_then(|r| r.get("data"))
            .and_then(|d| d.as_array())
            .unwrap();
        let rows = parse_gb_sina(items).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02");
        assert!(approx(rows[0].open, 4.20));
        assert!(approx(rows[0].close, 4.30));
        assert_eq!(rows[0].source, "sina");
        assert_eq!(rows[1].date, "2024-01-03");
        assert!(approx(rows[1].close, 4.40));
    }

    #[test]
    fn parses_bond_debt_nafmii() {
        let v = fixture("bond_debt_nafmii.json");
        let rows_arr = v.get("rows").and_then(|r| r.as_array()).unwrap();
        let rows = parse_bond_debt_nafmii(rows_arr).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].bond_name, "某某MTN");
        assert_eq!(rows[0].product_type, "中期票据");
        assert_eq!(rows[0].is_reg, "注册");
        assert!(approx(rows[0].amount, 10.0));
        assert_eq!(
            rows[0].reg_notice_no.as_deref(),
            Some("中市协注〔2024〕MTN1号")
        );
        assert_eq!(rows[0].release_time.as_deref(), Some("2024-01-02"));
        assert_eq!(rows[0].proj_phase, "完成");
        assert_eq!(rows[0].source, "nafmii");
        // Empty `regNoticeNo` becomes `None`.
        assert_eq!(rows[1].bond_name, "某某CP");
        assert_eq!(rows[1].reg_notice_no, None);
        assert_eq!(rows[1].is_reg, "备案");
    }
}
