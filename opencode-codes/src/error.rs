//! Error types for the opencode-codes crate.
//!
//! All fallible operations return [`Result<T>`], which uses [`enum@Error`] as the
//! error type. The core variants ([`Error::Http`], [`Error::Serde`]) are
//! available in the `types` tier and build on `wasm32`. Transport, SSE, and
//! server-lifecycle variants are gated behind the features whose dependencies
//! supply their source types.

use thiserror::Error;

/// Error type for the opencode-codes crate.
#[derive(Debug, Error)]
pub enum Error {
    /// The opencode server returned a non-success HTTP status.
    ///
    /// `status` is the HTTP status code; `body` is the (best-effort) response
    /// body, useful for surfacing the server's error payload.
    #[error("HTTP {status}: {body}")]
    Http {
        /// HTTP status code returned by the server.
        status: u16,
        /// Response body, captured for diagnostics.
        body: String,
    },

    /// JSON serialization or deserialization failed.
    ///
    /// Returned when request bodies can't be serialized or response/SSE
    /// payloads don't match the expected shape.
    #[error("JSON error: {0}")]
    Serde(#[from] serde_json::Error),

    /// A transport-level error occurred while talking to the server.
    ///
    /// Common causes: connection refused, TLS failure, timeout, broken pipe.
    ///
    /// Boxed because `reqwest::Error` embeds a `reqwest::Response` and is large;
    /// boxing keeps the whole [`enum@Error`] small enough that `-> Result<T>`
    /// does not trip `clippy::result_large_err`.
    #[cfg(feature = "async-client")]
    #[error("transport error: {0}")]
    Transport(Box<reqwest::Error>),

    /// The Server-Sent Events stream failed.
    ///
    /// Returned when the `GET /event` stream errors, closes unexpectedly, or
    /// yields a frame that cannot be decoded.
    ///
    /// Boxed for the same size reason as [`Error::Transport`].
    #[cfg(feature = "async-client")]
    #[error("SSE error: {0}")]
    Sse(Box<reqwest_eventsource::Error>),

    /// The managed `opencode serve` process could not be spawned or exited
    /// unexpectedly.
    #[cfg(feature = "server")]
    #[error("server lifecycle error: {0}")]
    Server(String),
}

#[cfg(feature = "async-client")]
impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Transport(Box::new(err))
    }
}

#[cfg(feature = "async-client")]
impl From<reqwest_eventsource::Error> for Error {
    fn from(err: reqwest_eventsource::Error) -> Self {
        Error::Sse(Box::new(err))
    }
}

/// A `Result` type alias using [`enum@Error`].
pub type Result<T> = std::result::Result<T, Error>;
