//! Stock holder / company-profile endpoints ported from akshare.
//!
//! | Rust fn                  | akshare source                                  | endpoint                          |
//! |--------------------------|------------------------------------------------|-----------------------------------|
//! | `stock_individual_info_em` | `stock/stock_info_em.py`                      | push2.eastmoney.com (JSON)        |
//! | `stock_zygc_em`          | `stock_fundamental/stock_zygc.py`             | emweb.securities.eastmoney.com    |
//! | `stock_sector_spot`      | `stock/stock_industry.py`                     | vip/money.finance.sina.com.cn     |
//! | `stock_yjbb_em`          | `stock_feature/stock_yjbb_em.py`             | datacenter-web.eastmoney.com      |
//!
//! `stock_zh_a_circulate` (listed in the port request) does **not** exist in this
//! akshare checkout (no `def stock_zh_a_circulate` anywhere), so it is skipped.
//!
//! All four ports are pure-JSON HTTP (no JS signing, no HTML scraping, no
//! encryption), mirroring the crate's existing Eastmoney datacenter ports
//! (see `src/stock/fundamental/eastmoney.rs`, ADR-0005).

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY, SOURCE_SINA};
use crate::core::error::{Error, Result};
use crate::core::json::*;

// ---------------------------------------------------------------------------
// Shared helpers (mirror src/stock/fundamental/eastmoney.rs)
// ---------------------------------------------------------------------------


// ===========================================================================
// stock_individual_info_em — 东方财富-个股-股票信息
// ===========================================================================

/// Per-share info for one stock, port of `stock_individual_info_em`.
///
/// akshare returns a 2-column (item/value) frame; we surface the nine mapped
/// fields as a single-row struct. Source field ids are Eastmoney's `fNNN`
/// push2 keys (akshare `stock_info_em.py`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockIndividualInfoRow {
    /// `f57` 股票代码
    pub code: String,
    /// `f58` 股票简称
    pub name: String,
    /// `f84` 总股本
    pub total_shares: Option<f64>,
    /// `f85` 流通股
    pub float_shares: Option<f64>,
    /// `f127` 行业
    pub industry: String,
    /// `f116` 总市值
    pub total_mktcap: Option<f64>,
    /// `f117` 流通市值
    pub float_mktcap: Option<f64>,
    /// `f189` 上市时间
    pub list_date: String,
    /// `f43` 最新价
    pub latest_price: Option<f64>,
    pub source: &'static str,
}

/// Port of `stock_individual_info_em(symbol)`.
///
/// `symbol` is a bare A-share code (e.g. `"603777"`); `6`-prefixed codes map to
/// the Shanghai secid (`1.603777`), everything else to Shenzhen (`0.<code>`).
pub async fn stock_individual_info_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<StockIndividualInfoRow>> {
    let market = if symbol.starts_with('6') { 1 } else { 0 };
    let secid = format!("{market}.{symbol}");
    let params = [
        ("fltt", "2"),
        ("invt", "2"),
        ("fields", "f43,f57,f58,f84,f85,f116,f117,f127,f189"),
        ("secid", secid.as_str()),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_individual_info_em",
            "https://push2.eastmoney.com/api/qt/stock/get",
            &params,
        )
        .await?;
    parse_individual_info(&v)
}

pub(crate) fn parse_individual_info(resp: &Value) -> Result<Vec<StockIndividualInfoRow>> {
    let data = resp
        .get("data")
        .and_then(|d| d.as_object())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data object at stock_individual_info_em".into(),
        })?;
    let d = Value::Object(data.clone());
    let row = StockIndividualInfoRow {
        code: opt_str_or(&d, "f57", ""),
        name: opt_str_or(&d, "f58", ""),
        total_shares: opt_f64(&d, "f84"),
        float_shares: opt_f64(&d, "f85"),
        industry: opt_str_or(&d, "f127", ""),
        total_mktcap: opt_f64(&d, "f116"),
        float_mktcap: opt_f64(&d, "f117"),
        list_date: opt_str_or(&d, "f189", ""),
        latest_price: opt_f64(&d, "f43"),
        source: SOURCE_EASTMONEY,
    };
    Ok(vec![row])
}

// ===========================================================================
// stock_zygc_em — 东方财富-个股-主营构成 (business composition)
// ===========================================================================

/// One line of a company's revenue breakdown, port of `stock_zygc_em`.
///
/// Despite the port request labeling this "股东增减持", the akshare source
/// (`stock_fundamental/stock_zygc.py`) is actually 主营构成 (main-business
/// composition) from Eastmoney's `BusinessAnalysis/PageAjax` endpoint.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockZygcRow {
    /// `SECURITY_CODE` 股票代码
    pub security_code: String,
    /// `REPORT_DATE` 报告日期
    pub report_date: String,
    /// `MAINOP_TYPE` 分类类型, mapped 1→按行业分类 / 2→按产品分类 / 3→按地区分类
    pub mainop_type: String,
    /// `ITEM_NAME` 主营构成
    pub item_name: String,
    /// `MAIN_BUSINESS_INCOME` 主营收入
    pub main_business_income: Option<f64>,
    /// `MBI_RATIO` 收入比例
    pub mbi_ratio: Option<f64>,
    /// `MAIN_BUSINESS_COST` 主营成本
    pub main_business_cost: Option<f64>,
    /// `MBC_RATIO` 成本比例
    pub mbc_ratio: Option<f64>,
    /// `MAIN_BUSINESS_RPOFIT` 主营利润
    pub main_business_profit: Option<f64>,
    /// `MBR_RATIO` 利润比例
    pub mbr_ratio: Option<f64>,
    /// `GROSS_RPOFIT_RATIO` 毛利率
    pub gross_profit_ratio: Option<f64>,
    pub source: &'static str,
}

fn map_mainop_type(t: &str) -> String {
    match t {
        "1" => "按行业分类",
        "2" => "按产品分类",
        "3" => "按地区分类",
        other => other,
    }
    .to_string()
}

/// Port of `stock_zygc_em(symbol)`.
///
/// `symbol` is the Eastmoney code form, e.g. `"SH688041"`.
pub async fn stock_zygc_em(client: &Client, symbol: &str) -> Result<Vec<StockZygcRow>> {
    let params = [("code", symbol)];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "stock_zygc_em",
            "https://emweb.securities.eastmoney.com/PC_HSF10/BusinessAnalysis/PageAjax",
            &params,
        )
        .await?;
    parse_zygc(&v)
}

pub(crate) fn parse_zygc(resp: &Value) -> Result<Vec<StockZygcRow>> {
    let arr = resp
        .get("zygcfx")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing zygcfx array at stock_zygc_em".into(),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(StockZygcRow {
            security_code: opt_str_or(item, "SECURITY_CODE", ""),
            report_date: opt_str_or(item, "REPORT_DATE", ""),
            mainop_type: map_mainop_type(opt_str_or(item, "MAINOP_TYPE", "").as_str()),
            item_name: opt_str_or(item, "ITEM_NAME", ""),
            main_business_income: opt_f64(item, "MAIN_BUSINESS_INCOME"),
            mbi_ratio: opt_f64(item, "MBI_RATIO"),
            main_business_cost: opt_f64(item, "MAIN_BUSINESS_COST"),
            mbc_ratio: opt_f64(item, "MBC_RATIO"),
            main_business_profit: opt_f64(item, "MAIN_BUSINESS_RPOFIT"),
            mbr_ratio: opt_f64(item, "MBR_RATIO"),
            gross_profit_ratio: opt_f64(item, "GROSS_RPOFIT_RATIO"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ===========================================================================
// stock_sector_spot — 新浪行业-板块行情
// ===========================================================================

/// One sector/board quote row, port of `stock_sector_spot`.
///
/// Sina returns a JSON object whose values are comma-separated strings; each
/// becomes one row. Field ids follow akshare `stock_industry.py` column order.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockSectorSpotRow {
    /// `v0` 板块代码 / label (Sina internal id, e.g. `hy001`)
    pub label: String,
    /// `v1` 板块 (display name)
    pub board: String,
    /// `v2` 公司家数
    pub num_companies: Option<f64>,
    /// `v3` 平均价格
    pub avg_price: Option<f64>,
    /// `v4` 涨跌额
    pub change: Option<f64>,
    /// `v5` 涨跌幅
    pub change_pct: Option<f64>,
    /// `v6` 总成交量
    pub total_volume: Option<f64>,
    /// `v7` 总成交额
    pub total_amount: Option<f64>,
    /// `v8` 股票代码
    pub stock_code: String,
    /// `v9` 个股-涨跌幅
    pub stock_change_pct: Option<f64>,
    /// `v10` 个股-当前价
    pub stock_current_price: Option<f64>,
    /// `v11` 个股-涨跌额
    pub stock_change: Option<f64>,
    /// `v12` 股票名称
    pub stock_name: String,
    pub source: &'static str,
}

/// Resolve the Sina URL + query params for a sector `indicator`.
///
/// Returns `Error::InvalidParam` for `"启明星行业"` (its response is GB2312 and
/// this crate has no GB2312 decoder) and for unknown indicators.
pub(crate) fn sector_request(
    indicator: &str,
) -> Result<(&'static str, Vec<(&'static str, &'static str)>)> {
    match indicator {
        "新浪行业" => Ok((
            "http://vip.stock.finance.sina.com.cn/q/view/newSinaHy.php",
            vec![],
        )),
        "概念" => Ok((
            "http://money.finance.sina.com.cn/q/view/newFLJK.php",
            vec![("param", "class")],
        )),
        "地域" => Ok((
            "http://money.finance.sina.com.cn/q/view/newFLJK.php",
            vec![("param", "area")],
        )),
        "行业" => Ok((
            "http://money.finance.sina.com.cn/q/view/newFLJK.php",
            vec![("param", "industry")],
        )),
        "启明星行业" => Err(Error::InvalidParam(
            "stock_sector_spot: 启明星行业 requires GB2312 decoding, not supported".into(),
        )),
        other => Err(Error::InvalidParam(format!(
            "stock_sector_spot: unknown indicator {other}"
        ))),
    }
}

/// Port of `stock_sector_spot(indicator)`.
///
/// `indicator` ∈ {"新浪行业", "概念", "地域", "行业"}.
pub async fn stock_sector_spot(
    client: &Client,
    indicator: &str,
) -> Result<Vec<StockSectorSpotRow>> {
    let (url, params) = sector_request(indicator)?;
    let text = client
        .get_text(SOURCE_SINA, "stock_sector_spot", url, &params, None)
        .await?;
    let start = text.find('{').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "no JSON object found in sina sector response".into(),
    })?;
    let v: Value = serde_json::from_str(&text[start..]).map_err(Error::Json)?;
    parse_sector(&v)
}

pub(crate) fn parse_sector(resp: &Value) -> Result<Vec<StockSectorSpotRow>> {
    let obj = resp.as_object().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "sector response is not a JSON object".into(),
    })?;
    let mut out = Vec::with_capacity(obj.len());
    for (label, val) in obj {
        let s = val.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "sector value is not a string".into(),
        })?;
        let p: Vec<&str> = s.split(',').collect();
        if p.len() != 13 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_SINA,
                message: format!("sector row has {} fields, expected 13", p.len()),
            });
        }
        out.push(StockSectorSpotRow {
            label: label.clone(),
            board: p[1].to_string(),
            num_companies: p[2].parse::<f64>().ok(),
            avg_price: p[3].parse::<f64>().ok(),
            change: p[4].parse::<f64>().ok(),
            change_pct: p[5].parse::<f64>().ok(),
            total_volume: p[6].parse::<f64>().ok(),
            total_amount: p[7].parse::<f64>().ok(),
            stock_code: p[8].to_string(),
            stock_change_pct: p[9].parse::<f64>().ok(),
            stock_current_price: p[10].parse::<f64>().ok(),
            stock_change: p[11].parse::<f64>().ok(),
            stock_name: p[12].to_string(),
            source: SOURCE_SINA,
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
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let txt = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    #[test]
    fn parses_individual_info() {
        let v = fixture("stock_individual_info_em.json");
        let rows = parse_individual_info(&v).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].code, "603777");
        assert_eq!(rows[0].name, "来伊份");
        assert_eq!(rows[0].total_shares, Some(33627.24));
        assert_eq!(rows[0].float_shares, Some(33627.24));
        assert_eq!(rows[0].industry, "食品饮料");
        assert_eq!(rows[0].total_mktcap, Some(326775.1733));
        assert_eq!(rows[0].float_mktcap, Some(326775.1733));
        assert_eq!(rows[0].list_date, "2016-10-12");
        assert_eq!(rows[0].latest_price, Some(9.72));
        assert_eq!(rows[0].source, "eastmoney");
    }

    #[test]
    fn parses_zygc() {
        let v = fixture("stock_zygc_em.json");
        let rows = parse_zygc(&v).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].security_code, "688041");
        assert_eq!(rows[0].report_date, "2024-03-31");
        assert_eq!(rows[0].mainop_type, "按行业分类");
        assert_eq!(rows[0].item_name, "工业");
        assert_eq!(rows[0].main_business_income, Some(120.5));
        assert_eq!(rows[0].mbi_ratio, Some(0.8234));
        assert_eq!(rows[0].gross_profit_ratio, Some(0.4521));
        assert_eq!(rows[1].mainop_type, "按产品分类");
        assert_eq!(rows[2].mainop_type, "按地区分类");
    }

    #[test]
    fn parses_sector_spot() {
        let v = fixture("stock_sector_spot.json");
        let rows = parse_sector(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].label, "hy001");
        assert_eq!(rows[0].board, "酿酒");
        assert_eq!(rows[0].num_companies, Some(45.0));
        assert_eq!(rows[0].avg_price, Some(182.34));
        assert_eq!(rows[0].change_pct, Some(1.25));
        assert_eq!(rows[0].stock_code, "600519");
        assert_eq!(rows[0].stock_name, "贵州茅台");
        assert_eq!(rows[1].board, "银行");
        assert_eq!(rows[1].num_companies, Some(42.0));
    }

    #[test]
    fn sector_request_rejects_unsupported() {
        assert!(sector_request("启明星行业").is_err());
        assert!(sector_request("bogus").is_err());
        assert!(sector_request("新浪行业").is_ok());
    }
}
