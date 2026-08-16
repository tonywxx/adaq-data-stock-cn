use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

// ---------------------------------------------------------------------------
// bond_zh_cov — 东方财富网-数据中心-可转债数据 (datacenter `RPT_BOND_CB_LIST`)
// https://data.eastmoney.com/kzz/default.html
// ---------------------------------------------------------------------------

const COV_URL: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";
const COV_REPORT: &str = "RPT_BOND_CB_LIST";
/// Quote columns injected into every row (mirrors akshare's `quoteColumns`).
/// They surface as `CONVERT_STOCK_PRICE` / `TRANSFER_PRICE` / `TRANSFER_VALUE` /
/// `CURRENT_BOND_PRICE` / `TRANSFER_PREMIUM_RATIO` on each item.
const COV_QUOTE_COLUMNS: &str = "f2~01~CONVERT_STOCK_CODE~CONVERT_STOCK_PRICE,\
f235~10~SECURITY_CODE~TRANSFER_PRICE,f236~10~SECURITY_CODE~TRANSFER_VALUE,\
f2~10~SECURITY_CODE~CURRENT_BOND_PRICE,f237~10~SECURITY_CODE~TRANSFER_PREMIUM_RATIO,\
f239~10~SECURITY_CODE~RESALE_TRIG_PRICE,f240~10~SECURITY_CODE~REDEEM_TRIG_PRICE,\
f23~01~CONVERT_STOCK_CODE~PBV_RATIO";

/// Convertible-bond listing row (`bond_zh_cov`).
///
/// Field names follow the Eastmoney datacenter `RPT_BOND_CB_LIST` report.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondZhCov {
    /// 债券代码 — `SECURITY_CODE`
    pub security_code: String,
    /// 债券简称 — `SECURITY_NAME_ABBR`
    pub security_name: String,
    /// 申购日期 — `PUBLIC_START_DATE`
    pub public_start_date: Option<String>,
    /// 申购代码 — `CORRECODE`
    pub corre_code: Option<String>,
    /// 正股代码 — `CONVERT_STOCK_CODE`
    pub convert_stock_code: Option<String>,
    /// 正股简称 — `SECURITY_SHORT_NAME`
    pub convert_stock_name: Option<String>,
    /// 信用评级 — `RATING`
    pub rating: Option<String>,
    /// 发行规模(亿元) — `ACTUAL_ISSUE_SCALE`
    pub issue_scale: Option<f64>,
    /// 上市时间 — `LISTING_DATE`
    pub listing_date: Option<String>,
    /// 原股东配售-股权登记日 — `SECURITY_START_DATE`
    pub record_date: Option<String>,
    /// 原股东配售-每股配售额 — `FIRST_PER_PREPLACING`
    pub per_preplacing: Option<f64>,
    /// 正股价 — `CONVERT_STOCK_PRICE` (quote-injected)
    pub convert_stock_price: Option<f64>,
    /// 转股价 — `TRANSFER_PRICE` (quote-injected)
    pub transfer_price: Option<f64>,
    /// 转股价值 — `TRANSFER_VALUE` (quote-injected)
    pub transfer_value: Option<f64>,
    /// 债现价 — `CURRENT_BOND_PRICE` (quote-injected)
    pub current_bond_price: Option<f64>,
    /// 转股溢价率 — `TRANSFER_PREMIUM_RATIO` (quote-injected)
    pub transfer_premium_ratio: Option<f64>,
    pub source: &'static str,
}

impl BondZhCov {
    fn new(code: String, name: String) -> Self {
        Self {
            security_code: code,
            security_name: name,
            public_start_date: None,
            corre_code: None,
            convert_stock_code: None,
            convert_stock_name: None,
            rating: None,
            issue_scale: None,
            listing_date: None,
            record_date: None,
            per_preplacing: None,
            convert_stock_price: None,
            transfer_price: None,
            transfer_value: None,
            current_bond_price: None,
            transfer_premium_ratio: None,
            source: SOURCE_EASTMONEY,
        }
    }
}

/// Convertible-bond listing data from Eastmoney (`bond_zh_cov`).
///
/// Walks datacenter `result.pages`, accumulating `result.data`.
pub async fn bond_zh_cov(client: &Client) -> Result<Vec<BondZhCov>> {
    let mut out = Vec::new();
    let mut page: u32 = 1;
    loop {
        let page_s = page.to_string();
        let params = [
            ("sortColumns", "PUBLIC_START_DATE"),
            ("sortTypes", "-1"),
            ("pageSize", "500"),
            ("pageNumber", page_s.as_str()),
            ("reportName", COV_REPORT),
            ("columns", "ALL"),
            ("quoteColumns", COV_QUOTE_COLUMNS),
            ("source", "WEB"),
            ("client", "WEB"),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, "bond_zh_cov", COV_URL, &params)
            .await?;
        let page_rows = parse_bond_zh_cov(&v)?;
        if page_rows.is_empty() {
            break;
        }
        out.extend(page_rows);
        let total_pages = v
            .get("result")
            .and_then(|r| r.get("pages"))
            .and_then(|p| p.as_u64())
            .unwrap_or(1);
        if page >= total_pages as u32 {
            break;
        }
        page += 1;
    }
    Ok(out)
}

pub(crate) fn parse_bond_zh_cov(resp: &Value) -> Result<Vec<BondZhCov>> {
    let data = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data".into(),
        })?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        if let Some(row) = parse_cov_item(item) {
            out.push(row);
        }
    }
    Ok(out)
}

fn parse_cov_item(item: &Value) -> Option<BondZhCov> {
    let code = fstr(item, "SECURITY_CODE");
    let name = fstr(item, "SECURITY_NAME_ABBR");
    if code.is_empty() && name.is_empty() {
        return None;
    }
    let mut row = BondZhCov::new(code, name);
    row.public_start_date = fstr_opt(item, "PUBLIC_START_DATE");
    row.corre_code = fstr_opt(item, "CORRECODE");
    row.convert_stock_code = fstr_opt(item, "CONVERT_STOCK_CODE");
    row.convert_stock_name = fstr_opt(item, "SECURITY_SHORT_NAME");
    row.rating = fstr_opt(item, "RATING");
    row.issue_scale = fnum(item, "ACTUAL_ISSUE_SCALE");
    row.listing_date = fstr_opt(item, "LISTING_DATE");
    row.record_date = fstr_opt(item, "SECURITY_START_DATE");
    row.per_preplacing = fnum(item, "FIRST_PER_PREPLACING");
    row.convert_stock_price = fnum(item, "CONVERT_STOCK_PRICE");
    row.transfer_price = fnum(item, "TRANSFER_PRICE");
    row.transfer_value = fnum(item, "TRANSFER_VALUE");
    row.current_bond_price = fnum(item, "CURRENT_BOND_PRICE");
    row.transfer_premium_ratio = fnum(item, "TRANSFER_PREMIUM_RATIO");
    Some(row)
}

// ---------------------------------------------------------------------------
// bond_zh_cov_value_analysis — 东方财富网-可转债价值分析 (datacenter `RPTA_WEB_KZZ_LS`)
// https://data.eastmoney.com/kzz/detail/<symbol>.html
// ---------------------------------------------------------------------------

const COV_VALUE_URL: &str = "https://datacenter-web.eastmoney.com/api/data/get";
const COV_VALUE_TYPE: &str = "RPTA_WEB_KZZ_LS";
const COV_VALUE_TOKEN: &str = "894050c76af8597a853f5b408b759f5d";

/// Convertible-bond value-analysis row (`bond_zh_cov_value_analysis`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondZhCovValueAnalysis {
    /// 转债代码 — `ZCODE`
    pub zcode: String,
    /// 日期 — `DATE`
    pub date: Option<String>,
    /// 收盘价 — `FCLOSE`
    pub close: Option<f64>,
    /// 纯债价值 — `PUREBONDVALUE`
    pub pure_bond_value: Option<f64>,
    /// 转股价值 — `SWAPVALUE`
    pub swap_value: Option<f64>,
    /// 纯债溢价率 — `PUREBONDOR`
    pub pure_bond_premium_ratio: Option<f64>,
    /// 转股溢价率 — `SWAPOR`
    pub swap_premium_ratio: Option<f64>,
    pub source: &'static str,
}

impl BondZhCovValueAnalysis {
    fn new(zcode: String) -> Self {
        Self {
            zcode,
            date: None,
            close: None,
            pure_bond_value: None,
            swap_value: None,
            pure_bond_premium_ratio: None,
            swap_premium_ratio: None,
            source: SOURCE_EASTMONEY,
        }
    }
}

/// Convertible-bond value analysis from Eastmoney (`bond_zh_cov_value_analysis`).
///
/// `symbol` is the convertible-bond code, e.g. `"113527"`.
pub async fn bond_zh_cov_value_analysis(
    client: &Client,
    symbol: &str,
) -> Result<Vec<BondZhCovValueAnalysis>> {
    if symbol.is_empty() {
        return Err(Error::InvalidParam("symbol must not be empty".into()));
    }
    let filter = format!("(zcode=\"{symbol}\")");
    let params = [
        ("sty", "ALL"),
        ("token", COV_VALUE_TOKEN),
        ("st", "date"),
        ("sr", "1"),
        ("source", "WEB"),
        ("type", COV_VALUE_TYPE),
        ("filter", filter.as_str()),
        ("p", "1"),
        ("ps", "8000"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "bond_zh_cov_value_analysis",
            COV_VALUE_URL,
            &params,
        )
        .await?;
    parse_bond_zh_cov_value_analysis(&v)
}

pub(crate) fn parse_bond_zh_cov_value_analysis(
    resp: &Value,
) -> Result<Vec<BondZhCovValueAnalysis>> {
    let data = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data".into(),
        })?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let zcode = fstr(item, "ZCODE");
        let mut row = BondZhCovValueAnalysis::new(zcode);
        row.date = fstr_opt(item, "DATE");
        row.close = fnum(item, "FCLOSE");
        row.pure_bond_value = fnum(item, "PUREBONDVALUE");
        row.swap_value = fnum(item, "SWAPVALUE");
        row.pure_bond_premium_ratio = fnum(item, "PUREBONDOR");
        row.swap_premium_ratio = fnum(item, "SWAPOR");
        out.push(row);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// bond_sh_buy_back_em / bond_sz_buy_back_em — 东方财富网 质押式回购
// https://quote.eastmoney.com/center/gridlist.html#bond_sh_buyback
// https://quote.eastmoney.com/center/gridlist.html#bond_sz_buyback
// ---------------------------------------------------------------------------

const BUYBACK_URL: &str = "https://push2.eastmoney.com/api/qt/clist/get";
const BUYBACK_FIELDS: &str = "f12,f13,f14,f1,f2,f4,f3,f152,f17,f18,f15,f16,f5,f6";

/// Repo (pledged-style buy-back) quote row (`bond_sh_buy_back_em` / `bond_sz_buy_back_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondBuyBackEm {
    /// 序号 — 1-based row position in the returned list
    pub index: u32,
    /// 代码 — `f12`
    pub code: String,
    /// 名称 — `f14`
    pub name: String,
    /// 最新价 — `f2` (÷1000)
    pub latest_price: Option<f64>,
    /// 涨跌额 — `f4` (÷1000)
    pub change: Option<f64>,
    /// 涨跌幅 — `f3` (÷100)
    pub pct_change: Option<f64>,
    /// 今开 — `f17` (÷1000)
    pub open: Option<f64>,
    /// 最高 — `f15` (÷1000)
    pub high: Option<f64>,
    /// 最低 — `f16` (÷1000)
    pub low: Option<f64>,
    /// 昨收 — `f18` (÷1000)
    pub prev_close: Option<f64>,
    /// 成交量 — `f5`
    pub volume: Option<f64>,
    /// 成交额 — `f6`
    pub amount: Option<f64>,
    pub source: &'static str,
}

/// 上证质押式回购 from Eastmoney (`bond_sh_buy_back_em`).
pub async fn bond_sh_buy_back_em(client: &Client) -> Result<Vec<BondBuyBackEm>> {
    fetch_buy_back(client, "m:1+b:MK0356").await
}

/// 深证质押式回购 from Eastmoney (`bond_sz_buy_back_em`).
pub async fn bond_sz_buy_back_em(client: &Client) -> Result<Vec<BondBuyBackEm>> {
    fetch_buy_back(client, "m:0+b:MK0356").await
}

async fn fetch_buy_back(client: &Client, fs: &str) -> Result<Vec<BondBuyBackEm>> {
    let params = [
        ("np", "1"),
        ("fltt", "1"),
        ("invt", "2"),
        ("fs", fs),
        ("fields", BUYBACK_FIELDS),
        ("fid", "f6"),
        ("pn", "1"),
        ("pz", "100"),
        ("po", "1"),
        ("dect", "1"),
        ("wbp2u", "|0|0|0|web"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "bond_buy_back_em", BUYBACK_URL, &params)
        .await?;
    parse_bond_buy_back_em(&v)
}

pub(crate) fn parse_bond_buy_back_em(resp: &Value) -> Result<Vec<BondBuyBackEm>> {
    let diff = resp
        .get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff".into(),
        })?;
    let mut out = Vec::with_capacity(diff.len());
    for (i, item) in diff.iter().enumerate() {
        let mut row = BondBuyBackEm {
            index: (i + 1) as u32,
            code: fstr(item, "f12"),
            name: fstr(item, "f14"),
            latest_price: fnum(item, "f2").map(|v| v / 1000.0),
            change: fnum(item, "f4").map(|v| v / 1000.0),
            pct_change: fnum(item, "f3").map(|v| v / 100.0),
            open: fnum(item, "f17").map(|v| v / 1000.0),
            high: fnum(item, "f15").map(|v| v / 1000.0),
            low: fnum(item, "f16").map(|v| v / 1000.0),
            prev_close: fnum(item, "f18").map(|v| v / 1000.0),
            volume: fnum(item, "f5"),
            amount: fnum(item, "f6"),
            source: SOURCE_EASTMONEY,
        };
        if row.code.is_empty() {
            row.code = fstr(item, "f13");
        }
        out.push(row);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn fstr(item: &Value, k: &str) -> String {
    item.get(k)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn fstr_opt(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let txt = std::fs::read_to_string(p).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    #[test]
    fn parses_bond_zh_cov_fixture() {
        let v = fixture("bond_zh_cov.json");
        let rows = parse_bond_zh_cov(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].security_code, "123283");
        assert_eq!(rows[0].security_name, "丰茂转债");
        assert_eq!(
            rows[0].public_start_date.as_deref(),
            Some("2026-08-18 00:00:00")
        );
        assert_eq!(rows[0].corre_code.as_deref(), Some("371459"));
        assert_eq!(rows[0].convert_stock_code.as_deref(), Some("301459"));
        assert_eq!(rows[0].convert_stock_name.as_deref(), Some("丰茂股份"));
        assert_eq!(rows[0].rating.as_deref(), Some("AA-"));
        assert_eq!(rows[0].issue_scale, Some(6.075298));
        assert_eq!(rows[0].record_date.as_deref(), Some("2026-08-17 00:00:00"));
        assert_eq!(rows[0].per_preplacing, Some(5.8347));
        // quote-injected fields present in the fixture
        assert_eq!(rows[0].transfer_price, Some(35.59));
        assert_eq!(rows[0].current_bond_price, Some(100.0));
        assert_eq!(rows[1].security_code, "113527");
        assert_eq!(rows[1].source, "eastmoney");
    }

    #[test]
    fn parses_bond_zh_cov_value_analysis_fixture() {
        let v = fixture("bond_zh_cov_value_analysis.json");
        let rows = parse_bond_zh_cov_value_analysis(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].zcode, "113527");
        assert_eq!(rows[0].date.as_deref(), Some("2019-01-24 00:00:00"));
        assert_eq!(rows[0].close, Some(98.50));
        assert_eq!(rows[0].pure_bond_value, Some(89.7223766891));
        assert_eq!(rows[0].swap_value, Some(98.1951871658));
        assert_eq!(rows[0].pure_bond_premium_ratio, Some(11.45491648));
        assert_eq!(rows[0].swap_premium_ratio, Some(1.8379850238));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].zcode, "113527");
    }

    #[test]
    fn parses_bond_sh_buy_back_em_fixture() {
        let v = fixture("bond_sh_buy_back_em.json");
        let rows = parse_bond_buy_back_em(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].index, 1);
        assert_eq!(rows[0].code, "204001");
        assert_eq!(rows[0].name, "GC001");
        assert_eq!(rows[0].latest_price, Some(1.915));
        assert_eq!(rows[0].pct_change, Some(-1.03));
        assert_eq!(rows[0].open, Some(1.920));
        assert_eq!(rows[0].high, Some(1.930));
        assert_eq!(rows[0].low, Some(1.910));
        assert_eq!(rows[0].prev_close, Some(1.935));
        assert_eq!(rows[0].volume, Some(123456789.0));
        assert_eq!(rows[0].amount, Some(987654321.0));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].index, 2);
        assert_eq!(rows[1].code, "204002");
    }

    #[test]
    fn parses_bond_sz_buy_back_em_fixture() {
        let v = fixture("bond_sz_buy_back_em.json");
        let rows = parse_bond_buy_back_em(&v).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].index, 1);
        assert_eq!(rows[0].code, "131810");
        assert_eq!(rows[0].name, "R-001");
        assert_eq!(rows[0].latest_price, Some(1.720));
        assert_eq!(rows[0].pct_change, Some(0.58));
        assert_eq!(rows[0].source, "eastmoney");
    }
}
