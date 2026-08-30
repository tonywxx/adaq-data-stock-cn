# AGENTS.md

This file provides guidance to CodeBuddy Code when working with code in this repository.

## Project Overview

`adaq-data-stock-cn` is a Rust **library + binary** crate (package `adaq-data-stock-cn`, version `0.1.6`, `edition = "2024"`) — a Rust reimplementation of [akshare](https://github.com/akfamily/akshare) for Chinese A-share / market-data, serving as the data layer for the AdaQ quant platform. The library exposes ~800 `pub fn` endpoints across ~40 domain modules; per `docs/MAPPING.md` ~944 of 1172 akshare top-level functions are ported (`DONE`), ~156 are `DEFERRED` by design (signing / token / JS-execution / HTML / Excel), and `UNKNOWN` is fully cleared. `cargo test` reports ~1066 passed / 20 ignored and `cargo clippy` is clean.

> Note: `edition = "2024"` requires a recent Rust toolchain (Rust 1.85+). Use `rustup update` if `cargo` rejects the edition.

## Build, Run, Test, Lint

This is a standard Cargo project. Common commands:

- Build: `cargo build` (debug) / `cargo build --release` (optimized)
- Run an example: `cargo run --example <name>`
- Test (all): `cargo test`
- Run a single test: `cargo test <test_name>` or `cargo test -- --exact <test_name>` (add `-- --nocapture` to see `println!` output)
- Lint: `cargo clippy --all-targets`
- Format: `cargo fmt` (apply) / `cargo fmt --check` (verify in CI)
- Check without building binaries: `cargo check`

Add a dependency with `cargo add <crate>` (requires network access). There is no workspace — a single package only.

## Architecture

The crate is a library (`src/lib.rs`):

- `Cargo.toml` — manifest with real dependencies: `tokio` (async runtime), `reqwest` + `rustls-tls` (HTTP), `serde`/`serde_json`/`chrono` (data), `scraper`/`calamine` (HTML/Excel parsing), `csv`/`zip`/`flate2`/`encoding_rs` (formats), `sha2` (signing), `thiserror` (errors), and `impersonate-rs` (curl-impersonate TLS/HTTP2 fingerprinting, the Rust analog of Python `curl_cffi`). Optional `arrow`/`parquet` behind the `parquet` feature. `build.rs` resolves the vendored `libcurl-impersonate` dylib (see `native/`).
- `src/lib.rs` — declares ~40 top-level domain modules (e.g. `stock`, `stock_fundamental`, `bond`, `fund`, `futures`, `index`, `option`, `forex`, `economic`, `air`, `article`, `qdii`, `reits`, `video`, …) and re-exports `Client`, `convert`, `impersonate`, `Error`/`Result` from `core`.
- `src/core/` — shared infrastructure: `client` (`reqwest` + impersonate `Client`), `impersonate` (TLS-fingerprint backends), `convert` (akshare-shaped output conversion), `error`, `html` (HTML fetch/parse helpers), `source` (multi-source fallback), and `js/` decode stubs.
- `src/<domain>/` — one module per akshare domain, each containing ported endpoint `fn`s plus `#[cfg(test)]` parsing tests against offline fixtures. `src/bin/` holds any domain binaries.
- `docs/` — architecture decisions (`docs/adr/0001`–`0012`), `docs/MAPPING.md` (akshare parity tracker + upstream anchors), `docs/ROADMAP.md`, `docs/PORTING_GUIDE.md`, `docs/IMPERSONATE_RETRIAGE.md`, `docs/agents/` (triage labels / issue tracker), and `docs/_draft_*.md` (per-domain porting drafts).
- `examples/`, `tests/` — integration examples and tests.
- `graphify-out/` — generated knowledge-graph artifacts produced by the graphify skill (see below). Excluded from git (see `.gitignore`).
- `target/` — Cargo build output. Excluded from git.

Tests live in `src/<domain>/*.rs` `#[cfg(test)] mod tests` blocks (offline fixture parsing) and `tests/` integration dir. The `parquet` feature is opt-in.

## Project Tooling Rules

This repo is wired for two local-first tools. Honor them when relevant:

### graphify (knowledge graph)
The repo maintains a knowledge graph in `graphify-out/` with god nodes, community structure, and cross-file relationships.

- For codebase questions, run `graphify query "<question>"` when `graphify-out/graph.json` exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts (these return a small scoped subgraph).
- If `graphify-out/wiki/index.md` exists, use it for broad navigation instead of raw source browsing.
- Read `graphify-out/GRAPH_REPORT.md` only for broad architecture review or when query/path/explain return too little.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).
- Dirty `graphify-out/` files after hooks or incremental updates are expected and not a reason to skip graphify. Only skip if the task is about stale/incorrect graph output or the user says not to.

