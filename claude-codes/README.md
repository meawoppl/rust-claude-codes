# claude-codes

[![Crates.io](https://img.shields.io/crates/v/claude-codes.svg)](https://crates.io/crates/claude-codes)
[![Documentation](https://docs.rs/claude-codes/badge.svg)](https://docs.rs/claude-codes)
[![CI](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/claude-codes.svg)](../LICENSE)
[![Downloads](https://img.shields.io/crates/d/claude-codes.svg)](https://crates.io/crates/claude-codes)

A typed Rust interface for the [Claude Code](https://docs.anthropic.com/en/docs/claude-code) JSON protocol.

Part of the [rust-code-agent-sdks](https://github.com/meawoppl/rust-code-agent-sdks) workspace.

## Overview

This library provides type-safe bindings for communicating with the Claude CLI via its JSON Lines protocol. It handles message serialization, streaming responses, and session management.

**Note:** The Claude CLI protocol is unstable and may change between versions. This crate tracks protocol changes and will warn if you're using an untested CLI version.

## Installation

### Default (All Features)
```bash
cargo add claude-codes
```

Requires the [Claude CLI](https://docs.anthropic.com/en/docs/claude-code) (`claude` binary) to be installed and available in PATH.

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
claude-codes = { version = "2", default-features = false, features = ["types"] }
```

This gives you access to all typed message structures (`ClaudeInput`, `ClaudeOutput`, `ContentBlock`, etc.) without pulling in tokio or other native-only dependencies. Useful for frontend apps, shared type definitions, or any WASM context needing Claude protocol types.

#### Sync Client Only
```toml
[dependencies]
claude-codes = { version = "2", default-features = false, features = ["sync-client"] }
```

#### Async Client Only
```toml
[dependencies]
claude-codes = { version = "2", default-features = false, features = ["async-client"] }
```

## Usage

### Async Client

```rust
use claude_codes::AsyncClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = AsyncClient::with_defaults().await?;

    let mut stream = client.query_stream("What is 2 + 2?").await?;

    while let Some(response) = stream.next().await {
        match response {
            Ok(output) => println!("Got: {}", output.message_type()),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    Ok(())
}
```

### Sync Client

```rust
use claude_codes::{SyncClient, ClaudeInput};
use uuid::Uuid;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = SyncClient::with_defaults()?;

    let input = ClaudeInput::user_message("What is 2 + 2?", Uuid::new_v4());
    let responses = client.query(input)?;

    for response in responses {
        println!("Got: {}", response.message_type());
    }

    Ok(())
}
```

### Sending Images

```rust
use claude_codes::{AsyncClient, ClaudeInput};
use base64::{engine::general_purpose::STANDARD, Engine};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = AsyncClient::with_defaults().await?;

    let image_data = std::fs::read("diagram.png")?;
    let base64_image = STANDARD.encode(&image_data);

    let input = ClaudeInput::user_message_with_image(
        base64_image,
        "image/png".to_string(),
        Some("What's in this image?".to_string()),
        uuid::Uuid::new_v4(),
    )?;

    client.send(&input).await?;

    Ok(())
}
```

### Raw Protocol Access

Use `RawAsyncClient` when the caller owns protocol interpretation and only
needs newline framing. Neither method decodes JSON.

```rust,no_run
use claude_codes::{ClaudeCliBuilder, RawAsyncClient};

# async fn example() -> claude_codes::Result<()> {
let builder = ClaudeCliBuilder::new();
let mut client = RawAsyncClient::start_with(builder).await?;
let input = claude_codes::ClaudeInput::user_message_without_session("Hello");
client.send(&input).await?;
let raw_line = client.next_line().await?;
# Ok(())
# }
```

Typed protocol parsing remains available separately:

```rust
use claude_codes::{Protocol, ClaudeOutput};

let json_line = r#"{"type":"assistant","message":{...}}"#;
let output: ClaudeOutput = Protocol::deserialize(json_line)?;

let serialized = Protocol::serialize(&output)?;
```

## Compatibility

## Login tooling (`auth` feature)

The CLI's login flows are interactive Ink TUIs (they hang on a pipe), so the
`auth` module drives them under a pseudo-terminal:

```rust,ignore
use claude_codes::auth::{auth_status, LoginFlow, LoginMode};

let mut flow = LoginFlow::start(LoginMode::SetupToken)?;
let url = flow.auth_url(Duration::from_secs(30))?;   // show to the user
// … user authorizes in a browser, brings back a code …
let outcome = flow.submit_code_and_wait(&code, Duration::from_secs(90))?;
// outcome.token = Some("sk-ant-oat01-…") — recovered via screen text,
// OSC 52 clipboard escapes, or the credentials-file watch.
```

Rejected codes keep the flow alive: call `retry_new_url()` for a fresh
authorize URL (the CLI rotates the PKCE challenge; the old code is dead).
Every failure self-describes — timeouts and child exits carry a channel
line naming what each detection source saw. `auth_status()` types
`claude auth status --json` (email, org, plan). Enable with
`features = ["auth"]`.

**Tested against:** Claude CLI 2.1.263

The crate version tracks the Claude CLI version. If you're using a different CLI version, please report whether it works at:
https://github.com/meawoppl/rust-code-agent-sdks/issues

## License

Apache-2.0. See [LICENSE](../LICENSE).
