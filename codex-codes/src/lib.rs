//! A typed Rust interface for the OpenAI Codex CLI protocol.
//!
//! This crate provides type-safe representations of the Codex CLI's output
//! and the app-server's JSON-RPC protocol, plus optional sync and async clients
//! for multi-turn conversations.
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
//! # Protocol
//!
//! The crate supports two protocol modes:
//!
//! - **Exec JSONL** — The `codex exec --json -` one-shot protocol, parsed via
//!   [`ThreadEvent`] and [`ThreadItem`]
//! - **App-server JSON-RPC** — The `codex app-server` multi-turn protocol,
//!   using types from [`jsonrpc`] and [`protocol`]
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
pub mod jsonrpc;
pub mod protocol;

#[cfg(any(feature = "sync-client", feature = "async-client"))]
pub mod cli;

#[cfg(any(feature = "sync-client", feature = "async-client"))]
pub mod version;

#[cfg(feature = "sync-client")]
pub mod client_sync;

#[cfg(feature = "async-client")]
pub mod client_async;

// Exec-level event types (JSONL protocol)
pub use io::events::{
    ItemCompletedEvent, ItemStartedEvent, ItemUpdatedEvent, ThreadError, ThreadErrorEvent,
    ThreadEvent, ThreadStartedEvent, TurnCompletedEvent, TurnFailedEvent, TurnStartedEvent, Usage,
};

// Thread item types (shared between exec and app-server)
pub use io::items::{
    AgentMessageItem, CommandExecutionItem, CommandExecutionStatus, ErrorItem, FileChangeItem,
    FileUpdateChange, McpToolCallError, McpToolCallItem, McpToolCallResult, McpToolCallStatus,
    PatchApplyStatus, PatchChangeKind, ReasoningItem, ThreadItem, TodoItem, TodoListItem,
    WebSearchItem,
};

// Configuration types
pub use io::options::{
    ApprovalMode, ModelReasoningEffort, SandboxMode, ThreadOptions, WebSearchMode,
};

// Error types (always available)
pub use error::{Error, Result};

// JSON-RPC types (always available)
pub use jsonrpc::{
    JsonRpcError, JsonRpcErrorData, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest,
    JsonRpcResponse, RequestId,
};

// App-server protocol types (always available)
pub use protocol::{
    AgentMessageDeltaNotification, CmdOutputDeltaNotification, CommandApprovalDecision,
    CommandExecutionApprovalParams, CommandExecutionApprovalResponse, ErrorNotification,
    FileChangeApprovalDecision, FileChangeApprovalParams, FileChangeApprovalResponse,
    FileChangeOutputDeltaNotification, ItemCompletedNotification, ItemStartedNotification,
    ReasoningDeltaNotification, ServerMessage, ThreadArchiveParams, ThreadArchiveResponse,
    ThreadStartParams, ThreadStartResponse, ThreadStartedNotification, ThreadStatus,
    ThreadStatusChangedNotification, ThreadTokenUsageUpdatedNotification, TokenUsage, Turn,
    TurnCompletedNotification, TurnError, TurnInterruptParams, TurnInterruptResponse,
    TurnStartParams, TurnStartResponse, TurnStartedNotification, TurnStatus, UserInput,
};

// CLI builder (feature-gated)
#[cfg(any(feature = "sync-client", feature = "async-client"))]
pub use cli::AppServerBuilder;

// Sync client
#[cfg(feature = "sync-client")]
pub use client_sync::{EventIterator, SyncClient};

// Async client
#[cfg(feature = "async-client")]
pub use client_async::{AsyncClient, EventStream};
