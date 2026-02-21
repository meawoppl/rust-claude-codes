//! A tightly typed Rust interface for the OpenAI Codex CLI JSON protocol.
//!
//! This crate provides type-safe representations of the Codex CLI's JSONL output,
//! mirroring the structure of the official TypeScript SDK (`@openai/codex-sdk`).
//!
//! # Feature Flags
//!
//! | Feature | Description | WASM-compatible |
//! |---------|-------------|-----------------|
//! | `types` | Core message types and protocol structs only | Yes |
//! | `sync-client` | Synchronous client with blocking I/O | No |
//! | `async-client` | Asynchronous client using tokio | No |
//!
//! All features are enabled by default.
//!
//! # Message Types
//!
//! The protocol uses two primary discriminated unions:
//!
//! - [`ThreadEvent`] — Events emitted during thread execution (thread/turn/item lifecycle)
//! - [`ThreadItem`] — Data items representing agent actions (messages, commands, file changes, etc.)
//!
//! # Configuration Types
//!
//! - [`ThreadOptions`] — Per-thread settings (model, sandbox mode, approval policy)
//! - [`ApprovalMode`], [`SandboxMode`], [`ModelReasoningEffort`], [`WebSearchMode`] — Typed enums
//!
//! # Example
//!
//! ```
//! use codex_codes::{ThreadEvent, ThreadItem};
//!
//! let json = r#"{"type":"thread.started","thread_id":"th_abc"}"#;
//! let event: ThreadEvent = serde_json::from_str(json).unwrap();
//! ```

mod io;

pub mod error;

#[cfg(any(feature = "sync-client", feature = "async-client"))]
pub mod cli;

#[cfg(any(feature = "sync-client", feature = "async-client"))]
pub mod version;

#[cfg(feature = "sync-client")]
pub mod client_sync;

#[cfg(feature = "async-client")]
pub mod client_async;

// Events
pub use io::events::{
    ItemCompletedEvent, ItemStartedEvent, ItemUpdatedEvent, ThreadError, ThreadErrorEvent,
    ThreadEvent, ThreadStartedEvent, TurnCompletedEvent, TurnFailedEvent, TurnStartedEvent, Usage,
};

// Items
pub use io::items::{
    AgentMessageItem, CommandExecutionItem, CommandExecutionStatus, ErrorItem, FileChangeItem,
    FileUpdateChange, McpToolCallError, McpToolCallItem, McpToolCallResult, McpToolCallStatus,
    PatchApplyStatus, PatchChangeKind, ReasoningItem, ThreadItem, TodoItem, TodoListItem,
    WebSearchItem,
};

// Options
pub use io::options::{
    ApprovalMode, ModelReasoningEffort, SandboxMode, ThreadOptions, WebSearchMode,
};

// Error types (always available)
pub use error::{Error, Result};

// CLI builder (feature-gated)
#[cfg(any(feature = "sync-client", feature = "async-client"))]
pub use cli::CodexCliBuilder;

// Sync client
#[cfg(feature = "sync-client")]
pub use client_sync::{EventIterator, SyncClient};

// Async client
#[cfg(feature = "async-client")]
pub use client_async::{AsyncClient, EventStream};
