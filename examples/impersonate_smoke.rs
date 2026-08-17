//! Live smoke test for the browser-impersonation backend (the primp technique).
//!
//! Demonstrates the Rust analog of the Python `primp`/`curl_cffi` lib:
//! `curl-impersonate` replays a real Chrome ClientHello so anti-bot-prone
//! sources accept our requests. Run:
//!   cargo run --example impersonate_smoke -- --nocapture
//!
//! (The binary loads the vendored `libcurl-impersonate` dylib via a baked
//! LC_RPATH — no sudo / DYLD_LIBRARY_PATH needed. See `.cargo/config.toml`.)
use adaq_data_stock_cn::core::impersonate::{self, sina_referer};
use std::time::Duration;

#[tokio::main]
async fn main() {
    let client = impersonate::Client::new();
    let cases: &[(&str, &str, Option<&[(&str, &str)]>)] = &[
        ("gtimg (Tencent)", "https://qt.gtimg.cn/q=sh600000", None),
        ("sina", "https://hq.sinajs.cn/list=sh600000", Some(&[sina_referer()])),
        ("baidu", "https://www.baidu.com/", None),
    ];
    for (name, url, hdrs) in cases {
        let res = tokio::time::timeout(Duration::from_secs(20), client.get_text(url, *hdrs)).await;
        match res {
            Ok(Ok(t)) => eprintln!("[{name}] OK len={} :: {}", t.len(), t.trim().chars().take(70).collect::<String>()),
            Ok(Err(e)) => eprintln!("[{name}] ERR {e}"),
            Err(_) => eprintln!("[{name}] TIMEOUT"),
        }
    }
}
