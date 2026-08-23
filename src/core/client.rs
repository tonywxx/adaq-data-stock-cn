use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tokio::sync::Semaphore;

use crate::core::error::{Error, Result};
#[cfg(target_os = "macos")]
use crate::core::impersonate;
use crate::core::resilience::{CacheLayer, RateLimiter, RetryPolicy};
use crate::core::util::urlencode;

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

/// Reqwest-backed HTTP client with resilience: retry/backoff, per-source rate
/// limiting, global concurrency cap, and an optional on-disk response cache
/// (ADR-0009). One of the two [`Backend`]s behind the unified [`Client`].
///
/// The retry/rate-limit/cache logic lives in [`crate::core::resilience`] so the
/// `impersonate` backend shares the exact same implementation (see C2).
#[derive(Clone)]
struct ReqwestBackend {
    inner: reqwest::Client,
    sem: Arc<Semaphore>,
    rate_limiter: RateLimiter,
    retry_policy: RetryPolicy,
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
            sem: Arc::new(Semaphore::new(cfg.max_concurrency.max(1))),
            rate_limiter: RateLimiter::new(cfg.per_source_rps),
            retry_policy: RetryPolicy::new(cfg.max_retries, cfg.base_backoff),
            cache,
        }
    }

    /// Read a still-fresh cached JSON response, if caching is on.
    fn cached(&self, source: &str, endpoint: &str, params: &[(&str, &str)]) -> Option<Value> {
        let cache = self.cache.as_ref()?;
        let key = cache.key(source, endpoint, params);
        cache.read(&key)
    }

    /// Best-effort write of a response to the on-disk cache.
    fn store(&self, source: &str, endpoint: &str, params: &[(&str, &str)], v: &Value) {
        if let Some(cache) = &self.cache {
            let key = cache.key(source, endpoint, params);
            cache.write(&key, v);
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
        if let Some(v) = self.cached(source, endpoint, params) {
            return Ok(v);
        }

        let resp = self
            .fetch_with_retry(source, reqwest::Method::GET, url, params, None, None)
            .await?;
        let value: Value = resp.json().await.map_err(Error::Http)?;

        self.store(source, endpoint, params, &value);
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
        if let Some(v) = self.cached(source, endpoint, params) {
            return Ok(v);
        }

        let resp = self
            .fetch_with_retry(source, reqwest::Method::GET, url, params, None, headers)
            .await?;
        let value: Value = resp.json().await.map_err(Error::Http)?;

        self.store(source, endpoint, params, &value);
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
        let rl = &self.rate_limiter;
        let inner = self.inner.clone();
        let url = url.to_string();
        let params = params.to_vec();
        let body = body.cloned();
        let headers = headers.map(|h| h.to_vec());
        // Retry/backoff now lives in `crate::core::resilience::RetryPolicy`,
        // shared with the impersonate backend.
        self.retry_policy
            .run(|| {
                let inner = inner.clone();
                let method = method.clone();
                let url = url.clone();
                let params = params.clone();
                let body = body.clone();
                let headers = headers.clone();
                async move {
                    rl.acquire(source).await;
                    let mut req = inner.request(method, &url);
                    if let Some(b) = body {
                        req = req.json(&b);
                    } else {
                        req = req.query(&params);
                    }
                    if let Some(h) = headers {
                        for (k, v) in h {
                            req = req.header(k, v);
                        }
                    }
                    let resp = match req.send().await {
                        Ok(r) => r,
                        Err(e) => return Err(Error::Http(e)),
                    };
                    if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        let retry_after = resp
                            .headers()
                            .get(reqwest::header::RETRY_AFTER)
                            .and_then(|v| v.to_str().ok())
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(1);
                        let _ = resp.bytes().await;
                        tokio::time::sleep(Duration::from_secs(retry_after)).await;
                        return Err(Error::RateLimited);
                    }
                    if !resp.status().is_success() {
                        return Err(Error::UpstreamChanged {
                            origin: source,
                            message: format!("HTTP {}", resp.status()),
                        });
                    }
                    Ok(resp)
                }
            })
            .await
    }
}

/// Which HTTP backend a [`Client`] dispatches to. Both implement the same
/// method surface, so callers depend on one `Client` interface and the
/// browser-impersonation (anti-bot) backend is reachable without a second type
/// (ADR-0009). Both backends share the same retry/backoff, per-source rate
/// limiting and on-disk cache via `crate::core::resilience` (C2): the reqwest
/// backend additionally enforces a global concurrency cap; the impersonate
/// backend additionally provides a real Chrome TLS/HTTP2 fingerprint.
#[derive(Clone)]
enum Backend {
    Reqwest(ReqwestBackend),
    // macOS-only: links the vendored `libcurl-impersonate` dylib.
    #[cfg(target_os = "macos")]
    Impersonate(impersonate::Client),
}


/// Append `params` as a query string to `url` (used by the macOS-only
/// impersonate backend, which takes a full URL rather than separate params).
#[cfg(target_os = "macos")]
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
    /// behind anti-bot WAFs (Sina, Baidu, …). It now shares the reqwest
    /// backend's retry/backoff + per-source rate-limiting via
    /// `crate::core::resilience` (C2); only the global concurrency cap is
    /// reqwest-specific. Cache stays off unless explicitly enabled.
    ///
    /// Only available on macOS, which links the `libcurl-impersonate` dylib.
    #[cfg(target_os = "macos")]
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
            #[cfg(target_os = "macos")]
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
            #[cfg(target_os = "macos")]
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
            #[cfg(target_os = "macos")]
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
            #[cfg(target_os = "macos")]
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
            #[cfg(target_os = "macos")]
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
            #[cfg(target_os = "macos")]
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

    // macOS only: `with_impersonate` links the `libcurl-impersonate` dylib.
    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn impersonate_backend_dispatches() {
        assert!(unreachable(&Client::with_impersonate()).await);
    }
}
