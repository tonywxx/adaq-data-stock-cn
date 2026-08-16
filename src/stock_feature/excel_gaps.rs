//! Excel-backed SZSE margin and HK-connect exchange-rate reports
//! (akshare `stock_feature/stock_margin_szse.py`, `stock_feature/stock_hsgt_exchange_rate.py`).

use calamine::{open_workbook_auto_from_rs, Reader, Sheets};

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/88.0.4324.150 Safari/537.36";

async fn fetch_bytes(
    url: &str,
    params: &[(&str, &str)],
    headers: &[(&str, &str)],
) -> Result<Vec<u8>> {
    let http = reqwest::Client::builder()
        .user_agent(UA)
        .build()
        .map_err(Error::Http)?;
    let mut req = http.get(url);
    if !params.is_empty() {
        req = req.query(params);
    }
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let resp = req.send().await.map_err(Error::Http)?;
    let bytes = resp.bytes().await.map_err(Error::Http)?;
    Ok(bytes.to_vec())
}

fn read_rows(bytes: &[u8], endpoint: &'static str) -> Result<Vec<Vec<String>>> {
    let mut wb: Sheets<std::io::Cursor<Vec<u8>>> =
        open_workbook_auto_from_rs(std::io::Cursor::new(bytes.to_vec())).map_err(|e| {
            Error::Parse {
                endpoint,
                message: e.to_string(),
            }
        })?;
    let range = wb
        .worksheet_range_at(0)
        .ok_or_else(|| Error::Parse {
            endpoint,
            message: "no sheet".into(),
        })?
        .map_err(|e| Error::Parse {
            endpoint,
            message: e.to_string(),
        })?;
    Ok(range
        .rows()
        .map(|r| r.iter().map(cell_to_string).collect())
        .collect())
}

fn parse_f64(s: &str) -> Option<f64> {
    let t: String = s.chars().filter(|c| *c != ',').collect();
    let t = t.trim();
    if t.is_empty() {
        return None;
    }
    t.parse::<f64>().ok()
}

fn cell_to_string(c: &calamine::Data) -> String {
    match c {
        calamine::Data::Empty => String::new(),
        calamine::Data::String(s) => s.trim().to_string(),
        calamine::Data::Int(i) => i.to_string(),
        calamine::Data::Float(f) => {
            if f.is_finite() && f.fract() == 0.0 && f.abs() < 1e15 {
                format!("{}", *f as i64)
            } else {
                format!("{}", f)
            }
        }
        calamine::Data::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}

fn col<'a>(row: &'a [String], i: usize) -> &'a str {
    row.get(i).map(|s| s.as_str()).unwrap_or("")
}

/// SZSE margin underlying securities row (`stock_margin_underlying_info_szse`, akshare `stock_feature/stock_margin_szse.py:15`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockMarginUnderlyingInfoSzse {
    /// Security code (akshare `证券代码`).
    pub security_code: String,
    /// Security abbreviation (akshare `证券简称`).
    pub security_abbr: String,
    /// Margin target flag (akshare `融资标的`).
    pub margin_target: String,
    /// Short-selling target flag (akshare `融券标的`).
    pub short_target: String,
    /// Margin-able today flag (akshare `当日可融资`).
    pub can_margin_today: String,
    /// Short-able today flag (akshare `当日可融券`).
    pub can_short_today: String,
    /// Short-sell price limit flag (akshare `融券卖出价格限制`).
    pub short_sell_price_limit: String,
    /// Price-limit flag (akshare `涨跌幅限制`).
    pub price_limit: String,
}

/// SZSE margin underlying securities (`stock_margin_underlying_info_szse`, akshare `stock_feature/stock_margin_szse.py:15`).
pub async fn stock_margin_underlying_info_szse(
    _client: &Client,
    date: &str,
) -> Result<Vec<StockMarginUnderlyingInfoSzse>> {
    let d = format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..]);
    let url = "https://www.szse.cn/api/report/ShowReport";
    let params = &[
        ("SHOWTYPE", "xlsx"),
        ("CATALOGID", "1834_xxpl"),
        ("txtDate", &d),
        ("tab1PAGENO", "1"),
        ("random", "0.7425245522795993"),
        ("TABKEY", "tab1"),
    ];
    let headers = &[("Referer", "https://www.szse.cn/disclosure/margin/object/index.html")];
    let bytes = fetch_bytes(url, params, headers).await?;
    parse_stock_margin_underlying_info_szse(&bytes)
}

pub(crate) fn parse_stock_margin_underlying_info_szse(
    bytes: &[u8],
) -> Result<Vec<StockMarginUnderlyingInfoSzse>> {
    let rows = read_rows(bytes, "stock_margin_underlying_info_szse")?;
    let mut out = Vec::new();
    for r in rows.iter().skip(1) {
        if r.iter().all(|c| c.is_empty()) {
            continue;
        }
        // akshare treats 证券代码 as text (leading-zero preserving).
        let code = col(r, 0).trim();
        out.push(StockMarginUnderlyingInfoSzse {
            security_code: code.to_string(),
            security_abbr: col(r, 1).trim().to_string(),
            margin_target: col(r, 2).to_string(),
            short_target: col(r, 3).to_string(),
            can_margin_today: col(r, 4).to_string(),
            can_short_today: col(r, 5).to_string(),
            short_sell_price_limit: col(r, 6).to_string(),
            price_limit: col(r, 7).to_string(),
        });
    }
    Ok(out)
}

/// SZSE margin trading detail row (`stock_margin_detail_szse`, akshare `stock_feature/stock_margin_szse.py:95`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockMarginDetailSzse {
    /// Security code (akshare `证券代码`).
    pub security_code: String,
    /// Security abbreviation (akshare `证券简称`).
    pub security_abbr: String,
    /// Margin buy amount in CNY (akshare `融资买入额`).
    pub margin_buy_amount: Option<f64>,
    /// Margin balance in CNY (akshare `融资余额`).
    pub margin_balance: Option<f64>,
    /// Short-sell volume (akshare `融券卖出量`).
    pub short_sell_volume: Option<f64>,
    /// Short balance volume (akshare `融券余量`).
    pub short_balance_volume: Option<f64>,
    /// Short balance amount in CNY (akshare `融券余额`).
    pub short_balance_amount: Option<f64>,
    /// Margin + short balance in CNY (akshare `融资融券余额`).
    pub margin_short_balance: Option<f64>,
}

/// SZSE margin trading detail (`stock_margin_detail_szse`, akshare `stock_feature/stock_margin_szse.py:95`).
pub async fn stock_margin_detail_szse(
    _client: &Client,
    date: &str,
) -> Result<Vec<StockMarginDetailSzse>> {
    let d = format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..]);
    let url = "https://www.szse.cn/api/report/ShowReport";
    let params = &[
        ("SHOWTYPE", "xlsx"),
        ("CATALOGID", "1837_xxpl"),
        ("txtDate", &d),
        ("tab2PAGENO", "1"),
        ("random", "0.24279342734085696"),
        ("TABKEY", "tab2"),
    ];
    let headers = &[("Referer", "https://www.szse.cn/disclosure/margin/margin/index.html")];
    let bytes = fetch_bytes(url, params, headers).await?;
    parse_stock_margin_detail_szse(&bytes)
}

pub(crate) fn parse_stock_margin_detail_szse(bytes: &[u8]) -> Result<Vec<StockMarginDetailSzse>> {
    let rows = read_rows(bytes, "stock_margin_detail_szse")?;
    let mut out = Vec::new();
    for r in rows.iter().skip(1) {
        if r.iter().all(|c| c.is_empty()) {
            continue;
        }
        out.push(StockMarginDetailSzse {
            security_code: col(r, 0).trim().to_string(),
            security_abbr: col(r, 1).trim().to_string(),
            margin_buy_amount: parse_f64(col(r, 2)),
            margin_balance: parse_f64(col(r, 3)),
            short_sell_volume: parse_f64(col(r, 4)),
            short_balance_volume: parse_f64(col(r, 5)),
            short_balance_amount: parse_f64(col(r, 6)),
            margin_short_balance: parse_f64(col(r, 7)),
        });
    }
    Ok(out)
}

/// SZSE HK-connect reference exchange-rate row (`stock_sgt_reference_exchange_rate_szse`, akshare `stock_feature/stock_hsgt_exchange_rate.py:47`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockSgtReferenceExchangeRateSzse {
    /// Applicable date `YYYY-MM-DD` (akshare `适用日期`).
    pub apply_date: String,
    /// Reference buy rate (akshare `参考汇率买入价`).
    pub buy_rate: Option<f64>,
    /// Reference sell rate (akshare `参考汇率卖出价`).
    pub sell_rate: Option<f64>,
    /// Currency (akshare `货币种类`).
    pub currency: String,
}

/// SZSE HK-connect reference exchange rate (`stock_sgt_reference_exchange_rate_szse`, akshare `stock_feature/stock_hsgt_exchange_rate.py:47`).
pub async fn stock_sgt_reference_exchange_rate_szse(
    _client: &Client,
) -> Result<Vec<StockSgtReferenceExchangeRateSzse>> {
    let url = "https://www.szse.cn/api/report/ShowReport";
    let params = &[
        ("SHOWTYPE", "xlsx"),
        ("CATALOGID", "SGT_LSHL"),
        ("TABKEY", "tab1"),
        ("random", "0.9184251620553985"),
    ];
    let bytes = fetch_bytes(url, params, &[]).await?;
    parse_stock_sgt_reference_exchange_rate_szse(&bytes)
}

pub(crate) fn parse_stock_sgt_reference_exchange_rate_szse(
    bytes: &[u8],
) -> Result<Vec<StockSgtReferenceExchangeRateSzse>> {
    let rows = read_rows(bytes, "stock_sgt_reference_exchange_rate_szse")?;
    let mut out = Vec::new();
    for r in rows.iter().skip(1) {
        if r.iter().all(|c| c.is_empty()) {
            continue;
        }
        out.push(StockSgtReferenceExchangeRateSzse {
            apply_date: col(r, 0).to_string(),
            buy_rate: parse_f64(col(r, 1)),
            sell_rate: parse_f64(col(r, 2)),
            currency: col(r, 3).to_string(),
        });
    }
    Ok(out)
}

/// SZSE HK-connect settlement exchange-rate row (`stock_sgt_settlement_exchange_rate_szse`, akshare `stock_feature/stock_hsgt_exchange_rate.py:18`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockSgtSettlementExchangeRateSzse {
    /// Applicable date `YYYY-MM-DD` (akshare `适用日期`).
    pub apply_date: String,
    /// Buy settlement rate (akshare `买入结算汇兑比率`).
    pub buy_settlement_rate: Option<f64>,
    /// Sell settlement rate (akshare `卖出结算汇兑比率`).
    pub sell_settlement_rate: Option<f64>,
    /// Currency (akshare `货币种类`).
    pub currency: String,
}

/// SZSE HK-connect settlement exchange rate (`stock_sgt_settlement_exchange_rate_szse`, akshare `stock_feature/stock_hsgt_exchange_rate.py:18`).
pub async fn stock_sgt_settlement_exchange_rate_szse(
    _client: &Client,
) -> Result<Vec<StockSgtSettlementExchangeRateSzse>> {
    let url = "https://www.szse.cn/api/report/ShowReport";
    let params = &[
        ("SHOWTYPE", "xlsx"),
        ("CATALOGID", "SGT_LSHL"),
        ("TABKEY", "tab2"),
        ("random", "0.9184251620553985"),
    ];
    let bytes = fetch_bytes(url, params, &[]).await?;
    parse_stock_sgt_settlement_exchange_rate_szse(&bytes)
}

pub(crate) fn parse_stock_sgt_settlement_exchange_rate_szse(
    bytes: &[u8],
) -> Result<Vec<StockSgtSettlementExchangeRateSzse>> {
    let rows = read_rows(bytes, "stock_sgt_settlement_exchange_rate_szse")?;
    let mut out = Vec::new();
    for r in rows.iter().skip(1) {
        if r.iter().all(|c| c.is_empty()) {
            continue;
        }
        out.push(StockSgtSettlementExchangeRateSzse {
            apply_date: col(r, 0).to_string(),
            buy_settlement_rate: parse_f64(col(r, 1)),
            sell_settlement_rate: parse_f64(col(r, 2)),
            currency: col(r, 3).to_string(),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(name),
        )
        .unwrap()
    }

    #[test]
    fn parses_stock_margin_underlying_info_szse() {
        let rows =
            parse_stock_margin_underlying_info_szse(&fixture("stock_margin_underlying_info_szse.xlsx"))
                .unwrap();
        assert!(rows.len() > 100);
        assert_eq!(rows[0].security_code, "000001");
        assert_eq!(rows[0].margin_target, "Y");
    }

    #[test]
    fn parses_stock_margin_detail_szse() {
        let rows = parse_stock_margin_detail_szse(&fixture("stock_margin_detail_szse.xlsx")).unwrap();
        assert!(rows.len() > 100);
        assert_eq!(rows[0].security_code, "000001");
        assert!((rows[0].margin_buy_amount.unwrap() - 67_452_975.0).abs() < 1.0);
        assert!((rows[0].margin_balance.unwrap() - 4_345_665_664.0).abs() < 1.0);
    }

    #[test]
    fn parses_stock_sgt_reference_exchange_rate_szse() {
        let rows = parse_stock_sgt_reference_exchange_rate_szse(
            &fixture("stock_sgt_reference_exchange_rate_szse.xlsx"),
        )
        .unwrap();
        assert!(rows.len() > 10);
        assert_eq!(rows[0].apply_date, "2026-08-14");
        assert_eq!(rows[0].currency, "HKD");
        assert!((rows[0].buy_rate.unwrap() - 0.83410).abs() < 1e-6);
    }

    #[test]
    fn parses_stock_sgt_settlement_exchange_rate_szse() {
        let rows = parse_stock_sgt_settlement_exchange_rate_szse(
            &fixture("stock_sgt_settlement_exchange_rate_szse.xlsx"),
        )
        .unwrap();
        assert!(rows.len() > 10);
        assert_eq!(rows[0].apply_date, "2026-08-14");
        assert!((rows[0].buy_settlement_rate.unwrap() - 0.85987).abs() < 1e-6);
    }
}
