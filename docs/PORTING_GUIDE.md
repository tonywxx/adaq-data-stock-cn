# Porting Guide — akshare → adaq-data-stock-cn (Rust)

This crate is a from-scratch Rust reimplementation of akshare. Every public
akshare function becomes a typed Rust function that returns `Vec<RowStruct>`
(optionally converted to JSON/CSV/Parquet via `core::convert`).

## Hard rules

1. **Do not edit `src/lib.rs` or any `mod.rs`.** The lead pre-registers every
   module. You ONLY create the leaf `.rs` file(s) you are assigned and the
   fixtures. (If you are assigned a brand-new top-level domain, you DO create
   its `mod.rs` and the lead has already added `pub mod <domain>;` to `lib.rs`.)
2. **No new dependencies.** Reuse what exists: `reqwest`, `serde`, `serde_json`,
   `tokio`, `csv`, `thiserror`, `sha2`. Do not add crates or change `Cargo.toml`.
3. **Offline parsing tests are mandatory** for every implemented function.
4. **DEFER, don't fake.** If a function needs JS-signed params (Eastmoney `ut`,
   Sina daily `wencode`, cninfo `Accept-Enckey`), HTML-table scraping, Excel/ZIP
   download, or third-party auth/sessions, do NOT implement it. Record it as
   `DEFERRED` in the module's `//!` doc and in `docs/MAPPING.md`.
5. Match akshare's column semantics and field order where it matters.

## Module skeleton (copy this shape)

```rust
//! <One-line description>. Ports `akshare/<source>.py`.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `foo` | `bar.py:13` | ... |
//!
//! ## DEFERRED
//! None.  (or list each deferred fn + exact reason)

use serde_json::Value;
use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE: &str = "eastmoney";

#[derive(Debug, Clone, serde::Serialize)]
pub struct FooRow {
    pub date: String,
    pub value: Option<f64>,
}

/// Parse `foo` rows from the already-fetched `Value`. `pub(crate)` so tests in
/// the same file can call it directly. Keep parsing pure (no I/O).
pub(crate) fn parse_foo(resp: &Value) -> Result<Vec<FooRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE,
        message: "expected array".into(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(FooRow {
            date: item.get("date").and_then(|v| v.as_str()).map(str::to_string).unwrap_or_default(),
            value: item.get("value").and_then(|v| v.as_f64()),
        });
    }
    Ok(out)
}

/// <doc> Default-call public API (async).
pub async fn foo(client: &Client) -> Result<Vec<FooRow>> {
    let v = client.get_json(SOURCE, "foo", URL, &[("a", "b")]).await?;
    parse_foo(&v)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(name: &str) -> Value {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures").join(name);
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }
    fn approx(a: Option<f64>, b: f64) -> bool {
        match a { Some(x) => (x - b).abs() < 1e-6, None => false }
    }
    #[test]
    fn parse_foo_ok() {
        let rows = parse_foo(&fixture("foo.json")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].date, "2024-01-01");
        assert!(approx(rows[0].value, 1.23));
    }
}
```

## Client API (`src/core/client.rs`)

- `client.get_json(source, endpoint, url, &[("k","v")]) -> Result<Value>` — GET, JSON.
- `client.get_json_with_headers(source, endpoint, url, params, Some(&[("Referer","...")]))` — GET with headers (Sina needs `Referer: https://finance.sina.com.cn`).
- `client.get_text(source, endpoint, url, params, headers) -> Result<String>` — non-JSON text.
- `client.post_form_json(source, endpoint, url, params, headers) -> Result<Value>` — POST form.

`source` is one of `"eastmoney" | "sina" | "tencent" | ...`; use a descriptive
string for new sources (e.g. `"baidu"`, `"cpcadata"`, `"exchangerate"`).

## Eastmoney `datacenter-web` pattern (macro/fund/stock datacenter endpoints)

Many endpoints hit `https://datacenter-web.eastmoney.com/api/data/v1/get` with a
`reportName` / `columns` / `filter` / `pageSize` query. Copy this verbatim:

```rust
const BASE: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";
fn emg_data_array(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("result").and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged { origin: "eastmoney", message: "missing result.data".into() })
}
// fetch:
let v = client.get_json("eastmoney", "ep", BASE,
    &[("reportName","RPT_NAME"),("columns","ALL"),("pageSize","1000"),("sortColumns","DATE"),("sortTypes","-1"),("source","WEB"),("client","WEB")]).await?;
let data = emg_data_array(&v)?;
```

## Fixture strategy

1. **Try to fetch a real response** from the exact URL the akshare source uses
   (replicate its params + headers, e.g. Sina needs `Referer`). Save the parsed
   JSON to `tests/fixtures/<name>.json`. This validates the live parse path.
2. **If the source is unreachable from this environment** (GFW block, JS-signed,
   auth), hand-craft a *minimal but realistic* fixture JSON that mirrors exactly
   the response shape the parser reads (read the akshare source's field access to
   know the keys). The test then proves the parse logic; note in the `//!` doc
   that the fixture is synthetic. Keep at least 2-3 rows.
3. Every fixture filename must be unique across the crate. Prefer `<fn>.json`.

## Convert layer (`src/core/convert.rs`)

```rust
use crate::core::convert;
let json = convert::to_json(&rows)?;   // -> String
let csv  = convert::to_csv(&rows)?;    // -> String
// to_parquet needs the `parquet` feature; only use if the source module already does.
```
Public functions may optionally expose `*_json`/`*_csv` wrappers, but the primary
return type is `Vec<Row>`.

## Docs

- End of module `//!`: add a `## DEFERRED` section listing any skipped functions
  with the exact reason (JS signature / cninfo auth / HTML scrape / Excel).
- Append rows to `docs/MAPPING.md` under the right section: one line per akshare
  function with columns `akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态`
  (`DONE` or `DEFERRED`).
- Update `ROADMAP.md` progress table: add a row per new domain with endpoint +
  test counts, and bump the running totals.
