//! Miscellaneous stock-fundamental endpoints that don't fit the Eastmoney
//! `datacenter-web` registration pattern.
//!
//! - `stock_kcb_detail_renewal` / `stock_kcb_renewal` — SSE (Shanghai Stock
//!   Exchange) 科创板 project-detail / project-list query
//!   (`query.sse.com.cn`, `sqlId=SH_XM_LB`). Pure HTTP JSON behind a `Referer`
//!   header; data is in `pageHelp.data` (akshare's `# TODO` stubs read the
//!   wrong key, which is why they never returned anything).
//! - `stock_notice_report` / `stock_individual_notice_report` — Eastmoney
//!   `np-anotice-stock` announcement API (`data.list`).
//!
//! Source of truth:
//! `akshare/stock_fundamental/stock_kcb_detail_sse.py:14`,
//! `stock_kcb_sse.py:14`, `stock_notice.py:133,151`.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};
use crate::core::json::*;

/// Default 科创板 audit number used by `stock_kcb_detail_renewal` (akshare hardcodes 926).
const DEFAULT_KCB_AUDIT_NUM: &str = "926";

/// SSE project-query host (requires a `kcb.sse.com.cn` Referer).
const SSE_QUERY: &str = "http://query.sse.com.cn";

/// Eastmoney announcement API.
const EM_NOTICE: &str = "https://np-anotice-stock.eastmoney.com/api/security/ann";

// ---------------------------------------------------------------------------
// Shared parse helpers
// ---------------------------------------------------------------------------

/// Read a field whose value may be a string *or* a number, returning it as a
/// string (used for `registeResult`, which is `1` in some SSE responses and
/// `""` in others).
fn fstr_flex(item: &Value, k: &str) -> Option<String> {
    match item.get(k)? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Extract `pageHelp.data` (the SSE project-record array).
fn sse_data(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("pageHelp")
        .and_then(|p| p.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing pageHelp.data".into(),
        })
}

// ===========================================================================
// stock_kcb_detail_renewal / stock_kcb_renewal — SSE 科创板 project query
// ===========================================================================

/// One 科创板 (SciTech board) project record from SSE `sqlId=SH_XM_LB`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct KcbRenewalRow {
    /// 项目编号 (`stockAuditNum`)
    pub stock_audit_num: Option<String>,
    /// 公司名称 (`stockAuditName`)
    pub stock_audit_name: Option<String>,
    /// 审核状态代码 (`currStatus`)
    pub curr_status: Option<i64>,
    /// 项目类型 (`projectType`)
    pub project_type: Option<i64>,
    /// 注册结果 (`registeResult`, may be a code or empty)
    pub registe_result: Option<String>,
    /// 更新日期 (`updateDate`)
    pub update_date: Option<String>,
    /// 创建时间 (`createTime`)
    pub create_time: Option<String>,
    /// 受理日期 (`auditApplyDate`)
    pub audit_apply_date: Option<String>,
    /// 拟发行股份(亿股) (`planIssueCapital`)
    pub plan_issue_capital: Option<f64>,
    /// 发行数量 (`issueAmount`)
    pub issue_amount: Option<String>,
    /// 中止/暂停状态 (`suspendStatus`)
    pub suspend_status: Option<String>,
    /// 文号 (`wenHao`)
    pub wen_hao: Option<String>,
    /// 征集方式 (`collectType`)
    pub collect_type: Option<i64>,
    /// 发行审核结果 (`commitiResult`)
    pub commiti_result: Option<String>,
    /// 上市板块类型 (`issueMarketType`)
    pub issue_market_type: Option<i64>,
    /// 统一社会信用代码 (`uniformCode`)
    pub uniform_code: Option<String>,
}

fn parse_kcb_renewal(resp: &Value) -> Result<Vec<KcbRenewalRow>> {
    let data = sse_data(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        out.push(KcbRenewalRow {
            stock_audit_num: opt_str(item, "stockAuditNum"),
            stock_audit_name: opt_str(item, "stockAuditName"),
            curr_status: opt_i64(item, "currStatus"),
            project_type: opt_i64(item, "projectType"),
            registe_result: fstr_flex(item, "registeResult"),
            update_date: opt_str(item, "updateDate"),
            create_time: opt_str(item, "createTime"),
            audit_apply_date: opt_str(item, "auditApplyDate"),
            plan_issue_capital: opt_f64(item, "planIssueCapital"),
            issue_amount: opt_str(item, "issueAmount"),
            suspend_status: opt_str(item, "suspendStatus"),
            wen_hao: opt_str(item, "wenHao"),
            collect_type: opt_i64(item, "collectType"),
            commiti_result: opt_str(item, "commitiResult"),
            issue_market_type: opt_i64(item, "issueMarketType"),
            uniform_code: opt_str(item, "uniformCode"),
        });
    }
    Ok(out)
}

/// Port of `stock_kcb_detail_renewal()` (akshare `stock_kcb_detail_sse.py:14`).
///
/// Single-project detail for the 科创板 audit number (akshare hardcodes `926`;
/// exposed as a parameter here, defaulting to `926`). Hits
/// `query.sse.com.cn/commonSoaQuery.do`.
pub async fn stock_kcb_detail_renewal(
    client: &Client,
    stock_audit_num: &str,
) -> Result<Vec<KcbRenewalRow>> {
    let params = [
        ("isPagination", "true"),
        ("sqlId", "SH_XM_LB"),
        ("stockAuditNum", stock_audit_num),
    ];
    let headers = [("Referer", "http://kcb.sse.com.cn/")];
    let v = client
        .get_json_with_headers(
            SOURCE_EASTMONEY,
            "stock_kcb_detail_renewal",
            &format!("{SSE_QUERY}/commonSoaQuery.do"),
            &params,
            Some(&headers),
        )
        .await?;
    parse_kcb_renewal(&v)
}

/// Convenience form matching akshare's parameterless `stock_kcb_detail_renewal()`.
pub async fn stock_kcb_detail_renewal_default(client: &Client) -> Result<Vec<KcbRenewalRow>> {
    stock_kcb_detail_renewal(client, DEFAULT_KCB_AUDIT_NUM).await
}

/// Port of `stock_kcb_renewal()` (akshare `stock_kcb_sse.py:14`).
///
/// Paginated 科创板 project list from `query.sse.com.cn/statusAction.do`.
pub async fn stock_kcb_renewal(client: &Client) -> Result<Vec<KcbRenewalRow>> {
    let mut out = Vec::new();
    let mut page: u32 = 1;
    loop {
        let page_s = page.to_string();
        let params = [
            ("isPagination", "true"),
            ("sqlId", "SH_XM_LB"),
            ("pageHelp.pageSize", "20"),
            ("offerType", ""),
            ("commitiResult", ""),
            ("registeResult", ""),
            ("province", ""),
            ("csrcCode", ""),
            ("currStatus", ""),
            ("order", "updateDate|desc,stockAuditNum|desc"),
            ("keyword", ""),
            ("auditApplyDateBegin", ""),
            ("auditApplyDateEnd", ""),
            ("pageHelp.pageNo", page_s.as_str()),
            ("pageHelp.beginPage", page_s.as_str()),
            ("pageHelp.endPage", page_s.as_str()),
        ];
        let headers = [("Referer", "http://kcb.sse.com.cn/")];
        let v = client
            .get_json_with_headers(
                SOURCE_EASTMONEY,
                "stock_kcb_renewal",
                &format!("{SSE_QUERY}/statusAction.do"),
                &params,
                Some(&headers),
            )
            .await?;
        let data = sse_data(&v)?;
        if data.is_empty() {
            break;
        }
        out.extend(parse_kcb_renewal(&v)?);
        if data.len() < 20 {
            break;
        }
        page += 1;
    }
    Ok(out)
}

// ===========================================================================
// stock_notice_report / stock_individual_notice_report — Eastmoney 公告
// ===========================================================================

/// Eastmoney announcement `f_node` codes, keyed by akshare's Chinese label.
const REPORT_MAP: &[(&str, &str)] = &[
    ("全部", "0"),
    ("财务报告", "1"),
    ("融资公告", "2"),
    ("风险提示", "3"),
    ("信息变更", "4"),
    ("重大事项", "5"),
    ("资产重组", "6"),
    ("持股变动", "7"),
];

fn report_node(symbol: &str) -> Result<&'static str> {
    for (k, v) in REPORT_MAP {
        if *k == symbol {
            return Ok(*v);
        }
    }
    Err(Error::InvalidParam(format!("unknown notice type: {symbol}")))
}

/// One A-share announcement row, port of `stock_notice_report` /
/// `stock_individual_notice_report` (Eastmoney `np-anotice-stock`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct NoticeRow {
    /// 代码 (`stock_code`, lifted from `codes`)
    pub stock_code: Option<String>,
    /// 名称 (`short_name`, lifted from `codes`)
    pub short_name: Option<String>,
    /// 公告标题 (`title`)
    pub title: Option<String>,
    /// 公告类型 (`column_name`, lifted from `columns`)
    pub column_name: Option<String>,
    /// 公告日期 (`notice_date`)
    pub notice_date: Option<String>,
    /// 网址 (derived: `https://data.eastmoney.com/notices/detail/{code}/{art_code}.html`)
    pub url: Option<String>,
}

/// Pick the A-share security object from an announcement's `codes` array
/// (akshare prefers the `ann_type` starting with `A` when several are present).
fn pick_code(codes: &Value) -> Option<&Value> {
    let arr = codes.as_array()?;
    if arr.len() == 1 {
        return arr.first();
    }
    arr.iter()
        .find(|c| {
            c.get("ann_type")
                .and_then(|t| t.as_str())
                .map(|t| t.starts_with('A'))
                .unwrap_or(false)
        })
        .or_else(|| arr.first())
}

/// Parse one Eastmoney announcement-list page (`data.list`) into [`NoticeRow`]s.
fn parse_notice_list(resp: &Value) -> Result<Vec<NoticeRow>> {
    let list = resp
        .get("data")
        .and_then(|d| d.get("list"))
        .and_then(|l| l.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.list".into(),
        })?;
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        let code = item
            .get("codes")
            .and_then(pick_code)
            .and_then(|c| c.get("stock_code"))
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());
        let short_name = item
            .get("codes")
            .and_then(pick_code)
            .and_then(|c| c.get("short_name"))
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());
        let art_code = opt_str(item, "art_code");
        let column_name = item
            .get("columns")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .and_then(|c| c.get("column_name"))
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());
        let url = match (&code, &art_code) {
            (Some(c), Some(a)) => {
                Some(format!("https://data.eastmoney.com/notices/detail/{c}/{a}.html"))
            }
            _ => None,
        };
        out.push(NoticeRow {
            stock_code: code,
            short_name,
            title: opt_str(item, "title"),
            column_name,
            notice_date: opt_str(item, "notice_date"),
            url,
        });
    }
    Ok(out)
}

/// Shared fetcher: paginate Eastmoney `np-anotice-stock` by `total_hits`.
async fn fetch_notice(
    client: &Client,
    symbol: &str,
    security: Option<&str>,
    begin_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<Vec<NoticeRow>> {
    let f_node = report_node(symbol)?;
    let mut params: Vec<(&str, String)> = vec![
        ("sr", "-1".into()),
        ("page_size", "100".into()),
        ("ann_type", "A".into()),
        ("client_source", "web".into()),
        ("f_node", f_node.into()),
        ("s_node", "0".into()),
    ];
    if let Some(s) = security {
        params.push(("stock_list", s.into()));
    }
    if let Some(b) = begin_date {
        params.push(("begin_time", b.into()));
    }
    if let Some(e) = end_date {
        params.push(("end_time", e.into()));
    }

    let mut out = Vec::new();
    let mut page: u32 = 1;
    loop {
        let mut owned: Vec<(String, String)> = params
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect();
        owned.push(("page_index".to_string(), page.to_string()));
        let borrowed: Vec<(&str, &str)> = owned
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let v = client
            .get_json(
                SOURCE_EASTMONEY,
                "stock_notice_report",
                EM_NOTICE,
                &borrowed,
            )
            .await?;
        let total = v
            .get("data")
            .and_then(|d| d.get("total_hits"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        let rows = parse_notice_list(&v)?;
        if rows.is_empty() {
            break;
        }
        out.extend(rows);
        if (page as u64) * 100 >= total {
            break;
        }
        page += 1;
    }
    Ok(out)
}

/// Port of `stock_notice_report(symbol, date)` (akshare `stock_notice.py:133`).
///
/// `date` is `YYYYMMDD`; `symbol` is a Chinese report-type label
/// (e.g. `"全部"`, `"财务报告"`).
pub async fn stock_notice_report(client: &Client, symbol: &str, date: &str) -> Result<Vec<NoticeRow>> {
    let (begin, end) = if date.len() == 8 {
        let d = format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8]);
        (Some(d.clone()), Some(d))
    } else {
        (None, None)
    };
    fetch_notice(
        client,
        symbol,
        None,
        begin.as_deref(),
        end.as_deref(),
    )
    .await
}

/// Port of `stock_individual_notice_report(security, symbol, begin_date, end_date)`
/// (akshare `stock_notice.py:151`).
///
/// `security` is the stock code; `begin_date`/`end_date` are `YYYYMMDD` (optional).
pub async fn stock_individual_notice_report(
    client: &Client,
    security: &str,
    symbol: &str,
    begin_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<Vec<NoticeRow>> {
    let fmt = |d: &str| {
        if d.len() == 8 {
            format!("{}-{}-{}", &d[0..4], &d[4..6], &d[6..8])
        } else {
            d.to_string()
        }
    };
    fetch_notice(
        client,
        symbol,
        Some(security),
        begin_date.map(fmt).as_deref(),
        end_date.map(fmt).as_deref(),
    )
    .await
}

// ===========================================================================
// Tests — offline, against fixtures in tests/fixtures/
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
    fn parses_stock_kcb_detail_renewal() {
        let rows = parse_kcb_renewal(&fixture("stock_kcb_detail_renewal.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].stock_audit_num, Some("926".into()));
        assert_eq!(
            rows[0].stock_audit_name,
            Some("北京浩瀚深度信息技术股份有限公司".into())
        );
        assert_eq!(rows[0].curr_status, Some(5));
        assert_eq!(rows[0].project_type, Some(0));
        assert_eq!(rows[0].registe_result, Some("1".into()));
        assert_eq!(rows[0].update_date, Some("20220630150515".into()));
        assert_eq!(rows[0].plan_issue_capital, Some(4.0));
        assert_eq!(rows[0].uniform_code, Some("91110108102094378J".into()));
    }

    #[test]
    fn parses_stock_kcb_renewal() {
        let rows = parse_kcb_renewal(&fixture("stock_kcb_renewal.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].stock_audit_num, Some("2062".into()));
        assert_eq!(rows[0].stock_audit_name, Some("傲拓科技股份有限公司".into()));
        assert_eq!(rows[0].curr_status, Some(2));
        // Empty-string `registeResult` should still parse (flex field).
        assert_eq!(rows[0].registe_result, Some("".into()));
        assert_eq!(rows[0].plan_issue_capital, Some(5.2496));
    }

    #[test]
    fn report_node_lookup() {
        assert_eq!(report_node("全部").unwrap(), "0");
        assert_eq!(report_node("财务报告").unwrap(), "1");
        assert!(report_node("未知").is_err());
    }

    #[test]
    fn parses_stock_notice_report() {
        let rows = parse_notice_list(&fixture("stock_notice_report.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].stock_code, Some("600350".into()));
        assert_eq!(rows[0].short_name, Some("山东高速".into()));
        assert_eq!(rows[0].title, Some("关于回购股份的进展公告".into()));
        assert_eq!(rows[0].column_name, Some("回购进展情况".into()));
        assert_eq!(rows[0].notice_date, Some("2024-05-06".into()));
        assert_eq!(
            rows[0].url,
            Some("https://data.eastmoney.com/notices/detail/600350/ARTICLE_ID_1.html".into())
        );
    }

    #[test]
    fn parses_stock_individual_notice_report() {
        let rows =
            parse_notice_list(&fixture("stock_individual_notice_report.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].stock_code, Some("300237".into()));
        assert_eq!(rows[0].short_name, Some("美晨科技".into()));
        assert_eq!(rows[0].column_name, Some("财务报告".into()));
        assert!(rows[0].url.is_some());
    }
}
