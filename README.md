# adaq-data-stock-cn

> 📖 中文文档: [README.zh-CN.md](README.zh-CN.md)

A Rust reimplementation of [akshare](https://github.com/akfamily/akshare), serving as the
A-share market data layer for the **AdaQ** quant platform.

Reimplements all of akshare's public APIs in pure Rust, covering upstreams such as Eastmoney,
Sina, Tencent, the exchanges, and Chinamoney. Every endpoint returns **typed structs** that
the conversion layer can emit as **JSON / CSV / Parquet**. Add it to your project with
`cargo add`.

## Parity progress

The full benchmark map lives in [`docs/MAPPING.md`](docs/MAPPING.md) — it is both a coverage
tracker and an upstream-sync anchor (see ADR-0012). Current stats (1172 akshare top-level
functions total):

| Status | Count | Notes |
|---|---:|---|
| `DONE` | 944 | fully ported to Rust endpoints |
| `DEFERRED` | 156 | blocked by signing / tokens / JS engine / HTML / Excel; deferred by design per ADR-0005/0008 |
| `INTERNAL` | 72 | akshare internal helpers, not public data endpoints, excluded from coverage |
| `UNKNOWN` | 0 | fully cleared (see `git log` "clear all UNKNOWN") |

> Public data-endpoint coverage = 944 / (1172 − 72) ≈ **85.8%**. The remaining 14.2% is a
> by-design deferral (needs the JS engine [`rquickjs`] or token reverse-engineering), excluding
> the 72 internal helpers.
> Note: 3 akshare internal modules (`utils/demjson`, `futures/symbol_var`) were previously
> mislabeled `DONE` and corrected to `INTERNAL` in this pass.

## Installation

```toml
[dependencies]
adaq-data-stock-cn = "0.1"
tokio = { version = "1", features = ["full"] }   # async runtime (for examples using macros)
```

Optional features:

- `parquet` — enable Parquet export (off by default to keep the core lean; see ADR-0001/0014).

```toml
adaq-data-stock-cn = { version = "0.1", features = ["parquet"] }
```

## Quick start

The first argument of every endpoint is the shared `Client` (built-in retry/backoff,
per-source rate limiting, concurrency cap, and optional on-disk cache; see ADR-0009).

```rust
use adaq_data_stock_cn::{Client, convert, stock};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    // A-share realtime snapshot (Eastmoney push2 clist)
    let spot = stock::stock_hist_em::stock_zh_a_spot_em(&client).await?;
    println!("snapshot: {} rows", spot.len());

    // Take a sample and serialize to JSON / CSV
    let sample = &spot[..spot.len().min(5)];
    println!("{}", convert::to_json(sample)?);
    println!("{}", convert::to_csv(sample)?);
    Ok(())
}
```

## Output formats

The conversion layer (`adaq_data_stock_cn::convert`) serializes any `Serialize` row type into
three formats:

| Function | Format | Notes |
|---|---|---|
| `convert::to_json(rows)` | JSON array | always available |
| `convert::to_csv(rows)` | CSV | header taken from struct field names |
| `convert::to_parquet(rows, path)` | Parquet file | requires the `parquet` feature |

## Multi-source fallback

Some endpoints embed a source chain (auto-fallback on primary-source failure; see ADR-0010).
For example, A-share daily history prefers Eastmoney and falls back to Tencent:

```rust
use adaq_data_stock_cn::{Client, stock};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    // symbol is the prefix-free code; period: daily/weekly/monthly; adjust: ""/"qfq"/"hfq"
    let hist = stock::hist::daily(&client, "600519", "daily", "", "20240101", "20241231").await?;
    println!("Kweichow Moutai: {} daily bars", hist.len());
    Ok(())
}
```

## Examples

Runnable end-to-end examples live in [`examples/`](examples/); run them with
`cargo run --example <name>`:

| Example | Endpoint | Notes |
|---|---|---|
| `stock_spot` | `stock_zh_a_spot_em` | A-share realtime snapshot → JSON / CSV |
| `stock_hist` | `stock::hist::daily` | A-share daily history (multi-source fallback) → CSV |
| `bond_cov_spot` | `bond_zh_hs_cov_spot` | CN convertible bonds realtime snapshot |
| `index_hist` | `index_zh_a_hist` | index daily history |
| `futures_spot` | `futures_zh_spot` | futures realtime snapshot |
| `parquet_export` | `convert::to_parquet` | Parquet export (needs `--features parquet`; build fixed this pass) |
| `impersonate_smoke` | `ImpersonateClient` | browser-fingerprint backend live test (Sina GBK / Baidu / Tencent gtimg) |

> Examples hit real upstreams and need network access. Offline, just verify compilation
> with `cargo build --examples`.

## Browser-fingerprint (anti-bot) backend

The default `Client` uses `reqwest` + rustls, whose TLS/HTTP2 handshake is trivially
fingerprinted and blocked by anti-bot middleboxes. This crate also ships a
**browser-impersonation HTTP backend** — the Rust equivalent of the Python
[`primp`](https://github.com/deedy5/primp) (`curl_cffi`): built on `curl-impersonate`, it
replays a real Chrome ClientHello so requests look like a genuine browser.

```rust
use adaq_data_stock_cn::ImpersonateClient;

let client = ImpersonateClient::new(); // impersonates Chrome 131, with real UA/Accept/Accept-Language
let html = client
    .get_text("https://hq.sinajs.cn/list=sh600000",
               Some(&[adaq_data_stock_cn::core::impersonate::sina_referer()]))
    .await?;
```

- Module: `src/core/impersonate.rs`, exported as `crate::ImpersonateClient` / `crate::impersonate`.
- Native lib: the `native/libcurl-impersonate/` dylibs are **vendored** in the repo (macOS
  arm64/x86_64). `build.rs` bakes an `LC_RPATH` into every binary, so **no sudo and no
  `DYLD_LIBRARY_PATH`** are required.
- GBK decoding: Sina/Baidu/jisilu return GBK pages; this backend always decodes via
  `encoding_rs::GBK` (UTF-8/BOM fallback), avoiding the underlying crate's strict UTF-8 panic
  on Chinese pages.
- When to use: anti-bot sources that block by TLS fingerprint (Cloudflare/Akamai-style).
  **Note:** the sources reachable from this environment (Sina/Baidu/Tencent/Eastmoney/xueqiu)
  are already accessible with the default `reqwest` `Client` + correct `Referer` headers and do
  not need impersonation; and `push2his.eastmoney.com` rejects the Chrome h2 fingerprint, so it
  continues to be served by the default `Client`.
- The deferred set is **not** unlocked by this: existing `DEFERRED` endpoints are mainly gated
  by JS engine / tokens / HTML / Excel (see
  [`docs/IMPERSONATE_RETRIAGE.md`](docs/IMPERSONATE_RETRIAGE.md)), not by TLS fingerprint.

## Documentation

- Benchmark map (coverage / upstream anchors): [`docs/MAPPING.md`](docs/MAPPING.md)
- Browser-impersonation re-triage: [`docs/IMPERSONATE_RETRIAGE.md`](docs/IMPERSONATE_RETRIAGE.md)
- Roadmap: [`ROADMAP.md`](ROADMAP.md)
- Architecture Decision Records: [`docs/adr/`](docs/adr)
- Porting guide: [`docs/PORTING_GUIDE.md`](docs/PORTING_GUIDE.md)

## License

Apache-2.0
