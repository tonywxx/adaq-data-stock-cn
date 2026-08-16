use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_GASGOO: &str = "gasgoo";
const SOURCE_TAPTAP: &str = "taptap";

// ---------------------------------------------------------------------------
// car_sale_rank_gasgoo — 盖世汽车-销量排行榜
// http://i.gasgoo.com/data/ranking
//
// NOTE: the upstream wraps its payload in a `d` field as a JSON *string* that
// must be decoded a second time (akshare uses `demjson.decode`). We replicate
// that double-decode. The inner record schema is undocumented/动态, so rows
// capture `raw` JSON plus best-effort `rank`/`name`/`sales` fields.
// ---------------------------------------------------------------------------

const GASGOO_URL: &str = "https://i.gasgoo.com/data/sales/AutoModelSalesRank.aspx/GetSalesRank";
const GASGOO_SYMBOL_MAP: &[(&str, &str)] = &[
    ("车型榜", "M"),
    ("车企榜", "F"),
    ("品牌榜", "B"),
];

/// 汽车销量排行行 (`car_sale_rank_gasgoo`). Inner schema is dynamic; `raw`
/// holds the full decoded record.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CarSaleRankRow {
    pub rank: Option<String>,
    pub name: Option<String>,
    pub sales: Option<f64>,
    pub raw: String,
}

/// 汽车销量排行 from Gasgoo (`car_sale_rank_gasgoo`).
///
/// `symbol` is one of `车型榜`/`车企榜`/`品牌榜`; `date` is `YYYYMM`.
pub async fn car_sale_rank_gasgoo(
    client: &Client,
    symbol: &str,
    date: &str,
) -> Result<Vec<CarSaleRankRow>> {
    let rank_type = map_lookup(GASGOO_SYMBOL_MAP, symbol, "symbol")?;
    if date.len() != 6 || !date.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::InvalidParam(format!("invalid date (need YYYYMM): {date}")));
    }
    let year = &date[..4];
    let month = date[4..].trim_start_matches('0');
    let body = serde_json::json!({
        "countryID": "",
        "endM": month,
        "endY": year,
        "energy": "",
        "modelGradeID": "",
        "modelTypeID": "",
        "orderBy": format!("{year}-{month}"),
        "queryDate": format!("{year}-{month}"),
        "rankType": rank_type,
        "startY": year,
        "startM": month,
    });
    let v = client
        .post_json(SOURCE_GASGOO, "car_sale_rank_gasgoo", GASGOO_URL, &body, None)
        .await?;
    parse_car_sale_rank_gasgoo(&v)
}

pub(crate) fn parse_car_sale_rank_gasgoo(resp: &Value) -> Result<Vec<CarSaleRankRow>> {
    let d = resp
        .get("d")
        .and_then(|x| x.as_str())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_GASGOO,
            message: "missing d".into(),
        })?;
    let inner: Value = serde_json::from_str(d).map_err(Error::Json)?;
    let list = inner.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_GASGOO,
        message: "decoded d is not an array".into(),
    })?;
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        out.push(CarSaleRankRow {
            rank: pick_opt_string(item, &["rank", "Rank"]),
            name: pick_opt_string(item, &["name", "Name", "modelName", "ModelName", "brandName", "BrandName"]),
            sales: pick_opt_num(item, &["sales", "SaleNum", "saleNum"]),
            raw: serde_json::to_string(item).unwrap_or_default(),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// game_hot_rank_taptap — TapTap 游戏榜单
// https://www.taptap.cn/top/played
// ---------------------------------------------------------------------------

const TAPTAP_BASE_URL: &str = "https://www.taptap.cn/webapiv2/app-top/v2/hits";
const TAPTAP_X_UA: &str = "V=1&PN=WebM&LANG=zh_CN&VN_CODE=102&LOC=CN&PLT=iOS&DS=Android\
&UID=12f0a48b-bd25-4dce-9d50-27924e83da1d&OS=iOS&OSV=18.5";
const TAPTAP_UA: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) \
AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1";
const TAPTAP_RANK_MAP: &[(&str, &str)] = &[
    ("热玩榜", "pop"),
    ("热门榜", "hot"),
    ("新品榜", "new"),
    ("预约榜", "reserve"),
    ("热卖榜", "sell"),
];

/// TapTap 游戏榜单行 (`game_hot_rank_taptap`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct GameHotRankRow {
    pub rank: u32,
    pub game_id: Option<String>,
    pub game_name: String,
    pub icon_url: String,
    pub rating: Option<f64>,
    pub hits_total: Option<f64>,
    pub play_total: Option<f64>,
    pub review_count: Option<f64>,
    pub fans_count: Option<f64>,
    pub rec_text: String,
    pub released_time: Option<i64>,
    pub tags: String,
    pub description: String,
    pub source: &'static str,
}

/// TapTap 游戏榜单 (`game_hot_rank_taptap`). `symbol` is a 榜单类型 label.
pub async fn game_hot_rank_taptap(client: &Client, symbol: &str) -> Result<Vec<GameHotRankRow>> {
    let type_name = map_lookup(TAPTAP_RANK_MAP, symbol, "symbol")?;
    let headers = [
        ("User-Agent", TAPTAP_UA),
        ("Referer", "https://www.taptap.cn/"),
        ("Accept", "application/json, text/plain, */*"),
    ];
    let mut offset: u32 = 0;
    let mut out: Vec<GameHotRankRow> = Vec::new();
    let mut total: Option<u64> = None;
    loop {
        let from_s = offset.to_string();
        let limit_s = "10".to_string();
        let params = [
            ("from", from_s.as_str()),
            ("limit", limit_s.as_str()),
            ("type_name", type_name.as_str()),
            ("X-UA", TAPTAP_X_UA),
        ];
        let v = client
            .get_json_with_headers(
                SOURCE_TAPTAP,
                "game_hot_rank_taptap",
                TAPTAP_BASE_URL,
                &params,
                Some(&headers),
            )
            .await?;
        if !v.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_TAPTAP,
                message: "taptap returned success=false".into(),
            });
        }
        let data = v.get("data").and_then(|d| d.as_object()).ok_or_else(|| {
            Error::UpstreamChanged {
                origin: SOURCE_TAPTAP,
                message: "missing data".into(),
            }
        })?;
        if total.is_none() {
            total = data.get("total").and_then(|t| t.as_u64());
        }
        let list = data
            .get("list")
            .and_then(|l| l.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_TAPTAP,
                message: "missing data.list".into(),
            })?;
        if list.is_empty() {
            break;
        }
        for item in list {
            let rank = (out.len() as u32) + 1;
            out.push(parse_taptap_item(item, rank));
        }
        if let Some(t) = total
            && (out.len() as u64) >= t {
                break;
            }
        offset += 10;
    }
    Ok(out)
}

pub(crate) fn parse_taptap_item(item: &Value, rank: u32) -> GameHotRankRow {
    let app = item.get("app");
    let stat = app.and_then(|a| a.get("stat"));
    let rating = stat
        .and_then(|s| s.get("rating"))
        .and_then(|r| r.get("score"))
        .and_then(num_val);
    let tags = app
        .and_then(|a| a.get("tags"))
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t.get("value").and_then(|v| v.as_str()))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let description = app
        .and_then(|a| a.get("description"))
        .and_then(|d| d.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or_default()
        .to_string();
    GameHotRankRow {
        rank,
        game_id: app.and_then(|a| a.get("id")).and_then(to_opt_string),
        game_name: app
            .and_then(|a| a.get("title"))
            .and_then(|t| t.as_str())
            .unwrap_or_default()
            .to_string(),
        icon_url: app
            .and_then(|a| a.get("icon"))
            .and_then(|i| i.get("url"))
            .and_then(|u| u.as_str())
            .unwrap_or_default()
            .to_string(),
        rating,
        hits_total: stat.and_then(|s| s.get("hits_total")).and_then(num_val),
        play_total: stat.and_then(|s| s.get("play_total")).and_then(num_val),
        review_count: stat.and_then(|s| s.get("review_count")).and_then(num_val),
        fans_count: stat.and_then(|s| s.get("fans_count")).and_then(num_val),
        rec_text: app
            .and_then(|a| a.get("rec_text"))
            .and_then(|t| t.as_str())
            .unwrap_or_default()
            .to_string(),
        released_time: app.and_then(|a| a.get("released_time")).and_then(|v| v.as_i64()),
        tags,
        description,
        source: SOURCE_TAPTAP,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn map_lookup(map: &[(&str, &str)], key: &str, kind: &str) -> Result<String> {
    for &(k, v) in map {
        if k == key {
            return Ok(v.to_string());
        }
    }
    Err(Error::InvalidParam(format!("unknown {kind}: {key}")))
}

fn num_val(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

fn to_opt_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn pick_opt_string(item: &Value, keys: &[&str]) -> Option<String> {
    for k in keys {
        if let Some(v) = item.get(k)
            && let Some(s) = to_opt_string(v) {
                return Some(s);
            }
    }
    None
}

fn pick_opt_num(item: &Value, keys: &[&str]) -> Option<f64> {
    for k in keys {
        if let Some(v) = item.get(k)
            && let Some(n) = num_val(v) {
                return Some(n);
            }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/{name}.json"));
        let txt = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    #[test]
    fn parses_car_sale_rank_gasgoo() {
        let v = fixture("car_sale_rank_gasgoo");
        let rows = parse_car_sale_rank_gasgoo(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].rank, Some("1".to_string()));
        assert_eq!(rows[0].name, Some("比亚迪".to_string()));
        assert_eq!(rows[0].sales, Some(219292.0));
        assert!(rows[0].raw.contains("比亚迪"));
        assert_eq!(rows[1].name, Some("一汽大众".to_string()));
    }

    #[test]
    fn parses_game_hot_rank_taptap() {
        let v = fixture("game_hot_rank_taptap");
        let data = v.get("data").unwrap().as_object().unwrap();
        let list = data.get("list").unwrap().as_array().unwrap();
        let rows: Vec<GameHotRankRow> = list
            .iter()
            .enumerate()
            .map(|(i, item)| parse_taptap_item(item, (i as u32) + 1))
            .collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].rank, 1);
        assert_eq!(rows[0].game_name, "原神");
        assert_eq!(rows[0].game_id, Some("1390".to_string()));
        assert_eq!(rows[0].rating, Some(8.5));
        assert_eq!(rows[0].tags, "开放世界, 二次元");
        assert_eq!(rows[1].game_name, "王者荣耀");
        assert_eq!(rows[1].hits_total, Some(1000000.0));
    }
}
