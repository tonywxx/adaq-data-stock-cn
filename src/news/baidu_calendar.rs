use serde::Serialize;
use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use crate::news::{fnum, fstr};

const SOURCE_BAIDU: &str = "baidu";
const CALENDAR_URL: &str = "https://finance.pae.baidu.com/sapi/v1/financecalendar";

/// Economic-calendar row (`news_economic_baidu`).
#[derive(Debug, Clone, Serialize)]
pub struct EventRow {
    pub date: String,
    pub time: String,
    /// Country / region name (upstream `country`).
    pub country: String,
    /// Event name (upstream `title`).
    pub event: String,
    /// Importance star rating (upstream `star`), if present.
    pub importance: Option<f64>,
}

/// Trading-reminder: suspension / resumption row (`news_trade_notify_suspend_baidu`).
#[derive(Debug, Clone, Serialize)]
pub struct SuspendRow {
    pub code: String,
    pub name: String,
    pub exchange: String,
    /// Suspension time (upstream `start`).
    pub start: String,
    /// Resumption time (upstream `end`).
    pub end: String,
    /// Reason for the suspension.
    pub reason: String,
    pub date: String,
    pub time: String,
}

/// Trading-reminder: dividend row (`news_trade_notify_dividend_baidu`).
#[derive(Debug, Clone, Serialize)]
pub struct DividendRow {
    pub code: String,
    pub name: String,
    pub exchange: String,
    /// Ex-dividend date (upstream `diviDate`).
    pub divi_date: String,
    /// Report period (upstream `date`).
    pub report_date: String,
    /// Cash dividend per share (upstream `diviCash`).
    pub divi_cash: String,
    /// Bonus shares per share (upstream `shareDivide`).
    pub share_divide: String,
    /// Capitalization transfer per share (upstream `transfer`).
    pub transfer: String,
}

/// Trading-reminder: earnings-report-time row (`news_report_time_baidu`).
#[derive(Debug, Clone, Serialize)]
pub struct ReportRow {
    pub code: String,
    pub name: String,
    pub exchange: String,
    /// Report type (upstream `reportType`).
    pub report_type: String,
    pub time: String,
    pub date: String,
    /// Market value (upstream `marketValue`).
    pub market_value: String,
}

/// Baidu finance calendar — economic data (`news_economic_baidu`).
///
/// `cookie` is the `BAIDUID=...; HMACOUNT=...` cookie string obtained from a browser
/// session. The shared [`Client`] has no cookie store, so the caller must supply it
/// (akshare acquires it via a two-step browser handshake, not JS signing).
pub async fn news_economic_baidu(
    client: &Client,
    date: &str,
    cookie: &str,
) -> Result<Vec<EventRow>> {
    let resp = baidu_calendar(client, date, "economic_data", cookie).await?;
    parse(&resp)
}

/// Baidu finance calendar — trading-reminder: suspension / resumption.
pub async fn news_trade_notify_suspend_baidu(
    client: &Client,
    date: &str,
    cookie: &str,
) -> Result<Vec<SuspendRow>> {
    let resp = baidu_calendar(client, date, "notify_suspend", cookie).await?;
    parse_suspend(&resp)
}

/// Baidu finance calendar — trading-reminder: dividend.
pub async fn news_trade_notify_dividend_baidu(
    client: &Client,
    date: &str,
    cookie: &str,
) -> Result<Vec<DividendRow>> {
    let resp = baidu_calendar(client, date, "notify_divide", cookie).await?;
    parse_dividend(&resp)
}

/// Baidu finance calendar — earnings report time.
pub async fn news_report_time_baidu(
    client: &Client,
    date: &str,
    cookie: &str,
) -> Result<Vec<ReportRow>> {
    let resp = baidu_calendar(client, date, "report_time", cookie).await?;
    parse_report(&resp)
}

/// Walk Baidu's paginated calendar API for `date` (format `YYYYMMDD`), returning the
/// merged raw response (all pages concatenated into `Result.calendarInfo[*].list`).
async fn baidu_calendar(
    client: &Client,
    date: &str,
    cate: &str,
    cookie: &str,
) -> Result<Value> {
    if date.len() != 8 || !date.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::InvalidParam(format!(
            "date must be YYYYMMDD, got `{date}`"
        )));
    }
    let formatted = format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..]);
    let cookie_h: &str = cookie;
    let headers: Vec<(&str, &str)> = vec![
        ("accept", "application/vnd.finance-web.v1+json"),
        ("origin", "https://finance.baidu.com"),
        ("referer", "https://finance.baidu.com/"),
        (
            "user-agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36",
        ),
        ("cookie", cookie_h),
    ];

    // `pn` is index 2 in this vec (set per page below).
    let mut base: Vec<(String, String)> = vec![
        ("start_date".into(), formatted.clone()),
        ("end_date".into(), formatted.clone()),
        ("pn".into(), "0".into()),
        ("rn".into(), "100".into()),
        ("cate".into(), cate.into()),
        ("finClientType".into(), "pc".into()),
    ];

    let params: Vec<(&str, &str)> = base.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let first_text = client
        .get_text(SOURCE_BAIDU, "baidu_calendar", CALENDAR_URL, &params, Some(&headers))
        .await?;
    let mut merged: Value = serde_json::from_str(&first_text).map_err(Error::Json)?;

    // Number of pages for the target date (100 rows per page).
    let total = find_total(&merged, &formatted);
    let total_pages = if total == 0 { 1 } else { total.div_ceil(100) };
    for pn in 1..total_pages {
        base[2].1 = pn.to_string();
        let params: Vec<(&str, &str)> =
            base.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let text = client
            .get_text(SOURCE_BAIDU, "baidu_calendar", CALENDAR_URL, &params, Some(&headers))
            .await?;
        let v: Value = serde_json::from_str(&text).map_err(Error::Json)?;
        merge_calendar_info(&mut merged, &v);
    }
    Ok(merged)
}

/// Extend each `Result.calendarInfo[*].list` in `merged` with the matching date's
/// list entries from `other` (Baidu returns one `calendarInfo` entry per date).
fn merge_calendar_info(merged: &mut Value, other: &Value) {
    let Some(m_list) = merged
        .get_mut("Result")
        .and_then(|r| r.get_mut("calendarInfo"))
        .and_then(|a| a.as_array_mut())
    else {
        return;
    };
    let Some(o_list) = other
        .get("Result")
        .and_then(|r| r.get("calendarInfo"))
        .and_then(|a| a.as_array())
    else {
        return;
    };
    for o_entry in o_list {
        let o_date = o_entry.get("date").and_then(|v| v.as_str());
        if let Some(m_entry) = m_list
            .iter_mut()
            .find(|e| e.get("date").and_then(|v| v.as_str()) == o_date)
            && let (Some(m_arr), Some(o_arr)) = (
                m_entry.get_mut("list").and_then(|l| l.as_array_mut()),
                o_entry.get("list").and_then(|l| l.as_array()),
            )
        {
            m_arr.extend(o_arr.iter().cloned());
        }
    }
}

/// Gather every `list` entry from `Result.calendarInfo` (all dates).
fn extract_all(resp: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    if let Some(ci) = resp
        .get("Result")
        .and_then(|r| r.get("calendarInfo"))
        .and_then(|a| a.as_array())
    {
        for item in ci {
            if let Some(list) = item.get("list").and_then(|l| l.as_array()) {
                out.extend(list.iter().cloned());
            }
        }
    }
    out
}

/// Look up the `total` record count for `target` date inside `Result.calendarInfo`.
fn find_total(resp: &Value, target: &str) -> u64 {
    resp.get("Result")
        .and_then(|r| r.get("calendarInfo"))
        .and_then(|a| a.as_array())
        .and_then(|ci| {
            ci.iter()
                .find(|i| i.get("date").and_then(|v| v.as_str()) == Some(target))
        })
        .and_then(|i| i.get("total"))
        .and_then(|t| t.as_u64())
        .unwrap_or(0)
}

/// Map decoded calendar JSON to [`EventRow`]s (economic data), skipping rows without a title.
pub(crate) fn parse(resp: &Value) -> Result<Vec<EventRow>> {
    Ok(extract_all(resp).into_iter().filter_map(|item| parse_event(&item)).collect())
}

fn parse_event(item: &Value) -> Option<EventRow> {
    let event = fstr(item, "title");
    if event.is_empty() {
        return None;
    }
    Some(EventRow {
        date: fstr(item, "date"),
        time: fstr(item, "time"),
        country: fstr(item, "country"),
        event,
        importance: item.get("star").and_then(|v| match v {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.parse::<f64>().ok(),
            _ => None,
        }),
    })
}

pub(crate) fn parse_suspend(resp: &Value) -> Result<Vec<SuspendRow>> {
    Ok(extract_all(resp).into_iter().filter_map(|item| parse_suspend_item(&item)).collect())
}

fn parse_suspend_item(item: &Value) -> Option<SuspendRow> {
    let code = fstr(item, "code");
    if code.is_empty() {
        return None;
    }
    Some(SuspendRow {
        code,
        name: fstr(item, "name"),
        exchange: fstr(item, "exchange"),
        start: fstr(item, "start"),
        end: fstr(item, "end"),
        reason: fstr(item, "reason"),
        date: fstr(item, "date"),
        time: fstr(item, "time"),
    })
}

pub(crate) fn parse_dividend(resp: &Value) -> Result<Vec<DividendRow>> {
    Ok(extract_all(resp)
        .into_iter()
        .filter_map(|item| parse_dividend_item(&item))
        .collect())
}

fn parse_dividend_item(item: &Value) -> Option<DividendRow> {
    let code = fstr(item, "code");
    if code.is_empty() {
        return None;
    }
    Some(DividendRow {
        code,
        name: fstr(item, "name"),
        exchange: fstr(item, "exchange"),
        divi_date: fstr(item, "diviDate"),
        report_date: fstr(item, "date"),
        divi_cash: fstr(item, "diviCash"),
        share_divide: fstr(item, "shareDivide"),
        transfer: fstr(item, "transfer"),
    })
}

pub(crate) fn parse_report(resp: &Value) -> Result<Vec<ReportRow>> {
    Ok(extract_all(resp).into_iter().filter_map(|item| parse_report_item(&item)).collect())
}

fn parse_report_item(item: &Value) -> Option<ReportRow> {
    let code = fstr(item, "code");
    if code.is_empty() {
        return None;
    }
    Some(ReportRow {
        code,
        name: fstr(item, "name"),
        exchange: fstr(item, "exchange"),
        report_type: fstr(item, "reportType"),
        time: fstr(item, "time"),
        date: fstr(item, "date"),
        market_value: fnum(item, "marketValue")
            .map(|v| v.to_string())
            .unwrap_or_else(|| fstr(item, "marketValue")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
        let txt = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    #[test]
    fn parses_news_economic_baidu() {
        let rows = parse(&fixture("news_economic_baidu.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2025-11-26");
        assert_eq!(rows[0].time, "21:30");
        assert_eq!(rows[0].country, "美国");
        assert_eq!(rows[0].event, "美国当周初请失业金人数");
        assert_eq!(rows[0].importance, Some(3.0));
        assert_eq!(rows[1].event, "美国11月密歇根大学消费者信心指数");
        assert_eq!(rows[1].importance, Some(2.0));
    }

    #[test]
    fn parses_trade_notify_suspend() {
        let rows = parse_suspend(&fixture("news_trade_notify_suspend_baidu.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].code, "600000");
        assert_eq!(rows[0].name, "浦发银行");
        assert_eq!(rows[0].start, "2025-11-26 09:30");
        assert_eq!(rows[0].end, "2025-11-27 09:30");
    }

    #[test]
    fn parses_trade_notify_dividend() {
        let rows = parse_dividend(&fixture("news_trade_notify_dividend_baidu.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].code, "000001");
        assert_eq!(rows[0].name, "平安银行");
        assert_eq!(rows[0].divi_cash, "0.5");
    }

    #[test]
    fn parses_report_time() {
        let rows = parse_report(&fixture("news_report_time_baidu.json")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].code, "300750");
        assert_eq!(rows[0].name, "宁德时代");
        assert_eq!(rows[0].report_type, "三季报");
    }
}
