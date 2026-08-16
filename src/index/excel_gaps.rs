//! Excel-backed CSI / CNI index reports (akshare `index/*.py`).
//!
//! Implemented: `index_csindex_all`, `index_detail_cni`,
//! `index_detail_hist_cni`, `index_stock_cons_csindex`,
//! `index_stock_cons_weight_csindex`, `stock_zh_index_value_csindex`.
//!
//! `index_detail_hist_adjust_cni` is **deferred**: the upstream cnindex
//! `download-adjustment` endpoint currently returns a plain-text error
//! ("样本历史调样信息不存在！") instead of a spreadsheet for every index
//! code, so no fixture can be captured.

use calamine::{open_workbook_auto_from_rs, Reader, Sheets};

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36";

async fn fetch_bytes(
    url: &str,
    params: &[(&str, &str)],
    headers: &[(&str, &str)],
) -> Result<Vec<u8>> {
    let http = reqwest::Client::builder()
        .user_agent(UA)
        .build()
        .map_err(Error::Http)?;
    let mut req = http.get(url);
    if !params.is_empty() {
        req = req.query(params);
    }
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let resp = req.send().await.map_err(Error::Http)?;
    let bytes = resp.bytes().await.map_err(Error::Http)?;
    Ok(bytes.to_vec())
}

async fn fetch_bytes_post_json(
    url: &str,
    body: &serde_json::Value,
    headers: &[(&str, &str)],
) -> Result<Vec<u8>> {
    let http = reqwest::Client::builder()
        .user_agent(UA)
        .build()
        .map_err(Error::Http)?;
    let mut req = http.post(url).json(body);
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let resp = req.send().await.map_err(Error::Http)?;
    let bytes = resp.bytes().await.map_err(Error::Http)?;
    Ok(bytes.to_vec())
}

fn read_rows(bytes: &[u8], endpoint: &'static str) -> Result<Vec<Vec<String>>> {
    let mut wb: Sheets<std::io::Cursor<Vec<u8>>> =
        open_workbook_auto_from_rs(std::io::Cursor::new(bytes.to_vec())).map_err(|e| {
            Error::Parse {
                endpoint,
                message: e.to_string(),
            }
        })?;
    let range = wb
        .worksheet_range_at(0)
        .ok_or_else(|| Error::Parse {
            endpoint,
            message: "no sheet".into(),
        })?
        .map_err(|e| Error::Parse {
            endpoint,
            message: e.to_string(),
        })?;
    Ok(range
        .rows()
        .map(|r| r.iter().map(cell_to_string).collect())
        .collect())
}

fn parse_f64(s: &str) -> Option<f64> {
    let t: String = s.chars().filter(|c| *c != ',').collect();
    let t = t.trim();
    if t.is_empty() {
        return None;
    }
    t.parse::<f64>().ok()
}

fn cell_to_string(c: &calamine::Data) -> String {
    match c {
        calamine::Data::Empty => String::new(),
        calamine::Data::String(s) => s.trim().to_string(),
        calamine::Data::Int(i) => i.to_string(),
        calamine::Data::Float(f) => {
            if f.is_finite() && f.fract() == 0.0 && f.abs() < 1e15 {
                format!("{}", *f as i64)
            } else {
                format!("{}", f)
            }
        }
        calamine::Data::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}

fn col<'a>(row: &'a [String], i: usize) -> &'a str {
    row.get(i).map(|s| s.as_str()).unwrap_or("")
}

/// Left-pad a code with '0' to width 6 (mirrors akshare `str.zfill(6)`).
fn zfill6(s: &str) -> String {
    format!("{:0>6}", s)
}

/// Convert `YYYYMMDD` to `YYYY-MM-DD`; pass through otherwise.
fn ymd8(s: &str) -> String {
    if s.len() == 8 && s.chars().all(|c| c.is_ascii_digit()) {
        format!("{}-{}-{}", &s[..4], &s[4..6], &s[6..])
    } else {
        s.to_string()
    }
}

// ---------------------------------------------------------------------------
// index_csindex_all
// ---------------------------------------------------------------------------

/// CSI index catalogue row (`index_csindex_all`, akshare `index/index_csindex.py:16`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexCsindexAll {
    /// Index code (akshare `指数代码`).
    pub index_code: String,
    /// Index abbreviation (akshare `指数简称`).
    pub index_abbr: String,
    /// Index full name (akshare `指数全称`).
    pub index_full: String,
    /// Base date `YYYY-MM-DD` (akshare `基日`).
    pub base_date: String,
    /// Base point (akshare `基点`).
    pub base_point: Option<f64>,
    /// Index series (akshare `指数系列`).
    pub index_series: String,
    /// Sample count (akshare `样本数量`).
    pub sample_count: Option<f64>,
    /// Latest close (akshare `最新收盘`).
    pub latest_close: Option<f64>,
    /// One-month return (akshare `近一个月收益率`).
    pub one_month_return: Option<f64>,
    /// Asset class (akshare `资产类别`).
    pub asset_class: String,
    /// Index hotspot (akshare `指数热点`).
    pub index_hot: String,
    /// Currency (akshare `指数币种`).
    pub currency: String,
    /// Cooperative index flag (akshare `合作指数`).
    pub cooperative: String,
    /// Tracked-product flag (akshare `跟踪产品`).
    pub tracked_product: String,
}

/// CSI index catalogue (`index_csindex_all`, akshare `index/index_csindex.py:16`).
pub async fn index_csindex_all(_client: &Client) -> Result<Vec<IndexCsindexAll>> {
    let url = "https://www.csindex.com.cn/csindex-home/exportExcel/indexAll/CH";
    let body = serde_json::json!({
        "sorter": {"sortField": "null", "sortOrder": null},
        "pager": {"pageNum": 1, "pageSize": 10},
        "indexFilter": {
            "ifCustomized": null, "ifTracked": null, "ifWeightCapped": null,
            "indexCompliance": null, "hotSpot": null, "indexClassify": null,
            "currency": null, "region": null, "indexSeries": ["1"], "undefined": null,
        },
    });
    let headers = &[("Content-Type", "application/json;charset=UTF-8")];
    let bytes = fetch_bytes_post_json(url, &body, headers).await?;
    parse_index_csindex_all(&bytes)
}

pub(crate) fn parse_index_csindex_all(bytes: &[u8]) -> Result<Vec<IndexCsindexAll>> {
    let rows = read_rows(bytes, "index_csindex_all")?;
    let mut out = Vec::new();
    for r in rows.iter().skip(1) {
        if r.iter().all(|c| c.is_empty()) {
            continue;
        }
        out.push(IndexCsindexAll {
            index_code: zfill6(col(r, 0)),
            index_abbr: col(r, 1).to_string(),
            index_full: col(r, 2).to_string(),
            base_date: col(r, 3).to_string(),
            base_point: parse_f64(col(r, 4)),
            index_series: col(r, 5).to_string(),
            sample_count: parse_f64(col(r, 6)),
            latest_close: parse_f64(col(r, 7)),
            one_month_return: parse_f64(col(r, 8)),
            asset_class: col(r, 9).to_string(),
            index_hot: col(r, 10).to_string(),
            currency: col(r, 11).to_string(),
            cooperative: col(r, 12).to_string(),
            tracked_product: col(r, 13).to_string(),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// index_detail_cni / index_detail_hist_cni
// ---------------------------------------------------------------------------

/// CNI index constituent sample row (`index_detail_cni` / `index_detail_hist_cni`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexDetailCni {
    /// Date `YYYY-MM-DD` (akshare `日期`).
    pub date: String,
    /// Sample code (akshare `样本代码`).
    pub sample_code: String,
    /// Sample abbreviation (akshare `样本简称`).
    pub sample_abbr: String,
    /// Industry (akshare `所属行业`).
    pub industry: String,
    /// Total market value in 100M CNY (akshare `总市值`).
    pub total_market_value: Option<f64>,
    /// Weight percentage (akshare `权重`).
    pub weight: Option<f64>,
}

fn parse_index_detail_cni_impl(bytes: &[u8], endpoint: &'static str) -> Result<Vec<IndexDetailCni>> {
    let rows = read_rows(bytes, endpoint)?;
    let mut out = Vec::new();
    for r in rows.iter().skip(1) {
        if r.iter().all(|c| c.is_empty()) {
            continue;
        }
        out.push(IndexDetailCni {
            date: col(r, 0).to_string(),
            sample_code: zfill6(col(r, 1)),
            sample_abbr: col(r, 2).to_string(),
            industry: col(r, 3).to_string(),
            total_market_value: parse_f64(col(r, 4)),
            weight: parse_f64(col(r, 5)),
        });
    }
    Ok(out)
}

/// CNI index current constituents (`index_detail_cni`, akshare `index/index_cni.py:134`).
pub async fn index_detail_cni(_client: &Client, symbol: &str) -> Result<Vec<IndexDetailCni>> {
    let url = format!("https://www.cnindex.com.cn/sample-detail/download-history?indexcode={symbol}");
    let bytes = fetch_bytes(&url, &[], &[]).await?;
    parse_index_detail_cni_impl(&bytes, "index_detail_cni")
}

/// CNI index historical constituents (`index_detail_hist_cni`, akshare `index/index_cni.py:164`).
pub async fn index_detail_hist_cni(_client: &Client, symbol: &str) -> Result<Vec<IndexDetailCni>> {
    let url = format!("https://www.cnindex.com.cn/sample-detail/download-history?indexcode={symbol}");
    let bytes = fetch_bytes(&url, &[], &[]).await?;
    parse_index_detail_cni_impl(&bytes, "index_detail_hist_cni")
}

// ---------------------------------------------------------------------------
// index_stock_cons_csindex / index_stock_cons_weight_csindex
// ---------------------------------------------------------------------------

/// CSI index constituent row (`index_stock_cons_csindex`, akshare `index/index_cons.py:126`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexStockConsCsindex {
    /// Date `YYYY-MM-DD` (akshare `日期`).
    pub date: String,
    /// Index code (akshare `指数代码`).
    pub index_code: String,
    /// Index name (akshare `指数名称`).
    pub index_name: String,
    /// Index English name (akshare `指数英文名称`).
    pub index_name_en: String,
    /// Constituent code (akshare `成分券代码`).
    pub constituent_code: String,
    /// Constituent name (akshare `成分券名称`).
    pub constituent_name: String,
    /// Constituent English name (akshare `成分券英文名称`).
    pub constituent_name_en: String,
    /// Exchange (akshare `交易所`).
    pub exchange: String,
    /// Exchange English name (akshare `交易所英文名称`).
    pub exchange_en: String,
}

/// CSI index constituents (`index_stock_cons_csindex`, akshare `index/index_cons.py:126`).
pub async fn index_stock_cons_csindex(
    _client: &Client,
    symbol: &str,
) -> Result<Vec<IndexStockConsCsindex>> {
    let url = format!(
        "https://oss-ch.csindex.com.cn/static/html/csindex/public/uploads/file/autofile/cons/{symbol}cons.xls"
    );
    let bytes = fetch_bytes(&url, &[], &[]).await?;
    parse_index_stock_cons_csindex(&bytes)
}

pub(crate) fn parse_index_stock_cons_csindex(bytes: &[u8]) -> Result<Vec<IndexStockConsCsindex>> {
    let rows = read_rows(bytes, "index_stock_cons_csindex")?;
    let mut out = Vec::new();
    for r in rows.iter().skip(1) {
        if r.iter().all(|c| c.is_empty()) {
            continue;
        }
        out.push(IndexStockConsCsindex {
            date: ymd8(col(r, 0)),
            index_code: zfill6(col(r, 1)),
            index_name: col(r, 2).to_string(),
            index_name_en: col(r, 3).to_string(),
            constituent_code: zfill6(col(r, 4)),
            constituent_name: col(r, 5).to_string(),
            constituent_name_en: col(r, 6).to_string(),
            exchange: col(r, 7).to_string(),
            exchange_en: col(r, 8).to_string(),
        });
    }
    Ok(out)
}

/// CSI index constituent weight row (`index_stock_cons_weight_csindex`, akshare `index/index_cons.py:160`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexStockConsWeightCsindex {
    /// Date `YYYY-MM-DD` (akshare `日期`).
    pub date: String,
    /// Index code (akshare `指数代码`).
    pub index_code: String,
    /// Index name (akshare `指数名称`).
    pub index_name: String,
    /// Index English name (akshare `指数英文名称`).
    pub index_name_en: String,
    /// Constituent code (akshare `成分券代码`).
    pub constituent_code: String,
    /// Constituent name (akshare `成分券名称`).
    pub constituent_name: String,
    /// Constituent English name (akshare `成分券英文名称`).
    pub constituent_name_en: String,
    /// Exchange (akshare `交易所`).
    pub exchange: String,
    /// Exchange English name (akshare `交易所英文名称`).
    pub exchange_en: String,
    /// Weight percentage (akshare `权重`).
    pub weight: Option<f64>,
}

/// CSI index constituent weights (`index_stock_cons_weight_csindex`, akshare `index/index_cons.py:160`).
pub async fn index_stock_cons_weight_csindex(
    _client: &Client,
    symbol: &str,
) -> Result<Vec<IndexStockConsWeightCsindex>> {
    let url = format!(
        "https://oss-ch.csindex.com.cn/static/html/csindex/public/uploads/file/autofile/closeweight/{symbol}closeweight.xls"
    );
    let bytes = fetch_bytes(&url, &[], &[]).await?;
    parse_index_stock_cons_weight_csindex(&bytes)
}

pub(crate) fn parse_index_stock_cons_weight_csindex(
    bytes: &[u8],
) -> Result<Vec<IndexStockConsWeightCsindex>> {
    let rows = read_rows(bytes, "index_stock_cons_weight_csindex")?;
    let mut out = Vec::new();
    for r in rows.iter().skip(1) {
        if r.iter().all(|c| c.is_empty()) {
            continue;
        }
        out.push(IndexStockConsWeightCsindex {
            date: ymd8(col(r, 0)),
            index_code: zfill6(col(r, 1)),
            index_name: col(r, 2).to_string(),
            index_name_en: col(r, 3).to_string(),
            constituent_code: zfill6(col(r, 4)),
            constituent_name: col(r, 5).to_string(),
            constituent_name_en: col(r, 6).to_string(),
            exchange: col(r, 7).to_string(),
            exchange_en: col(r, 8).to_string(),
            weight: parse_f64(col(r, 9)),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// stock_zh_index_value_csindex
// ---------------------------------------------------------------------------

/// CSI index valuation row (`stock_zh_index_value_csindex`, akshare `index/index_stock_zh_csindex.py:72`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockZhIndexValueCsindex {
    /// Date `YYYY-MM-DD` (akshare `日期`).
    pub date: String,
    /// Index code (akshare `指数代码`).
    pub index_code: String,
    /// Index Chinese full name (akshare `指数中文全称`).
    pub index_cn_full: String,
    /// Index Chinese abbreviation (akshare `指数中文简称`).
    pub index_cn_abbr: String,
    /// Index English full name (akshare `指数英文全称`).
    pub index_en_full: String,
    /// Index English abbreviation (akshare `指数英文简称`).
    pub index_en_abbr: String,
    /// P/E 1 (akshare `市盈率1`).
    pub pe1: Option<f64>,
    /// P/E 2 (akshare `市盈率2`).
    pub pe2: Option<f64>,
    /// Dividend yield 1 (akshare `股息率1`).
    pub dp1: Option<f64>,
    /// Dividend yield 2 (akshare `股息率2`).
    pub dp2: Option<f64>,
}

/// CSI index valuation (`stock_zh_index_value_csindex`, akshare `index/index_stock_zh_csindex.py:72`).
pub async fn stock_zh_index_value_csindex(
    _client: &Client,
    symbol: &str,
) -> Result<Vec<StockZhIndexValueCsindex>> {
    let url = format!(
        "https://oss-ch.csindex.com.cn/static/html/csindex/public/uploads/file/autofile/indicator/{symbol}indicator.xls"
    );
    let bytes = fetch_bytes(&url, &[], &[]).await?;
    parse_stock_zh_index_value_csindex(&bytes)
}

pub(crate) fn parse_stock_zh_index_value_csindex(
    bytes: &[u8],
) -> Result<Vec<StockZhIndexValueCsindex>> {
    let rows = read_rows(bytes, "stock_zh_index_value_csindex")?;
    let mut out = Vec::new();
    for r in rows.iter().skip(1) {
        if r.iter().all(|c| c.is_empty()) {
            continue;
        }
        out.push(StockZhIndexValueCsindex {
            date: ymd8(col(r, 0)),
            index_code: zfill6(col(r, 1)),
            index_cn_full: col(r, 2).to_string(),
            index_cn_abbr: col(r, 3).to_string(),
            index_en_full: col(r, 4).to_string(),
            index_en_abbr: col(r, 5).to_string(),
            pe1: parse_f64(col(r, 6)),
            pe2: parse_f64(col(r, 7)),
            dp1: parse_f64(col(r, 8)),
            dp2: parse_f64(col(r, 9)),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(name),
        )
        .unwrap()
    }

    #[test]
    fn parses_index_csindex_all() {
        let rows = parse_index_csindex_all(&fixture("index_csindex_all.xlsx")).unwrap();
        assert!(rows.len() > 10);
        assert_eq!(rows[0].index_code, "000300");
        assert_eq!(rows[0].index_abbr, "沪深300");
        assert_eq!(rows[0].base_date, "2004-12-31");
        assert!((rows[0].latest_close.unwrap() - 4665.88).abs() < 1e-6);
        assert!((rows[0].one_month_return.unwrap() + 2.72).abs() < 1e-6);
    }

    #[test]
    fn parses_index_detail_cni() {
        let rows = parse_index_detail_cni_impl(&fixture("index_detail_cni.xlsx"), "index_detail_cni").unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].sample_code, "000937");
        assert_eq!(rows[0].sample_abbr, "冀中能源");
        assert_eq!(rows[0].industry, "能源");
        assert!((rows[0].weight.unwrap() - 0.03).abs() < 1e-6);
    }

    #[test]
    fn parses_index_detail_hist_cni() {
        let rows = parse_index_detail_cni_impl(&fixture("index_detail_hist_cni.xlsx"), "index_detail_hist_cni").unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].sample_code, "000937");
    }

    #[test]
    fn parses_index_stock_cons_csindex() {
        let rows = parse_index_stock_cons_csindex(&fixture("index_stock_cons_csindex.xls")).unwrap();
        assert!(rows.len() > 100);
        assert_eq!(rows[0].index_code, "000300");
        assert_eq!(rows[0].constituent_code, "000001");
        assert_eq!(rows[0].date, "2026-08-14");
    }

    #[test]
    fn parses_index_stock_cons_weight_csindex() {
        let rows =
            parse_index_stock_cons_weight_csindex(&fixture("index_stock_cons_weight_csindex.xls"))
                .unwrap();
        assert!(rows.len() > 100);
        assert_eq!(rows[0].constituent_code, "000001");
        assert_eq!(rows[0].date, "2026-07-31");
        assert!((rows[0].weight.unwrap() - 0.433).abs() < 1e-6);
    }

    #[test]
    fn parses_stock_zh_index_value_csindex() {
        let rows =
            parse_stock_zh_index_value_csindex(&fixture("stock_zh_index_value_csindex.xls")).unwrap();
        assert!(rows.len() > 10);
        assert_eq!(rows[0].index_code, "H30374");
        assert_eq!(rows[0].date, "2026-08-13");
        assert!((rows[0].pe1.unwrap() - 16.44).abs() < 1e-6);
        assert!((rows[0].dp2.unwrap() - 1.81).abs() < 1e-6);
    }
}
