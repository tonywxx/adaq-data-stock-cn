//! 董监高及相关人员持股变动 (SSE / SZSE / BSE executive shareholdings).
//!
//! Ports three functions from `akshare/stock/stock_share_hold.py`:
//!
//! | Rust fn | akshare fn | source |
//! |---|---|---|
//! | `stock_share_hold_change_sse` | `stock_share_hold_change_sse` | `akshare/stock/stock_share_hold.py:21` |
//! | `stock_share_hold_change_szse` | `stock_share_hold_change_szse` | `akshare/stock/stock_share_hold.py:118` |
//! | `stock_share_hold_change_bse` | `stock_share_hold_change_bse` | `akshare/stock/stock_share_hold.py:196` |
//!
//! SSE/SZSE are JSON `GET` (with a `Referer`/`User-Agent` header) reading
//! `result` / `data[0].data`; BSE is a JSONP (`null(...)`) text endpoint
//! reading `[0].result.content`. akshare paginates every page — these ports
//! fetch the first page only.
//!
//! ## DEFERRED
//! None.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

fn str_of(v: Option<&Value>) -> Option<String> {
    match v {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    }
}

fn num_of(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => {
            let t = s.trim().replace(',', "");
            if t.is_empty() {
                None
            } else {
                t.parse().ok()
            }
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// 上交所 (SSE)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ShareHoldSseRow {
    #[serde(rename = "公司代码")]
    pub company_code: Option<String>,
    #[serde(rename = "公司名称")]
    pub company_abbr: Option<String>,
    #[serde(rename = "姓名")]
    pub name: Option<String>,
    #[serde(rename = "职务")]
    pub duty: Option<String>,
    #[serde(rename = "股票种类")]
    pub stock_type: Option<String>,
    #[serde(rename = "货币种类")]
    pub currency_type: Option<String>,
    #[serde(rename = "本次变动前持股数")]
    pub current_num: Option<f64>,
    #[serde(rename = "变动数")]
    pub change_num: Option<f64>,
    #[serde(rename = "本次变动平均价格")]
    pub current_avg_price: Option<f64>,
    #[serde(rename = "变动后持股数")]
    pub holdstock_num: Option<f64>,
    #[serde(rename = "变动原因")]
    pub change_reason: Option<String>,
    #[serde(rename = "变动日期")]
    pub change_date: Option<String>,
    #[serde(rename = "填报日期")]
    pub form_date: Option<String>,
}

pub(crate) fn parse_share_hold_sse(resp: &Value) -> Result<Vec<ShareHoldSseRow>> {
    let arr = resp
        .get("result")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: "sse",
            message: "missing result".into(),
        })?;
    Ok(arr
        .iter()
        .map(|o| ShareHoldSseRow {
            company_code: str_of(o.get("COMPANY_CODE")),
            company_abbr: str_of(o.get("COMPANY_ABBR")),
            name: str_of(o.get("NAME")),
            duty: str_of(o.get("DUTY")),
            stock_type: str_of(o.get("STOCK_TYPE")),
            currency_type: str_of(o.get("CURRENCY_TYPE")),
            current_num: num_of(o.get("CURRENT_NUM")),
            change_num: num_of(o.get("CHANGE_NUM")),
            current_avg_price: num_of(o.get("CURRENT_AVG_PRICE")),
            holdstock_num: num_of(o.get("HOLDSTOCK_NUM")),
            change_reason: str_of(o.get("CHANGE_REASON")),
            change_date: str_of(o.get("CHANGE_DATE")),
            form_date: str_of(o.get("FORM_DATE")),
        })
        .collect())
}

/// Port of `stock_share_hold_change_sse(symbol)` (first page only).
pub async fn stock_share_hold_change_sse(
    client: &Client,
    symbol: &str,
) -> Result<Vec<ShareHoldSseRow>> {
    let mut company_code = "";
    if symbol != "全部" {
        company_code = symbol;
    }
    let params = [
        ("isPagination", "true"),
        ("pageHelp.pageSize", "100"),
        ("pageHelp.pageNo", "1"),
        ("pageHelp.beginPage", "1"),
        ("pageHelp.cacheSize", "1"),
        ("pageHelp.endPage", "1"),
        ("sqlId", "COMMON_SSE_XXPL_CXJL_SSGSGFBDQK_S"),
        ("COMPANY_CODE", company_code),
        ("NAME", ""),
        ("BEGIN_DATE", "1990-01-01"),
        ("END_DATE", "2050-01-01"),
        ("BOARDTYPE", ""),
    ];
    let headers = [
        ("Host", "query.sse.com.cn"),
        ("Referer", "https://www.sse.com.cn/"),
    ];
    let v = client
        .get_json_with_headers("sse", "stock_share_hold_change_sse", "https://query.sse.com.cn/commonQuery.do", &params, Some(&headers))
        .await?;
    parse_share_hold_sse(&v)
}

// ---------------------------------------------------------------------------
// 深交所 (SZSE)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ShareHoldSzseRow {
    #[serde(rename = "证券代码")]
    pub zqdm: Option<String>,
    #[serde(rename = "证券简称")]
    pub zqjc: Option<String>,
    #[serde(rename = "董监高姓名")]
    pub ggxm: Option<String>,
    #[serde(rename = "变动日期")]
    pub jyrq: Option<String>,
    #[serde(rename = "变动股份数量")]
    pub bdgs: Option<f64>,
    #[serde(rename = "成交均价")]
    pub bdjj: Option<f64>,
    #[serde(rename = "变动原因")]
    pub bdyy: Option<String>,
    #[serde(rename = "变动比例")]
    pub cgbdbl: Option<f64>,
    #[serde(rename = "当日结存股数")]
    pub cgzs: Option<f64>,
    #[serde(rename = "股份变动人姓名")]
    pub gdxm: Option<String>,
    #[serde(rename = "职务")]
    pub zw: Option<String>,
    #[serde(rename = "变动人与董监高的关系")]
    pub gxlb: Option<String>,
}

pub(crate) fn parse_share_hold_szse(resp: &Value) -> Result<Vec<ShareHoldSzseRow>> {
    let arr = resp
        .get(0)
        .and_then(|z| z.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: "szse",
            message: "missing data[0].data".into(),
        })?;
    Ok(arr
        .iter()
        .map(|o| ShareHoldSzseRow {
            zqdm: str_of(o.get("zqdm")),
            zqjc: str_of(o.get("zqjc")),
            ggxm: str_of(o.get("ggxm")),
            jyrq: str_of(o.get("jyrq")),
            bdgs: num_of(o.get("bdgs")),
            bdjj: num_of(o.get("bdjj")),
            bdyy: str_of(o.get("bdyy")),
            cgbdbl: num_of(o.get("cgbdbl")),
            cgzs: num_of(o.get("cgzs")),
            gdxm: str_of(o.get("gdxm")),
            zw: str_of(o.get("zw")),
            gxlb: str_of(o.get("gxlb")),
        })
        .collect())
}

/// Port of `stock_share_hold_change_szse(symbol)` (first page only).
pub async fn stock_share_hold_change_szse(
    client: &Client,
    symbol: &str,
) -> Result<Vec<ShareHoldSzseRow>> {
    let mut txt = "";
    if symbol != "全部" {
        txt = symbol;
    }
    let params = [
        ("SHOWTYPE", "JSON"),
        ("CATALOGID", "1801_cxda"),
        ("TABKEY", "tab1"),
        ("PAGENO", "1"),
        ("random", "0.7874198771222201"),
        ("txtDMorJC", txt),
    ];
    let headers = [(
        "User-Agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/93.0.4577.63 Safari/537.36",
    )];
    let v = client
        .get_json_with_headers("szse", "stock_share_hold_change_szse", "https://www.szse.cn/api/report/ShowReport/data", &params, Some(&headers))
        .await?;
    parse_share_hold_szse(&v)
}

// ---------------------------------------------------------------------------
// 北交所 (BSE) — JSONP `null(...)`
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ShareHoldBseRow {
    #[serde(rename = "代码")]
    pub stock_code: Option<String>,
    #[serde(rename = "简称")]
    pub stock_name: Option<String>,
    #[serde(rename = "姓名")]
    pub djg_name: Option<String>,
    #[serde(rename = "职务")]
    pub duty: Option<String>,
    #[serde(rename = "变动日期")]
    pub change_date: Option<String>,
    #[serde(rename = "变动股数")]
    pub change_amount: Option<f64>,
    #[serde(rename = "变动前持股数")]
    pub pre_amount: Option<f64>,
    #[serde(rename = "变动后持股数")]
    pub new_amount: Option<f64>,
    #[serde(rename = "变动均价")]
    pub price: Option<f64>,
    #[serde(rename = "变动原因")]
    pub reason: Option<String>,
}

/// Parse the stripped BSE JSONP array `[{ result: { content: [...] } }]`.
pub(crate) fn parse_share_hold_bse(arr: &[Value]) -> Result<Vec<ShareHoldBseRow>> {
    let content = arr
        .first()
        .and_then(|z| z.get("result"))
        .and_then(|r| r.get("content"))
        .and_then(|c| c.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: "bse",
            message: "missing [0].result.content".into(),
        })?;
    Ok(content
        .iter()
        .map(|o| ShareHoldBseRow {
            stock_code: str_of(o.get("stockCode")),
            stock_name: str_of(o.get("stockName")),
            djg_name: str_of(o.get("djgName")),
            duty: str_of(o.get("duty")),
            change_date: str_of(o.get("changeDate")),
            change_amount: num_of(o.get("changeAmount")),
            pre_amount: num_of(o.get("preAmount")),
            new_amount: num_of(o.get("newAmount")),
            price: num_of(o.get("price")),
            reason: str_of(o.get("reason")),
        })
        .collect())
}

fn unwrap_bse_jsonp(text: &str) -> Result<Value> {
    let t = text.strip_prefix("null(").unwrap_or(text);
    let t = t.strip_suffix(')').unwrap_or(t);
    serde_json::from_str(t).map_err(Error::Json)
}

/// Port of `stock_share_hold_change_bse(symbol)` (first page only).
pub async fn stock_share_hold_change_bse(
    client: &Client,
    symbol: &str,
) -> Result<Vec<ShareHoldBseRow>> {
    let stock = if symbol == "全部" { "" } else { symbol };
    let params = [
        ("page", "0"),
        ("startTime", ""),
        ("endTime", ""),
        ("stockCode", stock),
        ("djgName", ""),
        ("ssgs", "1"),
        ("sortfield", "bean.change_date desc, bean.stock_code asc, bean.change_amount desc, bean.price"),
        ("sorttype", "desc"),
    ];
    let headers = [(
        "User-Agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/93.0.4577.63 Safari/537.36",
    )];
    let text = client
        .get_text("bse", "stock_share_hold_change_bse", "https://www.bse.cn/djgCgbdController/getDjgCgbdList.do", &params, Some(&headers))
        .await?;
    let arr = unwrap_bse_jsonp(&text)?;
    let arr = arr.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: "bse",
        message: "expected JSON array".into(),
    })?;
    parse_share_hold_bse(arr)
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
    fn parses_sse() {
        let rows = parse_share_hold_sse(&fixture("stock_share_hold_change_sse.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].company_code.as_deref(), Some("600000"));
        assert_eq!(rows[0].name.as_deref(), Some("张三"));
        assert!(approx(rows[0].change_num, 5000.0));
        assert!(approx(rows[0].current_avg_price, 8.5));
        assert!(approx(rows[1].holdstock_num, 12000.0));
    }

    #[test]
    fn parses_szse() {
        let rows = parse_share_hold_szse(&fixture("stock_share_hold_change_szse.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].zqdm.as_deref(), Some("000001"));
        assert!(approx(rows[0].bdgs, 3000.0));
        assert!(approx(rows[0].cgzs, 1_234_567.0));
        assert_eq!(rows[1].gxlb.as_deref(), Some("本人"));
    }

    #[test]
    fn parses_bse() {
        let arr = fixture("stock_share_hold_change_bse.json")
            .as_array()
            .unwrap()
            .clone();
        let rows = parse_share_hold_bse(&arr).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].stock_code.as_deref(), Some("430489"));
        assert!(approx(rows[0].change_amount, 2000.0));
        assert!(approx(rows[1].price, 9.1));
    }
}
