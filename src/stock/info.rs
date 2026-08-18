//! 股票基本信息 (stock basic info). Ports `akshare/stock/stock_info.py`.
//!
//! The akshare source scrapes SSE / SZSE / BSE / Sina endpoints — several via
//! Excel (`.xlsx`) or HTML-table download. Per the porting guide those are
//! replaced here with Eastmoney's `push2` **clist** JSON endpoint
//! (`emg_clist_array`), which exposes code/name lists for every A-share market
//! and the delisted board with no JS / token / Excel / HTML barrier. The `fs`
//! board filters are taken from akshare's own Eastmoney usage
//! (`stock_feature/stock_hist_em.py`, `stock_zh_a_special.py`).
//!
//! | Rust function | akshare source | Eastmoney `fs` / notes |
//! |---|---|---|
//! | `stock_info_a_code_name` | stock_info.py:439 | `m:0 t:6,m:0 t:80,m:1 t:2,m:1 t:23,m:0 t:81` (all A) |
//! | `stock_info_bj_name_code` | stock_info.py:184 | `m:0 t:81` (BSE) |
//! | `stock_info_sh_name_code` | stock_info.py:121 | `m:1 t:2` (SH main board A) |
//! | `stock_info_sz_name_code` | stock_info.py:19 | `m:0 t:6` (SZ main board A) |
//! | `stock_info_sh_delist` | stock_info.py:286 | `m:0 s:3` (delist board), SH codes |
//! | `stock_info_sz_delist` | stock_info.py:355 | `m:0 s:3` (delist board), SZ codes |
//!
//! ## DEFERRED
//!
//! * `stock_info_change_name` (stock_info.py:411) — akshare reads a **Sina HTML
//!   table** (`pd.read_html` of a `.phtml` page). Eastmoney clist carries no
//!   name-change-history field and HTML scraping is a hard-defer per the guide.
//! * `stock_info_sz_change_name` (stock_info.py:384) — akshare downloads an
//!   **SZSE `.xlsx`** via `ShowReport`. Excel/ZIP download is a hard-defer and
//!   Eastmoney clist has no name-change-history field.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

use crate::core::json::*;

const SOURCE: &str = "eastmoney";
const PUSH2: &str = "https://push2.eastmoney.com/api/qt/clist/get";

// Eastmoney clist board filters (from akshare's own Eastmoney usage).
const FS_SH_MAIN: &str = "m:1 t:2"; // 沪市主板 A 股
const FS_SZ_MAIN: &str = "m:0 t:6"; // 深市主板 A 股
const FS_BJ: &str = "m:0 t:81"; // 北交所
const FS_ALL_A: &str = "m:0 t:6,m:0 t:80,m:1 t:2,m:1 t:23,m:0 t:81"; // 全部 A 股
const FS_DELIST: &str = "m:0 s:3"; // 两网及退市 (delisted board)

/// Extract the `data.diff` row array from a push2 clist response.
fn emg_clist_array(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|a| a.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE,
            message: "missing data.diff".into(),
        })
}

/// Shared push2 clist fetch. Returns the raw `diff` row array.
async fn fetch_clist(
    client: &Client,
    fn_name: &'static str,
    fs: &str,
    fields: &str,
    page_size: &str,
) -> Result<Vec<Value>> {
    let params: &[(&str, &str)] = &[
        ("pn", "1"),
        ("pz", page_size),
        ("po", "1"),
        ("np", "1"),
        ("fltt", "2"),
        ("invt", "2"),
        ("fid", "f12"),
        ("fs", fs),
        ("fields", fields),
    ];
    let v = client.get_json(SOURCE, fn_name, PUSH2, params).await?;
    emg_clist_array(&v).cloned()
}

// ---------------------------------------------------------------------------
// Row structs
// ---------------------------------------------------------------------------

/// 证券代码 + 证券简称 (code + name). Used by A股 / SH / SZ name-code lists.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CodeNameRow {
    /// 证券代码 (Eastmoney `f12`)
    pub code: String,
    /// 证券简称 (Eastmoney `f14`)
    pub name: String,
}

/// 北交所股票列表 (BSE). `stock_info.py:184`. Beyond code/name, Eastmoney clist
/// exposes market-cap fields (`f20` 总市值, `f21` 流通市值); BSE-specific share
/// counts / 行业 / 地区 require the BSE JSON and are intentionally omitted.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BjNameCodeRow {
    /// 证券代码 (Eastmoney `f12`)
    pub code: String,
    /// 证券简称 (Eastmoney `f14`)
    pub name: String,
    /// 总市值 (Eastmoney `f20`)
    pub total_mv: Option<f64>,
    /// 流通市值 (Eastmoney `f21`)
    pub float_mv: Option<f64>,
}

/// 终止上市公司 (delisted). `stock_info.py:286` / `:355`. Eastmoney clist gives
/// code + name; the SSE/SZSE 上市日期 / 终止上市日期 columns need the exchange
/// query and are omitted.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DelistRow {
    /// 证券代码 (Eastmoney `f12`)
    pub code: String,
    /// 证券简称 (Eastmoney `f14`)
    pub name: String,
}

// ---------------------------------------------------------------------------
// Parsers
// ---------------------------------------------------------------------------

/// Parse code/name rows from a clist `diff` array.
pub(crate) fn parse_code_name(items: &[Value]) -> Result<Vec<CodeNameRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(code) = opt_str(item, "f12") else {
            continue;
        };
        out.push(CodeNameRow {
            code,
            name: opt_str(item, "f14").unwrap_or_default(),
        });
    }
    Ok(out)
}

/// Parse BSE rows (with market-cap fields) from a clist `diff` array.
pub(crate) fn parse_bj_name_code(items: &[Value]) -> Result<Vec<BjNameCodeRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(code) = opt_str(item, "f12") else {
            continue;
        };
        out.push(BjNameCodeRow {
            code,
            name: opt_str(item, "f14").unwrap_or_default(),
            total_mv: opt_f64(item, "f20"),
            float_mv: opt_f64(item, "f21"),
        });
    }
    Ok(out)
}

/// Exchange used to split the combined delist board (`m:0 s:3`) by code prefix.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Exchange {
    Sh,
    Sz,
}

fn is_exchange(code: &str, ex: Exchange) -> bool {
    match ex {
        Exchange::Sh => code.starts_with('6') || code.starts_with('9') || code.starts_with('5'),
        Exchange::Sz => code.starts_with('0') || code.starts_with('3') || code.starts_with('2'),
    }
}

/// Parse delisted rows from a clist `diff` array, keeping only `ex` codes.
pub(crate) fn parse_delist(items: &[Value], ex: Exchange) -> Result<Vec<DelistRow>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(code) = opt_str(item, "f12") else {
            continue;
        };
        if !is_exchange(&code, ex) {
            continue;
        }
        out.push(DelistRow {
            code,
            name: opt_str(item, "f14").unwrap_or_default(),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Public async API
// ---------------------------------------------------------------------------

/// 沪深京 A 股列表 (`stock_info.py:439`). Eastmoney clist over all A-share boards.
pub async fn stock_info_a_code_name(client: &Client) -> Result<Vec<CodeNameRow>> {
    let data = fetch_clist(
        client,
        "stock_info_a_code_name",
        FS_ALL_A,
        "f12,f14",
        "10000",
    )
    .await?;
    parse_code_name(&data)
}

/// 北京证券交易所-股票列表 (`stock_info.py:184`).
pub async fn stock_info_bj_name_code(client: &Client) -> Result<Vec<BjNameCodeRow>> {
    let data = fetch_clist(
        client,
        "stock_info_bj_name_code",
        FS_BJ,
        "f12,f14,f20,f21",
        "10000",
    )
    .await?;
    parse_bj_name_code(&data)
}

/// 上海证券交易所-股票列表 主板A股 (`stock_info.py:121`).
pub async fn stock_info_sh_name_code(client: &Client) -> Result<Vec<CodeNameRow>> {
    let data = fetch_clist(
        client,
        "stock_info_sh_name_code",
        FS_SH_MAIN,
        "f12,f14",
        "10000",
    )
    .await?;
    parse_code_name(&data)
}

/// 深圳证券交易所-股票列表 A股列表 (`stock_info.py:19`).
pub async fn stock_info_sz_name_code(client: &Client) -> Result<Vec<CodeNameRow>> {
    let data = fetch_clist(
        client,
        "stock_info_sz_name_code",
        FS_SZ_MAIN,
        "f12,f14",
        "10000",
    )
    .await?;
    parse_code_name(&data)
}

/// 上海证券交易所-终止上市公司 (`stock_info.py:286`).
pub async fn stock_info_sh_delist(client: &Client) -> Result<Vec<DelistRow>> {
    let data = fetch_clist(client, "stock_info_sh_delist", FS_DELIST, "f12,f14", "1000").await?;
    parse_delist(&data, Exchange::Sh)
}

/// 深圳证券交易所-终止上市公司 (`stock_info.py:355`).
pub async fn stock_info_sz_delist(client: &Client) -> Result<Vec<DelistRow>> {
    let data = fetch_clist(client, "stock_info_sz_delist", FS_DELIST, "f12,f14", "1000").await?;
    parse_delist(&data, Exchange::Sz)
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

    /// Extract the `data.diff` array from a fixture response.
    fn diff_of(name: &str) -> Vec<Value> {
        emg_clist_array(&fixture(name)).unwrap().clone()
    }

    #[test]
    fn parse_stock_info_a_code_name() {
        let rows = parse_code_name(&diff_of("stock_info_a_code_name.json")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].code, "600000");
        assert_eq!(rows[0].name, "浦发银行");
        assert_eq!(rows[1].code, "000001");
        assert_eq!(rows[2].code, "838030");
    }

    #[test]
    fn parse_stock_info_bj_name_code() {
        let rows = parse_bj_name_code(&diff_of("stock_info_bj_name_code.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "838030");
        assert_eq!(rows[0].name, "欧普泰");
        assert_eq!(rows[0].total_mv, Some(1.23e9));
        assert_eq!(rows[0].float_mv, Some(1.0e9));
    }

    #[test]
    fn parse_stock_info_sh_name_code() {
        let rows = parse_code_name(&diff_of("stock_info_sh_name_code.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "600000");
        assert_eq!(rows[0].name, "浦发银行");
    }

    #[test]
    fn parse_stock_info_sz_name_code() {
        let rows = parse_code_name(&diff_of("stock_info_sz_name_code.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "000001");
        assert_eq!(rows[0].name, "平安银行");
    }

    #[test]
    fn parse_stock_info_sh_delist_filters_sz() {
        // delist board mixes SH + SZ; only SH codes must survive.
        let rows = parse_delist(&diff_of("stock_info_sh_delist.json"), Exchange::Sh).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "600000");
        assert_eq!(rows[0].name, "浦发银行");
        assert_eq!(rows[1].code, "900948");
    }

    #[test]
    fn parse_stock_info_sz_delist_filters_sh() {
        let rows = parse_delist(&diff_of("stock_info_sz_delist.json"), Exchange::Sz).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "000001");
        assert_eq!(rows[0].name, "平安银行");
        assert_eq!(rows[1].code, "300111");
    }
}
