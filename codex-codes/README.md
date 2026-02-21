# codex-codes

[![Crates.io](https://img.shields.io/crates/v/codex-codes.svg)](https://crates.io/crates/codex-codes)
[![Documentation](https://docs.rs/codex-codes/badge.svg)](https://docs.rs/codex-codes)
[![CI](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/codex-codes.svg)](../LICENSE)
[![Downloads](https://img.shields.io/crates/d/codex-codes.svg)](https://crates.io/crates/codex-codes)

A typed Rust interface for the [OpenAI Codex CLI](https://github.com/openai/codex) JSONL protocol.

Part of the [rust-code-agent-sdks](https://github.com/meawoppl/rust-code-agent-sdks) workspace.

## Overview

This crate provides type-safe Rust representations of the Codex CLI's JSONL output format, mirroring the structure of the official [TypeScript SDK](https://github.com/openai/codex/tree/main/sdk/typescript). It includes optional sync and async clients for spawning and communicating with the Codex CLI.

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

### Async Client

```rust
use codex_codes::AsyncClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = AsyncClient::exec("What is 2 + 2?").await?;

    let mut stream = client.events();
    while let Some(result) = stream.next().await {
        let event = result?;
        println!("Event: {}", event.event_type());
    }

    Ok(())
}
```

### Sync Client

```rust
use codex_codes::SyncClient;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = SyncClient::exec("What is 2 + 2?")?;

    for result in client.events() {
        let event = result?;
        println!("Event: {}", event.event_type());
    }

    Ok(())
}
```

### Custom Builder

```rust
use codex_codes::{AsyncClient, CodexCliBuilder, SandboxMode};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let builder = CodexCliBuilder::new()
        .model("o4-mini")
        .sandbox(SandboxMode::ReadOnly)
        .full_auto(true);

    let mut client = AsyncClient::from_builder(builder, "List files in /tmp").await?;
    let events = client.collect_all().await?;

    for event in events {
        println!("{}", event.event_type());
    }

    Ok(())
}
```

### Raw Protocol Access

```rust
use codex_codes::{ThreadEvent, ThreadItem};

let event_json = r#"{"type":"thread.started","thread_id":"th_abc"}"#;
let event: ThreadEvent = serde_json::from_str(event_json).unwrap();

let item_json = r#"{"type":"agent_message","id":"msg_1","text":"Hello!"}"#;
let item: ThreadItem = serde_json::from_str(item_json).unwrap();
```

## Types

### Events (`ThreadEvent`)

Discriminated union of all events emitted during thread execution:

- `thread.started` — Thread initialized with an ID
- `turn.started` / `turn.completed` / `turn.failed` — Turn lifecycle
- `item.started` / `item.updated` / `item.completed` — Item lifecycle
- `error` — Thread-level error

### Items (`ThreadItem`)

Discriminated union of all agent action items:

- `agent_message` — Text output from the model
- `reasoning` — Chain-of-thought reasoning
- `command_execution` — Shell command with output and exit code
- `file_change` — File modifications (add/delete/update)
- `mcp_tool_call` — MCP tool invocation
- `web_search` — Web search query
- `todo_list` — Task tracking list
- `error` — Error item

### Options

- `ThreadOptions` — Per-thread configuration
- `ApprovalMode` — Tool execution approval policy
- `SandboxMode` — File system access control
- `ModelReasoningEffort` — Reasoning effort level
- `WebSearchMode` — Web search behavior

## Compatibility

**Tested against:** Codex CLI 0.104.0

The crate version tracks the Codex CLI version. If you're using a different CLI version, please report whether it works at:
https://github.com/meawoppl/rust-code-agent-sdks/issues

## License

Apache-2.0. See [LICENSE](../LICENSE).
