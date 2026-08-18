//! Long-tail A-share endpoints ported from akshare (the "more2" batch).
//!
//! | Rust fn                               | akshare fn                    | Source                  | Notes                                              |
//! |---------------------------------------|-------------------------------|-------------------------|----------------------------------------------------|
//! | `stock_zh_ah_spot_em`                 | `stock_zh_ah_spot_em`         | Eastmoney push2         | AH 股比价实时行情, `clist/get`, `fltt=1` (÷1000/÷100) |
//! | `stock_hsgt_sh_hk_spot_em`            | `stock_hsgt_sh_hk_spot_em`    | Eastmoney push2         | 沪港通-港股通(沪>港), `clist/get`, `fltt=1`          |
//! | `stock_zh_kcb_report_em`              | `stock_zh_kcb_report_em`      | Eastmoney np-anotice    | 科创板报告公告列表, `data.list`                      |
//! | `stock_repurchase_em`                 | `stock_repurchase_em`         | Eastmoney datacenter    | 股票回购数据, `RPTA_WEB_GETHGLIST_NEW`              |
//! | `stock_gsrl_gsdt_em`                  | `stock_gsrl_gsdt_em`          | Eastmoney datacenter    | 股市日历-公司动态, `RPT_ORGOP_ALL`                   |
//! | `stock_zh_a_new_em`                   | `stock_zh_a_new_em`           | Eastmoney push2 (`40.`) | 新股板块, `clist/get`, `fltt=2`                     |
//! | `stock_hold_management_detail_em`     | `stock_hold_management_detail_em` | Eastmoney datacenter | 高管持股变动明细, `RPT_EXECUTIVE_HOLD_DETAILS`       |
//! | `stock_yysj_em`                       | `stock_yysj_em`               | Eastmoney datacenter    | 预约披露时间, `RPT_PUBLIC_BS_APPOIN`                |
//! | `stock_jgdy_detail_em`                | `stock_jgdy_detail_em`        | Eastmoney datacenter    | 机构调研详细, `RPT_ORG_SURVEY` (quoteCols 最新价/涨跌幅) |
//! | `stock_gddh_em`                       | `stock_gddh_em`               | Eastmoney datacenter    | 股东大会, `RPT_GENERALMEETING_DETAIL`               |
//! | `stock_qsjy_em`                       | `stock_qsjy_em`               | Eastmoney datacenter    | 券商业绩月报, `RPT_PERFORMANCE`                     |
//! | `stock_qbzf_em`                       | `stock_qbzf_em`               | Eastmoney datacenter    | 全部增发, `RPT_SEO_DETAIL` (quoteCol 最新价)         |
//! | `stock_zdhtmx_em`                     | `stock_zdhtmx_em`             | Eastmoney datacenter    | 重大合同明细, `RPTA_WEB_ZDHT_LIST` (token)          |
//! | `stock_zh_scale_comparison_em`        | `stock_zh_scale_comparison_em`| Eastmoney datacenter    | 同行比较-公司规模, `RPT_PCF10_INDUSTRY_MARKET`       |
//! | `stock_zh_a_gdhs_detail_em`           | `stock_zh_a_gdhs_detail_em`   | Eastmoney datacenter    | 股东户数详情, `RPT_HOLDERNUM_DET`                   |
//!
//! All 15 are pure-JSON HTTP (no JS signing, no HTML scraping, no encryption).
//! Field ids are mapped by explicit Eastmoney key (`fNNN`, uppercase report
//! column, or `quoteColumns` alias) rather than by column *position*, so the
//! ports stay offline-verifiable against fixtures.
//!
//! ## Skips (already ported elsewhere in this crate)
//!
//! Confirmed via `grep -rn 'pub async fn <name>' src/` before porting:
//! - `stock_hk_ggt_components_em` → `src/stock/hk.rs`
//! - `stock_individual_fund_flow` / `stock_market_fund_flow` /
//!   `stock_hsgt_fund_flow_summary_em` → `src/stock/flow.rs`
//! - `stock_ipo_tutor_em` / `stock_ipo_declare_em` / `stock_ipo_review_em` →
//!   `src/stock/fundamental/registration.rs`
//! - `stock_new_a_spot_em` → `src/stock/stock_hist_em.rs` (same 新股 board as
//!   `stock_zh_a_new_em`, which is ported here)
//! - `stock_zh_a_gdhs` → `src/stock/extra.rs` (the summary sibling of
//!   `stock_zh_a_gdhs_detail_em`, ported here)
//!
//! ## Deferred (not offline-verifiable)
//!
//! - `stock_fhps_em`, `stock_jgdy_tj_em`, `stock_ggcg_em`, `stock_pg_em`,
//!   `stock_zcfz_em`, `stock_report_em` (+ `_sb`/`_zb`), the
//!   `stock_zh_growth/valuation/dupont_comparison_em` trio, and the
//!   `stock_analyst_*` family: their akshare code uses positional
//!   `columns=ALL` plus a *positional* column rename (no key dict). Reproducing
//!   the exact header order requires an online header dump, so they are not
//!   ported here (would be faking the schema).
//! - `stock_zh_a_spot` / industry-concept spot and push2his CSV kline endpoints
//!   are covered by other modules (`spot.rs`, `fund_flow.rs`).

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};
use crate::core::json::*;

/// Eastmoney `clist/get` endpoint base (matches `src/stock/more.rs`).
const CLIST_BASE: &str = "https://push2.eastmoney.com/api/qt/clist/get";
/// Eastmoney datacenter `v1/get` endpoint base.
const DATACENTER_BASE: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";
/// Eastmoney datacenter (securities) `v1/get` endpoint base — used by the
/// peer-comparison reports (different host from `DATACENTER_BASE`).
const DATACENTER_SECURITIES_BASE: &str = "https://datacenter.eastmoney.com/securities/api/data/v1/get";
/// Static Eastmoney `ut` token (no JS signing required, ADR-0005).
const CLIST_UT: &str = "bd1d9ddb04089700cf9c27f6f7426281";
/// Default page size, mirroring akshare (`pz=100`).
const PAGE_SIZE: u32 = 100;

// ---------------------------------------------------------------------------
// Shared helpers (mirror src/stock/more.rs)
// ---------------------------------------------------------------------------


/// Validate an `YYYYMMDD` date string and reformat as `YYYY-MM-DD` (as akshare
/// feeds the datacenter `filter` / `REPORT_DATE`).
fn fmt_date8(date: &str, what: &str) -> Result<String> {
    if date.len() == 8 && date.bytes().all(|b| b.is_ascii_digit()) {
        Ok(format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8]))
    } else {
        Err(Error::InvalidParam(format!(
            "{what} must be 8 ASCII digits YYYYMMDD, got {date:?}"
        )))
    }
}

// ---------------------------------------------------------------------------
// Shared fetchers (pagination handled here; parsers stay pure)
// ---------------------------------------------------------------------------

/// Fetch all pages of an Eastmoney datacenter `v1/get` report.
///
/// Loops `pageNumber`/`pageNo`/`pageNum`/`p` until `result.pages` is reached.
/// `result: null` or `result.data: null` yields an empty `Vec` (akshare returns
/// an empty frame in those cases).
async fn datacenter_all(
    client: &Client,
    endpoint: &'static str,
    base: &str,
    mut params: Vec<(String, String)>,
) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    let mut pn: u32 = 1;
    loop {
        // (Re)apply the page markers each iteration.
        params.retain(|(k, _)| {
            !matches!(k.as_str(), "pageNumber" | "pageNo" | "pageNum" | "p")
        });
        let pn_s = pn.to_string();
        params.push(("pageNumber".into(), pn_s.clone()));
        params.push(("pageNo".into(), pn_s.clone()));
        params.push(("pageNum".into(), pn_s.clone()));
        params.push(("p".into(), pn_s));
        let q: Vec<(&str, &str)> =
            params.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let v = client
            .get_json(SOURCE_EASTMONEY, endpoint, base, &q)
            .await?;
        let result = match v.get("result") {
            Some(r) if !r.is_null() => r,
            _ => return Ok(out),
        };
        let pages = result.get("pages").and_then(|p| p.as_u64()).unwrap_or(1);
        let data = result.get("data").and_then(|d| d.as_array());
        match data {
            Some(arr) => out.extend(arr.iter().cloned()),
            None => return Ok(out),
        }
        if pn as u64 >= pages {
            break;
        }
        pn += 1;
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    }
    Ok(out)
}

/// Fetch one page of an Eastmoney `np-anotice` announcement list. The caller
/// (e.g. `stock_zh_kcb_report_em`) drives the `page_index` loop, mirroring
/// akshare which iterates `from_page..=to_page` fetching each page once.
async fn anotice_page(
    client: &Client,
    endpoint: &'static str,
    params: Vec<(String, String)>,
) -> Result<Vec<Value>> {
    let q: Vec<(&str, &str)> = params.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            endpoint,
            "https://np-anotice-stock.eastmoney.com/api/security/ann",
            &q,
        )
        .await?;
    let data = v.get("data").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: format!("missing data at {endpoint}"),
    })?;
    let list = data
        .get("list")
        .and_then(|l| l.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: format!("missing data.list at {endpoint}"),
        })?;
    Ok(list.to_vec())
}

/// Fetch all pages of an Eastmoney `clist/get` query (`fltt` is caller-supplied:
/// `"2"` = human-readable numbers, `"1"` = scaled ints needing ÷1000/÷100).
async fn fetch_clist_pages(
    client: &Client,
    endpoint: &'static str,
    fs: &str,
    fid: &str,
    fields: &str,
    fltt: &str,
) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    let mut pn: u32 = 1;
    loop {
        let pn_s = pn.to_string();
        let pz_s = PAGE_SIZE.to_string();
        let params = [
            ("pn", pn_s.as_str()),
            ("pz", pz_s.as_str()),
            ("po", "1"),
            ("np", "1"),
            ("ut", CLIST_UT),
            ("fltt", fltt),
            ("invt", "2"),
            ("fid", fid),
            ("fs", fs),
            ("fields", fields),
            ("dect", "1"),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, endpoint, CLIST_BASE, &params)
            .await?;
        let total = v
            .get("data")
            .and_then(|d| d.get("total"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        let diff = v
            .get("data")
            .and_then(|d| d.get("diff"))
            .and_then(|d| d.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: format!("missing data.diff at {endpoint}"),
            })?;
        let n = diff.len();
        out.extend(diff.iter().cloned());
        let fetched = (pn as u64 - 1) * PAGE_SIZE as u64 + n as u64;
        if n == 0 || fetched >= total {
            break;
        }
        pn += 1;
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    }
    Ok(out)
}

// ===========================================================================
// 1. stock_zh_ah_spot_em — AH 股比价实时行情 (fltt=1)
// ===========================================================================

/// One AH-comparison row, port of `stock_zh_ah_spot_em`.
///
/// `fltt=1` means prices are ×1000 and percentages ×100; the parser divides
/// them back. Field ids are Eastmoney `clist/get` `fNNN` keys.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockZhAhSpotEmRow {
    /// `f193` 名称
    pub name: String,
    /// `f12` H 股代码
    pub h_code: String,
    /// `f2` 最新价-HKD (÷1000)
    pub h_price: Option<f64>,
    /// `f3` H 股-涨跌幅 (÷100)
    pub h_pct: Option<f64>,
    /// `f191` A 股代码
    pub a_code: String,
    /// `f186` 最新价-RMB (÷1000)
    pub a_price: Option<f64>,
    /// `f187` A 股-涨跌幅 (÷100)
    pub a_pct: Option<f64>,
    /// `f189` 比价 (A/H) (÷100)
    pub ratio: Option<f64>,
    /// `f188` 溢价 (H/A) (÷100)
    pub premium: Option<f64>,
    pub source: &'static str,
}

const FIELDS_AH: &str =
    "f193,f191,f192,f12,f13,f14,f1,f2,f4,f3,f152,f186,f190,f187,f189,f188";

/// Port of `stock_zh_ah_spot_em()`.
pub async fn stock_zh_ah_spot_em(client: &Client) -> Result<Vec<StockZhAhSpotEmRow>> {
    let items = fetch_clist_pages(
        client,
        "stock_zh_ah_spot_em",
        "b:DLMK0101",
        "f3",
        FIELDS_AH,
        "1",
    )
    .await?;
    parse_ah(&items)
}

/// Parse a `clist/get` `diff` array into [`StockZhAhSpotEmRow`]s.
pub(crate) fn parse_ah(items: &[Value]) -> Result<Vec<StockZhAhSpotEmRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(StockZhAhSpotEmRow {
            name: opt_str_or(item, "f193", ""),
            h_code: opt_str_or(item, "f12", ""),
            h_price: opt_f64(item, "f2").map(|x| x / 1000.0),
            h_pct: opt_f64(item, "f3").map(|x| x / 100.0),
            a_code: opt_str_or(item, "f191", ""),
            a_price: opt_f64(item, "f186").map(|x| x / 1000.0),
            a_pct: opt_f64(item, "f187").map(|x| x / 100.0),
            ratio: opt_f64(item, "f189").map(|x| x / 100.0),
            premium: opt_f64(item, "f188").map(|x| x / 100.0),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// 2. stock_hsgt_sh_hk_spot_em — 沪港通·港股通(沪>港) (fltt=1)
// ===========================================================================

/// One Shanghai-HK connect (沪>港) stock row, port of
/// `stock_hsgt_sh_hk_spot_em`. `fltt=1` scaled: prices/amounts ÷1000,
/// pct ÷100, volume/amount (手/元) ÷1e8.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockHsgtShHkSpotEmRow {
    /// `f12` 代码
    pub code: String,
    /// `f14` 名称
    pub name: String,
    /// `f2` 最新价 (÷1000)
    pub price: Option<f64>,
    /// `f4` 涨跌额 (÷1000)
    pub change: Option<f64>,
    /// `f3` 涨跌幅 (÷100)
    pub pct_change: Option<f64>,
    /// `f17` 今开 (÷1000)
    pub open: Option<f64>,
    /// `f15` 最高 (÷1000)
    pub high: Option<f64>,
    /// `f16` 最低 (÷1000)
    pub low: Option<f64>,
    /// `f18` 昨收 (÷1000)
    pub pre_close: Option<f64>,
    /// `f5` 成交量 (手, ÷1e8)
    pub volume: Option<f64>,
    /// `f6` 成交额 (元, ÷1e8)
    pub amount: Option<f64>,
    pub source: &'static str,
}

const FIELDS_SHHK: &str = "f12,f13,f14,f19,f1,f2,f4,f3,f152,f17,f18,f15,f16,f5,f6";

/// Port of `stock_hsgt_sh_hk_spot_em()`.
pub async fn stock_hsgt_sh_hk_spot_em(
    client: &Client,
) -> Result<Vec<StockHsgtShHkSpotEmRow>> {
    let items = fetch_clist_pages(
        client,
        "stock_hsgt_sh_hk_spot_em",
        "b:DLMK0144",
        "f12",
        FIELDS_SHHK,
        "1",
    )
    .await?;
    parse_sh_hk(&items)
}

/// Parse a `clist/get` `diff` array into [`StockHsgtShHkSpotEmRow`]s.
pub(crate) fn parse_sh_hk(items: &[Value]) -> Result<Vec<StockHsgtShHkSpotEmRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(StockHsgtShHkSpotEmRow {
            code: opt_str_or(item, "f12", ""),
            name: opt_str_or(item, "f14", ""),
            price: opt_f64(item, "f2").map(|x| x / 1000.0),
            change: opt_f64(item, "f4").map(|x| x / 1000.0),
            pct_change: opt_f64(item, "f3").map(|x| x / 100.0),
            open: opt_f64(item, "f17").map(|x| x / 1000.0),
            high: opt_f64(item, "f15").map(|x| x / 1000.0),
            low: opt_f64(item, "f16").map(|x| x / 1000.0),
            pre_close: opt_f64(item, "f18").map(|x| x / 1000.0),
            volume: opt_f64(item, "f5").map(|x| x / 100_000_000.0),
            amount: opt_f64(item, "f6").map(|x| x / 100_000_000.0),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// 3. stock_zh_kcb_report_em — 科创板报告公告列表
// ===========================================================================

/// One 科创板 (Sci-Tech board) announcement row, port of
/// `stock_zh_kcb_report_em`. Source items are `np-anotice` `data.list` entries
/// with a nested `codes` array and optional `columns` array.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockZhKcbReportEmRow {
    /// `codes[0].stock_code` 代码
    pub code: String,
    /// `codes[0].short_name` 名称
    pub name: String,
    /// `title` 公告标题
    pub title: String,
    /// `columns[0].column_name` 公告类型 (optional)
    pub ann_type: Option<String>,
    /// `notice_date` 公告日期
    pub notice_date: String,
    /// `art_code` 公告代码
    pub art_code: String,
    pub source: &'static str,
}

/// Port of `stock_zh_kcb_report_em(from_page, to_page)`.
pub async fn stock_zh_kcb_report_em(
    client: &Client,
    from_page: i64,
    to_page: i64,
) -> Result<Vec<StockZhKcbReportEmRow>> {
    let to = to_page.max(from_page);
    let mut out = Vec::new();
    for p in from_page..=to {
        let params = vec![
            ("sr".into(), "-1".into()),
            ("page_size".into(), "100".into()),
            ("page_index".into(), p.to_string()),
            ("ann_type".into(), "KCB".into()),
            ("client_source".into(), "web".into()),
            ("f_node".into(), "0".into()),
            ("s_node".into(), "0".into()),
        ];
        let items = anotice_page(client, "stock_zh_kcb_report_em", params).await?;
        out.extend(parse_kcb(&items)?);
    }
    Ok(out)
}

/// Parse `np-anotice` `data.list` items into [`StockZhKcbReportEmRow`]s.
pub(crate) fn parse_kcb(items: &[Value]) -> Result<Vec<StockZhKcbReportEmRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let codes = item
            .get("codes")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first());
        let code = codes
            .and_then(|c| c.get("stock_code"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let name = codes
            .and_then(|c| c.get("short_name"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let ann_type = item
            .get("columns")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .and_then(|c| c.get("column_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        out.push(StockZhKcbReportEmRow {
            code,
            name,
            title: opt_str_or(item, "title", ""),
            ann_type,
            notice_date: opt_str_or(item, "notice_date", ""),
            art_code: opt_str_or(item, "art_code", ""),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// 4. stock_repurchase_em — 股票回购数据 (RPTA_WEB_GETHGLIST_NEW)
// ===========================================================================

/// One stock-repurchase row, port of `stock_repurchase_em`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockRepurchaseEmRow {
    /// `DIM_SCODE` 股票代码
    pub code: String,
    /// `SECURITYSHORTNAME` 股票简称
    pub name: String,
    /// `NEWPRICE` 最新价
    pub new_price: Option<f64>,
    /// `REPURPRICECAP` 计划回购价格区间
    pub repur_price_cap: Option<String>,
    /// `REPURNUMLOWER` 计划回购数量区间-下限
    pub repur_num_lower: Option<f64>,
    /// `REPURNUMCAP` 计划回购数量区间-上限
    pub repur_num_cap: Option<f64>,
    /// `ZSZXX` 占公告前一日总股本比例-下限
    pub ratio_lower: Option<f64>,
    /// `ZSZSX` 占公告前一日总股本比例-上限
    pub ratio_cap: Option<f64>,
    /// `JEXX` 计划回购金额区间-下限
    pub amount_lower: Option<f64>,
    /// `JESX` 计划回购金额区间-上限
    pub amount_cap: Option<f64>,
    /// `DIM_TRADEDATE` 回购起始时间
    pub start_date: String,
    /// `REPURPROGRESS` 实施进度 (code, e.g. 001..006)
    pub progress: Option<String>,
    /// `REPURPRICELOWER1` 已回购股份价格区间-下限
    pub repur_price_lower1: Option<f64>,
    /// `REPURPRICECAP1` 已回购股份价格区间-上限
    pub repur_price_cap1: Option<f64>,
    /// `REPURNUM` 已回购股份数量
    pub repur_num: Option<f64>,
    /// `REPURAMOUNT` 已回购金额
    pub repur_amount: Option<f64>,
    /// `UPDATEDATE` 最新公告日期
    pub update_date: String,
    pub source: &'static str,
}

/// Port of `stock_repurchase_em()`.
pub async fn stock_repurchase_em(client: &Client) -> Result<Vec<StockRepurchaseEmRow>> {
    let params = vec![
        ("sortColumns".into(), "UPD,DIM_DATE,DIM_SCODE".into()),
        ("sortTypes".into(), "-1,-1,-1".into()),
        ("pageSize".into(), "500".into()),
        ("reportName".into(), "RPTA_WEB_GETHGLIST_NEW".into()),
        ("columns".into(), "ALL".into()),
        ("source".into(), "WEB".into()),
    ];
    let items = datacenter_all(client, "stock_repurchase_em", DATACENTER_BASE, params).await?;
    parse_repurchase(&items)
}

/// Parse a `result.data` array into [`StockRepurchaseEmRow`]s.
pub(crate) fn parse_repurchase(items: &[Value]) -> Result<Vec<StockRepurchaseEmRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(StockRepurchaseEmRow {
            code: opt_str_or(item, "DIM_SCODE", ""),
            name: opt_str_or(item, "SECURITYSHORTNAME", ""),
            new_price: opt_f64(item, "NEWPRICE"),
            repur_price_cap: item
                .get("REPURPRICECAP")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            repur_num_lower: opt_f64(item, "REPURNUMLOWER"),
            repur_num_cap: opt_f64(item, "REPURNUMCAP"),
            ratio_lower: opt_f64(item, "ZSZXX"),
            ratio_cap: opt_f64(item, "ZSZSX"),
            amount_lower: opt_f64(item, "JEXX"),
            amount_cap: opt_f64(item, "JESX"),
            start_date: opt_str_or(item, "DIM_TRADEDATE", ""),
            progress: match item.get("REPURPROGRESS") {
                Some(Value::String(s)) => Some(s.clone()),
                Some(Value::Number(n)) => Some(n.to_string()),
                _ => None,
            },
            repur_price_lower1: opt_f64(item, "REPURPRICELOWER1"),
            repur_price_cap1: opt_f64(item, "REPURPRICECAP1"),
            repur_num: opt_f64(item, "REPURNUM"),
            repur_amount: opt_f64(item, "REPURAMOUNT"),
            update_date: opt_str_or(item, "UPDATEDATE", ""),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// 5. stock_gsrl_gsdt_em — 股市日历·公司动态 (RPT_ORGOP_ALL)
// ===========================================================================

/// One company-dynamics (公司动态) row, port of `stock_gsrl_gsdt_em`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockGsrlGsdtEmRow {
    /// `SECURITY_CODE` 代码
    pub code: String,
    /// `SECURITY_NAME_ABBR` 简称
    pub name: String,
    /// `EVENT_TYPE` 事件类型
    pub event_type: String,
    /// `EVENT_CONTENT` 具体事项
    pub event_content: String,
    /// `TRADE_DATE` 交易日
    pub trade_date: String,
    pub source: &'static str,
}

/// Port of `stock_gsrl_gsdt_em(date)`.
pub async fn stock_gsrl_gsdt_em(client: &Client, date: &str) -> Result<Vec<StockGsrlGsdtEmRow>> {
    let d = fmt_date8(date, "stock_gsrl_gsdt_em date")?;
    let filter = format!("(TRADE_DATE='{d}')");
    let params = vec![
        ("sortColumns".into(), "SECURITY_CODE".into()),
        ("sortTypes".into(), "1".into()),
        ("pageSize".into(), "5000".into()),
        (
            "columns".into(),
            "SECURITY_CODE,SECUCODE,SECURITY_NAME_ABBR,EVENT_TYPE,EVENT_CONTENT,TRADE_DATE".into(),
        ),
        ("source".into(), "WEB".into()),
        ("client".into(), "WEB".into()),
        ("reportName".into(), "RPT_ORGOP_ALL".into()),
        ("filter".into(), filter),
    ];
    let items = datacenter_all(client, "stock_gsrl_gsdt_em", DATACENTER_BASE, params).await?;
    parse_gsrl(&items)
}

/// Parse a `result.data` array into [`StockGsrlGsdtEmRow`]s.
pub(crate) fn parse_gsrl(items: &[Value]) -> Result<Vec<StockGsrlGsdtEmRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(StockGsrlGsdtEmRow {
            code: opt_str_or(item, "SECURITY_CODE", ""),
            name: opt_str_or(item, "SECURITY_NAME_ABBR", ""),
            event_type: opt_str_or(item, "EVENT_TYPE", ""),
            event_content: opt_str_or(item, "EVENT_CONTENT", ""),
            trade_date: opt_str_or(item, "TRADE_DATE", ""),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// 6. stock_zh_a_new_em — 新股板块 (fltt=2)
// ===========================================================================

/// One new-share (新股) quote row, port of `stock_zh_a_new_em`.
///
/// Same `clist/get` `fNNN` schema as the ST board (`more.rs`); `fltt=2` means
/// prices/percentages are already human-readable.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockZhANewEmRow {
    /// `f12` 代码
    pub code: String,
    /// `f13` 市场 (1=沪, 0=深)
    pub market: Option<f64>,
    /// `f14` 名称
    pub name: String,
    /// `f2` 最新价
    pub price: Option<f64>,
    /// `f3` 涨跌幅 (%)
    pub pct_change: Option<f64>,
    /// `f4` 涨跌额
    pub change: Option<f64>,
    /// `f5` 成交量 (手)
    pub volume: Option<f64>,
    /// `f6` 成交额 (元)
    pub amount: Option<f64>,
    /// `f7` 振幅 (%)
    pub amplitude: Option<f64>,
    /// `f8` 换手率 (%)
    pub turnover: Option<f64>,
    /// `f9` 市盈率-动态
    pub pe_ttm: Option<f64>,
    /// `f10` 量比
    pub volume_ratio: Option<f64>,
    /// `f15` 最高
    pub high: Option<f64>,
    /// `f16` 最低
    pub low: Option<f64>,
    /// `f17` 今开
    pub open: Option<f64>,
    /// `f18` 昨收
    pub pre_close: Option<f64>,
    /// `f20` 总市值
    pub total_mktcap: Option<f64>,
    /// `f21` 流通市值
    pub float_mktcap: Option<f64>,
    /// `f23` 市净率
    pub pb: Option<f64>,
    pub source: &'static str,
}

const FIELDS_NEW: &str = "f2,f3,f4,f5,f6,f7,f8,f9,f10,f12,f13,f14,f15,f16,f17,\
f18,f20,f21,f23,f24,f25,f22,f11,f62,f128,f136,f115,f152";

/// Port of `stock_zh_a_new_em()`.
pub async fn stock_zh_a_new_em(client: &Client) -> Result<Vec<StockZhANewEmRow>> {
    let items = fetch_clist_pages(
        client,
        "stock_zh_a_new_em",
        "m:0 f:8,m:1 f:8",
        "f26",
        FIELDS_NEW,
        "2",
    )
    .await?;
    parse_new(&items)
}

/// Parse a `clist/get` `diff` array into [`StockZhANewEmRow`]s.
pub(crate) fn parse_new(items: &[Value]) -> Result<Vec<StockZhANewEmRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let code = opt_str_or(item, "f12", "");
        let name = opt_str_or(item, "f14", "");
        if code.is_empty() || name.is_empty() {
            continue;
        }
        out.push(StockZhANewEmRow {
            code,
            market: opt_f64(item, "f13"),
            name,
            price: opt_f64(item, "f2"),
            pct_change: opt_f64(item, "f3"),
            change: opt_f64(item, "f4"),
            volume: opt_f64(item, "f5"),
            amount: opt_f64(item, "f6"),
            amplitude: opt_f64(item, "f7"),
            turnover: opt_f64(item, "f8"),
            pe_ttm: opt_f64(item, "f9"),
            volume_ratio: opt_f64(item, "f10"),
            high: opt_f64(item, "f15"),
            low: opt_f64(item, "f16"),
            open: opt_f64(item, "f17"),
            pre_close: opt_f64(item, "f18"),
            total_mktcap: opt_f64(item, "f20"),
            float_mktcap: opt_f64(item, "f21"),
            pb: opt_f64(item, "f23"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// 7. stock_hold_management_detail_em — 高管持股变动明细 (RPT_EXECUTIVE_HOLD_DETAILS)
// ===========================================================================

/// One executive-holding change row, port of `stock_hold_management_detail_em`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockHoldManagementDetailEmRow {
    /// `CHANGE_DATE` 日期
    pub change_date: String,
    /// `SECURITY_CODE` 代码
    pub code: String,
    /// `SECURITY_NAME` 名称
    pub name: String,
    /// `PERSON_NAME` 变动人
    pub person: String,
    /// `CHANGE_SHARES` 变动股数
    pub change_shares: Option<f64>,
    /// `AVERAGE_PRICE` 成交均价
    pub avg_price: Option<f64>,
    /// `CHANGE_AMOUNT` 变动金额
    pub change_amount: Option<f64>,
    /// `CHANGE_REASON` 变动原因
    pub change_reason: String,
    /// `CHANGE_RATIO` 变动比例
    pub change_ratio: Option<f64>,
    /// `CHANGE_AFTER_HOLDNUM` 变动后持股数
    pub hold_after: Option<f64>,
    /// `HOLD_TYPE` 持股种类
    pub hold_type: String,
    /// `DSE_PERSON_NAME` 董监高人员姓名
    pub dse_person: String,
    /// `POSITION_NAME` 职务
    pub position: String,
    /// `PERSON_DSE_RELATION` 变动人与董监高的关系
    pub person_dse_relation: String,
    /// `BEGIN_HOLD_NUM` 开始时持有
    pub begin_hold: Option<f64>,
    /// `END_HOLD_NUM` 结束后持有
    pub end_hold: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_hold_management_detail_em()`.
pub async fn stock_hold_management_detail_em(
    client: &Client,
) -> Result<Vec<StockHoldManagementDetailEmRow>> {
    let params = vec![
        ("reportName".into(), "RPT_EXECUTIVE_HOLD_DETAILS".into()),
        ("columns".into(), "ALL".into()),
        ("quoteColumns".into(), "".into()),
        ("filter".into(), "".into()),
        ("pageSize".into(), "5000".into()),
        ("sortTypes".into(), "-1,1,1".into()),
        ("sortColumns".into(), "CHANGE_DATE,SECURITY_CODE,PERSON_NAME".into()),
        ("source".into(), "WEB".into()),
        ("client".into(), "WEB".into()),
    ];
    let items = datacenter_all(client, "stock_hold_management_detail_em", DATACENTER_BASE, params).await?;
    parse_hold_management(&items)
}

/// Parse a `result.data` array into [`StockHoldManagementDetailEmRow`]s.
pub(crate) fn parse_hold_management(
    items: &[Value],
) -> Result<Vec<StockHoldManagementDetailEmRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(StockHoldManagementDetailEmRow {
            change_date: opt_str_or(item, "CHANGE_DATE", ""),
            code: opt_str_or(item, "SECURITY_CODE", ""),
            name: opt_str_or(item, "SECURITY_NAME", ""),
            person: opt_str_or(item, "PERSON_NAME", ""),
            change_shares: opt_f64(item, "CHANGE_SHARES"),
            avg_price: opt_f64(item, "AVERAGE_PRICE"),
            change_amount: opt_f64(item, "CHANGE_AMOUNT"),
            change_reason: opt_str_or(item, "CHANGE_REASON", ""),
            change_ratio: opt_f64(item, "CHANGE_RATIO"),
            hold_after: opt_f64(item, "CHANGE_AFTER_HOLDNUM"),
            hold_type: opt_str_or(item, "HOLD_TYPE", ""),
            dse_person: opt_str_or(item, "DSE_PERSON_NAME", ""),
            position: opt_str_or(item, "POSITION_NAME", ""),
            person_dse_relation: opt_str_or(item, "PERSON_DSE_RELATION", ""),
            begin_hold: opt_f64(item, "BEGIN_HOLD_NUM"),
            end_hold: opt_f64(item, "END_HOLD_NUM"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// 8. stock_yysj_em — 预约披露时间 (RPT_PUBLIC_BS_APPOIN)
// ===========================================================================

/// One earnings-disclosure appointment row, port of `stock_yysj_em`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockYysjEmRow {
    /// `SECURITY_CODE` 股票代码
    pub code: String,
    /// `SECURITY_NAME_ABBR` 股票简称
    pub name: String,
    /// `FIRST_APPOINT_DATE` 首次预约时间
    pub first_appoint: String,
    /// `FIRST_CHANGE_DATE` 一次变更日期
    pub first_change: Option<String>,
    /// `SECOND_CHANGE_DATE` 二次变更日期
    pub second_change: Option<String>,
    /// `THIRD_CHANGE_DATE` 三次变更日期
    pub third_change: Option<String>,
    /// `ACTUAL_PUBLISH_DATE` 实际披露时间
    pub actual_publish: Option<String>,
    pub source: &'static str,
}

/// Valid `symbol` choices for `stock_yysj_em`.
const YYSJ_SYMBOLS: &[&str] = &[
    "沪深A股",
    "沪市A股",
    "科创板",
    "深市A股",
    "创业板",
    "京市A股",
    "ST板",
];

/// Build the `filter` for `stock_yysj_em` from `symbol` and a `YYYY-MM-DD` date.
fn yysj_filter(symbol: &str, date: &str) -> Result<String> {
    let base = format!("(SECURITY_TYPE_CODE in (\"058001001\",\"058001008\"))(TRADE_MARKET_CODE!=\"069001017\")(REPORT_DATE='{date}')");
    let filter = match symbol {
        "沪深A股" => base,
        "沪市A股" => format!(
            "(SECURITY_TYPE_CODE in (\"058001001\",\"058001008\"))(TRADE_MARKET_CODE in (\"069001001001\",\"069001001003\",\"069001001006\"))(REPORT_DATE='{date}')"
        ),
        "科创板" => format!(
            "(SECURITY_TYPE_CODE in (\"058001001\",\"058001008\"))(TRADE_MARKET_CODE=\"069001001006\")(REPORT_DATE='{date}')"
        ),
        "深市A股" => format!(
            "(SECURITY_TYPE_CODE=\"058001001\")(TRADE_MARKET_CODE in (\"069001002001\",\"069001002002\",\"069001002003\",\"069001002005\"))(REPORT_DATE='{date}')"
        ),
        "创业板" => format!(
            "(SECURITY_TYPE_CODE=\"058001001\")(TRADE_MARKET_CODE=\"069001002002\")(REPORT_DATE='{date}')"
        ),
        "京市A股" => format!(
            "(TRADE_MARKET_CODE=\"069001017\")(REPORT_DATE='{date}')"
        ),
        "ST板" => format!(
            "(TRADE_MARKET_CODE in(\"069001001003\",\"069001002005\"))(REPORT_DATE='{date}')"
        ),
        other => {
            return Err(Error::InvalidParam(format!(
                "stock_yysj_em: symbol must be one of {YYSJ_SYMBOLS:?}, got {other:?}"
            )))
        }
    };
    Ok(filter)
}

/// Port of `stock_yysj_em(symbol, date)`. `date` is `YYYYMMDD`.
pub async fn stock_yysj_em(
    client: &Client,
    symbol: &str,
    date: &str,
) -> Result<Vec<StockYysjEmRow>> {
    if !YYSJ_SYMBOLS.contains(&symbol) {
        return Err(Error::InvalidParam(format!(
            "stock_yysj_em: symbol must be one of {YYSJ_SYMBOLS:?}, got {symbol:?}"
        )));
    }
    let d = fmt_date8(date, "stock_yysj_em date")?;
    let filter = yysj_filter(symbol, &d)?;
    let params = vec![
        ("sortColumns".into(), "FIRST_APPOINT_DATE,SECURITY_CODE".into()),
        ("sortTypes".into(), "1,1".into()),
        ("pageSize".into(), "500".into()),
        ("reportName".into(), "RPT_PUBLIC_BS_APPOIN".into()),
        ("columns".into(), "ALL".into()),
        ("filter".into(), filter),
    ];
    let items = datacenter_all(client, "stock_yysj_em", DATACENTER_BASE, params).await?;
    parse_yysj(&items)
}

/// Parse a `result.data` array into [`StockYysjEmRow`]s.
pub(crate) fn parse_yysj(items: &[Value]) -> Result<Vec<StockYysjEmRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(StockYysjEmRow {
            code: opt_str_or(item, "SECURITY_CODE", ""),
            name: opt_str_or(item, "SECURITY_NAME_ABBR", ""),
            first_appoint: opt_str_or(item, "FIRST_APPOINT_DATE", ""),
            first_change: item.get("FIRST_CHANGE_DATE").and_then(|v| v.as_str()).map(|s| s.to_string()),
            second_change: item.get("SECOND_CHANGE_DATE").and_then(|v| v.as_str()).map(|s| s.to_string()),
            third_change: item.get("THIRD_CHANGE_DATE").and_then(|v| v.as_str()).map(|s| s.to_string()),
            actual_publish: item.get("ACTUAL_PUBLISH_DATE").and_then(|v| v.as_str()).map(|s| s.to_string()),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// 9. stock_jgdy_detail_em — 机构调研详细 (RPT_ORG_SURVEY)
// ===========================================================================

/// One institutional-research (机构调研) detail row, port of
/// `stock_jgdy_detail_em`. `CLOSE_PRICE` / `CHANGE_RATE` come from the
/// `quoteColumns` alias (`f2~01~SECURITY_CODE~CLOSE_PRICE` etc.).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockJgdyDetailEmRow {
    /// `SECURITY_CODE` 代码
    pub code: String,
    /// `SECURITY_NAME_ABBR` 名称
    pub name: String,
    /// `CLOSE_PRICE` 最新价 (quoteColumns)
    pub close_price: Option<f64>,
    /// `CHANGE_RATE` 涨跌幅 (quoteColumns)
    pub change_rate: Option<f64>,
    /// `RECEIVE_OBJECT` 调研机构
    pub receive_object: String,
    /// `ORG_TYPE` 机构类型
    pub org_type: String,
    /// `INVESTIGATORS` 调研人员
    pub investigators: String,
    /// `RECEPTIONIST` 接待人员
    pub receptionist: String,
    /// `RECEIVE_WAY_EXPLAIN` 接待方式
    pub receive_way: String,
    /// `RECEIVE_PLACE` 接待地点
    pub receive_place: String,
    /// `RECEIVE_START_DATE` 调研日期
    pub receive_start_date: String,
    /// `NOTICE_DATE` 公告日期
    pub notice_date: String,
    pub source: &'static str,
}

/// Port of `stock_jgdy_detail_em(date)`. `date` is `YYYYMMDD` (调研开始日下限).
pub async fn stock_jgdy_detail_em(
    client: &Client,
    date: &str,
) -> Result<Vec<StockJgdyDetailEmRow>> {
    let d = fmt_date8(date, "stock_jgdy_detail_em date")?;
    let filter = format!("(IS_SOURCE=\"1\")(RECEIVE_START_DATE>'{d}')");
    let params = vec![
        ("sortColumns".into(), "NOTICE_DATE,RECEIVE_START_DATE,SECURITY_CODE,NUMBERNEW".into()),
        ("sortTypes".into(), "-1,-1,1,-1".into()),
        ("pageSize".into(), "50".into()),
        ("reportName".into(), "RPT_ORG_SURVEY".into()),
        ("columns".into(), "SECUCODE,SECURITY_CODE,SECURITY_NAME_ABBR,NOTICE_DATE,RECEIVE_START_DATE,RECEIVE_OBJECT,RECEIVE_PLACE,RECEIVE_WAY_EXPLAIN,INVESTIGATORS,RECEPTIONIST,ORG_TYPE".into()),
        ("quoteColumns".into(), "f2~01~SECURITY_CODE~CLOSE_PRICE,f3~01~SECURITY_CODE~CHANGE_RATE".into()),
        ("quoteType".into(), "0".into()),
        ("source".into(), "WEB".into()),
        ("client".into(), "WEB".into()),
        ("filter".into(), filter),
    ];
    let items = datacenter_all(client, "stock_jgdy_detail_em", DATACENTER_BASE, params).await?;
    parse_jgdy(&items)
}

/// Parse a `result.data` array into [`StockJgdyDetailEmRow`]s.
pub(crate) fn parse_jgdy(items: &[Value]) -> Result<Vec<StockJgdyDetailEmRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(StockJgdyDetailEmRow {
            code: opt_str_or(item, "SECURITY_CODE", ""),
            name: opt_str_or(item, "SECURITY_NAME_ABBR", ""),
            close_price: opt_f64(item, "CLOSE_PRICE"),
            change_rate: opt_f64(item, "CHANGE_RATE"),
            receive_object: opt_str_or(item, "RECEIVE_OBJECT", ""),
            org_type: opt_str_or(item, "ORG_TYPE", ""),
            investigators: opt_str_or(item, "INVESTIGATORS", ""),
            receptionist: opt_str_or(item, "RECEPTIONIST", ""),
            receive_way: opt_str_or(item, "RECEIVE_WAY_EXPLAIN", ""),
            receive_place: opt_str_or(item, "RECEIVE_PLACE", ""),
            receive_start_date: opt_str_or(item, "RECEIVE_START_DATE", ""),
            notice_date: opt_str_or(item, "NOTICE_DATE", ""),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// 10. stock_gddh_em — 股东大会 (RPT_GENERALMEETING_DETAIL)
// ===========================================================================

/// One general-meeting (股东大会) row, port of `stock_gddh_em`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockGddhEmRow {
    /// `SECURITY_CODE` 代码
    pub code: String,
    /// `SECURITY_NAME_ABBR` 简称
    pub name: String,
    /// `MEETING_TITLE` 股东大会名称
    pub meeting_title: String,
    /// `START_ADJUST_DATE` 召开开始日
    pub start_date: String,
    /// `EQUITY_RECORD_DATE` 股权登记日
    pub equity_record_date: String,
    /// `ONSITE_RECORD_DATE` 现场登记日
    pub onsite_record_date: String,
    /// `DECISION_NOTICE_DATE` 决议公告日
    pub decision_notice_date: String,
    /// `NOTICE_DATE` 公告日
    pub notice_date: String,
    /// `WEB_START_DATE` 网络投票时间-开始日
    pub web_start_date: String,
    /// `WEB_END_DATE` 网络投票时间-结束日
    pub web_end_date: String,
    /// `SERIAL_NUM` 序列号
    pub serial_num: Option<String>,
    /// `PROPOSAL` 提案
    pub proposal: String,
    pub source: &'static str,
}

/// Port of `stock_gddh_em()`. `filter` is fixed `(IS_LASTDATE="1")`.
pub async fn stock_gddh_em(client: &Client) -> Result<Vec<StockGddhEmRow>> {
    let params = vec![
        ("sortColumns".into(), "NOTICE_DATE".into()),
        ("sortTypes".into(), "-1".into()),
        ("pageSize".into(), "500".into()),
        ("reportName".into(), "RPT_GENERALMEETING_DETAIL".into()),
        ("columns".into(), "SECURITY_CODE,SECURITY_NAME_ABBR,MEETING_TITLE,START_ADJUST_DATE,EQUITY_RECORD_DATE,ONSITE_RECORD_DATE,DECISION_NOTICE_DATE,NOTICE_DATE,WEB_START_DATE,WEB_END_DATE,SERIAL_NUM,PROPOSAL".into()),
        ("filter".into(), "(IS_LASTDATE=\"1\")".into()),
        ("source".into(), "WEB".into()),
        ("client".into(), "WEB".into()),
    ];
    let items = datacenter_all(client, "stock_gddh_em", DATACENTER_BASE, params).await?;
    parse_gddh(&items)
}

/// Parse a `result.data` array into [`StockGddhEmRow`]s.
pub(crate) fn parse_gddh(items: &[Value]) -> Result<Vec<StockGddhEmRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(StockGddhEmRow {
            code: opt_str_or(item, "SECURITY_CODE", ""),
            name: opt_str_or(item, "SECURITY_NAME_ABBR", ""),
            meeting_title: opt_str_or(item, "MEETING_TITLE", ""),
            start_date: opt_str_or(item, "START_ADJUST_DATE", ""),
            equity_record_date: opt_str_or(item, "EQUITY_RECORD_DATE", ""),
            onsite_record_date: opt_str_or(item, "ONSITE_RECORD_DATE", ""),
            decision_notice_date: opt_str_or(item, "DECISION_NOTICE_DATE", ""),
            notice_date: opt_str_or(item, "NOTICE_DATE", ""),
            web_start_date: opt_str_or(item, "WEB_START_DATE", ""),
            web_end_date: opt_str_or(item, "WEB_END_DATE", ""),
            serial_num: match item.get("SERIAL_NUM") {
                Some(Value::String(s)) => Some(s.clone()),
                Some(Value::Number(n)) => Some(n.to_string()),
                _ => None,
            },
            proposal: opt_str_or(item, "PROPOSAL", ""),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// 11. stock_qsjy_em — 券商业绩月报 (RPT_PERFORMANCE)
// ===========================================================================

/// One brokerage monthly-performance row, port of `stock_qsjy_em`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockQsjyEmRow {
    /// `SECURITY_CODE` 代码
    pub code: String,
    /// `SECURITY_NAME_ABBR` 简称
    pub name: String,
    /// `END_DATE` 数据月份
    pub end_date: String,
    /// `NETPROFIT` 当月净利润
    pub net_profit: Option<f64>,
    /// `NP_YOY` 当月净利润-同比
    pub np_yoy: Option<f64>,
    /// `NP_QOQ` 当月净利润-环比
    pub np_qoq: Option<f64>,
    /// `ACCUMPROFIT` 当年累计净利润
    pub accum_profit: Option<f64>,
    /// `ACCUMPROFIT_YOY` 当年累计净利润-同比
    pub accum_profit_yoy: Option<f64>,
    /// `OPERATE_INCOME` 当月营业收入
    pub operate_income: Option<f64>,
    /// `OI_YOY` 当月营业收入-同比
    pub oi_yoy: Option<f64>,
    /// `OI_QOQ` 当月营业收入-环比
    pub oi_qoq: Option<f64>,
    /// `ACCUMOI` 当年累计营业收入
    pub accum_oi: Option<f64>,
    /// `ACCUMOI_YOY` 当年累计营业收入-同比
    pub accum_oi_yoy: Option<f64>,
    /// `NET_ASSETS` 净资产
    pub net_assets: Option<f64>,
    /// `NA_YOY` 净资产-同比
    pub na_yoy: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_qsjy_em(date)`. `date` is `YYYYMMDD`.
pub async fn stock_qsjy_em(client: &Client, date: &str) -> Result<Vec<StockQsjyEmRow>> {
    let d = fmt_date8(date, "stock_qsjy_em date")?;
    let filter = format!("(END_DATE='{d}')");
    let params = vec![
        ("sortColumns".into(), "END_DATE".into()),
        ("sortTypes".into(), "-1".into()),
        ("pageSize".into(), "500".into()),
        ("reportName".into(), "RPT_PERFORMANCE".into()),
        ("columns".into(), "SECURITY_CODE,SECURITY_NAME_ABBR,END_DATE,NETPROFIT,NP_YOY,NP_QOQ,ACCUMPROFIT,ACCUMPROFIT_YOY,OPERATE_INCOME,OI_YOY,OI_QOQ,ACCUMOI,ACCUMOI_YOY,NET_ASSETS,NA_YOY".into()),
        ("source".into(), "WEB".into()),
        ("client".into(), "WEB".into()),
        ("filter".into(), filter),
    ];
    let items = datacenter_all(client, "stock_qsjy_em", DATACENTER_BASE, params).await?;
    parse_qsjy(&items)
}

/// Parse a `result.data` array into [`StockQsjyEmRow`]s.
pub(crate) fn parse_qsjy(items: &[Value]) -> Result<Vec<StockQsjyEmRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(StockQsjyEmRow {
            code: opt_str_or(item, "SECURITY_CODE", ""),
            name: opt_str_or(item, "SECURITY_NAME_ABBR", ""),
            end_date: opt_str_or(item, "END_DATE", ""),
            net_profit: opt_f64(item, "NETPROFIT"),
            np_yoy: opt_f64(item, "NP_YOY"),
            np_qoq: opt_f64(item, "NP_QOQ"),
            accum_profit: opt_f64(item, "ACCUMPROFIT"),
            accum_profit_yoy: opt_f64(item, "ACCUMPROFIT_YOY"),
            operate_income: opt_f64(item, "OPERATE_INCOME"),
            oi_yoy: opt_f64(item, "OI_YOY"),
            oi_qoq: opt_f64(item, "OI_QOQ"),
            accum_oi: opt_f64(item, "ACCUMOI"),
            accum_oi_yoy: opt_f64(item, "ACCUMOI_YOY"),
            net_assets: opt_f64(item, "NET_ASSETS"),
            na_yoy: opt_f64(item, "NA_YOY"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// 12. stock_qbzf_em — 全部增发 (RPT_SEO_DETAIL)
// ===========================================================================

/// One seasoned-offering (增发) row, port of `stock_qbzf_em`. `new_price`
/// comes from the `quoteColumns` alias `f2~01~SECURITY_CODE~NEW_PRICE`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockQbzfEmRow {
    /// `SECURITY_CODE` 股票代码
    pub code: String,
    /// `SECURITY_NAME_ABBR` 股票简称
    pub name: String,
    /// `CORRECODE` 增发代码
    pub corre_code: String,
    /// `SEO_TYPE` 发行方式 (code: 1=定向增发, 2=公开增发)
    pub seo_type: Option<f64>,
    /// `ISSUE_NUM` 发行总数
    pub issue_num: Option<f64>,
    /// `ONLINE_ISSUE_NUM` 网上发行
    pub online_issue_num: Option<f64>,
    /// `ISSUE_PRICE` 发行价格
    pub issue_price: Option<f64>,
    /// `NEW_PRICE` 最新价 (quoteColumns)
    pub new_price: Option<f64>,
    /// `ISSUE_DATE` 发行日期
    pub issue_date: String,
    /// `ISSUE_LISTING_DATE` 增发上市日期
    pub issue_listing_date: String,
    /// `LOCKIN_PERIOD` 锁定期
    pub lockin_period: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_qbzf_em()`.
pub async fn stock_qbzf_em(client: &Client) -> Result<Vec<StockQbzfEmRow>> {
    let params = vec![
        ("sortColumns".into(), "ISSUE_DATE".into()),
        ("sortTypes".into(), "-1".into()),
        ("pageSize".into(), "500".into()),
        ("reportName".into(), "RPT_SEO_DETAIL".into()),
        ("columns".into(), "ALL".into()),
        ("quoteColumns".into(), "f2~01~SECURITY_CODE~NEW_PRICE".into()),
        ("quoteType".into(), "0".into()),
        ("source".into(), "WEB".into()),
        ("client".into(), "WEB".into()),
    ];
    let items = datacenter_all(client, "stock_qbzf_em", DATACENTER_BASE, params).await?;
    parse_qbzf(&items)
}

/// Parse a `result.data` array into [`StockQbzfEmRow`]s.
pub(crate) fn parse_qbzf(items: &[Value]) -> Result<Vec<StockQbzfEmRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(StockQbzfEmRow {
            code: opt_str_or(item, "SECURITY_CODE", ""),
            name: opt_str_or(item, "SECURITY_NAME_ABBR", ""),
            corre_code: opt_str_or(item, "CORRECODE", ""),
            seo_type: opt_f64(item, "SEO_TYPE"),
            issue_num: opt_f64(item, "ISSUE_NUM"),
            online_issue_num: opt_f64(item, "ONLINE_ISSUE_NUM"),
            issue_price: opt_f64(item, "ISSUE_PRICE"),
            new_price: opt_f64(item, "NEW_PRICE"),
            issue_date: opt_str_or(item, "ISSUE_DATE", ""),
            issue_listing_date: opt_str_or(item, "ISSUE_LISTING_DATE", ""),
            lockin_period: opt_f64(item, "LOCKIN_PERIOD"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// 13. stock_zdhtmx_em — 重大合同明细 (RPTA_WEB_ZDHT_LIST)
// ===========================================================================

/// One major-contract (重大合同) row, port of `stock_zdhtmx_em`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockZdhtmxEmRow {
    /// `CONTRACTNAME` 合同名称
    pub contract_name: String,
    /// `COUNTERPARTY` 其他签署方
    pub counterparty: String,
    /// `DIM_RDATE` 公告日期
    pub notice_date: String,
    /// `SIGNATORY` 签署主体
    pub signatory: String,
    /// `SIGNATORYREL` 与上市公司关系
    pub signatory_rel: String,
    /// `SIGNDATE` 签署日期
    pub sign_date: String,
    /// `AMOUNTS` 合同金额
    pub amounts: Option<f64>,
    /// `SECURITYCODE` 股票代码
    pub code: String,
    /// `SECURITYSHORTNAME` 股票简称
    pub name: String,
    /// `CONTRACTTYPENAME` 合同类型
    pub contract_type: String,
    /// `SNDYYSR` 上年度营业收入
    pub snd_yysr: Option<f64>,
    /// `OPERATEREVE` 最新财务报表的营业收入
    pub operate_reve: Option<f64>,
    /// `ZSNDYYSRBL` 占上年度营业收入比例
    pub zsnd_yysr_bl: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_zdhtmx_em(start_date, end_date)`. Dates are `YYYYMMDD`.
pub async fn stock_zdhtmx_em(
    client: &Client,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<StockZdhtmxEmRow>> {
    let s = fmt_date8(start_date, "stock_zdhtmx_em start_date")?;
    let e = fmt_date8(end_date, "stock_zdhtmx_em end_date")?;
    let filter = format!("(DIM_RDATE>='{s}')(DIM_RDATE<='{e}')");
    let params = vec![
        ("sortColumns".into(), "DIM_RDATE".into()),
        ("sortTypes".into(), "-1".into()),
        ("pageSize".into(), "500".into()),
        ("columns".into(), "ALL".into()),
        ("token".into(), "894050c76af8597a853f5b408b759f5d".into()),
        ("reportName".into(), "RPTA_WEB_ZDHT_LIST".into()),
        ("filter".into(), filter),
    ];
    let items = datacenter_all(client, "stock_zdhtmx_em", DATACENTER_BASE, params).await?;
    parse_zdhtmx(&items)
}

/// Parse a `result.data` array into [`StockZdhtmxEmRow`]s.
pub(crate) fn parse_zdhtmx(items: &[Value]) -> Result<Vec<StockZdhtmxEmRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(StockZdhtmxEmRow {
            contract_name: opt_str_or(item, "CONTRACTNAME", ""),
            counterparty: opt_str_or(item, "COUNTERPARTY", ""),
            notice_date: opt_str_or(item, "DIM_RDATE", ""),
            signatory: opt_str_or(item, "SIGNATORY", ""),
            signatory_rel: opt_str_or(item, "SIGNATORYREL", ""),
            sign_date: opt_str_or(item, "SIGNDATE", ""),
            amounts: opt_f64(item, "AMOUNTS"),
            code: opt_str_or(item, "SECURITYCODE", ""),
            name: opt_str_or(item, "SECURITYSHORTNAME", ""),
            contract_type: opt_str_or(item, "CONTRACTTYPENAME", ""),
            snd_yysr: opt_f64(item, "SNDYYSR"),
            operate_reve: opt_f64(item, "OPERATEREVE"),
            zsnd_yysr_bl: opt_f64(item, "ZSNDYYSRBL"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// 14. stock_zh_scale_comparison_em — 同行比较·公司规模 (RPT_PCF10_INDUSTRY_MARKET)
// ===========================================================================

/// One peer-scale comparison row, port of `stock_zh_scale_comparison_em`.
///
/// `symbol` is e.g. `"SZ000895"`; the secucode filter becomes `"000895.SZ"`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockZhScaleComparisonEmRow {
    /// `CORRE_SECURITY_CODE` 代码
    pub code: String,
    /// `CORRE_SECURITY_NAME` 简称
    pub name: String,
    /// `TOTAL_CAP` 总市值
    pub total_cap: Option<f64>,
    /// `TOTAL_CAP_RANK` 总市值排名
    pub total_cap_rank: Option<f64>,
    /// `FREECAP` 流通市值
    pub free_cap: Option<f64>,
    /// `FREECAP_RANK` 流通市值排名
    pub free_cap_rank: Option<f64>,
    /// `TOTAL_OPERATEINCOME` 营业收入
    pub total_operate_income: Option<f64>,
    /// `TOTAL_OPERATEINCOME_RANK` 营业收入排名
    pub total_operate_income_rank: Option<f64>,
    /// `NETPROFIT` 净利润
    pub net_profit: Option<f64>,
    /// `NETPROFIT_RANK` 净利润排名
    pub net_profit_rank: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_zh_scale_comparison_em(symbol)`.
pub async fn stock_zh_scale_comparison_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<StockZhScaleComparisonEmRow>> {
    if symbol.len() < 4 {
        return Err(Error::InvalidParam(format!(
            "stock_zh_scale_comparison_em: symbol must look like 'SZ000895', got {symbol:?}"
        )));
    }
    let secucode = format!("{}.{}", &symbol[2..], &symbol[..2]);
    let filter = format!("(SECUCODE=\"{secucode}\")(CORRE_SECUCODE=\"{secucode}\")");
    let params = vec![
        ("reportName".into(), "RPT_PCF10_INDUSTRY_MARKET".into()),
        ("columns".into(), "SECUCODE,SECURITY_CODE,SECURITY_NAME_ABBR,ORG_CODE,CORRE_SECUCODE,CORRE_SECURITY_CODE,CORRE_SECURITY_NAME,CORRE_ORG_CODE,TOTAL_CAP,FREECAP,TOTAL_OPERATEINCOME,NETPROFIT,REPORT_TYPE,TOTAL_CAP_RANK,FREECAP_RANK,TOTAL_OPERATEINCOME_RANK,NETPROFIT_RANK".into()),
        ("quoteColumns".into(), "".into()),
        ("filter".into(), filter),
        ("pageNumber".into(), "1".into()),
        ("pageSize".into(), "5".into()),
        ("sortTypes".into(), "-1".into()),
        ("sortColumns".into(), "TOTAL_CAP".into()),
        ("source".into(), "HSF10".into()),
        ("client".into(), "PC".into()),
        ("v".into(), "005391946600478148".into()),
    ];
    let items = datacenter_all(
        client,
        "stock_zh_scale_comparison_em",
        DATACENTER_SECURITIES_BASE,
        params,
    )
    .await?;
    parse_scale_comparison(&items)
}

/// Parse a `result.data` array into [`StockZhScaleComparisonEmRow`]s.
pub(crate) fn parse_scale_comparison(
    items: &[Value],
) -> Result<Vec<StockZhScaleComparisonEmRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(StockZhScaleComparisonEmRow {
            code: opt_str_or(item, "CORRE_SECURITY_CODE", ""),
            name: opt_str_or(item, "CORRE_SECURITY_NAME", ""),
            total_cap: opt_f64(item, "TOTAL_CAP"),
            total_cap_rank: opt_f64(item, "TOTAL_CAP_RANK"),
            free_cap: opt_f64(item, "FREECAP"),
            free_cap_rank: opt_f64(item, "FREECAP_RANK"),
            total_operate_income: opt_f64(item, "TOTAL_OPERATEINCOME"),
            total_operate_income_rank: opt_f64(item, "TOTAL_OPERATEINCOME_RANK"),
            net_profit: opt_f64(item, "NETPROFIT"),
            net_profit_rank: opt_f64(item, "NETPROFIT_RANK"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// 15. stock_zh_a_gdhs_detail_em — 股东户数详情 (RPT_HOLDERNUM_DET)
// ===========================================================================

/// One shareholder-count detail row, port of `stock_zh_a_gdhs_detail_em`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockZhAGdhsDetailEmRow {
    /// `SECURITY_CODE` 代码
    pub code: String,
    /// `SECURITY_NAME_ABBR` 名称
    pub name: String,
    /// `CHANGE_SHARES` 股本变动
    pub change_shares: Option<f64>,
    /// `CHANGE_REASON` 股本变动原因
    pub change_reason: String,
    /// `END_DATE` 股东户数统计截止日
    pub end_date: String,
    /// `INTERVAL_CHRATE` 区间涨跌幅
    pub interval_chrate: Option<f64>,
    /// `AVG_MARKET_CAP` 户均持股市值
    pub avg_market_cap: Option<f64>,
    /// `AVG_HOLD_NUM` 户均持股数量
    pub avg_hold_num: Option<f64>,
    /// `TOTAL_MARKET_CAP` 总市值
    pub total_market_cap: Option<f64>,
    /// `TOTAL_A_SHARES` 总股本
    pub total_a_shares: Option<f64>,
    /// `HOLD_NOTICE_DATE` 股东户数公告日期
    pub hold_notice_date: String,
    /// `HOLDER_NUM` 股东户数-本次
    pub holder_num: Option<f64>,
    /// `PRE_HOLDER_NUM` 股东户数-上次
    pub pre_holder_num: Option<f64>,
    /// `HOLDER_NUM_CHANGE` 股东户数-增减
    pub holder_num_change: Option<f64>,
    /// `HOLDER_NUM_RATIO` 股东户数-增减比例
    pub holder_num_ratio: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_zh_a_gdhs_detail_em(symbol)`. `symbol` is a stock code, e.g.
/// `"000001"`.
pub async fn stock_zh_a_gdhs_detail_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<StockZhAGdhsDetailEmRow>> {
    let filter = format!("(SECURITY_CODE=\"{symbol}\")");
    let params = vec![
        ("sortColumns".into(), "END_DATE".into()),
        ("sortTypes".into(), "-1".into()),
        ("pageSize".into(), "500".into()),
        ("reportName".into(), "RPT_HOLDERNUM_DET".into()),
        ("columns".into(), "SECURITY_CODE,SECURITY_NAME_ABBR,CHANGE_SHARES,CHANGE_REASON,END_DATE,INTERVAL_CHRATE,AVG_MARKET_CAP,AVG_HOLD_NUM,TOTAL_MARKET_CAP,TOTAL_A_SHARES,HOLD_NOTICE_DATE,HOLDER_NUM,PRE_HOLDER_NUM,HOLDER_NUM_CHANGE,HOLDER_NUM_RATIO,END_DATE,PRE_END_DATE".into()),
        ("quoteColumns".into(), "f2,f3".into()),
        ("filter".into(), filter),
        ("source".into(), "WEB".into()),
        ("client".into(), "WEB".into()),
    ];
    let items = datacenter_all(client, "stock_zh_a_gdhs_detail_em", DATACENTER_BASE, params).await?;
    parse_gdhs_detail(&items)
}

/// Parse a `result.data` array into [`StockZhAGdhsDetailEmRow`]s.
pub(crate) fn parse_gdhs_detail(items: &[Value]) -> Result<Vec<StockZhAGdhsDetailEmRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(StockZhAGdhsDetailEmRow {
            code: opt_str_or(item, "SECURITY_CODE", ""),
            name: opt_str_or(item, "SECURITY_NAME_ABBR", ""),
            change_shares: opt_f64(item, "CHANGE_SHARES"),
            change_reason: opt_str_or(item, "CHANGE_REASON", ""),
            end_date: opt_str_or(item, "END_DATE", ""),
            interval_chrate: opt_f64(item, "INTERVAL_CHRATE"),
            avg_market_cap: opt_f64(item, "AVG_MARKET_CAP"),
            avg_hold_num: opt_f64(item, "AVG_HOLD_NUM"),
            total_market_cap: opt_f64(item, "TOTAL_MARKET_CAP"),
            total_a_shares: opt_f64(item, "TOTAL_A_SHARES"),
            hold_notice_date: opt_str_or(item, "HOLD_NOTICE_DATE", ""),
            holder_num: opt_f64(item, "HOLDER_NUM"),
            pre_holder_num: opt_f64(item, "PRE_HOLDER_NUM"),
            holder_num_change: opt_f64(item, "HOLDER_NUM_CHANGE"),
            holder_num_ratio: opt_f64(item, "HOLDER_NUM_RATIO"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// Tests — offline, against fixtures in tests/fixtures/
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Float approximate-equality, matching `src/stock/board.rs` test helper.
    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    fn fixture(name: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let txt = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    fn items_of(v: &Value) -> Vec<Value> {
        v.as_array().cloned().unwrap_or_default()
    }

    #[test]
    fn parses_ah_spot() {
        let rows = parse_ah(&items_of(&fixture("stock_zh_ah_spot_em.json"))).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "山东墨龙");
        assert_eq!(rows[0].h_code, "00568");
        // f2 = 3450 (×1000) → 3.45
        assert!(approx(rows[0].h_price, 3.45));
        // f3 = -496 (×100) → -4.96
        assert!(approx(rows[0].h_pct, -4.96));
        assert_eq!(rows[0].a_code, "002490");
        // f186 = 7820 (×1000) → 7.82
        assert!(approx(rows[0].a_price, 7.82));
        assert!(approx(rows[0].ratio, 0.45));
        assert!(approx(rows[0].premium, 12.5));
        assert_eq!(rows[0].source, "eastmoney");
    }

    #[test]
    fn parses_sh_hk_spot() {
        let rows = parse_sh_hk(&items_of(&fixture("stock_hsgt_sh_hk_spot_em.json"))).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "00700");
        assert_eq!(rows[0].name, "腾讯控股");
        // f2 = 368000 (×1000) → 368.0
        assert!(approx(rows[0].price, 368.0));
        assert!(approx(rows[0].pct_change, 1.23));
        assert!(approx(rows[0].volume, 12.34));
        assert!(approx(rows[0].amount, 4500.0));
        assert_eq!(rows[1].code, "09988");
    }

    #[test]
    fn parses_kcb_report() {
        let rows = parse_kcb(&items_of(&fixture("stock_zh_kcb_report_em.json"))).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "688001");
        assert_eq!(rows[0].name, "华兴源创");
        assert_eq!(rows[0].title, "关于2024年半年度报告的公告");
        assert_eq!(rows[0].ann_type, Some("年报".to_string()));
        assert_eq!(rows[0].notice_date, "2024-08-20");
        assert_eq!(rows[0].art_code, "ABC123");
        assert_eq!(rows[1].ann_type, None);
    }

    #[test]
    fn parses_repurchase() {
        let rows = parse_repurchase(&items_of(&fixture("stock_repurchase_em.json"))).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "600519");
        assert_eq!(rows[0].name, "贵州茅台");
        assert!(approx(rows[0].new_price, 1685.0));
        assert_eq!(rows[0].repur_price_cap, Some("1480.00-1685.00".to_string()));
        assert!(approx(rows[0].repur_num_lower, 1000000.0));
        assert!(approx(rows[0].repur_num_cap, 1200000.0));
        assert!(approx(rows[0].ratio_lower, 0.08));
        assert!(approx(rows[0].amount_lower, 1500000000.0));
        assert_eq!(rows[0].start_date, "2024-01-15");
        assert_eq!(rows[0].progress, Some("004".to_string()));
        assert!(approx(rows[0].repur_num, 1100000.0));
        assert_eq!(rows[0].update_date, "2024-06-30");
        assert_eq!(rows[1].progress, None);
    }

    #[test]
    fn parses_gsrl() {
        let rows = parse_gsrl(&items_of(&fixture("stock_gsrl_gsdt_em.json"))).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "600000");
        assert_eq!(rows[0].name, "浦发银行");
        assert_eq!(rows[0].event_type, "业绩说明会");
        assert_eq!(rows[0].event_content, "召开2024年半年度业绩说明会");
        assert_eq!(rows[0].trade_date, "2024-08-20");
        assert_eq!(rows[1].event_type, "股东大会");
    }

    #[test]
    fn parses_new_em() {
        let rows = parse_new(&items_of(&fixture("stock_zh_a_new_em.json"))).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "001234");
        assert_eq!(rows[0].name, "新股A");
        assert!(approx(rows[0].price, 12.34));
        assert!(approx(rows[0].pct_change, 44.0));
        assert!(approx(rows[0].total_mktcap, 1234000000.0));
        assert_eq!(rows[0].market, Some(0.0));
        assert_eq!(rows[1].code, "601234");
    }

    #[test]
    fn parses_hold_management() {
        let rows =
            parse_hold_management(&items_of(&fixture("stock_hold_management_detail_em.json")))
                .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].change_date, "2024-03-01");
        assert_eq!(rows[0].code, "300750");
        assert_eq!(rows[0].person, "张三");
        assert!(approx(rows[0].change_shares, -50000.0));
        assert!(approx(rows[0].avg_price, 185.5));
        assert_eq!(rows[0].hold_type, "A股");
        assert!(approx(rows[0].hold_after, 1200000.0));
        assert_eq!(rows[1].position, "董事");
    }

    #[test]
    fn yysj_symbol_validation() {
        assert!(yysj_filter("bogus", "20200331").is_err());
        let f = yysj_filter("沪深A股", "2020-03-31").unwrap();
        assert!(f.contains("069001017"));
        let f2 = yysj_filter("科创板", "2020-03-31").unwrap();
        assert!(f2.contains("069001001006"));
    }

    #[test]
    fn parses_yysj() {
        let rows = parse_yysj(&items_of(&fixture("stock_yysj_em.json"))).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "600519");
        assert_eq!(rows[0].name, "贵州茅台");
        assert_eq!(rows[0].first_appoint, "2024-03-31");
        assert_eq!(rows[0].first_change, Some("2024-04-10".to_string()));
        assert_eq!(rows[0].actual_publish, Some("2024-04-03".to_string()));
        assert_eq!(rows[1].second_change, Some("2024-04-20".to_string()));
    }

    #[test]
    fn parses_jgdy() {
        let rows = parse_jgdy(&items_of(&fixture("stock_jgdy_detail_em.json"))).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "002415");
        assert_eq!(rows[0].name, "海康威视");
        assert!(approx(rows[0].close_price, 32.5));
        assert!(approx(rows[0].change_rate, -1.2));
        assert_eq!(rows[0].receive_object, "易方达基金");
        assert_eq!(rows[0].org_type, "基金公司");
        assert_eq!(rows[0].receive_start_date, "2024-12-12");
        assert_eq!(rows[1].org_type, "证券公司");
    }

    #[test]
    fn parses_gddh() {
        let rows = parse_gddh(&items_of(&fixture("stock_gddh_em.json"))).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "600036");
        assert_eq!(rows[0].name, "招商银行");
        assert_eq!(rows[0].meeting_title, "2023年年度股东大会");
        assert_eq!(rows[0].start_date, "2024-05-20");
        assert_eq!(rows[0].equity_record_date, "2024-05-13");
        assert_eq!(rows[0].serial_num, Some("2024-05-20-1".to_string()));
        assert_eq!(rows[1].web_end_date, "2024-05-22");
    }

    #[test]
    fn parses_qsjy() {
        let rows = parse_qsjy(&items_of(&fixture("stock_qsjy_em.json"))).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "600030");
        assert_eq!(rows[0].name, "中信证券");
        assert_eq!(rows[0].end_date, "2020-07-31");
        assert!(approx(rows[0].net_profit, 4500000000.0));
        assert!(approx(rows[0].np_yoy, 12.5));
        assert!(approx(rows[0].accum_oi, 20000000000.0));
        assert!(approx(rows[0].na_yoy, 5.0));
        assert_eq!(rows[1].code, "601211");
    }

    #[test]
    fn parses_qbzf() {
        let rows = parse_qbzf(&items_of(&fixture("stock_qbzf_em.json"))).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "002230");
        assert_eq!(rows[0].name, "科大讯飞");
        assert_eq!(rows[0].corre_code, "08230");
        assert!(approx(rows[0].seo_type, 1.0));
        assert!(approx(rows[0].issue_num, 50000000.0));
        assert!(approx(rows[0].issue_price, 35.0));
        assert!(approx(rows[0].new_price, 42.1));
        assert_eq!(rows[0].issue_date, "2023-06-01");
        assert_eq!(rows[1].seo_type, Some(2.0));
    }

    #[test]
    fn parses_zdhtmx() {
        let rows = parse_zdhtmx(&items_of(&fixture("stock_zdhtmx_em.json"))).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].contract_name, "某重大供货合同");
        assert_eq!(rows[0].counterparty, "某客户");
        assert_eq!(rows[0].notice_date, "2023-08-19");
        assert!(approx(rows[0].amounts, 5000000000.0));
        assert_eq!(rows[0].code, "600900");
        assert_eq!(rows[0].name, "长江电力");
        assert!(approx(rows[0].zsnd_yysr_bl, 15.3));
        assert_eq!(rows[1].contract_type, "工程建设");
    }

    #[test]
    fn parses_scale_comparison() {
        let rows =
            parse_scale_comparison(&items_of(&fixture("stock_zh_scale_comparison_em.json")))
                .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "000895");
        assert_eq!(rows[0].name, "双汇发展");
        assert!(approx(rows[0].total_cap, 90000000000.0));
        assert!(approx(rows[0].total_cap_rank, 1.0));
        assert!(approx(rows[0].net_profit, 5000000000.0));
        assert_eq!(rows[1].code, "002714");
    }

    #[test]
    fn parses_gdhs_detail() {
        let rows =
            parse_gdhs_detail(&items_of(&fixture("stock_zh_a_gdhs_detail_em.json"))).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "000001");
        assert_eq!(rows[0].name, "平安银行");
        assert_eq!(rows[0].end_date, "2024-03-31");
        assert!(approx(rows[0].interval_chrate, -5.2));
        assert!(approx(rows[0].holder_num, 523400.0));
        assert!(approx(rows[0].pre_holder_num, 530100.0));
        assert!(approx(rows[0].holder_num_change, -6700.0));
        assert!(approx(rows[0].holder_num_ratio, -1.26));
        assert_eq!(rows[1].holder_num, None);
    }

    #[test]
    fn fmt_date8_rejects_bad() {
        assert!(fmt_date8("202001", "x").is_err());
        assert!(fmt_date8("2020010a", "x").is_err());
        assert_eq!(fmt_date8("20200331", "x").unwrap(), "2020-03-31");
    }
}
