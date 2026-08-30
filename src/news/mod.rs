//! News / NLP / event text & calendar endpoints.
//!
//! Ports the JSON-API functions from akshare's `news/`, `nlp/` and `event/`
//! packages. Functions are grouped by upstream source:
//!
//! - Eastmoney individual-stock news — [`stock_news_em`]
//! - Baidu finance calendar (economic data + trading reminders) — `news_economic_baidu`,
//!   `news_trade_notify_suspend_baidu`, `news_trade_notify_dividend_baidu`, `news_report_time_baidu`
//! - OwnThink knowledge graph / Q&A — [`nlp_ownthink`], [`nlp_answer`]
//!
//! Source-resilience notes (see implementation report): akshare's `event/` package in this
//! checkout contains only city/province mapping tables (no network endpoints), and there is no
//! `nlp_sentiment` / `event_economic_em` / `event_stock_open` / `event_stock_close`. Those named
//! endpoints are therefore skipped; the real JSON functions above are ported instead. The Baidu
//! calendar requires a caller-supplied cookie because the shared [`crate::core::client::Client`]
//! has no cookie store (the upstream cookie handshake is a two-step browser session, not JS signing).

pub mod baidu_calendar;
pub mod cls;
pub mod cninfo_irm;
pub mod nlp_ownthink;
pub mod stock_news;

pub use baidu_calendar::{
    DividendRow, EventRow, ReportRow, SuspendRow, news_economic_baidu, news_report_time_baidu,
    news_trade_notify_dividend_baidu, news_trade_notify_suspend_baidu,
};
pub use cls::{ClsTelegraphRow, telegraph};
pub use cninfo_irm::{CninfoIrmRow, cninfo_irm};
pub use nlp_ownthink::{KnowledgeRow, nlp_answer, nlp_ownthink};
pub use stock_news::{NewsRow, stock_news_em};

/// Extract a string field from an upstream JSON object, defaulting to `""`.
pub(crate) fn fstr(item: &serde_json::Value, k: &str) -> String {
    item.get(k)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

/// Extract a numeric field (accepts JSON number or numeric string), or `None`.
pub(crate) fn fnum(item: &serde_json::Value, k: &str) -> Option<f64> {
    match item.get(k) {
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        Some(serde_json::Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    }
}
