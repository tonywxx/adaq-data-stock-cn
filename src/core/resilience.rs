//! Shared HTTP resilience primitives used by both the `reqwest` and
//! `impersonate` backends (see architecture review C2).
//!
//! Before this module, the `reqwest` backend carried full retry/backoff,
//! per-source rate limiting, and an on-disk cache, while the `impersonate`
//! backend had none — a silent correctness gap for Sina/Baidu/jisilu traffic.
//! Centralizing the policy and state here means both backends share one
//! implementation, so a fix to backoff or rate-limit behavior lands once.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use crate::core::error::{Error, Result};

/// Per-source token-bucket-ish rate limiter.
///
/// Holds the last-seen instant per source and sleeps just enough to honor
/// `per_source_rps`. `None` rps disables limiting. Cheap to clone (shared
/// `Arc<Mutex>`), so the same limiter can guard every request on a `Client`.
#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<HashMap<&'static str, Instant>>>,
    per_source_rps: Option<f64>,
}

impl RateLimiter {
    pub fn new(per_source_rps: Option<f64>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            per_source_rps,
        }
    }

    /// Block until the caller may issue a request to `source` under the
    /// configured per-source rate. No-op when rate limiting is disabled.
    pub async fn acquire(&self, source: &'static str) {
        let rps = match self.per_source_rps {
            Some(r) if r > 0.0 => r,
            _ => return,
        };
        let min_interval = Duration::from_secs_f64(1.0 / rps);
        let mut guard = self.inner.lock().await;
        match guard.get(&source) {
            Some(last) if last.elapsed() < min_interval => {
                let wait = min_interval - last.elapsed();
                drop(guard);
                tokio::time::sleep(wait).await;
                let mut g = self.inner.lock().await;
                g.insert(source, Instant::now());
            }
            _ => {
                guard.insert(source, Instant::now());
            }
        }
    }
}

/// Exponential-backoff retry policy.
#[derive(Clone, Copy, Debug)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_backoff: Duration,
}

impl RetryPolicy {
    pub fn new(max_retries: u32, base_backoff: Duration) -> Self {
        Self {
            max_retries,
            base_backoff,
        }
    }

    /// Backoff before attempt `attempt` (0-based), doubling each time and
    /// capped at 10s — mirrors the previous `ReqwestBackend` behavior.
    fn backoff(&self, attempt: u32) -> Duration {
        let factor = 2u32.saturating_pow(attempt.saturating_sub(1));
        let ms = (self.base_backoff.as_millis() as u64) * factor as u64;
        Duration::from_millis(ms).min(Duration::from_secs(10))
    }

    /// Run `op`, retrying transient failures with exponential backoff.
    ///
    /// Retryable errors are network failures (`Http`), rate-limit rejections
    /// (`RateLimited`), and impersonate transport errors (`Impersonate`).
    /// Non-retryable errors (parse failures, schema drift, explicit upstream
    /// "not found") surface immediately.
    pub async fn run<T, F, Fut>(&self, mut op: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempt: u32 = 0;
        loop {
            match op().await {
                Ok(v) => return Ok(v),
                Err(e) => {
                    if attempt >= self.max_retries || !is_retryable(&e) {
                        return Err(e);
                    }
                    tokio::time::sleep(self.backoff(attempt)).await;
                    attempt += 1;
                }
            }
        }
    }
}

fn is_retryable(e: &Error) -> bool {
    matches!(
        e,
        Error::Http(_) | Error::RateLimited | Error::Impersonate(_)
    )
}

/// On-disk response cache, shared by both backends for GET/POST-JSON reads.
#[derive(Clone)]
pub struct CacheLayer {
    pub dir: PathBuf,
    pub ttl: Duration,
}

impl CacheLayer {
    /// Stable cache key from source + endpoint + params.
    pub fn key(&self, source: &str, endpoint: &str, params: &[(&str, &str)]) -> String {
        let mut s = format!("{source}:{endpoint}");
        for (k, v) in params {
            s.push_str(&format!("&{k}={v}"));
        }
        let mut h = Sha256::new();
        h.update(s.as_bytes());
        let digest = h.finalize();
        let mut out = String::with_capacity(digest.len() * 2);
        for b in digest.iter() {
            out.push_str(&format!("{:02x}", b));
        }
        out
    }

    pub fn path(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{key}.json"))
    }

    /// Read a still-fresh cached `Value`, or `None`.
    pub fn read(&self, key: &str) -> Option<Value> {
        let p = self.path(key);
        let data = std::fs::read(&p).ok()?;
        let meta = std::fs::metadata(&p).ok()?;
        let modified = meta.modified().ok()?;
        if modified.elapsed().ok()? >= self.ttl {
            return None;
        }
        serde_json::from_slice(&data).ok()
    }

    /// Best-effort write; cache failures are swallowed (never fail a request).
    pub fn write(&self, key: &str, v: &Value) {
        if let Ok(bytes) = serde_json::to_vec(v) {
            let _ = std::fs::write(self.path(key), bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rate_limiter_disabled_is_noop() {
        let rl = RateLimiter::new(None);
        // Should return immediately without panicking.
        rl.acquire("test").await;
    }

    #[test]
    fn cache_key_is_stable() {
        let c = CacheLayer {
            dir: PathBuf::from("/tmp"),
            ttl: Duration::from_secs(60),
        };
        assert_eq!(c.key("em", "ep", &[("a", "1")]), c.key("em", "ep", &[("a", "1")]));
        assert_ne!(c.key("em", "ep", &[("a", "1")]), c.key("em", "ep", &[("a", "2")]));
    }

    #[tokio::test]
    async fn retry_policy_gives_up_after_max() {
        let policy = RetryPolicy::new(2, Duration::from_millis(1));
        let mut calls = 0u32;
        let err = policy
            .run(|| {
                calls += 1;
                async move { Err::<(), _>(Error::Impersonate("boom".into())) }
            })
            .await;
        assert!(err.is_err());
        // initial attempt + 2 retries = 3 total invocations
        assert_eq!(calls, 3);
    }

    #[tokio::test]
    async fn retry_policy_does_not_retry_parse_errors() {
        let policy = RetryPolicy::new(3, Duration::from_millis(1));
        let mut calls = 0u32;
        let _ = policy
            .run(|| {
                calls += 1;
                async move { Err::<(), _>(Error::Parse { endpoint: "x", message: "m".into() }) }
            })
            .await;
        assert_eq!(calls, 1);
    }
}
