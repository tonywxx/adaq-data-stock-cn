use crate::core::client::Client;
use crate::core::error::Result;

pub mod chinamoney;
pub mod eastmoney;

pub use chinamoney::{RepoRate, repo_rate_hist, repo_rate_query};
pub use eastmoney::{InterbankRate, rate_interbank};

/// ChinaMoney source identifier (used for rate-limit buckets / error context).
pub(crate) const SOURCE_CHINAMONEY: &str = "chinamoney";

/// Aggregated repo fixing-rate series with single-source-chained fallback
/// (ADR-0010 style): the history POST endpoint first, then the ChinaMoney CSV
/// "query" endpoint filtered to the requested date window. Both normalize into
/// the canonical [`RepoRate`] type.
///
/// Note: akshare's `rate` package only ships ChinaMoney as an upstream for repo
/// fixing rates, so this is a same-origin fallback rather than a cross-source one.
pub async fn repo_rate(client: &Client, start_date: &str, end_date: &str) -> Result<Vec<RepoRate>> {
    match chinamoney::repo_rate_hist(client, start_date, end_date).await {
        Ok(rows) => Ok(rows),
        Err(_) => {
            let mut rows = chinamoney::repo_rate_query(client, "回购定盘利率").await?;
            let sd = chinamoney::fmt_date(start_date);
            let ed = chinamoney::fmt_date(end_date);
            rows.retain(|r| r.date.as_str() >= sd.as_str() && r.date.as_str() <= ed.as_str());
            Ok(rows)
        }
    }
}
