//! Miscellaneous stock utilities (Eastmoney datacenter / clist where possible).
//!
//! Ports of akshare functions that don't warrant their own module yet:
//! - `stock_zh_a_gdhs`  (股东户数, Eastmoney `RPT_HOLDERNUM_DET` / `_LATEST`)
//! - `stock_dividend`   (分红; akshare source is cninfo, JS-signed — see note)
//! - `stock_rank_em`    (人气榜; akshare source is emappdata, JSON-POST — see note)

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

// ---------------------------------------------------------------------------
// stock_zh_a_gdhs — shareholder count
// ---------------------------------------------------------------------------

const GDHS_URL: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";
const GDHS_PAGE_SIZE: u32 = 500;

/// Shareholder-count snapshot for one stock at one reporting date.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GdhsRow {
    pub code: String,
    pub name: String,
    pub holder_count: Option<f64>,
    pub prev_holder_count: Option<f64>,
    pub holder_change: Option<f64>,
    pub holder_ratio: Option<f64>,
    pub notice_date: Option<String>,
    pub source: &'static str,
}

/// Shareholder counts. `symbol` is `"最新"` for the latest snapshot, or a
/// quarter-end date like `"20230930"` (akshare semantics).
pub async fn stock_zh_a_gdhs(client: &Client, symbol: &str) -> Result<Vec<GdhsRow>> {
    let (report, filter) = if symbol == "最新" {
        ("RPT_HOLDERNUMLATEST", None)
    } else {
        let d = format!(
            "(END_DATE='{}-{}-{}')",
            &symbol[..4],
            &symbol[4..6],
            &symbol[6..]
        );
        ("RPT_HOLDERNUM_DET", Some(d))
    };
    let columns = "SECURITY_CODE,SECURITY_NAME_ABBR,END_DATE,INTERVAL_CHRATE,AVG_MARKET_CAP,AVG_HOLD_NUM,TOTAL_MARKET_CAP,TOTAL_A_SHARES,HOLD_NOTICE_DATE,HOLDER_NUM,PRE_HOLDER_NUM,HOLDER_NUM_CHANGE,HOLDER_NUM_RATIO,END_DATE,PRE_END_DATE";
    let mut out = Vec::new();
    let mut page: u32 = 1;
    loop {
        let page_s = page.to_string();
        let pz_s = GDHS_PAGE_SIZE.to_string();
        let mut params: Vec<(&str, &str)> = vec![
            ("sortColumns", "HOLD_NOTICE_DATE,SECURITY_CODE"),
            ("sortTypes", "-1,-1"),
            ("pageSize", &pz_s),
            ("pageNumber", &page_s),
            ("reportName", report),
            ("columns", columns),
            ("quoteColumns", "f2,f3"),
            ("source", "WEB"),
            ("client", "WEB"),
        ];
        if let Some(f) = &filter {
            params.push(("filter", f.as_str()));
        }
        let v = client
            .get_json(SOURCE_EASTMONEY, "stock_zh_a_gdhs", GDHS_URL, &params)
            .await?;
        let data = v
            .get("result")
            .and_then(|r| r.get("data"))
            .and_then(|d| d.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing result.data".into(),
            })?;
        if data.is_empty() {
            break;
        }
        out.extend(parse_gdhs(&v)?);
        let pages = v
            .get("result")
            .and_then(|r| r.get("pages"))
            .and_then(|p| p.as_u64())
            .unwrap_or(1);
        if page as u64 >= pages {
            break;
        }
        page += 1;
    }
    Ok(out)
}

/// Map an Eastmoney `RPT_HOLDERNUM_*` response to [`GdhsRow`]s. Rows without a
/// security code are skipped.
pub(crate) fn parse_gdhs(resp: &Value) -> Result<Vec<GdhsRow>> {
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
        let code = str_opt(item, "SECURITY_CODE");
        if code.is_empty() {
            continue;
        }
        let notice_date = item
            .get("HOLD_NOTICE_DATE")
            .and_then(|v| v.as_str())
            .filter(|s| s.len() >= 10)
            .map(|s| s[..10].to_string());
        out.push(GdhsRow {
            code,
            name: str_opt(item, "SECURITY_NAME_ABBR"),
            holder_count: num_opt(item, "HOLDER_NUM"),
            prev_holder_count: num_opt(item, "PRE_HOLDER_NUM"),
            holder_change: num_opt(item, "HOLDER_NUM_CHANGE"),
            holder_ratio: num_opt(item, "HOLDER_NUM_RATIO"),
            notice_date,
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// stock_dividend — historical dividends (cninfo)
// ---------------------------------------------------------------------------
//
// akshare's `stock_dividend_cninfo` hits cninfo's `p_sysapi1139` and requires a
// JS-computed `Accept-Enckey` header (`getResCode1` in cninfo.js). Because that
// needs a JS engine (violates the "no JS signing" rule), the LIVE fetch is not
// implemented here; the row mapping + parse are provided and fixture-tested so
// the lead can wire a token source later.

const DIVIDEND_URL: &str = "https://webapi.cninfo.com.cn/api/sysapi/p_sysapi1139";
const SOURCE_CNINFO: &str = "cninfo";

/// One historical dividend action for a stock.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DividendRow {
    pub announce_date: Option<String>,
    pub dividend_type: Option<String>,
    pub bonus_share_ratio: Option<f64>,
    pub reserve_to_share_ratio: Option<f64>,
    pub cash_div_ratio: Option<f64>,
    pub record_date: Option<String>,
    pub ex_date: Option<String>,
    pub pay_date: Option<String>,
    pub report_date: Option<String>,
}

/// Historical dividends for `symbol` (stock code). Live fetch requires the
/// cninfo `Accept-Enckey` header and therefore fails without a JS-signed token.
pub async fn stock_dividend(client: &Client, symbol: &str) -> Result<Vec<DividendRow>> {
    let params = [("scode", symbol)];
    let v = client
        .get_json(SOURCE_CNINFO, "stock_dividend", DIVIDEND_URL, &params)
        .await?;
    parse_dividend(&v)
}

/// Map a cninfo `records` response to [`DividendRow`]s.
pub(crate) fn parse_dividend(resp: &Value) -> Result<Vec<DividendRow>> {
    let records = resp
        .get("records")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CNINFO,
            message: "missing records".into(),
        })?;
    let non_empty = |s: Option<String>| s.filter(|x| !x.is_empty());
    let mut out = Vec::with_capacity(records.len());
    for item in records {
        let strf = |k: &str| item.get(k).and_then(|v| v.as_str()).map(|s| s.to_string());
        out.push(DividendRow {
            announce_date: non_empty(strf("F006D")),
            dividend_type: strf("F044V"),
            bonus_share_ratio: num_opt(item, "F010N"),
            reserve_to_share_ratio: num_opt(item, "F011N"),
            cash_div_ratio: num_opt(item, "F012N"),
            record_date: non_empty(strf("F018D")),
            ex_date: non_empty(strf("F020D")),
            pay_date: non_empty(strf("F023D")),
            report_date: strf("F001V"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// stock_rank_em — popularity ranking (emappdata)
// ---------------------------------------------------------------------------
//
// akshare's `stock_hot_rank_em` POSTs JSON to emappdata's `getAllCurrentList`
// and then enriches with push2 quotes. The Client only exposes a form-encoded
// POST (`post_form_json`), not a JSON-body POST, so the two-step LIVE fetch is
// not implemented here; the row mapping + parse are provided and fixture-tested.

const RANK_URL: &str = "https://emappdata.eastmoney.com/stockrank/getAllCurrentList";

/// One popularity-ranking entry (push2-enriched fields are filled at runtime).
#[derive(Debug, Clone, serde::Serialize)]
pub struct RankRow {
    pub rank: Option<u32>,
    pub code: String,
    pub name: String,
    pub price: Option<f64>,
    pub change: Option<f64>,
    pub pct_change: Option<f64>,
    pub source: &'static str,
}

/// Popularity ranking. Live fetch needs a JSON-body POST (emappdata) — see note.
pub async fn stock_rank_em(client: &Client) -> Result<Vec<RankRow>> {
    let params = [
        ("appId", "appId01"),
        ("globalId", "786e4c21-70dc-435a-93bb-38"),
        ("marketType", ""),
        ("pageNo", "1"),
        ("pageSize", "100"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_rank_em", RANK_URL, &params)
        .await?;
    let data = v.get("data").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "missing data".into(),
    })?;
    parse_rank(data)
}

/// Map the emappdata rank list (`[{sc, rk, ...}]`) to [`RankRow`]s.
pub(crate) fn parse_rank(resp: &Value) -> Result<Vec<RankRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "expected a JSON array".into(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let code = item
            .get("sc")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if code.is_empty() {
            continue;
        }
        let rank = item.get("rk").and_then(|v| v.as_u64()).map(|x| x as u32);
        out.push(RankRow {
            rank,
            code,
            name: String::new(),
            price: None,
            change: None,
            pct_change: None,
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

fn str_opt(item: &Value, k: &str) -> String {
    item.get(k)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn num_opt(item: &Value, k: &str) -> Option<f64> {
    match item.get(k) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn parses_gdhs_fixture() {
        let v = fixture("stock_zh_a_gdhs.json");
        let rows = parse_gdhs(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "600000");
        assert_eq!(rows[0].name, "浦发银行");
        assert_eq!(rows[0].holder_count, Some(123456.0));
        assert_eq!(rows[0].holder_change, Some(-544.0));
        assert_eq!(rows[0].notice_date.as_deref(), Some("2024-03-30"));
    }

    #[test]
    fn parses_dividend_fixture() {
        let v = fixture("stock_dividend.json");
        let rows = parse_dividend(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].announce_date.as_deref(), Some("2024-06-01"));
        assert_eq!(rows[0].cash_div_ratio, Some(5.0));
        assert_eq!(rows[0].bonus_share_ratio, Some(0.0));
        assert_eq!(rows[1].reserve_to_share_ratio, Some(3.0));
    }

    #[test]
    fn parses_rank_fixture() {
        let v = fixture("stock_rank_em.json");
        let rows = parse_rank(&v).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].code, "SZ000665");
        assert_eq!(rows[0].rank, Some(1));
        assert_eq!(rows[2].rank, Some(3));
    }
}
