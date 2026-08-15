use serde_json::Value;

use crate::core::client::{Client, SOURCE_TENCENT};
use crate::core::error::{Error, Result};
use crate::stock::spot::SpotQuote;

const SPOT_URL: &str = "https://proxy.finance.qq.com/cgi/cgi-bin/rank/hs/getBoardRankList";
const PAGE_SIZE: u32 = 200;

/// A-share real-time spot quotes from Tencent (`stock_zh_a_spot_tx`).
///
/// Tencent paginates `getBoardRankList` (200 rows/page) off `offset`/`count`. We walk
/// pages until the `data.rank_list` page is empty. Field names in the rank list are
/// best-effort tolerant (multiple aliases tried) since the API is undocumented.
/// Normalizes to [`SpotQuote`].
pub async fn spot(client: &Client) -> Result<Vec<SpotQuote>> {
    let mut out = Vec::new();
    let mut offset: u32 = 0;
    loop {
        let offset_s = offset.to_string();
        let count_s = PAGE_SIZE.to_string();
        let params = [
            ("_appver", "11.17.0"),
            ("board_code", "aStock"),
            ("sort_type", "price"),
            ("direct", "down"),
            ("offset", offset_s.as_str()),
            ("count", count_s.as_str()),
        ];
        let v = client
            .get_json(SOURCE_TENCENT, "stock_zh_a_spot_tx", SPOT_URL, &params)
            .await?;
        let rank_list = v
            .get("data")
            .and_then(|d| d.get("rank_list"))
            .and_then(|r| r.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_TENCENT,
                message: "missing data.rank_list".into(),
            })?;
        if rank_list.is_empty() {
            break;
        }
        out.extend(parse_rows(&v)?);
        if rank_list.len() < PAGE_SIZE as usize {
            break;
        }
        offset += PAGE_SIZE;
    }
    Ok(out)
}

pub(crate) fn parse_rows(resp: &Value) -> Result<Vec<SpotQuote>> {
    let rank_list = resp
        .get("data")
        .and_then(|d| d.get("rank_list"))
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_TENCENT,
            message: "missing data.rank_list".into(),
        })?;
    let mut out = Vec::with_capacity(rank_list.len());
    for item in rank_list {
        out.push(parse_item(item));
    }
    Ok(out)
}

fn parse_item(item: &Value) -> SpotQuote {
    SpotQuote {
        code: norm_code(fstr(item, "code")),
        name: fstr(item, "name"),
        price: first_num(item, &["price", "current", "now"]),
        pct_change: first_num(item, &["zdf", "changepercent"]),
        change: first_num(item, &["zde", "pricechange"]),
        volume: first_num(item, &["cjl", "volume"]),
        amount: first_num(item, &["cje", "amount"]),
        turnover_rate: first_num(item, &["turnoverrate", "hsl"]),
        pe: first_num(item, &["syl", "per"]),
        high: first_num(item, &["high"]),
        low: first_num(item, &["low"]),
        open: first_num(item, &["open"]),
        pre_close: first_num(item, &["prev_price", "prev_close", "settlement"]),
        total_mv: first_num(item, &["mktcap", "zsz"]),
        float_mv: first_num(item, &["nmc", "ltsz"]),
        source: SOURCE_TENCENT,
    }
}

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

fn fstr(item: &Value, k: &str) -> String {
    item.get(k)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn first_num(item: &Value, keys: &[&str]) -> Option<f64> {
    for k in keys {
        if let Some(v) = item.get(k) {
            match v {
                Value::Number(n) => return n.as_f64(),
                Value::String(s) => {
                    if let Ok(f) = s.parse::<f64>() {
                        return Some(f);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_tencent_spot_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/stock_zh_a_spot_tx.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_rows(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "600000");
        assert_eq!(rows[0].name, "浦发银行");
        assert_eq!(rows[0].price, Some(13.45));
        assert_eq!(rows[0].pct_change, Some(2.31));
        assert_eq!(rows[0].source, "tencent");
        assert_eq!(rows[1].code, "000001");
        assert_eq!(rows[1].name, "平安银行");
        assert_eq!(rows[1].pre_close, Some(13.15));
    }
}
