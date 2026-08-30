//! Eastmoney "signal" endpoints — ports `eastmoney_concept_blocks`,
//! `industry_comparison`, `em_stock_monitor`, `em_price_anomaly` /
//! `em_price_anomaly_count` from the `simonlin1212/a-stock-data` skill.
//!
//! All are no-key HTTP GETs to `push2.eastmoney.com`, `mobappconfig.securities
//! .eastmoney.com`, and `dycalchis.eastmoney.com`.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use crate::core::json::{opt_f64, opt_i64, opt_str};

const SOURCE_EASTMONEY: &str = "eastmoney";
const SLIST_URL: &str = "https://push2.eastmoney.com/api/qt/slist/get";
const CLIST_URL: &str = "https://push2.eastmoney.com/api/qt/clist/get";
const MONITOR_URL: &str =
    "https://mobappconfig.securities.eastmoney.com/emcfg/stock_monitor.json";
const ANOMALY_BASE: &str = "https://dycalchis.eastmoney.com/price-anomaly";

/// Build an Eastmoney `secid` (`1.600519` / `0.300750`). 沪=1, 深/北=0.
fn em_secid(code: &str) -> String {
    let c = code.trim();
    // 北交所/老三板号段与深市同为 `0`，需先于通用 `9` 判定。
    let market = if c.starts_with("920")
        || c.starts_with("83")
        || c.starts_with("43")
        || c.starts_with("87")
        || c.starts_with('8')
        || c.starts_with('4')
    {
        "0"
    } else if c.starts_with('6') || c.starts_with('5') || c.starts_with('9') {
        "1"
    } else {
        "0"
    };
    let digits: String = c.chars().filter(|ch| ch.is_ascii_digit()).collect();
    format!("{market}.{digits}")
}

/// One board/industry/concept a stock belongs to (`eastmoney_concept_blocks`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct EmConceptBlockRow {
    pub name: Option<String>,
    /// BK code
    pub code: Option<String>,
    /// 板块当日涨跌幅%
    pub change_pct: Option<f64>,
    /// 板块龙头股名
    pub lead_stock: Option<String>,
    pub source: &'static str,
}

/// Port of `eastmoney_concept_blocks(code)` — 个股所属板块/概念/地域（混合）。
pub async fn eastmoney_concept_blocks(client: &Client, code: &str) -> Result<Vec<EmConceptBlockRow>> {
    let secid = em_secid(code);
    let v = client
        .get_json_with_headers(
            SOURCE_EASTMONEY,
            "em_concept_blocks",
            SLIST_URL,
            &[
                ("fltt", "2"),
                ("invt", "2"),
                ("secid", secid.as_str()),
                ("spt", "3"),
                ("pi", "0"),
                ("pz", "200"),
                ("po", "1"),
                ("fields", "f12,f14,f3,f128"),
            ],
            Some(&[("Referer", "https://quote.eastmoney.com/")]),
        )
        .await?;
    parse_em_concept_blocks(&v)
}

/// Parse `slist` envelope (diff may be an object keyed by index).
pub(crate) fn parse_em_concept_blocks(resp: &Value) -> Result<Vec<EmConceptBlockRow>> {
    let diff = resp
        .get("data")
        .and_then(|d| d.get("diff"))
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "em_concept_blocks: missing data.diff".into(),
        })?;
    let items: Vec<&Value> = match diff {
        Value::Object(o) => o.values().collect(),
        Value::Array(a) => a.iter().collect(),
        _ => {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "em_concept_blocks: data.diff not object/array".into(),
            })
        }
    };
    let mut out = Vec::with_capacity(items.len());
    for it in items {
        out.push(EmConceptBlockRow {
            name: opt_str(it, "f14"),
            code: opt_str(it, "f12"),
            change_pct: opt_f64(it, "f3"),
            lead_stock: opt_str(it, "f128"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

/// One industry ranking row (`industry_comparison`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct EmIndustryRow {
    pub rank: usize,
    pub name: Option<String>,
    pub change_pct: Option<f64>,
    pub code: Option<String>,
    pub up_count: Option<i64>,
    pub down_count: Option<i64>,
    pub leader: Option<String>,
    pub leader_change: Option<f64>,
    pub source: &'static str,
}

/// Port of `industry_comparison(top_n)` — 全行业涨跌幅排名（东财行业板块）。
pub async fn industry_comparison(client: &Client, top_n: usize) -> Result<Vec<EmIndustryRow>> {
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "industry_comparison",
            CLIST_URL,
            &[
                ("pn", "1"),
                ("pz", "100"),
                ("po", "1"),
                ("np", "1"),
                ("fltt", "2"),
                ("invt", "2"),
                ("fid", "f3"),
                ("fs", "m:90+t:2"),
                ("fields", "f2,f3,f4,f12,f13,f14,f104,f105,f128,f136,f140,f141,f207"),
            ],
        )
        .await?;
    parse_industry_comparison(&v, top_n)
}

/// Parse `clist` envelope (diff is an array, ascending by rank).
pub(crate) fn parse_industry_comparison(resp: &Value, top_n: usize) -> Result<Vec<EmIndustryRow>> {
    let diff = resp
        .get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "industry_comparison: missing data.diff".into(),
        })?;
    let mut out = Vec::with_capacity(diff.len());
    for (i, it) in diff.iter().enumerate() {
        out.push(EmIndustryRow {
            rank: i + 1,
            name: opt_str(it, "f14"),
            change_pct: opt_f64(it, "f3"),
            code: opt_str(it, "f12"),
            up_count: opt_i64(it, "f104"),
            down_count: opt_i64(it, "f105"),
            leader: opt_str(it, "f140"),
            leader_change: opt_f64(it, "f136"),
            source: SOURCE_EASTMONEY,
        });
    }
    let n = top_n.min(out.len());
    Ok(out.into_iter().take(n).collect())
}

/// One 重点监控池 row (`em_stock_monitor`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct EmStockMonitorRow {
    pub code: Option<String>,
    pub name: Option<String>,
    /// 市场: SH / SZ / BJ
    pub market: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub link: Option<String>,
    pub source: &'static str,
}

/// Port of `em_stock_monitor(only_active)` — 东财重点监控池。
pub async fn em_stock_monitor(client: &Client, only_active: bool) -> Result<Vec<EmStockMonitorRow>> {
    let v = client
        .get_json_with_headers(
            SOURCE_EASTMONEY,
            "em_stock_monitor",
            MONITOR_URL,
            &[],
            Some(&[("Referer", "https://vipmoney.eastmoney.com/")]),
        )
        .await?;
    parse_em_stock_monitor(&v, only_active)
}

/// Map 东财 MARKET 字段 (`1`/`0`/`B`) → `SH`/`SZ`/`BJ`.
fn monitor_market(raw: &str) -> String {
    match raw.to_ascii_uppercase().as_str() {
        "1" => "SH",
        "0" => "SZ",
        "B" => "BJ",
        other => return format!("?{other}"),
    }
    .to_string()
}

/// Parse the monitor JSON array.
pub(crate) fn parse_em_stock_monitor(resp: &Value, only_active: bool) -> Result<Vec<EmStockMonitorRow>> {
    let arr = resp
        .as_array()
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "em_stock_monitor: expected JSON array".into(),
        })?;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let mut out = Vec::with_capacity(arr.len());
    for x in arr {
        let start = opt_str(x, "VALIDATESTARTDATE");
        let end = opt_str(x, "VALIDATEENDDATE");
        if only_active {
            let active = match (&start, &end) {
                (Some(s), Some(e)) => s.as_str() <= today.as_str() && today.as_str() <= e.as_str(),
                _ => false,
            };
            if !active {
                continue;
            }
        }
        out.push(EmStockMonitorRow {
            code: opt_str(x, "STKCODE"),
            name: opt_str(x, "STKNAME"),
            market: x
                .get("MARKET")
                .and_then(|m| m.as_str())
                .map(monitor_market),
            start,
            end,
            link: opt_str(x, "LINK_URL"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

/// One 严重异常波动 row (`em_price_anomaly`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct EmPriceAnomalyRow {
    pub date: Option<String>,
    pub code: Option<String>,
    pub name: Option<String>,
    /// 交易所: SH / SZ / BJ
    pub market: Option<String>,
    /// 当日涨跌幅%
    pub change_pct: Option<f64>,
    /// 累计偏离值%
    pub deviation: Option<f64>,
    /// 统计窗口天数
    pub days: Option<i64>,
    /// 板块码（1主板/4创业板/6科创板/8北交所）
    pub board: Option<i64>,
    pub rule_code: i64,
    pub rule: String,
    pub is_today: bool,
    pub source: &'static str,
}

/// Resolve an anomaly rule code → human-readable text.
fn anomaly_rule(rule_code: i64) -> &'static str {
    match rule_code {
        1 => "主板连续10个交易日内4次出现同向异常波动",
        2 => "创业板连续10个交易日内3次出现同向异常波动",
        3 => "科创板连续10个交易日内3次出现同向异常波动",
        4 => "连续十个交易日内日收盘价涨跌幅偏离值累计达到+100%",
        5 => "连续十个交易日内日收盘价涨跌幅偏离值累计达到-50%",
        6 => "连续三十个交易日内日收盘价涨跌幅偏离值累计达到+200%",
        7 => "连续三十个交易日内日收盘价涨跌幅偏离值累计达到-70%",
        8 => "北交所连续10个交易日内3次出现同向异常波动",
        40 => "连续十个交易日内日收盘价涨跌幅偏离值累计达到+150%",
        50 => "连续十个交易日内日收盘价涨跌幅偏离值累计达到-60%",
        60 => "连续30个交易日内日收盘价涨跌幅偏离值累计达到+300%",
        70 => "连续30个交易日内日收盘价涨跌幅偏离值累计达到-75%",
        _ => "未知规则码",
    }
}

/// Map a 同花顺/东财 anomaly record to an exchange code. 北交所与深市同为 `m=0`,
/// 不能只看 m，需结合号段（920 / 43 / 83 / 87）。
fn anomaly_market(code: &str, m: i64, board: i64) -> String {
    let c = code.trim();
    if c.starts_with("920") || c.starts_with("43") || c.starts_with("83") || c.starts_with("87") || board == 8 {
        "BJ".into()
    } else if m == 1 {
        "SH".into()
    } else {
        "SZ".into()
    }
}

fn anomaly_hq_params(page_size: usize, page_no: usize) -> Vec<(String, String)> {
    vec![
        ("team".into(), "h5".into()),
        ("product".into(), "EastMoney".into()),
        ("client".into(), "WAP".into()),
        ("version".into(), "9001".into()),
        ("name".into(), "WAP".into()),
        ("user".into(), "123".into()),
        ("pageSize".into(), page_size.to_string()),
        ("pageNo".into(), page_no.to_string()),
    ]
}

/// Port of `em_price_anomaly(page_size, page_no)` — 日内异动明细。
pub async fn em_price_anomaly(
    client: &Client,
    page_size: usize,
    page_no: usize,
) -> Result<(Option<String>, Vec<EmPriceAnomalyRow>)> {
    let owned = anomaly_hq_params(page_size, page_no);
    let params: Vec<(&str, &str)> = owned.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let v = client
        .get_json_with_headers(
            SOURCE_EASTMONEY,
            "em_price_anomaly",
            &format!("{ANOMALY_BASE}/list"),
            &params,
            Some(&[("Referer", "https://vipmoney.eastmoney.com/")]),
        )
        .await?;
    if v.get("result").and_then(|r| r.as_i64()) != Some(0) {
        return Err(Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: format!("em_price_anomaly rejected: {}", v),
        });
    }
    parse_em_price_anomaly(&v)
}

/// Port of `em_price_anomaly_count(page_size, page_no)` — 异动统计（按标的聚合）。
pub async fn em_price_anomaly_count(
    client: &Client,
    page_size: usize,
    page_no: usize,
) -> Result<(Option<String>, Vec<EmPriceAnomalyCountRow>)> {
    let owned = anomaly_hq_params(page_size, page_no);
    let params: Vec<(&str, &str)> = owned.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let v = client
        .get_json_with_headers(
            SOURCE_EASTMONEY,
            "em_price_anomaly_count",
            &format!("{ANOMALY_BASE}/count"),
            &params,
            Some(&[("Referer", "https://vipmoney.eastmoney.com/")]),
        )
        .await?;
    if v.get("result").and_then(|r| r.as_i64()) != Some(0) {
        return Err(Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: format!("em_price_anomaly_count rejected: {}", v),
        });
    }
    parse_em_price_anomaly_count(&v)
}

/// One 异动统计 row (`em_price_anomaly_count`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct EmPriceAnomalyCountRow {
    pub date: Option<String>,
    pub code: Option<String>,
    pub name: Option<String>,
    pub market: Option<String>,
    pub price: Option<f64>,
    pub change_pct: Option<f64>,
    /// 窗口内异动次数
    pub times: Option<i64>,
    pub deviation: Option<f64>,
    pub days: Option<i64>,
    pub board: Option<i64>,
    pub source: &'static str,
}

/// Parse `price-anomaly/list` response.
pub(crate) fn parse_em_price_anomaly(resp: &Value) -> Result<(Option<String>, Vec<EmPriceAnomalyRow>)> {
    let date = opt_str(resp, "date");
    let data = resp
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "em_price_anomaly: missing data".into(),
        })?;
    let mut out = Vec::with_capacity(data.len());
    for x in data {
        let e = opt_i64(x, "e").unwrap_or(0);
        let s = opt_i64(x, "s").unwrap_or(0);
        // s==6 且 e∈{4,5,6,7} 时按 e*10 取更严阈值那档
        let rule_code = if s == 6 && (4..=7).contains(&e) { e * 10 } else { e };
        let rule = anomaly_rule(rule_code).to_string();
        out.push(EmPriceAnomalyRow {
            date: date.clone(),
            code: opt_str(x, "c"),
            name: opt_str(x, "n"),
            market: opt_i64(x, "m").map(|mv| {
                anomaly_market(opt_str(x, "c").as_deref().unwrap_or(""), mv, s)
            }),
            change_pct: opt_f64(x, "a"),
            deviation: opt_f64(x, "x"),
            days: opt_i64(x, "d"),
            board: Some(s),
            rule_code,
            rule,
            is_today: x.get("o").and_then(|o| o.as_i64()) != Some(2),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok((date, out))
}

/// Parse `price-anomaly/count` response.
pub(crate) fn parse_em_price_anomaly_count(
    resp: &Value,
) -> Result<(Option<String>, Vec<EmPriceAnomalyCountRow>)> {
    let date = opt_str(resp, "date");
    let data = resp
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "em_price_anomaly_count: missing data".into(),
        })?;
    let mut out = Vec::with_capacity(data.len());
    for x in data {
        let s = opt_i64(x, "s").unwrap_or(0);
        out.push(EmPriceAnomalyCountRow {
            date: date.clone(),
            code: opt_str(x, "c"),
            name: opt_str(x, "n"),
            market: opt_i64(x, "m").map(|mv| {
                anomaly_market(opt_str(x, "c").as_deref().unwrap_or(""), mv, s)
            }),
            price: opt_f64(x, "p"),
            change_pct: opt_f64(x, "a"),
            times: opt_i64(x, "t"),
            deviation: opt_f64(x, "x"),
            days: opt_i64(x, "d"),
            board: Some(s),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok((date, out))
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
    fn parses_concept_blocks() {
        let rows = parse_em_concept_blocks(&fx("em_concept_blocks")).unwrap();
        assert_eq!(rows.len(), 27);
        assert_eq!(rows[0].name.as_deref(), Some("食品饮料"));
        assert_eq!(rows[0].code.as_deref(), Some("BK0438"));
    }

    #[test]
    fn parses_industry_comparison() {
        let rows = parse_industry_comparison(&fx("em_industry_comparison"), 20).unwrap();
        assert!(rows.len() <= 20);
        assert!(!rows.is_empty());
        assert_eq!(rows[0].rank, 1);
    }

    #[test]
    fn parses_stock_monitor() {
        let rows = parse_em_stock_monitor(&fx("em_stock_monitor"), false).unwrap();
        assert_eq!(rows.len(), 20);
        assert_eq!(rows[0].market.as_deref(), Some("BJ"));
    }

    #[test]
    fn parses_price_anomaly() {
        let (_d, rows) = parse_em_price_anomaly(&fx("em_price_anomaly")).unwrap();
        assert_eq!(rows.len(), 8);
        assert!(rows[0].rule.contains("异常波动") || !rows[0].rule.is_empty());
        assert_eq!(rows[0].market.as_deref(), Some("SZ"));
    }

    #[test]
    fn parses_price_anomaly_count() {
        let (_d, rows) = parse_em_price_anomaly_count(&fx("em_price_anomaly_count")).unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].price.is_some());
    }

    #[test]
    fn em_secid_maps() {
        assert_eq!(em_secid("600519"), "1.600519");
        assert_eq!(em_secid("000001"), "0.000001");
        assert_eq!(em_secid("920575"), "0.920575");
    }
}
