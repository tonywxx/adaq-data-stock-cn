use serde_json::Value;

use crate::core::client::{Client, SOURCE_SINA};
use crate::core::error::{Error, Result};
use crate::stock::intraday::IntradayRow;

const COUNT_URL: &str =
    "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/CN_Bill.GetBillListCount";
const LIST_URL: &str =
    "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/CN_Bill.GetBillList";
const PAGE_SIZE: u32 = 60;

/// Intraday bill (time & sales) data from Sina (`stock_intraday_sina`).
///
/// Sina paginates `CN_Bill.GetBillList` (60 rows/page) and requires a `Referer`
/// header. `date` is `YYYYMMDD`. Sina's bill feed carries no buy/sell flag, so
/// `direction` is left `None`. Normalizes to [`IntradayRow`].
pub async fn sina(client: &Client, symbol: &str, date: &str) -> Result<Vec<IntradayRow>> {
    if date.len() < 8 {
        return Err(Error::InvalidParam(format!(
            "date must be YYYYMMDD, got: {date}"
        )));
    }
    let day = format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..8]);
    let referer = format!(
        "https://vip.stock.finance.sina.com.cn/quotes_service/view/cn_bill.php?symbol={symbol}"
    );
    let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36";
    let headers = [("Referer", referer.as_str()), ("user-agent", ua)];

    let count_params = [
        ("symbol", symbol),
        ("num", "60"),
        ("page", "1"),
        ("sort", "ticktime"),
        ("asc", "0"),
        ("volume", "0"),
        ("amount", "0"),
        ("type", "0"),
        ("day", &day),
    ];
    let count_text = client
        .get_text(
            SOURCE_SINA,
            "stock_intraday_sina",
            COUNT_URL,
            &count_params,
            Some(&headers),
        )
        .await?;
    let total: u32 = count_text
        .trim()
        .parse()
        .map_err(|_| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "could not parse bill count".into(),
        })?;
    let total_pages = total.div_ceil(PAGE_SIZE);

    let mut out = Vec::new();
    for page in 1..=total_pages {
        let page_s = page.to_string();
        let params = [
            ("symbol", symbol),
            ("num", "60"),
            ("page", page_s.as_str()),
            ("sort", "ticktime"),
            ("asc", "0"),
            ("volume", "0"),
            ("amount", "0"),
            ("type", "0"),
            ("day", &day),
        ];
        let text = client
            .get_text(
                SOURCE_SINA,
                "stock_intraday_sina",
                LIST_URL,
                &params,
                Some(&headers),
            )
            .await?;
        let v: Value = serde_json::from_str(&text).map_err(|e| Error::Parse {
            endpoint: "stock_intraday_sina",
            message: e.to_string(),
        })?;
        out.extend(parse_rows(&v, symbol)?);
    }
    Ok(out)
}

pub(crate) fn parse_rows(resp: &Value, symbol: &str) -> Result<Vec<IntradayRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "expected a JSON array".into(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(IntradayRow {
            symbol: symbol.to_string(),
            time: str_field(item, "ticktime"),
            price: num_field(item, "price"),
            volume: num_field(item, "volume"),
            direction: None,
            source: SOURCE_SINA,
        });
    }
    Ok(out)
}

fn str_field(item: &Value, k: &str) -> String {
    item.get(k)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn num_field(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_sina_intraday_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/stock_intraday_sina.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_rows(&v, "sz000001").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].time, "2025-01-02 09:30:00");
        assert_eq!(rows[0].price, Some(10.10));
        assert_eq!(rows[0].volume, Some(100.0));
        assert_eq!(rows[0].direction, None);
        assert_eq!(rows[0].source, "sina");
        assert_eq!(rows[1].time, "2025-01-02 09:31:00");
    }
}
