use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// ChinaMoney source id (local, not in `core::client`).
const SOURCE_CHINAMONEY: &str = "chinamoney";

const SPOT_QUOTE_URL: &str = "https://www.chinamoney.com.cn/ags/ms/cm-u-md-bond/CbMktMakQuot";
const SPOT_DEAL_URL: &str = "https://www.chinamoney.com.cn/ags/ms/cm-u-md-bond/CbtPri";

// ---------------------------------------------------------------------------
// bond_spot_quote — 现券市场做市报价 (ChinaMoney, POST `records` array)
// https://www.chinamoney.com.cn/chinese/mkdatabond/
// ---------------------------------------------------------------------------

/// China interbank bond maker-quote row (`bond_spot_quote`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondSpotQuote {
    pub institution: String,
    pub bond_name: String,
    pub buy_net_price: Option<f64>,
    pub sell_net_price: Option<f64>,
    pub buy_yield: Option<f64>,
    pub sell_yield: Option<f64>,
    pub source: &'static str,
}

/// China interbank bond deal row (`bond_spot_deal`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondSpotDeal {
    pub bond_name: String,
    pub deal_net_price: Option<f64>,
    pub latest_yield: Option<f64>,
    pub change: Option<f64>,
    pub weighted_yield: Option<f64>,
    pub volume: Option<f64>,
    pub source: &'static str,
}

/// 现券市场做市报价 from ChinaMoney (`bond_spot_quote`).
///
/// POSTs to the `CbMktMakQuot` endpoint; `records` is a positional array.
/// akshare also calls a `bond_china_close_return_map()` pre-step (cookie/token
/// bootstrap) which we omit — see report.
pub async fn bond_spot_quote(client: &Client) -> Result<Vec<BondSpotQuote>> {
    let params = [("flag", "1"), ("lang", "cn")];
    let v = client
        .post_form_json(
            SOURCE_CHINAMONEY,
            "bond_spot_quote",
            SPOT_QUOTE_URL,
            &params,
            None,
        )
        .await?;
    parse_spot_quote(&v)
}

pub(crate) fn parse_spot_quote(resp: &Value) -> Result<Vec<BondSpotQuote>> {
    let records = resp
        .get("records")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CHINAMONEY,
            message: "missing records".into(),
        })?;
    let mut out = Vec::with_capacity(records.len());
    for rec in records {
        // Each record is a positional array (akshare renames by column index).
        let arr = match rec.as_array() {
            Some(a) => a,
            None => continue, // skip malformed row
        };
        if arr.len() < 14 {
            continue;
        }
        let institution = at_str(arr, 2);
        let bond_name = at_str(arr, 6);
        let (buy_yield, sell_yield) = split_pair(at_str(arr, 11));
        let (buy_net_price, sell_net_price) = split_pair(at_str(arr, 13));
        out.push(BondSpotQuote {
            institution,
            bond_name,
            buy_net_price,
            sell_net_price,
            buy_yield,
            sell_yield,
            source: SOURCE_CHINAMONEY,
        });
    }
    Ok(out)
}

/// 现券市场成交行情 from ChinaMoney (`bond_spot_deal`).
pub async fn bond_spot_deal(client: &Client) -> Result<Vec<BondSpotDeal>> {
    let params = [("flag", "1"), ("lang", "cn"), ("bondName", "")];
    let v = client
        .post_form_json(
            SOURCE_CHINAMONEY,
            "bond_spot_deal",
            SPOT_DEAL_URL,
            &params,
            None,
        )
        .await?;
    parse_spot_deal(&v)
}

pub(crate) fn parse_spot_deal(resp: &Value) -> Result<Vec<BondSpotDeal>> {
    let records = resp
        .get("records")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CHINAMONEY,
            message: "missing records".into(),
        })?;
    let mut out = Vec::with_capacity(records.len());
    for rec in records {
        let arr = match rec.as_array() {
            Some(a) => a,
            None => continue,
        };
        if arr.len() < 18 {
            continue;
        }
        let bond_name = at_str(arr, 2);
        let change = at_num(arr, 7);
        let weighted_yield = at_num(arr, 11);
        let deal_net_price = at_num(arr, 12);
        let latest_yield = at_num(arr, 15);
        let volume = at_num(arr, 17);
        out.push(BondSpotDeal {
            bond_name,
            deal_net_price,
            latest_yield,
            change,
            weighted_yield,
            volume,
            source: SOURCE_CHINAMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Helpers for positional `records` arrays
// ---------------------------------------------------------------------------

fn at_str(arr: &[Value], idx: usize) -> String {
    arr.get(idx)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn at_num(arr: &[Value], idx: usize) -> Option<f64> {
    arr.get(idx).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    })
}

/// Split a `"a/b"` string into `(a, b)` parsed as f64.
fn split_pair(s: String) -> (Option<f64>, Option<f64>) {
    let mut it = s.split('/');
    let a = it.next().and_then(|x| x.trim().parse::<f64>().ok());
    let b = it.next().and_then(|x| x.trim().parse::<f64>().ok());
    (a, b)
}

// ---------------------------------------------------------------------------
// macro_china_bond_public — 债券发行信息披露 (ChinaMoney, POST `records`)
// https://www.chinamoney.com.cn/chinese/xzjfx/
// ---------------------------------------------------------------------------

const BOND_EMIT_URL: &str = "https://www.chinamoney.com.cn/ags/ms/cm-u-bond-an/bnBondEmit";

/// One bond-issuance disclosure row (`macro_china_bond_public`).
///
/// akshare renames the upstream positional record columns; the meaningful
/// fields (in akshare output order) are 债券全称, 债券类型, 发行日期, 计息方式,
/// 价格, 债券期限, 计划发行量, 债券评级.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondPublicEmit {
    pub bond_full_name: String,
    pub bond_type: String,
    pub issue_date: String,
    pub interest_method: String,
    pub price: Option<f64>,
    pub bond_term: String,
    pub planned_issue_size: Option<f64>,
    pub bond_rating: String,
}

/// Parse `macro_china_bond_public` rows from an already-fetched ChinaMoney
/// `bnBondEmit` response (one page). `pub(crate)` so tests can call directly.
pub(crate) fn parse_bond_public(resp: &Value) -> Result<Vec<BondPublicEmit>> {
    let records = resp
        .get("records")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CHINAMONEY,
            message: "missing records".into(),
        })?;
    let mut out = Vec::with_capacity(records.len());
    for rec in records {
        let obj = match rec.as_object() {
            Some(o) => o,
            None => continue,
        };
        // akshare renames by position; read values in JSON order.
        let vals: Vec<&Value> = obj.values().collect();
        let g = |i: usize| -> &Value { vals.get(i).copied().unwrap_or(&Value::Null) };
        let str_at = |i: usize| -> String {
            match g(i) {
                Value::String(s) => s.clone(),
                Value::Null => String::new(),
                other => other.to_string(),
            }
        };
        let num_at = |i: usize| -> Option<f64> {
            match g(i) {
                Value::String(s) => s.parse::<f64>().ok(),
                Value::Number(n) => n.as_f64(),
                _ => None,
            }
        };
        out.push(BondPublicEmit {
            bond_full_name: str_at(0),
            bond_type: str_at(1),
            issue_date: str_at(3),
            interest_method: str_at(5),
            price: num_at(11),
            bond_term: str_at(7),
            planned_issue_size: num_at(12),
            bond_rating: str_at(9),
        });
    }
    Ok(out)
}

/// 中国-债券信息披露-债券发行 (`macro_china_bond_public`, `bond_china_money.py:313`).
///
/// Paginates ChinaMoney `bnBondEmit` (POST form). The upstream pre-step
/// `bond_china_close_return_map()` only bootstraps a cookie and is omitted.
pub async fn macro_china_bond_public(client: &Client) -> Result<Vec<BondPublicEmit>> {
    let mut page: i64 = 1;
    let mut total: i64 = 1;
    let mut out: Vec<BondPublicEmit> = Vec::new();
    loop {
        let page_s = page.to_string();
        let params = [
            ("enty", ""),
            ("bondType", ""),
            ("bondNameCode", ""),
            ("leadUnderwriter", ""),
            ("pageNo", page_s.as_str()),
            ("pageSize", "10"),
            ("limit", "1"),
        ];
        let v = client
            .post_form_json(
                SOURCE_CHINAMONEY,
                "macro_china_bond_public",
                BOND_EMIT_URL,
                &params,
                None,
            )
            .await?;
        if page == 1 {
            total = v
                .get("data")
                .and_then(|d| d.get("pageTotalSize"))
                .and_then(|t| t.as_i64())
                .unwrap_or(1);
        }
        let rows = parse_bond_public(&v)?;
        if rows.is_empty() && page > 1 {
            break;
        }
        out.extend(rows);
        if page >= total {
            break;
        }
        page += 1;
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
        let txt = std::fs::read_to_string(p).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    #[test]
    fn parses_spot_quote_fixture() {
        let v = fixture("bond_spot_quote.json");
        let rows = parse_spot_quote(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].institution, "机构A");
        assert_eq!(rows[0].bond_name, "债券X");
        assert_eq!(rows[0].buy_yield, Some(2.50));
        assert_eq!(rows[0].sell_yield, Some(2.60));
        assert_eq!(rows[0].buy_net_price, Some(100.1));
        assert_eq!(rows[0].sell_net_price, Some(100.3));
        assert_eq!(rows[0].source, "chinamoney");
        assert_eq!(rows[1].bond_name, "债券Y");
    }

    #[test]
    fn parses_spot_deal_fixture() {
        let v = fixture("bond_spot_deal.json");
        let rows = parse_spot_deal(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].bond_name, "债券Y");
        assert_eq!(rows[0].change, Some(0.3));
        assert_eq!(rows[0].weighted_yield, Some(2.65));
        assert_eq!(rows[0].deal_net_price, Some(100.5));
        assert_eq!(rows[0].latest_yield, Some(2.70));
        assert_eq!(rows[0].volume, Some(1000.0));
        assert_eq!(rows[0].source, "chinamoney");
        assert_eq!(rows[1].change, Some(0.1));
    }
}
