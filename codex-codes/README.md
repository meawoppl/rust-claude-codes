# codex-codes

[![Crates.io](https://img.shields.io/crates/v/codex-codes.svg)](https://crates.io/crates/codex-codes)
[![Documentation](https://docs.rs/codex-codes/badge.svg)](https://docs.rs/codex-codes)
[![CI](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/codex-codes.svg)](../LICENSE)
[![Downloads](https://img.shields.io/crates/d/codex-codes.svg)](https://crates.io/crates/codex-codes)

A typed Rust interface for the [OpenAI Codex CLI](https://github.com/openai/codex) JSONL protocol.

Part of the [rust-code-agent-sdks](https://github.com/meawoppl/rust-code-agent-sdks) workspace.

## Overview

This crate provides type-safe Rust representations of the Codex CLI's JSONL output format, mirroring the structure of the official [TypeScript SDK](https://github.com/openai/codex/tree/main/sdk/typescript). It is a pure types crate with no feature flags and is WASM-compatible out of the box.

**Tested against:** Codex CLI 0.104.0

## Installation

```bash
cargo add codex-codes
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

## Usage

```rust
use codex_codes::{ThreadEvent, ThreadItem};

let event_json = r#"{"type":"thread.started","thread_id":"th_abc"}"#;
let event: ThreadEvent = serde_json::from_str(event_json).unwrap();

let item_json = r#"{"type":"agent_message","id":"msg_1","text":"Hello!"}"#;
let item: ThreadItem = serde_json::from_str(item_json).unwrap();
```

### Parsing a JSONL stream

```rust
use codex_codes::ThreadEvent;

fn parse_stream(jsonl: &str) -> Vec<ThreadEvent> {
    jsonl
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
```

## License

Apache-2.0. See [LICENSE](../LICENSE).
