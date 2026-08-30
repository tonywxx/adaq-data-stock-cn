pub mod client;
pub mod eastmoney_push;
pub mod convert;
pub mod error;
pub mod html;
pub mod json;
pub mod pipeline;
pub mod resilience;
// macOS-only: links the vendored `libcurl-impersonate` dylib.
#[cfg(target_os = "macos")]
pub mod impersonate;
pub mod source;
pub mod util;
