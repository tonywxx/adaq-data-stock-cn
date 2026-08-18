//! 港股 (Hong Kong stock) data ports from akshare.
//!
//! | Rust function | akshare source | source / endpoint |
//! | --- | --- | --- |
//! | `security_profile_em` | `stock/stock_profile_em.py:13` | Eastmoney datacenter `RPT_HKF10_INFO_SECURITYINFO` |
//! | `company_profile_em` | `stock/stock_profile_em.py:79` | Eastmoney datacenter `RPT_HKF10_INFO_ORGPROFILE` |
//! | `financial_indicator_em` | `stock/stock_profile_em.py:153` | Eastmoney datacenter `RPT_CUSTOM_HKF10_FN_MAININDICATORMAX` |
//! | `dividend_payout_em` | `stock/stock_profile_em.py:237` | Eastmoney datacenter `RPT_HKF10_MAIN_DIVBASIC` |
//! | `famous_spot_em` | `stock/stock_hk_famous.py:13` | Eastmoney push2 `clist` (node `DLMK0106`) |
//! | `growth_comparison_em` | `stock/stock_hk_comparison_em.py:13` | Eastmoney datacenter `RPT_PCF10_INDUSTRY_HKGROWTH` |
//! | `valuation_comparison_em` | `stock/stock_hk_comparison_em.py:61` | Eastmoney datacenter `RPT_PCF10_INDUSTRY_HKCVALUE` |
//! | `scale_comparison_em` | `stock/stock_hk_comparison_em.py:118` | Eastmoney datacenter `RPT_PCF10_INDUSTRY_SCALE` |
//! | `ggt_components_em` | `stock_feature/stock_hsgt_em.py:94` | Eastmoney push2 `clist` (nodes `DLMK0146,DLMK0144`) |
//! | `spot` | `stock/stock_hk_sina.py:22` | Sina `Market_Center.getHKStockData` (JSON array) |
//!
//! ## DEFERRED
//!
//! These akshare functions are **not** implemented here, with the exact reason:
//!
//! * **`stock_hk_daily`** (`stock/stock_hk_sina.py:109`): Sina HK *historical* data
//!   is returned JS-encrypted; akshare decrypts it with `py_mini_racer`/`execjs`
//!   using `hk_js_decode`. Requires a JS runtime — out of scope (no execjs in crate).
//! * **`stock_hk_fhpx_detail_ths`** (`stock/stock_hk_fhpx_ths.py:15`): THS (10jqka)
//!   dividend page is scraped via `pd.read_html` (HTML `<table>`). No JSON API.
//! * **`stock_hk_gxl_lg`** (`stock_feature/stock_gxl_lg.py:54`): legulegu requires a
//!   session token (`get_token_lg`) **and** a CSRF cookie (`get_cookie_csrf`) fetched
//!   from a page first — cookie/session auth, not a plain GET.
//! * **`stock_hk_valuation_baidu`** (`stock_feature/stock_hk_valuation_baidu.py:14`):
//!   Baidu Gushitong `opendata` requires `curl_cffi` browser-impersonation headers
//!   (anti-bot) and is indicator/period dependent; not a plain JSON GET.
//! * **`stock_hk_profit_forecast_et`** (`stock_fundamental/stock_profit_forecast_hk_etnet.py:15`):
//!   etnet profit-forecast page is scraped via `pd.read_html` (4 indicator HTML
//!   tables). No JSON API.
//! * **`stock_hk_hot_rank_em`** / **`stock_hk_hot_rank_detail_em`** /
//!   **`stock_hk_hot_rank_detail_realtime_em`** / **`stock_hk_hot_rank_latest_em`**
//!   (`stock/stock_hk_hot_rank_em.py:13,60,85,108`): Eastmoney `emappdata` stockrank
//!   endpoints require a **JSON request body** (`requests.post(json=...)`). The crate's
//!   `Client` only exposes `post_form_json` (form-encoded) and `Client` must not be
//!   modified, so the JSON-POST transport is unavailable.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use crate::core::json::*;

const SOURCE_EASTMONEY: &str = "eastmoney";
const SOURCE_SINA: &str = "sina";

/// Eastmoney F10 securities datacenter (the `RPT_HK*` / `RPT_PCF10*` reports).
const DC_SEC: &str = "https://datacenter.eastmoney.com/securities/api/data/v1/get";
/// Eastmoney push2 quote-list endpoint (used by `famous_spot_em` / `ggt_components_em`).
const PUSH2: &str = "https://push2.eastmoney.com/api/qt/clist/get";
/// Sina HK all-stocks realtime spot endpoint (returns a JSON array of rows).
const SINA_HK_SPOT_URL: &str = "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHKStockData";

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Extract `result.data` from an Eastmoney datacenter response. Returns an empty
/// `Vec` when `result`/`data` is missing or null (the comparison reports return
/// `null` when there is nothing to compare against).
fn em_data_array(resp: &Value) -> Result<Vec<Value>> {
    match resp.get("result") {
        Some(Value::Null) | None => Ok(Vec::new()),
        Some(r) => match r.get("data") {
            Some(Value::Null) | None => Ok(Vec::new()),
            Some(d) => d.as_array().cloned().ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "result.data is not an array".into(),
            }),
        },
    }
}

/// Fetch an Eastmoney F10 securities datacenter report. Returns the `result.data`
/// row array for the caller's parser.
#[allow(clippy::too_many_arguments)]
async fn em_sec_fetch(
    client: &Client,
    fn_name: &'static str,
    report_name: &str,
    columns: &str,
    filter: &str,
    sort_types: &str,
    sort_columns: &str,
    page_size: &str,
    v: &str,
) -> Result<Vec<Value>> {
    let params: Vec<(&str, &str)> = vec![
        ("reportName", report_name),
        ("columns", columns),
        ("quoteColumns", ""),
        ("filter", filter),
        ("pageNumber", "1"),
        ("pageSize", page_size),
        ("sortTypes", sort_types),
        ("sortColumns", sort_columns),
        ("source", "F10"),
        ("client", "PC"),
        ("v", v),
    ];
    let vv = client
        .get_json(SOURCE_EASTMONEY, fn_name, DC_SEC, &params)
        .await?;
    em_data_array(&vv)
}

/// Extract and parse the `data.diff` object from a push2 `clist` response. The
/// `diff` is a JSON object keyed by row index; each value is a row of `f1..fN`.
fn push2_diff_rows(resp: &Value) -> Result<Vec<Value>> {
    let diff = resp
        .get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|d| d.as_object())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff".into(),
        })?;
    Ok(diff.values().cloned().collect())
}

/// Fetch a push2 `clist` page (single page, large `pz`). Returns the `diff` rows.
async fn push2_clist(
    client: &Client,
    fn_name: &'static str,
    fs: &str,
    fields: &str,
    pz: &str,
) -> Result<Value> {
    let params: Vec<(&str, &str)> = vec![
        ("pn", "1"),
        ("pz", pz),
        ("po", "1"),
        ("np", "2"),
        ("ut", "bd1d9ddb04089700cf9c27f6f7426281"),
        ("fltt", "2"),
        ("invt", "2"),
        ("dect", "1"),
        ("wbp2u", "|0|0|0|web"),
        ("fid", "f3"),
        ("fs", fs),
        ("fields", fields),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, fn_name, PUSH2, &params)
        .await?;
    Ok(v)
}

/// Parse one push2 `diff` row object (fields `f12` code, `f14` name, `f2/f3/f4/f5/f6`
/// quote fields, `f15/f16/f17/f18` high/low/open/preclose).
fn parse_push2_row(idx: usize, item: &Value) -> HkSpotQuoteRow {
    HkSpotQuoteRow {
        rank: idx + 1,
        code: opt_str(item, "f12").unwrap_or_default(),
        name: opt_str(item, "f14").unwrap_or_default(),
        price: opt_f64(item, "f2"),
        pct_change: opt_f64(item, "f3"),
        change: opt_f64(item, "f4"),
        volume: opt_f64(item, "f5"),
        amount: opt_f64(item, "f6"),
        high: opt_f64(item, "f15"),
        low: opt_f64(item, "f16"),
        open: opt_f64(item, "f17"),
        pre_close: opt_f64(item, "f18"),
    }
}

/// Parse a push2 `clist` response (`data.diff`) into [`HkSpotQuoteRow`]s.
pub(crate) fn parse_hk_spot(resp: &Value) -> Result<Vec<HkSpotQuoteRow>> {
    let rows = push2_diff_rows(resp)?;
    Ok(rows
        .iter()
        .enumerate()
        .map(|(i, item)| parse_push2_row(i, item))
        .collect())
}

// ---------------------------------------------------------------------------
// stock_hk_security_profile_em  (stock/stock_profile_em.py:13)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct SecurityProfileRow {
    /// 证券代码 (SECUCODE)
    pub secucode: String,
    /// 证券简称 (SECURITY_NAME_ABBR)
    pub name: String,
    /// 上市日期 (LISTING_DATE)
    pub listing_date: Option<String>,
    /// 证券类型 (SECURITY_TYPE)
    pub security_type: Option<String>,
    /// 发行价 (ISSUE_PRICE)
    pub issue_price: Option<f64>,
    /// 发行量(股) (ISSUE_NUM)
    pub issue_num: Option<f64>,
    /// 每手股数 (TRADE_UNIT)
    pub trade_unit: Option<f64>,
    /// 每股面值 (PAR_VALUE)
    pub par_value: Option<f64>,
    /// 交易所 (TRADE_MARKET)
    pub trade_market: Option<String>,
    /// 板块 (BOARD)
    pub board: Option<String>,
    /// 年结日 (YEAR_SETTLE_DAY)
    pub year_settle_day: Option<String>,
    /// ISIN（国际证券识别编码） (ISIN_CODE)
    pub isin_code: Option<String>,
    /// 是否沪港通标的 (GANGGUTONGBIAODISHEN)
    pub ganggutongbiaodishen: Option<String>,
    /// 是否深港通标的 (GANGGUTONGBIAODIHU)
    pub ganggutongbiaodihu: Option<String>,
}

/// Parse `stock_hk_security_profile_em` rows from a datacenter response.
pub(crate) fn parse_security_profile(items: &[Value]) -> Result<Vec<SecurityProfileRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(secucode) = opt_str(item, "SECUCODE") else {
            continue;
        };
        out.push(SecurityProfileRow {
            secucode,
            name: opt_str(item, "SECURITY_NAME_ABBR").unwrap_or_default(),
            listing_date: opt_str(item, "LISTING_DATE"),
            security_type: opt_str(item, "SECURITY_TYPE"),
            issue_price: opt_f64(item, "ISSUE_PRICE"),
            issue_num: opt_f64(item, "ISSUE_NUM"),
            trade_unit: opt_f64(item, "TRADE_UNIT"),
            par_value: opt_f64(item, "PAR_VALUE"),
            trade_market: opt_str(item, "TRADE_MARKET"),
            board: opt_str(item, "BOARD"),
            year_settle_day: opt_str(item, "YEAR_SETTLE_DAY"),
            isin_code: opt_str(item, "ISIN_CODE"),
            ganggutongbiaodishen: opt_str(item, "GANGGUTONGBIAODISHEN"),
            ganggutongbiaodihu: opt_str(item, "GANGGUTONGBIAODIHU"),
        });
    }
    Ok(out)
}

/// 东方财富-港股-证券资料 (akshare `stock_hk_security_profile_em`).
pub async fn security_profile_em(client: &Client, symbol: &str) -> Result<Vec<SecurityProfileRow>> {
    let filter = format!(r#"(SECUCODE="{symbol}.HK")"#);
    let data = em_sec_fetch(
        client,
        "stock_hk_security_profile_em",
        "RPT_HKF10_INFO_SECURITYINFO",
        "SECUCODE,SECURITY_CODE,SECURITY_NAME_ABBR,SECURITY_TYPE,LISTING_DATE,ISIN_CODE,BOARD,\
         TRADE_UNIT,TRADE_MARKET,GANGGUTONGBIAODISHEN,GANGGUTONGBIAODIHU,PAR_VALUE,\
         ISSUE_PRICE,ISSUE_NUM,YEAR_SETTLE_DAY",
        &filter,
        "",
        "",
        "200",
        "04748497219912483",
    )
    .await?;
    parse_security_profile(&data)
}

// ---------------------------------------------------------------------------
// stock_hk_company_profile_em  (stock/stock_profile_em.py:79)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct CompanyProfileRow {
    /// 公司名称 (ORG_NAME)
    pub org_name: String,
    /// 英文名称 (ORG_EN_ABBR)
    pub org_en_abbr: Option<String>,
    /// 注册地 (REG_PLACE)
    pub reg_place: Option<String>,
    /// 注册地址 (REG_ADDRESS)
    pub reg_address: Option<String>,
    /// 公司成立日期 (FOUND_DATE)
    pub found_date: Option<String>,
    /// 所属行业 (BELONG_INDUSTRY)
    pub belong_industry: Option<String>,
    /// 董事长 (CHAIRMAN)
    pub chairman: Option<String>,
    /// 公司秘书 (SECRETARY)
    pub secretary: Option<String>,
    /// 员工人数 (EMP_NUM)
    pub emp_num: Option<f64>,
    /// 办公地址 (ADDRESS)
    pub address: Option<String>,
    /// 公司网址 (ORG_WEB)
    pub org_web: Option<String>,
    /// E-MAIL (ORG_EMAIL)
    pub org_email: Option<String>,
    /// 年结日 (YEAR_SETTLE_DAY)
    pub year_settle_day: Option<String>,
    /// 联系电话 (ORG_TEL)
    pub org_tel: Option<String>,
    /// 核数师 (ACCOUNT_FIRM)
    pub account_firm: Option<String>,
    /// 传真 (ORG_FAX)
    pub org_fax: Option<String>,
    /// 公司介绍 (ORG_PROFILE)
    pub org_profile: Option<String>,
}

/// Parse `stock_hk_company_profile_em` rows from a datacenter response.
pub(crate) fn parse_company_profile(items: &[Value]) -> Result<Vec<CompanyProfileRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(org_name) = opt_str(item, "ORG_NAME") else {
            continue;
        };
        out.push(CompanyProfileRow {
            org_name,
            org_en_abbr: opt_str(item, "ORG_EN_ABBR"),
            reg_place: opt_str(item, "REG_PLACE"),
            reg_address: opt_str(item, "REG_ADDRESS"),
            found_date: opt_str(item, "FOUND_DATE"),
            belong_industry: opt_str(item, "BELONG_INDUSTRY"),
            chairman: opt_str(item, "CHAIRMAN"),
            secretary: opt_str(item, "SECRETARY"),
            emp_num: opt_f64(item, "EMP_NUM"),
            address: opt_str(item, "ADDRESS"),
            org_web: opt_str(item, "ORG_WEB"),
            org_email: opt_str(item, "ORG_EMAIL"),
            year_settle_day: opt_str(item, "YEAR_SETTLE_DAY"),
            org_tel: opt_str(item, "ORG_TEL"),
            account_firm: opt_str(item, "ACCOUNT_FIRM"),
            org_fax: opt_str(item, "ORG_FAX"),
            org_profile: opt_str(item, "ORG_PROFILE"),
        });
    }
    Ok(out)
}

/// 东方财富-港股-公司资料 (akshare `stock_hk_company_profile_em`).
pub async fn company_profile_em(client: &Client, symbol: &str) -> Result<Vec<CompanyProfileRow>> {
    let filter = format!(r#"(SECUCODE="{symbol}.HK")"#);
    let data = em_sec_fetch(
        client,
        "stock_hk_company_profile_em",
        "RPT_HKF10_INFO_ORGPROFILE",
        "SECUCODE,SECURITY_CODE,ORG_NAME,ORG_EN_ABBR,BELONG_INDUSTRY,FOUND_DATE,CHAIRMAN,\
         SECRETARY,ACCOUNT_FIRM,REG_ADDRESS,ADDRESS,YEAR_SETTLE_DAY,EMP_NUM,ORG_TEL,ORG_FAX,\
         ORG_EMAIL,ORG_WEB,ORG_PROFILE,REG_PLACE",
        &filter,
        "",
        "",
        "200",
        "04748497219912483",
    )
    .await?;
    parse_company_profile(&data)
}

// ---------------------------------------------------------------------------
// stock_hk_financial_indicator_em  (stock/stock_profile_em.py:153)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct FinancialIndicatorRow {
    /// 基本每股收益(元) (BASIC_EPS)
    pub basic_eps: Option<f64>,
    /// 每股净资产(元) (BPS)
    pub bps: Option<f64>,
    /// 法定股本(股) (COMMON_ACS)
    pub common_acs: Option<f64>,
    /// 每手股 (PER_SHARES)
    pub per_shares: Option<f64>,
    /// 每股股息TTM(港元) (DIVIDEND_TTM)
    pub dividend_ttm: Option<f64>,
    /// 派息比率(%) (DIVI_RATIO)
    pub divi_ratio: Option<f64>,
    /// 已发行股本(股) (ISSUED_COMMON_SHARES)
    pub issued_common_shares: Option<f64>,
    /// 已发行股本-H股(股) (HK_COMMON_SHARES)
    pub hk_common_shares: Option<f64>,
    /// 每股经营现金流(元) (PER_NETCASH_OPERATE)
    pub per_netcash_operate: Option<f64>,
    /// 股息率TTM(%) (DIVIDEND_RATE)
    pub dividend_rate: Option<f64>,
    /// 总市值(港元) (TOTAL_MARKET_CAP)
    pub total_market_cap: Option<f64>,
    /// 港股市值(港元) (HKSK_MARKET_CAP)
    pub hk_market_cap: Option<f64>,
    /// 营业总收入 (OPERATE_INCOME)
    pub operate_income: Option<f64>,
    /// 营业总收入滚动环比增长(%) (OPERATE_INCOME_QOQ)
    pub operate_income_qoq: Option<f64>,
    /// 销售净利率(%) (NET_PROFIT_RATIO)
    pub net_profit_ratio: Option<f64>,
    /// 净利润 (HOLDER_PROFIT)
    pub holder_profit: Option<f64>,
    /// 净利润滚动环比增长(%) (HOLDER_PROFIT_QOQ)
    pub holder_profit_qoq: Option<f64>,
    /// 股东权益回报率(%) (ROE_AVG)
    pub roe_avg: Option<f64>,
    /// 市盈率 (PE_TTM)
    pub pe_ttm: Option<f64>,
    /// 市净率 (PB_TTM)
    pub pb_ttm: Option<f64>,
    /// 总资产回报率(%) (ROA)
    pub roa: Option<f64>,
}

/// Parse `stock_hk_financial_indicator_em` rows from a datacenter response.
pub(crate) fn parse_financial_indicator(items: &[Value]) -> Result<Vec<FinancialIndicatorRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(FinancialIndicatorRow {
            basic_eps: opt_f64(item, "BASIC_EPS"),
            bps: opt_f64(item, "BPS"),
            common_acs: opt_f64(item, "COMMON_ACS"),
            per_shares: opt_f64(item, "PER_SHARES"),
            dividend_ttm: opt_f64(item, "DIVIDEND_TTM"),
            divi_ratio: opt_f64(item, "DIVI_RATIO"),
            issued_common_shares: opt_f64(item, "ISSUED_COMMON_SHARES"),
            hk_common_shares: opt_f64(item, "HK_COMMON_SHARES"),
            per_netcash_operate: opt_f64(item, "PER_NETCASH_OPERATE"),
            dividend_rate: opt_f64(item, "DIVIDEND_RATE"),
            total_market_cap: opt_f64(item, "TOTAL_MARKET_CAP"),
            hk_market_cap: opt_f64(item, "HKSK_MARKET_CAP"),
            operate_income: opt_f64(item, "OPERATE_INCOME"),
            operate_income_qoq: opt_f64(item, "OPERATE_INCOME_QOQ"),
            net_profit_ratio: opt_f64(item, "NET_PROFIT_RATIO"),
            holder_profit: opt_f64(item, "HOLDER_PROFIT"),
            holder_profit_qoq: opt_f64(item, "HOLDER_PROFIT_QOQ"),
            roe_avg: opt_f64(item, "ROE_AVG"),
            pe_ttm: opt_f64(item, "PE_TTM"),
            pb_ttm: opt_f64(item, "PB_TTM"),
            roa: opt_f64(item, "ROA"),
        });
    }
    Ok(out)
}

/// 东方财富-港股-核心必读-最新指标 (akshare `stock_hk_financial_indicator_em`).
pub async fn financial_indicator_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<FinancialIndicatorRow>> {
    let filter = format!(r#"(SECUCODE="{symbol}.HK")"#);
    let data = em_sec_fetch(
        client,
        "stock_hk_financial_indicator_em",
        "RPT_CUSTOM_HKF10_FN_MAININDICATORMAX",
        "ORG_CODE,SECUCODE,SECURITY_CODE,SECURITY_NAME_ABBR,SECURITY_INNER_CODE,REPORT_DATE,BASIC_EPS,\
         PER_NETCASH_OPERATE,BPS,BPS_NEDILUTED,COMMON_ACS,PER_SHARES,ISSUED_COMMON_SHARES,HK_COMMON_SHARES,\
         TOTAL_MARKET_CAP,HKSK_MARKET_CAP,OPERATE_INCOME,OPERATE_INCOME_SQ,OPERATE_INCOME_QOQ,\
         OPERATE_INCOME_QOQ_SQ,HOLDER_PROFIT,HOLDER_PROFIT_SQ,HOLDER_PROFIT_QOQ,HOLDER_PROFIT_QOQ_SQ,PE_TTM,\
         PE_TTM_SQ,PB_TTM,PB_TTM_SQ,NET_PROFIT_RATIO,NET_PROFIT_RATIO_SQ,ROE_AVG,ROE_AVG_SQ,ROA,\
         ROA_SQ,DIVIDEND_TTM,DIVIDEND_LFY,DIVI_RATIO,DIVIDEND_RATE,IS_CNY_CODE",
        &filter,
        "-1",
        "REPORT_DATE",
        "200",
        "07945646099062258",
    )
    .await?;
    parse_financial_indicator(&data)
}

// ---------------------------------------------------------------------------
// stock_hk_dividend_payout_em  (stock/stock_profile_em.py:237)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct DividendPayoutRow {
    /// 最新公告日期 (UPDATE_DATE)
    pub update_date: Option<String>,
    /// 财政年度 (YEAR)
    pub year: Option<String>,
    /// 分红方案 (PLAN_EXPLAIN)
    pub plan_explain: Option<String>,
    /// 分配类型 (REPORT_TYPE)
    pub report_type: Option<String>,
    /// 除净日 (EX_DIVIDEND_DATE)
    pub ex_dividend_date: Option<String>,
    /// 截至过户日 (TRANSFER_END_DATE)
    pub transfer_end_date: Option<String>,
    /// 发放日 (DIVIDEND_DATE)
    pub dividend_date: Option<String>,
}

/// Parse `stock_hk_dividend_payout_em` rows from a datacenter response.
pub(crate) fn parse_dividend_payout(items: &[Value]) -> Result<Vec<DividendPayoutRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(DividendPayoutRow {
            update_date: opt_str(item, "UPDATE_DATE"),
            year: opt_str(item, "YEAR"),
            plan_explain: opt_str(item, "PLAN_EXPLAIN"),
            report_type: opt_str(item, "REPORT_TYPE"),
            ex_dividend_date: opt_str(item, "EX_DIVIDEND_DATE"),
            transfer_end_date: opt_str(item, "TRANSFER_END_DATE"),
            dividend_date: opt_str(item, "DIVIDEND_DATE"),
        });
    }
    Ok(out)
}

/// 东方财富-港股-核心必读-分红派息 (akshare `stock_hk_dividend_payout_em`).
pub async fn dividend_payout_em(client: &Client, symbol: &str) -> Result<Vec<DividendPayoutRow>> {
    let filter = format!(r#"(SECURITY_CODE="{symbol}")(IS_BFP="0")"#);
    let data = em_sec_fetch(
        client,
        "stock_hk_dividend_payout_em",
        "RPT_HKF10_MAIN_DIVBASIC",
        "SECURITY_CODE,UPDATE_DATE,REPORT_TYPE,EX_DIVIDEND_DATE,DIVIDEND_DATE,\
         TRANSFER_END_DATE,YEAR,PLAN_EXPLAIN,IS_BFP",
        &filter,
        "-1,-1",
        "NOTICE_DATE,EX_DIVIDEND_DATE",
        "200",
        "035584639294227527",
    )
    .await?;
    parse_dividend_payout(&data)
}

// ---------------------------------------------------------------------------
// stock_hk_growth_comparison_em  (stock/stock_hk_comparison_em.py:13)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct GrowthComparisonRow {
    /// 代码 (CORRE_SECURITY_CODE)
    pub code: String,
    /// 简称 (CORRE_SECURITY_NAME)
    pub name: Option<String>,
    /// 基本每股收益同比增长率 (EPS_YOY)
    pub eps_yoy: Option<f64>,
    /// 基本每股收益同比增长率排名 (EPS_YOY_RANK)
    pub eps_yoy_rank: Option<f64>,
    /// 营业收入同比增长率 (OPERATE_INCOME_YOY)
    pub operate_income_yoy: Option<f64>,
    /// 营业收入同比增长率排名 (OPINCOME_YOY_RANK)
    pub operate_income_yoy_rank: Option<f64>,
    /// 营业利润率同比增长率 (OPERATE_PROFIT_YOY)
    pub operate_profit_yoy: Option<f64>,
    /// 营业利润率同比增长率排名 (OPROFIT_YOY_RANK)
    pub operate_profit_yoy_rank: Option<f64>,
    /// 总资产同比增长率 (TOTAL_ASSET_YOY)
    pub total_asset_yoy: Option<f64>,
    /// 总资产同比增长率排名 (TOASSET_YOY_RANK)
    pub total_asset_yoy_rank: Option<f64>,
}

/// Parse `stock_hk_growth_comparison_em` rows from a datacenter response.
pub(crate) fn parse_growth_comparison(items: &[Value]) -> Result<Vec<GrowthComparisonRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(code) = opt_str(item, "CORRE_SECURITY_CODE") else {
            continue;
        };
        out.push(GrowthComparisonRow {
            code,
            name: opt_str(item, "CORRE_SECURITY_NAME"),
            eps_yoy: opt_f64(item, "EPS_YOY"),
            eps_yoy_rank: opt_f64(item, "EPS_YOY_RANK"),
            operate_income_yoy: opt_f64(item, "OPERATE_INCOME_YOY"),
            operate_income_yoy_rank: opt_f64(item, "OPINCOME_YOY_RANK"),
            operate_profit_yoy: opt_f64(item, "OPERATE_PROFIT_YOY"),
            operate_profit_yoy_rank: opt_f64(item, "OPROFIT_YOY_RANK"),
            total_asset_yoy: opt_f64(item, "TOTAL_ASSET_YOY"),
            total_asset_yoy_rank: opt_f64(item, "TOASSET_YOY_RANK"),
        });
    }
    Ok(out)
}

/// 东方财富-港股-行业对比-成长性对比 (akshare `stock_hk_growth_comparison_em`).
pub async fn growth_comparison_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<GrowthComparisonRow>> {
    let filter = format!(r#"(SECUCODE="{symbol}.HK")(CORRE_SECUCODE="{symbol}.HK")"#);
    let data = em_sec_fetch(
        client,
        "stock_hk_growth_comparison_em",
        "RPT_PCF10_INDUSTRY_HKGROWTH",
        "SECUCODE,SECURITY_CODE,ORG_CODE,REPORT_DATE,TYPE_ID,TYPE_TYPE,\
         TYPE_NAME,TYPE_NAME_EN,CORRE_SECURITY_CODE,CORRE_SECUCODE,\
         CORRE_SECURITY_NAME,EPS_YOY,OPERATE_INCOME_YOY,OPERATE_PROFIT_YOY,\
         TOTAL_ASSET_YOY,EPS_YOY_RANK,OPINCOME_YOY_RANK,OPROFIT_YOY_RANK,TOASSET_YOY_RANK",
        &filter,
        "",
        "",
        "",
        "03313416193688571",
    )
    .await?;
    parse_growth_comparison(&data)
}

// ---------------------------------------------------------------------------
// stock_hk_valuation_comparison_em  (stock/stock_hk_comparison_em.py:61)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ValuationComparisonRow {
    /// 代码 (CORRE_SECURITY_CODE)
    pub code: String,
    /// 简称 (CORRE_SECURITY_NAME)
    pub name: Option<String>,
    /// 市盈率-TTM (PE_TTM)
    pub pe_ttm: Option<f64>,
    /// 市盈率-TTM排名 (PE_TTM_RANK)
    pub pe_ttm_rank: Option<f64>,
    /// 市盈率-LYR (PE_LYR)
    pub pe_lyr: Option<f64>,
    /// 市盈率-LYR排名 (PE_LYR_RANK)
    pub pe_lyr_rank: Option<f64>,
    /// 市净率-MRQ (PB_MQR)
    pub pb_mqr: Option<f64>,
    /// 市净率-MRQ排名 (PB_MQR_RANK)
    pub pb_mqr_rank: Option<f64>,
    /// 市净率-LYR (PB_LYR)
    pub pb_lyr: Option<f64>,
    /// 市净率-LYR排名 (PB_LYR_RANK)
    pub pb_lyr_rank: Option<f64>,
    /// 市销率-TTM (PS_TTM)
    pub ps_ttm: Option<f64>,
    /// 市销率-TTM排名 (PS_TTM_RANK)
    pub ps_ttm_rank: Option<f64>,
    /// 市销率-LYR (PS_LYR)
    pub ps_lyr: Option<f64>,
    /// 市销率-LYR排名 (PS_LYR_RANK)
    pub ps_lyr_rank: Option<f64>,
    /// 市现率-TTM (PCE_TTM)
    pub pce_ttm: Option<f64>,
    /// 市现率-TTM排名 (PCE_TTM_RANK)
    pub pce_ttm_rank: Option<f64>,
    /// 市现率-LYR (PCE_LYR)
    pub pce_lyr: Option<f64>,
    /// 市现率-LYR排名 (PCE_LYR_RANK)
    pub pce_lyr_rank: Option<f64>,
}

/// Parse `stock_hk_valuation_comparison_em` rows from a datacenter response.
pub(crate) fn parse_valuation_comparison(items: &[Value]) -> Result<Vec<ValuationComparisonRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(code) = opt_str(item, "CORRE_SECURITY_CODE") else {
            continue;
        };
        out.push(ValuationComparisonRow {
            code,
            name: opt_str(item, "CORRE_SECURITY_NAME"),
            pe_ttm: opt_f64(item, "PE_TTM"),
            pe_ttm_rank: opt_f64(item, "PE_TTM_RANK"),
            pe_lyr: opt_f64(item, "PE_LYR"),
            pe_lyr_rank: opt_f64(item, "PE_LYR_RANK"),
            pb_mqr: opt_f64(item, "PB_MQR"),
            pb_mqr_rank: opt_f64(item, "PB_MQR_RANK"),
            pb_lyr: opt_f64(item, "PB_LYR"),
            pb_lyr_rank: opt_f64(item, "PB_LYR_RANK"),
            ps_ttm: opt_f64(item, "PS_TTM"),
            ps_ttm_rank: opt_f64(item, "PS_TTM_RANK"),
            ps_lyr: opt_f64(item, "PS_LYR"),
            ps_lyr_rank: opt_f64(item, "PS_LYR_RANK"),
            pce_ttm: opt_f64(item, "PCE_TTM"),
            pce_ttm_rank: opt_f64(item, "PCE_TTM_RANK"),
            pce_lyr: opt_f64(item, "PCE_LYR"),
            pce_lyr_rank: opt_f64(item, "PCE_LYR_RANK"),
        });
    }
    Ok(out)
}

/// 东方财富-港股-行业对比-估值对比 (akshare `stock_hk_valuation_comparison_em`).
pub async fn valuation_comparison_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<ValuationComparisonRow>> {
    let filter = format!(r#"(SECUCODE="{symbol}.HK")(CORRE_SECUCODE="{symbol}.HK")"#);
    let data = em_sec_fetch(
        client,
        "stock_hk_valuation_comparison_em",
        "RPT_PCF10_INDUSTRY_HKCVALUE",
        "SECUCODE,SECURITY_CODE,ORG_CODE,REPORT_DATE,TYPE_ID,\
         TYPE_TYPE,TYPE_NAME,TYPE_NAME_EN,CORRE_SECURITY_CODE,\
         CORRE_SECUCODE,CORRE_SECURITY_NAME,PE_TTM,PE_LYR,PB_MQR,\
         PB_LYR,PS_TTM,PS_LYR,PCE_TTM,PCE_LYR,PE_TTM_RANK,PE_LYR_RANK,\
         PB_MQR_RANK,PB_LYR_RANK,PS_TTM_RANK,PS_LYR_RANK,PCE_TTM_RANK,PCE_LYR_RANK",
        &filter,
        "",
        "",
        "",
        "03445297742754925",
    )
    .await?;
    parse_valuation_comparison(&data)
}

// ---------------------------------------------------------------------------
// stock_hk_scale_comparison_em  (stock/stock_hk_comparison_em.py:118)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScaleComparisonRow {
    /// 代码 (CORRE_SECURITY_CODE)
    pub code: String,
    /// 简称 (CORRE_SECURITY_NAME)
    pub name: Option<String>,
    /// 总市值 (HKSDQMV)
    pub total_mv: Option<f64>,
    /// 总市值排名 (HKSDQMV_RANK)
    pub total_mv_rank: Option<f64>,
    /// 流通市值 (HKTOTAL_MARKET_CAP)
    pub float_mv: Option<f64>,
    /// 流通市值排名 (HKTOTAL_CAP_RANK)
    pub float_mv_rank: Option<f64>,
    /// 营业总收入 (OPERATE_INCOME)
    pub operate_income: Option<f64>,
    /// 营业总收入排名 (OPERATE_INCOME_RANK)
    pub operate_income_rank: Option<f64>,
    /// 净利润 (GROSS_PROFIT)
    pub net_profit: Option<f64>,
    /// 净利润排名 (GROSS_PROFIT_RANK)
    pub net_profit_rank: Option<f64>,
}

/// Parse `stock_hk_scale_comparison_em` rows from a datacenter response.
pub(crate) fn parse_scale_comparison(items: &[Value]) -> Result<Vec<ScaleComparisonRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(code) = opt_str(item, "CORRE_SECURITY_CODE") else {
            continue;
        };
        out.push(ScaleComparisonRow {
            code,
            name: opt_str(item, "CORRE_SECURITY_NAME"),
            total_mv: opt_f64(item, "HKSDQMV"),
            total_mv_rank: opt_f64(item, "HKSDQMV_RANK"),
            float_mv: opt_f64(item, "HKTOTAL_MARKET_CAP"),
            float_mv_rank: opt_f64(item, "HKTOTAL_CAP_RANK"),
            operate_income: opt_f64(item, "OPERATE_INCOME"),
            operate_income_rank: opt_f64(item, "OPERATE_INCOME_RANK"),
            net_profit: opt_f64(item, "GROSS_PROFIT"),
            net_profit_rank: opt_f64(item, "GROSS_PROFIT_RANK"),
        });
    }
    Ok(out)
}

/// 东方财富-港股-行业对比-规模对比 (akshare `stock_hk_scale_comparison_em`).
pub async fn scale_comparison_em(client: &Client, symbol: &str) -> Result<Vec<ScaleComparisonRow>> {
    let filter = format!(r#"(SECUCODE="{symbol}.HK")(CORRE_SECUCODE="{symbol}.HK")"#);
    let data = em_sec_fetch(
        client,
        "stock_hk_scale_comparison_em",
        "RPT_PCF10_INDUSTRY_SCALE",
        "SECURITY_CODE,SECUCODE,TYPE_ID,TYPE_TYPE,TYPE_NAME,\
         TYPE_NAME_EN,CORRE_SECURITY_CODE,CORRE_SECUCODE,\
         CORRE_SECURITY_NAME,MAXSTDREPORTDATE,HKSDQMV,\
         HKTOTAL_MARKET_CAP,OPERATE_INCOME,GROSS_PROFIT,\
         HKSDQMV_RANK,HKTOTAL_CAP_RANK,OPERATE_INCOME_RANK,GROSS_PROFIT_RANK",
        &filter,
        "",
        "",
        "",
        "07839693368708753",
    )
    .await?;
    parse_scale_comparison(&data)
}

// ---------------------------------------------------------------------------
// stock_hk_famous_spot_em  (stock/stock_hk_famous.py:13)  — push2 clist
// ---------------------------------------------------------------------------

/// A single HK quote row from a push2 `clist` response (shared by the well-known
/// HK stocks and the Stock-Connect component lists, which expose the same columns).
#[derive(Debug, Clone, serde::Serialize)]
pub struct HkSpotQuoteRow {
    /// 序号 (1-based ordinal over the returned `diff` rows)
    pub rank: usize,
    /// 代码 (f12)
    pub code: String,
    /// 名称 (f14)
    pub name: String,
    /// 最新价 (f2)
    pub price: Option<f64>,
    /// 涨跌幅 (f3)
    pub pct_change: Option<f64>,
    /// 涨跌额 (f4)
    pub change: Option<f64>,
    /// 成交量 (f5)
    pub volume: Option<f64>,
    /// 成交额 (f6)
    pub amount: Option<f64>,
    /// 最高 (f15)
    pub high: Option<f64>,
    /// 最低 (f16)
    pub low: Option<f64>,
    /// 今开 (f17)
    pub open: Option<f64>,
    /// 昨收 (f18)
    pub pre_close: Option<f64>,
}

/// 东方财富-行情中心-港股市场-知名港股 (akshare `stock_hk_famous_spot_em`).
pub async fn famous_spot_em(client: &Client) -> Result<Vec<HkSpotQuoteRow>> {
    let fields = "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f12,f13,f14,f15,f16,f17,f18,f20,f21,f23,f24,\
                  f25,f26,f22,f33,f11,f62,f128,f136,f115,f152";
    let v = push2_clist(
        client,
        "stock_hk_famous_spot_em",
        "b:DLMK0106",
        fields,
        "50000",
    )
    .await?;
    parse_hk_spot(&v)
}

// ---------------------------------------------------------------------------
// stock_hk_ggt_components_em  (stock_feature/stock_hsgt_em.py:94) — push2 clist
// ---------------------------------------------------------------------------

/// 东方财富-行情中心-港股市场-港股通成份股 (akshare `stock_hk_ggt_components_em`).
pub async fn ggt_components_em(client: &Client) -> Result<Vec<HkSpotQuoteRow>> {
    let fields = "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f12,f13,f14,f15,f16,f17,f18,f19,f20,f21,f23,f24,\
                  f25,f26,f22,f33,f11,f62,f128,f136,f115,f152";
    let v = push2_clist(
        client,
        "stock_hk_ggt_components_em",
        "b:DLMK0146,b:DLMK0144",
        fields,
        "100",
    )
    .await?;
    parse_hk_spot(&v)
}

// ---------------------------------------------------------------------------
// stock_hk_spot  (stock/stock_hk_sina.py:22) — Sina JSON array of arrays
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct HkSpotSinaRow {
    /// 日期时间 (index 12)
    pub datetime: Option<String>,
    /// 代码 (index 0)
    pub code: String,
    /// 中文名称 (index 1)
    pub name_cn: Option<String>,
    /// 英文名称 (index 2)
    pub name_en: Option<String>,
    /// 交易类型 (index 3)
    pub trade_type: Option<String>,
    /// 最新价 (index 4)
    pub price: Option<f64>,
    /// 涨跌额 (index 20)
    pub change: Option<f64>,
    /// 涨跌幅 (index 21)
    pub pct_change: Option<f64>,
    /// 昨收 (index 5)
    pub pre_close: Option<f64>,
    /// 今开 (index 6)
    pub open: Option<f64>,
    /// 最高 (index 7)
    pub high: Option<f64>,
    /// 最低 (index 8)
    pub low: Option<f64>,
    /// 成交量 (index 9)
    pub volume: Option<f64>,
    /// 成交额 (index 11)
    pub amount: Option<f64>,
    /// 买一 (index 13)
    pub bid1: Option<f64>,
    /// 卖一 (index 14)
    pub ask1: Option<f64>,
}

fn sina_arr_str(item: &Value, idx: usize) -> Option<String> {
    item.get(idx).and_then(|v| v.as_str()).map(str::to_string)
}

fn sina_arr_num(item: &Value, idx: usize) -> Option<f64> {
    item.get(idx).and_then(|v| v.as_f64())
}

/// Parse one Sina HK spot row (a positional JSON array).
fn parse_sina_item(item: &Value) -> HkSpotSinaRow {
    HkSpotSinaRow {
        datetime: sina_arr_str(item, 12),
        code: sina_arr_str(item, 0).unwrap_or_default(),
        name_cn: sina_arr_str(item, 1),
        name_en: sina_arr_str(item, 2),
        trade_type: sina_arr_str(item, 3),
        price: sina_arr_num(item, 4),
        change: sina_arr_num(item, 20),
        pct_change: sina_arr_num(item, 21),
        pre_close: sina_arr_num(item, 5),
        open: sina_arr_num(item, 6),
        high: sina_arr_num(item, 7),
        low: sina_arr_num(item, 8),
        volume: sina_arr_num(item, 9),
        amount: sina_arr_num(item, 11),
        bid1: sina_arr_num(item, 13),
        ask1: sina_arr_num(item, 14),
    }
}

/// Parse the Sina HK spot response (a JSON array of positional row arrays).
pub(crate) fn parse_sina_spot(resp: &Value) -> Result<Vec<HkSpotSinaRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "expected a JSON array".into(),
    })?;
    Ok(arr.iter().map(parse_sina_item).collect())
}

/// 新浪财经-港股-所有港股的实时行情数据 (akshare `stock_hk_spot`).
///
/// Sina paginates this endpoint (60 rows/page) and the source walks pages until an
/// empty page. We mirror that: fetch pages 1..=100, stop early on an empty page.
pub async fn spot(client: &Client) -> Result<Vec<HkSpotSinaRow>> {
    let mut out = Vec::new();
    for page in 1..=100 {
        let page_s = page.to_string();
        let params: Vec<(&str, &str)> = vec![
            ("page", page_s.as_str()),
            ("num", "60"),
            ("sort", "symbol"),
            ("asc", "1"),
            ("node", "qbgg_hk"),
            ("_s_r_a", "init"),
        ];
        let v = client
            .get_json(SOURCE_SINA, "stock_hk_spot", SINA_HK_SPOT_URL, &params)
            .await?;
        let arr = v.as_array().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "expected a JSON array".into(),
        })?;
        if arr.is_empty() {
            break;
        }
        out.extend(parse_sina_spot(&v)?);
    }
    Ok(out)
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

    // ---- Eastmoney F10 datacenter securities reports ----

    #[test]
    fn parse_security_profile_ok() {
        let rows = parse_security_profile(
            &em_data_array(&fixture("stock_hk_security_profile_em.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].secucode, "03900.HK");
        assert_eq!(rows[0].name, "NDRG");
        assert_eq!(rows[0].listing_date, Some("2010-09-29".into()));
        assert_eq!(rows[0].issue_price, Some(3.7));
        assert_eq!(rows[0].trade_unit, Some(2000.0));
        assert_eq!(rows[0].ganggutongbiaodishen, Some("是".into()));
    }

    #[test]
    fn parse_company_profile_ok() {
        let rows = parse_company_profile(
            &em_data_array(&fixture("stock_hk_company_profile_em.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].org_name, "Example Holdings Ltd");
        assert_eq!(rows[0].belong_industry, Some("银行".into()));
        assert_eq!(rows[0].emp_num, Some(12345.0));
        assert_eq!(rows[0].found_date, Some("1964-01-01".into()));
    }

    #[test]
    fn parse_financial_indicator_ok() {
        let rows = parse_financial_indicator(
            &em_data_array(&fixture("stock_hk_financial_indicator_em.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert!(approx(rows[0].basic_eps, 1.23));
        assert!(approx(rows[0].pe_ttm, 9.8));
        assert!(approx(rows[0].pb_ttm, 0.85));
        assert!(approx(rows[0].holder_profit, 1200000000.0));
        assert!(approx(rows[0].dividend_rate, 5.6));
    }

    #[test]
    fn parse_dividend_payout_ok() {
        let rows = parse_dividend_payout(
            &em_data_array(&fixture("stock_hk_dividend_payout_em.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].year, Some("2023".into()));
        assert_eq!(rows[0].plan_explain, Some("派息每股0.5港元".into()));
        assert_eq!(rows[0].ex_dividend_date, Some("2024-05-10".into()));
        assert_eq!(rows[1].report_type, Some("中期".into()));
    }

    #[test]
    fn parse_growth_comparison_ok() {
        let rows = parse_growth_comparison(
            &em_data_array(&fixture("stock_hk_growth_comparison_em.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "03900.HK");
        assert!(approx(rows[0].eps_yoy, 8.5));
        assert!(approx(rows[0].eps_yoy_rank, 3.0));
        assert!(approx(rows[1].operate_profit_yoy, 10.2));
    }

    #[test]
    fn parse_valuation_comparison_ok() {
        let rows = parse_valuation_comparison(
            &em_data_array(&fixture("stock_hk_valuation_comparison_em.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "03900.HK");
        assert!(approx(rows[0].pe_ttm, 9.8));
        assert!(approx(rows[0].pe_ttm_rank, 5.0));
        assert!(approx(rows[1].pb_lyr, 1.9));
    }

    #[test]
    fn parse_scale_comparison_ok() {
        let rows = parse_scale_comparison(
            &em_data_array(&fixture("stock_hk_scale_comparison_em.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "03900.HK");
        assert!(approx(rows[0].total_mv, 350000000000.0));
        assert!(approx(rows[0].total_mv_rank, 2.0));
        assert!(approx(rows[1].net_profit, 1200000000.0));
    }

    // ---- Eastmoney push2 clist ----

    #[test]
    fn parse_famous_spot_ok() {
        let rows = parse_hk_spot(&fixture("stock_hk_famous_spot_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].rank, 1);
        assert_eq!(rows[0].code, "00700");
        assert_eq!(rows[0].name, "腾讯控股");
        assert!(approx(rows[0].price, 372.8));
        assert!(approx(rows[0].pct_change, 1.23));
        assert!(approx(rows[0].pre_close, 368.3));
    }

    #[test]
    fn parse_ggt_components_ok() {
        let rows = parse_hk_spot(&fixture("stock_hk_ggt_components_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "00939");
        assert_eq!(rows[0].name, "建设银行");
        assert!(approx(rows[0].price, 6.12));
        assert!(approx(rows[0].volume, 123456789.0));
        assert!(approx(rows[1].amount, 987654321.0));
    }

    // ---- Sina HK spot ----

    #[test]
    fn parse_sina_spot_ok() {
        let rows = parse_sina_spot(&fixture("stock_hk_spot.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "00700");
        assert_eq!(rows[0].name_cn, Some("腾讯控股".to_string()));
        assert_eq!(rows[0].datetime, Some("2024-05-10 16:08:00".into()));
        assert!(approx(rows[0].price, 372.8));
        assert!(approx(rows[0].pct_change, 1.23));
        assert!(approx(rows[0].bid1, 372.6));
        assert!(approx(rows[1].ask1, 5.10));
    }
}
