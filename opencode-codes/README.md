# opencode-codes

[![Crates.io](https://img.shields.io/crates/v/opencode-codes.svg)](https://crates.io/crates/opencode-codes)
[![Documentation](https://docs.rs/opencode-codes/badge.svg)](https://docs.rs/opencode-codes)
[![CI](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/opencode-codes.svg)](../LICENSE)
[![Downloads](https://img.shields.io/crates/d/opencode-codes.svg)](https://crates.io/crates/opencode-codes)

A typed Rust interface for the [opencode](https://opencode.ai) agent server's
HTTP + Server-Sent Events protocol.

Part of the [rust-code-agent-sdks](https://github.com/meawoppl/rust-code-agent-sdks) workspace.

> ⚠️ **Maturity warning**: this crate is new and should be considered
> **highly untested** — it is under active development. The types are
> generated from opencode's published OpenAPI spec and the suite passes
> against a live server, but real-world mileage is minimal and the API may
> change between releases while the surface settles. Bug reports and wire
> captures that break the types are very welcome:
> <https://github.com/meawoppl/rust-code-agent-sdks/issues>.

## Overview

Where the sibling crates wrap CLIs that speak JSON-Lines over stdio,
`opencode-codes` wraps opencode's **local HTTP server**. It provides serde
models of the server's OpenAPI 3.1 wire contract, an async (Tokio) client over
`reqwest`, an SSE reader for the `GET /event` stream, and an optional managed
launcher for the `opencode serve` process.

**Tested against:** opencode 1.18.29

## Installation

### Default (types + async client)

```bash
cargo add opencode-codes
```

Requires an `opencode` binary to run a local server (or point the client at an
already-running server).

### Feature Flags

| Feature | Description | WASM-compatible |
|---------|-------------|-----------------|
| `types` | Core protocol types only (serde) | Yes |
| `async-client` | Async HTTP/SSE client using reqwest + tokio | No |
| `server` | Managed `opencode serve` launcher (picks a free port) | No |
| `integration-tests` | Enables tests that require a live opencode server | No |

`default = ["types", "async-client"]`.

#### Types Only (WASM-compatible)

```toml
[dependencies]
opencode-codes = { version = "1.18", default-features = false, features = ["types"] }
```

#### With the managed server launcher

```toml
[dependencies]
opencode-codes = { version = "1.18", features = ["server"] }
```

## Protocol

opencode exposes a REST + SSE surface. The conversation lifecycle:

1. **Create a session** — `POST /session`.
2. **Subscribe** — open the `GET /event` SSE stream.
3. **Prompt** — `POST /session/{sessionID}/prompt_async` returns immediately;
   the agent's work is observed on the SSE stream.
4. **Handle permissions** — a permission request arrives on the stream; the
   reply is a *separate* REST call to
   `POST /session/{sessionID}/permissions/{permissionID}`. Correlating a
   request to its reply is the consumer's job — this crate keeps the pending
   permission surface explicit rather than hiding it behind a callback.
5. **Reconcile** — SSE is best-effort and must not be trusted alone. Poll
   `GET /session/{sessionID}/message` to reconcile the authoritative message
   state against what the stream delivered.
6. **Abort** — `POST /session/{sessionID}/abort` cancels in-flight work.

Authentication is HTTP Basic when `OPENCODE_SERVER_PASSWORD` is set; the
username defaults to `"opencode"`.

## Quick start

```rust,ignore
use opencode_codes::client_async::OpencodeClient;
use opencode_codes::protocol_generated::types::{
    PromptAsyncParams, PromptAsyncParamsPartsItem, SessionCreateParams, TextPartInput,
};
use opencode_codes::sse::{EventStream, RetryConfig, StreamEvent};

#[tokio::main]
async fn main() -> opencode_codes::Result<()> {
    // Point the client at an already-running server (or use the `server`
    // feature to launch one on a free port).
    let base_url = "http://127.0.0.1:41999";
    let client = OpencodeClient::builder().base_url(base_url).build()?;

    let session = client
        .create_session(&SessionCreateParams {
            title: Some("demo".into()),
            ..Default::default()
        })
        .await?;

    // Subscribe to GET /event, reusing the client's base URL and auth.
    let mut events = client.event_stream(RetryConfig::default())?;

    client
        .prompt_async(
            &session.id,
            &PromptAsyncParams {
                parts: vec![PromptAsyncParamsPartsItem::Text(TextPartInput {
                    text: "List the files in this project".into(),
                    ..Default::default()
                })],
                ..Default::default()
            },
        )
        .await?;

    while let Some(item) = events.next().await {
        match item? {
            // A reconnect may have dropped frames; reconcile authoritatively.
            StreamEvent::Connected => {
                let _messages = client.list_messages(&session.id).await?;
            }
            // Observe message parts / tool activity. On a permission request,
            // reply via `client.respond_permission(..)`, then reconcile with a
            // poll of `client.list_messages(&session.id)`.
            StreamEvent::Event(_event) => {}
            StreamEvent::Unknown(_raw) => {}
            // `StreamEvent` is `#[non_exhaustive]`; a wildcard keeps downstream
            // code compiling as new variants are added.
            _ => {}
        }
    }
    Ok(())
}
```

## Provenance

Module layout, SSE handling, and server-lifecycle lessons were informed by the
Apache-2.0 licensed `opencode_rs` reference SDK. No code is copied verbatim; it
is credited here as a design reference in keeping with its license.

## License

Apache-2.0. See [LICENSE](../LICENSE).
