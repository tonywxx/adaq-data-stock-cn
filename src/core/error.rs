use thiserror::Error;

/// Crate-level error type.
///
/// Carries enough context (source / endpoint) to let callers distinguish
/// "upstream broke" from "my parameters were wrong" (ADR-0006).
#[derive(Debug, Error)]
pub enum Error {
    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("json (de)serialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid parameter: {0}")]
    InvalidParam(String),

    #[error("not found at endpoint `{endpoint}`: {message}")]
    NotFound {
        endpoint: &'static str,
        message: String,
    },

    #[error("parse error at endpoint `{endpoint}`: {message}")]
    Parse {
        endpoint: &'static str,
        message: String,
    },

    #[error("upstream `{origin}` returned unexpected data: {message}")]
    UpstreamChanged {
        origin: &'static str,
        message: String,
    },

    #[error("rate limited (429) by upstream")]
    RateLimited,

    #[error("cache error: {0}")]
    Cache(String),

    #[error("csv error: {0}")]
    Csv(String),

    #[error("parquet error: {0}")]
    Parquet(String),
}

pub type Result<T> = std::result::Result<T, Error>;
