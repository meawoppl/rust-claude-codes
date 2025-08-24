//! High-level client implementations for interacting with Claude.
//!
//! This module provides two client types for different use cases:
//!
//! - [`AsyncClient`] - Asynchronous client using Tokio (recommended for most applications)
//! - [`SyncClient`] - Synchronous client using standard library threads
//!
//! Both clients handle:
//! - Process lifecycle management
//! - Message serialization/deserialization
//! - Response streaming
//! - Error handling
//! - Automatic version compatibility checking
//!
//! # Choosing a Client
//!
//! Use [`AsyncClient`] when:
//! - Building async applications with Tokio
//! - You need concurrent operations
//! - Working with web frameworks like Axum or Actix
//!
//! Use [`SyncClient`] when:
//! - Building CLI tools without async runtime
//! - Integrating with synchronous codebases
//! - Simplicity is more important than performance
//!
//! # Example
//!
//! ```no_run
//! # async fn async_example() -> Result<(), Box<dyn std::error::Error>> {
//! use claude_codes::AsyncClient;
//!
//! let mut client = AsyncClient::with_defaults().await?;
//! let responses = client.query("Hello, Claude!").await?;
//! # Ok(())
//! # }
//!
//! # fn sync_example() -> Result<(), Box<dyn std::error::Error>> {
//! use claude_codes::{SyncClient, ClaudeInput};
//!
//! let mut client = SyncClient::with_defaults()?;
//! let input = ClaudeInput::user_message("Hello!", "session");
//! let responses = client.query(input)?;
//! # Ok(())
//! # }
//! ```

pub mod r#async;
pub mod sync;

pub use r#async::AsyncClient;
pub use sync::SyncClient;
