use std::future::Future;
use std::pin::Pin;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// Pinned, type-erased future resolving one source into the canonical `T`.
pub type SourceFut<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

/// Ordered multi-source fallback (ADR-0010).
///
/// Each source is an async fetcher that captures its own per-source arguments
/// (symbol, date, …) and resolves to the same canonical `T`. [`SourceChain::run`]
/// tries them in priority order and returns the first `Ok`; if every source
/// fails it returns the last error (richer than a generic "all sources failed").
///
/// This concentrates the otherwise hand-rolled `if let Ok(..) { return }` chains
/// that lived inside each logical-endpoint `mod.rs`, so the fallback loop has one
/// place to test and change. Per-source normalization stays inside each source
/// module — only the *try-order* is shared.
pub struct SourceChain<'a, T> {
    fetchers: Vec<Box<dyn Fn(&'a Client) -> SourceFut<'a, T> + Send + Sync + 'a>>,
}

impl<'a, T> Default for SourceChain<'a, T> {
    fn default() -> Self {
        Self {
            fetchers: Vec::new(),
        }
    }
}

impl<'a, T> SourceChain<'a, T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a source. The closure captures any per-source arguments and must
    /// outlive the chain (lifetime `'a`).
    pub fn push<F>(mut self, f: F) -> Self
    where
        F: Fn(&'a Client) -> SourceFut<'a, T> + Send + Sync + 'a,
    {
        self.fetchers.push(Box::new(f));
        self
    }

    /// Try each source in order; first `Ok` wins, else the last error.
    pub async fn run(self, client: &'a Client) -> Result<T> {
        let mut last_err: Option<Error> = None;
        for fetch in &self.fetchers {
            match fetch(client).await {
                Ok(value) => return Ok(value),
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or_else(|| Error::UpstreamChanged {
            origin: "source-chain",
            message: "no sources configured".into(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Two fake sources; the first always fails, the second always succeeds.
    async fn failing(_c: &Client) -> Result<String> {
        Err(Error::UpstreamChanged {
            origin: "fake-a",
            message: "boom".into(),
        })
    }
    async fn working(_c: &Client) -> Result<String> {
        Ok("ok".into())
    }

    #[tokio::test]
    async fn first_ok_wins() {
        let client = Client::new();
        let got = SourceChain::new()
            .push(|c| Box::pin(failing(c)))
            .push(|c| Box::pin(working(c)))
            .run(&client)
            .await
            .unwrap();
        assert_eq!(got, "ok");
    }

    #[tokio::test]
    async fn last_error_propagates() {
        let client = Client::new();
        let err = SourceChain::new()
            .push(|c| Box::pin(failing(c)))
            .run(&client)
            .await
            .unwrap_err();
        match err {
            Error::UpstreamChanged { origin, .. } => assert_eq!(origin, "fake-a"),
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
