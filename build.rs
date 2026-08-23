//! Build script for the browser-impersonation backend.
//!
//! `impersonate-rs` links the `libcurl-impersonate` shared library (macOS-only
//! `.dylib`). The dylib is large (~14 MiB) and would blow past crates.io's 10 MiB
//! per-crate limit, so it is **not** shipped in the published crate (see the
//! `exclude = ["native/"]` in `Cargo.toml`). Instead we resolve it two ways:
//!
//! 1. Repo checkout (local dev / CI): the dylib is vendored in
//!    `native/libcurl-impersonate/`. We point the linker and bake an absolute
//!    `@rpath` at that dir so no `DYLD_LIBRARY_PATH` / sudo is needed (SIP-proof).
//! 2. Published crate (consumers building from crates.io): `native/` is absent,
//!    so we require a system-installed `libcurl-impersonate` (e.g.
//!    `brew install curl-impersonate`) and point at its lib dir. If it is
//!    missing we fail the build with explicit install instructions.

use std::path::Path;

fn main() {
    // The dylib is macOS-only. On other targets the `impersonate` module is
    // cfg-gated out (see `Cargo.toml` + `src/core/mod.rs`), so there is nothing
    // to link — bail out early and let the reqwest-only build proceed.
    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.contains("apple") && !target.contains("darwin") {
        return;
    }

    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let vendored = Path::new(&manifest_dir).join("native/libcurl-impersonate");

    // Case 1: repo checkout — use the vendored dylib.
    if vendored.exists() {
        link_against(&vendored);
        println!("cargo:rerun-if-changed=native/libcurl-impersonate");
        return;
    }

    // Case 2: published crate — require a system-installed dylib.
    let system_dirs = [
        "/usr/local/lib",
        "/opt/homebrew/lib",
        "/opt/homebrew/opt/curl-impersonate/lib",
    ];
    for dir in system_dirs {
        if has_dylib(dir) {
            link_against(Path::new(dir));
            return;
        }
    }

    panic!(
        "libcurl-impersonate not found. This crate links the macOS-only \
         `libcurl-impersonate` dylib (browser-impersonation backend), which is not \
         bundled in the published crate.\n\
         Install it system-wide so `libcurl-impersonate-chrome.dylib` and \
         `libcurl-impersonate.4.dylib` are present in /usr/local/lib, e.g.:\n\
         \n    brew install curl-impersonate\n\
         \nIf `brew` installed it elsewhere (e.g. /opt/homebrew/lib) and the \
         `libcurl-impersonate-chrome.dylib` symlink is missing, symlink it:\n\
         \n    ln -s /opt/homebrew/lib/libcurl-impersonate.4.dylib \
         /usr/local/lib/libcurl-impersonate-chrome.dylib\n\
         \nAlternatively, build from the source checkout, which vendors the dylib \
         in native/libcurl-impersonate/."
    );
}

/// Emit the linker search path and bake an absolute `@rpath` so the loader
/// finds the dylib at runtime without `DYLD_LIBRARY_PATH` (SIP-proof).
fn link_against(dir: &Path) {
    println!("cargo:rustc-link-search={}", dir.display());
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", dir.display());
}

/// True if `dir` contains any of the dylib names the linker/loader need.
fn has_dylib(dir: &str) -> bool {
    let dir = Path::new(dir);
    if !dir.is_dir() {
        return false;
    }
    ["libcurl-impersonate-chrome.dylib", "libcurl-impersonate.4.dylib", "libcurl-impersonate.dylib"]
        .iter()
        .any(|name| dir.join(name).exists())
}
