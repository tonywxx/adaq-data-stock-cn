//! Excel-backed SZSE same-day option contracts (akshare `option/option_current_szse.py`).

use calamine::{open_workbook_auto_from_rs, Reader, Sheets};

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36";

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

/// SZSE current-day option contract (`option_current_day_szse`, akshare `option/option_current_szse.py:14`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct OptionCurrentDaySzse {
    /// Sequence number (akshare `序号`).
    pub seq: Option<f64>,
    /// Contract code (akshare `合约编码`).
    pub contract_code: String,
    /// Contract symbol (akshare `合约代码`).
    pub contract_symbol: String,
    /// Contract short name (akshare `合约简称`).
    pub contract_abbr: String,
    /// Underlying security name (code) (akshare `标的证券简称(代码)`).
    pub underlying: String,
    /// Contract type call/put (akshare `合约类型`).
    pub contract_type: String,
    /// Exercise price (akshare `行权价`).
    pub exercise_price: Option<f64>,
    /// Contract unit (akshare `合约单位`).
    pub contract_unit: Option<f64>,
    /// Last trading day `YYYY-MM-DD` (akshare `最后交易日`).
    pub last_trading_day: String,
    /// Exercise day `YYYY-MM-DD` (akshare `行权日`).
    pub exercise_day: String,
    /// Expiry day `YYYY-MM-DD` (akshare `到期日`).
    pub expiry_day: String,
    /// Delivery day `YYYY-MM-DD` (akshare `交收日`).
    pub delivery_day: String,
    /// Newly listed flag (akshare `新挂`).
    pub newly_listed: String,
    /// Upper limit price (akshare `涨停价格`).
    pub limit_up: Option<f64>,
    /// Lower limit price (akshare `跌停价格`).
    pub limit_down: Option<f64>,
    /// Previous settlement price (akshare `前结算价`).
    pub prev_settlement: Option<f64>,
    /// Contract adjustment flag (akshare `合约调整`).
    pub adjusted: String,
    /// Suspended flag (akshare `停牌`).
    pub halted: String,
    /// Total open interest (akshare `合约总持仓`).
    pub open_interest: Option<f64>,
    /// Listing reason (akshare `挂牌原因`).
    pub list_reason: String,
    /// Original contract code (akshare `原合约代码`).
    pub orig_contract_code: String,
    /// Original contract short name (akshare `原合约简称`).
    pub orig_contract_abbr: String,
    /// Original exercise price (akshare `原行权价格`).
    pub orig_exercise_price: Option<f64>,
    /// Original contract unit (akshare `原合约单位`).
    pub orig_contract_unit: Option<f64>,
    /// Remaining trading days to expiry (akshare `合约到期剩余交易天数`).
    pub remain_trade_days: Option<f64>,
    /// Remaining calendar days to expiry (akshare `合约到期剩余自然天数`).
    pub remain_calendar_days: Option<f64>,
    /// Remaining trading days to next adjustment (akshare `下次合约调整剩余交易天数`).
    pub next_adj_trade_days: Option<f64>,
    /// Remaining calendar days to next adjustment (akshare `下次合约调整剩余自然天数`).
    pub next_adj_calendar_days: Option<f64>,
    /// Trade date `YYYY-MM-DD` (akshare `交易日期`).
    pub trade_date: String,
}

// akshare reorders the sheet: 交易日期 (file col 1) moves to the end.
// Map from the Rust field order above to the file column index.
const FILE_COLS: [usize; 29] = [
    0, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 1,
];

/// SZSE current-day option contracts (`option_current_day_szse`, akshare `option/option_current_szse.py:14`).
pub async fn option_current_day_szse(_client: &Client) -> Result<Vec<OptionCurrentDaySzse>> {
    let url = "https://www.sse.org.cn/api/report/ShowReport";
    let params = &[
        ("SHOWTYPE", "xlsx"),
        ("CATALOGID", "option_drhy"),
        ("TABKEY", "tab1"),
    ];
    let bytes = fetch_bytes(url, params, &[]).await?;
    parse_option_current_day_szse(&bytes)
}

pub(crate) fn parse_option_current_day_szse(bytes: &[u8]) -> Result<Vec<OptionCurrentDaySzse>> {
    let rows = read_rows(bytes, "option_current_day_szse")?;
    let mut out = Vec::new();
    for r in rows.iter().skip(1) {
        if r.iter().all(|c| c.is_empty()) {
            continue;
        }
        let c = |i: usize| col(r, FILE_COLS[i]);
        let f = |i: usize| parse_f64(col(r, FILE_COLS[i]));
        out.push(OptionCurrentDaySzse {
            seq: f(0),
            contract_code: c(1).to_string(),
            contract_symbol: c(2).to_string(),
            contract_abbr: c(3).to_string(),
            underlying: c(4).to_string(),
            contract_type: c(5).to_string(),
            exercise_price: f(6),
            contract_unit: f(7),
            last_trading_day: c(8).to_string(),
            exercise_day: c(9).to_string(),
            expiry_day: c(10).to_string(),
            delivery_day: c(11).to_string(),
            newly_listed: c(12).to_string(),
            limit_up: f(13),
            limit_down: f(14),
            prev_settlement: f(15),
            adjusted: c(16).to_string(),
            halted: c(17).to_string(),
            open_interest: f(18),
            list_reason: c(19).to_string(),
            orig_contract_code: c(20).to_string(),
            orig_contract_abbr: c(21).to_string(),
            orig_exercise_price: f(22),
            orig_contract_unit: f(23),
            remain_trade_days: f(24),
            remain_calendar_days: f(25),
            next_adj_trade_days: f(26),
            next_adj_calendar_days: f(27),
            trade_date: c(28).to_string(),
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
    fn parses_option_current_day_szse() {
        let rows = parse_option_current_day_szse(&fixture("option_current_day_szse.xlsx")).unwrap();
        assert!(rows.len() > 10);
        assert_eq!(rows[0].seq, Some(1.0));
        assert_eq!(rows[0].contract_symbol, "159901C2609M003100");
        assert_eq!(rows[0].contract_type, "认购");
        assert!((rows[0].exercise_price.unwrap() - 3.1).abs() < 1e-6);
        assert_eq!(rows[0].trade_date, "2026-08-17");
        assert_eq!(rows[0].last_trading_day, "2026-09-23");
        assert!((rows[0].contract_unit.unwrap() - 10000.0).abs() < 1e-6);
        assert!(rows[0].open_interest.is_some());
    }
}
