# codex-codes

A tightly typed Rust interface for the [OpenAI Codex CLI](https://github.com/openai/codex) JSON protocol.

This crate provides type-safe Rust representations of the Codex CLI's JSONL output format, mirroring the structure of the official [TypeScript SDK](https://github.com/openai/codex/tree/main/sdk/typescript).

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

## Example

```rust
use codex_codes::{ThreadEvent, ThreadItem};

let event_json = r#"{"type":"thread.started","thread_id":"th_abc"}"#;
let event: ThreadEvent = serde_json::from_str(event_json).unwrap();

let item_json = r#"{"type":"agent_message","id":"msg_1","text":"Hello!"}"#;
let item: ThreadItem = serde_json::from_str(item_json).unwrap();
```

## License

Apache-2.0
