//! Build script for the browser-impersonation backend.
//!
//! `impersonate-rs` links the vendored `libcurl-impersonate` shared library
//! (see `native/libcurl-impersonate/`). The crate's own `build.rs` records a
//! `@rpath/libcurl-impersonate.4.dylib` load command but does NOT add an
//! `LC_RPATH` to our binaries, so the dynamic loader cannot resolve it without
//! an environment variable — which macOS SIP strips from `cargo`-spawned
//! processes.
//!
//! To make the dylib discoverable in a portable, sudo-free, SIP-proof way we
//! bake an absolute `LC_RPATH` pointing at the vendored dir into every binary
//! we link (tests, examples, the lib's dylibs). The path is derived from
//! `CARGO_MANIFEST_DIR`, so it works for any checkout location.

fn main() {
    // Only relevant on Apple platforms (the .dylib is macOS-only).
    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.contains("apple") && !target.contains("darwin") {
        return;
    }

    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let vendored = std::path::Path::new(&manifest_dir).join("native/libcurl-impersonate");

    // Library search path for the linker.
    println!("cargo:rustc-link-search=native/libcurl-impersonate");
    println!("cargo:rustc-link-search={}", vendored.display());

    // Bake an rpath so the loader finds the dylib at runtime without
    // DYLD_LIBRARY_PATH (SIP-proof).
    println!(
        "cargo:rustc-link-arg=-Wl,-rpath,{}",
        vendored.display()
    );

    // Re-link if the vendored lib changes.
    println!("cargo:rerun-if-changed=native/libcurl-impersonate");
}
