//! CCI commodity index daily history (`futures_index_ccidx`).
//!
//! Ports akshare `futures_index_ccidx`: the China Commodity Index (CCI) exposes
//! a daily date-line via a JSON GET to
//! `http://www.ccidx.com/CCI-ZZZS/index/getDateLine`.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use crate::core::json::*;

const BASE: &str = "http://www.ccidx.com/CCI-ZZZS/index/getDateLine";

const INDEX_MAP: &[(&str, &str)] = &[
    ("中证商品期货指数", "100001.CCI"),
    ("中证商品期货价格指数", "000001.CCI"),
];

/// One CCI daily index row (`futures_index_ccidx`).
///
/// akshare columns: 日期 (`date`), 指数代码 (`index_id`), 收盘点位
/// (`closing_price`), 结算点位 (`settle_price`), 涨跌 (`change`),
/// 涨跌幅 (`change_pct`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuturesIndexCcidxRow {
    pub date: String,
    pub index_id: String,
    pub closing_price: Option<f64>,
    pub settle_price: Option<f64>,
    pub change: Option<f64>,
    pub change_pct: Option<f64>,
}

/// CCI commodity index daily history (`futures_index_ccidx`).
///
/// `symbol` is one of `{"中证商品期货指数", "中证商品期货价格指数"}`.
pub async fn futures_index_ccidx(client: &Client, symbol: &str) -> Result<Vec<FuturesIndexCcidxRow>> {
    let index_id = INDEX_MAP
        .iter()
        .find(|(k, _)| *k == symbol)
        .map(|(_, v)| *v)
        .ok_or_else(|| Error::InvalidParam(format!("unknown symbol: {symbol}")))?;
    let params = [("indexId", index_id)];
    let v = client
        .get_json("ccidx", "futures_index_ccidx", BASE, &params)
        .await?;
    parse_ccidx(&v)
}

/// Parse the CCI `getDateLine` JSON into rows.
pub(crate) fn parse_ccidx(resp: &Value) -> Result<Vec<FuturesIndexCcidxRow>> {
    let lines = resp
        .get("data")
        .and_then(|d| d.get("dateLineJson"))
        .and_then(|l| l.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: "ccidx",
            message: "missing data.dateLineJson".into(),
        })?;
    let mut out = Vec::with_capacity(lines.len());
    for item in lines {
        out.push(FuturesIndexCcidxRow {
            date: opt_str_or(item, "tradeDate", ""),
            index_id: opt_str_or(item, "indexId", ""),
            closing_price: opt_f64(item, "closingPrice"),
            settle_price: opt_f64(item, "settlePrice"),
            change: opt_f64(item, "dailyIncreaseAndDecrease"),
            change_pct: opt_f64(item, "dailyIncreaseAndDecreasePercentage"),
        });
    }
    Ok(out)
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_index_ccidx_fixture() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/futures_index_ccidx.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_ccidx(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02");
        assert_eq!(rows[0].index_id, "100001.CCI");
        assert_eq!(rows[0].closing_price, Some(1000.5));
        assert_eq!(rows[0].settle_price, Some(999.8));
        assert_eq!(rows[0].change, Some(2.3));
        assert_eq!(rows[0].change_pct, Some(0.23));
        assert_eq!(rows[1].date, "2024-01-03");
    }
}
