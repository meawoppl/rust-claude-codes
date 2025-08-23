# claude-codes

A tightly typed Rust interface for the Claude Code JSON protocol.

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