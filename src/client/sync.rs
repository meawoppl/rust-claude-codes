//! Synchronous client for Claude communication (stub implementation)

use crate::error::Result;

/// Synchronous client for communicating with Claude
/// This is a stub implementation - will be implemented later with native sync I/O
pub struct SyncClient;

impl SyncClient {
    /// Create a new synchronous client with default settings
    pub fn new() -> Result<Self> {
        todo!("Implement sync client with native std::process")
    }
}
