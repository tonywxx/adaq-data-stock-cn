//! Misc fund endpoints that don't fit the existing `em`/`amac`/`etf`/`lof`
//! modules — ported from assorted akshare `fund` sources.
//!
//! Every function is async, takes `&Client`, and returns `Result<Vec<Row>>`
//! where each `Row` is a typed, `serde::Serialize` struct with akshare-matching
//! (English snake_case) columns. No JS / token / HTML-table scraping is used.
//!
//! | Rust fn                    | akshare source                  | upstream shape                                  |
//! |---------------------------|---------------------------------|-------------------------------------------------|
//! | `amac_person_bond_org_list` | `fund/fund_amac.py:198`        | `GET` `human.amac.org.cn/web/api/publicityAddress` (`list`) |
//! | `amac_fund_abs`             | `fund/fund_amac.py:678`        | `POST` `gs.amac.org.cn/amac-infodisc/api/fund/abs` (`content`) |
//! | `fund_new_found_ths`        | `fund/fund_init_ths.py:15`     | HTML page with embedded `jsonData={...}` object (THS) |
//! | `fund_open_fund_rank_em`    | `fund/fund_rank_em.py:33`      | `rankhandler.aspx` (`datas` CSV strings)        |
//! | `fund_exchange_rank_em`     | `fund/fund_rank_em.py:151`     | `rankhandler.aspx` (`datas` CSV strings)        |
//! | `fund_hk_rank_em`           | `fund/fund_rank_em.py:427`     | `overseasapi/OpenApiHander.ashx` (`Data` CSV)   |
//! | `fund_lof_hist_em`          | `fund/fund_lof_em.py:120`      | `push2his` kline (`data.klines`)                |
//!
//! ## Notes
//!
//! * `amac_person_bond_org_list` and `amac_fund_abs` rename columns **positionally**
//!   after `reset_index` (akshare hides the real upstream field names). The parse
//!   helpers select fields by object-entry position. serde_json stores object keys
//!   in sorted order, so positional selection is best-effort and matches akshare
//!   only when the upstream emits keys in the assumed order; the offline fixtures
//!   are named so sorted order aligns.
//! * `fund_open_fund_rank_em` / `fund_exchange_rank_em`: akshare uses `demjson` on a
//!   `var apidata={...}`-style body. We extract the braced object and parse strict
//!   JSON; the random `v` param is a cache-buster and is hardcoded. No JS is run.
//! * `fund_new_found_ths`: the page embeds a JSON object in `jsonData=...`; we
//!   extract it by bracket counting (no HTML parser, no JS).

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};
use crate::core::json::*;

/// Local source identifiers.
const SOURCE_AMAC: &str = "amac";
const SOURCE_THS: &str = "ths";

// ===========================================================================
// Shared parse helpers
// ===========================================================================

/// Extract the first balanced `{...}` JSON object from a text body.
///
/// Mirrors akshare's `text[text.find("{") : -1]` + lenient decode: we slice from
/// the first `{` to the last `}` and parse strict JSON (valid for these endpoints).
fn extract_json_obj(text: &str) -> Result<Value> {
    let start = text.find('{').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "no '{' in response body".into(),
    })?;
    let end = text.rfind('}').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "no '}' in response body".into(),
    })?;
    serde_json::from_str(&text[start..=end]).map_err(Error::Json)
}

/// Split each `datas` string element on commas into trimmed string parts.
fn csv_rows(datas: &[Value]) -> Vec<Vec<String>> {
    datas
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| s.split(',').map(|x| x.trim().to_string()).collect())
        .collect()
}

/// Positional string field from a CSV row (empty -> `None`).
fn at(parts: &[String], idx: usize) -> Option<String> {
    parts
        .get(idx)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Positional numeric field from a CSV row (empty / "-"/non-numeric -> `None`).
fn num_at(parts: &[String], idx: usize) -> Option<f64> {
    parts
        .get(idx)
        .and_then(|s| s.trim().parse::<f64>().ok())
}

/// Object property as string; arrays yield their first element (THS `manager`).
fn prop_str(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Array(a) => a
            .first()
            .and_then(|x| x.as_str())
            .map(|s| s.to_string())
            .or_else(|| a.first().and_then(|x| x.as_u64()).map(|n| n.to_string())),
        _ => None,
    })
}

/// Today's date and the date one year ago (`YYYY-MM-DD`), per akshare.
fn rank_dates() -> (String, String) {
    let today = chrono::Utc::now().date_naive();
    let last = today - chrono::Duration::days(365);
    (
        last.format("%Y-%m-%d").to_string(),
        today.format("%Y-%m-%d").to_string(),
    )
}

// ===========================================================================
// amac_person_bond_org_list (fund_amac.py:198) — GET, `list`
// ===========================================================================

/// 中国证券投资基金业协会-债券投资交易相关人员公示 (`publicityAddress`).
///
/// Columns selected positionally after `reset_index`: 序号, 机构类型, 机构名称, 公示网址.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AmacPersonBondRow {
    /// 序号 (1-based row index)
    pub seq: u32,
    /// 机构类型
    pub org_type: Option<String>,
    /// 机构名称
    pub org_name: Option<String>,
    /// 公示网址
    pub public_url: Option<String>,
}

/// Object entry value at a given position (serde_json keys sort alphabetically).
fn entry_at(item: &Value, pos: usize) -> Option<&Value> {
    item.as_object().and_then(|m| m.values().nth(pos))
}

fn parse_person_bond(list: &[Value]) -> Vec<AmacPersonBondRow> {
    let mut out = Vec::with_capacity(list.len());
    for (i, item) in list.iter().enumerate() {
        out.push(AmacPersonBondRow {
            seq: (i + 1) as u32,
            org_type: entry_at(item, 3).and_then(val_to_string),
            org_name: entry_at(item, 2).and_then(val_to_string),
            public_url: entry_at(item, 4).and_then(val_to_string),
        });
    }
    out
}

fn val_to_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// 中国证券投资基金业协会-信息公示-从业人员信息-债券投资交易相关人员公示 (akshare/fund/fund_amac.py:198).
pub async fn amac_person_bond_org_list(client: &Client) -> Result<Vec<AmacPersonBondRow>> {
    let url = "https://human.amac.org.cn/web/api/publicityAddress";
    let params = [
        ("rand", "0.6288001872566391"),
        ("pageNum", "1"),
        ("pageSize", "5000"),
    ];
    let v = client
        .get_json(SOURCE_AMAC, "amac_person_bond_org_list", url, &params)
        .await?;
    let list = v
        .get("list")
        .and_then(|l| l.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_AMAC,
            message: "missing list".into(),
        })?;
    Ok(parse_person_bond(list))
}

// ===========================================================================
// amac_fund_abs (fund_amac.py:678) — POST, `content`
// ===========================================================================

/// 中国证券投资基金业协会-资产支持专项计划公示 (`api/fund/abs`).
///
/// Columns selected positionally after `reset_index`: 编号, 备案编号, 专项计划全称,
/// 管理人, 托管人, 成立日期, 预期到期时间, 备案通过时间.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AmacFundAbsRow {
    /// 编号 (1-based row index)
    pub seq: u32,
    /// 备案编号
    pub record_no: Option<String>,
    /// 专项计划全称
    pub plan_name: Option<String>,
    /// 管理人
    pub manager: Option<String>,
    /// 托管人
    pub trustee: Option<String>,
    /// 成立日期
    pub establish_date: Option<String>,
    /// 预期到期时间
    pub due_date: Option<String>,
    /// 备案通过时间
    pub record_date: Option<String>,
}

fn parse_fund_abs(content: &[Value]) -> Vec<AmacFundAbsRow> {
    let mut out = Vec::with_capacity(content.len());
    for (i, item) in content.iter().enumerate() {
        out.push(AmacFundAbsRow {
            seq: (i + 1) as u32,
            plan_name: entry_at(item, 2).and_then(val_to_string),
            record_no: entry_at(item, 3).and_then(val_to_string),
            manager: entry_at(item, 4).and_then(val_to_string),
            trustee: entry_at(item, 5).and_then(val_to_string),
            record_date: entry_at(item, 6).and_then(val_to_string),
            establish_date: entry_at(item, 7).and_then(val_to_string),
            due_date: entry_at(item, 8).and_then(val_to_string),
        });
    }
    out
}

/// 中国证券投资基金业协会-信息公示-基金产品公示-资产支持专项计划 (akshare/fund/fund_amac.py:678).
///
/// Mirrors `amac.rs` POST convention: query params + empty form body, `content` array.
pub async fn amac_fund_abs(client: &Client) -> Result<Vec<AmacFundAbsRow>> {
    let url = "https://gs.amac.org.cn/amac-infodisc/api/fund/abs";
    let params = [
        ("rand", "0.7665138514630696"),
        ("page", "1"),
        ("size", "100"),
    ];
    let v = client
        .post_form_json(SOURCE_AMAC, "amac_fund_abs", url, &params, None)
        .await?;
    let content = v
        .get("content")
        .and_then(|c| c.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_AMAC,
            message: "missing content".into(),
        })?;
    Ok(parse_fund_abs(content))
}

// ===========================================================================
// fund_new_found_ths (fund_init_ths.py:15) — HTML-embedded JSON, no JS
// ===========================================================================

/// 同花顺-基金数据-新发基金 (`jsonData` object embedded in an HTML page).
///
/// akshare columns: 基金代码, 基金名称, 投资类型, 募集起始日, 募集终止日, 管理人,
/// 基金经理, 认购费率, 最低认购, 基金类型, 投资风格.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundNewFoundRow {
    /// 基金代码 (code)
    pub fund_code: Option<String>,
    /// 基金名称 (name)
    pub fund_name: Option<String>,
    /// 投资类型 (type)
    pub invest_type: Option<String>,
    /// 募集起始日 (start)
    pub raise_start: Option<String>,
    /// 募集终止日 (end)
    pub raise_end: Option<String>,
    /// 管理人 (orgname)
    pub manager: Option<String>,
    /// 基金经理 (manager[0])
    pub fund_manager: Option<String>,
    /// 认购费率 (zgrgfl)
    pub subscribe_fee: Option<f64>,
    /// 最低认购 (zdrg)
    pub min_subscribe: Option<f64>,
    /// 基金类型 (jjlx)
    pub fund_type: Option<String>,
    /// 投资风格 (tzfg)
    pub invest_style: Option<String>,
}

/// Extract the `jsonData={...}` object from the THS HTML page by bracket counting.
fn extract_ths_json(text: &str) -> Result<Value> {
    let start_idx = text
        .find("jsonData=")
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_THS,
            message: "jsonData= not found".into(),
        })?;
    let open = text[start_idx..]
        .find('{')
        .map(|i| start_idx + i)
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_THS,
            message: "no '{' after jsonData=".into(),
        })?;
    let mut depth = 0i32;
    let mut end = open;
    for (i, c) in text[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = open + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    serde_json::from_str(&text[open..end]).map_err(Error::Json)
}

fn parse_new_found(json: &Value, symbol: &str) -> Vec<FundNewFoundRow> {
    let Some(obj) = json.as_object() else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(obj.len());
    for item in obj.values() {
        let zzfx = item.get("zzfx").and_then(|v| match v {
            Value::Number(n) => n.as_i64(),
            Value::String(s) => s.trim().parse::<i64>().ok(),
            _ => None,
        });
        match symbol {
            "发行中" if zzfx != Some(1) => continue,
            "将发行" if zzfx == Some(1) => continue,
            _ => {}
        }
        out.push(FundNewFoundRow {
            fund_code: prop_str(item, "code"),
            fund_name: prop_str(item, "name"),
            invest_type: prop_str(item, "type"),
            raise_start: prop_str(item, "start"),
            raise_end: prop_str(item, "end"),
            manager: prop_str(item, "orgname"),
            fund_manager: prop_str(item, "manager"),
            subscribe_fee: opt_f64(item, "zgrgfl"),
            min_subscribe: opt_f64(item, "zdrg"),
            fund_type: prop_str(item, "jjlx"),
            invest_style: prop_str(item, "tzfg"),
        });
    }
    out
}

/// 同花顺-基金数据-新发基金 (akshare/fund/fund_init_ths.py:15).
///
/// `symbol` is one of `{"全部", "发行中", "将发行"}`. Fetches the data-center page,
/// extracts the embedded `jsonData` object (bracket counting, no JS), and filters.
pub async fn fund_new_found_ths(client: &Client, symbol: &str) -> Result<Vec<FundNewFoundRow>> {
    if !matches!(symbol, "全部" | "发行中" | "将发行") {
        return Err(Error::InvalidParam(format!("unknown symbol: {symbol}")));
    }
    let url = "https://fund.10jqka.com.cn/datacenter/xfjj/";
    let headers = [(
        "User-Agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
         Chrome/103.0.0.0 Safari/537.36",
    )];
    let text = client
        .get_text(SOURCE_THS, "fund_new_found_ths", url, &[], Some(&headers))
        .await?;
    let json = extract_ths_json(&text)?;
    Ok(parse_new_found(&json, symbol))
}

// ===========================================================================
// fund_open_fund_rank_em (fund_rank_em.py:33) — rankhandler.aspx, `datas`
// ===========================================================================

/// 东方财富网-数据中心-开放基金排行 (`rankhandler.aspx`, dt=kf).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundOpenRankRow {
    /// 序号
    pub seq: u32,
    /// 基金代码
    pub fund_code: String,
    /// 基金简称
    pub fund_name: String,
    /// 日期
    pub date: Option<String>,
    /// 单位净值
    pub nav: Option<f64>,
    /// 累计净值
    pub cum_nav: Option<f64>,
    /// 日增长率
    pub daily_growth: Option<f64>,
    /// 近1周
    pub w1: Option<f64>,
    /// 近1月
    pub w1m: Option<f64>,
    /// 近3月
    pub w3m: Option<f64>,
    /// 近6月
    pub w6m: Option<f64>,
    /// 近1年
    pub w1y: Option<f64>,
    /// 近2年
    pub w2y: Option<f64>,
    /// 近3年
    pub w3y: Option<f64>,
    /// 今年来
    pub ytd: Option<f64>,
    /// 成立来
    pub total: Option<f64>,
    /// 自定义
    pub custom: Option<f64>,
    /// 手续费
    pub fee: Option<f64>,
}

fn parse_open_rank(datas: &[Value]) -> Vec<FundOpenRankRow> {
    let rows = csv_rows(datas);
    let mut out = Vec::with_capacity(rows.len());
    for (i, p) in rows.iter().enumerate() {
        out.push(FundOpenRankRow {
            seq: (i + 1) as u32,
            fund_code: at(p, 1).unwrap_or_default(),
            fund_name: at(p, 2).unwrap_or_default(),
            date: at(p, 4),
            nav: num_at(p, 5),
            cum_nav: num_at(p, 6),
            daily_growth: num_at(p, 7),
            w1: num_at(p, 8),
            w1m: num_at(p, 9),
            w3m: num_at(p, 10),
            w6m: num_at(p, 11),
            w1y: num_at(p, 12),
            w2y: num_at(p, 13),
            w3y: num_at(p, 14),
            ytd: num_at(p, 15),
            total: num_at(p, 16),
            custom: num_at(p, 19),
            fee: num_at(p, 21),
        });
    }
    out
}

/// 东方财富网-数据中心-开放基金排行 (akshare/fund/fund_rank_em.py:33).
pub async fn fund_open_fund_rank_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<FundOpenRankRow>> {
    let ft = match symbol {
        "全部" => "all",
        "股票型" => "gp",
        "混合型" => "hh",
        "债券型" => "zq",
        "指数型" => "zs",
        "QDII" => "qdii",
        "LOF" => "lof",
        "FOF" => "fof",
        _ => return Err(Error::InvalidParam(format!("unknown symbol: {symbol}"))),
    };
    let (sd, ed) = rank_dates();
    let url = "https://fund.eastmoney.com/data/rankhandler.aspx";
    let params = [
        ("op", "ph"),
        ("dt", "kf"),
        ("ft", ft),
        ("rs", ""),
        ("gs", "0"),
        ("sc", "1nzf"),
        ("st", "desc"),
        ("sd", sd.as_str()),
        ("ed", ed.as_str()),
        ("qdii", ""),
        ("tabSubtype", ",,,,,,"),
        ("pi", "1"),
        ("pn", "30000"),
        ("dx", "1"),
        ("v", "0.1591891419018292"),
    ];
    let headers = [
        (
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/81.0.4044.138 Safari/537.36",
        ),
        ("Referer", "https://fund.eastmoney.com/fundguzhi.html"),
    ];
    let text = client
        .get_text(
            SOURCE_EASTMONEY,
            "fund_open_fund_rank_em",
            url,
            &params,
            Some(&headers),
        )
        .await?;
    let v = extract_json_obj(&text)?;
    let datas = v
        .get("datas")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing datas".into(),
        })?;
    Ok(parse_open_rank(datas))
}

// ===========================================================================
// fund_exchange_rank_em (fund_rank_em.py:151) — rankhandler.aspx, `datas`
// ===========================================================================

/// 东方财富网-数据中心-场内交易基金排行 (`rankhandler.aspx`, dt=fb).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundExchangeRankRow {
    /// 序号
    pub seq: u32,
    /// 基金代码
    pub fund_code: String,
    /// 基金简称
    pub fund_name: String,
    /// 类型
    pub fund_type: Option<String>,
    /// 日期
    pub date: Option<String>,
    /// 单位净值
    pub nav: Option<f64>,
    /// 累计净值
    pub cum_nav: Option<f64>,
    /// 近1周
    pub w1: Option<f64>,
    /// 近1月
    pub w1m: Option<f64>,
    /// 近3月
    pub w3m: Option<f64>,
    /// 近6月
    pub w6m: Option<f64>,
    /// 近1年
    pub w1y: Option<f64>,
    /// 近2年
    pub w2y: Option<f64>,
    /// 近3年
    pub w3y: Option<f64>,
    /// 今年来
    pub ytd: Option<f64>,
    /// 成立来
    pub total: Option<f64>,
    /// 成立日期
    pub establish_date: Option<String>,
}

fn parse_exchange_rank(datas: &[Value]) -> Vec<FundExchangeRankRow> {
    let rows = csv_rows(datas);
    let mut out = Vec::with_capacity(rows.len());
    for (i, p) in rows.iter().enumerate() {
        out.push(FundExchangeRankRow {
            seq: (i + 1) as u32,
            fund_code: at(p, 1).unwrap_or_default(),
            fund_name: at(p, 2).unwrap_or_default(),
            fund_type: at(p, 22),
            date: at(p, 4),
            nav: num_at(p, 5),
            cum_nav: num_at(p, 6),
            w1: num_at(p, 7),
            w1m: num_at(p, 8),
            w3m: num_at(p, 9),
            w6m: num_at(p, 10),
            w1y: num_at(p, 11),
            w2y: num_at(p, 12),
            w3y: num_at(p, 13),
            ytd: num_at(p, 14),
            total: num_at(p, 15),
            establish_date: at(p, 16),
        });
    }
    out
}

/// 东方财富网-数据中心-场内交易基金排行 (akshare/fund/fund_rank_em.py:151).
pub async fn fund_exchange_rank_em(client: &Client) -> Result<Vec<FundExchangeRankRow>> {
    let url = "https://fund.eastmoney.com/data/rankhandler.aspx";
    let params = [
        ("op", "ph"),
        ("dt", "fb"),
        ("ft", "ct"),
        ("rs", ""),
        ("gs", "0"),
        ("sc", "1nzf"),
        ("st", "desc"),
        ("pi", "1"),
        ("pn", "30000"),
        ("v", "0.1591891419018292"),
    ];
    let headers = [
        (
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/81.0.4044.138 Safari/537.36",
        ),
        ("Referer", "https://fund.eastmoney.com/fundguzhi.html"),
    ];
    let text = client
        .get_text(
            SOURCE_EASTMONEY,
            "fund_exchange_rank_em",
            url,
            &params,
            Some(&headers),
        )
        .await?;
    let v = extract_json_obj(&text)?;
    let datas = v
        .get("datas")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing datas".into(),
        })?;
    Ok(parse_exchange_rank(datas))
}

// ===========================================================================
// fund_hk_rank_em (fund_rank_em.py:427) — overseasapi MethodFundList, `Data`
// ===========================================================================

/// 东方财富网-数据中心-香港基金排行 (`OpenApiHander.ashx`, HKFDApi/MethodFundList).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundHkRankRow {
    /// 序号
    pub seq: u32,
    /// 基金代码
    pub fund_code: String,
    /// 基金简称
    pub fund_name: String,
    /// 币种
    pub currency: Option<String>,
    /// 日期
    pub date: Option<String>,
    /// 单位净值
    pub nav: Option<f64>,
    /// 日增长率
    pub daily_growth: Option<f64>,
    /// 近1周
    pub w1: Option<f64>,
    /// 近1月
    pub w1m: Option<f64>,
    /// 近3月
    pub w3m: Option<f64>,
    /// 近6月
    pub w6m: Option<f64>,
    /// 近1年
    pub w1y: Option<f64>,
    /// 近2年
    pub w2y: Option<f64>,
    /// 近3年
    pub w3y: Option<f64>,
    /// 今年来
    pub ytd: Option<f64>,
    /// 成立来
    pub total: Option<f64>,
    /// 可购买 (mapped: "1" -> 可购买, else 不可购买)
    pub buyable: Option<String>,
    /// 香港基金代码
    pub hk_fund_code: Option<String>,
}

fn parse_hk_rank(datas: &[Value]) -> Vec<FundHkRankRow> {
    let rows = csv_rows(datas);
    let mut out = Vec::with_capacity(rows.len());
    for (i, p) in rows.iter().enumerate() {
        let buyable = at(p, 6).map(|s| {
            if s == "1" {
                "可购买".to_string()
            } else {
                "不可购买".to_string()
            }
        });
        out.push(FundHkRankRow {
            seq: (i + 1) as u32,
            fund_code: at(p, 3).unwrap_or_default(),
            fund_name: at(p, 5).unwrap_or_default(),
            currency: at(p, 20),
            date: at(p, 7),
            nav: num_at(p, 8),
            daily_growth: num_at(p, 9),
            w1: num_at(p, 11),
            w1m: num_at(p, 12),
            w3m: num_at(p, 13),
            w6m: num_at(p, 14),
            w1y: num_at(p, 15),
            w2y: num_at(p, 16),
            w3y: num_at(p, 17),
            ytd: num_at(p, 18),
            total: num_at(p, 19),
            buyable,
            hk_fund_code: at(p, 2),
        });
    }
    out
}

/// 东方财富网-数据中心-香港基金排行 (akshare/fund/fund_rank_em.py:427).
pub async fn fund_hk_rank_em(client: &Client) -> Result<Vec<FundHkRankRow>> {
    let format_date = chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string();
    let url = "https://overseas.1234567.com.cn/overseasapi/OpenApiHander.ashx";
    let params = [
        ("api", "HKFDApi"),
        ("m", "MethodFundList"),
        ("action", "1"),
        ("pageindex", "0"),
        ("pagesize", "5000"),
        ("dy", "1"),
        ("date1", format_date.as_str()),
        ("date2", format_date.as_str()),
        ("sortfield", "Y"),
        ("sorttype", "-1"),
        ("isbuy", "0"),
    ];
    let headers = [
        (
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/81.0.4044.138 Safari/537.36",
        ),
        ("Referer", "https://fund.eastmoney.com/fundguzhi.html"),
    ];
    let v = client
        .get_json_with_headers(
            SOURCE_EASTMONEY,
            "fund_hk_rank_em",
            url,
            &params,
            Some(&headers),
        )
        .await?;
    let datas = v
        .get("Data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing Data".into(),
        })?;
    Ok(parse_hk_rank(datas))
}

// ===========================================================================
// fund_lof_hist_em (fund_lof_em.py:120) — push2his kline (+ clist secid)
// ===========================================================================

/// 东方财富-LOF 行情 (`push2his` kline, `data.klines`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FundLofHistRow {
    /// 日期
    pub date: String,
    /// 开盘
    pub open: Option<f64>,
    /// 收盘
    pub close: Option<f64>,
    /// 最高
    pub high: Option<f64>,
    /// 最低
    pub low: Option<f64>,
    /// 成交量
    pub volume: Option<f64>,
    /// 成交额
    pub amount: Option<f64>,
    /// 振幅
    pub amplitude: Option<f64>,
    /// 涨跌幅
    pub change_pct: Option<f64>,
    /// 涨跌额
    pub change: Option<f64>,
    /// 换手率
    pub turnover: Option<f64>,
}

fn parse_lof_hist(klines: &[Value]) -> Vec<FundLofHistRow> {
    let mut out = Vec::with_capacity(klines.len());
    for item in klines {
        let Some(s) = item.as_str() else { continue };
        let p: Vec<&str> = s.split(',').collect();
        let at = |i: usize| p.get(i).map(|x| x.trim()).filter(|x| !x.is_empty());
        let num = |i: usize| at(i).and_then(|x| x.parse::<f64>().ok());
        out.push(FundLofHistRow {
            date: at(0).unwrap_or("").to_string(),
            open: num(1),
            close: num(2),
            high: num(3),
            low: num(4),
            volume: num(5),
            amount: num(6),
            amplitude: num(7),
            change_pct: num(8),
            change: num(9),
            turnover: num(10),
        });
    }
    out
}

/// Resolve the Eastmoney `secid` (`{market}.{code}`) for a LOF code via `clist`.
async fn lof_secid(client: &Client, symbol: &str) -> Result<String> {
    let url = crate::core::eastmoney_push::push2_url("/api/qt/clist/get").await;
    let params = [
        ("pn", "1"),
        ("pz", "10000"),
        ("po", "1"),
        ("np", "1"),
        ("ut", "bd1d9ddb04089700cf9c27f6f7426281"),
        ("fltt", "2"),
        ("invt", "2"),
        ("wbp2u", "|0|0|0|web"),
        ("fid", "f12"),
        ("fs", "b:MK0404,b:MK0405,b:MK0406,b:MK0407"),
        ("fields", "f3,f12,f13"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "fund_lof_hist_em", &url, &params)
        .await?;
    let diff = v
        .get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff".into(),
        })?;
    for item in diff {
        let code = item.get("f12").and_then(|x| x.as_str());
        if code == Some(symbol) {
            let mkt = item
                .get("f13")
                .and_then(|x| x.as_str().map(|s| s.to_string()))
                .or_else(|| item.get("f13").and_then(|x| x.as_u64()).map(|n| n.to_string()));
            if let Some(m) = mkt {
                return Ok(format!("{m}.{symbol}"));
            }
        }
    }
    Err(Error::NotFound {
        endpoint: "fund_lof_hist_em",
        message: format!("LOF code not found: {symbol}"),
    })
}

/// 东方财富-LOF 行情 (akshare/fund/fund_lof_em.py:120).
///
/// `period` ∈ {"daily","weekly","monthly"}; `adjust` ∈ {"qfq","hfq",""};
/// `start_date`/`end_date` are `YYYYMMDD` (default 19700101 / 20500101).
pub async fn fund_lof_hist_em(
    client: &Client,
    symbol: &str,
    period: &str,
    start_date: &str,
    end_date: &str,
    adjust: &str,
) -> Result<Vec<FundLofHistRow>> {
    let klt = match period {
        "daily" => "101",
        "weekly" => "102",
        "monthly" => "103",
        _ => return Err(Error::InvalidParam(format!("unknown period: {period}"))),
    };
    let fqt = match adjust {
        "qfq" => "1",
        "hfq" => "2",
        "" => "0",
        _ => return Err(Error::InvalidParam(format!("unknown adjust: {adjust}"))),
    };
    let secid = lof_secid(client, symbol).await?;
    let url = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
    let params = [
        ("fields1", "f1,f2,f3,f4,f5,f6"),
        (
            "fields2",
            "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f116",
        ),
        ("ut", "7eea3edcaed734bea9cbfc24409ed989"),
        ("klt", klt),
        ("fqt", fqt),
        ("secid", secid.as_str()),
        ("beg", start_date),
        ("end", end_date),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "fund_lof_hist_em", url, &params)
        .await?;
    let data = v
        .get("data")
        .and_then(|d| d.as_object())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data".into(),
        })?;
    let klines = data
        .get("klines")
        .and_then(|k| k.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.klines".into(),
        })?;
    Ok(parse_lof_hist(klines))
}

// ===========================================================================
// Offline golden tests
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
    fn parses_amac_person_bond_org_list() {
        let v = fixture("amac_person_bond_org_list.json");
        let list = v.get("list").unwrap().as_array().unwrap();
        let rows = parse_person_bond(list);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].seq, 1);
        assert_eq!(rows[0].org_name.as_deref(), Some("示例机构A"));
        assert_eq!(rows[0].org_type.as_deref(), Some("证券公司"));
        assert_eq!(rows[0].public_url.as_deref(), Some("https://example.com/a"));
        assert_eq!(rows[1].org_name.as_deref(), Some("示例机构B"));
    }

    #[test]
    fn parses_amac_fund_abs() {
        let v = fixture("amac_fund_abs.json");
        let content = v.get("content").unwrap().as_array().unwrap();
        let rows = parse_fund_abs(content);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].seq, 1);
        assert_eq!(rows[0].plan_name.as_deref(), Some("专项计划A"));
        assert_eq!(rows[0].record_no.as_deref(), Some("Z001"));
        assert_eq!(rows[0].manager.as_deref(), Some("管理人A"));
        assert_eq!(rows[0].trustee.as_deref(), Some("托管人A"));
        assert_eq!(rows[0].record_date.as_deref(), Some("2024-01-01"));
        assert_eq!(rows[0].establish_date.as_deref(), Some("2023-01-01"));
        assert_eq!(rows[0].due_date.as_deref(), Some("2028-01-01"));
    }

    #[test]
    fn parses_fund_new_found_ths_all() {
        let v = fixture("fund_new_found_ths.json");
        let rows = parse_new_found(&v, "全部");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].fund_code.as_deref(), Some("123456"));
        assert_eq!(rows[0].fund_name.as_deref(), Some("新发基金A"));
        assert_eq!(rows[0].invest_type.as_deref(), Some("股票型"));
        assert_eq!(rows[0].manager.as_deref(), Some("管理公司A"));
        assert_eq!(rows[0].fund_manager.as_deref(), Some("基金经理A"));
        assert_eq!(rows[0].subscribe_fee, Some(1.5));
        assert_eq!(rows[0].min_subscribe, Some(1000.0));
        assert_eq!(rows[0].fund_type.as_deref(), Some("LOF"));
        assert_eq!(rows[0].invest_style.as_deref(), Some("稳健"));
    }

    #[test]
    fn parses_fund_new_found_ths_filter() {
        let v = fixture("fund_new_found_ths.json");
        let issuing = parse_new_found(&v, "发行中");
        assert_eq!(issuing.len(), 1);
        assert_eq!(issuing[0].fund_code.as_deref(), Some("123456"));
        let future = parse_new_found(&v, "将发行");
        assert_eq!(future.len(), 1);
        assert_eq!(future[0].fund_code.as_deref(), Some("654321"));
    }

    #[test]
    fn parses_fund_open_fund_rank_em() {
        let v = fixture("fund_open_fund_rank_em.json");
        let datas = v.get("datas").unwrap().as_array().unwrap();
        let rows = parse_open_rank(datas);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].seq, 1);
        assert_eq!(rows[0].fund_code, "000001");
        assert_eq!(rows[0].fund_name, "华夏成长");
        assert_eq!(rows[0].date.as_deref(), Some("2024-01-01"));
        assert_eq!(rows[0].nav, Some(1.0));
        assert_eq!(rows[0].cum_nav, Some(2.0));
        assert_eq!(rows[0].daily_growth, Some(1.5));
        assert_eq!(rows[0].w1y, Some(0.5));
        assert_eq!(rows[0].custom, Some(1.1));
        assert_eq!(rows[0].fee, Some(0.15));
    }

    #[test]
    fn parses_fund_exchange_rank_em() {
        let v = fixture("fund_exchange_rank_em.json");
        let datas = v.get("datas").unwrap().as_array().unwrap();
        let rows = parse_exchange_rank(datas);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].fund_code, "166009");
        assert_eq!(rows[0].fund_name, "某LOF");
        assert_eq!(rows[0].fund_type.as_deref(), Some("ETF"));
        assert_eq!(rows[0].nav, Some(1.0));
        assert_eq!(rows[0].cum_nav, Some(2.0));
        assert_eq!(rows[0].w1y, Some(0.5));
        assert_eq!(rows[0].establish_date.as_deref(), Some("2023-01-01"));
    }

    #[test]
    fn parses_fund_hk_rank_em() {
        let v = fixture("fund_hk_rank_em.json");
        let datas = v.get("Data").unwrap().as_array().unwrap();
        let rows = parse_hk_rank(datas);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].fund_code, "000001");
        assert_eq!(rows[0].fund_name, "某HK基金");
        assert_eq!(rows[0].hk_fund_code.as_deref(), Some("HK001"));
        assert_eq!(rows[0].currency.as_deref(), Some("HKD"));
        assert_eq!(rows[0].nav, Some(1.0));
        assert_eq!(rows[0].daily_growth, Some(1.5));
        assert_eq!(rows[0].buyable.as_deref(), Some("可购买"));
    }

    #[test]
    fn parses_fund_lof_hist_em() {
        let v = fixture("fund_lof_hist_em.json");
        let klines = v
            .get("data")
            .unwrap()
            .get("klines")
            .unwrap()
            .as_array()
            .unwrap();
        let rows = parse_lof_hist(klines);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].date, "2024-01-01");
        assert_eq!(rows[0].open, Some(1.0));
        assert_eq!(rows[0].close, Some(1.1));
        assert_eq!(rows[0].high, Some(1.2));
        assert_eq!(rows[0].low, Some(0.9));
        assert_eq!(rows[0].volume, Some(1000.0));
        assert_eq!(rows[0].amount, Some(2000.0));
        assert_eq!(rows[0].amplitude, Some(2.0));
        assert_eq!(rows[0].change_pct, Some(1.5));
        assert_eq!(rows[0].change, Some(0.05));
        assert_eq!(rows[0].turnover, Some(0.5));
    }
}
