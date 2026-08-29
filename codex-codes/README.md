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

**Tested against:** Codex CLI 0.150.1

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
codex-codes = { version = "0.128", default-features = false, features = ["types"] }
```

#### Sync Client Only
```toml
[dependencies]
codex-codes = { version = "0.128", default-features = false, features = ["sync-client"] }
```

#### Async Client Only
```toml
[dependencies]
codex-codes = { version = "0.128", default-features = false, features = ["async-client"] }
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

Use `RawAsyncClient` when the caller owns JSON-RPC correlation and only needs
newline framing. Neither method decodes JSON.

```rust,no_run
use codex_codes::{AppServerBuilder, JsonRpcRequest, RawAsyncClient, RequestId};

# async fn example() -> codex_codes::Result<()> {
let builder = AppServerBuilder::new().working_directory("/workspace");
let mut client = RawAsyncClient::start_with(builder).await?;
let request = JsonRpcRequest {
    id: RequestId::Integer(1),
    method: "initialize".to_string(),
    params: Some(serde_json::json!({})),
};
client.send(&request).await?;
let raw_line = client.next_line().await?;
# Ok(())
# }
```

Typed protocol parsing remains available separately:

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

**Tested against:** Codex CLI 0.150.1

The crate version means **tested against**: it names the newest Codex CLI
the live integration suite has passed against. If you're using a different
CLI version, please report whether it works at:
https://github.com/meawoppl/rust-code-agent-sdks/issues

## Coverage scorecard

The Codex CLI publishes its own JSON Schema bundle via `codex app-server generate-json-schema --out DIR`. A snapshot of the output lives at `tests/schemas/codex_app_server_protocol.v2.schemas.json`.

Run the scorecard to see which JSON-RPC methods this crate models vs. what the upstream schema enumerates, and whether our typed structs' serde shape still matches the wire:

```bash
cargo run --example schema_coverage
```

Per method, the report marks:

- `✓` modeled in `codex-codes` and a hand-rolled sample validates against the schema (drift-checked)
- `◐` modeled, but no sample registered yet — grow the registry in `examples/schema_coverage.rs` to drift-check it
- `⚠` modeled, sample serialized, but did NOT match the schema (drift)
- `✗` not modeled at all

Override the schema with `CODEX_SCHEMA_PATH=/path/to/fresh/schemas.json` to validate against a freshly-generated schema (e.g. in CI).

## License

Apache-2.0. See [LICENSE](../LICENSE).

## Account & auth helpers

Codex login is drivable through the app-server protocol itself — no process
scraping:

- `account_read` — current account (plan, email, auth mode)
- `account_login_start` — three modes: `apiKey` (completes inline),
  `chatgpt` (returns a browser auth URL), `chatgptDeviceCode` (returns a
  user code + verification URL); browser/device completion arrives as the
  `account/login/completed` notification on the same connection
- `account_login_cancel`, `account_logout`, `account_rate_limits_read`,
  `account_usage_read`

For cheap status probes without an app-server connection,
`auth_local::auth_status_local()` reads `$CODEX_HOME/auth.json` and decodes
display-only email/plan labels from the stored id_token — best-effort by
design (the file is codex-internal; the crate owns that risk with fixture
and live tests). `logged_in` there means *stored*, not *live* — use
`account_read` or `codex login status` for liveness.
