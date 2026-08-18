//! `stock_feature` **涨停板池 (limit-up / limit-down pool)** endpoints.
//!
//! Port of akshare `stock_ztb_em.py`. Eastmoney `push2ex` exposes six "topic
//! pools" for the limit-up/limit-down board专题. Five are ported here:
//!
//! | akshare fn                          | push2ex endpoint            | pool            |
//! |-------------------------------------|-----------------------------|-----------------|
//! | `stock_zt_pool_previous_em`         | `getYesterdayZTPool`        | 昨日涨停股池     |
//! | `stock_zt_pool_strong_em`           | `getTopicQSPool`            | 强势股池         |
//! | `stock_zt_pool_sub_new_em`          | `getTopicCXPooll`           | 次新股池         |
//! | `stock_zt_pool_zbgc_em`             | `getTopicZBPool`            | 炸板股池         |
//! | `stock_zt_pool_dtgc_em`             | `getTopicDTPool`            | 跌停股池         |
//!
//! `stock_zt_pool_em` (涨停股池, `getTopicZTPool`) is already ported in
//! `crate::stock::more` and is intentionally **not** duplicated here.
//!
//! All six share the same response envelope: `{"data":{"tc","qdate","pool":[...]}}`
//! where each `pool` item is a flat object. `p` (最新价) and `ztp` (涨停价) are
//! raw integers in units of 1/1000 (akshare divides by 1000). Time fields
//! (`fbt`/`lbt`/`yfbt`) are integers rendered as zero-padded HHMMSS. `zttj` is
//! `{"days":int,"ct":int}` → formatted as `"days/ct"`.
//!
//! Upstream keeps only recent data for some pools; an empty `pool` (or
//! `data: null`) yields an empty `Vec`, mirroring akshare returning an empty
//! frame. Real captured fixtures carry a trade date of 2026-08-14.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};
use crate::core::json::*;

/// Static `ut` token used by the `push2ex` 涨停板 endpoints (a literal constant
/// in akshare, not JS-signed).
const UT: &str = "7eea3edcaed734bea9cbfc24409ed989";
const DPT: &str = "wz.ztzt";
const BASE: &str = "https://push2ex.eastmoney.com";

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------


/// Validate an `YYYYMMDD` trading-day argument.
fn check_date8(date: &str, what: &str) -> Result<()> {
    if date.len() == 8 && date.bytes().all(|b| b.is_ascii_digit()) {
        Ok(())
    } else {
        Err(Error::InvalidParam(format!(
            "{what} must be 8 ASCII digits YYYYMMDD, got {date:?}"
        )))
    }
}

/// Format an Eastmoney HHMMSS-style time (integer or string) as a zero-padded
/// 6-char string, matching akshare's `str.zfill(6)`. `None` when null.
fn fmt_time(v: Option<&Value>) -> Option<String> {
    let v = v?;
    match v {
        Value::Number(n) => n.as_i64().map(|x| format!("{x:06}")),
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }
        _ => None,
    }
}

/// Format an Eastmoney `YYYYMMDD` date (integer or string) as `YYYY-MM-DD`.
/// `0` / null / empty → `None` (akshare coerces these to NaT).
fn fmt_date8(v: Option<&Value>) -> Option<String> {
    let v = v?;
    let s = match v {
        Value::Number(n) => n.as_i64().map(|x| x.to_string()),
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }
        _ => None,
    }?;
    if s == "0" {
        return None;
    }
    if s.len() == 8 {
        Some(format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8]))
    } else {
        Some(s)
    }
}

/// Extract `data.zttj` (`{"days","ct"}`) as the `"days/ct"` string.
fn zt_stat(item: &Value) -> Option<String> {
    let z = item.get("zttj")?;
    let d = z.get("days").and_then(|v| v.as_i64())?;
    let c = z.get("ct").and_then(|v| v.as_i64())?;
    Some(format!("{d}/{c}"))
}

/// Extract the `data.pool` array. `data: null` or empty/missing pool → empty
/// `Vec` (mirrors akshare returning an empty frame for non-trading days).
fn p2ex_pool(resp: &Value) -> Result<Vec<Value>> {
    match resp.get("data") {
        None => Err(Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data at push2ex zt pool".into(),
        }),
        Some(d) if d.is_null() => Ok(Vec::new()),
        Some(d) => Ok(d
            .get("pool")
            .and_then(|p| p.as_array())
            .cloned()
            .unwrap_or_default()),
    }
}

fn base_params<'a>(
    date: &'a str,
    pagesize: &'a str,
    sort: &'a str,
) -> Vec<(&'static str, &'a str)> {
    vec![
        ("ut", UT),
        ("dpt", DPT),
        ("Pageindex", "0"),
        ("pagesize", pagesize),
        ("sort", sort),
        ("date", date),
    ]
}

// ---------------------------------------------------------------------------
// 昨日涨停股池 (getYesterdayZTPool)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ZtPreviousRow {
    pub code: String,
    pub name: String,
    /// `p` 最新价 (÷1000)
    pub price: Option<f64>,
    /// `ztp` 涨停价 (÷1000)
    pub limit_price: Option<f64>,
    /// `zdp` 涨跌幅 (%)
    pub pct_change: Option<f64>,
    /// `amount` 成交额 (元)
    pub amount: Option<f64>,
    /// `ltsz` 流通市值
    pub float_mktcap: Option<f64>,
    /// `tshare` 总市值
    pub total_mktcap: Option<f64>,
    /// `hs` 换手率 (%)
    pub turnover: Option<f64>,
    /// `zf` 振幅 (%)
    pub amplitude: Option<f64>,
    /// `zs` 涨速 (%)
    pub rise_speed: Option<f64>,
    /// `yfbt` 昨日封板时间 (HHMMSS)
    pub prev_seal_time: Option<String>,
    /// `ylbc` 昨日连板数
    pub prev_boards: Option<f64>,
    /// `zttj` 涨停统计 (`days/ct`)
    pub zt_stat: Option<String>,
    /// `hybk` 所属行业
    pub industry: String,
}

pub async fn stock_zt_pool_previous_em(
    client: &Client,
    date: &str,
) -> Result<Vec<ZtPreviousRow>> {
    check_date8(date, "stock_zt_pool_previous_em date")?;
    let params = base_params(date, "5000", "zs:desc");
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_zt_pool_previous_em",
            &format!("{BASE}/getYesterdayZTPool"),
            &params,
        )
        .await?;
    parse_zt_previous(&v)
}

pub(crate) fn parse_zt_previous(resp: &Value) -> Result<Vec<ZtPreviousRow>> {
    let mut out = Vec::new();
    for item in p2ex_pool(resp)? {
        let code = opt_str_or(&item, "c", "");
        let name = opt_str_or(&item, "n", "");
        if code.is_empty() || name.is_empty() {
            continue;
        }
        out.push(ZtPreviousRow {
            code,
            name,
            price: opt_f64(&item, "p").map(|x| x / 1000.0),
            limit_price: opt_f64(&item, "ztp").map(|x| x / 1000.0),
            pct_change: opt_f64(&item, "zdp"),
            amount: opt_f64(&item, "amount"),
            float_mktcap: opt_f64(&item, "ltsz"),
            total_mktcap: opt_f64(&item, "tshare"),
            turnover: opt_f64(&item, "hs"),
            amplitude: opt_f64(&item, "zf"),
            rise_speed: opt_f64(&item, "zs"),
            prev_seal_time: fmt_time(item.get("yfbt")),
            prev_boards: opt_f64(&item, "ylbc"),
            zt_stat: zt_stat(&item),
            industry: opt_str_or(&item, "hybk", ""),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// 强势股池 (getTopicQSPool)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ZtStrongRow {
    pub code: String,
    pub name: String,
    /// `p` 最新价 (÷1000)
    pub price: Option<f64>,
    /// `ztp` 涨停价 (÷1000)
    pub limit_price: Option<f64>,
    /// `zdp` 涨跌幅 (%)
    pub pct_change: Option<f64>,
    /// `amount` 成交额 (元)
    pub amount: Option<f64>,
    /// `ltsz` 流通市值
    pub float_mktcap: Option<f64>,
    /// `tshare` 总市值
    pub total_mktcap: Option<f64>,
    /// `hs` 换手率 (%)
    pub turnover: Option<f64>,
    /// `nh` 是否新高 (1→是)
    pub is_new_high: Option<String>,
    /// `cc` 入选理由 (1/2/3 → 文案)
    pub reason: Option<String>,
    /// `lb` 量比
    pub volume_ratio: Option<f64>,
    /// `zs` 涨速 (%)
    pub rise_speed: Option<f64>,
    /// `zttj` 涨停统计 (`days/ct`)
    pub zt_stat: Option<String>,
    /// `hybk` 所属行业
    pub industry: String,
}

pub async fn stock_zt_pool_strong_em(client: &Client, date: &str) -> Result<Vec<ZtStrongRow>> {
    check_date8(date, "stock_zt_pool_strong_em date")?;
    let params = base_params(date, "5000", "zdp:desc");
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_zt_pool_strong_em",
            &format!("{BASE}/getTopicQSPool"),
            &params,
        )
        .await?;
    parse_zt_strong(&v)
}

pub(crate) fn parse_zt_strong(resp: &Value) -> Result<Vec<ZtStrongRow>> {
    const REASON: &[(i64, &str)] = &[
        (1, "60日新高"),
        (2, "近期多次涨停"),
        (3, "60日新高且近期多次涨停"),
    ];
    let mut out = Vec::new();
    for item in p2ex_pool(resp)? {
        let code = opt_str_or(&item, "c", "");
        let name = opt_str_or(&item, "n", "");
        if code.is_empty() || name.is_empty() {
            continue;
        }
        let reason = item
            .get("cc")
            .and_then(|v| v.as_i64())
            .and_then(|c| REASON.iter().find(|(k, _)| *k == c).map(|(_, s)| s.to_string()));
        let is_new_high = item
            .get("nh")
            .and_then(|v| v.as_i64())
            .map(|x| if x == 1 { "是" } else { "否" }.to_string());
        out.push(ZtStrongRow {
            code,
            name,
            price: opt_f64(&item, "p").map(|x| x / 1000.0),
            limit_price: opt_f64(&item, "ztp").map(|x| x / 1000.0),
            pct_change: opt_f64(&item, "zdp"),
            amount: opt_f64(&item, "amount"),
            float_mktcap: opt_f64(&item, "ltsz"),
            total_mktcap: opt_f64(&item, "tshare"),
            turnover: opt_f64(&item, "hs"),
            is_new_high,
            reason,
            volume_ratio: opt_f64(&item, "lb"),
            rise_speed: opt_f64(&item, "zs"),
            zt_stat: zt_stat(&item),
            industry: opt_str_or(&item, "hybk", ""),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// 次新股池 (getTopicCXPooll)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ZtSubNewRow {
    pub code: String,
    pub name: String,
    /// `p` 最新价 (÷1000)
    pub price: Option<f64>,
    /// `ztp` 涨停价 (÷1000)
    pub limit_price: Option<f64>,
    /// `zdp` 涨跌幅 (%)
    pub pct_change: Option<f64>,
    /// `amount` 成交额 (元)
    pub amount: Option<f64>,
    /// `ltsz` 流通市值
    pub float_mktcap: Option<f64>,
    /// `tshare` 总市值
    pub total_mktcap: Option<f64>,
    /// `hs` 转手率 (%)
    pub turnover: Option<f64>,
    /// `ods` 开板几日
    pub open_days: Option<f64>,
    /// `od` 开板日期 (YYYY-MM-DD)
    pub open_date: Option<String>,
    /// `ipod` 上市日期 (YYYY-MM-DD)
    pub ipo_date: Option<String>,
    /// `nh` 是否新高 (1→是)
    pub is_new_high: Option<String>,
    /// `zttj` 涨停统计 (`days/ct`)
    pub zt_stat: Option<String>,
    /// `hybk` 所属行业
    pub industry: String,
}

pub async fn stock_zt_pool_sub_new_em(
    client: &Client,
    date: &str,
) -> Result<Vec<ZtSubNewRow>> {
    check_date8(date, "stock_zt_pool_sub_new_em date")?;
    let params = base_params(date, "5000", "ods:asc");
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_zt_pool_sub_new_em",
            &format!("{BASE}/getTopicCXPooll"),
            &params,
        )
        .await?;
    parse_zt_sub_new(&v)
}

pub(crate) fn parse_zt_sub_new(resp: &Value) -> Result<Vec<ZtSubNewRow>> {
    let mut out = Vec::new();
    for item in p2ex_pool(resp)? {
        let code = opt_str_or(&item, "c", "");
        let name = opt_str_or(&item, "n", "");
        if code.is_empty() || name.is_empty() {
            continue;
        }
        let is_new_high = item
            .get("nh")
            .and_then(|v| v.as_i64())
            .map(|x| if x == 1 { "是" } else { "否" }.to_string());
        out.push(ZtSubNewRow {
            code,
            name,
            price: opt_f64(&item, "p").map(|x| x / 1000.0),
            limit_price: opt_f64(&item, "ztp").map(|x| x / 1000.0),
            pct_change: opt_f64(&item, "zdp"),
            amount: opt_f64(&item, "amount"),
            float_mktcap: opt_f64(&item, "ltsz"),
            total_mktcap: opt_f64(&item, "tshare"),
            turnover: opt_f64(&item, "hs"),
            open_days: opt_f64(&item, "ods"),
            open_date: fmt_date8(item.get("od")),
            ipo_date: fmt_date8(item.get("ipod")),
            is_new_high,
            zt_stat: zt_stat(&item),
            industry: opt_str_or(&item, "hybk", ""),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// 炸板股池 (getTopicZBPool)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ZtZbgcRow {
    pub code: String,
    pub name: String,
    /// `p` 最新价 (÷1000)
    pub price: Option<f64>,
    /// `ztp` 涨停价 (÷1000)
    pub limit_price: Option<f64>,
    /// `zdp` 涨跌幅 (%)
    pub pct_change: Option<f64>,
    /// `amount` 成交额 (元)
    pub amount: Option<f64>,
    /// `ltsz` 流通市值
    pub float_mktcap: Option<f64>,
    /// `tshare` 总市值
    pub total_mktcap: Option<f64>,
    /// `hs` 换手率 (%)
    pub turnover: Option<f64>,
    /// `fbt` 首次封板时间 (HHMMSS)
    pub first_time: Option<String>,
    /// `zbc` 炸板次数
    pub explode_count: Option<f64>,
    /// `zf` 振幅 (%)
    pub amplitude: Option<f64>,
    /// `zs` 涨速 (%)
    pub rise_speed: Option<f64>,
    /// `zttj` 涨停统计 (`days/ct`)
    pub zt_stat: Option<String>,
    /// `hybk` 所属行业
    pub industry: String,
}

pub async fn stock_zt_pool_zbgc_em(client: &Client, date: &str) -> Result<Vec<ZtZbgcRow>> {
    check_date8(date, "stock_zt_pool_zbgc_em date")?;
    let params = base_params(date, "5000", "fbt:asc");
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_zt_pool_zbgc_em",
            &format!("{BASE}/getTopicZBPool"),
            &params,
        )
        .await?;
    parse_zt_zbgc(&v)
}

pub(crate) fn parse_zt_zbgc(resp: &Value) -> Result<Vec<ZtZbgcRow>> {
    let mut out = Vec::new();
    for item in p2ex_pool(resp)? {
        let code = opt_str_or(&item, "c", "");
        let name = opt_str_or(&item, "n", "");
        if code.is_empty() || name.is_empty() {
            continue;
        }
        out.push(ZtZbgcRow {
            code,
            name,
            price: opt_f64(&item, "p").map(|x| x / 1000.0),
            limit_price: opt_f64(&item, "ztp").map(|x| x / 1000.0),
            pct_change: opt_f64(&item, "zdp"),
            amount: opt_f64(&item, "amount"),
            float_mktcap: opt_f64(&item, "ltsz"),
            total_mktcap: opt_f64(&item, "tshare"),
            turnover: opt_f64(&item, "hs"),
            first_time: fmt_time(item.get("fbt")),
            explode_count: opt_f64(&item, "zbc"),
            amplitude: opt_f64(&item, "zf"),
            rise_speed: opt_f64(&item, "zs"),
            zt_stat: zt_stat(&item),
            industry: opt_str_or(&item, "hybk", ""),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// 跌停股池 (getTopicDTPool)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ZtDtgcRow {
    pub code: String,
    pub name: String,
    /// `zdp` 涨跌幅 (%)
    pub pct_change: Option<f64>,
    /// `p` 最新价 (÷1000)
    pub price: Option<f64>,
    /// `amount` 成交额 (元)
    pub amount: Option<f64>,
    /// `ltsz` 流通市值
    pub float_mktcap: Option<f64>,
    /// `tshare` 总市值
    pub total_mktcap: Option<f64>,
    /// `pe` 动态市盈率
    pub pe: Option<f64>,
    /// `hs` 换手率 (%)
    pub turnover: Option<f64>,
    /// `fund` 封单资金
    pub seal_fund: Option<f64>,
    /// `lbt` 最后封板时间 (HHMMSS)
    pub last_time: Option<String>,
    /// `fba` 板上成交额
    pub on_board_amount: Option<f64>,
    /// `days` 连续跌停
    pub consecutive_downs: Option<f64>,
    /// `oc` 开板次数
    pub open_count: Option<f64>,
    /// `hybk` 所属行业
    pub industry: String,
}

pub async fn stock_zt_pool_dtgc_em(client: &Client, date: &str) -> Result<Vec<ZtDtgcRow>> {
    check_date8(date, "stock_zt_pool_dtgc_em date")?;
    let params = base_params(date, "10000", "fund:asc");
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_zt_pool_dtgc_em",
            &format!("{BASE}/getTopicDTPool"),
            &params,
        )
        .await?;
    parse_zt_dtgc(&v)
}

pub(crate) fn parse_zt_dtgc(resp: &Value) -> Result<Vec<ZtDtgcRow>> {
    let mut out = Vec::new();
    for item in p2ex_pool(resp)? {
        let code = opt_str_or(&item, "c", "");
        let name = opt_str_or(&item, "n", "");
        if code.is_empty() || name.is_empty() {
            continue;
        }
        out.push(ZtDtgcRow {
            code,
            name,
            pct_change: opt_f64(&item, "zdp"),
            price: opt_f64(&item, "p").map(|x| x / 1000.0),
            amount: opt_f64(&item, "amount"),
            float_mktcap: opt_f64(&item, "ltsz"),
            total_mktcap: opt_f64(&item, "tshare"),
            pe: opt_f64(&item, "pe"),
            turnover: opt_f64(&item, "hs"),
            seal_fund: opt_f64(&item, "fund"),
            last_time: fmt_time(item.get("lbt")),
            on_board_amount: opt_f64(&item, "fba"),
            consecutive_downs: opt_f64(&item, "days"),
            open_count: opt_f64(&item, "oc"),
            industry: opt_str_or(&item, "hybk", ""),
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
    fn parses_zt_previous() {
        let rows = parse_zt_previous(&fixture("stock_zt_pool_previous_em.json")).unwrap();
        assert!(!rows.is_empty());
        let r = &rows[0];
        assert_eq!(r.code, "000006");
        assert_eq!(r.name, "深振业Ａ");
        assert!(approx(r.price, 7.520));
        assert!(approx(r.limit_price, 8.470));
        assert!(approx(r.pct_change, -2.3376622));
        assert_eq!(r.zt_stat.as_deref(), Some("2/1"));
        assert_eq!(r.industry, "房地产开");
        assert_eq!(r.prev_seal_time.as_deref(), Some("132727"));
    }

    #[test]
    fn parses_zt_strong() {
        let rows = parse_zt_strong(&fixture("stock_zt_pool_strong_em.json")).unwrap();
        assert!(!rows.is_empty());
        let r = &rows[0];
        assert_eq!(r.code, "300404");
        assert_eq!(r.name, "博济医药");
        assert!(approx(r.price, 17.340));
        assert!(approx(r.limit_price, 17.340));
        assert!(approx(r.pct_change, 20.0));
        assert_eq!(r.is_new_high.as_deref(), Some("是"));
        assert_eq!(r.reason.as_deref(), Some("60日新高且近期多次涨停"));
        assert!(approx(r.volume_ratio, 1.5058461));
        assert_eq!(r.zt_stat.as_deref(), Some("2/2"));
        assert_eq!(r.industry, "医疗服务");
    }

    #[test]
    fn parses_zt_sub_new() {
        let rows = parse_zt_sub_new(&fixture("stock_zt_pool_sub_new_em.json")).unwrap();
        assert!(!rows.is_empty());
        let r = &rows[0];
        assert_eq!(r.code, "920138");
        assert_eq!(r.name, "杰理科技");
        assert!(approx(r.price, 45.240));
        assert!(approx(r.limit_price, 60.560));
        assert_eq!(r.open_days, Some(3.0));
        assert_eq!(r.open_date.as_deref(), Some("2026-08-12"));
        assert_eq!(r.ipo_date.as_deref(), Some("2026-08-12"));
        assert_eq!(r.is_new_high.as_deref(), Some("否"));
        assert_eq!(r.zt_stat.as_deref(), Some("0/0"));
        assert_eq!(r.industry, "半导体");
    }

    #[test]
    fn parses_zt_zbgc() {
        let rows = parse_zt_zbgc(&fixture("stock_zt_pool_zbgc_em.json")).unwrap();
        assert!(!rows.is_empty());
        let r = &rows[0];
        assert_eq!(r.code, "300333");
        assert_eq!(r.name, "兆日科技");
        assert!(approx(r.price, 11.280));
        assert!(approx(r.limit_price, 12.770));
        assert_eq!(r.first_time.as_deref(), Some("092500"));
        assert_eq!(r.explode_count, Some(1.0));
        assert!(approx(r.amplitude, 17.011_278));
        assert_eq!(r.zt_stat.as_deref(), Some("3/2"));
        assert_eq!(r.industry, "计算机设");
    }

    #[test]
    fn parses_zt_dtgc() {
        let rows = parse_zt_dtgc(&fixture("stock_zt_pool_dtgc_em.json")).unwrap();
        assert!(!rows.is_empty());
        let r = &rows[0];
        assert_eq!(r.code, "600683");
        assert_eq!(r.name, "京投发展");
        assert!(approx(r.pct_change, -9.976_798));
        assert!(approx(r.price, 11.640));
        assert!(approx(r.pe, -13.765_398));
        assert_eq!(r.last_time.as_deref(), Some("150000"));
        assert_eq!(r.consecutive_downs, Some(1.0));
        assert_eq!(r.open_count, Some(7.0));
        assert_eq!(r.industry, "房地产开");
    }
}
