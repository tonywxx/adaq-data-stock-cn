//! Commitment-of-Traders (COT) member position-ranking endpoints for Chinese
//! futures exchanges, ported from `akshare/futures/cot.py`.
//!
//! | Rust fn                      | akshare source (`futures/cot.py`) | transport / notes                           |
//! | ---------------------------- | --------------------------------- | ------------------------------------------- |
//! | `get_shfe_rank_table`        | `cot.py:275`                      | SHFE JSON (`o_cursor`)                      |
//! | `futures_gfex_position_rank` | `cot.py:1292`                     | GFEX JSON (3 POST pages per contract)       |
//!
//! ## DEFERRED
//! The following `cot.py` functions are **not** implemented. Each needs either
//! HTML scraping, Excel/`.xls`/`.xlsx` parsing, ZIP extraction, or a charset
//! decoder that is unavailable without editing `Cargo.toml`:
//! - `get_rank_sum_daily` (`cot.py:56`) — date-looping aggregator over
//!   `get_rank_sum`; depends on the deferred CZCE/DCE Excel parsers and issues
//!   many sequential per-day fetches.
//! - `get_rank_sum` (`cot.py:110`) — combines SHFE/CZCE/DCE/CFFEX/GFEX ranking
//!   tables; depends on the deferred CZCE (Excel) and DCE (Excel/HTML) parsers
//!   and sums across rank buckets (top5/10/15/20).
//! - `get_rank_table_czce` (`cot.py:408`) — downloads a `.xls`/`.xlsx` file and
//!   parses it with pandas' Excel reader; needs an Excel parser.
//! - `get_dce_rank_table` (`cot.py:566`) — downloads Excel and/or scrapes HTML
//!   via BeautifulSoup; needs an Excel/HTML parser.
//! - `get_cffex_rank_table` (`cot.py:716`) — the CSV response is GBK-encoded with
//!   no `charset` declared in `Content-Type`, so `reqwest` will not decode it; a
//!   GBK charset decoder would require a new dependency (Cargo.toml edit).
//! - `futures_dce_position_rank` (`cot.py:818`) — POST returns a ZIP archive of
//!   text tables; needs a ZIP parser.
//! - `futures_dce_position_rank_other` (`cot.py:1052`) — scrapes HTML via
//!   BeautifulSoup; needs an HTML parser.

use serde_json::Value;
use std::collections::HashMap;

use scraper::{Html, Selector};

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use crate::core::json::*;

/// SHFE daily position-rank endpoint (JSON, `o_cursor`).
const SOURCE_SHFE: &str = "shfe";
/// GFEX daily position-rank endpoints (JSON, multi-page POST).
const SOURCE_GFEX: &str = "gfex";

const SHFE_RANK_URL: &str = "https://www.shfe.com.cn/data/tradedata/future/dailydata/pm{date}.dat";
/// akshare `cons.shfe_headers`.
const SHFE_HEADERS: &[(&str, &str)] = &[(
    "User-Agent",
    "Mozilla/4.0 (compatible; MSIE 5.5; Windows NT)",
)];

const GFEX_VARS_URL: &str = "http://www.gfex.com.cn/u/interfacesWebVariety/loadList";
const GFEX_CONTRACT_URL: &str =
    "http://www.gfex.com.cn/u/interfacesWebTiMemberDealPosiQuotes/loadListContract_id";
const GFEX_DATA_URL: &str = "http://www.gfex.com.cn/u/interfacesWebTiMemberDealPosiQuotes/loadList";
/// akshare GFEX `User-Agent` header.
const GFEX_HEADERS: &[(&str, &str)] = &[(
    "User-Agent",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
     Chrome/119.0.0.0 Safari/537.36",
)];

// ---------------------------------------------------------------------------
// get_shfe_rank_table — SHFE member position ranking (JSON)
// ---------------------------------------------------------------------------

/// One SHFE member position-ranking row (`get_shfe_rank_table`).
///
/// akshare columns: 排名, 成交量会员, 成交量, 成交量变化, 持多单会员, 持多单,
/// 持多单变化, 持空单会员, 持空单, 持空单变化, 合约, 品种, 日期.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShfeRankRow {
    /// Rank. akshare column `RANK`.
    pub rank: Option<i64>,
    /// Volume member name. akshare column `PARTICIPANTABBR1`.
    pub vol_party_name: String,
    /// Volume. akshare column `CJ1`.
    pub vol: Option<f64>,
    /// Volume change. akshare column `CJ1_CHG`.
    pub vol_chg: Option<f64>,
    /// Long-position member name. akshare column `PARTICIPANTABBR2`.
    pub long_party_name: String,
    /// Long open interest. akshare column `CJ2`.
    pub long_open_interest: Option<f64>,
    /// Long open-interest change. akshare column `CJ2_CHG`.
    pub long_open_interest_chg: Option<f64>,
    /// Short-position member name. akshare column `PARTICIPANTABBR3`.
    pub short_party_name: String,
    /// Short open interest. akshare column `CJ3`.
    pub short_open_interest: Option<f64>,
    /// Short open-interest change. akshare column `CJ3_CHG`.
    pub short_open_interest_chg: Option<f64>,
    /// Instrument id, e.g. `cu2410`. akshare column `INSTRUMENTID`.
    pub symbol: String,
    /// Variety (alphabetic prefix of `symbol`). akshare derives `variety`.
    pub variety: String,
    /// Trade date `YYYYMMDD`.
    pub date: String,
}

/// SHFE member position ranking (`get_shfe_rank_table`).
///
/// `date` is `YYYYMMDD`. `vars_list`, when provided, keeps only rows whose
/// `variety` is in the list (akshare filters by `vars_list` after the fetch);
/// pass `None` to return every contract traded that day.
pub async fn get_shfe_rank_table(
    client: &Client,
    date: &str,
    vars_list: Option<&[&str]>,
) -> Result<Vec<ShfeRankRow>> {
    if date.len() != 8 || !date.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::InvalidParam("date must be YYYYMMDD".into()));
    }
    let url = SHFE_RANK_URL.replace("{date}", date);
    let v = client
        .get_json_with_headers(
            SOURCE_SHFE,
            "get_shfe_rank_table",
            &url,
            &[],
            Some(SHFE_HEADERS),
        )
        .await?;
    let rows = parse_shfe_rank(&v, date)?;
    match vars_list {
        Some(vars) => {
            let wanted: std::collections::HashSet<String> =
                vars.iter().map(|s| s.to_lowercase()).collect();
            Ok(rows
                .into_iter()
                .filter(|r| wanted.contains(&r.variety.to_lowercase()))
                .collect())
        }
        None => Ok(rows),
    }
}

/// Parse SHFE `o_cursor` JSON into ranking rows.
pub(crate) fn parse_shfe_rank(resp: &Value, date: &str) -> Result<Vec<ShfeRankRow>> {
    let cursor = match resp.get("o_cursor").and_then(|c| c.as_array()) {
        Some(a) => a,
        None => return Ok(Vec::new()),
    };
    let mut out = Vec::with_capacity(cursor.len());
    for item in cursor {
        let symbol = opt_str(item, "INSTRUMENTID").unwrap_or_default();
        let variety = variety_of(&symbol);
        out.push(ShfeRankRow {
            rank: opt_i64(item, "RANK"),
            vol_party_name: opt_str(item, "PARTICIPANTABBR1").unwrap_or_default(),
            vol: opt_f64(item, "CJ1"),
            vol_chg: opt_f64(item, "CJ1_CHG"),
            long_party_name: opt_str(item, "PARTICIPANTABBR2").unwrap_or_default(),
            long_open_interest: opt_f64(item, "CJ2"),
            long_open_interest_chg: opt_f64(item, "CJ2_CHG"),
            short_party_name: opt_str(item, "PARTICIPANTABBR3").unwrap_or_default(),
            short_open_interest: opt_f64(item, "CJ3"),
            short_open_interest_chg: opt_f64(item, "CJ3_CHG"),
            symbol,
            variety,
            date: date.to_string(),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// futures_gfex_position_rank — GFEX daily position ranking (JSON)
// ---------------------------------------------------------------------------

/// One GFEX member position-ranking row (`futures_gfex_position_rank`).
///
/// akshare columns: 排名, 成交量会员, 成交量, 成交量变化, 持多单会员, 持多单,
/// 持多单变化, 持空单会员, 持空单, 持空单变化, 合约, 品种, 日期.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GfexRankRow {
    /// Rank (1-based member index). akshare assigns `index + 1`.
    pub rank: i64,
    /// Volume member name. GFEX field `abbr` (data_type=1).
    pub vol_party_name: String,
    /// Volume. GFEX field `todayQty` (data_type=1).
    pub vol: Option<f64>,
    /// Volume change. GFEX field `qtySub` (data_type=1).
    pub vol_chg: Option<f64>,
    /// Long-position member name. GFEX field `abbr` (data_type=2).
    pub long_party_name: String,
    /// Long open interest. GFEX field `todayQty` (data_type=2).
    pub long_open_interest: Option<f64>,
    /// Long open-interest change. GFEX field `qtySub` (data_type=2).
    pub long_open_interest_chg: Option<f64>,
    /// Short-position member name. GFEX field `abbr` (data_type=3).
    pub short_party_name: String,
    /// Short open interest. GFEX field `todayQty` (data_type=3).
    pub short_open_interest: Option<f64>,
    /// Short open-interest change. GFEX field `qtySub` (data_type=3).
    pub short_open_interest_chg: Option<f64>,
    /// Contract id, e.g. `si2312`. akshare `symbol`.
    pub symbol: String,
    /// Variety (uppercased var), e.g. `SI`. akshare `variety`.
    pub variety: String,
    /// Trade date `YYYYMMDD`.
    pub date: String,
}

/// GFEX daily member position ranking (`futures_gfex_position_rank`).
///
/// `date` is `YYYYMMDD`. `vars_list` selects varieties (e.g. `["si", "lc"]`);
/// pass `None` to fetch every GFEX variety via the exchange's variety list.
/// For each contract the function issues three POSTs (volume / long / short
/// pages) and stitches them into one row per member, dropping the trailing
/// total row as akshare does.
pub async fn futures_gfex_position_rank(
    client: &Client,
    date: &str,
    vars_list: Option<&[&str]>,
) -> Result<Vec<GfexRankRow>> {
    if date.len() != 8 || !date.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::InvalidParam("date must be YYYYMMDD".into()));
    }
    let vars: Vec<String> = match vars_list {
        Some(v) => v.iter().map(|s| s.to_lowercase()).collect(),
        None => gfex_vars_list(client).await?,
    };
    let mut out = Vec::new();
    for var in &vars {
        let contracts = gfex_contract_list(client, var, date).await?;
        for contract in &contracts {
            let pages = [
                gfex_contract_page(client, contract, var, date, 1).await?,
                gfex_contract_page(client, contract, var, date, 2).await?,
                gfex_contract_page(client, contract, var, date, 3).await?,
            ];
            let rows = parse_gfex_contract_data(&pages, contract, &var.to_uppercase(), date)?;
            out.extend(rows);
        }
    }
    Ok(out)
}

/// Fetch the full GFEX variety list (`__futures_gfex_vars_list`).
async fn gfex_vars_list(client: &Client) -> Result<Vec<String>> {
    let v = client
        .post_form_json(
            SOURCE_GFEX,
            "futures_gfex_position_rank",
            GFEX_VARS_URL,
            &[],
            Some(GFEX_HEADERS),
        )
        .await?;
    let mut out = Vec::new();
    if let Some(arr) = v.get("data").and_then(|d| d.as_array()) {
        for item in arr {
            if let Some(id) = item.get("varietyId").and_then(|x| x.as_str()) {
                out.push(id.to_string());
            }
        }
    }
    Ok(out)
}

/// Fetch the contract-id list for one variety/date (`__futures_gfex_contract_list`).
async fn gfex_contract_list(client: &Client, var: &str, date: &str) -> Result<Vec<String>> {
    let params = [("variety", var), ("trade_date", date)];
    let v = client
        .post_form_json(
            SOURCE_GFEX,
            "futures_gfex_position_rank",
            GFEX_CONTRACT_URL,
            &params,
            Some(GFEX_HEADERS),
        )
        .await?;
    let mut out = Vec::new();
    if let Some(arr) = v.get("data").and_then(|d| d.as_array()) {
        for item in arr {
            if let Some(s) = item.as_str() {
                out.push(s.to_string());
            }
        }
    }
    Ok(out)
}

/// Fetch one GFEX ranking page (`__futures_gfex_contract_data`, single `data_type`).
async fn gfex_contract_page(
    client: &Client,
    contract: &str,
    var: &str,
    date: &str,
    data_type: u32,
) -> Result<Value> {
    let dt = data_type.to_string();
    let params = [
        ("trade_date", date),
        ("trade_type", "0"),
        ("variety", var),
        ("contract_id", contract),
        ("data_type", dt.as_str()),
    ];
    client
        .post_form_json(
            SOURCE_GFEX,
            "futures_gfex_position_rank",
            GFEX_DATA_URL,
            &params,
            Some(GFEX_HEADERS),
        )
        .await
}

/// One parsed GFEX page row (identical field names across all three `data_type`s).
struct GfexPageRow {
    abbr: String,
    qty: Option<f64>,
    qty_chg: Option<f64>,
}

/// Parse a single GFEX page JSON (`{ "data": [ {abbr, todayQty, qtySub}, ... ] }`).
fn parse_gfex_page(page: &Value) -> Vec<GfexPageRow> {
    let mut out = Vec::new();
    if let Some(arr) = page.get("data").and_then(|d| d.as_array()) {
        for item in arr {
            out.push(GfexPageRow {
                abbr: opt_str(item, "abbr").unwrap_or_default(),
                qty: opt_f64(item, "todayQty"),
                qty_chg: opt_f64(item, "qtySub"),
            });
        }
    }
    out
}

/// Stitch three GFEX pages (volume/long/short) into per-member rows.
///
/// Mirrors akshare's horizontal concat of the three `data_type` pages then
/// `iloc[:-1]` to drop the trailing total row. `pages` order is
/// `[volume, long, short]`.
pub(crate) fn parse_gfex_contract_data(
    pages: &[Value; 3],
    symbol: &str,
    variety: &str,
    date: &str,
) -> Result<Vec<GfexRankRow>> {
    let vol = parse_gfex_page(&pages[0]);
    let long = parse_gfex_page(&pages[1]);
    let short = parse_gfex_page(&pages[2]);
    let n = vol.len().max(long.len()).max(short.len());
    if n == 0 {
        return Ok(Vec::new());
    }
    // akshare drops the trailing total row (big_df.iloc[:-1]).
    let last = n - 1;
    let mut out = Vec::with_capacity(last);
    for i in 0..last {
        let v = vol.get(i);
        let l = long.get(i);
        let s = short.get(i);
        out.push(GfexRankRow {
            rank: i as i64 + 1,
            vol_party_name: v.map(|x| x.abbr.clone()).unwrap_or_default(),
            vol: v.and_then(|x| x.qty),
            vol_chg: v.and_then(|x| x.qty_chg),
            long_party_name: l.map(|x| x.abbr.clone()).unwrap_or_default(),
            long_open_interest: l.and_then(|x| x.qty),
            long_open_interest_chg: l.and_then(|x| x.qty_chg),
            short_party_name: s.map(|x| x.abbr.clone()).unwrap_or_default(),
            short_open_interest: s.and_then(|x| x.qty),
            short_open_interest_chg: s.and_then(|x| x.qty_chg),
            symbol: symbol.to_string(),
            variety: variety.to_string(),
            date: date.to_string(),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Variety = leading alphabetic prefix of a contract symbol (e.g. `cu2410` → `cu`).
fn variety_of(symbol: &str) -> String {
    symbol
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect()
}

// ---------------------------------------------------------------------------
// futures_dce_position_rank_other — DCE member position ranking (HTML)
// ---------------------------------------------------------------------------

/// DCE member position-ranking row for a specific contract, as returned by
/// `futures_dce_position_rank_other` (akshare `cot.py:1052`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct DcePositionRankOtherRow {
    /// 名次.
    pub rank: Option<f64>,
    /// 成交量会员 (volume broker).
    pub vol_party_name: String,
    /// 成交量.
    pub vol: Option<f64>,
    /// 比上交易增减 (volume change).
    pub vol_chg: Option<f64>,
    /// 持多单会员 (long broker).
    pub long_party_name: String,
    /// 持多单 (long open interest).
    pub long_open_interest: Option<f64>,
    /// 持多单变化 (long change).
    pub long_open_interest_chg: Option<f64>,
    /// 持空单会员 (short broker).
    pub short_party_name: String,
    /// 持空单 (short open interest).
    pub short_open_interest: Option<f64>,
    /// 持空单变化 (short change).
    pub short_open_interest_chg: Option<f64>,
    /// 品种 (variety, e.g. `c`).
    pub variety: String,
    /// 合约 (contract, e.g. `c2401`).
    pub symbol: String,
}

/// Extract `symbol` from `javascript:setVariety('symbol')` onclick attributes.
///
/// Mirrors akshare `soup.find_all(attrs={"class": "selBox"})[-3].find_all("input")`.
fn parse_dce_symbols(html: &str, endpoint: &'static str) -> Result<Vec<String>> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse(".selBox")
        .map_err(|e| Error::Parse { endpoint, message: format!("selBox selector: {e}") })?;
    let boxes: Vec<scraper::ElementRef> = doc.select(&sel).collect();
    if boxes.len() < 3 {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: format!("expected >=3 .selBox, found {}", boxes.len()),
        });
    }
    let box_el = boxes[boxes.len() - 3];
    let mut out = Vec::new();
    for inp in box_el.select(&Selector::parse("input").unwrap()) {
        if let Some(onclick) = inp.value().attr("onclick") {
            if let Some(s) = onclick.strip_prefix("javascript:setVariety('").and_then(|s| s.strip_suffix("');")) {
                out.push(s.to_string());
            }
        }
    }
    Ok(out)
}

/// Extract `contract` ids from `javascript:setContract_id('id')` onclick
/// attributes, prefixing 4-char ids with the variety symbol (akshare logic).
fn parse_dce_contracts(html: &str, symbol: &str, endpoint: &'static str) -> Result<Vec<String>> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("input[name=\"contract\"]")
        .map_err(|e| Error::Parse { endpoint, message: format!("contract selector: {e}") })?;
    let mut out = Vec::new();
    for inp in doc.select(&sel) {
        if let Some(onclick) = inp.value().attr("onclick") {
            if let Some(c) = onclick.strip_prefix("javascript:setContract_id('").and_then(|s| s.strip_suffix("');")) {
                let contract = if c.len() == 4 {
                    format!("{symbol}{c}")
                } else {
                    c.to_string()
                };
                out.push(contract);
            }
        }
    }
    Ok(out)
}

/// Parse one DCE per-contract ranking table (`pd.read_html(r.text)[1].iloc[:-1, :]`).
///
/// The raw table has 12 columns; indices 4 and 9 are spacer columns (`_`) that
/// akshare drops. The trailing `合计` total row is also dropped.
pub(crate) fn parse_dce_rank_table(
    html: &str,
    symbol: &str,
    contract: &str,
    endpoint: &'static str,
) -> Result<Vec<DcePositionRankOtherRow>> {
    let all = crate::core::html::tables_with(html, endpoint, "table")?;
    let rows = all
        .into_iter()
        .nth(1)
        .ok_or_else(|| Error::UpstreamChanged {
            origin: endpoint,
            message: "rank table[1] not found".into(),
        })?;
    let out: Vec<DcePositionRankOtherRow> = rows
        .iter()
        .skip(1) // header
        .filter(|r| r.len() >= 12 && !r[1].contains("合计"))
        .map(|r| {
            let num = |s: &str| -> Option<f64> {
                let t = s.trim().replace(',', "");
                if t.is_empty() || t == "--" {
                    None
                } else {
                    t.parse::<f64>().ok()
                }
            };
            DcePositionRankOtherRow {
                rank: num(&r[0]),
                vol_party_name: r[1].clone(),
                vol: num(&r[2]),
                vol_chg: num(&r[3]),
                long_party_name: r[5].clone(),
                long_open_interest: num(&r[6]),
                long_open_interest_chg: num(&r[7]),
                short_party_name: r[9].clone(),
                short_open_interest: num(&r[10]),
                short_open_interest_chg: num(&r[11]),
                variety: symbol.to_uppercase(),
                symbol: contract.to_string(),
            }
        })
        .collect();
    Ok(out)
}

/// 大连商品交易所-每日持仓排名-具体合约-补充 (`futures_dce_position_rank_other`,
/// akshare `cot.py:1052`).
///
/// Scrapes the DCE member position-ranking pages in three steps (variety →
/// contract → ranking table) and returns a map keyed by contract code.
pub async fn futures_dce_position_rank_other(
    client: &Client,
    date: &str,
) -> Result<HashMap<String, Vec<DcePositionRankOtherRow>>> {
    let endpoint = "futures_dce_position_rank_other";
    let url = "http://www.dce.com.cn/publicweb/quotesdata/memberDealPosiQuotes.html";
    // Build the DCE date payload (year, month-1, day) from YYYYMMDD.
    let (y, m, d) = if date.len() == 8 {
        (
            date[..4].parse::<i32>().unwrap_or(1970),
            date[4..6].parse::<i32>().unwrap_or(1),
            date[6..8].parse::<i32>().unwrap_or(1),
        )
    } else {
        (1970, 1, 1)
    };
    let year = y.to_string();
    let month = (m - 1).to_string();
    let day = d.to_string();
    let base: Vec<(&str, &str)> = vec![
        ("memberDealPosiQuotes.variety", "c"),
        ("memberDealPosiQuotes.trade_type", "0"),
        ("year", &year),
        ("month", &month),
        ("day", &day),
        ("contract.contract_id", "all"),
        ("contract.variety_id", "c"),
        ("contract", ""),
    ];
    let html = client.post_form_text(SOURCE_SHFE, endpoint, url, &base, None).await?;
    let symbols = parse_dce_symbols(&html, endpoint)?;
    let mut out: HashMap<String, Vec<DcePositionRankOtherRow>> = HashMap::new();
    for symbol in symbols {
        let sym_payload: Vec<(&str, &str)> = vec![
            ("memberDealPosiQuotes.variety", &symbol),
            ("memberDealPosiQuotes.trade_type", "0"),
            ("year", &year),
            ("month", &month),
            ("day", &day),
            ("contract.contract_id", "all"),
            ("contract.variety_id", &symbol),
            ("contract", ""),
        ];
        let html2 = client
            .post_form_text(SOURCE_SHFE, endpoint, url, &sym_payload, None)
            .await?;
        let contracts = parse_dce_contracts(&html2, &symbol, endpoint)?;
        for contract in contracts {
            let con_payload: Vec<(&str, &str)> = vec![
                ("memberDealPosiQuotes.variety", &symbol),
                ("memberDealPosiQuotes.trade_type", "0"),
                ("year", &year),
                ("month", &month),
                ("day", &day),
                ("contract.contract_id", &contract),
                ("contract.variety_id", &symbol),
                ("contract", ""),
            ];
            let html3 = client
                .post_form_text(SOURCE_SHFE, endpoint, url, &con_payload, None)
                .await?;
            let rows = parse_dce_rank_table(&html3, &symbol, &contract, endpoint)?;
            out.insert(contract, rows);
        }
    }
    Ok(out)
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

    #[test]
    fn parses_shfe_rank_fixture() {
        let v = fixture("futures_cot_shfe_rank.json");
        let rows = parse_shfe_rank(&v, "20240509").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].rank, Some(1));
        assert_eq!(rows[0].vol_party_name, "永安期货");
        assert_eq!(rows[0].vol, Some(12345.0));
        assert_eq!(rows[0].vol_chg, Some(-50.0));
        assert_eq!(rows[0].long_open_interest, Some(23456.0));
        assert_eq!(rows[0].short_open_interest_chg, Some(200.0));
        assert_eq!(rows[0].symbol, "cu2410");
        assert_eq!(rows[0].variety, "cu");
        assert_eq!(rows[0].date, "20240509");
        // "" parses to None for the optional numeric fields.
        assert_eq!(rows[1].vol, None);
        assert_eq!(rows[1].long_open_interest, None);
    }

    #[test]
    fn parses_gfex_contract_data_fixture() {
        let v = fixture("futures_cot_gfex_rank.json");
        let pages = [
            v.get("vol").cloned().unwrap(),
            v.get("long").cloned().unwrap(),
            v.get("short").cloned().unwrap(),
        ];
        // 4 rows per page (3 members + 1 trailing total) -> 3 members after drop.
        let rows = parse_gfex_contract_data(&pages, "si2312", "SI", "20231113").unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].rank, 1);
        assert_eq!(rows[0].vol_party_name, "中信期货");
        assert_eq!(rows[0].vol, Some(2940.0));
        assert_eq!(rows[0].vol_chg, Some(-2409.0));
        assert_eq!(rows[0].long_open_interest, Some(4350.0));
        assert_eq!(rows[0].long_open_interest_chg, Some(-100.0));
        assert_eq!(rows[0].short_open_interest, Some(9308.0));
        assert_eq!(rows[0].short_open_interest_chg, Some(50.0));
        assert_eq!(rows[0].symbol, "si2312");
        assert_eq!(rows[0].variety, "SI");
        assert_eq!(rows[0].date, "20231113");
        // Trailing "合计" total row must be dropped.
        assert!(!rows.iter().any(|r| r.vol_party_name == "合计"));
        // Last member retains its data.
        assert_eq!(rows[2].vol_party_name, "永安期货");
        assert_eq!(rows[2].short_open_interest, Some(7000.0));
    }

    fn load_html(name: &str) -> String {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        std::fs::read_to_string(p).unwrap()
    }

    #[test]
    fn parses_dce_symbols() {
        // akshare reads `soup.find_all(class="selBox")[-3]` — only the third
        // box from the end contributes its input symbols.
        let syms =
            parse_dce_symbols(&load_html("futures_dce_position_rank_other_symbols.html"), "futures_dce_position_rank_other")
                .unwrap();
        assert_eq!(syms, vec!["c".to_string()]);
    }

    #[test]
    fn parses_dce_contracts() {
        let cons = parse_dce_contracts(
            &load_html("futures_dce_position_rank_other_contracts.html"),
            "c",
            "futures_dce_position_rank_other",
        )
        .unwrap();
        // 4-char contract ids are prefixed with the variety symbol.
        assert!(cons.contains(&"c2401".to_string()));
        assert!(cons.contains(&"c2405".to_string()));
    }

    #[test]
    fn parses_dce_rank_table() {
        let rows = parse_dce_rank_table(
            &load_html("futures_dce_position_rank_other_rank.html"),
            "c",
            "c2401",
            "futures_dce_position_rank_other",
        )
        .unwrap();
        assert_eq!(rows.len(), 2, "expected 2 members (合计 dropped)");
        assert_eq!(rows[0].rank, Some(1.0));
        assert_eq!(rows[0].vol_party_name, "中信期货");
        assert!((rows[0].vol.unwrap() - 12345.0).abs() < 1e-9);
        assert!((rows[0].long_open_interest.unwrap() - 23456.0).abs() < 1e-9);
        assert_eq!(rows[0].variety, "C");
        assert_eq!(rows[0].symbol, "c2401");
    }
}
