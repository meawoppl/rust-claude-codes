//! Error types for the harness client.

use std::path::PathBuf;

/// Anything that can go wrong driving a localharness process.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The `localharness` binary could not be located.
    ///
    /// It ships only inside the platform wheels published to PyPI, so there is
    /// no package manager that will put it on `PATH` for you — see
    /// [`crate::process::find_harness`].
    #[error(
        "could not find the localharness binary; set ANTIGRAVITY_HARNESS_PATH, put \
         `localharness` on PATH, or install the google-antigravity wheel"
    )]
    HarnessNotFound,

    /// The configured binary path does not exist or is not executable.
    #[error("localharness at {path} is not usable: {source}")]
    HarnessNotExecutable {
        /// The path that was tried.
        path: PathBuf,
        /// The underlying filesystem error.
        source: std::io::Error,
    },

    /// The harness exited before completing the stdio handshake.
    ///
    /// `stderr` is included because the harness reports configuration problems
    /// there and then exits, rather than replying on the wire.
    #[error("localharness exited during handshake: {stderr}")]
    HandshakeFailed {
        /// Whatever the process wrote to stderr before dying.
        stderr: String,
    },

    /// The handshake reply could not be parsed.
    #[error("malformed handshake reply: {0}")]
    Handshake(#[from] crate::handshake::DecodeError),

    /// The WebSocket never accepted a connection on the port the harness
    /// reported.
    #[error("could not connect to the harness WebSocket on port {port} after {attempts} attempts")]
    WebSocketUnreachable {
        /// The port from `OutputConfig`.
        port: u16,
        /// How many connection attempts were made.
        attempts: u32,
    },

    /// The WebSocket closed or failed mid-session.
    ///
    /// Boxed because `tungstenite::Error` is by far the largest thing this enum
    /// carries, and every fallible call in the crate returns this type.
    #[error("websocket error: {0}")]
    #[cfg(feature = "async-client")]
    WebSocket(Box<tokio_tungstenite::tungstenite::Error>),

    /// A frame arrived that this crate could not decode.
    ///
    /// The raw text is retained: unlike an unknown enum value or an unknown
    /// `oneof` arm — both of which decode fine by design — this means the frame
    /// is structurally unexpected, and the payload is what makes it reportable.
    #[error("could not decode frame: {source}")]
    Decode {
        /// The serde failure.
        source: serde_json::Error,
        /// The frame exactly as received.
        raw: String,
    },

    /// The session ended while a caller was waiting on it.
    #[error("the harness session has closed")]
    SessionClosed,

    /// The agent stopped without completing the turn.
    ///
    /// Raised after the turn's steps have been delivered, so whatever the agent
    /// managed to say before failing has already been seen.
    #[error("the turn failed: {message}")]
    Turn {
        /// What the harness reported on the trajectory.
        message: String,
    },

    /// Underlying process or socket I/O failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Convenience alias for fallible harness operations.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(feature = "async-client")]
impl From<tokio_tungstenite::tungstenite::Error> for Error {
    fn from(e: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::WebSocket(Box::new(e))
    }
}

#[cfg(feature = "async-client")]
impl Error {
    /// Builds a [`Error::Decode`] that keeps the offending frame.
    pub(crate) fn decode(source: serde_json::Error, raw: impl Into<String>) -> Self {
        Self::Decode {
            source,
            raw: raw.into(),
        }
    }
}
