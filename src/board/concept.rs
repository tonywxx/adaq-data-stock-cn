//! Eastmoney 概念板块 (concept sector boards).

use serde_json::Value;

use crate::board::{BoardConsRow, BoardRow, PAGE_SIZE, fetch_clist_page, fnum, fstr};
use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

// `fs` / `fid` / `fields` replicate akshare's `stock_board_concept_em.py` exactly.
// Note: the concept-name field list omits f5/f6, so `volume`/`amount` are `None`.
const FS_NAME: &str = "m:90 t:3 f:!50";
const FID_NAME: &str = "f12";
const FIELDS_NAME: &str = "f2,f3,f4,f8,f12,f14,f15,f16,f17,f18,f20,f21,f24,f25,\
f22,f33,f11,f62,f128,f124,f107,f104,f105,f136";

const FID_CONS: &str = "f12";
const FIELDS_CONS: &str = "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f12,f13,f14,f15,f16,f17,f18,f20,f21,\
f23,f24,f25,f22,f11,f62,f128,f136,f115,f152,f45";

/// 概念板块-名称: list of concept boards with their spot statistics.
///
/// Port of akshare `stock_board_concept_name_em`.
pub async fn stock_board_concept_name_em(client: &Client) -> Result<Vec<BoardRow>> {
    fetch_boards(
        client,
        "stock_board_concept_name_em",
        FS_NAME,
        FID_NAME,
        FIELDS_NAME,
    )
    .await
}

/// 概念板块-板块成份: constituent stocks of one concept board.
///
/// `symbol` may be a `BKxxxx` board code or a Chinese board name (resolved via
/// [`stock_board_concept_name_em`]). Port of akshare `stock_board_concept_cons_em`.
pub async fn stock_board_concept_cons_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<BoardConsRow>> {
    let code = resolve_code(client, symbol).await?;
    fetch_cons(
        client,
        "stock_board_concept_cons_em",
        &code,
        FID_CONS,
        FIELDS_CONS,
    )
    .await
}

async fn fetch_boards(
    client: &Client,
    endpoint: &'static str,
    fs: &str,
    fid: &str,
    fields: &str,
) -> Result<Vec<BoardRow>> {
    let mut out = Vec::new();
    let mut pn = 1u32;
    loop {
        let v = fetch_clist_page(client, endpoint, fs, fid, fields, pn, PAGE_SIZE).await?;
        let total = v
            .get("data")
            .and_then(|d| d.get("total"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        let rows = parse(&v)?;
        if rows.is_empty() {
            break;
        }
        out.extend(rows);
        if (pn as u64) * PAGE_SIZE as u64 >= total {
            break;
        }
        pn += 1;
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    }
    Ok(out)
}

async fn fetch_cons(
    client: &Client,
    endpoint: &'static str,
    code: &str,
    fid: &str,
    fields: &str,
) -> Result<Vec<BoardConsRow>> {
    let fs = format!("b:{code} f:!50");
    let mut out = Vec::new();
    let mut pn = 1u32;
    loop {
        let v = fetch_clist_page(client, endpoint, &fs, fid, fields, pn, PAGE_SIZE).await?;
        let total = v
            .get("data")
            .and_then(|d| d.get("total"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        let rows = parse_cons(&v, code)?;
        if rows.is_empty() {
            break;
        }
        out.extend(rows);
        if (pn as u64) * PAGE_SIZE as u64 >= total {
            break;
        }
        pn += 1;
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    }
    Ok(out)
}

/// Resolve a board `symbol` (either `BKxxxx` code or Chinese name) to its `BKxxxx` code.
async fn resolve_code(client: &Client, symbol: &str) -> Result<String> {
    if symbol.starts_with("BK") && symbol[2..].chars().all(|c| c.is_ascii_digit()) {
        Ok(symbol.to_string())
    } else {
        let rows = stock_board_concept_name_em(client).await?;
        rows.into_iter()
            .find(|r| r.name == symbol)
            .map(|r| r.code)
            .ok_or_else(|| Error::InvalidParam(format!("unknown concept board: {symbol}")))
    }
}

/// Parse an Eastmoney `clist/get` `data.diff` array into [`BoardRow`]s.
pub(crate) fn parse(resp: &Value) -> Result<Vec<BoardRow>> {
    let diff = resp
        .get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff".into(),
        })?;
    let mut out = Vec::with_capacity(diff.len());
    for item in diff {
        let code = fstr(item, "f12");
        let name = fstr(item, "f14");
        if code.is_empty() || name.is_empty() {
            continue;
        }
        out.push(BoardRow {
            code,
            name,
            price: fnum(item, "f2"),
            pct_change: fnum(item, "f3"),
            open: fnum(item, "f17"),
            high: fnum(item, "f15"),
            low: fnum(item, "f16"),
            pre_close: fnum(item, "f18"),
            volume: fnum(item, "f5"),
            amount: fnum(item, "f6"),
            turnover: fnum(item, "f8"),
        });
    }
    Ok(out)
}

/// Parse an Eastmoney `clist/get` `data.diff` array into [`BoardConsRow`]s.
pub(crate) fn parse_cons(resp: &Value, board_code: &str) -> Result<Vec<BoardConsRow>> {
    let diff = resp
        .get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff".into(),
        })?;
    let mut out = Vec::with_capacity(diff.len());
    for item in diff {
        let code = fstr(item, "f12");
        let name = fstr(item, "f14");
        if code.is_empty() || name.is_empty() {
            continue;
        }
        out.push(BoardConsRow {
            code,
            name,
            price: fnum(item, "f2"),
            pct_change: fnum(item, "f3"),
            board_code: board_code.to_string(),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_concept_name_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/board_concept_name_em.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "BK0655");
        assert_eq!(rows[0].name, "融资融券");
        assert_eq!(rows[0].price, Some(987.65));
        assert_eq!(rows[0].pct_change, Some(3.21));
        // Concept-name field list omits f5/f6 → volume/amount are None.
        assert!(rows[0].volume.is_none());
        assert!(rows[0].amount.is_none());
        assert_eq!(rows[1].name, "可燃冰");
    }

    #[test]
    fn parses_concept_cons_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/board_concept_cons_em.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_cons(&v, "BK0655").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "000001");
        assert_eq!(rows[0].name, "平安银行");
        assert_eq!(rows[0].board_code, "BK0655");
        assert_eq!(rows[0].price, Some(25.60));
        assert_eq!(rows[1].name, "中信证券");
        assert_eq!(rows[1].pct_change, Some(-2.30));
    }
}
