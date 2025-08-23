# claude-codes

A tightly typed Rust interface for the Claude Code JSON protocol.

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

## Features

- Type-safe message encoding/decoding
- JSON Lines protocol support
- Async and sync I/O support
- Comprehensive error handling
- Stream processing utilities

## Installation

```toml
[dependencies]
claude-codes = "0.0.1"
```

## Usage

```rust
use claude_codes::{Protocol, Request, Response};

// Serialize a request
let request = Request {
    // ... request fields
};
let json_line = Protocol::serialize(&request)?;

// Deserialize a response
let response: Response = Protocol::deserialize(&json_line)?;
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.