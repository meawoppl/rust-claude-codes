//! Error type shared across the crate.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to deserialize wire line: {source}; raw: {raw_line}")]
    Deserialization {
        source: serde_json::Error,
        raw_line: String,
    },

    #[error("failed to serialize command: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("pi binary not found on PATH")]
    BinaryNotFound,

    #[error("pi exited before answering; stderr: {0}")]
    ProcessExited(String),

    #[error("command failed: {0}")]
    CommandFailed(String),
}

pub type Result<T> = std::result::Result<T, Error>;
