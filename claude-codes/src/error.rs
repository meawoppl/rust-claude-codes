//! Error types for the Claude Code protocol

use crate::io::ParseError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Invalid message type: {0}")]
    InvalidMessageType(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid state transition: {0}")]
    InvalidState(String),

    #[error("Authorization code rejected: {message}")]
    CodeRejected { message: String },

    #[error(
        "Timed out awaiting a login outcome; CLI screen content after submission:\n{transcript}"
    )]
    LoginTimeout { transcript: String },

    /// The login CLI process exited without producing a login outcome.
    ///
    /// Reading `code` (signal deaths surface as `128 + signo`):
    /// - `0` / small integers — the CLI exited on its own (its error text,
    ///   if any, is in `transcript`).
    /// - `143` (SIGTERM) — terminated externally, OR the host application
    ///   dropped the `LoginFlow` mid-flight (the crate's `Drop` kills with
    ///   SIGTERM and, with the `log` feature, warns
    ///   "LoginFlow dropped while unfinished" — check logs to attribute).
    /// - `137` (SIGKILL) — killed externally (OOM killer, `kill -9`); the
    ///   crate never sends SIGKILL.
    #[error("Login CLI exited (code {code:?}) without a login outcome; output:\n{transcript}")]
    LoginChildExited {
        code: Option<u32>,
        transcript: String,
    },

    #[error("Timeout occurred")]
    Timeout,

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Deserialization error: {0}")]
    Deserialization(#[from] ParseError),

    #[error("Session UUID not yet available - no response received")]
    SessionNotInitialized,

    #[error("Binary not found: '{name}' is not on PATH. Is it installed?")]
    BinaryNotFound { name: String },

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, Error>;
