//! Eastmoney 行业板块 (industry sector boards).

use serde_json::Value;

use crate::board::{fnum, fstr, fetch_clist_page, BoardConsRow, BoardRow, PAGE_SIZE};
use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

// `fs` / `fid` / `fields` replicate akshare's `stock_board_industry_em.py` exactly.
const FS_NAME: &str = "m:90 t:2 f:!50";
const FID_NAME: &str = "f3";
const FIELDS_NAME: &str = "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f12,f13,f14,f15,f16,f17,f18,f20,f21,\
f23,f24,f25,f26,f22,f33,f11,f62,f128,f136,f115,f152,f124,f107,f104,f105,f140,f141,f207,f208,f209,f222";

const FID_CONS: &str = "f3";
const FIELDS_CONS: &str = "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f12,f13,f14,f15,f16,f17,f18,f20,f21,\
f23,f24,f25,f22,f11,f62,f128,f136,f115,f152,f45";

/// 行业板块-名称: list of industry boards with their spot statistics.
///
/// Port of akshare `stock_board_industry_name_em`.
pub async fn stock_board_industry_name_em(client: &Client) -> Result<Vec<BoardRow>> {
    fetch_boards(
        client,
        "stock_board_industry_name_em",
        FS_NAME,
        FID_NAME,
        FIELDS_NAME,
    )
    .await
}

/// 行业板块-板块成份: constituent stocks of one industry board.
///
/// `symbol` may be a `BKxxxx` board code or a Chinese board name (resolved via
/// [`stock_board_industry_name_em`]). Port of akshare `stock_board_industry_cons_em`.
pub async fn stock_board_industry_cons_em(client: &Client, symbol: &str) -> Result<Vec<BoardConsRow>> {
    let code = resolve_code(client, symbol).await?;
    fetch_cons(
        client,
        "stock_board_industry_cons_em",
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
        let rows = stock_board_industry_name_em(client).await?;
        rows.into_iter()
            .find(|r| r.name == symbol)
            .map(|r| r.code)
            .ok_or_else(|| Error::InvalidParam(format!("unknown industry board: {symbol}")))
    }
}

/// Parse an Eastmoney `clist/get` `data.diff` array into [`BoardRow`]s.
/// Malformed rows (missing code/name) are skipped.
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
    fn parses_industry_name_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/board_industry_name_em.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "BK1027");
        assert_eq!(rows[0].name, "小金属");
        assert_eq!(rows[0].price, Some(1234.56));
        assert_eq!(rows[0].pct_change, Some(1.23));
        assert_eq!(rows[0].high, Some(1250.0));
        assert_eq!(rows[1].name, "互联网服务");
        assert_eq!(rows[1].code, "BK0735");
    }

    #[test]
    fn parses_industry_cons_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/board_industry_cons_em.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_cons(&v, "BK1027").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "600000");
        assert_eq!(rows[0].name, "浦发银行");
        assert_eq!(rows[0].board_code, "BK1027");
        assert_eq!(rows[0].price, Some(13.45));
        assert_eq!(rows[1].name, "招商银行");
        assert_eq!(rows[1].pct_change, Some(-1.20));
    }

    #[test]
    fn skips_malformed_rows() {
        let v: Value = serde_json::json!({
            "data": {
                "total": 2,
                "diff": [
                    {"f12": "BK0001", "f14": "正常板块", "f2": "100.0", "f3": "1.0"},
                    {"f12": "", "f14": "缺代码", "f2": "1.0"},
                    {"f2": "5.0"}
                ]
            }
        });
        let rows = parse(&v).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].code, "BK0001");
    }
}
