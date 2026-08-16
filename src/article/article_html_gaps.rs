//! Article-domain realized-volatility endpoints ported from `akshare/article/risk_rv.py`.
//!
//! Both upstreams (Oxford-Man Institute `realized.oxford-man.ox.ac.uk`) are
//! unreachable from the build sandbox (connection refused / DNS), so the
//! parsers follow akshare's logic but their tests are `#[ignore]`d — no live
//! fixture could be captured:
//!
//! * [`article_oman_rv`] — `risk_rv.py:18`
//! * [`article_oman_rv_short`] — `risk_rv.py:78`

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// One dated realized-volatility observation (Oxford-Man `realized library`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct OmanRvRow {
    /// Observation epoch in milliseconds (akshare index, `unit="ms"`).
    pub date_ms: i64,
    /// Realized-volatility value for the requested index.
    pub value: Option<f64>,
}

/// Oxford-Man Institute realized library — full series for one index
/// (`article_oman_rv`, akshare `risk_rv.py:18`).
pub async fn article_oman_rv(client: &Client, symbol: &str, index: &str) -> Result<Vec<OmanRvRow>> {
    let url = "https://realized.oxford-man.ox.ac.uk/theme/js/visualization-data.js?20191111113154";
    let html = client
        .get_text("oxford_man", "article_oman_rv", url, &[], None)
        .await?;
    parse_article_oman_rv(&html, "article_oman_rv", symbol, index)
}

pub(crate) fn parse_article_oman_rv(
    html: &str,
    endpoint: &'static str,
    symbol: &str,
    index: &str,
) -> Result<Vec<OmanRvRow>> {
    let v = extract_json(html, endpoint, false)?;
    let obj = v
        .get(format!(".{symbol}"))
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: format!("missing .{symbol}") })?;
    let dates = obj
        .get("dates")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "missing dates".into() })?;
    let series = obj
        .get(index)
        .and_then(|i| i.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: format!("missing {index}.data") })?;
    let n = dates.len().min(series.len());
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(OmanRvRow {
            date_ms: dates[i].as_i64().unwrap_or(0),
            value: series[i].as_f64(),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "empty series".into() });
    }
    Ok(out)
}

/// Oxford-Man Institute realized library — short front-page series
/// (`article_oman_rv_short`, akshare `risk_rv.py:78`).
pub async fn article_oman_rv_short(client: &Client, symbol: &str) -> Result<Vec<OmanRvRow>> {
    let url = "https://realized.oxford-man.ox.ac.uk/theme/js/front-page-chart.js";
    let headers: &[(&str, &str)] = &[
        ("Referer", "https://realized.oxford-man.ox.ac.uk/?from=groupmessage&isappinstalled=0"),
        (
            "user-agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/78.0.3904.97 Safari/537.36",
        ),
    ];
    let html = client
        .get_text("oxford_man", "article_oman_rv_short", url, &[], Some(headers))
        .await?;
    parse_article_oman_rv_short(&html, "article_oman_rv_short", symbol)
}

pub(crate) fn parse_article_oman_rv_short(
    html: &str,
    endpoint: &'static str,
    symbol: &str,
) -> Result<Vec<OmanRvRow>> {
    let v = extract_json(html, endpoint, true)?;
    let obj = v
        .get(format!(".{symbol}"))
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: format!("missing .{symbol}") })?;
    let data = obj
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "missing data".into() })?;
    let mut out = Vec::with_capacity(data.len());
    for row in data {
        let arr = match row.as_array() {
            Some(a) => a,
            None => continue,
        };
        out.push(OmanRvRow {
            date_ms: arr.first().and_then(|x| x.as_i64()).unwrap_or(0),
            value: arr.get(1).and_then(|x| x.as_f64()),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "empty series".into() });
    }
    Ok(out)
}

/// Extract the embedded JSON object from the JS file (the content of the lone
/// `<p>` tag). Mirrors akshare's `soup.find("p").get_text()` +
/// `json.loads(text[find("{"):rfind("};")+1])`.
fn extract_json(html: &str, endpoint: &'static str, short: bool) -> Result<Value> {
    let start = html
        .find('{')
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "no '{' in payload".into() })?;
    let end = if short {
        html.rfind('}').map(|e| e + 1)
    } else {
        html.rfind("};").map(|e| e + 1)
    }
    .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "no json terminator".into() })?;
    let sub = &html[start..end];
    serde_json::from_str(sub).map_err(|e| Error::Parse { endpoint, message: format!("json: {e}") })
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Both Oxford-Man upstreams are unreachable from the build sandbox
    // (realized.oxford-man.ox.ac.uk → connection refused / no DNS). No live
    // fixture could be captured, so tests are ignored.

    #[test]
    #[ignore = "upstream realized.oxford-man.ox.ac.uk unreachable from build sandbox"]
    fn parses_article_oman_rv() {
        let _ = parse_article_oman_rv("", "article_oman_rv", "FTSE", "rk_th2");
    }

    #[test]
    #[ignore = "upstream realized.oxford-man.ox.ac.uk unreachable from build sandbox"]
    fn parses_article_oman_rv_short() {
        let _ = parse_article_oman_rv_short("", "article_oman_rv_short", "FTSE");
    }
}
