use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::sync::{Mutex, Semaphore};

use crate::core::error::{Error, Result};

/// Source identifiers, used for rate-limit buckets and error context.
pub const SOURCE_EASTMONEY: &str = "eastmoney";
pub const SOURCE_SINA: &str = "sina";
pub const SOURCE_TENCENT: &str = "tencent";

/// Tunable client behavior (ADR-0009).
#[derive(Clone, Debug)]
pub struct ClientConfig {
    /// Per-request timeout.
    pub timeout: Duration,
    /// Max retries on transient failures / 5xx (not on 429, which uses Retry-After).
    pub max_retries: u32,
    /// Base backoff, doubled each retry attempt (capped at 10s).
    pub base_backoff: Duration,
    /// Per-source rate limit in requests/sec. `None` disables rate limiting.
    pub per_source_rps: Option<f64>,
    /// Global concurrency cap across all in-flight requests.
    pub max_concurrency: usize,
    /// On-disk response cache. Off by default (ADR-0009).
    pub cache: CacheConfig,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(15),
            max_retries: 3,
            base_backoff: Duration::from_millis(300),
            per_source_rps: Some(5.0),
            max_concurrency: 8,
            cache: CacheConfig::Off,
        }
    }
}

#[derive(Clone, Debug)]
pub enum CacheConfig {
    Off,
    On { dir: PathBuf, ttl: Duration },
}

#[derive(Clone)]
struct CacheLayer {
    dir: PathBuf,
    ttl: Duration,
}

/// Shared HTTP client with resilience: retry/backoff, per-source rate limiting,
/// global concurrency cap, and an optional on-disk response cache (ADR-0009).
#[derive(Clone)]
pub struct Client {
    inner: reqwest::Client,
    cfg: ClientConfig,
    sem: Arc<Semaphore>,
    rate: Arc<Mutex<HashMap<&'static str, Instant>>>,
    cache: Option<CacheLayer>,
}

impl Client {
    pub fn new() -> Self {
        Self::with_config(ClientConfig::default())
    }

    pub fn with_config(cfg: ClientConfig) -> Self {
        let inner = reqwest::Client::builder()
            .timeout(cfg.timeout)
            .user_agent("Mozilla/5.0 (compatible; adaq-data-stock-cn/0.1)")
            .build()
            .expect("failed to build reqwest client");
        let cache = match &cfg.cache {
            CacheConfig::Off => None,
            CacheConfig::On { dir, ttl } => {
                let _ = std::fs::create_dir_all(dir);
                Some(CacheLayer {
                    dir: dir.clone(),
                    ttl: *ttl,
                })
            }
        };
        Self {
            inner,
            cfg: cfg.clone(),
            sem: Arc::new(Semaphore::new(cfg.max_concurrency.max(1))),
            rate: Arc::new(Mutex::new(HashMap::new())),
            cache,
        }
    }

    fn cache_key(&self, source: &str, endpoint: &str, params: &[(&str, &str)]) -> String {
        use sha2::{Digest, Sha256};
        let mut s = format!("{source}:{endpoint}");
        for (k, v) in params {
            s.push_str(&format!("&{k}={v}"));
        }
        let mut h = Sha256::new();
        h.update(s.as_bytes());
        format!("{:x}", h.finalize())
    }

    fn cache_path(&self, key: &str) -> PathBuf {
        self.cache.as_ref().unwrap().dir.join(format!("{key}.json"))
    }

    async fn rate_limit(&self, source: &'static str) {
        let rps = match self.cfg.per_source_rps {
            Some(r) if r > 0.0 => r,
            _ => return,
        };
        let min_interval = Duration::from_secs_f64(1.0 / rps);
        let mut guard = self.rate.lock().await;
        match guard.get(&source) {
            Some(last) if last.elapsed() < min_interval => {
                let wait = min_interval - last.elapsed();
                drop(guard);
                tokio::time::sleep(wait).await;
                let mut g = self.rate.lock().await;
                g.insert(source, Instant::now());
            }
            _ => {
                guard.insert(source, Instant::now());
            }
        }
    }

    /// Fetch a JSON endpoint, with retry/backoff, rate limiting, concurrency cap and optional cache.
    pub async fn get_json(
        &self,
        source: &'static str,
        endpoint: &'static str,
        url: &str,
        params: &[(&str, &str)],
    ) -> Result<Value> {
        let key = self.cache_key(source, endpoint, params);
        if let Some(cache) = &self.cache {
            let p = self.cache_path(&key);
            if let (Ok(data), Ok(meta)) = (std::fs::read(&p), std::fs::metadata(&p))
                && let Ok(modified) = meta.modified()
                && modified.elapsed().map(|e| e < cache.ttl).unwrap_or(false)
                && let Ok(v) = serde_json::from_slice::<Value>(&data)
            {
                return Ok(v);
            }
        }

        let resp = self
            .fetch_with_retry(source, reqwest::Method::GET, url, params, None)
            .await?;
        let value: Value = resp.json().await.map_err(Error::Http)?;

        if let Some(_cache) = &self.cache {
            let p = self.cache_path(&key);
            if let Ok(bytes) = serde_json::to_vec(&value) {
                let _ = std::fs::write(p, bytes);
            }
        }
        Ok(value)
    }

    /// Fetch a JSON endpoint with optional per-request `headers` (e.g. Sina's
    /// `Referer`), layered on top of the client's default `User-Agent`. Same
    /// retry/backoff, rate limiting, concurrency cap and caching as [`get_json`].
    pub async fn get_json_with_headers(
        &self,
        source: &'static str,
        endpoint: &'static str,
        url: &str,
        params: &[(&str, &str)],
        headers: Option<&[(&str, &str)]>,
    ) -> Result<Value> {
        let key = self.cache_key(source, endpoint, params);
        if let Some(cache) = &self.cache {
            let p = self.cache_path(&key);
            if let (Ok(data), Ok(meta)) = (std::fs::read(&p), std::fs::metadata(&p))
                && let Ok(modified) = meta.modified()
                && modified.elapsed().map(|e| e < cache.ttl).unwrap_or(false)
                && let Ok(v) = serde_json::from_slice::<Value>(&data)
            {
                return Ok(v);
            }
        }

        let resp = self
            .fetch_with_retry(source, reqwest::Method::GET, url, params, headers)
            .await?;
        let value: Value = resp.json().await.map_err(Error::Http)?;

        if let Some(_cache) = &self.cache {
            let p = self.cache_path(&key);
            if let Ok(bytes) = serde_json::to_vec(&value) {
                let _ = std::fs::write(p, bytes);
            }
        }
        Ok(value)
    }

    /// Fetch a text endpoint (for sources that return non-JSON / lenient JSON). Same resilience as [`get_json`].
    ///
    /// `headers` lets a caller attach per-request headers (e.g. Sina's `Referer`),
    /// layered on top of the client's default `User-Agent`.
    pub async fn get_text(
        &self,
        source: &'static str,
        _endpoint: &'static str,
        url: &str,
        params: &[(&str, &str)],
        headers: Option<&[(&str, &str)]>,
    ) -> Result<String> {
        let resp = self
            .fetch_with_retry(source, reqwest::Method::GET, url, params, headers)
            .await?;
        resp.text().await.map_err(Error::Http)
    }

    /// POST form-encoded params and parse the JSON response. Same resilience
    /// (retry/backoff, rate limiting, concurrency cap) as [`get_json`].
    ///
    /// Used by sources that require a POST (e.g. ChinaMoney). Caching is not
    /// applied to POSTs (ADR-0009).
    pub async fn post_form_json(
        &self,
        source: &'static str,
        _endpoint: &'static str,
        url: &str,
        params: &[(&str, &str)],
        headers: Option<&[(&str, &str)]>,
    ) -> Result<Value> {
        let resp = self
            .fetch_with_retry(source, reqwest::Method::POST, url, params, headers)
            .await?;
        resp.json().await.map_err(Error::Http)
    }

    async fn fetch_with_retry(
        &self,
        source: &'static str,
        method: reqwest::Method,
        url: &str,
        params: &[(&str, &str)],
        headers: Option<&[(&str, &str)]>,
    ) -> Result<reqwest::Response> {
        // Hold a concurrency permit for the whole request lifecycle.
        let _permit = self.sem.acquire().await.map_err(|_| Error::RateLimited)?;

        let mut attempt: u32 = 0;
        loop {
            self.rate_limit(source).await;
            let mut req = self.inner.request(method.clone(), url).query(params);
            if let Some(h) = headers {
                for (k, v) in h {
                    req = req.header(*k, *v);
                }
            }
            let resp = match req.send().await {
                Ok(r) => r,
                Err(e) => {
                    attempt += 1;
                    if attempt > self.cfg.max_retries {
                        return Err(Error::Http(e));
                    }
                    self.backoff(attempt).await;
                    continue;
                }
            };

            if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                attempt += 1;
                if attempt > self.cfg.max_retries {
                    return Err(Error::RateLimited);
                }
                let retry_after = resp
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(1);
                let _ = resp.bytes().await;
                tokio::time::sleep(Duration::from_secs(retry_after)).await;
                continue;
            }

            if !resp.status().is_success() {
                attempt += 1;
                if attempt > self.cfg.max_retries {
                    return Err(Error::UpstreamChanged {
                        origin: source,
                        message: format!("HTTP {}", resp.status()),
                    });
                }
                self.backoff(attempt).await;
                continue;
            }

            return Ok(resp);
        }
    }

    async fn backoff(&self, attempt: u32) {
        let factor = 2u32.saturating_pow(attempt.saturating_sub(1));
        let ms = (self.cfg.base_backoff.as_millis() as u64) * factor as u64;
        let d = Duration::from_millis(ms).min(Duration::from_secs(10));
        tokio::time::sleep(d).await;
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
