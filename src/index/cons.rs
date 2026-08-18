//! 指数成份股 / 国证指数 / 中证指数列表. Ports `akshare/index/index_cons.py`,
//! `akshare/index/index_cni.py` and `akshare/index/index_csindex.py`.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `index_stock_cons_sina` | `index_cons.py:20` | Sina `Market_Center.getHQNodeData(Simple)` JSON; `symbol="000300"` paginates `hs300` |
//! | `index_all_cni` | `index_cni.py:16` | 国证指数 `cnindex.com.cn/index/indexList` JSON |
//! | `index_hist_cni` | `index_cni.py:67` | 国证历史行情 `hq.cnindex.com.cn/.../getIndexDailyDataWithDataFormat` JSON |
//! | `stock_a_code_to_symbol` | `index_cons.py:196` | pure code→symbol helper (returns `Vec<CodeSymbolRow>`) |
//!
//! ## DEFERRED
//!
//! * `index_stock_cons_csindex` (`index_cons.py:126`) — fetches `{symbol}cons.xls`
//!   from `oss-ch.csindex.com.cn` and parses with `pd.read_excel`. Excel/XLS
//!   download (rule 4: Excel/ZIP), not ported.
//! * `index_stock_cons_weight_csindex` (`index_cons.py:160`) — fetches
//!   `{symbol}closeweight.xls` and parses with `pd.read_excel`. Excel download;
//!   not ported.
//! * `index_stock_info` (`index_cons.py:70`) — scrapes an HTML `<table>` from
//!   `joinquant.com` via `pd.read_html`. HTML-table scraping (rule 4); not ported.
//! * `index_detail_cni` (`index_cni.py:134`) — downloads `sample-detail/download-history`
//!   as Excel and parses with `pd.read_excel`. Excel download; not ported.
//! * `index_detail_hist_cni` (`index_cni.py:164`) — same `download-history` Excel
//!   endpoint as `index_detail_cni`. Excel download; not ported.
//! * `index_detail_hist_adjust_cni` (`index_cni.py:191`) — downloads
//!   `sample-detail/download-adjustment` as Excel (`openpyxl`). Excel download;
//!   not ported.
//! * `index_csindex_all` (`index_csindex.py:16`) — POSTs to
//!   `csindex.com.cn/csindex-home/exportExcel/indexAll/CH` and parses the returned
//!   XLS with `pd.read_excel`. Excel download; not ported.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_SINA};
use crate::core::error::{Error, Result};
use crate::core::json::*;

const SOURCE_CNINDEX: &str = "cnindex";

const SINA_HQ_NODE_DATA_SIMPLE: &str = "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeDataSimple";
const SINA_HQ_NODE_DATA: &str = "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeData";
const SINA_HQ_NODE_COUNT: &str = "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeStockCountSimple";

const CNINDEX_INDEX_LIST: &str = "https://www.cnindex.com.cn/index/indexList";
const CNINDEX_HIST: &str = "http://hq.cnindex.com.cn/market/market/getIndexDailyDataWithDataFormat";

// ===========================================================================
// helpers
// ===========================================================================

/// Parse a percent string like `"1.23%"` into `0.0123`; tolerates a bare number.
fn fpct(item: &Value, k: &str) -> Option<f64> {
    match item.get(k) {
        Some(Value::String(s)) => {
            let cleaned = s.trim().trim_end_matches('%').trim();
            cleaned.parse::<f64>().ok().map(|x| x / 100.0)
        }
        Some(Value::Number(n)) => n.as_f64().map(|x| x / 100.0),
        _ => None,
    }
}

/// Read a numeric element at `idx` of an array-valued item (used by cni history,
/// where each row is a JSON array, not an object).
fn arr_num_at(item: &Value, idx: usize) -> Option<f64> {
    item.get(idx)
        .and_then(|v| v.as_str())
        .and_then(|s| s.trim().parse::<f64>().ok())
}

fn arr_str_at(item: &Value, idx: usize) -> Option<String> {
    item.get(idx).and_then(|v| v.as_str()).map(str::to_string)
}

// ===========================================================================
// index_stock_cons_sina  (akshare index_cons.py:20)
// ===========================================================================

/// A single constituent returned by Sina's `Market_Center.getHQNodeData(Simple)`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexStockConsSinaRow {
    /// 新浪代码 (Sina `symbol`, e.g. `sh600000`)
    pub symbol: String,
    /// 代码 (Sina `code`, e.g. `600000`)
    pub code: String,
    /// 名称 (Sina `name`)
    pub name: String,
    /// 最新价 (Sina `trade`)
    pub trade: Option<f64>,
    /// 今开 (Sina `open`)
    pub open: Option<f64>,
    /// 最高 (Sina `high`)
    pub high: Option<f64>,
    /// 最低 (Sina `low`)
    pub low: Option<f64>,
    /// 成交量 (Sina `volume`)
    pub volume: Option<f64>,
    /// 成交额 (Sina `amount`)
    pub amount: Option<f64>,
    /// 涨跌额 (Sina `pricechange`)
    pub price_change: Option<f64>,
    /// 涨跌幅 (Sina `changepercent`, parsed from `"1.23%"` → `0.0123`)
    pub change_percent: Option<f64>,
}

/// Parse `index_stock_cons_sina` rows from the already-fetched Sina JSON value
/// (a JSON array of node-dict objects).
pub(crate) fn parse_index_stock_cons_sina(resp: &Value) -> Result<Vec<IndexStockConsSinaRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "expected array".into(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let Some(symbol) = opt_str(item, "symbol") else {
            continue;
        };
        out.push(IndexStockConsSinaRow {
            symbol,
            code: opt_str(item, "code").unwrap_or_default(),
            name: opt_str(item, "name").unwrap_or_default(),
            trade: opt_f64(item, "trade"),
            open: opt_f64(item, "open"),
            high: opt_f64(item, "high"),
            low: opt_f64(item, "low"),
            volume: opt_f64(item, "volume"),
            amount: opt_f64(item, "amount"),
            price_change: opt_f64(item, "pricechange"),
            change_percent: fpct(item, "changepercent"),
        });
    }
    Ok(out)
}

/// 新浪-指数成份股 (`index_stock_cons_sina`, akshare `index_cons.py:20`).
///
/// `symbol="000300"` special-cases the `hs300` node: it first hits
/// `getHQNodeStockCountSimple` to learn the constituent count, then paginates
/// `getHQNodeData` (80 per page). Any other symbol uses the single-shot
/// `getHQNodeDataSimple` against `zhishu_{symbol}`.
pub async fn index_stock_cons_sina(
    client: &Client,
    symbol: &str,
) -> Result<Vec<IndexStockConsSinaRow>> {
    let headers = Some(&[("Referer", "https://finance.sina.com.cn")][..]);
    if symbol == "000300" {
        let count_text = client
            .get_text(
                SOURCE_SINA,
                "index_stock_cons_sina_count",
                SINA_HQ_NODE_COUNT,
                &[("node", "hs300")],
                headers,
            )
            .await?;
        let count: f64 = serde_json::from_str::<Value>(&count_text)
            .map_err(Error::Json)?
            .as_f64()
            .or_else(|| {
                serde_json::from_str::<Value>(&count_text)
                    .ok()
                    .and_then(|v| v.as_str().and_then(|s| s.parse().ok()))
            })
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_SINA,
                message: "bad constituent count".into(),
            })?;
        let page_num = (count / 80.0).ceil() as usize + 1;
        let mut all = Vec::new();
        for page in 1..page_num {
            let page_s = page.to_string();
            let params: Vec<(&str, &str)> = vec![
                ("page", &page_s),
                ("num", "80"),
                ("sort", "symbol"),
                ("asc", "1"),
                ("node", "hs300"),
                ("symbol", ""),
                ("_s_r_a", "init"),
            ];
            let txt = client
                .get_text(
                    SOURCE_SINA,
                    "index_stock_cons_sina",
                    SINA_HQ_NODE_DATA,
                    &params,
                    headers,
                )
                .await?;
            let v: Value = serde_json::from_str(&txt).map_err(Error::Json)?;
            all.extend(parse_index_stock_cons_sina(&v)?);
        }
        Ok(all)
    } else {
        let node = format!("zhishu_{symbol}");
        let params: Vec<(&str, &str)> = vec![
            ("page", "1"),
            ("num", "3000"),
            ("sort", "symbol"),
            ("asc", "1"),
            ("node", &node),
            ("_s_r_a", "setlen"),
        ];
        let txt = client
            .get_text(
                SOURCE_SINA,
                "index_stock_cons_sina",
                SINA_HQ_NODE_DATA_SIMPLE,
                &params,
                headers,
            )
            .await?;
        let v: Value = serde_json::from_str(&txt).map_err(Error::Json)?;
        parse_index_stock_cons_sina(&v)
    }
}

// ===========================================================================
// index_all_cni  (akshare index_cni.py:16)
// ===========================================================================

/// 国证指数-最近交易日的所有指数 (`index_all_cni`, akshare `index_cni.py:16`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexAllCniRow {
    /// 指数代码 (cnindex `indexcode`)
    pub index_code: String,
    /// 指数简称 (cnindex `indexname`)
    pub index_name: String,
    /// 样本数 (cnindex `samplesize`)
    pub sample_size: Option<f64>,
    /// 收盘点位 (cnindex `closeingPoint`)
    pub close_point: Option<f64>,
    /// 涨跌幅 (cnindex `percent`)
    pub change_percent: Option<f64>,
    /// PE滚动 (cnindex `peDynamic`)
    pub pe_dynamic: Option<f64>,
    /// 成交量 (cnindex `volume`, ÷100000)
    pub volume: Option<f64>,
    /// 成交额 (cnindex `amount`, ÷1e8)
    pub amount: Option<f64>,
    /// 总市值 (cnindex `totalMarketValue`, ÷1e8)
    pub total_market_value: Option<f64>,
    /// 自由流通市值 (cnindex `freeMarketValue`, ÷1e8)
    pub free_market_value: Option<f64>,
}

/// Parse `index_all_cni` rows from the already-fetched cnindex JSON value.
/// Mirrors akshare's column scaling (volume ÷1e5, amount/total/free ÷1e8).
pub(crate) fn parse_index_all_cni(resp: &Value) -> Result<Vec<IndexAllCniRow>> {
    let rows = resp
        .get("data")
        .and_then(|d| d.get("rows"))
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CNINDEX,
            message: "missing data.rows".into(),
        })?;
    let mut out = Vec::with_capacity(rows.len());
    for item in rows {
        let Some(index_code) = opt_str(item, "indexcode") else {
            continue;
        };
        out.push(IndexAllCniRow {
            index_code,
            index_name: opt_str(item, "indexname").unwrap_or_default(),
            sample_size: opt_f64(item, "samplesize"),
            close_point: opt_f64(item, "closeingPoint"),
            change_percent: opt_f64(item, "percent"),
            pe_dynamic: opt_f64(item, "peDynamic"),
            volume: opt_f64(item, "volume").map(|v| v / 100_000.0),
            amount: opt_f64(item, "amount").map(|v| v / 100_000_000.0),
            total_market_value: opt_f64(item, "totalMarketValue").map(|v| v / 100_000_000.0),
            free_market_value: opt_f64(item, "freeMarketValue").map(|v| v / 100_000_000.0),
        });
    }
    Ok(out)
}

/// 国证指数-所有指数 (`index_all_cni`, akshare `index_cni.py:16`).
pub async fn index_all_cni(client: &Client) -> Result<Vec<IndexAllCniRow>> {
    let v = client
        .get_json(
            SOURCE_CNINDEX,
            "index_all_cni",
            CNINDEX_INDEX_LIST,
            &[("channelCode", "-1"), ("rows", "2000"), ("pageNum", "1")],
        )
        .await?;
    parse_index_all_cni(&v)
}

// ===========================================================================
// index_hist_cni  (akshare index_cni.py:67)
// ===========================================================================

/// 国证指数-历史行情 (`index_hist_cni`, akshare `index_cni.py:67`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexHistCniRow {
    /// 日期
    pub date: String,
    /// 开盘价
    pub open: Option<f64>,
    /// 最高价
    pub high: Option<f64>,
    /// 最低价
    pub low: Option<f64>,
    /// 收盘价
    pub close: Option<f64>,
    /// 涨跌幅 (parsed from `"1.23%"` → `0.0123`)
    pub change_percent: Option<f64>,
    /// 成交量
    pub volume: Option<f64>,
    /// 成交额
    pub amount: Option<f64>,
}

/// Parse `index_hist_cni` rows. The upstream returns `data.data` as an array of
/// arrays (not objects) in the column order akshare documents:
/// `[日期,_,最高价,开盘价,最低价,收盘价,_,涨跌幅,成交额,成交量,_]`.
pub(crate) fn parse_index_hist_cni(resp: &Value) -> Result<Vec<IndexHistCniRow>> {
    let rows = resp
        .get("data")
        .and_then(|d| d.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CNINDEX,
            message: "missing data.data".into(),
        })?;
    let mut out = Vec::with_capacity(rows.len());
    for item in rows {
        let Some(date) = arr_str_at(item, 0) else {
            continue;
        };
        let change_percent = item
            .get(7)
            .and_then(|v| v.as_str())
            .map(|s| s.trim_end_matches('%').trim().parse::<f64>().unwrap_or(0.0) / 100.0);
        out.push(IndexHistCniRow {
            date,
            open: arr_num_at(item, 3),
            high: arr_num_at(item, 2),
            low: arr_num_at(item, 4),
            close: arr_num_at(item, 5),
            change_percent,
            volume: arr_num_at(item, 9),
            amount: arr_num_at(item, 8),
        });
    }
    Ok(out)
}

/// 国证指数-历史行情 (`index_hist_cni`, akshare `index_cni.py:67`).
///
/// `start_date`/`end_date` are `YYYYMMDD` strings, reformatted to `YYYY-MM-DD`
/// for the request (matching akshare).
pub async fn index_hist_cni(
    client: &Client,
    symbol: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<IndexHistCniRow>> {
    let sd = format!(
        "{}-{}-{}",
        &start_date[..4],
        &start_date[4..6],
        &start_date[6..]
    );
    let ed = format!("{}-{}-{}", &end_date[..4], &end_date[4..6], &end_date[6..]);
    let v = client
        .get_json(
            SOURCE_CNINDEX,
            "index_hist_cni",
            CNINDEX_HIST,
            &[
                ("indexCode", symbol),
                ("startDate", &sd),
                ("endDate", &ed),
                ("frequency", "day"),
            ],
        )
        .await?;
    parse_index_hist_cni(&v)
}

// ===========================================================================
// stock_a_code_to_symbol  (akshare index_cons.py:196)
// ===========================================================================

/// A (code, market-symbol) pair produced by [`stock_a_code_to_symbol`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct CodeSymbolRow {
    /// 股票代码 (e.g. `600000`)
    pub code: String,
    /// 带市场前缀的符号 (e.g. `sh600000` / `sz600000`)
    pub symbol: String,
}

/// 输入股票代码判断股票市场 (`stock_a_code_to_symbol`, akshare `index_cons.py:196`).
///
/// Pure helper (no network): codes starting with `6` or `900` map to `sh`, all
/// others to `sz`. Unlike akshare's single-string return, this returns one row
/// per input code so multiple codes can be mapped at once.
pub fn stock_a_code_to_symbol(symbols: &[&str]) -> Vec<CodeSymbolRow> {
    symbols
        .iter()
        .map(|s| {
            let prefix = if s.starts_with('6') || s.starts_with("900") {
                "sh"
            } else {
                "sz"
            };
            CodeSymbolRow {
                code: s.to_string(),
                symbol: format!("{prefix}{s}"),
            }
        })
        .collect()
}

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
    fn parse_index_stock_cons_sina_ok() {
        let rows = parse_index_stock_cons_sina(&fixture("index_stock_cons_sina.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "sh600000");
        assert_eq!(rows[0].code, "600000");
        assert_eq!(rows[0].name, "浦发银行");
        assert!(approx(rows[0].trade, 9.85));
        assert!(approx(rows[0].change_percent, 0.0051));
        assert_eq!(rows[1].symbol, "sz000001");
        assert!(approx(rows[1].change_percent, 0.009));
    }

    #[test]
    fn parse_index_all_cni_ok() {
        let rows = parse_index_all_cni(&fixture("index_all_cni.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].index_code, "399001");
        assert_eq!(rows[0].sample_size, Some(500.0));
        assert!(approx(rows[0].volume, 12345.6789));
        assert!(approx(rows[0].amount, 2345.67890123));
        assert!(approx(rows[0].total_market_value, 3456.78901234));
        assert_eq!(rows[1].index_code, "399005");
        assert!(approx(rows[1].change_percent, 0.45));
    }

    #[test]
    fn parse_index_hist_cni_ok() {
        let rows = parse_index_hist_cni(&fixture("index_hist_cni.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2023-01-16");
        assert!(approx(rows[0].open, 9800.0));
        assert!(approx(rows[0].high, 9920.0));
        assert!(approx(rows[0].close, 9850.0));
        assert!(approx(rows[0].change_percent, 0.0123));
        assert!(approx(rows[0].volume, 98765432.0));
        assert!(approx(rows[0].amount, 1234567890.0));
        assert!(approx(rows[1].change_percent, -0.002));
    }

    #[test]
    fn stock_a_code_to_symbol_ok() {
        let rows = stock_a_code_to_symbol(&["600000", "000001", "900901", "300750"]);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].symbol, "sh600000");
        assert_eq!(rows[1].symbol, "sz000001");
        assert_eq!(rows[2].symbol, "sh900901");
        assert_eq!(rows[3].symbol, "sz300750");
    }
}
