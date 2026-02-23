# codex-codes

[![Crates.io](https://img.shields.io/crates/v/codex-codes.svg)](https://crates.io/crates/codex-codes)
[![Documentation](https://docs.rs/codex-codes/badge.svg)](https://docs.rs/codex-codes)
[![CI](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/codex-codes.svg)](../LICENSE)
[![Downloads](https://img.shields.io/crates/d/codex-codes.svg)](https://crates.io/crates/codex-codes)

A typed Rust interface for the [OpenAI Codex CLI](https://github.com/openai/codex) app-server JSON-RPC protocol.

Part of the [rust-code-agent-sdks](https://github.com/meawoppl/rust-code-agent-sdks) workspace.

## Overview

This crate provides type-safe Rust representations of the Codex CLI's JSON-RPC protocol, used by `codex app-server`. It includes optional sync and async clients for multi-turn conversations with the Codex agent.

**Tested against:** Codex CLI 0.104.0

## Installation

### Default (All Features)
```bash
cargo add codex-codes
```

Requires the [Codex CLI](https://github.com/openai/codex) (`codex` binary) to be installed and available in PATH.

### Feature Flags

| Feature | Description | WASM-compatible |
|---------|-------------|-----------------|
| `types` | Core message types only (minimal dependencies) | Yes |
| `sync-client` | Synchronous client with blocking I/O | No |
| `async-client` | Asynchronous client with tokio runtime | No |

All features are enabled by default.

#### Types Only (WASM-compatible)
```toml
[dependencies]
codex-codes = { version = "0.100", default-features = false, features = ["types"] }
```

#### Sync Client Only
```toml
[dependencies]
codex-codes = { version = "0.100", default-features = false, features = ["sync-client"] }
```

#### Async Client Only
```toml
[dependencies]
codex-codes = { version = "0.100", default-features = false, features = ["async-client"] }
```

## Usage

### Async Client (Multi-Turn)

```rust
use codex_codes::{AsyncClient, ThreadStartParams, TurnStartParams, UserInput, ServerMessage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = AsyncClient::start().await?;

    // Start a thread
    let thread = client.thread_start(&ThreadStartParams::default()).await?;

    // Send a turn
    client.turn_start(&TurnStartParams {
        thread_id: thread.thread_id().to_string(),
        input: vec![UserInput::Text { text: "What is 2 + 2?".into() }],
        model: None,
        reasoning_effort: None,
        sandbox_policy: None,
    }).await?;

    // Stream notifications
    while let Some(msg) = client.next_message().await? {
        match msg {
            ServerMessage::Notification { method, params } => {
                println!("{}: {:?}", method, params);
                if method == "turn/completed" { break; }
            }
            ServerMessage::Request { id, method, .. } => {
                // Handle approval requests
                client.respond(id, &serde_json::json!({"decision": "accept"})).await?;
            }
        }
    }

    client.shutdown().await?;
    Ok(())
}
```

### Sync Client (Multi-Turn)

```rust
use codex_codes::{SyncClient, ThreadStartParams, TurnStartParams, UserInput, ServerMessage};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = SyncClient::start()?;

    let thread = client.thread_start(&ThreadStartParams::default())?;
    client.turn_start(&TurnStartParams {
        thread_id: thread.thread_id().to_string(),
        input: vec![UserInput::Text { text: "What is 2 + 2?".into() }],
        model: None,
        reasoning_effort: None,
        sandbox_policy: None,
    })?;

    for result in client.events() {
        let msg = result?;
        match &msg {
            ServerMessage::Notification { method, .. } => {
                if method == "turn/completed" { break; }
            }
            _ => {}
        }
    }

    Ok(())
}
```

### Raw Protocol Access

```rust
use codex_codes::{ThreadItem, JsonRpcMessage, RequestId};

// Parse exec-format JSONL events
let item_json = r#"{"type":"agent_message","id":"msg_1","text":"Hello!"}"#;
let item: ThreadItem = serde_json::from_str(item_json).unwrap();

// Parse app-server JSON-RPC messages
let rpc_json = r#"{"id":1,"result":{"threadId":"th_abc"}}"#;
let msg: JsonRpcMessage = serde_json::from_str(rpc_json).unwrap();
```

## Protocol

The crate supports two protocol modes:

### App-Server JSON-RPC (Primary)

The `codex app-server --listen stdio://` process speaks a JSON-RPC 2.0 protocol (without the `"jsonrpc":"2.0"` field) over newline-delimited stdio.

**Lifecycle:** `initialize` -> `thread/start` -> `turn/start` -> stream notifications -> `turn/completed` -> next `turn/start`

**Approval flows:** The server sends requests back to the client for command execution and file change approvals.

### Exec JSONL (Legacy)

The `codex exec --json -` one-shot protocol emits `ThreadEvent` JSONL lines. These types are still available for parsing captures.

## Types

### JSON-RPC (`jsonrpc` module)

- `RequestId` -- String or integer request identifier
- `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`, `JsonRpcNotification`
- `JsonRpcMessage` -- Untagged union of all message types

### Protocol (`protocol` module)

- Thread lifecycle: `ThreadStartParams/Response`, `ThreadArchiveParams/Response`
- Turn lifecycle: `TurnStartParams/Response`, `TurnInterruptParams/Response`
- Notifications: `TurnCompletedNotification`, `AgentMessageDeltaNotification`, etc.
- Approvals: `CommandExecutionApprovalParams/Response`, `FileChangeApprovalParams/Response`
- `UserInput`, `Turn`, `TurnStatus`, `ServerMessage`

### Items (`ThreadItem`)

Discriminated union of agent action items (shared between exec and app-server):

- `agent_message` / `agentMessage` -- Text output from the model
- `reasoning` -- Chain-of-thought reasoning
- `command_execution` / `commandExecution` -- Shell command with output
- `file_change` / `fileChange` -- File modifications
- `mcp_tool_call` / `mcpToolCall` -- MCP tool invocation
- `web_search` / `webSearch` -- Web search query
- `todo_list` / `todoList` -- Task tracking list
- `error` -- Error item

### Events (`ThreadEvent`) -- Exec Format

- `thread.started`, `turn.started`, `turn.completed`, `turn.failed`
- `item.started`, `item.updated`, `item.completed`
- `error`

## Compatibility

**Tested against:** Codex CLI 0.104.0

The crate version tracks the Codex CLI version. If you're using a different CLI version, please report whether it works at:
https://github.com/meawoppl/rust-code-agent-sdks/issues

## License

Apache-2.0. See [LICENSE](../LICENSE).
