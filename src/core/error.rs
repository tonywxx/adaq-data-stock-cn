use thiserror::Error;

/// Crate-level error type (ADR-0006).
///
/// Carries enough context (source / endpoint) to let callers distinguish
/// "upstream broke" from "my parameters were wrong". Variant choice is a
/// contract — pick by *where the failure happened*, not by how bad it is:
///
/// | Variant | Meaning | Retryable? | Typical cause |
/// |---|---|---|---|
/// | `Http` | reqwest transport failure | yes | connection reset, TLS error |
/// | `Impersonate` | curl-impersonate transport failure | yes | blocked / fingerprint mismatch |
/// | `Json` / `Io` / `Csv` / `Parquet` | a concrete format failed | no | malformed payload |
/// | `InvalidParam` | caller passed a bad argument | no | unknown period / adjust |
/// | `NotFound` | upstream answered "no data" for a valid request | no | symbol has no history |
/// | `Parse` | data arrived but a row/cell couldn't be shaped | no | schema drift, wrong column count |
/// | `UpstreamChanged` | upstream changed at the *protocol* level | no | HTTP 4xx/5xx, error envelope, missing top-level field |
/// | `RateLimited` | upstream returned 429 | yes (after `Retry-After`) | throttling |
///
/// `Parse` vs `UpstreamChanged`: both mean "upstream gave us something we
/// didn't expect", but `Parse` is about *content* we received (a row we can't
/// map), while `UpstreamChanged` is about the *response shape / status* (the
/// endpoint itself moved). Resilience treats only network/transport errors as
/// retryable, so `Parse` / `UpstreamChanged` / `InvalidParam` / `NotFound`
/// surface immediately.
#[derive(Debug, Error)]
pub enum Error {
    /// reqwest transport failure (connection reset, TLS error, timeout).
    /// Retryable by [`crate::core::resilience::RetryPolicy`].
    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON (de)serialization failure (serde).
    #[error("json (de)serialization failed: {0}")]
    Json(#[from] serde_json::Error),

    /// Filesystem I/O error (cache read/write, fixture loading).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Caller passed a bad argument (unknown period/adjust, bad symbol format).
    /// Not retryable — fix the input.
    #[error("invalid parameter: {0}")]
    InvalidParam(String),

    /// Upstream answered "no data" for an otherwise valid request (e.g. a
    /// symbol with no trading history). Distinct from `UpstreamChanged`, which
    /// means the endpoint itself broke.
    #[error("not found at endpoint `{endpoint}`: {message}")]
    NotFound {
        endpoint: &'static str,
        message: String,
    },

    /// Data was received but a row/cell could not be shaped into the expected
    /// type — schema drift, malformed CSV/JSON, wrong column count. Content-level
    /// failure; not retryable.
    #[error("parse error at endpoint `{endpoint}`: {message}")]
    Parse {
        endpoint: &'static str,
        message: String,
    },

    /// Upstream changed at the *protocol* level: HTTP 4xx/5xx, an error envelope,
    /// or a missing top-level field that indicates the endpoint itself moved.
    /// Not a content/row parse failure (that is [`Error::Parse`]).
    #[error("upstream `{origin}` returned unexpected data: {message}")]
    UpstreamChanged {
        origin: &'static str,
        message: String,
    },

    /// Upstream returned HTTP 429; the resilience layer honors `Retry-After`.
    #[error("rate limited (429) by upstream")]
    RateLimited,

    /// CSV (de)serialization failure.
    #[error("csv error: {0}")]
    Csv(String),

    /// Parquet (de)serialization failure (only with the `parquet` feature).
    #[error("parquet error: {0}")]
    Parquet(String),

    /// Browser-impersonation (curl-impersonate) transport failure. Retryable by
    /// [`crate::core::resilience::RetryPolicy`].
    #[error("browser-impersonation request failed: {0}")]
    Impersonate(String),
}

pub type Result<T> = std::result::Result<T, Error>;
