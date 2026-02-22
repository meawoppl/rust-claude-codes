# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.100.1] - 2026-02-21

### Changed

- **Replaced `codex exec` with `codex app-server` JSON-RPC protocol** — The crate now wraps `codex app-server --listen stdio://` instead of the one-shot `codex exec --json -`. This enables multi-turn conversations, approval flows, and streaming notifications.

### Added

- **`jsonrpc` module** — JSON-RPC message types (`JsonRpcMessage`, `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`, `JsonRpcNotification`, `RequestId`) matching the app-server wire format (no `"jsonrpc":"2.0"` field)
- **`protocol` module** — App-server v2 protocol types including thread/turn lifecycle params, server notifications, approval flow types, and method name constants
- **`AppServerBuilder`** — Replaces `CodexCliBuilder` for spawning the long-lived app-server process
- **Multi-turn `AsyncClient`** — JSON-RPC client with `thread_start`, `turn_start`, `turn_interrupt`, `thread_archive`, `respond`, and `next_message` methods
- **Multi-turn `SyncClient`** — Blocking counterpart with the same API surface
- **Approval flow support** — `CommandExecutionApprovalParams/Response` and `FileChangeApprovalParams/Response` for handling server-to-client approval requests
- **Streaming delta notifications** — `AgentMessageDeltaNotification`, `CmdOutputDeltaNotification`, `FileChangeOutputDeltaNotification`, `ReasoningDeltaNotification`
- **camelCase serde aliases** on `ThreadItem` variants and status enums for app-server compatibility (snake_case exec format still supported)
- **`Declined` variant** on `CommandExecutionStatus` for commands rejected via approval flow
- **Comprehensive documentation** — Module-level docs with examples, lifecycle guides, error conditions, notification reference tables, and rustdoc examples across all public modules

### Removed

- `CodexCliBuilder` (replaced by `AppServerBuilder`)
- One-shot exec client API

## [0.100.0] - 2026-02-17

### Added

- Initial release of `codex-codes` crate
- Typed Rust bindings for the OpenAI Codex CLI JSON protocol
- `ThreadEvent` and `ThreadItem` types for parsing exec-format JSONL events
- `ThreadOptions` configuration types (`ApprovalMode`, `SandboxMode`, `WebSearchMode`)
- Sync and async clients wrapping `codex exec --json -`
- Feature flags: `types` (WASM-compatible), `sync-client`, `async-client`
- Integration tests with captured protocol message test cases
- Version compatibility checking against installed Codex CLI
