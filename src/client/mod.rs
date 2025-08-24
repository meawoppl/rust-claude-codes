//! Client module for Claude communication
//!
//! This module provides both synchronous and asynchronous clients
//! for interacting with Claude via the JSON protocol.

pub mod r#async;
pub mod sync;

pub use r#async::AsyncClient;
pub use sync::SyncClient;
