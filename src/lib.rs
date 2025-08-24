//! A tightly typed Rust interface for the Claude Code JSON protocol
//!
//! This crate provides type-safe encoding and decoding for the JSON lines
//! protocol used by Claude Code for communication.

pub mod cli;
pub mod client;
pub mod error;
pub mod io;
pub mod messages;
pub mod protocol;
pub mod types;
pub mod version;

pub use cli::{ClaudeCliBuilder, PermissionMode};
pub use client::{AsyncClient, SyncClient};
pub use error::{Error, Result};
pub use io::{AssistantMessageContent, ClaudeInput, ClaudeOutput, ParseError};
pub use messages::*;
pub use protocol::Protocol;
pub use types::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
