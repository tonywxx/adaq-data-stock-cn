//! Eastmoney `push2` host resolution.
//!
//! Eastmoney serves realtime quote data from a pool of mirror servers addressed
//! as `push{N}.eastmoney.com` for `N` in `2..=99` (e.g. `push2`, `push23`,
//! `push99`; the bare `push2.eastmoney.com` host *is* id 2).
//!
//! DEFAULT behavior: the first call to [`eastmoney_push_host`] / [`push2_url`]
//! probes a prioritized set of these mirrors and remembers the first one that
//! answers with valid data; every later call reuses that host (cached in a
//! `OnceCell`). So the whole process is pinned to a single working server, and
//! the host is verified to return data *before* any real request is served —
//! without probing on every call.
//!
//! OPTIONAL: if you would rather skip the startup probe, call
//! [`use_random_push2_host`] (or [`set_push2_host`]) to pin a random (or chosen)
//! host up front. `push2his.eastmoney.com` (kline / history) is a different host
//! and is intentionally NOT managed here.

use std::sync::OnceLock;
use std::time::Duration;

use serde_json::Value;
use tokio::sync::OnceCell;

/// Inclusive lower bound of the `push{N}.eastmoney.com` mirror ids (push2).
pub const PUSH2_MIN_ID: u32 = 2;
/// Inclusive upper bound of the `push{N}.eastmoney.com` mirror ids (push99).
pub const PUSH2_MAX_ID: u32 = 99;

/// Mirrors probed by default, ordered by historical usage in this crate.
const CANDIDATE_IDS: &[u32] = &[2, 5, 16, 23, 40, 48, 70, 88, 91, 95];

/// Resolved host, probed once on first use and cached for the process lifetime.
static PUSH2_HOST: OnceCell<String> = OnceCell::const_new();
/// Optional override (random or manually chosen) installed by [`set_push2_host`]
/// / [`use_random_push2_host`]; takes precedence over the probed default.
static OVERRIDE_HOST: OnceLock<String> = OnceLock::new();

/// Returns the active Eastmoney `push2` host (e.g. `push23.eastmoney.com`).
///
/// By default this probes [`CANDIDATE_IDS`] once (lazily, on the first call),
/// caches the first mirror that returns valid data, and serves every later call
/// from that cache — so the host is verified before it is used, but probing
/// happens only once. An override installed by [`set_push2_host`] /
/// [`use_random_push2_host`] takes precedence.
pub async fn eastmoney_push_host() -> String {
    if let Some(h) = OVERRIDE_HOST.get() {
        return h.clone();
    }
    PUSH2_HOST
        .get_or_init(|| async { resolve().await })
        .await
        .clone()
}

/// Build a full `push2` URL for `path` (e.g. `/api/qt/clist/get`) using the
/// active host. The first call probes/verifies the host; later calls reuse it.
///
/// ```no_run
/// # async fn _doc(client: &adaq_data_stock_cn::core::client::Client) {
/// let url = adaq_data_stock_cn::core::eastmoney_push::push2_url("/api/qt/clist/get").await;
/// # }
/// ```
pub async fn push2_url(path: &str) -> String {
    let host = eastmoney_push_host().await;
    format!("https://{host}{path}")
}

/// Opt OUT of the startup probe and instead pin a random mirror id in `2..=99`
/// (no network verification). Useful when you already know a mirror is reachable.
/// No-op if an override is already set.
pub fn use_random_push2_host() {
    set_push2_host(random_push2_host());
}

/// Pin a specific host (e.g. after verifying reachability yourself). Takes
/// precedence over the default probe. No-op if an override is already set.
pub fn set_push2_host(host: impl Into<String>) {
    let _ = OVERRIDE_HOST.set(host.into());
}

/// Eagerly trigger the default probe and return the verified host. Equivalent to
/// calling [`eastmoney_push_host`] once; provided so callers can warm up the host
/// explicitly at startup before issuing real requests. Returns the active host.
pub async fn detect_push2_host() -> String {
    eastmoney_push_host().await
}

/// Probe candidates; return the first that answers with a well-formed envelope.
async fn resolve() -> String {
    for &id in CANDIDATE_IDS {
        let host = host_for(id);
        if probe(&host).await {
            return host;
        }
    }
    "push2.eastmoney.com".to_string()
}

/// Maps a mirror id to its host string: `push{N}.eastmoney.com`.
fn host_for(id: u32) -> String {
    format!("push{id}.eastmoney.com")
}

/// Picks a random mirror id in `2..=99` and returns its host. Uses a small
/// xorshift64 PRNG seeded from the clock so we avoid pulling in a `rand` dep.
fn random_push2_host() -> String {
    host_for(random_push2_id())
}

fn random_push2_id() -> u32 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15);
    let mut x = nanos | 1; // ensure a non-zero seed
    // xorshift64
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    let range = (PUSH2_MAX_ID - PUSH2_MIN_ID + 1) as u64; // 98
    PUSH2_MIN_ID + (x % range) as u32
}

/// Lightweight probe: a tiny `clist/get` page. A mirror is "good" if it returns
/// HTTP 200 with a JSON envelope carrying `data`. Short timeout, no retries, so
/// a dead mirror fails fast instead of stalling resolution.
async fn probe(host: &str) -> bool {
    let http = match reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    let params = [
        ("pn", "1"),
        ("pz", "1"),
        ("po", "1"),
        ("np", "1"),
        ("fltt", "2"),
        ("invt", "2"),
        ("fid", "f12"),
        ("fs", "m:0 t:6"),
        ("fields", "f12,f14"),
    ];
    let req = http
        .get(format!("https://{host}/api/qt/clist/get"))
        .query(&params);
    match req.send().await {
        Ok(resp) if resp.status().is_success() => {
            matches!(resp.json::<Value>().await, Ok(v) if v.get("data").is_some())
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_for_id_2_is_bare() {
        assert_eq!(host_for(2), "push2.eastmoney.com");
    }

    #[test]
    fn host_for_id_99_is_prefixed() {
        assert_eq!(host_for(99), "push99.eastmoney.com");
    }

    #[test]
    fn random_id_stays_in_range() {
        for _ in 0..10_000 {
            let id = random_push2_id();
            assert!((PUSH2_MIN_ID..=PUSH2_MAX_ID).contains(&id), "id {id} out of range");
        }
    }

    #[test]
    fn url_shape() {
        // Synchronous stand-in for `push2_url` to assert the format without
        // touching the network / OnceCell.
        let host = host_for(99);
        let url = format!("https://{host}/api/qt/clist/get");
        assert_eq!(url, "https://push99.eastmoney.com/api/qt/clist/get");
    }
}
