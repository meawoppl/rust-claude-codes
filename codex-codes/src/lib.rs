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
//! See `examples/async_client.rs` and `examples/sync_client.rs` for runnable
//! versions of the usage patterns below. The high-level shape:
//!
//! ```ignore
//! let mut client = AsyncClient::start().await?;
//! let thread = client
//!     .thread_start(&serde_json::from_value(serde_json::json!({}))?)
//!     .await?;
//! client.turn_start(&TurnStartParams {
//!     thread_id: thread.thread.id.clone(),
//!     input: vec![UserInput::Text {
//!         text: "What is 2 + 2?".into(),
//!         text_elements: None,
//!     }],
//!     // All other fields default to None — populate when overriding
//!     // model / approval / sandbox / etc. for this turn.
//!     ..serde_json::from_value(serde_json::json!({"threadId": thread.thread.id, "input": []}))?
//! }).await?;
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
//! codex-codes = { version = "0.128", default-features = false, features = ["types"] }
//! ```
//!
//! # Version Compatibility
//!
//! The Codex CLI protocol is evolving. This crate automatically checks your
//! installed CLI version and warns if it's newer than tested. Current tested
//! version: **0.137.0**
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

pub mod io;

pub mod error;
pub mod jsonrpc;
pub mod messages;
pub mod models;
pub mod protocol;
pub mod protocol_generated;

#[cfg(any(feature = "sync-client", feature = "async-client"))]
pub mod auth_local;
#[cfg(any(feature = "sync-client", feature = "async-client"))]
pub mod cli;

#[cfg(any(feature = "sync-client", feature = "async-client"))]
pub mod version;

#[cfg(any(feature = "sync-client", feature = "async-client"))]
mod process;

#[cfg(any(feature = "sync-client", feature = "async-client"))]
mod stderr_drain;

#[cfg(feature = "sync-client")]
pub mod client_sync;

#[cfg(feature = "async-client")]
pub mod client_async;

#[cfg(feature = "async-client")]
pub mod client_raw_async;

// Exec-level event types (JSONL protocol)
pub use io::events::{
    ItemCompletedEvent, ItemStartedEvent, ItemUpdatedEvent, ThreadError, ThreadErrorEvent,
    ThreadEvent, ThreadStartedEvent, TurnCompletedEvent, TurnFailedEvent, TurnStartedEvent, Usage,
};

// Error types (always available)
pub use error::{Error, ParseError, Result};

// JSON-RPC types (always available)
pub use jsonrpc::{
    JsonRpcError, JsonRpcErrorData, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest,
    JsonRpcResponse, RequestId,
};

// App-server protocol types — generated from the upstream JSON Schema and
// re-exported through `protocol::*`.
pub use protocol::*;

// Typed message dispatch (notifications + server-to-client requests)
pub use messages::{Notification, ServerMessage, ServerRequest};

// Model catalog convenience enum
pub use models::CodexModel;

// CLI builder (feature-gated)
#[cfg(any(feature = "sync-client", feature = "async-client"))]
pub use cli::AppServerBuilder;

// Sync client
#[cfg(feature = "sync-client")]
pub use client_sync::{EventIterator, SyncClient};

// Async client
#[cfg(feature = "async-client")]
pub use client_async::{AsyncClient, EventStream};

#[cfg(feature = "async-client")]
pub use client_raw_async::RawAsyncClient;
