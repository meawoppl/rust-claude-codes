# claude-codes

A tightly typed Rust interface for the Claude Code JSON protocol.

## Features

- Type-safe message encoding/decoding
- JSON Lines protocol support
- Async and sync I/O support
- Comprehensive error handling
- Stream processing utilities
- Automatic Claude CLI version compatibility checking
- Test-driven protocol discovery for handling new message types

## ⚠️ Important Notice: Unstable Interface

This crate provides bindings for the **Claude Code CLI**, which is currently an **unstable and rapidly evolving interface**. The underlying protocol and message formats may change without notice between Claude CLI versions.

### Compatibility Reporting

- **Current tested version**: Claude CLI 1.0.89
- **Compatibility reports needed**: If you're using this crate with a different version of Claude CLI (whether it works or fails), please report your experience at:
  
  **https://github.com/meawoppl/rust-claude-codes/pulls**

When creating a compatibility report, please include:
- Your Claude CLI version (run `claude --version`)
- Whether the crate worked correctly or what errors you encountered
- Any message types that failed to deserialize

The crate will automatically warn you if you're using a newer Claude CLI version than what has been tested. You can work around version checks if needed (see documentation), but please report your results to help the community!

## Development Workflow

This project uses a strict PR-based workflow with automated quality checks:

### Branch Protection & Git Hooks

1. **No direct commits to main** - All changes must go through feature branches and PRs
2. **Automated pre-commit checks** - Run `./setup_hooks.sh` to install local git hooks that enforce:
   - Branch protection (prevents commits to main)
   - Code formatting (`cargo fmt`)
   - Linting (`cargo clippy`)
   - JSON test case formatting
   - All tests passing

### Getting Started

```bash
# Clone the repository
git clone https://github.com/meawoppl/rust-claude-codes
cd rust-claude-codes

# Install git hooks (required for all contributors)
./setup_hooks.sh

# Create a feature branch for your work
git checkout -b feature/your-feature-name

# Make your changes, then commit
git add .
git commit -m "Your descriptive commit message"

# Push to GitHub and create a PR
git push origin feature/your-feature-name
```

### Test-Driven Protocol Development

This crate uses a unique test-driven approach for protocol development:

1. **Run the test client**: `cargo run --bin claude-test`
2. **Failed deserializations are automatically saved** to `test_cases/failed_deserializations/`
3. **Format test cases**: `./format_test_cases.sh`
4. **Run tests to see what needs implementing**: `cargo test deserialization`
5. **Add missing message types** to the `ClaudeOutput` enum
6. **Tests turn green** as the protocol is implemented

### CI/CD Requirements

All PRs must pass the following GitHub Actions checks:

- ✅ Code formatting (`cargo fmt --all -- --check`)
- ✅ Clippy linting (`cargo clippy --all-targets --all-features -- -D warnings`)
- ✅ All tests passing (`cargo test --all-features`)
- ✅ JSON test cases properly formatted
- ✅ Documentation builds (`cargo doc --no-deps`)
- ✅ MSRV compatibility (Rust 1.85+)

## Installation

```toml
[dependencies]
claude-codes = "0.0.5"
```

## Usage

### Async Client (Recommended)

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