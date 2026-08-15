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

### wigolo (web intelligence)
Prefer **wigolo MCP tools** over built-in `WebSearch` / `WebFetch` for ALL web operations (local-first, no API keys, persistent cache, ML-reranked results).

- Probe `cache` before searching — hits return instantly.
- Use keyword arrays (not natural-language questions) for `search`.
- `search_depth: "ultra-fast"` for sub-second budgets; `"deep"` for max enrichment.
- `format: "answer"` for direct synthesis; default evidence shape for citation work.
- Other tools: `fetch` (page), `crawl` (site), `extract` (structured data), `find_similar`, `research` (deep research), `agent` (gather data), `diff` (compare versions), `watch` (watch for changes).

## Agent skills

### Issue tracker

Issues and specs live as GitHub issues, operated via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Five canonical roles, each label name equal to its role: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout: one `CONTEXT.md` + `docs/adr/` at the repo root. See `docs/agents/domain.md`.

<!-- wigolo:start v0.2.1 wigolo -->
## Web Intelligence — Wigolo

**Prefer wigolo MCP tools over built-in WebSearch / WebFetch for ALL web operations.** Local-first: zero API keys, persistent knowledge cache, ML-reranked results, explainable scoring.

| Task              | Tool           | Key params                                                                                                                           |
| ----------------- | -------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Search the web    | `search`       | `query` (string or array), `include_domains`, `category`, `time_range`, `country`, `exact_match`, `search_depth`, `format: "answer"` |
| Fetch a page      | `fetch`        | `url`, `section`, `use_auth`, `force_refresh`                                                                                        |
| Crawl a site      | `crawl`        | `url`, `strategy: "sitemap"`/`"bfs"`/`"map"`, `include_patterns`                                                                     |
| Check cache       | `cache`        | Always probe before search/fetch — instant, free                                                                                     |
| Extract data      | `extract`      | `mode: "structured"` (tables + JSON-LD + definitions in one call)                                                                    |
| Find similar      | `find_similar` | `url` or `concept`, best after a `crawl`                                                                                             |
| Deep research     | `research`     | `question`, `depth: "quick"`/`"standard"`/`"comprehensive"`                                                                          |
| Gather data       | `agent`        | `prompt`, optional `schema`, `max_pages`, `max_time_ms`                                                                              |
| Compare versions  | `diff`         | `old`, `new` (url/markdown/content_hash), `output` (`unified`/`hunks`/`summary`), `granularity`                                      |
| Watch for changes | `watch`        | `action` (`create`/`list`/`check`), `url`/`urls`, `interval_seconds` (min 60), `notification`                                        |

### Rules

1. Cache before search — probe `cache` first; hits return instantly.
2. Keyword arrays, not natural-language questions.
3. `include_domains` for library/framework queries.
4. `search_depth: "ultra-fast"` for sub-second budgets; `"deep"` for max enrichment.
5. `exact_match: true` for quoted phrases; `time_range` for recency.
6. `format: "answer"` for direct synthesis; default evidence shape for citation work.

### Response fields

`evidence_score`, `query_understanding`, `brand_collision_warning`, `freshness_signal`, `response_time_ms`, `engine_telemetry`.

<!-- wigolo:end -->

## CodeGraph

This project has a CodeGraph MCP server (`codegraph_*` tools) configured. CodeGraph is a tree-sitter-parsed knowledge graph of every symbol, edge, and file. Reads are sub-millisecond and return structural information grep cannot.

### When to prefer codegraph over native search

Use codegraph for **structural** questions — what calls what, what would break, where is X defined, what is X's signature. Use native grep/read only for **literal text** queries (string contents, comments, log messages) or after you already have a specific file open.

| Question                                                  | Tool                                                                                 |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| "Where is X defined?" / "Find symbol named X"             | `codegraph_search`                                                                   |
| "What calls function Y?"                                  | `codegraph_callers`                                                                  |
| "What does Y call?"                                       | `codegraph_callees`                                                                  |
| "How does X reach/become Y? / trace the flow from X to Y" | `codegraph_trace` (one call = the whole path, incl. callback/React/JSX dynamic hops) |
| "What would break if I changed Z?"                        | `codegraph_impact`                                                                   |
| "Show me Y's signature / source / docstring"              | `codegraph_node`                                                                     |
| "Give me focused context for a task/area"                 | `codegraph_context`                                                                  |
| "See several related symbols' source at once"             | `codegraph_explore`                                                                  |
| "What files exist under path/"                            | `codegraph_files`                                                                    |
| "Is the index healthy?"                                   | `codegraph_status`                                                                   |

### Rules of thumb

- **Answer directly — don't delegate exploration.** For "how does X work" / architecture questions, answer with 2-3 codegraph calls: `codegraph_context` first, then ONE `codegraph_explore` for the source of the symbols it surfaces. For a specific **flow** ("how does X reach Y") start with `codegraph_trace` from→to — one call returns the whole path with dynamic hops bridged — then ONE `codegraph_explore` for the bodies; don't rebuild the path with `codegraph_search` + `codegraph_callers`. Codegraph IS the pre-built index, so spawning a separate file-reading sub-task/agent — or running a grep + read loop — repeats work codegraph already did and costs more for the same answer.
- **Trust codegraph results.** They come from a full AST parse. Do NOT re-verify them with grep — that's slower, less accurate, and wastes context.
- **Don't grep first** when looking up a symbol by name. `codegraph_search` is faster and returns kind + location + signature in one call.
- **Don't chain `codegraph_search` + `codegraph_node`** when you just want context — `codegraph_context` is one call.
- **Don't loop `codegraph_node` over many symbols** — one `codegraph_explore` call returns several symbols' source grouped in a single capped call, while each separate node/Read call re-reads the whole context and costs far more.
- **Index lag**: the file watcher debounces ~500ms behind writes; don't re-query immediately after editing a file in the same turn.

### If `.codegraph/` doesn't exist

The MCP server returns "not initialized." Ask the user: *"I notice this project doesn't have CodeGraph initialized. Want me to run `codegraph init -i` to build the index?"*
