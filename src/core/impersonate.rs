//! Browser-impersonation HTTP backend — the Rust analog of the Python
//! [`primp`](https://github.com/deedy5/primp) (curl_cffi) library.
//!
//! The default [`crate::core::client::Client`] uses `reqwest` + rustls, whose
//! TLS/HTTP2 handshake is trivially fingerprinted by anti-bot middleboxes
//! (Akamai, Cloudflare, Sina/Baidu WAFs). This module wraps
//! [`impersonate_rs`], which links `curl-impersonate` and replays a real
//! Chrome/Edge/Firefox ClientHello + ALPN + HTTP2 settings, so our requests
//! look like a genuine browser.
//!
//! `impersonate_rs` is a **synchronous** (blocking curl) client, so every
//! call is dispatched onto `tokio::task::spawn_blocking` to avoid stalling the
//! async runtime. The public methods are async and mirror the existing
//! `Client` API (`get_text`, `get_json`, `post_form_*`, …).
//!
//! ## Native library
//!
//! The crate links the vendored `libcurl-impersonate-chrome` shared library
//! (see `native/libcurl-impersonate/` and `.cargo/config.toml`). No system
//! install or `sudo` is required.
//!
//! ## GBK decoding
//!
//! Several Chinese sources (Sina, Baidu, jisilu) return GBK/GB2312 text. The
//! underlying crate's `text()` is strict UTF-8 and panics on those bytes, so
//! we always pull raw [`Response::bytes`] and decode via [`encoding_rs`]
//! (`GBK` with UTF-8/BOM fallback) — same strategy as
//! `stock_fundamental_html_gaps.rs`.

use std::collections::HashMap;
use std::time::Duration;

use impersonate_rs::{Browser, Client as ImpClient};
use serde_json::Value;

use crate::core::error::{Error, Result};

/// Default browser profile to impersonate. Chrome 131 is broadly accepted and
/// recent enough to clear most WAFs while staying stable in curl-impersonate.
pub const DEFAULT_BROWSER: Browser = Browser::Chrome131;

/// Default per-request timeout for impersonated requests.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);

/// Default Accept-Language, matching a typical zh-CN desktop Chrome.
const DEFAULT_ACCEPT_LANGUAGE: &str = "zh-CN,zh;q=0.9,en;q=0.8";

/// A browser-impersonating HTTP client (async wrapper over `impersonate_rs`).
#[derive(Clone)]
pub struct Client {
    inner: ImpClient,
    timeout: Duration,
}

impl Client {
    /// Build a client that impersonates [`DEFAULT_BROWSER`] (Chrome 131) with
    /// sensible desktop-Chrome default headers.
    pub fn new() -> Self {
        Self::with_browser(DEFAULT_BROWSER)
    }

    /// Build a client impersonating a specific [`Browser`] profile.
    pub fn with_browser(browser: Browser) -> Self {
        let inner = ImpClient::builder()
            .impersonate(browser)
            .timeout(DEFAULT_TIMEOUT)
            .build();
        Self {
            inner,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Fetch a URL as text, decoding GBK/GB2312/UTF-8 automatically.
    ///
    /// `headers` lets callers attach per-request headers (e.g. Sina's
    /// required `Referer`). The client already sends realistic Chrome headers.
    pub async fn get_text(
        &self,
        url: &str,
        headers: Option<&[(&str, &str)]>,
    ) -> Result<String> {
        self.fetch_text("GET", url, None, headers).await
    }

    /// POST form-encoded params and return the decoded text response.
    pub async fn post_form_text(
        &self,
        url: &str,
        form: &[(&str, &str)],
        headers: Option<&[(&str, &str)]>,
    ) -> Result<String> {
        self.fetch_text("POST", url, Some(form), headers).await
    }

    /// Fetch a URL and parse the response as JSON (`serde_json::Value`).
    pub async fn get_json(
        &self,
        url: &str,
        headers: Option<&[(&str, &str)]>,
    ) -> Result<Value> {
        let text = self.get_text(url, headers).await?;
        serde_json::from_str(&text).map_err(Error::Json)
    }

    /// POST form-encoded params and parse the JSON response.
    pub async fn post_form_json(
        &self,
        url: &str,
        form: &[(&str, &str)],
        headers: Option<&[(&str, &str)]>,
    ) -> Result<Value> {
        let text = self.post_form_text(url, form, headers).await?;
        serde_json::from_str(&text).map_err(Error::Json)
    }

    async fn fetch_text(
        &self,
        method: &str,
        url: &str,
        form: Option<&[(&str, &str)]>,
        headers: Option<&[(&str, &str)]>,
    ) -> Result<String> {
        let inner = self.inner.clone();
        let timeout = self.timeout;
        let url = url.to_string();
        let method = method.to_string();
        let extra_headers: Vec<(String, String)> = headers
            .map(|h| h.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect())
            .unwrap_or_default();
        let form_pairs: Vec<(String, String)> = form
            .map(|f| f.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect())
            .unwrap_or_default();

        tokio::task::spawn_blocking(move || {
            // Build the request. `impersonate_rs::Client` is cheap to clone and
            // already carries the browser profile + default Chrome headers.
            let mut req = match method.as_str() {
                "POST" => inner.post(&url),
                _ => inner.get(&url),
            };
            req = req.timeout(timeout);

            for (k, v) in &extra_headers {
                req = req.header(k, v).map_err(|e| Error::Impersonate(e.to_string()))?;
            }
            if !form_pairs.is_empty() {
                // URL-encode the form body manually (avoids an extra serde dep
                // round-trip in the builder and keeps ordering explicit).
                let body = form_pairs
                    .iter()
                    .map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v)))
                    .collect::<Vec<_>>()
                    .join("&");
                req = req.body(body);
            }
            let resp = req.send().map_err(|e| Error::Impersonate(e.to_string()))?;
            if resp.status() >= 400 {
                return Err(Error::UpstreamChanged {
                    origin: "impersonate",
                    message: format!("HTTP {}", resp.status()),
                });
            }
            let bytes = resp.bytes();
            Ok(decode_body(bytes))
        })
        .await
        .map_err(|e| Error::Impersonate(format!("task join error: {e}")))?
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

/// Decode an HTTP body, preferring UTF-8 (with BOM) and falling back to GBK,
/// which covers the GB2312/GBK pages returned by Sina, Baidu, jisilu, etc.
///
/// `impersonate_rs::Response::text()` is strict UTF-8 and panics on GBK bytes,
/// so callers must use [`impersonate_rs::Response::bytes`] + this helper.
pub fn decode_body(bytes: &[u8]) -> String {
    // Fast path: valid UTF-8.
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    // BOM-aware UTF-8.
    if bytes.starts_with(b"\xef\xbb\xbf")
        && let Ok(s) = std::str::from_utf8(&bytes[3..])
    {
        return s.to_string();
    }
    // Fall back to GBK (superset of GB2312).
    match encoding_rs::GBK.decode_without_bom_handling_and_without_replacement(bytes) {
        Some(s) => s.into_owned(),
        None => String::from_utf8_lossy(bytes).into_owned(),
    }
}

/// Minimal percent-encoder for form bodies (RFC 3986, encode spaces as `+`
/// and everything non-alphanumeric as `%XX`). Mirrors how browsers submit
/// `application/x-www-form-urlencoded`.
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
                out.push(hex_digit(b >> 4));
                out.push(hex_digit(b & 0x0f));
            }
        }
    }
    out
}

fn hex_digit(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        _ => (b'A' + (n - 10)) as char,
    }
}

/// Convenience: a one-shot impersonated GET returning decoded text.
pub async fn get_text(url: &str, headers: Option<&[(&str, &str)]>) -> Result<String> {
    Client::new().get_text(url, headers).await
}

/// Build a standard Sina `Referer` header (Sina's realtime/qt endpoints
/// reject requests without a `Referer` of a Sina property with HTTP 403).
pub fn sina_referer() -> (&'static str, &'static str) {
    ("Referer", "https://finance.sina.com.cn/")
}

/// Combine the client's default headers with caller-supplied ones into a
/// single `HashMap` (exposed for tests / debugging).
pub fn default_headers_map() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("Accept-Language", DEFAULT_ACCEPT_LANGUAGE);
    m.insert("Accept", "*/*");
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_utf8_passthrough() {
        let s = "hello 世界";
        assert_eq!(decode_body(s.as_bytes()), s);
    }

    #[test]
    fn decode_gbk_roundtrip() {
        // "测试" in GBK
        let gbk = [0xb2, 0xe2, 0xca, 0xd4];
        assert_eq!(decode_body(&gbk), "测试");
    }

    #[test]
    fn urlencode_basic() {
        assert_eq!(urlencode("a b"), "a+b");
        assert_eq!(urlencode("a&b=c"), "a%26b%3Dc");
    }

    // Live integration test — requires network + the vendored
    // `libcurl-impersonate` dylib (see `.cargo/config.toml` + `native/`).
    // Ignored by default so offline/CI builds stay green. Run with:
    //   cargo test --lib core::impersonate::tests::live_sources -- --ignored --nocapture
    //
    // Validates the impersonation backend reaches real anti-bot-prone sources
    // (Sina realtime quotes need a Sina `Referer`; Baidu is header/UA gated).
    // NOTE: Eastmoney `push2his` is intentionally excluded — that host rejects
    // the curl-impersonate TLS/HTTP2 handshake (Curl 56) but is served fine by
    // the standard `reqwest` `Client`, so it does not need impersonation.
    #[tokio::test]
    #[ignore = "live network + native lib; run explicitly"]
    async fn live_sources() {
        use std::time::Duration;
        let client = Client::new();
        let cases: &[(&str, &str, Option<&[(&str, &str)]>)] = &[
            ("gtimg", "https://qt.gtimg.cn/q=sh600000", None),
            ("sina", "https://hq.sinajs.cn/list=sh600000", Some(&[sina_referer()])),
            ("baidu", "https://www.baidu.com/", None),
        ];
        for (name, url, hdrs) in cases {
            let res = tokio::time::timeout(Duration::from_secs(20), client.get_text(url, *hdrs))
                .await;
            match res {
                Ok(Ok(body)) => assert!(!body.is_empty(), "{name}: empty body"),
                Ok(Err(e)) => panic!("{name}: request failed: {e}"),
                Err(_) => panic!("{name}: request timed out"),
            }
        }
    }
}
