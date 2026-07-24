# rust-code-agent-sdks

Typed Rust interfaces for AI code agent CLI protocols.

This workspace provides two independent crates for interacting with [Claude Code](https://docs.anthropic.com/en/docs/claude-code) and [OpenAI Codex](https://github.com/openai/codex) via their JSON/JSONL streaming protocols.

## Crates

| Crate | Version | Docs | CI | WASM |
|-------|---------|------|----|------|
| [`claude-codes`](./claude-codes/) | [![Crates.io](https://img.shields.io/crates/v/claude-codes.svg)](https://crates.io/crates/claude-codes) | [![docs.rs](https://docs.rs/claude-codes/badge.svg)](https://docs.rs/claude-codes) | [![CI](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml) | [![Feature Matrix](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml) |
| [`codex-codes`](./codex-codes/) | [![Crates.io](https://img.shields.io/crates/v/codex-codes.svg)](https://crates.io/crates/codex-codes) | [![docs.rs](https://docs.rs/codex-codes/badge.svg)](https://docs.rs/codex-codes) | [![CI](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml) | [![Feature Matrix](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml) |

## Versioning

Each crate's version tracks the CLI it wraps:

- **`claude-codes`** version tracks the Claude CLI it targets and may sit slightly ahead of the CLI it was last integration-tested against. Currently `claude-codes 2.1.164`, tested against Claude CLI `2.1.219`.
- **`codex-codes`** version tracks the Codex CLI it has been tested against, sitting a small offset behind while the bindings stabilize. Currently `codex-codes 0.143.5`, tested against Codex CLI `0.143.0`.

Both crates will warn (or fail gracefully) if the installed CLI version diverges from the tested version.

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

## Testing Approach

Both crates share the same testing philosophy:

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
```

See each crate's README for detailed usage:
- [claude-codes README](./claude-codes/README.md)
- [codex-codes README](./codex-codes/README.md)

## License

Apache-2.0. See [LICENSE](LICENSE).
