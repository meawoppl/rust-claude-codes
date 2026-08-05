# rust-code-agent-sdks

Typed Rust interfaces for AI code agent CLI protocols.

This workspace provides independent crates for interacting with [Claude Code](https://docs.anthropic.com/en/docs/claude-code), [OpenAI Codex](https://github.com/openai/codex), [opencode](https://opencode.ai), and [Meta Muse Code](https://dev.meta.ai/docs) via their streaming protocols (JSON/JSONL over stdio, or HTTP + SSE).

## Crates

| Crate | Version | Docs | CI | WASM |
|-------|---------|------|----|------|
| [`claude-codes`](./claude-codes/) | [![Crates.io](https://img.shields.io/crates/v/claude-codes.svg)](https://crates.io/crates/claude-codes) | [![docs.rs](https://docs.rs/claude-codes/badge.svg)](https://docs.rs/claude-codes) | [![CI](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml) | [![Feature Matrix](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml) |
| [`codex-codes`](./codex-codes/) | [![Crates.io](https://img.shields.io/crates/v/codex-codes.svg)](https://crates.io/crates/codex-codes) | [![docs.rs](https://docs.rs/codex-codes/badge.svg)](https://docs.rs/codex-codes) | [![CI](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml) | [![Feature Matrix](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml) |
| [`opencode-codes`](./opencode-codes/) | [![Crates.io](https://img.shields.io/crates/v/opencode-codes.svg)](https://crates.io/crates/opencode-codes) | [![docs.rs](https://docs.rs/opencode-codes/badge.svg)](https://docs.rs/opencode-codes) | [![CI](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml) | [![Feature Matrix](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml) |
| [`muse-codes`](./muse-codes/) | [![Crates.io](https://img.shields.io/crates/v/muse-codes.svg)](https://crates.io/crates/muse-codes) | [![docs.rs](https://docs.rs/muse-codes/badge.svg)](https://docs.rs/muse-codes) | [![CI](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml) | [![Feature Matrix](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml) |

## Versioning

Each crate's version tracks the CLI it wraps:

- **`claude-codes`** version tracks the Claude CLI it targets and may sit slightly ahead of the CLI it was last integration-tested against. Currently `claude-codes 2.1.223`, tested against Claude CLI `2.1.222`.
- **`codex-codes`** version tracks the Codex CLI it has been tested against, sitting a small offset behind while the bindings stabilize. Currently `codex-codes 0.146.3`, tested against Codex CLI `0.146.0`.
- **`opencode-codes`** version tracks the opencode release train it wraps. Currently `opencode-codes 1.18.5`, tested against opencode `1.18.5`.
- **`muse-codes`** version tracks the Muse Code release its stream captures were taken from, with patch offsets for crate-side additions. Currently `muse-codes 0.1.3`, tested against Muse Code `0.1.0` (build `0.1.0-R708.1`).

`claude-codes` and `codex-codes` warn (or fail gracefully) when the installed
CLI version diverges from the tested version. `opencode-codes` tracks the
opencode release train by version but ships no runtime version-divergence check.

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

### muse-codes

`muse-codes` wraps Muse Code's headless JSONL event journal (`muse exec --json`):

| Feature | Description | WASM-compatible |
|---------|-------------|-----------------|
| `types` | Journal envelope + payload models (serde only) | Yes |
| `async-client` | Tokio client spawning `muse exec --json` | No |

`default = ["types", "async-client"]`. For WASM or type-sharing use cases:

```toml
[dependencies]
muse-codes = { version = "0.1", default-features = false, features = ["types"] }
```

## Session Forking

All three runtimes can fork a session/thread — branch an existing history
into a new one and diverge without touching the source — but the semantics
differ, and consumers should design for the asymmetry:

| | claude-codes | codex-codes | opencode-codes |
|---|---|---|---|
| Mechanism | `ClaudeCliBuilder::fork_from(src)` → `--resume <src> --fork-session --session-id <new>` | `AsyncClient::thread_fork(ThreadForkParams)` (`thread/fork`) | `fork_session(id)` (`POST /session/{id}/fork`) |
| Fork point | **Whole history only** — the CLI's headless surface exposes no at-point cut | **Any turn** — `last_turn_id` cuts the source at that turn | **Whole history only** — no at-point cut in the 1.18.x spec |
| New identity | Caller-supplied or generated UUID, known **before** spawn | Server-assigned thread id, returned in the response | Server-assigned `ses…` id, returned in the response |
| Per-fork overrides | Anything expressible as CLI flags (model, cwd, tools, …) | `model`, `cwd`, `sandbox`, `approval_policy`, `ephemeral`, … | `directory` / `workspace` targeting only |
| Precondition | Source session must exist on disk | Source thread needs ≥ 1 persisted turn (else "no rollout found") | None — a fresh session forks fine |

All three are covered by live integration tests
(`test_fork_session_carries_history_under_new_id`,
`test_async_client_thread_fork`, `fork_session_returns_new_session`).

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
  muse-codes/            # Meta Muse Code headless JSONL stream bindings
    src/                 # Journal envelope + payload types, exec client
    test_cases/          # Real CLI captures (echo provider)
    tests/               # Corpus tests + stream fingerprint snapshot
```

See each crate's README for detailed usage:
- [claude-codes README](./claude-codes/README.md)
- [codex-codes README](./codex-codes/README.md)
- [opencode-codes README](./opencode-codes/README.md)
- [muse-codes README](./muse-codes/README.md)

## License

Apache-2.0. See [LICENSE](LICENSE).
