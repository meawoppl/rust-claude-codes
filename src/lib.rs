//! A tightly typed Rust interface for the Claude Code JSON protocol
//!
//! This crate provides type-safe bindings for interacting with the Claude CLI
//! through its JSON Lines protocol. It handles the complexity of message serialization,
//! deserialization, and streaming communication with Claude.
//!
//! # Quick Start
//!
//! Add this crate to your project:
//! ```bash
//! cargo add claude-codes
//! ```
//!
//! ## Using the Async Client (Recommended)
//!
//! ```ignore
//! use claude_codes::AsyncClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a client with automatic version checking
//!     let mut client = AsyncClient::with_defaults().await?;
//!
//!     // Send a query and stream responses
//!     let mut stream = client.query_stream("What is 2 + 2?").await?;
//!
//!     while let Some(response) = stream.next().await {
//!         match response {
//!             Ok(output) => {
//!                 println!("Received: {}", output.message_type());
//!                 // Handle different message types
//!             }
//!             Err(e) => eprintln!("Error: {}", e),
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Using the Sync Client
//!
//! ```ignore
//! use claude_codes::{SyncClient, ClaudeInput};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a synchronous client
//!     let mut client = SyncClient::with_defaults()?;
//!
//!     // Build a structured input message
//!     let input = ClaudeInput::user_message("What is 2 + 2?", uuid::Uuid::new_v4());
//!
//!     // Send and collect all responses
//!     let responses = client.query(input)?;
//!
//!     for response in responses {
//!         println!("Received: {}", response.message_type());
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! # Architecture
//!
//! The crate is organized into several key modules:
//!
//! - [`client`] - High-level async and sync clients for easy interaction
//! - [`protocol`] - Core JSON Lines protocol implementation
//! - [`io`] - Top-level message types (`ClaudeInput`, `ClaudeOutput`)
//! - [`messages`] - Detailed message structures for requests and responses
//! - [`cli`] - Builder for configuring Claude CLI invocation
//! - [`error`] - Error types and result aliases
//! - [`version`] - Version compatibility checking
//!
//! # Version Compatibility
//!
//! ⚠️ **Important**: The Claude CLI protocol is unstable and evolving. This crate
//! automatically checks your Claude CLI version and warns if it's newer than tested.
//!
//! Current tested version: **2.1.3**
//!
//! Report compatibility issues at: <https://github.com/meawoppl/rust-claude-codes/pulls>
//!
//! # Message Types
//!
//! The protocol uses several message types:
//!
//! - **System** - Initialization and metadata messages
//! - **User** - Input messages from the user
//! - **Assistant** - Claude's responses
//! - **Result** - Session completion with timing and cost info
//!
//! # Examples
//!
//! See the `examples/` directory for complete working examples:
//! - `async_client.rs` - Simple async client usage
//! - `sync_client.rs` - Synchronous client usage
//! - `basic_repl.rs` - Interactive REPL implementation

// Core modules always available
pub mod error;
pub mod io;
pub mod messages;
pub mod protocol;
pub mod tool_inputs;
pub mod types;

// Client modules
#[cfg(feature = "async-client")]
pub mod client_async;
#[cfg(feature = "sync-client")]
pub mod client_sync;

// Client-related modules
#[cfg(any(feature = "sync-client", feature = "async-client"))]
pub mod cli;
#[cfg(any(feature = "sync-client", feature = "async-client"))]
pub mod version;

// Core exports always available
pub use error::{Error, Result};
pub use io::{AssistantMessageContent, ClaudeInput, ClaudeOutput, ParseError};
pub use messages::*;
pub use protocol::{MessageEnvelope, Protocol};
pub use types::*;

// Control protocol types for tool permission handling
pub use io::{
    ControlRequest, ControlRequestMessage, ControlRequestPayload, ControlResponse,
    ControlResponseMessage, ControlResponsePayload, HookCallbackRequest, InitializeRequest,
    McpMessageRequest, PermissionResult, ToolPermissionRequest, ToolUseBlock,
};

// Typed tool input types
pub use tool_inputs::{
    AllowedPrompt, AskUserQuestionInput, BashInput, EditInput, EnterPlanModeInput,
    ExitPlanModeInput, GlobInput, GrepInput, KillShellInput, NotebookEditInput, Question,
    QuestionMetadata, QuestionOption, ReadInput, SkillInput, TaskInput, TaskOutputInput,
    TodoItem, TodoWriteInput, ToolInput, WebFetchInput, WebSearchInput, WriteInput,
};

// Client exports
#[cfg(feature = "async-client")]
pub use client_async::{AsyncClient, AsyncStreamProcessor};
#[cfg(feature = "sync-client")]
pub use client_sync::{StreamProcessor, SyncClient};

// Client-related exports
#[cfg(any(feature = "sync-client", feature = "async-client"))]
pub use cli::{ClaudeCliBuilder, PermissionMode};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
