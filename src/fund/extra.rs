//! Extra fund endpoints (akshare `fund` package) ported to pure-HTTP Rust.
//!
//! This module ports a batch of akshare fund functions that are reachable with
//! ordinary HTTP (JSON, lenient-JSON, or JSONP) and require no JS evaluation or
//! encryption. Each function returns `Result<Vec<SomeRow>>` and is paired with a
//! `pub(crate) fn parse_*` used by the offline tests.
//!
//! Provenance / mapping for this akshare checkout (`/Users/tony/github/akshare`):
//!
//! | Rust fn                 | akshare source                          | upstream shape            |
//! |-------------------------|-----------------------------------------|---------------------------|
//! | `fund_open_fund_name_em`| `fund/fund_em.py::fund_name_em`         | JS file `var r = [...]`   |
//! | `fund_value_em`         | `fund/fund_em.py::fund_value_estimation_em` (API path) | JSON (`GetFundGZList`) |
//! | `fund_hist_em`          | `fund/fund_em.py` `f10/lsjz` history    | JSON (`LSJZList`)         |
//! | `fund_money_meta`       | `fund/fund_em.py::fund_money_fund_info_em` | JSON (`LSJZList`)     |
//! | `fund_etf_category_sina`| `fund/fund_etf_sina.py::fund_etf_category_sina` | JSONP (`getHQNodeDataSimple`) |
//!
//! SKIPPED: `fund_name` (task's `fund/fund_name.py`) — that file does not exist in
//! this akshare version; its all-funds code/name/type list is exposed here as
//! `fund_open_fund_name_em` (akshare `fund_name_em`).
//!
//! NOTE on `fund_value_em`: akshare's `指数型` branch scrapes static HTML pages
//! (`lof_fundguzhi*.html`); that HTML path is intentionally not ported (it needs
//! `py_mini_racer`/BeautifulSoup). The `GetFundGZList` JSON API path is used for
//! all other types. The Eastmoney GZ list field keys (`FCODE`, `GSZ`, `GZZZL`,
//! `DWJZ`, `JZZZL`, `GZMS`, …) are inferred from the public API and should be
//! validated against a live sample before production use.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY, SOURCE_SINA};
use crate::core::error::{Error, Result};
use crate::fund::{fnum, fstr};

// ---------------------------------------------------------------------------
// `fund_open_fund_name_em` — all-fund code/name/type list (Eastmoney)
// ---------------------------------------------------------------------------

const FUND_NAME_URL: &str = "https://fund.eastmoney.com/js/fundcode_search.js";

/// One fund's code / pinyin / name / type, from Eastmoney's `fundcode_search.js`.
///
/// akshare columns: 基金代码, 拼音缩写, 基金简称, 基金类型, 拼音全称.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundOpenFundNameRow {
    /// akshare 基金代码
    pub fund_code: String,
    /// akshare 拼音缩写
    pub pinyin_abbr: String,
    /// akshare 基金简称
    pub short_name: String,
    /// akshare 基金类型
    pub fund_type: String,
    /// akshare 拼音全称
    pub pinyin_full: String,
}

/// All-fund code/name/type list (akshare `fund_name_em`).
///
/// Fetches the Eastmoney `fundcode_search.js` file, which is a JS assignment
/// `var r = [[...],...];`. We strip the `var r = ` prefix and trailing `;`, then
/// parse the JSON array — no JS evaluation required (ADR-0005).
pub async fn fund_open_fund_name_em(client: &Client) -> Result<Vec<FundOpenFundNameRow>> {
    let text = client
        .get_text(
            SOURCE_EASTMONEY,
            "fund_open_fund_name_em",
            FUND_NAME_URL,
            &[],
            None,
        )
        .await?;
    let body = text.strip_prefix("var r = ").unwrap_or(&text);
    let body = body.strip_suffix(';').unwrap_or(body).trim();
    let v: Value = serde_json::from_str(body).map_err(Error::Json)?;
    parse_fund_open_fund_name_em(&v)
}

pub(crate) fn parse_fund_open_fund_name_em(resp: &Value) -> Result<Vec<FundOpenFundNameRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "fund code search payload is not a JSON array".into(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for row in arr {
        let cells = match row.as_array() {
            Some(c) if c.len() >= 5 => c,
            _ => continue, // skip malformed rows
        };
        out.push(FundOpenFundNameRow {
            fund_code: cell_str(cells, 0),
            pinyin_abbr: cell_str(cells, 1),
            short_name: cell_str(cells, 2),
            fund_type: cell_str(cells, 3),
            pinyin_full: cell_str(cells, 4),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// `fund_value_em` — fund NAV estimation (Eastmoney `GetFundGZList`)
// ---------------------------------------------------------------------------

const FUND_VALUE_URL: &str = "https://api.fund.eastmoney.com/FundGuZhi/GetFundGZList";

const FUND_VALUE_TYPE_MAP: &[(&str, &str)] = &[
    ("全部", "1"),
    ("股票型", "2"),
    ("混合型", "3"),
    ("债券型", "4"),
    ("指数型", "5"),
    ("QDII", "6"),
    ("ETF联接", "7"),
    ("LOF", "8"),
    ("场内交易基金", "9"),
];

/// One fund's NAV estimation snapshot (akshare `fund_value_estimation_em`).
///
/// akshare columns: 基金代码, 基金名称, {估算日期}-估算数据-估算值,
/// {估算日期}-估算数据-估算增长率, {估算日期}-公布数据-单位净值,
/// {估算日期}-公布数据-日增长率, 估算偏差.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundValueRow {
    /// akshare 基金代码
    pub fund_code: String,
    /// akshare 基金名称
    pub fund_name: String,
    /// akshare 估算日期 (Data.gzrq)
    pub estimate_date: String,
    /// akshare {估算日期}-估算数据-估算值 (GSZ)
    pub gz_value: Option<f64>,
    /// akshare {估算日期}-估算数据-估算增长率 (GZZZL)
    pub gz_growth_rate: Option<f64>,
    /// akshare {估算日期}-公布数据-单位净值 (DWJZ)
    pub published_nav: Option<f64>,
    /// akshare {估算日期}-公布数据-日增长率 (JZZZL)
    pub published_growth_rate: Option<f64>,
    /// akshare 估算偏差 (GZMS)
    pub estimate_deviation: Option<f64>,
}

/// Fund NAV estimation by type (akshare `fund_value_estimation_em`, API path).
///
/// `symbol` is one of `FUND_VALUE_TYPE_MAP` keys. NOTE: the `指数型` HTML static
/// page path from akshare is not ported; the `GetFundGZList` API is used for all
/// types (the index type may be incomplete upstream via this API).
pub async fn fund_value_em(client: &Client, symbol: &str) -> Result<Vec<FundValueRow>> {
    let ty = type_map(symbol)?;
    let page_index = "1".to_string();
    let page_size = "20000".to_string();
    let ts = "0".to_string();
    let params = [
        ("type", ty),
        ("sort", "3"),
        ("orderType", "desc"),
        ("canbuy", "0"),
        ("pageIndex", page_index.as_str()),
        ("pageSize", page_size.as_str()),
        ("_", ts.as_str()),
    ];
    let text = client
        .get_text(
            SOURCE_EASTMONEY,
            "fund_value_em",
            FUND_VALUE_URL,
            &params,
            Some(&[("Referer", "https://fund.eastmoney.com/")]),
        )
        .await?;
    let v: Value = serde_json::from_str(&text).map_err(Error::Json)?;
    parse_fund_value_em(&v)
}

pub(crate) fn parse_fund_value_em(resp: &Value) -> Result<Vec<FundValueRow>> {
    let data = resp.get("Data").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "fund_value_em: missing Data".into(),
    })?;
    let estimate_date = fstr(data, "gzrq");
    let list =
        data.get("list")
            .and_then(|l| l.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "fund_value_em: missing Data.list".into(),
            })?;
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        out.push(FundValueRow {
            fund_code: fstr(item, "FCODE"),
            fund_name: fstr(item, "SHORTNAME"),
            estimate_date: estimate_date.clone(),
            gz_value: fnum(item, "GSZ"),
            gz_growth_rate: fnum(item, "GZZZL"),
            published_nav: fnum(item, "DWJZ"),
            published_growth_rate: fnum(item, "JZZZL"),
            estimate_deviation: fnum(item, "GZMS"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// `fund_hist_em` — open/ETF/LOF fund NAV history (Eastmoney `f10/lsjz`)
// ---------------------------------------------------------------------------

const FUND_HIST_URL: &str = "https://api.fund.eastmoney.com/f10/lsjz";

/// One fund's NAV history row (akshare `f10/lsjz` `LSJZList`).
///
/// akshare columns: 净值日期, 单位净值, 累计净值, 日增长率, 申购状态, 赎回状态.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundHistRow {
    /// fund code echoed back (the `fundCode` requested)
    pub fund_code: String,
    /// akshare 净值日期 (FSRQ)
    pub nav_date: String,
    /// akshare 单位净值 (DWJZ)
    pub unit_nav: Option<f64>,
    /// akshare 累计净值 (LJJZ)
    pub acc_nav: Option<f64>,
    /// akshare 日增长率 (JZZZL)
    pub daily_growth_rate: Option<f64>,
    /// akshare 申购状态 (SGZT)
    pub purchase_status: String,
    /// akshare 赎回状态 (SHZT)
    pub redeem_status: String,
}

/// Fund NAV history (akshare `fund_hist_em` / `f10/lsjz`).
///
/// `start_date` / `end_date` are `YYYYMMDD` (akshare convention). Requests the
/// full `LSJZList` in one page (`pageSize=10000`); the upstream `lsjz` API is the
/// same mechanism akshare uses for `fund_open_fund_info_em` / `fund_etf_fund_info_em`.
pub async fn fund_hist_em(
    client: &Client,
    fund_code: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<FundHistRow>> {
    if start_date.len() != 8 || end_date.len() != 8 {
        return Err(Error::InvalidParam(
            "start_date and end_date must be YYYYMMDD".into(),
        ));
    }
    let sd = format!(
        "{}-{}-{}",
        &start_date[0..4],
        &start_date[4..6],
        &start_date[6..8]
    );
    let ed = format!(
        "{}-{}-{}",
        &end_date[0..4],
        &end_date[4..6],
        &end_date[6..8]
    );
    let page_index = "1".to_string();
    let page_size = "10000".to_string();
    let ts = "0".to_string();
    let params = [
        ("fundCode", fund_code),
        ("pageIndex", page_index.as_str()),
        ("pageSize", page_size.as_str()),
        ("startDate", sd.as_str()),
        ("endDate", ed.as_str()),
        ("_", ts.as_str()),
    ];
    let referer = format!("https://fundf10.eastmoney.com/jjjz_{fund_code}.html");
    let text = client
        .get_text(
            SOURCE_EASTMONEY,
            "fund_hist_em",
            FUND_HIST_URL,
            &params,
            Some(&[("Referer", referer.as_str())]),
        )
        .await?;
    let v: Value = serde_json::from_str(&text).map_err(Error::Json)?;
    parse_fund_hist_em(&v, fund_code)
}

pub(crate) fn parse_fund_hist_em(resp: &Value, fund_code: &str) -> Result<Vec<FundHistRow>> {
    let data = resp.get("Data").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "fund_hist_em: missing Data".into(),
    })?;
    let list = data
        .get("LSJZList")
        .and_then(|l| l.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "fund_hist_em: missing Data.LSJZList".into(),
        })?;
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        out.push(FundHistRow {
            fund_code: fund_code.to_string(),
            nav_date: fstr(item, "FSRQ"),
            unit_nav: fnum(item, "DWJZ"),
            acc_nav: fnum(item, "LJJZ"),
            daily_growth_rate: fnum(item, "JZZZL"),
            purchase_status: fstr(item, "SGZT"),
            redeem_status: fstr(item, "SHZT"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// `fund_money_meta` — money-market fund NAV (Eastmoney `f10/lsjz`)
// ---------------------------------------------------------------------------

/// One money-market fund's NAV row (akshare `fund_money_fund_info_em`).
///
/// akshare columns: 净值日期, 每万份收益, 7日年化收益率, 申购状态, 赎回状态.
/// For money funds Eastmoney stores 每万份收益 in `DWJZ` and 7日年化收益率 in `LJJZ`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundMoneyMetaRow {
    /// fund code echoed back (the `fundCode` requested)
    pub fund_code: String,
    /// akshare 净值日期 (FSRQ)
    pub nav_date: String,
    /// akshare 每万份收益 (DWJZ)
    pub million_income: Option<f64>,
    /// akshare 7日年化收益率 (LJJZ)
    pub annualized_7day: Option<f64>,
    /// akshare 申购状态 (SGZT)
    pub purchase_status: String,
    /// akshare 赎回状态 (SHZT)
    pub redeem_status: String,
}

/// Money-market fund NAV history/metadata (akshare `fund_money_meta`, backed by
/// `fund_money_fund_info_em` / `f10/lsjz`).
///
/// `start_date` / `end_date` are `YYYYMMDD`. This akshare version has no
/// `fund/fund_money.py`; the closest money-fund endpoint (`fund_money_fund_info_em`)
/// is reused here. The upstream `lsjz` API is identical to [`fund_hist_em`].
pub async fn fund_money_meta(
    client: &Client,
    fund_code: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<FundMoneyMetaRow>> {
    if start_date.len() != 8 || end_date.len() != 8 {
        return Err(Error::InvalidParam(
            "start_date and end_date must be YYYYMMDD".into(),
        ));
    }
    let sd = format!(
        "{}-{}-{}",
        &start_date[0..4],
        &start_date[4..6],
        &start_date[6..8]
    );
    let ed = format!(
        "{}-{}-{}",
        &end_date[0..4],
        &end_date[4..6],
        &end_date[6..8]
    );
    let page_index = "1".to_string();
    let page_size = "10000".to_string();
    let ts = "0".to_string();
    let params = [
        ("fundCode", fund_code),
        ("pageIndex", page_index.as_str()),
        ("pageSize", page_size.as_str()),
        ("startDate", sd.as_str()),
        ("endDate", ed.as_str()),
        ("_", ts.as_str()),
    ];
    let referer = format!("https://fundf10.eastmoney.com/jjjz_{fund_code}.html");
    let text = client
        .get_text(
            SOURCE_EASTMONEY,
            "fund_money_meta",
            FUND_HIST_URL,
            &params,
            Some(&[("Referer", referer.as_str())]),
        )
        .await?;
    let v: Value = serde_json::from_str(&text).map_err(Error::Json)?;
    parse_fund_money_meta(&v, fund_code)
}

pub(crate) fn parse_fund_money_meta(
    resp: &Value,
    fund_code: &str,
) -> Result<Vec<FundMoneyMetaRow>> {
    let data = resp.get("Data").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "fund_money_meta: missing Data".into(),
    })?;
    let list = data
        .get("LSJZList")
        .and_then(|l| l.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "fund_money_meta: missing Data.LSJZList".into(),
        })?;
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        out.push(FundMoneyMetaRow {
            fund_code: fund_code.to_string(),
            nav_date: fstr(item, "FSRQ"),
            million_income: fnum(item, "DWJZ"),
            annualized_7day: fnum(item, "LJJZ"),
            purchase_status: fstr(item, "SGZT"),
            redeem_status: fstr(item, "SHZT"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// `fund_etf_category_sina` — Sina ETF/LOF/closed-end fund category list
// ---------------------------------------------------------------------------

const SINA_HQ_URL: &str = "https://vip.stock.finance.sina.com.cn/quotes_service/api/jsonp.php/IO.XSRV2.CallbackList['da_yPT46_Ll7K6WD']/Market_Center.getHQNodeDataSimple";

/// One Sina fund quote in a category (akshare `fund_etf_category_sina`).
///
/// akshare columns: 代码, 名称, 最新价, 涨跌额, 涨跌幅, 买入, 卖出, 昨收, 今开,
/// 最高, 最低, 成交量, 成交额. `category` is the requested symbol.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundEtfCategorySinaRow {
    /// akshare 代码 (symbol)
    pub code: String,
    /// akshare 名称 (name)
    pub name: String,
    /// akshare 最新价 (trade)
    pub price: Option<f64>,
    /// akshare 涨跌额 (pricechange)
    pub price_change: Option<f64>,
    /// akshare 涨跌幅 (changepercent)
    pub pct_change: Option<f64>,
    /// akshare 买入 (buy)
    pub buy: Option<f64>,
    /// akshare 卖出 (sell)
    pub sell: Option<f64>,
    /// akshare 昨收 (settlement)
    pub pre_close: Option<f64>,
    /// akshare 今开 (open)
    pub open: Option<f64>,
    /// akshare 最高 (high)
    pub high: Option<f64>,
    /// akshare 最低 (low)
    pub low: Option<f64>,
    /// akshare 成交量 (volume)
    pub volume: Option<f64>,
    /// akshare 成交额 (amount)
    pub amount: Option<f64>,
    /// requested category (封闭式基金 / ETF基金 / LOF基金)
    pub category: String,
}

/// Sina fund category list (akshare `fund_etf_category_sina`).
///
/// `symbol` is one of `{"封闭式基金", "ETF基金", "LOF基金"}`. The response is JSONP
/// (`Callback([...])`); we slice from the first `[` to the last `]` and parse the
/// JSON array — no JS evaluation (ADR-0005).
pub async fn fund_etf_category_sina(
    client: &Client,
    symbol: &str,
) -> Result<Vec<FundEtfCategorySinaRow>> {
    let node = match symbol {
        "封闭式基金" => "close_fund",
        "ETF基金" => "etf_hq_fund",
        "LOF基金" => "lof_hq_fund",
        _ => return Err(Error::InvalidParam(format!("unknown symbol: {symbol}"))),
    };
    let page = "1".to_string();
    let num = "5000".to_string();
    let asc = "0".to_string();
    let marker = "qvvne".to_string();
    let params = [
        ("page", page.as_str()),
        ("num", num.as_str()),
        ("sort", "symbol"),
        ("asc", asc.as_str()),
        ("node", node),
        ("[object HTMLDivElement]", marker.as_str()),
    ];
    let text = client
        .get_text(
            SOURCE_SINA,
            "fund_etf_category_sina",
            SINA_HQ_URL,
            &params,
            Some(&[("Referer", "https://vip.stock.finance.sina.com.cn/")]),
        )
        .await?;
    // The callback wrapper is `...CallbackList['...'](`, so the first `([`
    // marks the start of the actual data array (akshare uses `find("([")`).
    let start = text
        .find("([")
        .map(|i| i + 1)
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "fund_etf_category_sina: missing '([' in JSONP payload".into(),
        })?;
    let end = text.rfind(']').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "fund_etf_category_sina: missing ']' in JSONP payload".into(),
    })?;
    let body = &text[start..=end];
    let v: Value = serde_json::from_str(body).map_err(Error::Json)?;
    parse_fund_etf_category_sina(&v, symbol)
}

pub(crate) fn parse_fund_etf_category_sina(
    resp: &Value,
    category: &str,
) -> Result<Vec<FundEtfCategorySinaRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "fund_etf_category_sina: payload is not a JSON array".into(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(FundEtfCategorySinaRow {
            code: fstr(item, "symbol"),
            name: fstr(item, "name"),
            price: fnum(item, "trade"),
            price_change: fnum(item, "pricechange"),
            pct_change: fnum(item, "changepercent"),
            buy: fnum(item, "buy"),
            sell: fnum(item, "sell"),
            pre_close: fnum(item, "settlement"),
            open: fnum(item, "open"),
            high: fnum(item, "high"),
            low: fnum(item, "low"),
            volume: fnum(item, "volume"),
            amount: fnum(item, "amount"),
            category: category.to_string(),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Extract a string cell from a JSON array by index.
fn cell_str(cells: &[Value], idx: usize) -> String {
    cells
        .get(idx)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn type_map(symbol: &str) -> Result<&'static str> {
    FUND_VALUE_TYPE_MAP
        .iter()
        .find(|(k, _)| *k == symbol)
        .map(|(_, v)| *v)
        .ok_or_else(|| Error::InvalidParam(format!("unknown fund_value_em symbol: {symbol}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let txt = std::fs::read_to_string(path).expect("fixture missing");
        serde_json::from_str(&txt).expect("fixture is not valid JSON")
    }

    #[test]
    fn parses_fund_open_fund_name_em() {
        let v = fixture("fund_open_fund_name_em.json");
        let rows = parse_fund_open_fund_name_em(&v).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].fund_code, "000001");
        assert_eq!(rows[0].pinyin_abbr, "HXCZHH");
        assert_eq!(rows[0].short_name, "华夏成长混合");
        assert_eq!(rows[0].fund_type, "混合型");
        assert_eq!(rows[0].pinyin_full, "HUAXIACHENGZHANGHUNHE");
        assert_eq!(rows[2].fund_code, "510300");
        assert_eq!(rows[2].fund_type, "ETF-场内");
    }

    #[test]
    fn parses_fund_value_em() {
        let v = fixture("fund_value_em.json");
        let rows = parse_fund_value_em(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].fund_code, "000001");
        assert_eq!(rows[0].fund_name, "华夏成长混合");
        assert_eq!(rows[0].estimate_date, "2025-01-03");
        assert_eq!(rows[0].gz_value, Some(1.1234));
        assert_eq!(rows[0].gz_growth_rate, Some(1.05));
        assert_eq!(rows[0].published_nav, Some(1.1100));
        assert_eq!(rows[0].published_growth_rate, Some(0.90));
        assert_eq!(rows[0].estimate_deviation, Some(0.0134));
        assert_eq!(rows[1].fund_code, "110011");
        assert_eq!(rows[1].gz_growth_rate, Some(-0.50));
    }

    #[test]
    fn parses_fund_hist_em() {
        let v = fixture("fund_hist_em.json");
        let rows = parse_fund_hist_em(&v, "000001").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].fund_code, "000001");
        assert_eq!(rows[0].nav_date, "2025-01-03");
        assert_eq!(rows[0].unit_nav, Some(1.2345));
        assert_eq!(rows[0].acc_nav, Some(2.3456));
        assert_eq!(rows[0].daily_growth_rate, Some(1.23));
        assert_eq!(rows[0].purchase_status, "开放");
        assert_eq!(rows[0].redeem_status, "开放");
        assert_eq!(rows[1].nav_date, "2025-01-02");
        assert_eq!(rows[1].unit_nav, Some(1.2200));
    }

    #[test]
    fn parses_fund_money_meta() {
        let v = fixture("fund_money_meta.json");
        let rows = parse_fund_money_meta(&v, "000009").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].fund_code, "000009");
        assert_eq!(rows[0].nav_date, "2025-01-03");
        assert_eq!(rows[0].million_income, Some(0.5123));
        assert_eq!(rows[0].annualized_7day, Some(1.8456));
        assert_eq!(rows[0].purchase_status, "开放");
        assert_eq!(rows[0].redeem_status, "开放");
        assert_eq!(rows[1].million_income, Some(0.4987));
    }

    #[test]
    fn parses_fund_etf_category_sina() {
        let v = fixture("fund_etf_category_sina.json");
        let rows = parse_fund_etf_category_sina(&v, "LOF基金").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "sh510050");
        assert_eq!(rows[0].name, "华夏上证50ETF");
        assert_eq!(rows[0].price, Some(2.85));
        assert_eq!(rows[0].price_change, Some(0.03));
        assert_eq!(rows[0].pct_change, Some(1.05));
        assert_eq!(rows[0].buy, Some(2.84));
        assert_eq!(rows[0].sell, Some(2.86));
        assert_eq!(rows[0].pre_close, Some(2.82));
        assert_eq!(rows[0].open, Some(2.83));
        assert_eq!(rows[0].high, Some(2.88));
        assert_eq!(rows[0].low, Some(2.81));
        assert_eq!(rows[0].volume, Some(12_345_678.0));
        assert_eq!(rows[0].amount, Some(35_123_456.0));
        assert_eq!(rows[0].category, "LOF基金");
        assert_eq!(rows[1].code, "sz159915");
        assert_eq!(rows[1].pct_change, Some(-0.52));
    }
}
