# hermes-codes

Typed Rust SDK for the [NousResearch Hermes agent](https://github.com/NousResearch/hermes-agent).

Hermes exposes its machine interface through the
[Agent Client Protocol](https://agentclientprotocol.com) (`hermes acp` —
newline-delimited JSON-RPC 2.0 over stdio, bidirectional). This crate models
that protocol **from the schema, not from another crate**: types are
generated from the exact schema variant hermes speaks, and an async Tokio
client drives the adapter process.

Tested against hermes-agent `0.20.0`.

## The provenance chain

hermes-agent speaks ACP through the Python SDK it pins, and that SDK
generates its models from the **unstable** schema variants at a specific
upstream tag:

```
hermes-agent 0.20.0
  └─ pins agent-client-protocol==0.9.0        (PyPI)
       └─ generated from refs/tags/v0.11.2    (zed-industries/agent-client-protocol)
            └─ schema.unstable.json + meta.unstable.json   ← our snapshots
```

The snapshots live in `tests/schemas/` next to
`hermes_acp_provenance.json`, which records the chain. A nightly workflow
re-derives every link from upstream and files an issue on drift; a second
CI job regenerates the bindings from the committed snapshots and fails if
the output differs from what's committed.

Because the snapshots are the *unstable* variants, this crate models the
methods hermes actually has (including `session/fork`, `session/resume`,
`session/close`, `session/set_model`) rather than only the stable spec.

## Features

| Feature | Description | WASM-compatible |
|---------|-------------|-----------------|
| `types` | Generated ACP types + hermes `_meta` extensions (serde only) | Yes |
| `async-client` | Tokio client that spawns and drives `hermes acp` | No |

`default = ["types", "async-client"]`.

## Quick start

```rust,ignore
use hermes_codes::{AsyncClient, ContentBlock, NewSessionRequest, PromptRequest, TextContent};

let (mut client, _init) = AsyncClient::start().await?;

let session = client
    .session_new(&NewSessionRequest {
        cwd: std::env::current_dir()?.to_string_lossy().into_owned(),
        ..Default::default()
    })
    .await?;

let outcome = client
    .session_prompt_with(
        &PromptRequest {
            session_id: session.session_id.clone(),
            prompt: vec![ContentBlock::Text(TextContent {
                text: "Summarize this repo".into(),
                ..Default::default()
            })],
            ..Default::default()
        },
        |msg| {
            // Streamed session/update notifications arrive here; answer
            // permission requests by returning Some(result_json).
            None
        },
    )
    .await?;
println!("stop reason: {:?}", outcome.stop_reason);
```

ACP is bidirectional: the agent sends the client requests it expects
answers to (`session/request_permission`, and `fs/*` / `terminal/*` if you
advertised those capabilities at `initialize`). Answer them inside the
`session_prompt_with` callback or via `respond` / `respond_error`.

## Hermes extensions

Hermes attaches implementation data under ACP's `_meta.hermes`
extensibility key. `HermesMeta::from_meta` lifts it from any generated
type's `meta` field — `sessionProvenance` (session rotation/replay
lineage) and compaction-summary markers are modeled; everything else rides
in an open `extra` map.

## Testing

- **Golden corpus**: `tests/golden/` is vendored verbatim from the ACP
  Python SDK's test fixtures (Apache-2.0) at the exact version hermes
  pins — every round-trip test runs against frames the reference
  implementation produces.
- **Schema coverage**: `cargo run --example schema_coverage` reports the
  method table and validates golden round-trips per method.
- **Live tests** (feature `integration-tests`) require a `hermes`
  binary on `PATH` with configured provider credentials.
