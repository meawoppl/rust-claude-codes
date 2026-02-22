//! Error types for the codex-codes crate.
//!
//! All fallible operations return [`Result<T>`], which uses [`enum@Error`] as the
//! error type. The variants cover JSON serialization, I/O, protocol-level
//! issues, and JSON-RPC errors from the app-server.

use thiserror::Error;

/// All possible errors from codex-codes operations.
#[derive(Error, Debug)]
pub enum Error {
    /// JSON serialization or deserialization failed.
    ///
    /// Returned when request parameters can't be serialized or
    /// response payloads don't match expected types.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// An I/O error occurred communicating with the app-server process.
    ///
    /// Common causes: process not found, pipe broken, permission denied.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// A protocol-level error (e.g., missing stdin/stdout pipes).
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// The app-server connection was closed unexpectedly.
    #[error("Connection closed")]
    ConnectionClosed,

    /// A message from the server could not be deserialized.
    ///
    /// Includes the raw message text for debugging. If you encounter this,
    /// please report it — it likely indicates a protocol change.
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// The app-server process exited with a non-zero status.
    #[error("Process exited with status {0}: {1}")]
    ProcessFailed(i32, String),

    /// The server returned a JSON-RPC error response.
    ///
    /// Contains the error code and message from the server.
    /// See the Codex CLI docs for error code meanings.
    #[error("JSON-RPC error ({code}): {message}")]
    JsonRpc { code: i64, message: String },

    /// The server closed the connection (EOF on stdout).
    ///
    /// Returned by `request()` if the server exits mid-conversation.
    #[error("Server closed connection")]
    ServerClosed,

    /// An unclassified error.
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// A `Result` type alias using [`enum@Error`].
pub type Result<T> = std::result::Result<T, Error>;
