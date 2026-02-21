//! A tightly typed Rust interface for the OpenAI Codex CLI JSON protocol.
//!
//! This crate provides type-safe representations of the Codex CLI's JSONL output,
//! mirroring the structure of the official TypeScript SDK (`@openai/codex-sdk`).
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
