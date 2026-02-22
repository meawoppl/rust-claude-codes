use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Process exited with status {0}: {1}")]
    ProcessFailed(i32, String),

    #[error("JSON-RPC error ({code}): {message}")]
    JsonRpc { code: i64, message: String },

    #[error("Server closed connection")]
    ServerClosed,

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, Error>;
