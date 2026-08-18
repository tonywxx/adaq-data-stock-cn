//! 东方财富网-数据中心-股东分析 (akshare `akshare/stock_feature/stock_gdfx_em.py`).
//!
//! Ported public functions (all pure Eastmoney JSON, no JS/token/signature):
//!
//! | Rust fn                               | akshare fn                        | Endpoint                       | Data path        |
//! |---------------------------------------|-----------------------------------|--------------------------------|------------------|
//! | `stock_gdfx_free_holding_statistics_em` | `stock_gdfx_free_holding_statistics_em` | datacenter `RPT_COOPFREEHOLDERS_ANALYSIS` | `result.data`    |
//! | `stock_gdfx_holding_statistics_em`      | `stock_gdfx_holding_statistics_em`      | datacenter `RPT_COOPHOLDERS_ANALYSIS`     | `result.data`    |
//! | `stock_gdfx_free_holding_change_em`     | `stock_gdfx_free_holding_change_em`     | datacenter `RPT_FREEHOLDERS_BASIC_INFO`   | `result.data`    |
//! | `stock_gdfx_holding_change_em`          | `stock_gdfx_holding_change_em`          | datacenter `RPT_HOLDERS_BASIC_INFO`       | `result.data`    |
//! | `stock_gdfx_free_top_10_em`             | `stock_gdfx_free_top_10_em`             | emweb `PageSDLTGD`              | `sdltgd`         |
//! | `stock_gdfx_top_10_em`                  | `stock_gdfx_top_10_em`                  | emweb `PageSDGD`                | `sdgd`           |
//! | `stock_gdfx_free_holding_detail_em`     | `stock_gdfx_free_holding_detail_em`     | datacenter `RPT_F10_EH_FREEHOLDERS`       | `result.data`    |
//! | `stock_gdfx_holding_detail_em`          | `stock_gdfx_holding_detail_em`          | datacenter `RPT_DMSK_HOLDERS`           | `result.data`    |
//! | `stock_gdfx_free_holding_analyse_em`    | `stock_gdfx_free_holding_analyse_em`    | datacenter `RPT_CUSTOM_F10_EH_FREEHOLDERS_JOIN_FREEHOLDER_SHAREANALYSIS` | `result.data` |
//! | `stock_gdfx_holding_analyse_em`         | `stock_gdfx_holding_analyse_em`         | datacenter `RPT_CUSTOM_DMSK_HOLDERS_JOIN_HOLDER_SHAREANALYSIS` | `result.data` |
//! | `stock_gdfx_free_holding_teamwork_em`   | `stock_gdfx_free_holding_teamwork_em`   | datacenter `RPT_COOPFREEHOLDER`          | `result.data`    |
//! | `stock_gdfx_holding_teamwork_em`        | `stock_gdfx_holding_teamwork_em`        | datacenter `RPT_TENHOLDERS_COOPHOLDERS`  | `result.data`    |
//!
//! ## Field-name fidelity note
//!
//! For `free/top_10`, `statistics`, `change` and `teamwork` functions, akshare
//! discards the upstream Eastmoney JSON field keys and replaces them with
//! **positional** Chinese column labels (`big_df.columns = [...]`). The real
//! upstream field names are therefore not recoverable from the akshare source.
//! The field names used in those row structs below are **inferred** from the
//! report name, `sortColumns` and column semantics, and must be verified
//! against a live sample before production use. The four `*_detail_em` /
//! `*_analyse_em` functions use akshare's `.rename(columns={...})`, so their
//! field names ARE the real upstream keys and are ported exactly.
//!
//! ## DEFERRED
//!
//! None. Every public function in `stock_gdfx_em.py` is a pure HTTP request to
//! an Eastmoney JSON API (datacenter-web or emweb.securities.eastmoney.com);
//! none require JS execution, tokens, signatures, `execjs`/`MiniRacer`, cookies
//! or HTML/Excel scraping.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use crate::core::json::*;

const SOURCE_EASTMONEY: &str = "eastmoney";
const DATACENTER: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";

// ---------------------------------------------------------------------------
// Helpers (copied verbatim per porting brief)
// ---------------------------------------------------------------------------


/// Extract `result.data` (the row array) from a datacenter-web response.
fn result_data(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data".into(),
        })
}

/// Extract a top-level array field (e.g. `sdltgd`/`sdgd`) from an emweb response.
fn root_array<'a>(resp: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    resp.get(key)
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: format!("missing {key} array"),
        })
}

/// Validate an `YYYYMMDD` date and return it dashed as `YYYY-MM-DD`
/// (Eastmoney's expected `END_DATE` / emweb `date` form).
fn fmt_date8(date: &str) -> Result<String> {
    if date.len() == 8 && date.bytes().all(|b| b.is_ascii_digit()) {
        Ok(format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8]))
    } else {
        Err(Error::InvalidParam(format!(
            "date must be 8 ASCII digits YYYYMMDD, got {date:?}"
        )))
    }
}

// ===========================================================================
// stock_gdfx_free_holding_statistics_em — 数据中心-股东持股统计-十大流通股东
// ===========================================================================

/// One row of free-float top-10 holder statistics, port of
/// `stock_gdfx_free_holding_statistics_em` (Eastmoney `RPT_COOPFREEHOLDERS_ANALYSIS`).
///
/// Field names are **inferred** (akshare relabels positionally — see module note).
#[derive(Debug, Clone, serde::Serialize)]
pub struct GdfxFreeHoldingStatistics {
    /// 股东名称 (inferred `HOLDER_NAME`)
    pub holder_name: Option<String>,
    /// 股东类型 (inferred `HOLDER_TYPE`)
    pub holder_type: Option<String>,
    /// 统计次数 (inferred `STATISTICS_TIMES`)
    pub statistics_times: Option<i64>,
    /// 公告日后涨幅统计-10个交易日-平均涨幅 (inferred `D10_AVG`)
    pub d10_avg: Option<f64>,
    /// 公告日后涨幅统计-10个交易日-最大涨幅 (inferred `D10_MAX`)
    pub d10_max: Option<f64>,
    /// 公告日后涨幅统计-10个交易日-最小涨幅 (inferred `D10_MIN`)
    pub d10_min: Option<f64>,
    /// 公告日后涨幅统计-30个交易日-平均涨幅 (inferred `D30_AVG`)
    pub d30_avg: Option<f64>,
    /// 公告日后涨幅统计-30个交易日-最大涨幅 (inferred `D30_MAX`)
    pub d30_max: Option<f64>,
    /// 公告日后涨幅统计-30个交易日-最小涨幅 (inferred `D30_MIN`)
    pub d30_min: Option<f64>,
    /// 公告日后涨幅统计-60个交易日-平均涨幅 (inferred `D60_AVG`)
    pub d60_avg: Option<f64>,
    /// 公告日后涨幅统计-60个交易日-最大涨幅 (inferred `D60_MAX`)
    pub d60_max: Option<f64>,
    /// 公告日后涨幅统计-60个交易日-最小涨幅 (inferred `D60_MIN`)
    pub d60_min: Option<f64>,
    /// 持有个股 (inferred `HOLD_STOCKS`)
    pub hold_stocks: Option<String>,
    pub source: &'static str,
}

/// Port of `stock_gdfx_free_holding_statistics_em(date)`.
pub async fn stock_gdfx_free_holding_statistics_em(
    client: &Client,
    date: &str,
) -> Result<Vec<GdfxFreeHoldingStatistics>> {
    let d = fmt_date8(date)?;
    let filter = format!("(HOLDNUM_CHANGE_TYPE=\"001\")(END_DATE='{d}')");
    let params = [
        ("reportName", "RPT_COOPFREEHOLDERS_ANALYSIS"),
        ("columns", "ALL"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("sortColumns", "STATISTICS_TIMES,COOPERATION_HOLDER_MARK"),
        ("sortTypes", "-1,-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_gdfx_free_holding_statistics_em",
            DATACENTER,
            &params,
        )
        .await?;
    parse_free_holding_statistics(&v)
}

pub(crate) fn parse_free_holding_statistics(
    resp: &Value,
) -> Result<Vec<GdfxFreeHoldingStatistics>> {
    let data = result_data(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(GdfxFreeHoldingStatistics {
            holder_name: opt_str(item, "HOLDER_NAME"),
            holder_type: opt_str(item, "HOLDER_TYPE"),
            statistics_times: opt_i64(item, "STATISTICS_TIMES"),
            d10_avg: opt_f64(item, "D10_AVG"),
            d10_max: opt_f64(item, "D10_MAX"),
            d10_min: opt_f64(item, "D10_MIN"),
            d30_avg: opt_f64(item, "D30_AVG"),
            d30_max: opt_f64(item, "D30_MAX"),
            d30_min: opt_f64(item, "D30_MIN"),
            d60_avg: opt_f64(item, "D60_AVG"),
            d60_max: opt_f64(item, "D60_MAX"),
            d60_min: opt_f64(item, "D60_MIN"),
            hold_stocks: opt_str(item, "HOLD_STOCKS"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_gdfx_holding_statistics_em — 数据中心-股东持股统计-十大股东
// ===========================================================================

/// One row of top-10 holder statistics, port of `stock_gdfx_holding_statistics_em`
/// (Eastmoney `RPT_COOPHOLDERS_ANALYSIS`). Field names inferred — see module note.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GdfxHoldingStatistics {
    /// 股东名称 (inferred `HOLDER_NAME`)
    pub holder_name: Option<String>,
    /// 股东类型 (inferred `HOLDER_TYPE`)
    pub holder_type: Option<String>,
    /// 统计次数 (inferred `STATISTICS_TIMES`)
    pub statistics_times: Option<i64>,
    /// 公告日后涨幅统计-10个交易日-平均涨幅 (inferred `D10_AVG`)
    pub d10_avg: Option<f64>,
    /// 公告日后涨幅统计-10个交易日-最大涨幅 (inferred `D10_MAX`)
    pub d10_max: Option<f64>,
    /// 公告日后涨幅统计-10个交易日-最小涨幅 (inferred `D10_MIN`)
    pub d10_min: Option<f64>,
    /// 公告日后涨幅统计-30个交易日-平均涨幅 (inferred `D30_AVG`)
    pub d30_avg: Option<f64>,
    /// 公告日后涨幅统计-30个交易日-最大涨幅 (inferred `D30_MAX`)
    pub d30_max: Option<f64>,
    /// 公告日后涨幅统计-30个交易日-最小涨幅 (inferred `D30_MIN`)
    pub d30_min: Option<f64>,
    /// 公告日后涨幅统计-60个交易日-平均涨幅 (inferred `D60_AVG`)
    pub d60_avg: Option<f64>,
    /// 公告日后涨幅统计-60个交易日-最大涨幅 (inferred `D60_MAX`)
    pub d60_max: Option<f64>,
    /// 公告日后涨幅统计-60个交易日-最小涨幅 (inferred `D60_MIN`)
    pub d60_min: Option<f64>,
    /// 持有个股 (inferred `HOLD_STOCKS`)
    pub hold_stocks: Option<String>,
    pub source: &'static str,
}

/// Port of `stock_gdfx_holding_statistics_em(date)`.
pub async fn stock_gdfx_holding_statistics_em(
    client: &Client,
    date: &str,
) -> Result<Vec<GdfxHoldingStatistics>> {
    let d = fmt_date8(date)?;
    let filter = format!("(HOLDNUM_CHANGE_TYPE=\"001\")(END_DATE='{d}')");
    let params = [
        ("reportName", "RPT_COOPHOLDERS_ANALYSIS"),
        ("columns", "ALL"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("sortColumns", "STATISTICS_TIMES,COOPERATION_HOLDER_MARK"),
        ("sortTypes", "-1,-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_gdfx_holding_statistics_em",
            DATACENTER,
            &params,
        )
        .await?;
    parse_holding_statistics(&v)
}

pub(crate) fn parse_holding_statistics(resp: &Value) -> Result<Vec<GdfxHoldingStatistics>> {
    let data = result_data(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(GdfxHoldingStatistics {
            holder_name: opt_str(item, "HOLDER_NAME"),
            holder_type: opt_str(item, "HOLDER_TYPE"),
            statistics_times: opt_i64(item, "STATISTICS_TIMES"),
            d10_avg: opt_f64(item, "D10_AVG"),
            d10_max: opt_f64(item, "D10_MAX"),
            d10_min: opt_f64(item, "D10_MIN"),
            d30_avg: opt_f64(item, "D30_AVG"),
            d30_max: opt_f64(item, "D30_MAX"),
            d30_min: opt_f64(item, "D30_MIN"),
            d60_avg: opt_f64(item, "D60_AVG"),
            d60_max: opt_f64(item, "D60_MAX"),
            d60_min: opt_f64(item, "D60_MIN"),
            hold_stocks: opt_str(item, "HOLD_STOCKS"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_gdfx_free_holding_change_em — 数据中心-股东持股变动统计-十大流通股东
// ===========================================================================

/// One row of free-float holder change statistics, port of
/// `stock_gdfx_free_holding_change_em` (Eastmoney `RPT_FREEHOLDERS_BASIC_INFO`).
/// Field names inferred — see module note.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GdfxFreeHoldingChange {
    /// 股东名称 (inferred `HOLDER_NAME`)
    pub holder_name: Option<String>,
    /// 股东类型 (inferred `HOLDER_TYPE`)
    pub holder_type: Option<String>,
    /// 期末持股只数统计-总持有 (inferred `HOLD_NUM_TOTAL`)
    pub hold_num_total: Option<f64>,
    /// 期末持股只数统计-新进 (inferred `HOLD_NUM_NEW`)
    pub hold_num_new: Option<f64>,
    /// 期末持股只数统计-增加 (inferred `HOLD_NUM_INCREASE`)
    pub hold_num_increase: Option<f64>,
    /// 期末持股只数统计-减少 (inferred `HOLD_NUM_DECREASE`)
    pub hold_num_decrease: Option<f64>,
    /// 期末持股只数统计-不变 (inferred `HOLD_NUM_UNCHANGED`)
    pub hold_num_unchanged: Option<f64>,
    /// 流通市值统计 (inferred `MARKET_CAP`)
    pub market_cap: Option<f64>,
    /// 持有个股 (inferred `HOLD_STOCKS`)
    pub hold_stocks: Option<String>,
    pub source: &'static str,
}

/// Port of `stock_gdfx_free_holding_change_em(date)`.
pub async fn stock_gdfx_free_holding_change_em(
    client: &Client,
    date: &str,
) -> Result<Vec<GdfxFreeHoldingChange>> {
    let d = fmt_date8(date)?;
    let filter = format!("(END_DATE='{d}')");
    let params = [
        ("reportName", "RPT_FREEHOLDERS_BASIC_INFO"),
        ("columns", "ALL"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("sortColumns", "HOLDER_NUM,HOLDER_NEW"),
        ("sortTypes", "-1,-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_gdfx_free_holding_change_em",
            DATACENTER,
            &params,
        )
        .await?;
    parse_free_holding_change(&v)
}

pub(crate) fn parse_free_holding_change(resp: &Value) -> Result<Vec<GdfxFreeHoldingChange>> {
    let data = result_data(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(GdfxFreeHoldingChange {
            holder_name: opt_str(item, "HOLDER_NAME"),
            holder_type: opt_str(item, "HOLDER_TYPE"),
            hold_num_total: opt_f64(item, "HOLD_NUM_TOTAL"),
            hold_num_new: opt_f64(item, "HOLD_NUM_NEW"),
            hold_num_increase: opt_f64(item, "HOLD_NUM_INCREASE"),
            hold_num_decrease: opt_f64(item, "HOLD_NUM_DECREASE"),
            hold_num_unchanged: opt_f64(item, "HOLD_NUM_UNCHANGED"),
            market_cap: opt_f64(item, "MARKET_CAP"),
            hold_stocks: opt_str(item, "HOLD_STOCKS"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_gdfx_holding_change_em — 数据中心-股东持股变动统计-十大股东
// ===========================================================================

/// One row of top-10 holder change statistics, port of
/// `stock_gdfx_holding_change_em` (Eastmoney `RPT_HOLDERS_BASIC_INFO`).
/// Field names inferred — see module note.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GdfxHoldingChange {
    /// 股东名称 (inferred `HOLDER_NAME`)
    pub holder_name: Option<String>,
    /// 股东类型 (inferred `HOLDER_TYPE`)
    pub holder_type: Option<String>,
    /// 期末持股只数统计-总持有 (inferred `HOLD_NUM_TOTAL`)
    pub hold_num_total: Option<f64>,
    /// 期末持股只数统计-新进 (inferred `HOLD_NUM_NEW`)
    pub hold_num_new: Option<f64>,
    /// 期末持股只数统计-增加 (inferred `HOLD_NUM_INCREASE`)
    pub hold_num_increase: Option<f64>,
    /// 期末持股只数统计-减少 (inferred `HOLD_NUM_DECREASE`)
    pub hold_num_decrease: Option<f64>,
    /// 期末持股只数统计-不变 (inferred `HOLD_NUM_UNCHANGED`)
    pub hold_num_unchanged: Option<f64>,
    /// 流通市值统计 (inferred `MARKET_CAP`)
    pub market_cap: Option<f64>,
    /// 持有个股 (inferred `HOLD_STOCKS`)
    pub hold_stocks: Option<String>,
    pub source: &'static str,
}

/// Port of `stock_gdfx_holding_change_em(date)`.
pub async fn stock_gdfx_holding_change_em(
    client: &Client,
    date: &str,
) -> Result<Vec<GdfxHoldingChange>> {
    let d = fmt_date8(date)?;
    let filter = format!("(END_DATE='{d}')");
    let params = [
        ("reportName", "RPT_HOLDERS_BASIC_INFO"),
        ("columns", "ALL"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("sortColumns", "HOLDER_NUM,HOLDER_NEW"),
        ("sortTypes", "-1,-1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_gdfx_holding_change_em",
            DATACENTER,
            &params,
        )
        .await?;
    parse_holding_change(&v)
}

pub(crate) fn parse_holding_change(resp: &Value) -> Result<Vec<GdfxHoldingChange>> {
    let data = result_data(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(GdfxHoldingChange {
            holder_name: opt_str(item, "HOLDER_NAME"),
            holder_type: opt_str(item, "HOLDER_TYPE"),
            hold_num_total: opt_f64(item, "HOLD_NUM_TOTAL"),
            hold_num_new: opt_f64(item, "HOLD_NUM_NEW"),
            hold_num_increase: opt_f64(item, "HOLD_NUM_INCREASE"),
            hold_num_decrease: opt_f64(item, "HOLD_NUM_DECREASE"),
            hold_num_unchanged: opt_f64(item, "HOLD_NUM_UNCHANGED"),
            market_cap: opt_f64(item, "MARKET_CAP"),
            hold_stocks: opt_str(item, "HOLD_STOCKS"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_gdfx_free_top_10_em — 个股-十大流通股东 (emweb PageSDLTGD)
// ===========================================================================

/// One free-float top-10 holder row, port of `stock_gdfx_free_top_10_em`
/// (emweb `PC_HSF10/ShareholderResearch/PageSDLTGD`, data under `sdltgd`).
/// Field names inferred — see module note.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GdfxFreeTop10 {
    /// 名次 (inferred `RANK`)
    pub rank: Option<i64>,
    /// 股东名称 (inferred `HOLDER_NAME`)
    pub holder_name: Option<String>,
    /// 股东性质 (inferred `HOLDER_NATURE`)
    pub holder_nature: Option<String>,
    /// 股份类型 (inferred `SHARES_TYPE`)
    pub shares_type: Option<String>,
    /// 持股数 (inferred `HOLD_NUM`)
    pub hold_num: Option<f64>,
    /// 占总流通股本持股比例 (inferred `HOLD_RATIO`)
    pub hold_ratio: Option<f64>,
    /// 增减 (inferred `HOLD_CHANGE`)
    pub hold_change: Option<f64>,
    /// 变动比率 (inferred `CHANGE_RATIO`)
    pub change_ratio: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_gdfx_free_top_10_em(symbol, date)`.
///
/// `symbol` is a market-prefixed code (e.g. `sh688686`); `date` is `YYYYMMDD`.
pub async fn stock_gdfx_free_top_10_em(
    client: &Client,
    symbol: &str,
    date: &str,
) -> Result<Vec<GdfxFreeTop10>> {
    let d = fmt_date8(date)?;
    let code = symbol.to_uppercase();
    let params = [("code", code.as_str()), ("date", d.as_str())];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_gdfx_free_top_10_em",
            "https://emweb.securities.eastmoney.com/PC_HSF10/ShareholderResearch/PageSDLTGD",
            &params,
        )
        .await?;
    parse_free_top_10(&v)
}

pub(crate) fn parse_free_top_10(resp: &Value) -> Result<Vec<GdfxFreeTop10>> {
    let data = root_array(resp, "sdltgd")?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(GdfxFreeTop10 {
            rank: opt_i64(item, "RANK"),
            holder_name: opt_str(item, "HOLDER_NAME"),
            holder_nature: opt_str(item, "HOLDER_NATURE"),
            shares_type: opt_str(item, "SHARES_TYPE"),
            hold_num: opt_f64(item, "HOLD_NUM"),
            hold_ratio: opt_f64(item, "HOLD_RATIO"),
            hold_change: opt_f64(item, "HOLD_CHANGE"),
            change_ratio: opt_f64(item, "CHANGE_RATIO"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_gdfx_top_10_em — 个股-十大股东 (emweb PageSDGD)
// ===========================================================================

/// One top-10 holder row, port of `stock_gdfx_top_10_em`
/// (emweb `PC_HSF10/ShareholderResearch/PageSDGD`, data under `sdgd`).
/// Field names inferred — see module note.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GdfxTop10 {
    /// 名次 (inferred `RANK`)
    pub rank: Option<i64>,
    /// 股东名称 (inferred `HOLDER_NAME`)
    pub holder_name: Option<String>,
    /// 股份类型 (inferred `SHARES_TYPE`)
    pub shares_type: Option<String>,
    /// 持股数 (inferred `HOLD_NUM`)
    pub hold_num: Option<f64>,
    /// 占总股本持股比例 (inferred `HOLD_RATIO`)
    pub hold_ratio: Option<f64>,
    /// 增减 (inferred `HOLD_CHANGE`)
    pub hold_change: Option<f64>,
    /// 变动比率 (inferred `CHANGE_RATIO`)
    pub change_ratio: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_gdfx_top_10_em(symbol, date)`.
pub async fn stock_gdfx_top_10_em(
    client: &Client,
    symbol: &str,
    date: &str,
) -> Result<Vec<GdfxTop10>> {
    let d = fmt_date8(date)?;
    let code = symbol.to_uppercase();
    let params = [("code", code.as_str()), ("date", d.as_str())];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_gdfx_top_10_em",
            "https://emweb.securities.eastmoney.com/PC_HSF10/ShareholderResearch/PageSDGD",
            &params,
        )
        .await?;
    parse_top_10(&v)
}

pub(crate) fn parse_top_10(resp: &Value) -> Result<Vec<GdfxTop10>> {
    let data = root_array(resp, "sdgd")?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(GdfxTop10 {
            rank: opt_i64(item, "RANK"),
            holder_name: opt_str(item, "HOLDER_NAME"),
            shares_type: opt_str(item, "SHARES_TYPE"),
            hold_num: opt_f64(item, "HOLD_NUM"),
            hold_ratio: opt_f64(item, "HOLD_RATIO"),
            hold_change: opt_f64(item, "HOLD_CHANGE"),
            change_ratio: opt_f64(item, "CHANGE_RATIO"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_gdfx_free_holding_detail_em — 数据中心-股东持股明细-十大流通股东
// ===========================================================================

/// One free-float holder detail row, port of `stock_gdfx_free_holding_detail_em`
/// (Eastmoney `RPT_F10_EH_FREEHOLDERS`). Field names are the **real** upstream
/// keys (akshare renames them via `.rename`, so we recover them exactly).
#[derive(Debug, Clone, serde::Serialize)]
pub struct GdfxFreeHoldingDetail {
    /// 股东名称 (`HOLDER_NAME`)
    pub holder_name: Option<String>,
    /// 股东类型 (`HOLDER_TYPE`)
    pub holder_type: Option<String>,
    /// 股票代码 (`SECURITY_CODE`)
    pub security_code: Option<String>,
    /// 股票简称 (`SECURITY_NAME_ABBR`)
    pub security_name: Option<String>,
    /// 报告期 (`END_DATE`)
    pub end_date: Option<String>,
    /// 期末持股-数量 (`HOLD_NUM`)
    pub hold_num: Option<f64>,
    /// 期末持股-数量变化 (`XZCHANGE`)
    pub xzchange: Option<f64>,
    /// 期末持股-数量变化比例 (`CHANGE_RATIO`)
    pub change_ratio: Option<f64>,
    /// 期末持股-持股变动 (`HOLDNUM_CHANGE_NAME`)
    pub holdnum_change_name: Option<String>,
    /// 期末持股-流通市值 (`HOLDER_MARKET_CAP`)
    pub holder_market_cap: Option<f64>,
    /// 公告日 (`UPDATE_DATE`)
    pub update_date: Option<String>,
    pub source: &'static str,
}

/// Port of `stock_gdfx_free_holding_detail_em(date)`.
pub async fn stock_gdfx_free_holding_detail_em(
    client: &Client,
    date: &str,
) -> Result<Vec<GdfxFreeHoldingDetail>> {
    let d = fmt_date8(date)?;
    let filter = format!("(END_DATE='{d}')");
    let params = [
        ("reportName", "RPT_F10_EH_FREEHOLDERS"),
        ("columns", "ALL"),
        ("pageSize", "2000"),
        ("pageNumber", "1"),
        ("sortColumns", "UPDATE_DATE,SECURITY_CODE,HOLDER_RANK"),
        ("sortTypes", "-1,1,1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_gdfx_free_holding_detail_em",
            DATACENTER,
            &params,
        )
        .await?;
    parse_free_holding_detail(&v)
}

pub(crate) fn parse_free_holding_detail(resp: &Value) -> Result<Vec<GdfxFreeHoldingDetail>> {
    let data = result_data(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(GdfxFreeHoldingDetail {
            holder_name: opt_str(item, "HOLDER_NAME"),
            holder_type: opt_str(item, "HOLDER_TYPE"),
            security_code: opt_str(item, "SECURITY_CODE"),
            security_name: opt_str(item, "SECURITY_NAME_ABBR"),
            end_date: opt_str(item, "END_DATE"),
            hold_num: opt_f64(item, "HOLD_NUM"),
            xzchange: opt_f64(item, "XZCHANGE"),
            change_ratio: opt_f64(item, "CHANGE_RATIO"),
            holdnum_change_name: opt_str(item, "HOLDNUM_CHANGE_NAME"),
            holder_market_cap: opt_f64(item, "HOLDER_MARKET_CAP"),
            update_date: opt_str(item, "UPDATE_DATE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_gdfx_holding_detail_em — 数据中心-股东持股明细-十大股东
// ===========================================================================

/// One top-10 holder detail row, port of `stock_gdfx_holding_detail_em`
/// (Eastmoney `RPT_DMSK_HOLDERS`). Field names are the **real** upstream keys.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GdfxHoldingDetail {
    /// 股东名称 (`HOLDER_NAME`)
    pub holder_name: Option<String>,
    /// 股东类型 (`HOLDER_NEWTYPE`)
    pub holder_newtype: Option<String>,
    /// 股东排名 (`RANK`)
    pub rank: Option<i64>,
    /// 股票代码 (`SECURITY_CODE`)
    pub security_code: Option<String>,
    /// 股票简称 (`SECURITY_NAME_ABBR`)
    pub security_name: Option<String>,
    /// 报告期 (`END_DATE`)
    pub end_date: Option<String>,
    /// 期末持股-数量 (`HOLD_NUM`)
    pub hold_num: Option<f64>,
    /// 期末持股-数量变化 (`HOLD_NUM_CHANGE`)
    pub hold_num_change: Option<f64>,
    /// 期末持股-数量变化比例 (`HOLD_RATIO_CHANGE`)
    pub hold_ratio_change: Option<f64>,
    /// 期末持股-持股变动 (`HOLDNUM_CHANGE_NAME`)
    pub holdnum_change_name: Option<String>,
    /// 期末持股-流通市值 (`HOLDER_MARKET_CAP`)
    pub holder_market_cap: Option<f64>,
    /// 公告日 (`NOTICE_DATE`)
    pub notice_date: Option<String>,
    pub source: &'static str,
}

/// Port of `stock_gdfx_holding_detail_em(date, indicator, symbol)`.
///
/// `indicator` ∈ {"个人","基金","QFII","社保","券商","信托"}; `symbol` (持股变动)
/// ∈ {"新进","增加","不变","减少"}. Both feed the `filter` unvalidated (mirrors
/// akshare, which passes them straight through).
pub async fn stock_gdfx_holding_detail_em(
    client: &Client,
    date: &str,
    indicator: &str,
    symbol: &str,
) -> Result<Vec<GdfxHoldingDetail>> {
    let d = fmt_date8(date)?;
    let filter = format!(
        "(HOLDER_NEWTYPE=\"{indicator}\")(HOLDNUM_CHANGE_NAME=\"{symbol}\")(END_DATE='{d}')"
    );
    let params = [
        ("reportName", "RPT_DMSK_HOLDERS"),
        ("columns", "ALL"),
        ("pageSize", "50"),
        ("pageNumber", "1"),
        ("sortColumns", "NOTICE_DATE,SECURITY_CODE,RANK"),
        ("sortTypes", "-1,1,1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_gdfx_holding_detail_em",
            DATACENTER,
            &params,
        )
        .await?;
    parse_holding_detail(&v)
}

pub(crate) fn parse_holding_detail(resp: &Value) -> Result<Vec<GdfxHoldingDetail>> {
    let data = result_data(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(GdfxHoldingDetail {
            holder_name: opt_str(item, "HOLDER_NAME"),
            holder_newtype: opt_str(item, "HOLDER_NEWTYPE"),
            rank: opt_i64(item, "RANK"),
            security_code: opt_str(item, "SECURITY_CODE"),
            security_name: opt_str(item, "SECURITY_NAME_ABBR"),
            end_date: opt_str(item, "END_DATE"),
            hold_num: opt_f64(item, "HOLD_NUM"),
            hold_num_change: opt_f64(item, "HOLD_NUM_CHANGE"),
            hold_ratio_change: opt_f64(item, "HOLD_RATIO_CHANGE"),
            holdnum_change_name: opt_str(item, "HOLDNUM_CHANGE_NAME"),
            holder_market_cap: opt_f64(item, "HOLDER_MARKET_CAP"),
            notice_date: opt_str(item, "NOTICE_DATE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_gdfx_free_holding_analyse_em — 数据中心-股东持股分析-十大流通股东
// ===========================================================================

/// One free-float holder analysis row, port of
/// `stock_gdfx_free_holding_analyse_em`
/// (Eastmoney `RPT_CUSTOM_F10_EH_FREEHOLDERS_JOIN_FREEHOLDER_SHAREANALYSIS`).
/// Field names are the **real** upstream keys.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GdfxFreeHoldingAnalyse {
    /// 股东名称 (`HOLDER_NAME`)
    pub holder_name: Option<String>,
    /// 股东类型 (`HOLDER_TYPE`)
    pub holder_type: Option<String>,
    /// 股票代码 (`SECURITY_CODE`)
    pub security_code: Option<String>,
    /// 股票简称 (`SECURITY_NAME_ABBR`)
    pub security_name: Option<String>,
    /// 报告期 (`END_DATE`)
    pub end_date: Option<String>,
    /// 期末持股-数量 (`HOLD_NUM`)
    pub hold_num: Option<f64>,
    /// 期末持股-数量变化 (`XZCHANGE`)
    pub xzchange: Option<f64>,
    /// 期末持股-数量变化比例 (`HOLD_RATIO_CHANGE`)
    pub hold_ratio_change: Option<f64>,
    /// 期末持股-持股变动 (`HOLDNUM_CHANGE_NAME`)
    pub holdnum_change_name: Option<String>,
    /// 期末持股-流通市值 (`HOLDER_MARKET_CAP`)
    pub holder_market_cap: Option<f64>,
    /// 公告日 (`UPDATE_DATE`)
    pub update_date: Option<String>,
    /// 公告日后涨跌幅-10个交易日 (`D10_ADJCHRATE`)
    pub d10_adjchrate: Option<f64>,
    /// 公告日后涨跌幅-30个交易日 (`D30_ADJCHRATE`)
    pub d30_adjchrate: Option<f64>,
    /// 公告日后涨跌幅-60个交易日 (`D60_ADJCHRATE`)
    pub d60_adjchrate: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_gdfx_free_holding_analyse_em(date)`.
pub async fn stock_gdfx_free_holding_analyse_em(
    client: &Client,
    date: &str,
) -> Result<Vec<GdfxFreeHoldingAnalyse>> {
    let d = fmt_date8(date)?;
    let filter = format!("(END_DATE='{d}')");
    let params = [
        (
            "reportName",
            "RPT_CUSTOM_F10_EH_FREEHOLDERS_JOIN_FREEHOLDER_SHAREANALYSIS",
        ),
        ("columns", "ALL;D10_ADJCHRATE,D30_ADJCHRATE,D60_ADJCHRATE"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("sortColumns", "UPDATE_DATE,SECURITY_CODE,HOLDER_RANK"),
        ("sortTypes", "-1,1,1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_gdfx_free_holding_analyse_em",
            DATACENTER,
            &params,
        )
        .await?;
    parse_free_holding_analyse(&v)
}

pub(crate) fn parse_free_holding_analyse(resp: &Value) -> Result<Vec<GdfxFreeHoldingAnalyse>> {
    let data = result_data(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(GdfxFreeHoldingAnalyse {
            holder_name: opt_str(item, "HOLDER_NAME"),
            holder_type: opt_str(item, "HOLDER_TYPE"),
            security_code: opt_str(item, "SECURITY_CODE"),
            security_name: opt_str(item, "SECURITY_NAME_ABBR"),
            end_date: opt_str(item, "END_DATE"),
            hold_num: opt_f64(item, "HOLD_NUM"),
            xzchange: opt_f64(item, "XZCHANGE"),
            hold_ratio_change: opt_f64(item, "HOLD_RATIO_CHANGE"),
            holdnum_change_name: opt_str(item, "HOLDNUM_CHANGE_NAME"),
            holder_market_cap: opt_f64(item, "HOLDER_MARKET_CAP"),
            update_date: opt_str(item, "UPDATE_DATE"),
            d10_adjchrate: opt_f64(item, "D10_ADJCHRATE"),
            d30_adjchrate: opt_f64(item, "D30_ADJCHRATE"),
            d60_adjchrate: opt_f64(item, "D60_ADJCHRATE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_gdfx_holding_analyse_em — 数据中心-股东持股分析-十大股东
// ===========================================================================

/// One top-10 holder analysis row, port of `stock_gdfx_holding_analyse_em`
/// (Eastmoney `RPT_CUSTOM_DMSK_HOLDERS_JOIN_HOLDER_SHAREANALYSIS`).
/// Field names are the **real** upstream keys.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GdfxHoldingAnalyse {
    /// 股东名称 (`HOLDER_NAME`)
    pub holder_name: Option<String>,
    /// 股东类型 (`HOLDER_TYPE_ORG`)
    pub holder_type_org: Option<String>,
    /// 股票代码 (`SECURITY_CODE`)
    pub security_code: Option<String>,
    /// 股票简称 (`SECURITY_NAME_ABBR`)
    pub security_name: Option<String>,
    /// 报告期 (`END_DATE`)
    pub end_date: Option<String>,
    /// 期末持股-数量 (`HOLD_NUM`)
    pub hold_num: Option<f64>,
    /// 期末持股-数量变化 (`HOLD_NUM_CHANGE`)
    pub hold_num_change: Option<f64>,
    /// 期末持股-数量变化比例 (`HOLD_RATIO_CHANGE`)
    pub hold_ratio_change: Option<f64>,
    /// 期末持股-持股变动 (`HOLDNUM_CHANGE_NAME`)
    pub holdnum_change_name: Option<String>,
    /// 期末持股-流通市值 (`HOLDER_MARKET_CAP`)
    pub holder_market_cap: Option<f64>,
    /// 公告日 (`NOTICE_DATE`)
    pub notice_date: Option<String>,
    /// 公告日后涨跌幅-10个交易日 (`D10_ADJCHRATE`)
    pub d10_adjchrate: Option<f64>,
    /// 公告日后涨跌幅-30个交易日 (`D30_ADJCHRATE`)
    pub d30_adjchrate: Option<f64>,
    /// 公告日后涨跌幅-60个交易日 (`D60_ADJCHRATE`)
    pub d60_adjchrate: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_gdfx_holding_analyse_em(date)`.
pub async fn stock_gdfx_holding_analyse_em(
    client: &Client,
    date: &str,
) -> Result<Vec<GdfxHoldingAnalyse>> {
    let d = fmt_date8(date)?;
    let filter = format!("(END_DATE='{d}')");
    let params = [
        (
            "reportName",
            "RPT_CUSTOM_DMSK_HOLDERS_JOIN_HOLDER_SHAREANALYSIS",
        ),
        ("columns", "ALL;D10_ADJCHRATE,D30_ADJCHRATE,D60_ADJCHRATE"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("sortColumns", "NOTICE_DATE,SECURITY_CODE,RANK"),
        ("sortTypes", "-1,1,1"),
        ("source", "WEB"),
        ("client", "WEB"),
        ("filter", filter.as_str()),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_gdfx_holding_analyse_em",
            DATACENTER,
            &params,
        )
        .await?;
    parse_holding_analyse(&v)
}

pub(crate) fn parse_holding_analyse(resp: &Value) -> Result<Vec<GdfxHoldingAnalyse>> {
    let data = result_data(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(GdfxHoldingAnalyse {
            holder_name: opt_str(item, "HOLDER_NAME"),
            holder_type_org: opt_str(item, "HOLDER_TYPE_ORG"),
            security_code: opt_str(item, "SECURITY_CODE"),
            security_name: opt_str(item, "SECURITY_NAME_ABBR"),
            end_date: opt_str(item, "END_DATE"),
            hold_num: opt_f64(item, "HOLD_NUM"),
            hold_num_change: opt_f64(item, "HOLD_NUM_CHANGE"),
            hold_ratio_change: opt_f64(item, "HOLD_RATIO_CHANGE"),
            holdnum_change_name: opt_str(item, "HOLDNUM_CHANGE_NAME"),
            holder_market_cap: opt_f64(item, "HOLDER_MARKET_CAP"),
            notice_date: opt_str(item, "NOTICE_DATE"),
            d10_adjchrate: opt_f64(item, "D10_ADJCHRATE"),
            d30_adjchrate: opt_f64(item, "D30_ADJCHRATE"),
            d60_adjchrate: opt_f64(item, "D60_ADJCHRATE"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_gdfx_free_holding_teamwork_em — 数据中心-股东协同-十大流通股东
// ===========================================================================

/// One free-float holder teamwork row, port of
/// `stock_gdfx_free_holding_teamwork_em` (Eastmoney `RPT_COOPFREEHOLDER`).
/// Field names inferred — see module note.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GdfxFreeHoldingTeamwork {
    /// 股东名称 (inferred `HOLDER_NAME`)
    pub holder_name: Option<String>,
    /// 股东类型 (inferred `HOLDER_TYPE`)
    pub holder_type: Option<String>,
    /// 协同股东名称 (inferred `COOP_HOLDER_NAME`)
    pub coop_holder_name: Option<String>,
    /// 协同股东类型 (inferred `COOP_HOLDER_TYPE`)
    pub coop_holder_type: Option<String>,
    /// 协同次数 (inferred `COOP_NUM`)
    pub coop_num: Option<f64>,
    /// 个股详情 (inferred `STOCK_DETAIL`)
    pub stock_detail: Option<String>,
    pub source: &'static str,
}

/// Port of `stock_gdfx_free_holding_teamwork_em(symbol)`.
///
/// `symbol` ∈ {"全部","个人","基金","QFII","社保","券商","信托"}; `"全部"` omits
/// the `filter` (mirrors akshare's empty `symbol_dict`).
pub async fn stock_gdfx_free_holding_teamwork_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<GdfxFreeHoldingTeamwork>> {
    let filter = if symbol != "全部" {
        Some(format!("(HOLDER_TYPE=\"{symbol}\")"))
    } else {
        None
    };
    let mut params: Vec<(&str, &str)> = vec![
        ("reportName", "RPT_COOPFREEHOLDER"),
        ("columns", "ALL"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("sortColumns", "COOPERAT_NUM,HOLDER_NEW,COOPERAT_HOLDER_NEW"),
        ("sortTypes", "-1,-1,-1"),
        ("source", "WEB"),
        ("client", "WEB"),
    ];
    if let Some(f) = &filter {
        params.push(("filter", f.as_str()));
    }
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_gdfx_free_holding_teamwork_em",
            DATACENTER,
            &params,
        )
        .await?;
    parse_free_holding_teamwork(&v)
}

pub(crate) fn parse_free_holding_teamwork(resp: &Value) -> Result<Vec<GdfxFreeHoldingTeamwork>> {
    let data = result_data(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(GdfxFreeHoldingTeamwork {
            holder_name: opt_str(item, "HOLDER_NAME"),
            holder_type: opt_str(item, "HOLDER_TYPE"),
            coop_holder_name: opt_str(item, "COOP_HOLDER_NAME"),
            coop_holder_type: opt_str(item, "COOP_HOLDER_TYPE"),
            coop_num: opt_f64(item, "COOP_NUM"),
            stock_detail: opt_str(item, "STOCK_DETAIL"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_gdfx_holding_teamwork_em — 数据中心-股东协同-十大股东
// ===========================================================================

/// One top-10 holder teamwork row, port of `stock_gdfx_holding_teamwork_em`
/// (Eastmoney `RPT_TENHOLDERS_COOPHOLDERS`). Field names inferred — see module note.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GdfxHoldingTeamwork {
    /// 股东名称 (inferred `HOLDER_NAME`)
    pub holder_name: Option<String>,
    /// 股东类型 (inferred `HOLDER_TYPE`)
    pub holder_type: Option<String>,
    /// 协同股东名称 (inferred `COOP_HOLDER_NAME`)
    pub coop_holder_name: Option<String>,
    /// 协同股东类型 (inferred `COOP_HOLDER_TYPE`)
    pub coop_holder_type: Option<String>,
    /// 协同次数 (inferred `COOP_NUM`)
    pub coop_num: Option<f64>,
    /// 个股详情 (inferred `STOCK_DETAIL`)
    pub stock_detail: Option<String>,
    pub source: &'static str,
}

/// Port of `stock_gdfx_holding_teamwork_em(symbol)`.
pub async fn stock_gdfx_holding_teamwork_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<GdfxHoldingTeamwork>> {
    let filter = if symbol != "全部" {
        Some(format!("(HOLDER_TYPE=\"{symbol}\")"))
    } else {
        None
    };
    let mut params: Vec<(&str, &str)> = vec![
        ("reportName", "RPT_TENHOLDERS_COOPHOLDERS"),
        ("columns", "ALL"),
        ("pageSize", "500"),
        ("pageNumber", "1"),
        ("sortColumns", "COOPERAT_NUM,HOLDER_NEW,COOPERAT_HOLDER_NEW"),
        ("sortTypes", "-1,-1,-1"),
        ("source", "WEB"),
        ("client", "WEB"),
    ];
    if let Some(f) = &filter {
        params.push(("filter", f.as_str()));
    }
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_gdfx_holding_teamwork_em",
            DATACENTER,
            &params,
        )
        .await?;
    parse_holding_teamwork(&v)
}

pub(crate) fn parse_holding_teamwork(resp: &Value) -> Result<Vec<GdfxHoldingTeamwork>> {
    let data = result_data(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(GdfxHoldingTeamwork {
            holder_name: opt_str(item, "HOLDER_NAME"),
            holder_type: opt_str(item, "HOLDER_TYPE"),
            coop_holder_name: opt_str(item, "COOP_HOLDER_NAME"),
            coop_holder_type: opt_str(item, "COOP_HOLDER_TYPE"),
            coop_num: opt_f64(item, "COOP_NUM"),
            stock_detail: opt_str(item, "STOCK_DETAIL"),
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

    fn fixture(name: &str) -> Value {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    #[test]
    fn parses_free_holding_statistics() {
        let rows =
            parse_free_holding_statistics(&fixture("stock_gdfx_free_holding_statistics_em.json"))
                .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].holder_name,
            Some("香港中央结算有限公司".to_string())
        );
        assert_eq!(rows[0].statistics_times, Some(12));
        assert_eq!(rows[0].d10_avg, Some(2.35));
        assert_eq!(rows[0].d60_min, Some(-1.5));
        assert_eq!(rows[1].holder_type, Some("基金".to_string()));
    }

    #[test]
    fn parses_holding_statistics() {
        let rows =
            parse_holding_statistics(&fixture("stock_gdfx_holding_statistics_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].holder_name,
            Some("中国证券金融股份有限公司".to_string())
        );
        assert_eq!(rows[0].d30_max, Some(5.1));
        assert_eq!(rows[1].statistics_times, Some(3));
    }

    #[test]
    fn parses_free_holding_change() {
        let rows =
            parse_free_holding_change(&fixture("stock_gdfx_free_holding_change_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].holder_name,
            Some("中央汇金资产管理有限责任公司".to_string())
        );
        assert_eq!(rows[0].hold_num_total, Some(100.0));
        assert_eq!(rows[0].hold_num_new, Some(10.0));
        assert_eq!(rows[0].market_cap, Some(5000.0));
        assert_eq!(rows[1].hold_num_decrease, Some(5.0));
    }

    #[test]
    fn parses_holding_change() {
        let rows = parse_holding_change(&fixture("stock_gdfx_holding_change_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].holder_name,
            Some("全国社保基金一零二组合".to_string())
        );
        assert_eq!(rows[0].hold_num_increase, Some(20.0));
        assert_eq!(rows[0].hold_num_unchanged, Some(8.0));
        assert_eq!(rows[1].hold_stocks, Some("600000,000001".to_string()));
    }

    #[test]
    fn parses_free_top_10() {
        let rows = parse_free_top_10(&fixture("stock_gdfx_free_top_10_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].rank, Some(1));
        assert_eq!(
            rows[0].holder_name,
            Some("香港中央结算有限公司".to_string())
        );
        assert_eq!(rows[0].holder_nature, Some("境外法人".to_string()));
        assert_eq!(rows[0].hold_num, Some(12345678.0));
        assert_eq!(rows[0].hold_ratio, Some(12.34));
        assert_eq!(rows[0].change_ratio, Some(0.5));
        assert_eq!(rows[1].hold_change, Some(-1000.0));
    }

    #[test]
    fn parses_top_10() {
        let rows = parse_top_10(&fixture("stock_gdfx_top_10_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].rank, Some(1));
        assert_eq!(
            rows[0].holder_name,
            Some("中国移动通信集团有限公司".to_string())
        );
        assert_eq!(rows[0].shares_type, Some("限售流通股".to_string()));
        assert_eq!(rows[0].hold_num, Some(99999999.0));
        assert_eq!(rows[0].hold_ratio, Some(45.6));
        assert_eq!(rows[1].change_ratio, Some(-0.2));
    }

    #[test]
    fn parses_free_holding_detail() {
        let rows =
            parse_free_holding_detail(&fixture("stock_gdfx_free_holding_detail_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].holder_name,
            Some("香港中央结算有限公司".to_string())
        );
        assert_eq!(rows[0].security_code, Some("600000".to_string()));
        assert_eq!(rows[0].security_name, Some("浦发银行".to_string()));
        assert_eq!(rows[0].end_date, Some("2021-09-30".to_string()));
        assert_eq!(rows[0].hold_num, Some(1000000.0));
        assert_eq!(rows[0].xzchange, Some(50000.0));
        assert_eq!(rows[0].change_ratio, Some(5.0));
        assert_eq!(rows[0].holder_market_cap, Some(12000000.0));
        assert_eq!(rows[0].update_date, Some("2021-10-30".to_string()));
        assert_eq!(rows[1].holdnum_change_name, Some("新进".to_string()));
    }

    #[test]
    fn parses_holding_detail() {
        let rows = parse_holding_detail(&fixture("stock_gdfx_holding_detail_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].holder_name,
            Some("全国社保基金一零二组合".to_string())
        );
        assert_eq!(rows[0].holder_newtype, Some("社保".to_string()));
        assert_eq!(rows[0].rank, Some(3));
        assert_eq!(rows[0].security_code, Some("000001".to_string()));
        assert_eq!(rows[0].end_date, Some("2023-03-31".to_string()));
        assert_eq!(rows[0].hold_num, Some(2000000.0));
        assert_eq!(rows[0].hold_num_change, Some(100000.0));
        assert_eq!(rows[0].hold_ratio_change, Some(5.0));
        assert_eq!(rows[0].notice_date, Some("2023-04-28".to_string()));
        assert_eq!(rows[1].holder_market_cap, Some(30000000.0));
    }

    #[test]
    fn parses_free_holding_analyse() {
        let rows = parse_free_holding_analyse(&fixture("stock_gdfx_free_holding_analyse_em.json"))
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].holder_name,
            Some("香港中央结算有限公司".to_string())
        );
        assert_eq!(rows[0].security_code, Some("600519".to_string()));
        assert_eq!(rows[0].hold_num, Some(8888888.0));
        assert_eq!(rows[0].d10_adjchrate, Some(3.2));
        assert_eq!(rows[0].d30_adjchrate, Some(5.4));
        assert_eq!(rows[0].d60_adjchrate, Some(-2.1));
        assert_eq!(rows[1].holder_market_cap, Some(1500000000.0));
    }

    #[test]
    fn parses_holding_analyse() {
        let rows = parse_holding_analyse(&fixture("stock_gdfx_holding_analyse_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].holder_name,
            Some("中国移动通信集团有限公司".to_string())
        );
        assert_eq!(rows[0].holder_type_org, Some("国有法人".to_string()));
        assert_eq!(rows[0].security_code, Some("600941".to_string()));
        assert_eq!(rows[0].hold_num_change, Some(0.0));
        assert_eq!(rows[0].d10_adjchrate, Some(1.1));
        assert_eq!(rows[0].d60_adjchrate, Some(4.4));
        assert_eq!(rows[1].notice_date, Some("2022-04-30".to_string()));
    }

    #[test]
    fn parses_free_holding_teamwork() {
        let rows =
            parse_free_holding_teamwork(&fixture("stock_gdfx_free_holding_teamwork_em.json"))
                .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].holder_name,
            Some("全国社保基金一零二组合".to_string())
        );
        assert_eq!(rows[0].holder_type, Some("社保".to_string()));
        assert_eq!(
            rows[0].coop_holder_name,
            Some("全国社保基金一零三组合".to_string())
        );
        assert_eq!(rows[0].coop_num, Some(6.0));
        assert_eq!(rows[1].coop_holder_type, Some("基金".to_string()));
    }

    #[test]
    fn parses_holding_teamwork() {
        let rows = parse_holding_teamwork(&fixture("stock_gdfx_holding_teamwork_em.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].holder_name,
            Some("中国证券金融股份有限公司".to_string())
        );
        assert_eq!(
            rows[0].coop_holder_name,
            Some("中央汇金资产管理有限责任公司".to_string())
        );
        assert_eq!(rows[0].coop_num, Some(9.0));
        assert_eq!(rows[1].stock_detail, Some("个股详情数据".to_string()));
    }
}
