//! 集思录 (Jisilu) convertible-bond endpoints. Ports `akshare/bond/bond_convert.py`.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `bond_cb_jsl` | `bond_convert.py:31` | POST `cb_list_new`, `rows[].cell` |
//! | `bond_cb_redeem_jsl` | `bond_convert.py:165` | POST `redeem_list`, `rows[].cell` |
//!
//! Both endpoints return pure JSON (`{"rows":[{"cell":{...}}]}`); no JS / cookie
//! auth is required in practice (verified by live fetch). `bond_cb_jsl` accepts
//! an optional browser `cookie` (passed straight through as a `Cookie` header),
//! matching akshare's `cookie` parameter.
//!
//! ## DEFERRED
//! - `bond_cb_index_jsl` (`bond_convert.py:17`) — uses `demjson.decode` on a
//!   non-strict JSON body; DEFERRED (demjson / non-strict JSON).
//! - `bond_cb_adj_logs_jsl` (`bond_convert.py:297`) — parses an HTML `<table>`
//!   via `pd.read_html`; DEFERRED (HTML-table scrape).

use serde_json::{Map, Value};
use serde_json::json;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE: &str = "jisilu";

const CB_LIST_URL: &str = "https://www.jisilu.cn/data/cbnew/cb_list_new/?___jsl=LST___t=1";
const REDEEM_LIST_URL: &str = "https://www.jisilu.cn/data/cbnew/redeem_list/?___jsl=LST___t=1";

fn cell_str(cell: &Map<String, Value>, key: &str) -> String {
    cell.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

/// Parse a cell value as `f64`, handling both JSON numbers and numeric strings.
fn cell_f64(cell: &Map<String, Value>, key: &str) -> Option<f64> {
    match cell.get(key) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// bond_cb_jsl — 集思录可转债列表
// ---------------------------------------------------------------------------

/// 集思录可转债行情行 (`bond_cb_jsl`). Column names / order match akshare output.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CbJslRow {
    #[serde(rename = "代码")] pub code: String,
    #[serde(rename = "转债名称")] pub bond_nm: String,
    #[serde(rename = "现价")] pub price: Option<f64>,
    #[serde(rename = "涨跌幅")] pub increase_rt: Option<f64>,
    #[serde(rename = "正股代码")] pub stock_id: String,
    #[serde(rename = "正股名称")] pub stock_nm: String,
    #[serde(rename = "正股价")] pub sprice: Option<f64>,
    #[serde(rename = "正股涨跌")] pub sincrease_rt: Option<f64>,
    #[serde(rename = "正股PB")] pub pb: Option<f64>,
    #[serde(rename = "转股价")] pub convert_price: Option<f64>,
    #[serde(rename = "转股价值")] pub convert_value: Option<f64>,
    #[serde(rename = "转股溢价率")] pub premium_rt: Option<f64>,
    #[serde(rename = "债券评级")] pub rating_cd: String,
    #[serde(rename = "回售触发价")] pub put_convert_price: Option<f64>,
    #[serde(rename = "强赎触发价")] pub force_redeem_price: Option<f64>,
    #[serde(rename = "转债占比")] pub convert_amt_ratio: Option<f64>,
    #[serde(rename = "到期时间")] pub maturity_dt: String,
    #[serde(rename = "剩余年限")] pub year_left: Option<f64>,
    #[serde(rename = "剩余规模")] pub curr_iss_amt: Option<f64>,
    #[serde(rename = "成交额")] pub volume: Option<f64>,
    #[serde(rename = "换手率")] pub turnover_rt: Option<f64>,
    #[serde(rename = "到期税前收益")] pub ytm_rt: Option<f64>,
    #[serde(rename = "双低")] pub dblow: Option<f64>,
}

pub(crate) fn parse_cb_jsl(resp: &Value) -> Result<Vec<CbJslRow>> {
    let rows = resp
        .get("rows")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing rows".into(),
        })?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let cell = match row.get("cell").and_then(|c| c.as_object()) {
            Some(c) => c,
            None => continue,
        };
        out.push(CbJslRow {
            code: cell_str(cell, "bond_id"),
            bond_nm: cell_str(cell, "bond_nm"),
            price: cell_f64(cell, "price"),
            increase_rt: cell_f64(cell, "increase_rt"),
            stock_id: cell_str(cell, "stock_id"),
            stock_nm: cell_str(cell, "stock_nm"),
            sprice: cell_f64(cell, "sprice"),
            sincrease_rt: cell_f64(cell, "sincrease_rt"),
            pb: cell_f64(cell, "pb"),
            convert_price: cell_f64(cell, "convert_price"),
            convert_value: cell_f64(cell, "convert_value"),
            premium_rt: cell_f64(cell, "premium_rt"),
            rating_cd: cell_str(cell, "rating_cd"),
            put_convert_price: cell_f64(cell, "put_convert_price"),
            force_redeem_price: cell_f64(cell, "force_redeem_price"),
            convert_amt_ratio: cell_f64(cell, "convert_amt_ratio"),
            maturity_dt: cell_str(cell, "maturity_dt"),
            year_left: cell_f64(cell, "year_left"),
            curr_iss_amt: cell_f64(cell, "curr_iss_amt"),
            volume: cell_f64(cell, "volume"),
            turnover_rt: cell_f64(cell, "turnover_rt"),
            ytm_rt: cell_f64(cell, "ytm_rt"),
            dblow: cell_f64(cell, "dblow"),
        });
    }
    Ok(out)
}

/// 集思录可转债列表 (`bond_cb_jsl`).
///
/// POSTs to the `cb_list_new` endpoint with the same JSON payload akshare sends
/// (only listed convertibles). `cookie` is optional and forwarded as a `Cookie`
/// header, mirroring akshare's `cookie` argument.
pub async fn bond_cb_jsl(client: &Client, cookie: Option<&str>) -> Result<Vec<CbJslRow>> {
    let body = json!({
        "fprice": "",
        "tprice": "",
        "curr_iss_amt": "",
        "volume": "",
        "svolume": "",
        "premium_rt": "",
        "ytm_rt": "",
        "market": "",
        "rating_cd": "",
        "is_search": "N",
        "market_cd[]": ["shmb", "shkc", "szmb", "szcy"],
        "btype": "",
        "listed": "Y",
        "qflag": "N",
        "sw_cd": "",
        "bond_ids": "",
        "rp": "50",
    });
    let mut hdrs: Vec<(&str, &str)> = vec![
        ("X-Requested-With", "XMLHttpRequest"),
        ("Referer", "https://www.jisilu.cn/data/cbnew/"),
    ];
    if let Some(c) = cookie {
        hdrs.push(("Cookie", c));
    }
    let v = client
        .post_json(SOURCE, "bond_cb_jsl", CB_LIST_URL, &body, Some(&hdrs))
        .await?;
    parse_cb_jsl(&v)
}

// ---------------------------------------------------------------------------
// bond_cb_redeem_jsl — 集思录可转债强赎
// ---------------------------------------------------------------------------

/// 集思录可转债强赎行 (`bond_cb_redeem_jsl`). Column names / order match akshare.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CbRedeemJslRow {
    #[serde(rename = "代码")] pub code: String,
    #[serde(rename = "名称")] pub bond_nm: String,
    #[serde(rename = "现价")] pub price: Option<f64>,
    #[serde(rename = "正股代码")] pub stock_id: String,
    #[serde(rename = "正股名称")] pub stock_nm: String,
    #[serde(rename = "规模")] pub orig_iss_amt: Option<f64>,
    #[serde(rename = "剩余规模")] pub curr_iss_amt: Option<f64>,
    #[serde(rename = "转股起始日")] pub convert_dt: String,
    #[serde(rename = "最后交易日")] pub delist_dt: String,
    #[serde(rename = "到期日")] pub maturity_dt: String,
    #[serde(rename = "转股价")] pub convert_price: Option<f64>,
    #[serde(rename = "强赎触发比")] pub redeem_price_ratio: Option<f64>,
    #[serde(rename = "强赎触发价")] pub force_redeem_price: Option<f64>,
    #[serde(rename = "正股价")] pub sprice: Option<f64>,
    #[serde(rename = "强赎价")] pub real_force_redeem_price: Option<f64>,
    #[serde(rename = "强赎天计数")] pub redeem_count: String,
    #[serde(rename = "强赎条款")] pub redeem_tc: String,
    #[serde(rename = "强赎状态")] pub redeem_status: String,
}

/// Strip HTML tags from a string (no regex crate available).
fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// Replicate akshare's `redeem_count` regex `\d{1,2}/\d{1,2} \| \d{1,2}` without
/// the `regex` crate: pull the first `digits/digits | digits` span after
/// stripping HTML tags. Falls back to the trimmed text.
fn redeem_count_short(s: &str) -> String {
    let cleaned = strip_html(s);
    let chars: Vec<(usize, char)> = cleaned.char_indices().collect();
    let n = chars.len();
    let is_d = |c: char| c.is_ascii_digit();
    for i in 0..n {
        let (bi, c) = chars[i];
        if !is_d(c) {
            continue;
        }
        let mut j = i;
        while j < n && is_d(chars[j].1) {
            j += 1;
        }
        if j < n && chars[j].1 == '/' {
            let mut l = j + 1;
            while l < n && is_d(chars[l].1) {
                l += 1;
            }
            if l + 2 < n && chars[l].1 == ' ' && chars[l + 1].1 == '|' && chars[l + 2].1 == ' ' {
                let mut p = l + 3;
                while p < n && is_d(chars[p].1) {
                    p += 1;
                }
                if p > l + 3 {
                    let start = bi;
                    let last = chars[p - 1];
                    let end = last.0 + last.1.len_utf8();
                    return cleaned[start..end].to_string();
                }
            }
        }
    }
    cleaned.trim().to_string()
}

/// Map jisilu `redeem_icon` to akshare's Chinese `强赎状态` label.
fn redeem_status(icon: &str) -> String {
    match icon {
        "R" => "已公告强赎",
        "O" => "公告要强赎",
        "G" => "公告不强赎",
        "B" => "已满足强赎条件",
        _ => "",
    }
    .to_string()
}

/// Parse `"130%"`-style strings into f64 (strip trailing `%`).
fn pct_f64(s: &str) -> Option<f64> {
    s.trim().trim_end_matches('%').trim().parse::<f64>().ok()
}

pub(crate) fn parse_cb_redeem_jsl(resp: &Value) -> Result<Vec<CbRedeemJslRow>> {
    let rows = resp
        .get("rows")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing rows".into(),
        })?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let cell = match row.get("cell").and_then(|c| c.as_object()) {
            Some(c) => c,
            None => continue,
        };
        let ratio_raw = cell_str(cell, "redeem_price_ratio");
        let icon = cell_str(cell, "redeem_icon");
        let count_raw = cell_str(cell, "redeem_count");
        out.push(CbRedeemJslRow {
            code: cell_str(cell, "bond_id"),
            bond_nm: cell_str(cell, "bond_nm"),
            price: cell_f64(cell, "price"),
            stock_id: cell_str(cell, "stock_id"),
            stock_nm: cell_str(cell, "stock_nm"),
            orig_iss_amt: cell_f64(cell, "orig_iss_amt"),
            curr_iss_amt: cell_f64(cell, "curr_iss_amt"),
            convert_dt: cell_str(cell, "convert_dt"),
            delist_dt: cell_str(cell, "delist_dt"),
            maturity_dt: cell_str(cell, "maturity_dt"),
            convert_price: cell_f64(cell, "convert_price"),
            redeem_price_ratio: if ratio_raw.is_empty() {
                None
            } else {
                pct_f64(&ratio_raw)
            },
            force_redeem_price: cell_f64(cell, "force_redeem_price"),
            sprice: cell_f64(cell, "sprice"),
            real_force_redeem_price: cell_f64(cell, "real_force_redeem_price"),
            redeem_count: redeem_count_short(&count_raw),
            redeem_tc: cell_str(cell, "redeem_tc"),
            redeem_status: redeem_status(&icon),
        });
    }
    Ok(out)
}

/// 集思录可转债强赎 (`bond_cb_redeem_jsl`).
///
/// POSTs to the `redeem_list` endpoint; returns pure JSON (`rows[].cell`).
pub async fn bond_cb_redeem_jsl(client: &Client) -> Result<Vec<CbRedeemJslRow>> {
    let body = json!({ "rp": "50" });
    let hdrs: [(&str, &str); 1] = [("X-Requested-With", "XMLHttpRequest")];
    let v = client
        .post_json(
            SOURCE,
            "bond_cb_redeem_jsl",
            REDEEM_LIST_URL,
            &body,
            Some(&hdrs),
        )
        .await?;
    parse_cb_redeem_jsl(&v)
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
    fn parse_cb_jsl_ok() {
        let rows = parse_cb_jsl(&fixture("bond_cb_jsl.json")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].code, "123283");
        assert_eq!(rows[0].bond_nm, "丰茂转债");
        assert!(approx(rows[0].price, 100.0));
        assert!(approx(rows[0].sprice, 37.57));
        assert!(approx(rows[0].convert_price, 35.59));
        assert!(approx(rows[0].premium_rt, -5.27));
        assert!(approx(rows[0].dblow, 94.73));
        assert_eq!(rows[0].rating_cd, "AA-");
        assert!(approx(rows[0].year_left, 6.003));
        assert_eq!(rows[0].maturity_dt, "2032-08-17");
    }

    #[test]
    fn parse_cb_redeem_jsl_ok() {
        let rows = parse_cb_redeem_jsl(&fixture("bond_cb_redeem_jsl.json")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].code, "113697");
        assert_eq!(rows[0].bond_nm, "应流转债");
        assert!(approx(rows[0].price, 161.049));
        assert!(approx(rows[0].orig_iss_amt, 15.0));
        assert!(approx(rows[0].curr_iss_amt, 14.653));
        assert_eq!(rows[0].convert_dt, "2026-03-25");
        assert_eq!(rows[0].maturity_dt, "2031-09-19");
        assert!(approx(rows[0].convert_price, 30.31));
        assert!(approx(rows[0].redeem_price_ratio, 130.0));
        assert!(approx(rows[0].force_redeem_price, 39.403));
        assert!(approx(rows[0].sprice, 48.98));
        assert!(approx(rows[0].real_force_redeem_price, 100.094));
        assert_eq!(rows[0].redeem_status, "已公告强赎");
        // redeem_count regex-equivalent: keep first `digits/digits | digits` span.
        assert!(rows[0].redeem_count.contains('/'));
    }
}
