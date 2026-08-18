use serde_json::Value;

use crate::core::client::{Client, SOURCE_SINA};
use crate::core::error::{Error, Result};
use crate::core::json::*;
use crate::stock::spot::SpotQuote;

const COUNT_URL: &str = "http://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeStockCount?node=hs_a";
const SPOT_URL: &str = "http://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeData";
const PAGE_SIZE: u32 = 80;
const NODE: &str = "hs_a";

/// A-share real-time spot quotes from Sina (`stock_zh_a_spot`).
///
/// Sina paginates `Market_Center.getHQNodeData` (80 rows/page); we first read the
/// total count, then walk the pages. The response is a lenient JSON array of objects
/// (akshare uses `demjson`); we parse it strictly with `serde_json` since the live
/// feed is well-formed JSON. Normalizes to [`SpotQuote`].
pub async fn spot(client: &Client) -> Result<Vec<SpotQuote>> {
    let count_text = client
        .get_text(SOURCE_SINA, "stock_zh_a_spot", COUNT_URL, &[], None)
        .await?;
    let count: u32 = extract_first_number(&count_text)
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "could not parse total stock count".into(),
        })?
        .max(1);
    let total_pages = count.div_ceil(PAGE_SIZE);

    let mut out = Vec::new();
    for page in 1..=total_pages {
        let page_s = page.to_string();
        let params = [
            ("page", page_s.as_str()),
            ("num", "80"),
            ("sort", "symbol"),
            ("asc", "1"),
            ("node", NODE),
            ("symbol", ""),
            ("_s_r_a", "page"),
        ];
        let text = client
            .get_text(SOURCE_SINA, "stock_zh_a_spot", SPOT_URL, &params, None)
            .await?;
        let v: Value = serde_json::from_str(&text).map_err(|e| Error::Parse {
            endpoint: "stock_zh_a_spot",
            message: e.to_string(),
        })?;
        out.extend(parse_rows(&v)?);
    }
    Ok(out)
}

pub(crate) fn parse_rows(resp: &Value) -> Result<Vec<SpotQuote>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "expected a JSON array".into(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(parse_item(item));
    }
    Ok(out)
}

fn parse_item(item: &Value) -> SpotQuote {
    SpotQuote {
        code: norm_code(opt_str_or(item, "code", "")),
        name: opt_str_or(item, "name", ""),
        price: opt_f64(item, "trade"),
        pct_change: opt_f64(item, "changepercent"),
        change: opt_f64(item, "pricechange"),
        volume: opt_f64(item, "volume"),
        amount: opt_f64(item, "amount"),
        turnover_rate: opt_f64(item, "turnoverratio"),
        pe: opt_f64(item, "per"),
        high: opt_f64(item, "high"),
        low: opt_f64(item, "low"),
        open: opt_f64(item, "open"),
        pre_close: opt_f64(item, "settlement"),
        total_mv: opt_f64(item, "mktcap"),
        float_mv: opt_f64(item, "nmc"),
        source: SOURCE_SINA,
    }
}

/// Sina's `code` field is the bare numeric code (e.g. "600000"); strip any leading
/// market prefix just in case the feed returns "sh600000".
fn norm_code(s: String) -> String {
    let s = s.trim();
    if let Some(rest) = s
        .strip_prefix("sh")
        .or_else(|| s.strip_prefix("sz"))
        .or_else(|| s.strip_prefix("bj"))
        && rest.chars().all(|c| c.is_ascii_digit())
    {
        return rest.to_string();
    }
    s.to_string()
}


/// Pull the first run of digits out of a response body (Sina's count endpoint
/// returns a bare number wrapped in light JSONP-ish text).
fn extract_first_number(text: &str) -> Option<u32> {
    let digits: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_sina_spot_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/stock_zh_a_spot_sina.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_rows(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "600000");
        assert_eq!(rows[0].name, "浦发银行");
        assert_eq!(rows[0].price, Some(13.45));
        assert_eq!(rows[0].pct_change, Some(2.31));
        assert_eq!(rows[0].source, "sina");
        assert_eq!(rows[1].code, "000001");
        assert_eq!(rows[1].name, "平安银行");
        assert_eq!(rows[1].pre_close, Some(13.15));
    }
}
