//! Error types for the hermes-codes SDK.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("JSON-RPC error {code}: {message}")]
    JsonRpc { code: i64, message: String },

    #[error("Binary not found: '{name}' is not on PATH. Is hermes-agent installed?")]
    BinaryNotFound { name: String },

    #[error("The hermes acp process closed the connection")]
    ServerClosed,

    #[error("Timeout occurred")]
    Timeout,
}

pub type Result<T> = std::result::Result<T, Error>;
