//! Exchange public contract-info (交易参数汇总) endpoints, ported from
//! `akshare/futures_derivative/futures_contract_info_*.py`.
//!
//! | Rust fn                    | akshare source                                  | transport / notes                  |
//! | -------------------------- | ----------------------------------------------- | ---------------------------------- |
//! | `futures_contract_info_dce`  | `futures_contract_info_dce.py:13`   | DCE JSON (`data`) — POST           |
//! | `futures_contract_info_gfex` | `futures_contract_info_gfex.py:13`  | GFEX JSON (`data`) — POST          |
//! | `futures_contract_info_ine`  | `futures_contract_info_ine.py:13`   | INE JSON (`ContractBaseInfo`) — GET |
//! | `futures_contract_info_shfe` | `futures_contract_info_shfe.py:13`  | SHFE JSON (`ContractBaseInfo`) — GET |
//!
//! ## DEFERRED
//!
//! - `futures_contract_info_cffex` (`futures_contract_info_cffex.py:15`) — the
//!   upstream is an XML document (`index.xml` parsed with `xml.etree`), which
//!   needs an XML parser crate; `Cargo.toml` must not be edited.
//! - `futures_contract_info_czce` (`futures_contract_info_czce.py:15`) — same
//!   XML (`FutureDataReferenceData.xml`) barrier as CFFEX; deferred.
//!
//! NOTE: `post_form_json` places POST params in the query string (see
//! `core::client`). DCE's akshare source posts a JSON body; we approximate with
//! query params. Offline parse tests validate the row mapping regardless.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_DCE: &str = "dce";
const SOURCE_GFEX: &str = "gfex";
const SOURCE_INE: &str = "ine";
const SOURCE_SHFE: &str = "shfe";

const DCE_URL: &str = "http://www.dce.com.cn/dcereport/publicweb/tradepara/contractInfo";
const GFEX_URL: &str = "http://www.gfex.com.cn/u/interfacesWebTtQueryContractInfo/loadList";
const INE_URL: &str = "https://www.ine.cn/data/busiparamdata/future/ContractBaseInfo{date}.dat";
const SHFE_URL: &str =
    "https://www.shfe.com.cn/data/busiparamdata/future/ContractBaseInfo{date}.dat";

const GFEX_HEADERS: &[(&str, &str)] = &[(
    "User-Agent",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
     Chrome/119.0.0.0 Safari/537.36",
)];
const SHFE_HEADERS: &[(&str, &str)] = &[(
    "User-Agent",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
     Chrome/119.0.0.0 Safari/537.36",
)];

// ---------------------------------------------------------------------------
// DCE
// ---------------------------------------------------------------------------

/// One DCE contract-info row (`futures_contract_info_dce`).
///
/// akshare columns: 品种名称, 合约, 交易单位, 最小变动价位, 开始交易日,
/// 最后交易日, 最后交割日.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DceContractRow {
    /// Variety name. akshare `品种名称` (`variety`).
    pub variety: String,
    /// Contract id, e.g. `c2501`. akshare `合约` (`contractId`).
    pub symbol: String,
    /// Trade unit. akshare `交易单位` (`unit`).
    pub unit: Option<f64>,
    /// Minimum price tick. akshare `最小变动价位` (`tick`).
    pub tick: Option<f64>,
    /// First trade date `YYYYMMDD`. akshare `开始交易日` (`startTradeDate`).
    pub start_trade_date: Option<String>,
    /// Last trade date `YYYYMMDD`. akshare `最后交易日` (`endTradeDate`).
    pub end_trade_date: Option<String>,
    /// Last delivery date `YYYYMMDD`. akshare `最后交割日` (`endDeliveryDate`).
    pub end_delivery_date: Option<String>,
}

/// DCE contract info (`futures_contract_info_dce`). No params; returns all
/// varieties (`varietyId=all`).
pub async fn futures_contract_info_dce(client: &Client) -> Result<Vec<DceContractRow>> {
    let v = client
        .post_form_json(
            SOURCE_DCE,
            "futures_contract_info_dce",
            DCE_URL,
            &[("lang", "zh"), ("tradeType", "1"), ("varietyId", "all")],
            None,
        )
        .await?;
    parse_dce_contract_info(&v)
}

/// Parse DCE `data` array into contract rows.
pub(crate) fn parse_dce_contract_info(resp: &Value) -> Result<Vec<DceContractRow>> {
    let arr =
        resp.get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_DCE,
                message: "missing data array".into(),
            })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(DceContractRow {
            variety: fstr(item, "variety").unwrap_or_default(),
            symbol: fstr(item, "contractId").unwrap_or_default(),
            unit: fnum(item, "unit"),
            tick: fnum(item, "tick"),
            start_trade_date: fstr(item, "startTradeDate"),
            end_trade_date: fstr(item, "endTradeDate"),
            end_delivery_date: fstr(item, "endDeliveryDate"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// GFEX
// ---------------------------------------------------------------------------

/// One GFEX contract-info row (`futures_contract_info_gfex`).
///
/// akshare columns: 品种, 合约代码, 交易单位, 最小变动单位, 开始交易日,
/// 最后交易日, 最后交割日.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GfexContractRow {
    /// Variety. akshare `品种` (`variety`).
    pub variety: String,
    /// Contract id, e.g. `si2501`. akshare `合约代码` (`contractId`).
    pub symbol: String,
    /// Trade unit. akshare `交易单位` (`unit`).
    pub unit: Option<f64>,
    /// Minimum price tick. akshare `最小变动单位` (`tick`).
    pub tick: Option<f64>,
    /// First trade date `YYYYMMDD`. akshare `开始交易日` (`startTradeDate`).
    pub start_trade_date: Option<String>,
    /// Last trade date `YYYYMMDD`. akshare `最后交易日` (`endTradeDate`).
    pub end_trade_date: Option<String>,
    /// Last delivery date `YYYYMMDD`. akshare `最后交割日` (`endDeliveryDate0`).
    pub end_delivery_date: Option<String>,
}

/// GFEX contract info (`futures_contract_info_gfex`). No params; returns every
/// variety (`variety=""`).
pub async fn futures_contract_info_gfex(client: &Client) -> Result<Vec<GfexContractRow>> {
    let v = client
        .post_form_json(
            SOURCE_GFEX,
            "futures_contract_info_gfex",
            GFEX_URL,
            &[("variety", ""), ("trade_type", "0")],
            Some(GFEX_HEADERS),
        )
        .await?;
    parse_gfex_contract_info(&v)
}

/// Parse GFEX `data` array into contract rows.
pub(crate) fn parse_gfex_contract_info(resp: &Value) -> Result<Vec<GfexContractRow>> {
    let arr =
        resp.get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_GFEX,
                message: "missing data array".into(),
            })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(GfexContractRow {
            variety: fstr(item, "variety").unwrap_or_default(),
            symbol: fstr(item, "contractId").unwrap_or_default(),
            unit: fnum(item, "unit"),
            tick: fnum(item, "tick"),
            start_trade_date: fstr(item, "startTradeDate"),
            end_trade_date: fstr(item, "endTradeDate"),
            end_delivery_date: fstr(item, "endDeliveryDate0"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// INE / SHFE (shared `ContractBaseInfo` shape)
// ---------------------------------------------------------------------------

/// One INE/SHFE contract-info row (`futures_contract_info_ine` /
/// `futures_contract_info_shfe`).
///
/// akshare columns: 合约代码, 上市日, 到期日, 开始交割日, 最后交割日,
/// 挂牌基准价, 交易日.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContractBaseInfoRow {
    /// Contract id, e.g. `sc2410`. akshare `合约代码` (`INSTRUMENTID`).
    pub symbol: String,
    /// Listed date `YYYYMMDD`. akshare `上市日` (`OPENDATE`).
    pub open_date: Option<String>,
    /// Expiry date `YYYYMMDD`. akshare `到期日` (`EXPIREDATE`).
    pub expire_date: Option<String>,
    /// First delivery date `YYYYMMDD`. akshare `开始交割日` (`STARTDELIVDATE`).
    pub start_delivery_date: Option<String>,
    /// Last delivery date `YYYYMMDD`. akshare `最后交割日` (`ENDDELIVDATE`).
    pub end_delivery_date: Option<String>,
    /// Listing base price. akshare `挂牌基准价` (`BASISPRICE`).
    pub basis_price: Option<f64>,
    /// Trading day `YYYYMMDD`. akshare `交易日` (`TRADINGDAY`).
    pub trading_day: Option<String>,
    /// Upstream `update_date` (SHFE only; `None` for INE).
    pub update_date: Option<String>,
}

/// INE contract info (`futures_contract_info_ine`). `date` is `YYYYMMDD`.
pub async fn futures_contract_info_ine(
    client: &Client,
    date: &str,
) -> Result<Vec<ContractBaseInfoRow>> {
    if date.len() != 8 || !date.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::InvalidParam("date must be YYYYMMDD".into()));
    }
    let url = INE_URL.replace("{date}", date);
    let v = client
        .get_json(
            SOURCE_INE,
            "futures_contract_info_ine",
            &url,
            &[("rnd", "0.8312696798757147")],
        )
        .await?;
    parse_contract_base_info(&v, SOURCE_INE)
}

/// SHFE contract info (`futures_contract_info_shfe`). `date` is `YYYYMMDD`.
pub async fn futures_contract_info_shfe(
    client: &Client,
    date: &str,
) -> Result<Vec<ContractBaseInfoRow>> {
    if date.len() != 8 || !date.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::InvalidParam("date must be YYYYMMDD".into()));
    }
    let url = SHFE_URL.replace("{date}", date);
    let v = client
        .get_json_with_headers(
            SOURCE_SHFE,
            "futures_contract_info_shfe",
            &url,
            &[],
            Some(SHFE_HEADERS),
        )
        .await?;
    parse_contract_base_info(&v, SOURCE_SHFE)
}

/// Parse an INE/SHFE `ContractBaseInfo` array (plus optional `update_date`).
pub(crate) fn parse_contract_base_info(
    resp: &Value,
    origin: &'static str,
) -> Result<Vec<ContractBaseInfoRow>> {
    let arr = resp
        .get("ContractBaseInfo")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin,
            message: "missing ContractBaseInfo array".into(),
        })?;
    let update_date = resp
        .get("update_date")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(ContractBaseInfoRow {
            symbol: fstr(item, "INSTRUMENTID").unwrap_or_default(),
            open_date: fstr(item, "OPENDATE"),
            expire_date: fstr(item, "EXPIREDATE"),
            start_delivery_date: fstr(item, "STARTDELIVDATE"),
            end_delivery_date: fstr(item, "ENDDELIVDATE"),
            basis_price: fnum(item, "BASISPRICE"),
            trading_day: fstr(item, "TRADINGDAY"),
            update_date: update_date.clone(),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Extract a string field, returning `None` when missing or not a string.
fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(str::to_string)
}

/// Extract a numeric field, tolerating numeric strings.
fn fnum(item: &Value, k: &str) -> Option<f64> {
    match item.get(k) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// tests (offline fixtures)
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

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    #[test]
    fn parse_dce_contract_info_ok() {
        let rows = parse_dce_contract_info(&fixture("futures_contract_info_dce.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].variety, "玉米");
        assert_eq!(rows[0].symbol, "c2501");
        assert!(approx(rows[0].unit, 10.0));
        assert!(approx(rows[0].tick, 1.0));
        assert_eq!(rows[0].start_trade_date, Some("20240902".into()));
        assert_eq!(rows[0].end_trade_date, Some("20250115".into()));
        assert_eq!(rows[1].symbol, "a2501");
        assert_eq!(rows[1].unit, None);
    }

    #[test]
    fn parse_gfex_contract_info_ok() {
        let rows = parse_gfex_contract_info(&fixture("futures_contract_info_gfex.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].variety, "工业硅");
        assert_eq!(rows[0].symbol, "si2501");
        assert!(approx(rows[0].unit, 5.0));
        assert!(approx(rows[0].tick, 5.0));
        assert_eq!(rows[0].end_delivery_date, Some("20250120".into()));
        assert_eq!(rows[1].symbol, "lc2501");
    }

    #[test]
    fn parse_ine_contract_base_info_ok() {
        let rows = parse_contract_base_info(&fixture("futures_contract_info_ine.json"), SOURCE_INE)
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "sc2410");
        assert_eq!(rows[0].open_date, Some("20240902".into()));
        assert_eq!(rows[0].expire_date, Some("20241015".into()));
        assert!(approx(rows[0].basis_price, 550.0));
        assert_eq!(rows[0].update_date, None);
        assert_eq!(rows[1].symbol, "lu2410");
    }

    #[test]
    fn parse_shfe_contract_base_info_ok() {
        let rows =
            parse_contract_base_info(&fixture("futures_contract_info_shfe.json"), SOURCE_SHFE)
                .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "cu2410");
        assert_eq!(rows[0].open_date, Some("20240301".into()));
        assert!(approx(rows[0].basis_price, 68000.0));
        assert_eq!(rows[0].trading_day, Some("20240513".into()));
        assert_eq!(rows[0].update_date, Some("2024-05-13 15:30".into()));
        assert_eq!(rows[1].symbol, "al2410");
    }
}
