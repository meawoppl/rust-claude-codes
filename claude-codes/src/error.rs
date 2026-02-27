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

    #[error("Timeout occurred")]
    Timeout,

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Deserialization error: {0}")]
    Deserialization(#[from] ParseError),

    #[error("Session UUID not yet available - no response received")]
    SessionNotInitialized,

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, Error>;
