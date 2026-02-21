# claude-codes

[![Crates.io](https://img.shields.io/crates/v/claude-codes.svg)](https://crates.io/crates/claude-codes)
[![Documentation](https://docs.rs/claude-codes/badge.svg)](https://docs.rs/claude-codes)
[![CI](https://github.com/meawoppl/rust-claude-codes/actions/workflows/ci.yml/badge.svg)](https://github.com/meawoppl/rust-claude-codes/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/claude-codes.svg)](LICENSE)
[![Downloads](https://img.shields.io/crates/d/claude-codes.svg)](https://crates.io/crates/claude-codes)

A typed Rust interface for the Claude Code JSON protocol.

## Overview

This library provides type-safe bindings for communicating with the Claude CLI via its JSON Lines protocol. It handles message serialization, streaming responses, and session management.

**Note:** The Claude CLI protocol is unstable and may change between versions. This crate tracks protocol changes and will warn if you're using an untested CLI version.

## Features

- Type-safe message encoding/decoding
- JSON Lines protocol support with streaming
- Async and sync client implementations
- Image support (JPEG, PNG, GIF, WebP) with base64 encoding
- Tool use blocks for Claude's tool capabilities
- OAuth and API key authentication via environment variables
- UUID-based session management

## Installation

### Default Installation (All Features)
```bash
cargo add claude-codes
```

Requires the [Claude CLI](https://docs.anthropic.com/en/docs/claude-code) (`claude` binary) to be installed and available in PATH.

### Feature Flags

- **`types`** - Core message types only (WASM-compatible, minimal dependencies)
- **`sync-client`** - Synchronous client with blocking I/O
- **`async-client`** - Asynchronous client with tokio runtime
- **Default** - All features enabled

#### Types Only
```toml
[dependencies]
claude-codes = { version = "2", default-features = false, features = ["types"] }
```

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

### WASM Support

The `types` feature is fully compatible with `wasm32-unknown-unknown`, making it suitable for sharing Claude message types between native and browser/WASM code:

```toml
[dependencies]
claude-codes = { version = "2", default-features = false, features = ["types"] }
```

This gives you access to all the typed message structures (`ClaudeInput`, `ClaudeOutput`, `ContentBlock`, etc.) without pulling in tokio or other native-only dependencies. Useful for:

- Frontend applications that communicate with a Claude proxy
- Shared type definitions across native backend and WASM frontend
- Any WASM context needing Claude protocol types

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

```rust
use claude_codes::{Protocol, ClaudeOutput};

let json_line = r#"{"type":"assistant","message":{...}}"#;
let output: ClaudeOutput = Protocol::deserialize(json_line)?;

let serialized = Protocol::serialize(&output)?;
```

## Compatibility

**Tested version:** Claude CLI 2.1.3

If you're using a different CLI version, please report whether it works at:
https://github.com/meawoppl/rust-claude-codes/issues

Include:
- Your Claude CLI version (`claude --version`)
- Whether messages serialized/deserialized correctly
- Any errors encountered

## License

Apache-2.0. See [LICENSE](LICENSE).

## Contributing

Contributions welcome. Any contribution submitted for inclusion will be licensed under Apache-2.0.
