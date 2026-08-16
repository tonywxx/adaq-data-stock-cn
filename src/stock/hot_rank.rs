//! 东方财富个股人气榜 (Eastmoney stock popularity rank).
//!
//! Ports akshare `stock_hot_rank_em.py` (6 functions) and `stock_hot_up_em.py`
//! (飙升榜). Every endpoint is a JSON-body POST to
//! `emappdata.eastmoney.com/stockrank`; `stock_hot_rank_em` / `stock_hot_up_em`
//! additionally fetch realtime prices via a second Eastmoney `push2` GET.
//!
//! | Rust fn | akshare fn | source | akshare file:line |
//! |---|---|---|---|
//! | `stock_hot_rank_em` | `stock_hot_rank_em` | eastmoney | `akshare/stock/stock_hot_rank_em.py:16` |
//! | `stock_hot_up_em` | `stock_hot_up_em` | eastmoney | `akshare/stock/stock_hot_up_em.py:13` |
//! | `stock_hot_rank_detail_em` | `stock_hot_rank_detail_em` | eastmoney | `akshare/stock/stock_hot_rank_em.py:53` |
//! | `stock_hot_rank_detail_realtime_em` | `stock_hot_rank_detail_realtime_em` | eastmoney | `akshare/stock/stock_hot_rank_em.py:90` |
//! | `stock_hot_keyword_em` | `stock_hot_keyword_em` | eastmoney | `akshare/stock/stock_hot_rank_em.py:124` |
//! | `stock_hot_rank_latest_em` | `stock_hot_rank_latest_em` | eastmoney | `akshare/stock/stock_hot_rank_em.py:158` |
//! | `stock_hot_rank_relate_em` | `stock_hot_rank_relate_em` | eastmoney | `akshare/stock/stock_hot_rank_em.py:191` |
//! | `stock_hk_hot_rank_em` | `stock_hk_hot_rank_em` | eastmoney | `akshare/stock/stock_hk_hot_rank_em.py:16` |
//! | `stock_hk_hot_rank_detail_em` | `stock_hk_hot_rank_detail_em` | eastmoney | `akshare/stock/stock_hk_hot_rank_em.py:53` |
//! | `stock_hk_hot_rank_detail_realtime_em` | `stock_hk_hot_rank_detail_realtime_em` | eastmoney | `akshare/stock/stock_hk_hot_rank_em.py:90` |
//! | `stock_hk_hot_rank_latest_em` | `stock_hk_hot_rank_latest_em` | eastmoney | `akshare/stock/stock_hk_hot_rank_em.py:124` |
//!
//! All endpoints share a fixed `appId` / `globalId` (same as akshare). Raw
//! upstream values are stored as-is; `涨跌额` is derived as `最新价 * 涨跌幅 / 100`
//! to mirror akshare's computed column.

use std::collections::HashMap;

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

const EM_APPDATA: &str = "https://emappdata.eastmoney.com/stockrank";
const PUSH2: &str = "https://push2.eastmoney.com/api/qt/ulist.np/get";
const APP_ID: &str = "appId01";
const GLOBAL_ID: &str = "786e4c21-70dc-435a-93bb-38";

// ---------------------------------------------------------------------------
// Shared field helpers (per-module copies of the crate-wide convention)
// ---------------------------------------------------------------------------

/// Read a string-or-number field as `String` (missing → empty).
fn fstr(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

/// Read an optional string field (missing / null / empty → `None`).
fn str_of(v: Option<&Value>) -> Option<String> {
    match v {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

/// Read a numeric field (number, or comma-grouped string) as `f64`.
fn num_of(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => {
            let t = s.trim().replace(',', "");
            if t.is_empty() {
                None
            } else {
                t.parse::<f64>().ok()
            }
        }
        _ => None,
    }
}

/// Read a percentage field (`"12.34%"` or number) as a fraction (`0.1234`).
fn num_pct(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Number(n)) => n.as_f64().map(|x| x / 100.0),
        Some(Value::String(s)) => {
            let t = s.trim().trim_end_matches('%').replace(',', "");
            if t.is_empty() {
                None
            } else {
                t.parse::<f64>().ok().map(|x| x / 100.0)
            }
        }
        _ => None,
    }
}

/// Build the shared emappdata POST body (without endpoint-specific fields).
fn base_payload() -> Value {
    serde_json::json!({ "appId": APP_ID, "globalId": GLOBAL_ID, "marketType": "" })
}

/// Convert an emappdata security code (`SZ000665`) to a push2 `secid` (`0.000665`).
fn to_secid(sc: &str) -> Option<String> {
    if sc.len() < 2 {
        return None;
    }
    let market = if &sc[..2] == "SZ" { "0" } else { "1" };
    Some(format!("{market}.{}", &sc[2..]))
}

// ---------------------------------------------------------------------------
// 人气榜 / 飙升榜 (rank + realtime prices)
// ---------------------------------------------------------------------------

/// One row of the Eastmoney stock-popularity / surge ranking.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HotRankRow {
    /// 当前排名 (`rk`).
    pub rank: Option<f64>,
    /// 代码 (full `sc`, e.g. `SZ000665`).
    pub code: String,
    /// 股票名称 (`f14`).
    pub name: Option<String>,
    /// 最新价 (`f2`).
    pub price: Option<f64>,
    /// 涨跌额 (`最新价 * 涨跌幅 / 100`).
    pub change: Option<f64>,
    /// 涨跌幅 (`f3`).
    pub pct: Option<f64>,
    /// 排名较昨日变动 (`hrc`); only present for 飙升榜.
    pub rank_change: Option<f64>,
}

/// Fetch realtime prices for the ranked codes via a second `push2` GET.
async fn fetch_rank_prices(client: &Client, rank_arr: &[Value]) -> Result<Vec<Value>> {
    let mut secids = Vec::new();
    for item in rank_arr {
        if let Some(secid) = to_secid(&fstr(item.get("sc"))) {
            secids.push(secid);
        }
    }
    if secids.is_empty() {
        return Ok(Vec::new());
    }
    let joined = secids.join(",");
    let params = [
        ("ut", "f057cbcbce2a86e2866ab8877db1d059"),
        ("fltt", "2"),
        ("invt", "2"),
        ("fields", "f14,f3,f12,f2"),
        ("secids", joined.as_str()),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_hot_rank_em_prices",
            PUSH2,
            &params,
        )
        .await?;
    Ok(v.get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default())
}

/// Realtime price info looked up from the push2 `diff` rows.
struct PriceInfo {
    name: Option<String>,
    price: Option<f64>,
    pct: Option<f64>,
}

/// Parse a popularity/surge ranking, merged with the push2 price rows.
pub(crate) fn parse_hot_rank(rank_arr: &[Value], diff: &[Value], with_change: bool) -> Vec<HotRankRow> {
    let mut prices: HashMap<String, PriceInfo> = HashMap::new();
    for d in diff {
        let code = fstr(d.get("f12"));
        let name = str_of(d.get("f14"));
        let price = num_of(d.get("f2"));
        let pct = num_of(d.get("f3"));
        prices.insert(code, PriceInfo { name, price, pct });
    }
    let mut out = Vec::with_capacity(rank_arr.len());
    for item in rank_arr {
        let code = fstr(item.get("sc"));
        let six = if code.len() >= 2 { code[2..].to_string() } else { code.clone() };
        let (name, price, pct) = prices
            .get(&six)
            .map(|p| (p.name.clone(), p.price, p.pct))
            .unwrap_or((None, None, None));
        let rank = num_of(item.get("rk"));
        let rank_change = if with_change { num_of(item.get("hrc")) } else { None };
        let change = match (price, pct) {
            (Some(p), Some(c)) => Some(p * c / 100.0),
            _ => None,
        };
        out.push(HotRankRow {
            rank,
            code,
            name,
            price,
            change,
            pct,
            rank_change,
        });
    }
    out
}

/// Port of `stock_hot_rank_em()` — Eastmoney 个股人气榜.
pub async fn stock_hot_rank_em(client: &Client) -> Result<Vec<HotRankRow>> {
    let mut body = base_payload();
    body["pageNo"] = serde_json::json!(1);
    body["pageSize"] = serde_json::json!(100);
    let v = client
        .post_json(
            SOURCE_EASTMONEY,
            "stock_hot_rank_em",
            &format!("{EM_APPDATA}/getAllCurrentList"),
            &body,
            None,
        )
        .await?;
    let rank_arr = v
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data at stock_hot_rank_em".into(),
        })?;
    let diff = fetch_rank_prices(client, rank_arr).await?;
    Ok(parse_hot_rank(rank_arr, &diff, false))
}

/// Port of `stock_hot_up_em()` — Eastmoney 个股人气榜-飙升榜.
pub async fn stock_hot_up_em(client: &Client) -> Result<Vec<HotRankRow>> {
    let mut body = base_payload();
    body["pageNo"] = serde_json::json!(1);
    body["pageSize"] = serde_json::json!(100);
    let v = client
        .post_json(
            SOURCE_EASTMONEY,
            "stock_hot_up_em",
            &format!("{EM_APPDATA}/getAllHisRcList"),
            &body,
            None,
        )
        .await?;
    let rank_arr = v
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data at stock_hot_up_em".into(),
        })?;
    let diff = fetch_rank_prices(client, rank_arr).await?;
    Ok(parse_hot_rank(rank_arr, &diff, true))
}

// ---------------------------------------------------------------------------
// 历史趋势及粉丝特征 (detail)
// ---------------------------------------------------------------------------

/// One row of the historical rank + follower-profile trend for a stock.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HotDetailRow {
    /// 时间 (`date`).
    pub time: String,
    /// 排名 (`rk`).
    pub rank: Option<f64>,
    /// 证券代码 (the requested `symbol`).
    pub code: String,
    /// 新晋粉丝 (`newUidRate` as a fraction).
    pub new_fans: Option<f64>,
    /// 铁杆粉丝 (`oldUidRate` as a fraction).
    pub old_fans: Option<f64>,
}

/// Parse the detail (rank history merged with follower-profile history).
pub(crate) fn parse_hot_detail(rank_arr: &[Value], prof_arr: &[Value], symbol: &str) -> Vec<HotDetailRow> {
    let n = rank_arr.len().max(prof_arr.len());
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let r = rank_arr.get(i);
        let p = prof_arr.get(i);
        let time = r
            .and_then(|x| str_of(x.get("date")).or_else(|| str_of(x.get("time"))))
            .unwrap_or_default();
        let rank = r.and_then(|x| num_of(x.get("rk")));
        let new_fans = p.and_then(|x| num_pct(x.get("newUidRate")));
        let old_fans = p.and_then(|x| num_pct(x.get("oldUidRate")));
        out.push(HotDetailRow {
            time,
            rank,
            code: symbol.to_string(),
            new_fans,
            old_fans,
        });
    }
    out
}

/// Port of `stock_hot_rank_detail_em(symbol)` — Eastmoney 历史趋势及粉丝特征.
pub async fn stock_hot_rank_detail_em(client: &Client, symbol: &str) -> Result<Vec<HotDetailRow>> {
    let mut body = base_payload();
    body["srcSecurityCode"] = serde_json::json!(symbol);
    body["yearType"] = serde_json::json!("5");
    let v = client
        .post_json(
            SOURCE_EASTMONEY,
            "stock_hot_rank_detail_em",
            &format!("{EM_APPDATA}/getHisList"),
            &body,
            None,
        )
        .await?;
    let rank_arr = v
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data at stock_hot_rank_detail_em".into(),
        })?;
    let v2 = client
        .post_json(
            SOURCE_EASTMONEY,
            "stock_hot_rank_detail_em",
            &format!("{EM_APPDATA}/getHisProfileList"),
            &body,
            None,
        )
        .await?;
    let prof_arr = v2
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data at stock_hot_rank_detail_em(profile)".into(),
        })?;
    Ok(parse_hot_detail(rank_arr, prof_arr, symbol))
}

// ---------------------------------------------------------------------------
// 实时变动 / 热门关键词 / 最新排名 / 相关股票
// ---------------------------------------------------------------------------

/// One row of the realtime rank movement for a stock.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HotRealtimeRow {
    /// 时间 (`date`).
    pub time: String,
    /// 排名 (`rk`).
    pub rank: Option<f64>,
}

/// Port of `stock_hot_rank_detail_realtime_em(symbol)` — Eastmoney 实时变动.
pub async fn stock_hot_rank_detail_realtime_em(client: &Client, symbol: &str) -> Result<Vec<HotRealtimeRow>> {
    let mut body = base_payload();
    body["srcSecurityCode"] = serde_json::json!(symbol);
    let v = client
        .post_json(
            SOURCE_EASTMONEY,
            "stock_hot_rank_detail_realtime_em",
            &format!("{EM_APPDATA}/getCurrentList"),
            &body,
            None,
        )
        .await?;
    let arr = v
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data at stock_hot_rank_detail_realtime_em".into(),
        })?;
    Ok(arr
        .iter()
        .map(|item| HotRealtimeRow {
            time: str_of(item.get("date"))
                .or_else(|| str_of(item.get("time")))
                .unwrap_or_default(),
            rank: num_of(item.get("rk")),
        })
        .collect())
}

/// One row of the hot keywords (concepts) for a stock.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HotKeywordRow {
    /// 时间 (`date`).
    pub time: String,
    /// 股票代码 (`sc`).
    pub code: String,
    /// 概念名称 (`name`).
    pub concept_name: Option<String>,
    /// 概念代码 (`code`).
    pub concept_code: Option<String>,
    /// 热度 (`hot`).
    pub heat: Option<f64>,
}

/// Port of `stock_hot_keyword_em(symbol)` — Eastmoney 热门关键词.
pub async fn stock_hot_keyword_em(client: &Client, symbol: &str) -> Result<Vec<HotKeywordRow>> {
    let mut body = base_payload();
    body["srcSecurityCode"] = serde_json::json!(symbol);
    let v = client
        .post_json(
            SOURCE_EASTMONEY,
            "stock_hot_keyword_em",
            &format!("{EM_APPDATA}/getHotStockRankList"),
            &body,
            None,
        )
        .await?;
    let arr = v
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data at stock_hot_keyword_em".into(),
        })?;
    Ok(arr
        .iter()
        .map(|item| HotKeywordRow {
            time: str_of(item.get("date")).unwrap_or_default(),
            code: fstr(item.get("sc")),
            concept_name: str_of(item.get("name")),
            concept_code: str_of(item.get("code")),
            heat: num_of(item.get("hot")),
        })
        .collect())
}

/// One row of the latest rank snapshot for a stock.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HotLatestRow {
    /// 指标名 (dict key).
    pub item: String,
    /// 指标值 (raw).
    pub value: Option<String>,
}

/// Port of `stock_hot_rank_latest_em(symbol)` — Eastmoney 最新排名.
pub async fn stock_hot_rank_latest_em(client: &Client, symbol: &str) -> Result<Vec<HotLatestRow>> {
    let mut body = base_payload();
    body["srcSecurityCode"] = serde_json::json!(symbol);
    let v = client
        .post_json(
            SOURCE_EASTMONEY,
            "stock_hot_rank_latest_em",
            &format!("{EM_APPDATA}/getCurrentLatest"),
            &body,
            None,
        )
        .await?;
    let obj = v
        .get("data")
        .and_then(|d| d.as_object())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data at stock_hot_rank_latest_em".into(),
        })?;
    Ok(obj
        .iter()
        .map(|(k, val)| HotLatestRow {
            item: k.clone(),
            value: str_of(Some(val)),
        })
        .collect())
}

/// One row of a related stock for a given stock.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HotRelateRow {
    /// 时间 (`date`).
    pub time: String,
    /// 股票代码 (`sc`).
    pub code: String,
    /// 相关股票代码 (`relateSc`).
    pub related_code: String,
    /// 涨跌幅 (`pct`) as a fraction.
    pub pct: Option<f64>,
}

/// Port of `stock_hot_rank_relate_em(symbol)` — Eastmoney 相关股票.
pub async fn stock_hot_rank_relate_em(client: &Client, symbol: &str) -> Result<Vec<HotRelateRow>> {
    let mut body = base_payload();
    body["srcSecurityCode"] = serde_json::json!(symbol);
    let v = client
        .post_json(
            SOURCE_EASTMONEY,
            "stock_hot_rank_relate_em",
            &format!("{EM_APPDATA}/getFollowStockRankList"),
            &body,
            None,
        )
        .await?;
    let arr = v
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data at stock_hot_rank_relate_em".into(),
        })?;
    Ok(arr
        .iter()
        .map(|item| HotRelateRow {
            time: str_of(item.get("date")).unwrap_or_default(),
            code: fstr(item.get("sc")),
            related_code: fstr(item.get("relateSc")),
            pct: num_pct(item.get("pct")),
        })
        .collect())
}

// ---------------------------------------------------------------------------
// 港股个股人气榜 (HK market — emappdata POST with `marketType=000003`)
// ---------------------------------------------------------------------------

/// One row of the Eastmoney HK stock-popularity ranking.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HkHotRankRow {
    /// 当前排名 (`rk`).
    pub rank: Option<f64>,
    /// 代码 (`sc` with `HK|` stripped, e.g. `00700`).
    pub code: String,
    /// 股票名称 (`f14`, from the push2 realtime price GET).
    pub name: Option<String>,
    /// 最新价 (`f2`, from the push2 realtime price GET).
    pub price: Option<f64>,
    /// 涨跌幅 (`f3`, from the push2 realtime price GET).
    pub pct: Option<f64>,
}

/// Convert an emappdata HK security code (`HK|00700`) to a push2 `secid`
/// (`116.00700`) — HK uses the `116.` prefix (vs `0.`/`1.` for A-shares).
fn hk_to_secid(sc: &str) -> Option<String> {
    let rest = sc.strip_prefix("HK|")?;
    Some(format!("116.{rest}"))
}

/// Fetch realtime prices for the ranked HK codes via a second `push2` GET
/// (`secid` prefix `116.`, fields `f14,f3,f12,f2`).
async fn fetch_hk_rank_prices(client: &Client, rank_arr: &[Value]) -> Result<Vec<Value>> {
    let mut secids = Vec::new();
    for item in rank_arr {
        if let Some(secid) = hk_to_secid(&fstr(item.get("sc"))) {
            secids.push(secid);
        }
    }
    if secids.is_empty() {
        return Ok(Vec::new());
    }
    let secids_joined = secids.join(",");
    let params = [
        ("ut", "f057cbcbce2a86e2866ab8877db1d059"),
        ("fltt", "2"),
        ("invt", "2"),
        ("fields", "f14,f3,f12,f2"),
        ("secids", secids_joined.as_str()),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_hk_hot_rank_em_prices",
            PUSH2,
            &params,
        )
        .await?;
    Ok(v.get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default())
}

/// Parse the HK popularity ranking, merged with the push2 price rows.
pub(crate) fn parse_hk_hot_rank(rank_arr: &[Value], diff: &[Value]) -> Vec<HkHotRankRow> {
    let mut prices: HashMap<String, PriceInfo> = HashMap::new();
    for d in diff {
        let code = fstr(d.get("f12"));
        prices.insert(
            code,
            PriceInfo {
                name: str_of(d.get("f14")),
                price: num_of(d.get("f2")),
                pct: num_of(d.get("f3")),
            },
        );
    }
    let mut out = Vec::with_capacity(rank_arr.len());
    for item in rank_arr {
        let sc = fstr(item.get("sc"));
        let code = sc.split('|').nth(1).unwrap_or(&sc).to_string();
        let p = prices.get(&code);
        let (name, price, pct) = p
            .map(|x| (x.name.clone(), x.price, x.pct))
            .unwrap_or((None, None, None));
        out.push(HkHotRankRow {
            rank: num_of(item.get("rk")),
            code,
            name,
            price,
            pct,
        });
    }
    out
}

/// Port of `stock_hk_hot_rank_em()` — Eastmoney 港股个股人气榜.
pub async fn stock_hk_hot_rank_em(client: &Client) -> Result<Vec<HkHotRankRow>> {
    let mut body = base_payload();
    body["marketType"] = serde_json::json!("000003");
    body["pageNo"] = serde_json::json!(1);
    body["pageSize"] = serde_json::json!(100);
    let v = client
        .post_json(
            SOURCE_EASTMONEY,
            "stock_hk_hot_rank_em",
            &format!("{EM_APPDATA}/getAllCurrHkUsList"),
            &body,
            None,
        )
        .await?;
    let rank_arr = v
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data at stock_hk_hot_rank_em".into(),
        })?;
    let diff = fetch_hk_rank_prices(client, rank_arr).await?;
    Ok(parse_hk_hot_rank(rank_arr, &diff))
}

/// One row of the HK historical rank trend for a stock.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HkHotDetailRow {
    /// 时间 (`date`/`time`).
    pub time: String,
    /// 排名 (`rk`).
    pub rank: Option<f64>,
    /// 证券代码 (the requested `symbol`).
    pub code: String,
}

/// Port of `stock_hk_hot_rank_detail_em(symbol)` — Eastmoney 港股历史趋势.
pub async fn stock_hk_hot_rank_detail_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<HkHotDetailRow>> {
    let mut body = base_payload();
    body["marketType"] = serde_json::json!("000003");
    body["srcSecurityCode"] = serde_json::json!(format!("HK|{symbol}"));
    let v = client
        .post_json(
            SOURCE_EASTMONEY,
            "stock_hk_hot_rank_detail_em",
            &format!("{EM_APPDATA}/getHisHkUsList"),
            &body,
            None,
        )
        .await?;
    let arr = v
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data at stock_hk_hot_rank_detail_em".into(),
        })?;
    Ok(arr
        .iter()
        .map(|item| HkHotDetailRow {
            time: str_of(item.get("date"))
                .or_else(|| str_of(item.get("time")))
                .unwrap_or_default(),
            rank: num_of(item.get("rk")),
            code: symbol.to_string(),
        })
        .collect())
}

/// One row of the HK realtime rank movement for a stock.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HkHotRealtimeRow {
    /// 时间 (`date`/`time`).
    pub time: String,
    /// 排名 (`rk`).
    pub rank: Option<f64>,
}

/// Port of `stock_hk_hot_rank_detail_realtime_em(symbol)` — Eastmoney 港股实时变动.
pub async fn stock_hk_hot_rank_detail_realtime_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<HkHotRealtimeRow>> {
    let mut body = base_payload();
    body["marketType"] = serde_json::json!("000003");
    body["srcSecurityCode"] = serde_json::json!(format!("HK|{symbol}"));
    let v = client
        .post_json(
            SOURCE_EASTMONEY,
            "stock_hk_hot_rank_detail_realtime_em",
            &format!("{EM_APPDATA}/getCurrentHkUsList"),
            &body,
            None,
        )
        .await?;
    let arr = v
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data at stock_hk_hot_rank_detail_realtime_em".into(),
        })?;
    Ok(arr
        .iter()
        .map(|item| HkHotRealtimeRow {
            time: str_of(item.get("date"))
                .or_else(|| str_of(item.get("time")))
                .unwrap_or_default(),
            rank: num_of(item.get("rk")),
        })
        .collect())
}

/// One row of the HK latest rank snapshot for a stock.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HkHotLatestRow {
    /// 指标名 (dict key).
    pub item: String,
    /// 指标值 (raw).
    pub value: Option<String>,
}

/// Port of `stock_hk_hot_rank_latest_em(symbol)` — Eastmoney 港股最新排名.
pub async fn stock_hk_hot_rank_latest_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<HkHotLatestRow>> {
    let mut body = base_payload();
    body["marketType"] = serde_json::json!("000003");
    body["srcSecurityCode"] = serde_json::json!(format!("HK|{symbol}"));
    let v = client
        .post_json(
            SOURCE_EASTMONEY,
            "stock_hk_hot_rank_latest_em",
            &format!("{EM_APPDATA}/getCurrentHkUsLatest"),
            &body,
            None,
        )
        .await?;
    let obj = v
        .get("data")
        .and_then(|d| d.as_object())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data at stock_hk_hot_rank_latest_em".into(),
        })?;
    Ok(obj
        .iter()
        .map(|(k, val)| HkHotLatestRow {
            item: k.clone(),
            value: str_of(Some(val)),
        })
        .collect())
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let p = p.join("tests/fixtures").join(name);
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    /// Approximate float comparison for `Option<f64>` fields (never `.unwrap()`).
    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    #[test]
    fn parses_hot_rank_em() {
        let rank = fixture("stock_hot_rank_em.json")
            .get("data")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        let diff = fixture("stock_hot_rank_em_diff.json")
            .get("data")
            .unwrap()
            .get("diff")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        let rows = parse_hot_rank(&rank, &diff, false);
        assert_eq!(rows.len(), 2);
        assert!(approx(rows[0].rank, 1.0));
        assert_eq!(rows[0].code, "SZ000665");
        assert_eq!(rows[0].name.as_deref(), Some("湖北广电"));
        assert!(approx(rows[0].price, 12.34));
        assert!(approx(rows[0].pct, 1.5));
        assert!(approx(rows[0].change, 12.34 * 1.5 / 100.0));
        assert_eq!(rows[0].rank_change, None);
        assert_eq!(rows[1].code, "SH600000");
        assert_eq!(rows[1].name.as_deref(), Some("浦发银行"));
    }

    #[test]
    fn parses_hot_up_em() {
        let rank = fixture("stock_hot_up_em.json")
            .get("data")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        let diff = fixture("stock_hot_up_em_diff.json")
            .get("data")
            .unwrap()
            .get("diff")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        let rows = parse_hot_rank(&rank, &diff, true);
        assert_eq!(rows.len(), 1);
        assert!(approx(rows[0].rank, 3.0));
        assert!(approx(rows[0].rank_change, 5.0));
    }

    #[test]
    fn parses_hot_detail() {
        let rank = fixture("stock_hot_rank_detail_em.json")
            .get("data")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        let prof = fixture("stock_hot_rank_detail_em_profile.json")
            .get("data")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        let rows = parse_hot_detail(&rank, &prof, "SZ000665");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "SZ000665");
        assert_eq!(rows[0].time, "2024-01-01");
        assert!(approx(rows[0].rank, 10.0));
        assert!(approx(rows[0].new_fans, 0.1234));
        assert!(approx(rows[0].old_fans, 0.8766));
        assert!(approx(rows[1].rank, 8.0));
    }

    #[test]
    fn parses_hot_rank_detail_realtime() {
        let arr = fixture("stock_hot_rank_detail_realtime_em.json")
            .get("data")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        let rows: Vec<HotRealtimeRow> = arr
            .iter()
            .map(|item| HotRealtimeRow {
                time: str_of(item.get("date"))
                    .or_else(|| str_of(item.get("time")))
                    .unwrap_or_default(),
                rank: num_of(item.get("rk")),
            })
            .collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].time, "2024-01-01 09:30");
        assert!(approx(rows[0].rank, 10.0));
        assert!(approx(rows[1].rank, 9.0));
    }

    #[test]
    fn parses_hot_keyword() {
        let arr = fixture("stock_hot_keyword_em.json")
            .get("data")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        let rows: Vec<HotKeywordRow> = arr
            .iter()
            .map(|item| HotKeywordRow {
                time: str_of(item.get("date")).unwrap_or_default(),
                code: fstr(item.get("sc")),
                concept_name: str_of(item.get("name")),
                concept_code: str_of(item.get("code")),
                heat: num_of(item.get("hot")),
            })
            .collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "000665");
        assert_eq!(rows[0].concept_name.as_deref(), Some("人工智能"));
        assert_eq!(rows[0].concept_code.as_deref(), Some("BK0800"));
        assert!(approx(rows[0].heat, 12345.0));
    }

    #[test]
    fn parses_hot_rank_latest() {
        let obj = fixture("stock_hot_rank_latest_em.json")
            .get("data")
            .unwrap()
            .as_object()
            .unwrap()
            .clone();
        let rows: Vec<HotLatestRow> = obj
            .iter()
            .map(|(k, val)| HotLatestRow {
                item: k.clone(),
                value: str_of(Some(val)),
            })
            .collect();
        assert_eq!(rows.len(), 3);
        let rank_row = rows.iter().find(|r| r.item == "排名").unwrap();
        assert_eq!(rank_row.value.as_deref(), Some("5"));
        let time_row = rows.iter().find(|r| r.item == "时间").unwrap();
        assert_eq!(time_row.value.as_deref(), Some("2024-01-01 09:30"));
    }

    #[test]
    fn parses_hot_rank_relate() {
        let arr = fixture("stock_hot_rank_relate_em.json")
            .get("data")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        let rows: Vec<HotRelateRow> = arr
            .iter()
            .map(|item| HotRelateRow {
                time: str_of(item.get("date")).unwrap_or_default(),
                code: fstr(item.get("sc")),
                related_code: fstr(item.get("relateSc")),
                pct: num_pct(item.get("pct")),
            })
            .collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "000665");
        assert_eq!(rows[0].related_code, "000001");
        assert!(approx(rows[0].pct, 0.0123));
    }

    // ---- HK 个股人气榜 (emappdata POST, marketType=000003) ----

    #[test]
    fn parses_hk_hot_rank_em() {
        let rank = fixture("stock_hk_hot_rank_em.json")
            .get("data")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        let diff = fixture("stock_hk_hot_rank_em_diff.json")
            .get("data")
            .unwrap()
            .get("diff")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        let rows = parse_hk_hot_rank(&rank, &diff);
        assert_eq!(rows.len(), 2);
        assert!(approx(rows[0].rank, 1.0));
        assert_eq!(rows[0].code, "00700");
        assert_eq!(rows[0].name.as_deref(), Some("腾讯控股"));
        assert!(approx(rows[0].price, 380.0));
        assert!(approx(rows[0].pct, 1.25));
        assert_eq!(rows[1].code, "09988");
        assert_eq!(rows[1].name.as_deref(), Some("阿里巴巴-SW"));
    }

    #[test]
    fn parses_hk_hot_rank_detail() {
        let arr = fixture("stock_hk_hot_rank_detail_em.json")
            .get("data")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        let rows: Vec<HkHotDetailRow> = arr
            .iter()
            .map(|item| HkHotDetailRow {
                time: str_of(item.get("date"))
                    .or_else(|| str_of(item.get("time")))
                    .unwrap_or_default(),
                rank: num_of(item.get("rk")),
                code: "00700".to_string(),
            })
            .collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "00700");
        assert_eq!(rows[0].time, "2024-01-01");
        assert!(approx(rows[0].rank, 10.0));
        assert!(approx(rows[1].rank, 8.0));
    }

    #[test]
    fn parses_hk_hot_rank_detail_realtime() {
        let arr = fixture("stock_hk_hot_rank_detail_realtime_em.json")
            .get("data")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        let rows: Vec<HkHotRealtimeRow> = arr
            .iter()
            .map(|item| HkHotRealtimeRow {
                time: str_of(item.get("date"))
                    .or_else(|| str_of(item.get("time")))
                    .unwrap_or_default(),
                rank: num_of(item.get("rk")),
            })
            .collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].time, "2024-01-01 09:30");
        assert!(approx(rows[0].rank, 10.0));
        assert!(approx(rows[1].rank, 9.0));
    }

    #[test]
    fn parses_hk_hot_rank_latest() {
        let obj = fixture("stock_hk_hot_rank_latest_em.json")
            .get("data")
            .unwrap()
            .as_object()
            .unwrap()
            .clone();
        let rows: Vec<HkHotLatestRow> = obj
            .iter()
            .map(|(k, val)| HkHotLatestRow {
                item: k.clone(),
                value: str_of(Some(val)),
            })
            .collect();
        assert_eq!(rows.len(), 3);
        let rank_row = rows.iter().find(|r| r.item == "排名").unwrap();
        assert_eq!(rank_row.value.as_deref(), Some("5"));
        let time_row = rows.iter().find(|r| r.item == "时间").unwrap();
        assert_eq!(time_row.value.as_deref(), Some("2024-01-01 09:30"));
    }
}
