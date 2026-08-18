//! Misc 指数 endpoints (akshare `index/`): 生猪现货, 商品现货, A股新闻情绪.
//!
//! | Rust function | akshare source | source | note |
//! |---|---|---|---|
//! | `index_hog_spot_price` | `index_hog.py:13` | nxin | GET; `data` is array-of-arrays, 日期 is ms-epoch → +8h date |
//! | `spot_goods` | `index_spot.py:13` | sina | GET; `result.data.data` |
//! | `index_news_sentiment_scope` | `index_zh_a_scope.py:13` | chinascope | GET; response is a JSON array |

use chrono::{DateTime, Duration, Utc};
use serde_json::Value;

use crate::core::client::{Client, SOURCE_SINA};
use crate::core::error::{Error, Result};
use crate::core::json::*;

const SOURCE_NXIN: &str = "nxin";
const SOURCE_CHINASCOPE: &str = "chinascope";

const HOG_SPOT_URL: &str = "https://hqb.nxin.com/pigindex/getPigIndexChart.shtml";
const SPOT_GOODS_URL: &str =
    "https://stock.finance.sina.com.cn/futures/api/openapi.php/GoodsIndexService.get_goods_index";
const NEWS_SENTIMENT_URL: &str = "https://www.chinascope.com/inews/senti/index";

// ---------------------------------------------------------------------------
// shared rows
// ---------------------------------------------------------------------------

/// One 行情宝 生猪现货 price point (`index_hog_spot_price`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct HogSpotPriceRow {
    /// 日期 (ms-epoch → UTC+8 date string)
    pub date: Option<String>,
    /// 指数
    pub index: Option<f64>,
    /// 4个月均线
    pub ma_4: Option<f64>,
    /// 6个月均线
    pub ma_6: Option<f64>,
    /// 12个月均线
    pub ma_12: Option<f64>,
    /// 预售均价
    pub presale_avg: Option<f64>,
    /// 成交均价
    pub trade_avg: Option<f64>,
    /// 成交均重
    pub trade_weight: Option<f64>,
}

/// One 新浪 商品现货 price point (`spot_goods`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SpotGoodsRow {
    /// 日期 (opendate, `YYYY-MM-DD`)
    pub date: Option<String>,
    /// 指数 (price)
    pub index: Option<f64>,
    /// 涨跌额 (zde)
    pub change_amount: Option<f64>,
    /// 涨跌幅 (zdf)
    pub change_pct: Option<f64>,
}

/// One 数库 A股新闻情绪 index point (`index_news_sentiment_scope`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct NewsSentimentScopeRow {
    /// 日期 (tradeDate)
    pub date: Option<String>,
    /// 市场情绪指数 (maIndex1)
    pub sentiment_index: Option<f64>,
    /// 沪深300指数 (marketClose)
    pub hs300_index: Option<f64>,
}

// ---------------------------------------------------------------------------
// parse cores
// ---------------------------------------------------------------------------

/// Parse `index_hog_spot_price` rows. Upstream `data` is an array of 8-element
/// arrays `[日期(ms), 指数, 4月均线, 6月均线, 12月均线, 预售均价, 成交均价, 成交均重]`.
pub(crate) fn parse_index_hog_spot_price(resp: &Value) -> Result<Vec<HogSpotPriceRow>> {
    let arr = resp
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_NXIN,
            message: "missing data array".into(),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for row in arr {
        let Some(cells) = row.as_array() else {
            continue;
        };
        out.push(HogSpotPriceRow {
            date: cells
                .first()
                .and_then(|v| v.as_i64())
                .and_then(ms_epoch_to_cst_date),
            index: arr_num(cells, 1),
            ma_4: arr_num(cells, 2),
            ma_6: arr_num(cells, 3),
            ma_12: arr_num(cells, 4),
            presale_avg: arr_num(cells, 5),
            trade_avg: arr_num(cells, 6),
            trade_weight: arr_num(cells, 7),
        });
    }
    Ok(out)
}

/// Parse `spot_goods` rows (`result.data.data` array of objects).
pub(crate) fn parse_spot_goods(resp: &Value) -> Result<Vec<SpotGoodsRow>> {
    let arr = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "missing result.data.data".into(),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(SpotGoodsRow {
            date: opt_str(item, "opendate"),
            index: opt_f64(item, "price"),
            change_amount: opt_f64(item, "zde"),
            change_pct: opt_f64(item, "zdf"),
        });
    }
    Ok(out)
}

/// Parse `index_news_sentiment_scope` rows (response is a JSON array).
pub(crate) fn parse_index_news_sentiment_scope(resp: &Value) -> Result<Vec<NewsSentimentScopeRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_CHINASCOPE,
        message: "expected array".into(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(NewsSentimentScopeRow {
            date: opt_str(item, "tradeDate"),
            sentiment_index: opt_f64(item, "maIndex1"),
            hs300_index: opt_f64(item, "marketClose"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// public functions
// ---------------------------------------------------------------------------

/// 行情宝-生猪市场价格指数 (akshare `index_hog_spot_price`).
pub async fn index_hog_spot_price(client: &Client) -> Result<Vec<HogSpotPriceRow>> {
    let v = client
        .get_json(
            SOURCE_NXIN,
            "index_hog_spot_price",
            HOG_SPOT_URL,
            &[("regionId", "0")],
        )
        .await?;
    parse_index_hog_spot_price(&v)
}

/// 新浪财经-商品现货价格指数 (akshare `spot_goods`).
///
/// `symbol` is one of `{"波罗的海干散货指数","钢坯价格指数","澳大利亚粉矿价格"}`.
pub async fn spot_goods(client: &Client, symbol: &str) -> Result<Vec<SpotGoodsRow>> {
    let code = match symbol {
        "波罗的海干散货指数" => "BDI",
        "钢坯价格指数" => "GP",
        "澳大利亚粉矿价格" => "PB",
        other => {
            return Err(Error::InvalidParam(format!(
                "unknown symbol: {other} (expected 波罗的海干散货指数/钢坯价格指数/澳大利亚粉矿价格)"
            )))
        }
    };
    let v = client
        .get_json(
            SOURCE_SINA,
            "spot_goods",
            SPOT_GOODS_URL,
            &[("symbol", code), ("table", "0")],
        )
        .await?;
    parse_spot_goods(&v)
}

/// 数库-A股新闻情绪指数 (akshare `index_news_sentiment_scope`).
pub async fn index_news_sentiment_scope(client: &Client) -> Result<Vec<NewsSentimentScopeRow>> {
    let v = client
        .get_json(
            SOURCE_CHINASCOPE,
            "index_news_sentiment_scope",
            NEWS_SENTIMENT_URL,
            &[("period", "YEAR")],
        )
        .await?;
    parse_index_news_sentiment_scope(&v)
}

// ---------------------------------------------------------------------------
// private helpers (verbatim per task instructions)
// ---------------------------------------------------------------------------


/// Read a numeric element at `idx` of an upstream array row.
fn arr_num(cells: &[Value], idx: usize) -> Option<f64> {
    cells.get(idx).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

/// Convert a millisecond epoch to a `UTC+8` date string (`YYYY-MM-DD`),
/// mirroring akshare's `pd.to_datetime(unit="ms") + Timedelta(hours=8)`.
fn ms_epoch_to_cst_date(ms: i64) -> Option<String> {
    let secs = ms / 1000;
    let nanos = (ms % 1000) * 1_000_000;
    let dt = DateTime::<Utc>::from_timestamp(secs, nanos as u32)?;
    let cst = dt + Duration::hours(8);
    Some(cst.format("%Y-%m-%d").to_string())
}

// ---------------------------------------------------------------------------
// offline parse tests
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

    #[test]
    fn test_parse_index_hog_spot_price() {
        let rows = parse_index_hog_spot_price(&fixture("index_hog_spot_price.json")).unwrap();
        assert_eq!(rows.len(), 2);
        // 2024-01-02 00:00:00 UTC = 2024-01-02 08:00 CST → date "2024-01-02"
        assert_eq!(rows[0].date.as_deref(), Some("2024-01-02"));
        assert_eq!(rows[0].index, Some(105.5));
        assert_eq!(rows[0].ma_4, Some(104.2));
        assert_eq!(rows[0].ma_6, Some(103.1));
        assert_eq!(rows[0].ma_12, Some(101.9));
        assert_eq!(rows[0].presale_avg, Some(14.2));
        assert_eq!(rows[0].trade_avg, Some(13.8));
        assert_eq!(rows[0].trade_weight, Some(120.5));
        assert_eq!(rows[1].index, Some(106.0));
    }

    #[test]
    fn test_parse_spot_goods() {
        let rows = parse_spot_goods(&fixture("spot_goods.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date.as_deref(), Some("2024-01-02"));
        assert_eq!(rows[0].index, Some(1876.0));
        assert_eq!(rows[0].change_amount, Some(-12.3));
        assert_eq!(rows[0].change_pct, Some(-0.0065));
        assert_eq!(rows[1].index, Some(1880.5));
    }

    #[test]
    fn test_parse_index_news_sentiment_scope() {
        let rows = parse_index_news_sentiment_scope(&fixture("index_news_sentiment_scope.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date.as_deref(), Some("2024-01-02"));
        assert_eq!(rows[0].sentiment_index, Some(0.6234));
        assert_eq!(rows[0].hs300_index, Some(3456.78));
        assert_eq!(rows[1].sentiment_index, Some(0.6012));
    }

    #[test]
    fn test_spot_goods_bad_symbol() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = Client::new();
            let err = spot_goods(&client, "黄金").await;
            assert!(matches!(err, Err(Error::InvalidParam(_))));
        });
    }
}
