//! Error types for the muse-codes SDK.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Binary not found: '{name}' is not on PATH. Is Muse Code installed?")]
    BinaryNotFound { name: String },

    #[error("muse exec exited (code {code:?}) before the run reached a terminal state")]
    ExitedEarly { code: Option<i32> },
}

pub type Result<T> = std::result::Result<T, Error>;
