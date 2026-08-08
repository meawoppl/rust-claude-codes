# rust-code-agent-sdks

Typed Rust interfaces for AI code agent CLI protocols.

This workspace provides independent crates for interacting with [Claude Code](https://docs.anthropic.com/en/docs/claude-code), [OpenAI Codex](https://github.com/openai/codex), [opencode](https://opencode.ai), and [Google Antigravity](https://antigravity.google) via their streaming protocols (JSON/JSONL over stdio, HTTP + SSE, or protobuf-JSON over a WebSocket).

## Crates

| Crate | Version | Docs | CI | WASM |
|-------|---------|------|----|------|
| [`claude-codes`](./claude-codes/) | [![Crates.io](https://img.shields.io/crates/v/claude-codes.svg)](https://crates.io/crates/claude-codes) | [![docs.rs](https://docs.rs/claude-codes/badge.svg)](https://docs.rs/claude-codes) | [![CI](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml) | [![Feature Matrix](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml) |
| [`codex-codes`](./codex-codes/) | [![Crates.io](https://img.shields.io/crates/v/codex-codes.svg)](https://crates.io/crates/codex-codes) | [![docs.rs](https://docs.rs/codex-codes/badge.svg)](https://docs.rs/codex-codes) | [![CI](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml) | [![Feature Matrix](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml) |
| [`opencode-codes`](./opencode-codes/) | [![Crates.io](https://img.shields.io/crates/v/opencode-codes.svg)](https://crates.io/crates/opencode-codes) | [![docs.rs](https://docs.rs/opencode-codes/badge.svg)](https://docs.rs/opencode-codes) | [![CI](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml) | [![Feature Matrix](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml) |
| [`antigravity-codes`](./antigravity-codes/) | [![Crates.io](https://img.shields.io/crates/v/antigravity-codes.svg)](https://crates.io/crates/antigravity-codes) | [![docs.rs](https://docs.rs/antigravity-codes/badge.svg)](https://docs.rs/antigravity-codes) | [![CI](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml) | [![Feature Matrix](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml) |

## Versioning

Each crate's version tracks the CLI it wraps:

- **`claude-codes`** version tracks the Claude CLI it targets and may sit slightly ahead of the CLI it was last integration-tested against. Currently `claude-codes 2.1.166`, tested against Claude CLI `2.1.220`.
- **`codex-codes`** version tracks the Codex CLI it has been tested against, sitting a small offset behind while the bindings stabilize. Currently `codex-codes 0.146.0`, tested against Codex CLI `0.146.0`.
- **`opencode-codes`** version tracks the opencode release train it wraps. Currently `opencode-codes 1.18.5`, tested against opencode `1.18.5`.
- **`antigravity-codes`** version tracks the `google-antigravity` wheel whose bundled harness it was generated from. Currently `antigravity-codes 0.1.10`, tested against google-antigravity `0.1.10`.

`claude-codes` and `codex-codes` warn (or fail gracefully) when the installed
CLI version diverges from the tested version. `opencode-codes` tracks the
opencode release train by version but ships no runtime version-divergence check.
`antigravity-codes` *cannot* check: the harness takes no arguments and reports
no version of its own, so it absorbs skew at the type level instead — unknown
enum values and unknown `oneof` arms decode rather than fail.

## Feature Flags

### claude-codes

`claude-codes` is structured into three feature flags to control dependency weight:

| Feature | Description | WASM-compatible |
|---------|-------------|-----------------|
| `types` | Core message types and protocol structs only | Yes |
| `sync-client` | Synchronous client with blocking I/O | No |
| `async-client` | Asynchronous client using tokio | No |

All features are enabled by default. For WASM or type-sharing use cases:

```toml
[dependencies]
claude-codes = { version = "2", default-features = false, features = ["types"] }
```

### codex-codes

`codex-codes` mirrors the same feature flag structure:

| Feature | Description | WASM-compatible |
|---------|-------------|-----------------|
| `types` | Core message types and protocol structs only | Yes |
| `sync-client` | Synchronous client with blocking I/O | No |
| `async-client` | Asynchronous client using tokio | No |

All features are enabled by default. For WASM or type-sharing use cases:

```toml
[dependencies]
codex-codes = { version = "0.142", default-features = false, features = ["types"] }
```

### opencode-codes

`opencode-codes` wraps an HTTP + SSE server rather than a stdio CLI, so its flags differ:

| Feature | Description | WASM-compatible |
|---------|-------------|-----------------|
| `types` | Core protocol types only (serde) | Yes |
| `async-client` | Async HTTP/SSE client using reqwest + tokio | No |
| `server` | Managed `opencode serve` launcher (picks a free port) | No |

`default = ["types", "async-client"]` (there is no sync client). For WASM or type-sharing use cases:

```toml
[dependencies]
opencode-codes = { version = "1.18", default-features = false, features = ["types"] }
```

### antigravity-codes

`antigravity-codes` wraps a Go binary that bootstraps over stdio and then serves a loopback WebSocket, so its flags differ again:

| Feature | Description | WASM-compatible |
|---------|-------------|-----------------|
| `types` | Wire types and the stdio handshake codec only (serde) | Yes |
| `async-client` | Async WebSocket client using tokio | No |
| `integration-tests` | Enables tests that require a real harness binary | No |

`default = ["types", "async-client"]` (there is no sync client — the protocol is
bidirectional, with the harness making requests of the client mid-turn). For
WASM or type-sharing use cases:

```toml
[dependencies]
antigravity-codes = { version = "0.1", default-features = false, features = ["types"] }
```

Note that the `localharness` binary is distributed only inside the
`google-antigravity` wheels on PyPI; see the
[crate README](./antigravity-codes/README.md) for how to obtain it.

## Testing Approach

The crates share the same testing philosophy:

1. **Unit tests** validate serde round-tripping for every type variant against hand-crafted JSON.
2. **Integration tests** deserialize real JSONL captures from actual CLI sessions. These captures live in each crate's `test_cases/` directory and are checked into the repo, so deserialization is validated against real-world protocol output.
3. **CI matrix** tests each feature combination independently, including WASM builds via `wasm32-unknown-unknown`, clippy, rustfmt, and MSRV (1.85).

To run all tests locally:

```bash
cargo test --workspace
```

## Workspace Structure

```
rust-code-agent-sdks/
  claude-codes/          # Claude Code CLI protocol bindings
    src/                 # Types, sync/async clients, protocol handling
    tests/               # Deserialization + integration tests
    test_cases/          # Real CLI captures and failure cases
    examples/            # async_client, sync_client, basic_repl
  codex-codes/           # Codex CLI protocol bindings
    src/                 # Types, sync/async clients, CLI builder
    tests/               # Integration tests
    test_cases/          # Real CLI captures
    examples/            # async_client, sync_client, basic_repl
  opencode-codes/        # opencode HTTP + SSE server bindings
    src/                 # Types, async client, HTTP/SSE transport, server launcher
    tests/               # Drift checks and schema snapshot
  antigravity-codes/     # Antigravity localharness protobuf-JSON bindings
    src/                 # Types, handshake codec, process launcher, WebSocket client
    tests/               # Corpus, integration tests, descriptor snapshots
    test_cases/          # Captured and synthetic wire frames
    examples/            # stream_chat, custom_tool, capture_frames
```

See each crate's README for detailed usage:
- [claude-codes README](./claude-codes/README.md)
- [codex-codes README](./codex-codes/README.md)
- [opencode-codes README](./opencode-codes/README.md)
- [antigravity-codes README](./antigravity-codes/README.md)

## License

Apache-2.0. See [LICENSE](LICENSE).
