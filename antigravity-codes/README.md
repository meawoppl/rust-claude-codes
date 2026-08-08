# antigravity-codes

[![Crates.io](https://img.shields.io/crates/v/antigravity-codes.svg)](https://crates.io/crates/antigravity-codes)
[![docs.rs](https://docs.rs/antigravity-codes/badge.svg)](https://docs.rs/antigravity-codes)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](../LICENSE)

Typed Rust interface for the [Google Antigravity](https://antigravity.google)
agent runtime — the `localharness` binary that ships inside the
[`google-antigravity`](https://pypi.org/project/google-antigravity/) wheels.

Tested against **google-antigravity 0.1.10**.

> **Maturity warning**: this crate is new and should be considered **highly
> untested**. Upstream is alpha (`0.1.x`) and reserves protobuf extension
> ranges on its hottest messages, so expect churn. Wire captures that break the
> types are very welcome in
> [issues](https://github.com/meawoppl/rust-code-agent-sdks/issues).

## What this wraps

`google-antigravity` on PyPI is a **Python client for a compiled Go binary**
called `localharness`, which is where the agent loop, the built-in tools, and
the model calls actually live. This crate is a client for that same binary — a
sibling of the Python SDK, not a binding to it. No Python at runtime.

## Getting the binary

The harness is distributed **only** inside the platform wheels on PyPI. There
is no standalone release, so nothing will put it on your `PATH` for you:

```sh
pip download google-antigravity --no-deps -d /tmp/ag
unzip -o -j /tmp/ag/*.whl 'google/antigravity/bin/localharness' -d ~/.local/bin
export ANTIGRAVITY_HARNESS_PATH=~/.local/bin/localharness
```

Discovery order is `$ANTIGRAVITY_HARNESS_PATH`, then `localharness` on `PATH`,
then whatever you pass to `HarnessOptions::binary`.

## Protocol

stdio is used **only to bootstrap**, then everything moves to a loopback
WebSocket:

| Step | Transport | Payload |
|---|---|---|
| 1. Handshake | stdio, `u32le`-length-prefixed | binary protobuf `InputConfig` → `OutputConfig` (port + API key) |
| 2. Connect | `ws://127.0.0.1:{port}/` | `x-goog-api-key` header |
| 3. Initialize | WebSocket | `InitializeConversationEvent` → `InitializeConversationResponse` |
| 4. Converse | WebSocket | `InputEvent` ↔ `OutputEvent` |

Everything after the handshake is protobuf's canonical **JSON** mapping:
`camelCase` members, 64-bit integers as strings, `bytes` as base64, enums as
value names.

A conversation **must** be configured with at least one model. A harness
initialised with none exits immediately and drops the socket without an error
frame — the crate surfaces the process's stderr in that case, because that is
the only diagnosis available.

Two things that bite on a first run:

- **Built-in tools are off unless enabled.** A harness with none will answer
  "I do not have file reading or command execution tools enabled" rather than
  read your workspace. `HarnessOptions` defaults to
  `HarnessSideTools::read_only()` — list, search, find, view, fetch — matching
  the reference Python SDK. Widen with `HarnessSideTools::all()` (shell and file
  writes) or narrow with `::none()`.
- **Free-tier quota is per model, and the `pro` models have none.** A request
  against one returns `429 … limit: 0` rather than an answer. `gemini-flash-latest`
  works on a free key.

## Usage

```toml
[dependencies]
antigravity-codes = "0.1"
```

```rust,no_run
use antigravity_codes::{Client, HarnessOptions, ModelBuilder};

#[tokio::main]
async fn main() -> antigravity_codes::Result<()> {
    let mut client = Client::launch(
        HarnessOptions::new()
            .workspace("/tmp/project")
            .model(ModelBuilder::gemini(
                "gemini-flash-latest",
                std::env::var("GEMINI_API_KEY").unwrap(),
            )),
    )
    .await?;

    let mut turn = client.send("What files are here?").await?;
    while let Some(step) = turn.next_step().await? {
        if let Some(text) = step.user_facing_text() {
            println!("{text}");
        }
    }

    client.shutdown().await
}
```

### Answering the harness

A turn is not one-way. Depending on configuration the harness stops and waits
for the client, and stays blocked until answered:

| Request | Raised when | Answered with |
|---|---|---|
| `ToolCall` | the model calls a tool declared via `HarnessOptions::tool` | `ToolResponse` |
| `CallHookRequest` | a lifecycle hook registered via `HarnessOptions::hook` fires | `CallHookResponse` |
| `PolicyDecisionRequest` | a dynamic policy rule needs adjudicating | `PolicyDecisionResponse` |
| `UserQuestionsRequest` | the agent asks the user something | `UserQuestionsResponse` |
| tool confirmation | a tool needs approval before it runs | `ToolConfirmation` |

`Client` answers all five from the `Handlers` you register. None of them arrive
unless the corresponding feature was configured, so an empty `Handlers` is fine
for plain chat. When one does arrive unhandled, the defaults keep the turn
moving: an unimplemented tool fails that one call, hooks return "no opinion",
policy returns `NO_MATCH`, questions are cancelled, and **tool confirmations are
refused** — silently approving would undo the control you asked for.

## Clients

| Type | What it gives you |
|------|-------------------|
| `RawClient` | The frames, unchanged. You drive the loop. |
| `Client` | Turn-oriented: streams assembled `Step`s and answers the harness for you. |

## Feature Flags

| Feature | Description | WASM-compatible |
|---------|-------------|-----------------|
| `types` | Wire types and the handshake codec only (serde) | Yes |
| `async-client` | Async WebSocket client using tokio | No |
| `integration-tests` | Enables tests that need a real harness binary | No |

```toml
antigravity-codes = { version = "0.1", default-features = false, features = ["types"] }
```

## Examples

```sh
export GEMINI_API_KEY=...            # https://aistudio.google.com/apikey
export ANTIGRAVITY_HARNESS_PATH=~/.local/bin/localharness

cargo run -p antigravity-codes --example stream_chat -- "what files are here?"
cargo run -p antigravity-codes --example custom_tool
cargo run -p antigravity-codes --example capture_frames -- ./captures "hello"
```

All three take `ANTIGRAVITY_MODEL` to override the model, defaulting to
`gemini-flash-latest`.

## Regenerating the protocol

The wire types are generated from the `FileDescriptorProto` embedded in the
wheel's `localharness_pb2.py` — **not** from the `.proto` files in the upstream
repo, which run ahead of what ships and are written in protobuf edition 2024
(unparseable by `protoc` < v31, and unsupported by `prost`).

```sh
pip download google-antigravity --no-deps -d /tmp/ag
python3 ../scripts/codegen_antigravity.py --wheel /tmp/ag/*.whl
```

Drift against the latest published wheel is checked nightly by
`scripts/check_antigravity_schema_drift.py`, which reads ~30 KB of the 37 MB
wheel using HTTP range requests.

## Testing

```sh
cargo test -p antigravity-codes --all-features

# Against a real harness. Most of these need no API key: the harness runs the
# whole turn lifecycle locally and only fails when it calls the model.
ANTIGRAVITY_HARNESS_PATH=~/.local/bin/localharness \
  cargo test -p antigravity-codes --features integration-tests
```

## License

Apache-2.0
