# claude-codes

A tightly typed Rust interface for the Claude Code JSON protocol.

## ⚠️ Important Notice: Unstable Interface

This crate provides bindings for the **Claude Code CLI**, which is currently an **unstable and rapidly evolving interface**. The underlying protocol and message formats may change without notice between Claude CLI versions.

## Features

- Type-safe message encoding/decoding
- JSON Lines protocol support
- Async and sync I/O support
- Comprehensive error handling
- Stream processing utilities
- Automatic Claude CLI version compatibility checking
- Test-driven protocol discovery for handling new message types

### Compatibility Reporting

- **Current tested version**: Claude CLI 1.0.89
- **Compatibility reports needed**: If you're using this crate with a different version of Claude CLI (whether it works or fails), please report your experience at:
  
  **https://github.com/meawoppl/rust-claude-codes/pulls**

When creating a compatibility report, please include:
- Your Claude CLI version (run `claude --version`)
- Whether the crate worked correctly or what errors you encountered
- Any message types that failed to deserialize

The crate will automatically warn you if you're using a newer Claude CLI version than what has been tested. You can work around version checks if needed (see documentation), but please report your results to help the community!

## Installation

```bash
cargo add claude-codes
```

## Usage

### Async Client

```rust
use claude_codes::AsyncClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client (checks Claude CLI compatibility)
    let mut client = AsyncClient::with_defaults().await?;
    
    // Send a query and stream responses
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client (checks Claude CLI compatibility)
    let mut client = SyncClient::with_defaults()?;
    
    // Send a query
    let input = ClaudeInput::user_message("What is 2 + 2?", "session-1");
    let responses = client.query(input)?;
    
    for response in responses {
        println!("Got: {}", response.message_type());
    }
    
    Ok(())
}
```

### Working with Raw Protocol

```rust
use claude_codes::{Protocol, ClaudeOutput};

// Deserialize a JSON Lines message
let json_line = r#"{"type":"assistant","message":{...}}"#;
let output: ClaudeOutput = Protocol::deserialize(json_line)?;

// Serialize for sending
let serialized = Protocol::serialize(&output)?;
```

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0).

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be licensed under the Apache-2.0 license,
without any additional terms or conditions.