# AGENTS.md

This file provides guidance to CodeBuddy Code when working with code in this repository.

## Project Overview

`adaq-data-stock-cn` is a Rust binary crate (package name `adaq-data-stock-cn`, version `0.1.0`, `edition = "2024"`). The name suggests tooling for Chinese stock-market (A-share) data, but the crate is currently a scaffold: the only source file is `src/main.rs`, which prints `Hello, world!`. There are no dependencies declared and no tests yet.

> Note: `edition = "2024"` requires a recent Rust toolchain (Rust 1.85+). Use `rustup update` if `cargo` rejects the edition.

## Build, Run, Test, Lint

This is a standard Cargo project. Common commands:

- Build: `cargo build` (debug) / `cargo build --release` (optimized)
- Run: `cargo run`
- Test (all): `cargo test`
- Run a single test: `cargo test <test_name>` or `cargo test -- --exact <test_name>` (add `-- --nocapture` to see `println!` output)
- Lint: `cargo clippy --all-targets`
- Format: `cargo fmt` (apply) / `cargo fmt --check` (verify in CI)
- Check without building binaries: `cargo check`

Add a dependency with `cargo add <crate>` (requires network access). There is no workspace — a single package only.

## Architecture

The repository is intentionally minimal:

- `Cargo.toml` — package manifest; no `[dependencies]` yet, so nothing compiles beyond the standard library.
- `src/main.rs` — the sole binary entry point (`fn main`).
- `graphify-out/` — generated knowledge-graph artifacts produced by the graphify skill (see below). Excluded from git (see `.gitignore`).
- `target/` — Cargo build output. Excluded from git.

As the crate grows, prefer adding library code under `src/lib.rs` (with `src/main.rs` thin and delegating to the lib) so logic stays unit-testable. Tests conventionally live in `src/*.rs` `#[cfg(test)] mod tests` blocks or a `tests/` integration directory.

## Project Tooling Rules

This repo is wired for two local-first tools. Honor them when relevant:

### graphify (knowledge graph)
The repo maintains a knowledge graph in `graphify-out/` with god nodes, community structure, and cross-file relationships.

- For codebase questions, run `graphify query "<question>"` when `graphify-out/graph.json` exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts (these return a small scoped subgraph).
- If `graphify-out/wiki/index.md` exists, use it for broad navigation instead of raw source browsing.
- Read `graphify-out/GRAPH_REPORT.md` only for broad architecture review or when query/path/explain return too little.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).
- Dirty `graphify-out/` files after hooks or incremental updates are expected and not a reason to skip graphify. Only skip if the task is about stale/incorrect graph output or the user says not to.

