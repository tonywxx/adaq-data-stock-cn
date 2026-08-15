use crate::core::client::{Client, SOURCE_SINA};
use crate::core::error::{Error, Result};

/// Real-time spot quote for a single SSE/SZSE option from Sina's `hq.sinajs.cn`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OptionSinaSpotRow {
    pub code: String,
    pub name: String,
    pub price: Option<f64>,
    pub pct_change: Option<f64>,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub pre_close: Option<f64>,
    pub open_interest: Option<f64>,
    pub source: &'static str,
}

/// Daily history for a CFFEX option contract from Sina's `FutureOptionAllService`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OptionCffexDailyRow {
    pub symbol: String,
    pub date: String,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: Option<f64>,
    pub volume: Option<f64>,
    pub source: &'static str,
}

/// Real-time option spot from Sina (`option_sse_spot_price_sina`).
///
/// Sina requires a `Referer` header on `hq.sinajs.cn`; it is attached via the
/// `headers` arg to [`Client::get_text`].
pub async fn option_sina_spot(client: &Client, symbol: &str) -> Result<Vec<OptionSinaSpotRow>> {
    let url = format!("https://hq.sinajs.cn/list=CON_OP_{symbol}");
    let headers = [("Referer", "https://vip.stock.finance.sina.com.cn/")];
    let text = client
        .get_text(SOURCE_SINA, "option_sina_spot", &url, &[], Some(&headers))
        .await?;
    parse_spot(&text)
}

/// Daily history of a CFFEX option contract from Sina (`option_cffex_hs300_daily_sina`).
///
/// The endpoint is a JSONP envelope (`var _<symbol>YYYY_M_D=[...]`); we strip the
/// wrapper and parse the inner array of `[open, high, low, close, volume, date]`.
pub async fn option_cffex_daily(client: &Client, symbol: &str) -> Result<Vec<OptionCffexDailyRow>> {
    let (y, m, d) = today_ymd();
    let url = format!(
        "https://stock.finance.sina.com.cn/futures/api/jsonp.php/var%20_{symbol}{y}_{m}_{d}=/FutureOptionAllService.getOptionDayline"
    );
    let params = [("symbol", symbol)];
    let text = client
        .get_text(SOURCE_SINA, "option_cffex_daily", &url, &params, None)
        .await?;
    parse_cffex_daily(&text, symbol)
}

pub(crate) fn parse_spot(text: &str) -> Result<Vec<OptionSinaSpotRow>> {
    let code = match text.find("hq_str_") {
        Some(start) => {
            let rest = &text[start + "hq_str_".len()..];
            let end = rest.find('=').unwrap_or(rest.len());
            rest[..end].to_string()
        }
        None => String::new(),
    };
    let (open_q, close_q) = (text.find('"'), text.rfind('"'));
    let body = match (open_q, close_q) {
        (Some(o), Some(c)) if c > o => &text[o + 1..c],
        _ => return Ok(Vec::new()),
    };
    let parts: Vec<&str> = body.split(',').collect();
    if parts.len() < 43 {
        return Err(Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: format!("spot has {} fields, expected >= 43", parts.len()),
        });
    }
    let f = |i: usize| parse_f64(parts[i]);
    Ok(vec![OptionSinaSpotRow {
        code,
        name: parts[37].to_string(),
        price: f(2),
        pct_change: f(6),
        open: f(9),
        high: f(39),
        low: f(40),
        pre_close: f(8),
        open_interest: f(5),
        source: SOURCE_SINA,
    }])
}

pub(crate) fn parse_cffex_daily(text: &str, symbol: &str) -> Result<Vec<OptionCffexDailyRow>> {
    let (open, close) = (text.find('['), text.rfind(']'));
    let body = match (open, close) {
        (Some(o), Some(c)) if c > o => &text[o..=c],
        _ => {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_SINA,
                message: "cffex daily response not wrapped in []".into(),
            })
        }
    };
    let arr: Vec<Vec<serde_json::Value>> = serde_json::from_str(body).map_err(|e| Error::Parse {
        endpoint: "option_cffex_daily",
        message: e.to_string(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for row in arr {
        if row.len() < 6 {
            continue;
        }
        let num = |v: &serde_json::Value| v.as_str().and_then(parse_f64).or_else(|| v.as_f64());
        out.push(OptionCffexDailyRow {
            symbol: symbol.to_string(),
            date: row[5].as_str().unwrap_or_default().to_string(),
            open: num(&row[0]),
            high: num(&row[1]),
            low: num(&row[2]),
            close: num(&row[3]),
            volume: num(&row[4]),
            source: SOURCE_SINA,
        });
    }
    Ok(out)
}

fn parse_f64(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        t.parse::<f64>().ok()
    }
}

/// Current local date as (year, month, day) without external crates
/// (Hinnant's `civil_from_days`, days since Unix epoch).
fn today_ymd() -> (i64, u32, u32) {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let z = secs / 86_400 + 719_468;
    let era = if z >= 0 { z / 146_097 } else { (z - 146_096) / 146_097 };
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let dofy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * dofy + 2) / 153;
    let d = dofy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_sina_spot_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/option_sina_spot.json");
        let txt = std::fs::read_to_string(path).unwrap();
        // Fixture wraps the raw Sina text in a {"text": "..."} envelope.
        let v: serde_json::Value = serde_json::from_str(&txt).unwrap();
        let body = v.get("text").and_then(|t| t.as_str()).unwrap();
        let rows = parse_spot(body).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].code, "CON_OP_10003720");
        assert_eq!(rows[0].name, "50ETF购1月3000");
        assert_eq!(rows[0].price, Some(0.1234));
        assert_eq!(rows[0].pct_change, Some(2.31));
        assert_eq!(rows[0].open, Some(0.1210));
        assert_eq!(rows[0].high, Some(0.1330));
        assert_eq!(rows[0].low, Some(0.1200));
        assert_eq!(rows[0].pre_close, Some(0.1205));
        assert_eq!(rows[0].open_interest, Some(98765.0));
        assert_eq!(rows[0].source, "sina");
    }

    #[test]
    fn parses_cffex_daily_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/option_cffex_daily.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&txt).unwrap();
        let body = v.get("text").and_then(|t| t.as_str()).unwrap();
        let rows = parse_cffex_daily(body, "io2202P4350").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "io2202P4350");
        assert_eq!(rows[0].date, "2022-01-04");
        assert_eq!(rows[0].open, Some(0.0500));
        assert_eq!(rows[0].high, Some(0.0600));
        assert_eq!(rows[0].low, Some(0.0400));
        assert_eq!(rows[0].close, Some(0.0550));
        assert_eq!(rows[0].volume, Some(1234.0));
        assert_eq!(rows[0].source, "sina");
        assert_eq!(rows[1].date, "2022-01-05");
        assert_eq!(rows[1].close, Some(0.0580));
    }
}
