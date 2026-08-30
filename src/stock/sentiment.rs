//! Market-heat / sentiment endpoints — ports `ths_hot_list`, `em_hot_rank`,
//! `em_hot_concept` from the `simonlin1212/a-stock-data` skill.
//!
//! - 同花顺热榜 (`ths_hot_list`): single GET to 10jqka.
//! - 东财人气榜 (`em_hot_rank`): Eastmoney `getAllCurrentList` returns only
//!   prefixed codes, so names/prices are fetched in a second call to
//!   `push2.eastmoney.com/api/qt/ulist.np/get` and merged back by code.
//! - 东财个股概念命中 (`em_hot_concept`): Eastmoney `getHotStockRankList`.

use std::collections::HashMap;

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use crate::core::json::{opt_f64, opt_i64, opt_str};

/// `(name, price, pct)` keyed by 6-digit code, returned by `ulist.np`.
type NameEntry = (Option<String>, Option<f64>, Option<f64>);

const SOURCE_THS: &str = "ths";
const SOURCE_EASTMONEY: &str = "eastmoney";
const THS_HOT_URL: &str =
    "https://dq.10jqka.com.cn/fuyao/hot_list_data/out/hot_list/v1/stock";
const EM_RANK_URL: &str = "https://emappdata.eastmoney.com/stockrank/getAllCurrentList";
const EM_CONCEPT_URL: &str =
    "https://emappdata.eastmoney.com/stockrank/getHotStockRankList";
const EM_ULIST_URL: &str = "https://push2.eastmoney.com/api/qt/ulist.np/get";
const EM_APP_ID: &str = "appId01";
const EM_GLOBAL_ID: &str = "786e4c21-70dc-435a-93bb-38";

/// One row of the 同花顺 hot list.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ThsHotRow {
    pub rank: Option<i64>,
    pub code: Option<String>,
    pub name: Option<String>,
    /// 人气值 (`rate`)
    pub heat: Option<f64>,
    /// 涨跌幅% (`rise_and_fall`)
    pub pct: Option<f64>,
    /// 排名变化 (`hot_rank_chg`)
    pub rank_chg: Option<i64>,
    /// 概念标签 (`tag.concept_tag`)
    pub concepts: Vec<String>,
    /// 人气标签 (`tag.popularity_tag`)
    pub tag: Option<String>,
    pub source: &'static str,
}

/// One row of the 东财 个股人气榜 (merged with name/price from `ulist.np`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct EmHotRankRow {
    pub rank: i64,
    /// 6-digit code (prefix stripped)
    pub code: String,
    pub name: Option<String>,
    pub price: Option<f64>,
    pub pct: Option<f64>,
    /// 排名变化 (`hisRc`)
    pub rank_chg: Option<i64>,
    pub source: &'static str,
}

/// One 东财 个股热门概念命中 row.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EmHotConceptRow {
    pub calc_time: Option<String>,
    /// e.g. `SH600519`
    pub security_code: Option<String>,
    pub concept_name: Option<String>,
    pub concept_id: Option<String>,
    /// 命中热度 (`hitCount`)
    pub hit_count: Option<i64>,
    pub source: &'static str,
}

/// Port of `ths_hot_list(period)`. `period` is `"hour"` (default) or `"day"`.
pub async fn ths_hot_list(client: &Client, period: &str) -> Result<Vec<ThsHotRow>> {
    let v = client
        .get_json(
            SOURCE_THS,
            "ths_hot_list",
            THS_HOT_URL,
            &[("stock_type", "a"), ("type", period), ("list_type", "normal")],
        )
        .await?;
    parse_ths_hot_list(&v)
}

/// Port of `em_hot_rank(top)`. `top` is the page size (rank list length).
///
/// Two sequential calls: `getAllCurrentList` for ranks + codes, then
/// `ulist.np` for names/prices, merged back by code.
pub async fn em_hot_rank(client: &Client, top: u32) -> Result<Vec<EmHotRankRow>> {
    let body = serde_json::json!({
        "appId": EM_APP_ID,
        "globalId": EM_GLOBAL_ID,
        "marketType": "",
        "pageNo": 1,
        "pageSize": top,
    });
    let rank_v = client
        .post_json(SOURCE_EASTMONEY, "em_hot_rank", EM_RANK_URL, &body, None)
        .await?;
    let tuples = parse_em_hot_rank_tuples(&rank_v)?;
    if tuples.is_empty() {
        return Ok(Vec::new());
    }

    // Build Eastmoney secids: SZ -> 0.xxxxxx, SH/BJ -> 1.xxxxxx.
    let secids: Vec<String> = tuples
        .iter()
        .map(|(sc, _, _)| {
            let (market, code) = sc.split_at(2);
            let market = if market.eq_ignore_ascii_case("SZ") {
                "0"
            } else {
                "1"
            };
            format!("{market}.{code}")
        })
        .collect();
    let secids = secids.join(",");

    let names_v = client
        .get_json_with_headers(
            SOURCE_EASTMONEY,
            "em_hot_rank",
            EM_ULIST_URL,
            &[
                ("ut", "f057cbcbce2a86e2866ab8877db1d059"),
                ("fltt", "2"),
                ("invt", "2"),
                ("fields", "f14,f3,f12,f2"),
                ("secids", secids.as_str()),
            ],
            Some(&[("Referer", "https://quote.eastmoney.com/")]),
        )
        .await?;
    let names = parse_em_hot_rank_names(&names_v)?;

    let mut out = Vec::with_capacity(tuples.len());
    for (sc, rk, his_rc) in tuples {
        let code = sc[2..].to_string();
        let (name, price, pct) = names.get(&code).cloned().unwrap_or((None, None, None));
        out.push(EmHotRankRow {
            rank: rk,
            code,
            name,
            price,
            pct,
            rank_chg: his_rc,
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

/// Port of `em_hot_concept(code)`. `code` is a 6-digit A-share code; the market
/// prefix (`SH`/`SZ`/`BJ`) is derived automatically for the upstream request.
pub async fn em_hot_concept(client: &Client, code: &str) -> Result<Vec<EmHotConceptRow>> {
    let prefix = em_prefix(code).to_ascii_uppercase();
    let src = format!("{prefix}{code}");
    let body = serde_json::json!({
        "appId": EM_APP_ID,
        "globalId": EM_GLOBAL_ID,
        "srcSecurityCode": src,
    });
    let v = client
        .post_json(SOURCE_EASTMONEY, "em_hot_concept", EM_CONCEPT_URL, &body, None)
        .await?;
    parse_em_hot_concept(&v)
}

/// Derive the Eastmoney-style market prefix for a 6-digit A-share code.
///
/// Mirrors the skill's `get_prefix`: 5/6/9* → `sh`, 0/3* → `sz`, 4/8*/92* → `bj`.
/// Explicit `sh`/`sz`/`bj` prefixes on the input are honored verbatim.
fn em_prefix(code: &str) -> &'static str {
    let c = code.to_ascii_lowercase();
    let prefixed = |p: &str| {
        c.strip_prefix(p)
            .is_some_and(|r| r.chars().all(|x| x.is_ascii_digit()))
    };
    if prefixed("sh") {
        return "sh";
    }
    if prefixed("sz") {
        return "sz";
    }
    if prefixed("bj") {
        return "bj";
    }
    if c.starts_with("92") || c.starts_with("4") || c.starts_with("8") {
        "bj"
    } else if c.starts_with('5') || c.starts_with('6') || c.starts_with('9') {
        "sh"
    } else {
        "sz"
    }
}

/// Parse `getAllCurrentList` into the raw `(sc, rk, hisRc)` tuples.
fn parse_em_hot_rank_tuples(resp: &Value) -> Result<Vec<(String, i64, Option<i64>)>> {
    let data = resp
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "em_hot_rank: missing data".into(),
        })?;
    let mut out = Vec::with_capacity(data.len());
    for it in data {
        let sc = opt_str(it, "sc").ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "em_hot_rank: missing sc".into(),
        })?;
        let rk = opt_i64(it, "rk").unwrap_or(0);
        let his_rc = opt_i64(it, "hisRc");
        out.push((sc, rk, his_rc));
    }
    Ok(out)
}

/// Parse `ulist.np` names into a map keyed by 6-digit code → (name, price, pct).
pub(crate) fn parse_em_hot_rank_names(resp: &Value) -> Result<HashMap<String, NameEntry>> {
    let diff = resp
        .get("data")
        .and_then(|d| d.get("diff"))
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "em_hot_rank names: missing data.diff".into(),
        })?;
    // `diff` may be an array OR an object keyed by index (normalize both).
    let items: Vec<&Value> = match diff {
        Value::Array(a) => a.iter().collect(),
        Value::Object(o) => o.values().collect(),
        _ => {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "em_hot_rank names: data.diff not array/object".into(),
            })
        }
    };
    let mut map = std::collections::HashMap::with_capacity(items.len());
    for it in items {
        let code = match opt_str(it, "f12") {
            Some(c) => c,
            None => continue,
        };
        map.insert(
            code,
            (opt_str(it, "f14"), opt_f64(it, "f2"), opt_f64(it, "f3")),
        );
    }
    Ok(map)
}

/// Parse the 同花顺 hot list envelope.
pub(crate) fn parse_ths_hot_list(resp: &Value) -> Result<Vec<ThsHotRow>> {
    let list = resp
        .get("data")
        .and_then(|d| d.get("stock_list"))
        .and_then(|s| s.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_THS,
            message: "ths_hot_list: missing data.stock_list".into(),
        })?;
    let mut out = Vec::with_capacity(list.len());
    for it in list {
        let tag = it.get("tag");
        let concepts = tag
            .and_then(|t| t.get("concept_tag"))
            .and_then(|c| c.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let tag_str = tag
            .and_then(|t| t.get("popularity_tag"))
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());
        out.push(ThsHotRow {
            rank: opt_i64(it, "order"),
            code: opt_str(it, "code"),
            name: opt_str(it, "name"),
            heat: opt_f64(it, "rate"),
            pct: opt_f64(it, "rise_and_fall"),
            rank_chg: opt_i64(it, "hot_rank_chg"),
            concepts,
            tag: tag_str,
            source: SOURCE_THS,
        });
    }
    Ok(out)
}

/// Parse the 东财 `getHotStockRankList` envelope.
pub(crate) fn parse_em_hot_concept(resp: &Value) -> Result<Vec<EmHotConceptRow>> {
    let data = resp
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "em_hot_concept: missing data".into(),
        })?;
    let mut out = Vec::with_capacity(data.len());
    for it in data {
        out.push(EmHotConceptRow {
            calc_time: opt_str(it, "calcTime"),
            security_code: opt_str(it, "srcSecurityCode"),
            concept_name: opt_str(it, "conceptName"),
            concept_id: opt_str(it, "conceptId"),
            hit_count: opt_i64(it, "hitCount"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!("tests/fixtures/{name}.json"));
        let txt = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    #[test]
    fn parses_ths_hot_list_fixture() {
        let rows = parse_ths_hot_list(&fixture("ths_hot_list")).unwrap();
        assert_eq!(rows.len(), 100);
        let r0 = &rows[0];
        assert_eq!(r0.code.as_deref(), Some("000560"));
        assert_eq!(r0.name.as_deref(), Some("我爱我家"));
        assert!(r0.heat.is_some());
        assert!(!r0.concepts.is_empty());
        assert!(r0.tag.is_some());
    }

    #[test]
    fn parses_em_hot_rank_fixture() {
        let tuples = parse_em_hot_rank_tuples(&fixture("em_hot_rank")).unwrap();
        assert_eq!(tuples.len(), 5);
        assert_eq!(tuples[0].0, "SZ000560");
        assert_eq!(tuples[0].1, 1);
    }

    #[test]
    fn parses_em_hot_rank_names_fixture() {
        let names = parse_em_hot_rank_names(&fixture("em_hot_rank_names")).unwrap();
        assert_eq!(names.get("000560").map(|n| n.0.clone()), Some(Some("我爱我家".into())));
        assert!(names.get("600162").is_some());
        assert_eq!(names.get("600162").map(|n| n.0.clone()), Some(Some("香江控股".into())));
    }

    #[test]
    fn parses_em_hot_concept_fixture() {
        let rows = parse_em_hot_concept(&fixture("em_hot_concept")).unwrap();
        assert_eq!(rows.len(), 9);
        assert_eq!(rows[0].concept_name.as_deref(), Some("白酒"));
        assert_eq!(rows[0].concept_id.as_deref(), Some("BK0896"));
        assert_eq!(rows[0].hit_count, Some(9620));
    }

    #[test]
    fn em_prefix_rules() {
        assert_eq!(em_prefix("600519"), "sh");
        assert_eq!(em_prefix("000001"), "sz");
        assert_eq!(em_prefix("920001"), "bj");
        assert_eq!(em_prefix("sh600519"), "sh");
    }
}
