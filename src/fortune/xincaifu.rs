//! 新财富 500 人富豪榜 (port of `akshare/fortune/fortune_xincaifu_500.py`).
//!
//! `xincaifu_rank` is the only pure-HTTP function in the fortune domain: it
//! GETs a JSONP endpoint and reads `data.rows`. The response is wrapped as
//! `jsonpCallback({...});`, so we strip the padding before parsing.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `xincaifu_rank` | `fortune_xincaifu_500.py:15` | `service.ikuyu.cn/.../bdListAction.do`, JSONP `data.rows` |
//!
//! ## DEFERRED
//!
//! None in this file; see `crate::fortune` for the domain-level deferrals
//! (`index_bloomberg_billionaires*`, `forbes_rank`, `hurun_rank`,
//! `business_value_artist`, `online_value_artist`).

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE: &str = "xincaifu";
const URL: &str = "http://service.ikuyu.cn/XinCaiFu2/pcremoting/bdListAction.do";

/// Read a string field.
fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(str::to_string)
}

/// Read a numeric field (handles JSON numbers and numeric strings).
fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

/// A single entry of the 新财富 500 人富豪榜.
///
/// Mirrors akshare's output column order: 排名, 财富, 姓名, 主要公司, 相关行业,
/// 公司总部, 性别, 年龄, 年份 → `rank, wealth, name, company, industry,
/// headquarters, sex, age, year`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct XincaifuRow {
    /// 排名 (akshare `rank`)
    pub rank: Option<String>,
    /// 财富 / 净资产，单位亿元 (akshare `assets`)
    pub wealth: Option<f64>,
    /// 姓名 (akshare `name`)
    pub name: Option<String>,
    /// 主要公司 (akshare `company`)
    pub company: Option<String>,
    /// 相关行业 (akshare `industry`)
    pub industry: Option<String>,
    /// 公司总部 (akshare `addr`)
    pub headquarters: Option<String>,
    /// 性别 (akshare `sex`)
    pub sex: Option<String>,
    /// 年龄 (akshare `age`)
    pub age: Option<String>,
    /// 年份 (akshare `year`)
    pub year: Option<String>,
}

/// Strip a JSONP wrapper (`name({...});`) and parse the inner JSON object.
fn strip_jsonp(text: &str) -> Result<Value> {
    let s = text.trim();
    let start = s.find('(').map(|i| i + 1).unwrap_or(0);
    let end = s.rfind(')').unwrap_or(s.len());
    if start >= end {
        return Err(Error::UpstreamChanged {
            origin: SOURCE,
            message: "JSONP response missing callback wrapper".into(),
        });
    }
    serde_json::from_str(&s[start..end]).map_err(|e| Error::UpstreamChanged {
        origin: SOURCE,
        message: format!("invalid JSONP body: {e}"),
    })
}

/// Parse `xincaifu_rank` rows from the already-stripped JSON value.
/// `pub(crate)` so tests can call it directly. Pure (no I/O).
pub(crate) fn parse_xincaifu_rank(resp: &Value) -> Result<Vec<XincaifuRow>> {
    let rows = resp
        .get("data")
        .and_then(|d| d.get("rows"))
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing data.rows".into(),
        })?;
    let mut out = Vec::with_capacity(rows.len());
    for item in rows {
        out.push(XincaifuRow {
            rank: fstr(item, "rank"),
            wealth: fnum(item, "assets"),
            name: fstr(item, "name"),
            company: fstr(item, "company"),
            industry: fstr(item, "industry"),
            headquarters: fstr(item, "addr"),
            sex: fstr(item, "sex"),
            age: fstr(item, "age"),
            year: fstr(item, "year"),
        });
    }
    Ok(out)
}

/// Shared fetch + parse for a given `year` (akshare `fortune_xincaifu_500.py:15`).
async fn fetch_xincaifu(client: &Client, year: &str) -> Result<Vec<XincaifuRow>> {
    let params: &[(&str, &str)] = &[
        ("method", "getPage"),
        ("callback", "jsonpCallback"),
        ("sortBy", ""),
        ("order", ""),
        ("type", "4"),
        ("keyword", ""),
        ("pageSize", "1000"),
        ("year", year),
        ("pageNo", "1"),
        ("from", "jsonp"),
    ];
    let text = client
        .get_text(SOURCE, "xincaifu_rank", URL, params, None)
        .await?;
    let v = strip_jsonp(&text)?;
    parse_xincaifu_rank(&v)
}

/// 新财富 500 人富豪榜, default `year="2022"` (akshare `fortune_xincaifu_500.py:15`).
pub async fn xincaifu_rank(client: &Client) -> Result<Vec<XincaifuRow>> {
    xincaifu_rank_year(client, "2022").await
}

/// 新财富 500 人富豪榜 for an explicit `year` (data spans 2003–present).
pub async fn xincaifu_rank_year(client: &Client, year: &str) -> Result<Vec<XincaifuRow>> {
    fetch_xincaifu(client, year).await
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
    fn parse_xincaifu_rank_ok() {
        // Fixture is the already-stripped JSONP body (`{"data":{"rows":[...]}}`).
        let rows = parse_xincaifu_rank(&fixture("xincaifu_rank.json")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].rank, Some("1".to_string()));
        assert!(approx(rows[0].wealth, 2000.0));
        assert_eq!(rows[0].name, Some("张三".to_string()));
        assert_eq!(rows[0].company, Some("某某实业".to_string()));
        assert_eq!(rows[0].industry, Some("房地产".to_string()));
        assert_eq!(rows[0].headquarters, Some("北京".to_string()));
        assert_eq!(rows[0].sex, Some("男".to_string()));
        assert_eq!(rows[0].age, Some("55".to_string()));
        assert_eq!(rows[0].year, Some("2022".to_string()));

        assert_eq!(rows[2].rank, Some("3".to_string()));
        assert_eq!(rows[2].sex, None);
        assert!(approx(rows[2].wealth, 1500.0));
    }

    #[test]
    fn strip_jsonp_ok() {
        let v = strip_jsonp("jsonpCallback({\"data\":{\"rows\":[]}});").unwrap();
        assert!(v.get("data").is_some());
        assert!(strip_jsonp("not jsonp").is_err());
    }
}
