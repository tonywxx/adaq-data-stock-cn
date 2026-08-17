use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::sync::{Mutex, Semaphore};

use crate::core::error::{Error, Result};
use crate::core::impersonate;

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

/// Reqwest-backed HTTP client with resilience: retry/backoff, per-source rate
/// limiting, global concurrency cap, and an optional on-disk response cache
/// (ADR-0009). One of the two [`Backend`]s behind the unified [`Client`].
#[derive(Clone)]
struct ReqwestBackend {
    inner: reqwest::Client,
    cfg: ClientConfig,
    sem: Arc<Semaphore>,
    rate: Arc<Mutex<HashMap<&'static str, Instant>>>,
    cache: Option<CacheLayer>,
}

impl ReqwestBackend {
    fn with_config(cfg: ClientConfig) -> Self {
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
        let digest = h.finalize();
        let mut out = String::with_capacity(digest.len() * 2);
        for b in digest.iter() {
            out.push_str(&format!("{:02x}", b));
        }
        out
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
    async fn get_json(
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
            .fetch_with_retry(source, reqwest::Method::GET, url, params, None, None)
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
    async fn get_json_with_headers(
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
            .fetch_with_retry(source, reqwest::Method::GET, url, params, None, headers)
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
    async fn get_text(
        &self,
        source: &'static str,
        _endpoint: &'static str,
        url: &str,
        params: &[(&str, &str)],
        headers: Option<&[(&str, &str)]>,
    ) -> Result<String> {
        let resp = self
            .fetch_with_retry(source, reqwest::Method::GET, url, params, None, headers)
            .await?;
        resp.text().await.map_err(Error::Http)
    }

    /// POST form-encoded params and parse the JSON response. Same resilience
    /// (retry/backoff, rate limiting, concurrency cap) as [`get_json`].
    ///
    /// Used by sources that require a POST (e.g. ChinaMoney). Caching is not
    /// applied to POSTs (ADR-0009).
    async fn post_form_json(
        &self,
        source: &'static str,
        _endpoint: &'static str,
        url: &str,
        params: &[(&str, &str)],
        headers: Option<&[(&str, &str)]>,
    ) -> Result<Value> {
        let resp = self
            .fetch_with_retry(source, reqwest::Method::POST, url, params, None, headers)
            .await?;
        resp.json().await.map_err(Error::Http)
    }

    /// POST form-encoded params and return the raw text response (for sources
    /// that return HTML, not JSON). Same resilience as [`get_json`].
    ///
    /// Used by exchange COT pages (e.g. DCE member position ranks) that answer
    /// a form POST with an HTML table. Caching is not applied to POSTs (ADR-0009).
    async fn post_form_text(
        &self,
        source: &'static str,
        _endpoint: &'static str,
        url: &str,
        params: &[(&str, &str)],
        headers: Option<&[(&str, &str)]>,
    ) -> Result<String> {
        let resp = self
            .fetch_with_retry(source, reqwest::Method::POST, url, params, None, headers)
            .await?;
        resp.text().await.map_err(Error::Http)
    }

    /// POST a JSON request body and parse the JSON response. Same resilience
    /// (retry/backoff, rate limiting, concurrency cap) as [`get_json`].
    ///
    /// Used by sources that require a JSON body (e.g. Eastmoney `emappdata`
    /// stock-rank endpoints). Caching is not applied to POSTs (ADR-0009).
    async fn post_json(
        &self,
        source: &'static str,
        _endpoint: &'static str,
        url: &str,
        body: &Value,
        headers: Option<&[(&str, &str)]>,
    ) -> Result<Value> {
        let resp = self
            .fetch_with_retry(
                source,
                reqwest::Method::POST,
                url,
                &[],
                Some(body),
                headers,
            )
            .await?;
        resp.json().await.map_err(Error::Http)
    }

    async fn fetch_with_retry(
        &self,
        source: &'static str,
        method: reqwest::Method,
        url: &str,
        params: &[(&str, &str)],
        body: Option<&Value>,
        headers: Option<&[(&str, &str)]>,
    ) -> Result<reqwest::Response> {
        // Hold a concurrency permit for the whole request lifecycle.
        let _permit = self.sem.acquire().await.map_err(|_| Error::RateLimited)?;

        let mut attempt: u32 = 0;
        loop {
            self.rate_limit(source).await;
            let mut req = self.inner.request(method.clone(), url);
            if let Some(b) = body {
                req = req.json(b);
            } else {
                req = req.query(params);
            }
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

/// Which HTTP backend a [`Client`] dispatches to. Both implement the same
/// method surface, so callers depend on one `Client` interface and the
/// browser-impersonation (anti-bot) backend is reachable without a second type
/// (ADR-0009). The reqwest backend carries full resilience; the impersonate
/// backend trades that for a real Chrome TLS/HTTP2 fingerprint.
#[derive(Clone)]
enum Backend {
    Reqwest(ReqwestBackend),
    Impersonate(impersonate::Client),
}

const URLSAFE_HEX: &[u8; 16] = b"0123456789ABCDEF";

/// Percent-encode query params (RFC 3986, spaces as `+`).
fn urlencode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push(URLSAFE_HEX[(b >> 4) as usize] as char);
                out.push(URLSAFE_HEX[(b & 0x0f) as usize] as char);
            }
        }
    }
    out
}

/// Append `params` as a query string to `url` (used by the impersonate backend,
/// which takes a full URL rather than separate params).
fn with_query(url: &str, params: &[(&str, &str)]) -> String {
    if params.is_empty() {
        return url.to_string();
    }
    let q: String = params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v)))
        .collect::<Vec<_>>()
        .join("&");
    if url.contains('?') {
        format!("{url}&{q}")
    } else {
        format!("{url}?{q}")
    }
}

/// Shared HTTP client with a pluggable backend (ADR-0009).
///
/// Construct with [`Client::new`] / [`Client::with_config`] for the resilient
/// reqwest backend, or [`Client::with_impersonate`] for the browser-impersonation
/// backend that clears anti-bot WAFs. Both expose the identical method surface
/// (`get_json`, `get_text`, `post_form_*`, `post_json`), so endpoint code never
/// branches on the backend.
#[derive(Clone)]
pub struct Client {
    backend: Backend,
}

impl Client {
    /// Resilient reqwest backend with default config.
    pub fn new() -> Self {
        Self::with_config(ClientConfig::default())
    }

    /// Resilient reqwest backend with explicit config.
    pub fn with_config(cfg: ClientConfig) -> Self {
        Self {
            backend: Backend::Reqwest(ReqwestBackend::with_config(cfg)),
        }
    }

    /// Browser-impersonation backend (Chrome TLS/HTTP2 fingerprint) for sources
    /// behind anti-bot WAFs (Sina, Baidu, …). Note: this backend does not apply
    /// the reqwest retry/rate-limit/cache resilience — it trades that for a
    /// genuine browser handshake.
    pub fn with_impersonate() -> Self {
        Self {
            backend: Backend::Impersonate(impersonate::Client::new()),
        }
    }

    /// Fetch a JSON endpoint.
    pub async fn get_json(
        &self,
        source: &'static str,
        endpoint: &'static str,
        url: &str,
        params: &[(&str, &str)],
    ) -> Result<Value> {
        match &self.backend {
            Backend::Reqwest(b) => b.get_json(source, endpoint, url, params).await,
            Backend::Impersonate(c) => {
                let text = c.get_text(&with_query(url, params), None).await?;
                serde_json::from_str(&text).map_err(Error::Json)
            }
        }
    }

    /// Fetch a JSON endpoint with optional per-request `headers`.
    pub async fn get_json_with_headers(
        &self,
        source: &'static str,
        endpoint: &'static str,
        url: &str,
        params: &[(&str, &str)],
        headers: Option<&[(&str, &str)]>,
    ) -> Result<Value> {
        match &self.backend {
            Backend::Reqwest(b) => {
                b.get_json_with_headers(source, endpoint, url, params, headers)
                    .await
            }
            Backend::Impersonate(c) => {
                let text = c.get_text(&with_query(url, params), headers).await?;
                serde_json::from_str(&text).map_err(Error::Json)
            }
        }
    }

    /// Fetch a text endpoint.
    pub async fn get_text(
        &self,
        source: &'static str,
        endpoint: &'static str,
        url: &str,
        params: &[(&str, &str)],
        headers: Option<&[(&str, &str)]>,
    ) -> Result<String> {
        match &self.backend {
            Backend::Reqwest(b) => b.get_text(source, endpoint, url, params, headers).await,
            Backend::Impersonate(c) => c.get_text(&with_query(url, params), headers).await,
        }
    }

    /// POST form-encoded params and parse the JSON response.
    pub async fn post_form_json(
        &self,
        source: &'static str,
        endpoint: &'static str,
        url: &str,
        params: &[(&str, &str)],
        headers: Option<&[(&str, &str)]>,
    ) -> Result<Value> {
        match &self.backend {
            Backend::Reqwest(b) => {
                b.post_form_json(source, endpoint, url, params, headers).await
            }
            Backend::Impersonate(c) => c.post_form_json(url, params, headers).await,
        }
    }

    /// POST form-encoded params and return the raw text response.
    pub async fn post_form_text(
        &self,
        source: &'static str,
        endpoint: &'static str,
        url: &str,
        params: &[(&str, &str)],
        headers: Option<&[(&str, &str)]>,
    ) -> Result<String> {
        match &self.backend {
            Backend::Reqwest(b) => b.post_form_text(source, endpoint, url, params, headers).await,
            Backend::Impersonate(c) => c.post_form_text(url, params, headers).await,
        }
    }

    /// POST a JSON request body and parse the JSON response.
    pub async fn post_json(
        &self,
        source: &'static str,
        endpoint: &'static str,
        url: &str,
        body: &Value,
        headers: Option<&[(&str, &str)]>,
    ) -> Result<Value> {
        match &self.backend {
            Backend::Reqwest(b) => b.post_json(source, endpoint, url, body, headers).await,
            Backend::Impersonate(c) => c.post_json(url, body, headers).await,
        }
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Offline smoke test: both backends dispatch through the same `Client`
    // interface and fail the same way on an unreachable host. Proves the seam
    // is real (no separate `ImpersonateClient` type to branch on).
    async fn unreachable(client: &Client) -> bool {
        client
            .get_json("test", "test", "http://127.0.0.1:1/", &[])
            .await
            .is_err()
    }

    #[tokio::test]
    async fn reqwest_backend_dispatches() {
        assert!(unreachable(&Client::new()).await);
    }

    #[tokio::test]
    async fn impersonate_backend_dispatches() {
        assert!(unreachable(&Client::with_impersonate()).await);
    }
}
