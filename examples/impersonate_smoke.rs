//! Live smoke test for the browser-impersonation backend (the primp technique).
//!
//! Demonstrates the Rust analog of the Python `primp`/`curl_cffi` lib:
//! `curl-impersonate` replays a real Chrome ClientHello so anti-bot-prone
//! sources accept our requests. Run:
//!   cargo run --example impersonate_smoke -- --nocapture
//!
//! (The binary loads the vendored `libcurl-impersonate` dylib via a baked
//! LC_RPATH — no sudo / DYLD_LIBRARY_PATH needed. See `.cargo/config.toml`.)
//!
//! The impersonation backend is macOS-only, so on Windows/Linux this example
//! compiles to an empty `main` stub and there is nothing to run.

#[cfg(target_os = "macos")]
mod real {
    use adaq_data_stock_cn::core::impersonate::{self, sina_referer};
    use std::time::Duration;

    pub async fn run() {
        let client = impersonate::Client::new();
        let cases: &[(&str, &str, Option<&[(&str, &str)]>)] = &[
            ("gtimg (Tencent)", "https://qt.gtimg.cn/q=sh600000", None),
            ("sina", "https://hq.sinajs.cn/list=sh600000", Some(&[sina_referer()])),
            ("baidu", "https://www.baidu.com/", None),
        ];
        for (name, url, hdrs) in cases {
            let res =
                tokio::time::timeout(Duration::from_secs(20), client.get_text(url, *hdrs)).await;
            match res {
                Ok(Ok(t)) => eprintln!(
                    "[{name}] OK len={} :: {}",
                    t.len(),
                    t.trim().chars().take(70).collect::<String>()
                ),
                Ok(Err(e)) => eprintln!("[{name}] ERR {e}"),
                Err(_) => eprintln!("[{name}] TIMEOUT"),
            }
        }
    }
}

#[cfg(target_os = "macos")]
#[tokio::main]
async fn main() {
    real::run().await;
}

// Non-macOS stub: keeping a valid (empty) `main` means `cargo build` /
// `cargo clippy --all-targets` succeed on Windows/Linux too.
#[cfg(not(target_os = "macos"))]
fn main() {}
