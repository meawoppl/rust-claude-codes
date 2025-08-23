//! A tightly typed Rust interface for the Claude Code JSON protocol
//!
//! This crate provides type-safe encoding and decoding for the JSON lines
//! protocol used by Claude Code for communication.

pub mod error;
pub mod messages;
pub mod protocol;
pub mod types;

pub use error::{Error, Result};
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
