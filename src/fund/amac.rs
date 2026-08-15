//! China Asset Management Association (AMAC, 中国证券投资基金业协会) public
//! disclosure data — akshare `akshare/fund/fund_amac.py` ported to pure-HTTP Rust.
//!
//! Every ported function hits an AMAC `gs.amac.org.cn` JSON API via `POST`
//! (query params `rand`/`page`/`size`, empty `{}` body) and returns the
//! `content` array of row objects. All are async and take `&Client`.
//!
//! Provenance / mapping for this akshare checkout (`/Users/tony/github/akshare`):
//!
//! | Rust fn                     | akshare source line | upstream shape            |
//! |-----------------------------|---------------------|---------------------------|
//! | `amac_member_info`          | fund_amac.py:44     | POST, `content`           |
//! | `amac_person_fund_org_list` | fund_amac.py:96     | POST, `content` (+orgType)|
//! | `amac_manager_info`         | fund_amac.py:240    | POST, `content`           |
//! | `amac_manager_classify_info`| fund_amac.py:294    | POST, `content`           |
//! | `amac_member_sub_info`      | fund_amac.py:365    | POST, `content`           |
//! | `amac_fund_info`            | fund_amac.py:415    | POST, `content`           |
//! | `amac_securities_info`      | fund_amac.py:476    | POST, `content`           |
//! | `amac_aoin_info`            | fund_amac.py:530    | POST, `content`           |
//! | `amac_fund_sub_info`        | fund_amac.py:577    | POST, `content`           |
//! | `amac_fund_account_info`    | fund_amac.py:629    | POST, `content`           |
//! | `amac_futures_info`         | fund_amac.py:737    | POST, `content`           |
//! | `amac_manager_cancelled_info`| fund_amac.py:792   | POST, `content`           |
//!
//! NOTE on pagination: akshare loops over every `totalPages` page to assemble
//! the full table. Mirroring the convention used elsewhere in this crate (see
//! `fund/extra.rs`), the async fns here fetch only the **first** page
//! (`page=0`/`page=1`); pagination is intentionally not implemented. The
//! `parse_*` helpers only ever handle a single `content` payload, so the
//! offline fixtures store one response page.
//!
//! DEFERRED (not ported, with reason):
//! - `amac_person_bond_org_list` (fund_amac.py:198) — `GET` to a *different* host
//!   (`human.amac.org.cn`) using a custom TLS context
//!   (`ssl.OP_LEGACY_SERVER_CONNECT`); the client's standard TLS may fail to
//!   connect, and the response columns are selected *positionally* after
//!   `reset_index`, so the real JSON field names are not exposed in akshare.
//! - `amac_fund_abs` (fund_amac.py:678) — also renames columns *positionally*
//!   after `reset_index`; the underlying `content` JSON field keys are not
//!   named in the akshare source, so a faithful typed row cannot be derived.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// Local source identifier for the AMAC disclosure API.
const SOURCE_AMAC: &str = "amac";

/// Mirrors the fixed `rand` query param akshare hardcodes (no caching-buster
/// randomness is reproduced client-side).
const RAND: &str = "0.7665138514630696";

// ---------------------------------------------------------------------------
// AMAC endpoints
// ---------------------------------------------------------------------------

const MEMBER_URL: &str = "https://gs.amac.org.cn/amac-infodisc/api/pof/member";
const PERSON_ORG_URL: &str = "https://gs.amac.org.cn/amac-infodisc/api/pof/personOrg";
const MANAGER_URL: &str = "https://gs.amac.org.cn/amac-infodisc/api/pof/manager";
const POF_MEMBER_URL: &str = "https://gs.amac.org.cn/amac-infodisc/api/pof/pofMember";
const FUND_URL: &str = "https://gs.amac.org.cn/amac-infodisc/api/pof/fund";
const SECURITIES_URL: &str = "https://gs.amac.org.cn/amac-infodisc/api/pof/securities";
const AOIN_URL: &str = "https://gs.amac.org.cn/amac-infodisc/api/aoin/product";
const SUBFUND_URL: &str = "https://gs.amac.org.cn/amac-infodisc/api/pof/subfund";
const FUND_ACCOUNT_URL: &str = "https://gs.amac.org.cn/amac-infodisc/api/fund/account";
const FUTURES_URL: &str = "https://gs.amac.org.cn/amac-infodisc/api/pof/futures";
const CANCELLED_URL: &str = "https://gs.amac.org.cn/amac-infodisc/api/cancelled/manager";

// ---------------------------------------------------------------------------
// helpers (defined privately; verbatim per porting spec)
// ---------------------------------------------------------------------------

fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}
fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}
#[allow(dead_code)]
fn inum(item: &Value, k: &str) -> Option<i64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    })
}

/// Extract the `content` array from an AMAC `{"content":[...], ...}` payload.
fn content_array(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("content")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_AMAC,
            message: "missing content array".into(),
        })
}

/// Extract an optional boolean field (e.g. AMAC's `hasSpecialTips` flags).
fn fbool(item: &Value, k: &str) -> Option<bool> {
    item.get(k).and_then(|v| v.as_bool())
}

// ---------------------------------------------------------------------------
// amac_member_info — 会员机构综合查询
// ---------------------------------------------------------------------------

/// AMAC member institution (akshare `amac_member_info`).
///
/// akshare columns: 机构（会员）名称, 会员代表, 会员类型, 会员编号, 入会时间,
/// 机构类型, 是否星标.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AmacMemberInfoRow {
    /// akshare 机构（会员）名称 (managerName)
    pub manager_name: Option<String>,
    /// akshare 会员代表 (memberBehalf)
    pub member_representative: Option<String>,
    /// akshare 会员类型 (memberType)
    pub member_type: Option<String>,
    /// akshare 会员编号 (memberCode)
    pub member_code: Option<String>,
    /// akshare 入会时间 (memberDate, millisecond epoch)
    pub member_date: Option<f64>,
    /// akshare 机构类型 (primaryInvestType)
    pub primary_invest_type: Option<String>,
    /// akshare 是否星标 (markStar)
    pub mark_star: Option<String>,
}

/// AMAC member institution comprehensive query (`amac_member_info`).
pub async fn amac_member_info(client: &Client) -> Result<Vec<AmacMemberInfoRow>> {
    let params = [("rand", RAND), ("page", "1"), ("size", "20")];
    let v = client
        .post_form_json(SOURCE_AMAC, "amac_member_info", MEMBER_URL, &params, None)
        .await?;
    parse_member_info(&v)
}

pub(crate) fn parse_member_info(resp: &Value) -> Result<Vec<AmacMemberInfoRow>> {
    let data = content_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(AmacMemberInfoRow {
            manager_name: fstr(item, "managerName"),
            member_representative: fstr(item, "memberBehalf"),
            member_type: fstr(item, "memberType"),
            member_code: fstr(item, "memberCode"),
            member_date: fnum(item, "memberDate"),
            primary_invest_type: fstr(item, "primaryInvestType"),
            mark_star: fstr(item, "markStar"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// amac_person_fund_org_list — 基金从业人员资格注册信息
// ---------------------------------------------------------------------------

/// Symbol → AMAC `orgType` code (akshare `symbol_map`).
const PERSON_ORG_SYMBOL_MAP: &[(&str, &str)] = &[
    ("保险公司子公司", "bxgszgs"),
    ("期货公司资管子公司", "qhgszgzgs"),
    ("公募基金管理公司资管子公司", "gmjjglgszgzgs"),
    ("商业银行", "syyh"),
    ("交易所", "jys"),
    ("证券公司私募基金子公司", "zqgssmjjzgs"),
    ("地方自律组织", "dfzlzz"),
    ("证券公司", "zqgs"),
    ("评价机构", "pjjg"),
    ("独立第三方销售机构", "dldsfxsjg"),
    ("证券投资咨询机构", "zqtzzxjg"),
    ("外资私募证券基金管理人", "wzsmzqjjglr"),
    ("境外机构", "jwjg"),
    ("证券公司子公司", "zqgszgs"),
    ("公募基金管理公司", "gmjjglgs"),
    ("媒体机构", "mtjg"),
    ("支付结算", "zfjs"),
    ("证券公司资管子公司", "zqgszgzgs"),
    ("会计师事务所", "kjssws"),
    ("独立服务机构", "dlfwjg"),
    ("律师事务所", "lssws"),
    ("期货公司", "qhgs"),
    ("保险公司", "bxgs"),
    ("其他", "qt"),
    ("外包服务机构", "wbfwjg"),
    ("私募基金管理人", "smjjglr"),
];

/// Fund-practitioner registration record (akshare `amac_person_fund_org_list`).
///
/// akshare columns: 机构名称, 机构类型, 员工人数, 基金从业资格, 基金销售业务资格,
/// 投资经理, 基金经理.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AmacPersonFundOrgRow {
    /// requested `symbol` echoed back (akshare param)
    pub symbol: String,
    /// akshare 机构名称 (orgName)
    pub org_name: Option<String>,
    /// akshare 机构类型 (orgType)
    pub org_type: Option<String>,
    /// akshare 员工人数 (workerTotalNum)
    pub worker_total_num: Option<f64>,
    /// akshare 基金从业资格 (operNum)
    pub qualification_num: Option<f64>,
    /// akshare 基金销售业务资格 (salesmanNum)
    pub salesman_num: Option<f64>,
    /// akshare 投资经理 (investmentManagerNum)
    pub investment_manager_num: Option<f64>,
    /// akshare 基金经理 (fundManagerNum)
    pub fund_manager_num: Option<f64>,
}

/// Fund-practitioner qualification registration by org type
/// (`amac_person_fund_org_list`).
///
/// NOTE: akshare sends `orgType` in the JSON *body*; the client's
/// `post_form_json` only attaches query params, so `orgType` is passed as a
/// query parameter here (the upstream reads it either way for this endpoint).
pub async fn amac_person_fund_org_list(
    client: &Client,
    symbol: &str,
) -> Result<Vec<AmacPersonFundOrgRow>> {
    let org_type = person_org_symbol_map(symbol)?;
    let params = [
        ("rand", RAND),
        ("page", "1"),
        ("size", "20"),
        ("orgType", org_type),
    ];
    let v = client
        .post_form_json(
            SOURCE_AMAC,
            "amac_person_fund_org_list",
            PERSON_ORG_URL,
            &params,
            None,
        )
        .await?;
    parse_person_fund_org_list(&v, symbol)
}

pub(crate) fn parse_person_fund_org_list(
    resp: &Value,
    symbol: &str,
) -> Result<Vec<AmacPersonFundOrgRow>> {
    let data = content_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(AmacPersonFundOrgRow {
            symbol: symbol.to_string(),
            org_name: fstr(item, "orgName"),
            org_type: fstr(item, "orgType"),
            worker_total_num: fnum(item, "workerTotalNum"),
            qualification_num: fnum(item, "operNum"),
            salesman_num: fnum(item, "salesmanNum"),
            investment_manager_num: fnum(item, "investmentManagerNum"),
            fund_manager_num: fnum(item, "fundManagerNum"),
        });
    }
    Ok(out)
}

fn person_org_symbol_map(symbol: &str) -> Result<&'static str> {
    PERSON_ORG_SYMBOL_MAP
        .iter()
        .find(|(k, _)| *k == symbol)
        .map(|(_, v)| *v)
        .ok_or_else(|| Error::InvalidParam(format!("unknown person org symbol: {symbol}")))
}

// ---------------------------------------------------------------------------
// amac_manager_info — 私募基金管理人综合查询
// ---------------------------------------------------------------------------

/// Private-fund manager comprehensive record (akshare `amac_manager_info`).
///
/// akshare columns: 私募基金管理人名称, 法定代表人/执行事务合伙人(委派代表)姓名,
/// 机构类型, 注册地, 登记编号, 成立时间, 登记时间.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AmacManagerInfoRow {
    /// akshare 私募基金管理人名称 (managerName)
    pub manager_name: Option<String>,
    /// akshare 法定代表人/执行事务合伙人(委派代表)姓名 (artificialPersonName)
    pub artificial_person_name: Option<String>,
    /// akshare 机构类型 (primaryInvestType)
    pub primary_invest_type: Option<String>,
    /// akshare 注册地 (registerProvince)
    pub register_province: Option<String>,
    /// akshare 登记编号 (registerNo)
    pub register_no: Option<String>,
    /// akshare 成立时间 (establishDate, millisecond epoch)
    pub establish_date: Option<f64>,
    /// akshare 登记时间 (registerDate, millisecond epoch)
    pub register_date: Option<f64>,
}

/// Private-fund manager comprehensive query (`amac_manager_info`).
pub async fn amac_manager_info(client: &Client) -> Result<Vec<AmacManagerInfoRow>> {
    let params = [("rand", RAND), ("page", "1"), ("size", "100")];
    let v = client
        .post_form_json(SOURCE_AMAC, "amac_manager_info", MANAGER_URL, &params, None)
        .await?;
    parse_manager_info(&v)
}

pub(crate) fn parse_manager_info(resp: &Value) -> Result<Vec<AmacManagerInfoRow>> {
    let data = content_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(AmacManagerInfoRow {
            manager_name: fstr(item, "managerName"),
            artificial_person_name: fstr(item, "artificialPersonName"),
            primary_invest_type: fstr(item, "primaryInvestType"),
            register_province: fstr(item, "registerProvince"),
            register_no: fstr(item, "registerNo"),
            establish_date: fnum(item, "establishDate"),
            register_date: fnum(item, "registerDate"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// amac_manager_classify_info — 私募基金管理人分类公示
// ---------------------------------------------------------------------------

/// Private-fund manager classified record (akshare `amac_manager_classify_info`).
///
/// akshare columns: 私募基金管理人名称, 法定代表人/执行事务合伙人(委派代表)姓名,
/// 机构类型, 登记编号, 注册地, 办公地, 成立时间, 登记时间, 在管基金数量,
/// 会员类型, 是否有提示信息, 是否有诚信信息.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AmacManagerClassifyRow {
    /// akshare 私募基金管理人名称 (managerName)
    pub manager_name: Option<String>,
    /// akshare 法定代表人/执行事务合伙人(委派代表)姓名 (artificialPersonName)
    pub artificial_person_name: Option<String>,
    /// akshare 机构类型 (primaryInvestType)
    pub primary_invest_type: Option<String>,
    /// akshare 登记编号 (registerNo)
    pub register_no: Option<String>,
    /// akshare 注册地 (registerProvince)
    pub register_province: Option<String>,
    /// akshare 办公地 (officeAdrAgg)
    pub office_address: Option<String>,
    /// akshare 成立时间 (establishDate, millisecond epoch)
    pub establish_date: Option<f64>,
    /// akshare 登记时间 (registerDate, millisecond epoch)
    pub register_date: Option<f64>,
    /// akshare 在管基金数量 (fundCount)
    pub fund_count: Option<f64>,
    /// akshare 会员类型 (memberType)
    pub member_type: Option<String>,
    /// akshare 是否有提示信息 (hasSpecialTips)
    pub has_special_tips: Option<bool>,
    /// akshare 是否有诚信信息 (hasCreditTips)
    pub has_credit_tips: Option<bool>,
}

/// Private-fund manager classified disclosure (`amac_manager_classify_info`).
pub async fn amac_manager_classify_info(
    client: &Client,
) -> Result<Vec<AmacManagerClassifyRow>> {
    let params = [("rand", RAND), ("page", "1"), ("size", "100")];
    let v = client
        .post_form_json(
            SOURCE_AMAC,
            "amac_manager_classify_info",
            MANAGER_URL,
            &params,
            None,
        )
        .await?;
    parse_manager_classify_info(&v)
}

pub(crate) fn parse_manager_classify_info(resp: &Value) -> Result<Vec<AmacManagerClassifyRow>> {
    let data = content_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(AmacManagerClassifyRow {
            manager_name: fstr(item, "managerName"),
            artificial_person_name: fstr(item, "artificialPersonName"),
            primary_invest_type: fstr(item, "primaryInvestType"),
            register_no: fstr(item, "registerNo"),
            register_province: fstr(item, "registerProvince"),
            office_address: fstr(item, "officeAdrAgg"),
            establish_date: fnum(item, "establishDate"),
            register_date: fnum(item, "registerDate"),
            fund_count: fnum(item, "fundCount"),
            member_type: fstr(item, "memberType"),
            has_special_tips: fbool(item, "hasSpecialTips"),
            has_credit_tips: fbool(item, "hasCreditTips"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// amac_member_sub_info — 证券公司私募基金子公司管理人信息公示
// ---------------------------------------------------------------------------

/// Securities-firm private-fund subsidiary manager record
/// (akshare `amac_member_sub_info`).
///
/// akshare columns: 机构（会员）名称, 会员代表, 会员类型, 会员编号, 入会时间, 公司类型.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AmacMemberSubInfoRow {
    /// akshare 机构（会员）名称 (managerName)
    pub manager_name: Option<String>,
    /// akshare 会员代表 (memberBehalf)
    pub member_representative: Option<String>,
    /// akshare 会员类型 (memberType)
    pub member_type: Option<String>,
    /// akshare 会员编号 (memberCode)
    pub member_code: Option<String>,
    /// akshare 入会时间 (memberDate, millisecond epoch)
    pub member_date: Option<f64>,
    /// akshare 公司类型 (primaryInvestType)
    pub company_type: Option<String>,
}

/// Securities-firm private-fund subsidiary manager disclosure
/// (`amac_member_sub_info`).
pub async fn amac_member_sub_info(client: &Client) -> Result<Vec<AmacMemberSubInfoRow>> {
    let params = [("rand", RAND), ("page", "0"), ("size", "20")];
    let v = client
        .post_form_json(SOURCE_AMAC, "amac_member_sub_info", POF_MEMBER_URL, &params, None)
        .await?;
    parse_member_sub_info(&v)
}

pub(crate) fn parse_member_sub_info(resp: &Value) -> Result<Vec<AmacMemberSubInfoRow>> {
    let data = content_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(AmacMemberSubInfoRow {
            manager_name: fstr(item, "managerName"),
            member_representative: fstr(item, "memberBehalf"),
            member_type: fstr(item, "memberType"),
            member_code: fstr(item, "memberCode"),
            member_date: fnum(item, "memberDate"),
            company_type: fstr(item, "primaryInvestType"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// amac_fund_info — 私募基金管理人基金产品
// ---------------------------------------------------------------------------

/// Private-fund product record (akshare `amac_fund_info`).
///
/// akshare columns: 基金名称, 私募基金管理人名称, 私募基金管理人类型, 运行状态,
/// 备案时间, 建立时间, 托管人名称.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AmacFundInfoRow {
    /// akshare 基金名称 (fundName)
    pub fund_name: Option<String>,
    /// akshare 私募基金管理人名称 (managerName)
    pub manager_name: Option<String>,
    /// akshare 私募基金管理人类型 (managerType)
    pub manager_type: Option<String>,
    /// akshare 运行状态 (workingState)
    pub working_state: Option<String>,
    /// akshare 备案时间 (putOnRecordDate, millisecond epoch)
    pub put_on_record_date: Option<f64>,
    /// akshare 建立时间 (establishDate, millisecond epoch)
    pub establish_date: Option<f64>,
    /// akshare 托管人名称 (mandatorName)
    pub mandator_name: Option<String>,
}

/// Private-fund manager products (`amac_fund_info`).
///
/// `start_page`/`end_page` are accepted to mirror akshare but **pagination is
/// not implemented** — only the first page is fetched (see module note).
pub async fn amac_fund_info(
    client: &Client,
    _start_page: &str,
    _end_page: &str,
) -> Result<Vec<AmacFundInfoRow>> {
    let params = [("rand", RAND), ("page", "1"), ("size", "100")];
    let v = client
        .post_form_json(SOURCE_AMAC, "amac_fund_info", FUND_URL, &params, None)
        .await?;
    parse_fund_info(&v)
}

pub(crate) fn parse_fund_info(resp: &Value) -> Result<Vec<AmacFundInfoRow>> {
    let data = content_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(AmacFundInfoRow {
            fund_name: fstr(item, "fundName"),
            manager_name: fstr(item, "managerName"),
            manager_type: fstr(item, "managerType"),
            working_state: fstr(item, "workingState"),
            put_on_record_date: fnum(item, "putOnRecordDate"),
            establish_date: fnum(item, "establishDate"),
            mandator_name: fstr(item, "mandatorName"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// amac_securities_info — 证券公司集合资管产品公示
// ---------------------------------------------------------------------------

/// Securities-firm pooled asset-management product record
/// (akshare `amac_securities_info`).
///
/// akshare columns: 产品名称, 产品编码, 管理人名称, 成立日期, 到期时间, 投资类型,
/// 是否分级, 托管人名称, 备案日期, 运作状态.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AmacSecuritiesInfoRow {
    /// akshare 产品名称 (cpmc)
    pub product_name: Option<String>,
    /// akshare 产品编码 (cpbm)
    pub product_code: Option<String>,
    /// akshare 管理人名称 (gljg)
    pub manager_name: Option<String>,
    /// akshare 成立日期 (slrq, millisecond epoch)
    pub establish_date: Option<f64>,
    /// akshare 到期时间 (dqr, millisecond epoch)
    pub due_date: Option<f64>,
    /// akshare 投资类型 (tzlx)
    pub invest_type: Option<String>,
    /// akshare 是否分级 (sffj)
    pub is_graded: Option<String>,
    /// akshare 托管人名称 (tgjg)
    pub trustee: Option<String>,
    /// akshare 备案日期 (barq, millisecond epoch)
    pub record_date: Option<f64>,
    /// akshare 运作状态 (yzzt)
    pub working_status: Option<String>,
}

/// Securities-firm pooled asset-management product disclosure
/// (`amac_securities_info`).
pub async fn amac_securities_info(client: &Client) -> Result<Vec<AmacSecuritiesInfoRow>> {
    let params = [("rand", RAND), ("page", "0"), ("size", "20")];
    let v = client
        .post_form_json(SOURCE_AMAC, "amac_securities_info", SECURITIES_URL, &params, None)
        .await?;
    parse_securities_info(&v)
}

pub(crate) fn parse_securities_info(resp: &Value) -> Result<Vec<AmacSecuritiesInfoRow>> {
    let data = content_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(AmacSecuritiesInfoRow {
            product_name: fstr(item, "cpmc"),
            product_code: fstr(item, "cpbm"),
            manager_name: fstr(item, "gljg"),
            establish_date: fnum(item, "slrq"),
            due_date: fnum(item, "dqr"),
            invest_type: fstr(item, "tzlx"),
            is_graded: fstr(item, "sffj"),
            trustee: fstr(item, "tgjg"),
            record_date: fnum(item, "barq"),
            working_status: fstr(item, "yzzt"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// amac_aoin_info — 证券公司直投基金
// ---------------------------------------------------------------------------

/// Securities-firm direct-investment fund record (akshare `amac_aoin_info`).
///
/// akshare columns: 产品编码, 产品名称, 直投子公司, 管理机构, 设立日期.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AmacAoinInfoRow {
    /// akshare 产品编码 (code)
    pub code: Option<String>,
    /// akshare 产品名称 (name)
    pub name: Option<String>,
    /// akshare 直投子公司 (aoinName)
    pub aoin_name: Option<String>,
    /// akshare 管理机构 (managerName)
    pub manager_name: Option<String>,
    /// akshare 设立日期 (createDate, millisecond epoch)
    pub create_date: Option<f64>,
}

/// Securities-firm direct-investment fund disclosure (`amac_aoin_info`).
pub async fn amac_aoin_info(client: &Client) -> Result<Vec<AmacAoinInfoRow>> {
    let params = [("rand", RAND), ("page", "0"), ("size", "20")];
    let v = client
        .post_form_json(SOURCE_AMAC, "amac_aoin_info", AOIN_URL, &params, None)
        .await?;
    parse_aoin_info(&v)
}

pub(crate) fn parse_aoin_info(resp: &Value) -> Result<Vec<AmacAoinInfoRow>> {
    let data = content_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(AmacAoinInfoRow {
            code: fstr(item, "code"),
            name: fstr(item, "name"),
            aoin_name: fstr(item, "aoinName"),
            manager_name: fstr(item, "managerName"),
            create_date: fnum(item, "createDate"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// amac_fund_sub_info — 证券公司私募投资基金
// ---------------------------------------------------------------------------

/// Securities-firm private investment fund record (akshare `amac_fund_sub_info`).
///
/// akshare columns: 产品编码, 产品名称, 私募基金管理人名称, 托管人名称, 成立日期,
/// 备案日期.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AmacFundSubInfoRow {
    /// akshare 产品编码 (productCode)
    pub product_code: Option<String>,
    /// akshare 产品名称 (productName)
    pub product_name: Option<String>,
    /// akshare 私募基金管理人名称 (mgrName)
    pub manager_name: Option<String>,
    /// akshare 托管人名称 (trustee)
    pub trustee: Option<String>,
    /// akshare 成立日期 (foundDate, millisecond epoch)
    pub found_date: Option<f64>,
    /// akshare 备案日期 (registeredDate, millisecond epoch)
    pub registered_date: Option<f64>,
}

/// Securities-firm private investment fund disclosure (`amac_fund_sub_info`).
pub async fn amac_fund_sub_info(client: &Client) -> Result<Vec<AmacFundSubInfoRow>> {
    let params = [("rand", RAND), ("page", "0"), ("size", "20")];
    let v = client
        .post_form_json(SOURCE_AMAC, "amac_fund_sub_info", SUBFUND_URL, &params, None)
        .await?;
    parse_fund_sub_info(&v)
}

pub(crate) fn parse_fund_sub_info(resp: &Value) -> Result<Vec<AmacFundSubInfoRow>> {
    let data = content_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(AmacFundSubInfoRow {
            product_code: fstr(item, "productCode"),
            product_name: fstr(item, "productName"),
            manager_name: fstr(item, "mgrName"),
            trustee: fstr(item, "trustee"),
            found_date: fnum(item, "foundDate"),
            registered_date: fnum(item, "registeredDate"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// amac_fund_account_info — 基金公司及子公司集合资管产品公示
// ---------------------------------------------------------------------------

/// Fund-company pooled asset-management product record
/// (akshare `amac_fund_account_info`).
///
/// akshare columns: 成立日期, 产品编码, 产品名称, 管理人名称.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AmacFundAccountInfoRow {
    /// akshare 成立日期 (registerDate, millisecond epoch)
    pub register_date: Option<f64>,
    /// akshare 产品编码 (registerCode)
    pub register_code: Option<String>,
    /// akshare 产品名称 (name)
    pub name: Option<String>,
    /// akshare 管理人名称 (manager)
    pub manager: Option<String>,
}

/// Fund-company & subsidiary pooled asset-management product disclosure
/// (`amac_fund_account_info`).
pub async fn amac_fund_account_info(client: &Client) -> Result<Vec<AmacFundAccountInfoRow>> {
    let params = [("rand", RAND), ("page", "0"), ("size", "20")];
    let v = client
        .post_form_json(
            SOURCE_AMAC,
            "amac_fund_account_info",
            FUND_ACCOUNT_URL,
            &params,
            None,
        )
        .await?;
    parse_fund_account_info(&v)
}

pub(crate) fn parse_fund_account_info(resp: &Value) -> Result<Vec<AmacFundAccountInfoRow>> {
    let data = content_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(AmacFundAccountInfoRow {
            register_date: fnum(item, "registerDate"),
            register_code: fstr(item, "registerCode"),
            name: fstr(item, "name"),
            manager: fstr(item, "manager"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// amac_futures_info — 期货公司集合资管产品公示
// ---------------------------------------------------------------------------

/// Futures-company pooled asset-management product record
/// (akshare `amac_futures_info`).
///
/// akshare columns: 产品名称, 产品编码, 管理人名称, 托管人名称, 成立日期, 投资类型,
/// 是否分级, 备案日期, 到期日, 运作状态.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AmacFuturesInfoRow {
    /// akshare 产品名称 (mpiName)
    pub product_name: Option<String>,
    /// akshare 产品编码 (mpiProductCode)
    pub product_code: Option<String>,
    /// akshare 管理人名称 (aoiName)
    pub manager_name: Option<String>,
    /// akshare 托管人名称 (mpiTrustee)
    pub trustee: Option<String>,
    /// akshare 成立日期 (mpiCreateDate, millisecond epoch)
    pub create_date: Option<f64>,
    /// akshare 投资类型 (tzlx)
    pub invest_type: Option<String>,
    /// akshare 是否分级 (sfjgh)
    pub is_graded: Option<String>,
    /// akshare 备案日期 (registeredDate, millisecond epoch)
    pub registered_date: Option<f64>,
    /// akshare 到期日 (dueDate, millisecond epoch)
    pub due_date: Option<f64>,
    /// akshare 运作状态 (fundStatus)
    pub fund_status: Option<String>,
}

/// Futures-company pooled asset-management product disclosure
/// (`amac_futures_info`).
pub async fn amac_futures_info(client: &Client) -> Result<Vec<AmacFuturesInfoRow>> {
    let params = [("rand", RAND), ("page", "0"), ("size", "20")];
    let v = client
        .post_form_json(SOURCE_AMAC, "amac_futures_info", FUTURES_URL, &params, None)
        .await?;
    parse_futures_info(&v)
}

pub(crate) fn parse_futures_info(resp: &Value) -> Result<Vec<AmacFuturesInfoRow>> {
    let data = content_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(AmacFuturesInfoRow {
            product_name: fstr(item, "mpiName"),
            product_code: fstr(item, "mpiProductCode"),
            manager_name: fstr(item, "aoiName"),
            trustee: fstr(item, "mpiTrustee"),
            create_date: fnum(item, "mpiCreateDate"),
            invest_type: fstr(item, "tzlx"),
            is_graded: fstr(item, "sfjgh"),
            registered_date: fnum(item, "registeredDate"),
            due_date: fnum(item, "dueDate"),
            fund_status: fstr(item, "fundStatus"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// amac_manager_cancelled_info — 已注销私募基金管理人名单
// ---------------------------------------------------------------------------

/// Cancelled private-fund manager record (akshare `amac_manager_cancelled_info`).
///
/// akshare columns: 管理人名称, 统一社会信用代码, 登记时间, 注销时间, 注销类型.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AmacManagerCancelledRow {
    /// akshare 管理人名称 (orgName)
    pub org_name: Option<String>,
    /// akshare 统一社会信用代码 (orgCode)
    pub org_code: Option<String>,
    /// akshare 登记时间 (orgSignDate, millisecond epoch)
    pub sign_date: Option<f64>,
    /// akshare 注销时间 (cancelDate, millisecond epoch)
    pub cancel_date: Option<f64>,
    /// akshare 注销类型 (status: 100 主动注销 / 200 依公告注销 / 300 协会注销)
    pub status: Option<f64>,
}

/// Cancelled private-fund manager list (`amac_manager_cancelled_info`).
pub async fn amac_manager_cancelled_info(
    client: &Client,
) -> Result<Vec<AmacManagerCancelledRow>> {
    let params = [("rand", RAND), ("page", "0"), ("size", "20")];
    let v = client
        .post_form_json(
            SOURCE_AMAC,
            "amac_manager_cancelled_info",
            CANCELLED_URL,
            &params,
            None,
        )
        .await?;
    parse_manager_cancelled_info(&v)
}

pub(crate) fn parse_manager_cancelled_info(resp: &Value) -> Result<Vec<AmacManagerCancelledRow>> {
    let data = content_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(AmacManagerCancelledRow {
            org_name: fstr(item, "orgName"),
            org_code: fstr(item, "orgCode"),
            sign_date: fnum(item, "orgSignDate"),
            cancel_date: fnum(item, "cancelDate"),
            status: fnum(item, "status"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

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
    fn parses_amac_member_info() {
        let rows = parse_member_info(&fixture("amac_member_info.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].manager_name.as_deref(), Some("中国证券投资基金业协会"));
        assert_eq!(rows[0].member_code.as_deref(), Some("AMAC001"));
        assert_eq!(rows[0].member_date, Some(1_700_000_000_000.0));
        assert_eq!(rows[0].mark_star.as_deref(), Some("0"));
        assert_eq!(rows[1].primary_invest_type.as_deref(), Some("公募"));
    }

    #[test]
    fn parses_amac_person_fund_org_list() {
        let rows =
            parse_person_fund_org_list(&fixture("amac_person_fund_org_list.json"), "公募基金管理公司")
                .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "公募基金管理公司");
        assert_eq!(rows[0].org_name.as_deref(), Some("华夏基金管理有限公司"));
        assert_eq!(rows[0].worker_total_num, Some(800.0));
        assert_eq!(rows[0].qualification_num, Some(750.0));
        assert_eq!(rows[0].fund_manager_num, Some(55.0));
        assert_eq!(rows[1].salesman_num, Some(20.0));
    }

    #[test]
    fn person_org_symbol_map_rejects_unknown() {
        assert!(person_org_symbol_map("nope").is_err());
        assert_eq!(person_org_symbol_map("公募基金管理公司").unwrap(), "gmjjglgs");
    }

    #[test]
    fn parses_amac_manager_info() {
        let rows = parse_manager_info(&fixture("amac_manager_info.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].manager_name.as_deref(), Some("北京某私募管理有限公司"));
        assert_eq!(rows[0].register_no.as_deref(), Some("P1000001"));
        assert_eq!(rows[0].register_province.as_deref(), Some("北京市"));
        assert_eq!(rows[0].establish_date, Some(1_380_000_000_000.0));
        assert_eq!(rows[0].register_date, Some(1_390_000_000_000.0));
        assert_eq!(rows[1].primary_invest_type.as_deref(), Some("股权投资基金"));
    }

    #[test]
    fn parses_amac_manager_classify_info() {
        let rows = parse_manager_classify_info(&fixture("amac_manager_classify_info.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].manager_name.as_deref(), Some("上海某私募管理有限公司"));
        assert_eq!(rows[0].fund_count, Some(12.0));
        assert_eq!(rows[0].has_special_tips, Some(false));
        assert_eq!(rows[0].has_credit_tips, Some(true));
        assert_eq!(rows[1].office_address.as_deref(), Some("广州市天河区"));
    }

    #[test]
    fn parses_amac_member_sub_info() {
        let rows = parse_member_sub_info(&fixture("amac_member_sub_info.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].manager_name.as_deref(), Some("中信证券私募子公司"));
        assert_eq!(rows[0].member_code.as_deref(), Some("MEM001"));
        assert_eq!(rows[0].company_type.as_deref(), Some("证券子公司"));
        assert_eq!(rows[1].member_representative.as_deref(), Some("郑十"));
    }

    #[test]
    fn parses_amac_fund_info() {
        let rows = parse_fund_info(&fixture("amac_fund_info.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].fund_name.as_deref(), Some("聚宽一号私募证券投资基金"));
        assert_eq!(rows[0].manager_name.as_deref(), Some("聚宽投资"));
        assert_eq!(rows[0].working_state.as_deref(), Some("正在运作"));
        assert_eq!(rows[0].establish_date, Some(1_600_000_000_000.0));
        assert_eq!(rows[1].mandator_name.as_deref(), Some("中信证券"));
    }

    #[test]
    fn parses_amac_securities_info() {
        let rows = parse_securities_info(&fixture("amac_securities_info.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].product_name.as_deref(), Some("国君资管增强一号"));
        assert_eq!(rows[0].product_code.as_deref(), Some("S80001"));
        assert_eq!(rows[0].invest_type.as_deref(), Some("固定收益类"));
        assert_eq!(rows[0].working_status.as_deref(), Some("正常运作"));
        assert_eq!(rows[1].is_graded.as_deref(), Some("是"));
    }

    #[test]
    fn parses_amac_aoin_info() {
        let rows = parse_aoin_info(&fixture("amac_aoin_info.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code.as_deref(), Some("AOIN001"));
        assert_eq!(rows[0].name.as_deref(), Some("中信直投基金"));
        assert_eq!(rows[0].aoin_name.as_deref(), Some("中信直接投资子公司"));
        assert_eq!(rows[0].create_date, Some(1_500_000_000_000.0));
        assert_eq!(rows[1].manager_name.as_deref(), Some("华泰证券"));
    }

    #[test]
    fn parses_amac_fund_sub_info() {
        let rows = parse_fund_sub_info(&fixture("amac_fund_sub_info.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].product_code.as_deref(), Some("S60001"));
        assert_eq!(rows[0].product_name.as_deref(), Some("招商私募投资基金"));
        assert_eq!(rows[0].manager_name.as_deref(), Some("招商资管"));
        assert_eq!(rows[0].trustee.as_deref(), Some("招商银行"));
        assert_eq!(rows[0].registered_date, Some(1_620_000_000_000.0));
        assert_eq!(rows[1].found_date, Some(1_600_000_000_000.0));
    }

    #[test]
    fn parses_amac_fund_account_info() {
        let rows = parse_fund_account_info(&fixture("amac_fund_account_info.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].register_code.as_deref(), Some("F70001"));
        assert_eq!(rows[0].name.as_deref(), Some("易方达集合资管计划"));
        assert_eq!(rows[0].manager.as_deref(), Some("易方达基金"));
        assert_eq!(rows[0].register_date, Some(1_640_000_000_000.0));
        assert_eq!(rows[1].name.as_deref(), Some("南方基金资管产品"));
    }

    #[test]
    fn parses_amac_futures_info() {
        let rows = parse_futures_info(&fixture("amac_futures_info.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].product_name.as_deref(), Some("永安期货资管一号"));
        assert_eq!(rows[0].product_code.as_deref(), Some("F80001"));
        assert_eq!(rows[0].invest_type.as_deref(), Some("混合类"));
        assert_eq!(rows[0].registered_date, Some(1_650_000_000_000.0));
        assert_eq!(rows[0].fund_status.as_deref(), Some("正常"));
        assert_eq!(rows[1].due_date, Some(1_690_000_000_000.0));
    }

    #[test]
    fn parses_amac_manager_cancelled_info() {
        let rows = parse_manager_cancelled_info(&fixture("amac_manager_cancelled_info.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].org_name.as_deref(), Some("某已注销私募公司"));
        assert_eq!(rows[0].org_code.as_deref(), Some("91310000XXXX"));
        assert_eq!(rows[0].status, Some(300.0));
        assert_eq!(rows[0].cancel_date, Some(1_680_000_000_000.0));
        assert_eq!(rows[1].sign_date, Some(1_440_000_000_000.0));
    }
}
