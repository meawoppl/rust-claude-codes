//! A typed Rust interface for the [OpenAI Codex CLI](https://github.com/openai/codex) protocol.
//!
//! This crate provides type-safe bindings for communicating with the Codex CLI's
//! app-server via its JSON-RPC protocol. It handles message framing, request/response
//! correlation, approval flows, and streaming notifications for multi-turn agent
//! conversations.
//!
//! # Quick Start
//!
//! ```bash
//! cargo add codex-codes
//! ```
//!
//! ## Using the Async Client (Recommended)
//!
//! ```ignore
//! use codex_codes::{AsyncClient, ThreadStartParams, TurnStartParams, UserInput, ServerMessage};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Start the app-server (spawns `codex app-server --listen stdio://`)
//!     let mut client = AsyncClient::start().await?;
//!
//!     // Create a thread (a conversation session)
//!     let thread = client.thread_start(&ThreadStartParams::default()).await?;
//!
//!     // Send a turn (a user message that triggers an agent response)
//!     client.turn_start(&TurnStartParams {
//!         thread_id: thread.thread_id().to_string(),
//!         input: vec![UserInput::Text { text: "What is 2 + 2?".into() }],
//!         model: None,
//!         reasoning_effort: None,
//!         sandbox_policy: None,
//!     }).await?;
//!
//!     // Stream notifications until the turn completes
//!     while let Some(msg) = client.next_message().await? {
//!         match msg {
//!             ServerMessage::Notification { method, params } => {
//!                 if method == "turn/completed" { break; }
//!             }
//!             ServerMessage::Request { id, .. } => {
//!                 // Approval request — auto-accept for this example
//!                 client.respond(id, &serde_json::json!({"decision": "accept"})).await?;
//!             }
//!         }
//!     }
//!
//!     client.shutdown().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Using the Sync Client
//!
//! ```ignore
//! use codex_codes::{SyncClient, ThreadStartParams, TurnStartParams, UserInput, ServerMessage};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut client = SyncClient::start()?;
//!     let thread = client.thread_start(&ThreadStartParams::default())?;
//!
//!     client.turn_start(&TurnStartParams {
//!         thread_id: thread.thread_id().to_string(),
//!         input: vec![UserInput::Text { text: "What is 2 + 2?".into() }],
//!         model: None,
//!         reasoning_effort: None,
//!         sandbox_policy: None,
//!     })?;
//!
//!     for result in client.events() {
//!         match result? {
//!             ServerMessage::Notification { method, .. } => {
//!                 if method == "turn/completed" { break; }
//!             }
//!             _ => {}
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! # Architecture
//!
//! The crate is organized into several key modules:
//!
//! - [`client_async`] / [`client_sync`] — High-level clients that manage the
//!   app-server process, request/response correlation, and message buffering
//! - [`protocol`] — App-server v2 request params, response types, and notification
//!   bodies (thread/turn lifecycle, approvals, deltas)
//! - [`jsonrpc`] — Low-level JSON-RPC message types (request, response, error,
//!   notification) matching the app-server's wire format
//! - [`cli`] — Builder for spawning `codex app-server --listen stdio://`
//! - [`error`] — Error types and result aliases
//! - [`version`] — Version compatibility checking against the installed CLI
//!
//! # Protocol Overview
//!
//! The Codex app-server communicates via newline-delimited JSON-RPC 2.0 over stdio
//! (without the standard `"jsonrpc":"2.0"` field). The conversation lifecycle is:
//!
//! 1. **Initialize** — `initialize` + `initialized` handshake (handled automatically by `start()`)
//! 2. **Start a thread** — `thread/start` creates a conversation session
//! 3. **Start a turn** — `turn/start` sends user input, triggering agent work
//! 4. **Stream notifications** — The server emits `item/agentMessage/delta`,
//!    `item/commandExecution/outputDelta`, etc. as the agent works
//! 5. **Handle approvals** — The server may send requests like
//!    `item/commandExecution/requestApproval` that require a response
//! 6. **Turn completes** — `turn/completed` signals the agent is done
//! 7. **Repeat** — Send another `turn/start` for follow-up questions
//!
//! # Feature Flags
//!
//! | Feature | Description | WASM-compatible |
//! |---------|-------------|-----------------|
//! | `types` | Core message types and protocol structs only | Yes |
//! | `sync-client` | Synchronous client with blocking I/O | No |
//! | `async-client` | Asynchronous client using tokio | No |
//!
//! All features are enabled by default. For WASM or type-sharing use cases:
//!
//! ```toml
//! [dependencies]
//! codex-codes = { version = "0.100", default-features = false, features = ["types"] }
//! ```
//!
//! # Version Compatibility
//!
//! The Codex CLI protocol is evolving. This crate automatically checks your
//! installed CLI version and warns if it's newer than tested. Current tested
//! version: **0.104.0**
//!
//! Report compatibility issues at: <https://github.com/meawoppl/rust-code-agent-sdks/issues>
//!
//! # Examples
//!
//! See the `examples/` directory for complete working examples:
//! - `async_client.rs` — Single-turn async query with streaming deltas
//! - `sync_client.rs` — Single-turn synchronous query
//! - `basic_repl.rs` — Interactive REPL with multi-turn conversation and approval handling
//!
//! # Parsing Raw Protocol Messages
//!
//! ```
//! use codex_codes::{ThreadEvent, ThreadItem, JsonRpcMessage};
//!
//! // Parse exec-format JSONL events
//! let json = r#"{"type":"thread.started","thread_id":"th_abc"}"#;
//! let event: ThreadEvent = serde_json::from_str(json).unwrap();
//!
//! // Parse app-server JSON-RPC messages
//! let rpc = r#"{"id":1,"result":{"threadId":"th_abc"}}"#;
//! let msg: JsonRpcMessage = serde_json::from_str(rpc).unwrap();
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
    AgentMessageDeltaNotification, ClientInfo, CmdOutputDeltaNotification, CommandApprovalDecision,
    CommandExecutionApprovalParams, CommandExecutionApprovalResponse, ErrorNotification,
    FileChangeApprovalDecision, FileChangeApprovalParams, FileChangeApprovalResponse,
    FileChangeOutputDeltaNotification, InitializeCapabilities, InitializeParams,
    InitializeResponse, ItemCompletedNotification, ItemStartedNotification,
    ReasoningDeltaNotification, ServerMessage, ThreadArchiveParams, ThreadArchiveResponse,
    ThreadInfo, ThreadStartParams, ThreadStartResponse, ThreadStartedNotification, ThreadStatus,
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
