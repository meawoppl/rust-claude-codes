# rust-code-agent-sdks

Typed Rust interfaces for AI code agent CLI protocols.

This workspace provides independent crates for interacting with [Claude Code](https://docs.anthropic.com/en/docs/claude-code), [OpenAI Codex](https://github.com/openai/codex), [opencode](https://opencode.ai), [Meta Muse Code](https://dev.meta.ai/docs), and [Google Antigravity](https://antigravity.google) via their streaming protocols (JSON/JSONL over stdio, HTTP + SSE, or protobuf-JSON over a WebSocket).

## Crates

| Crate | Version | Docs | CI | WASM |
|-------|---------|------|----|------|
| [`claude-codes`](./claude-codes/) | [![Crates.io](https://img.shields.io/crates/v/claude-codes.svg)](https://crates.io/crates/claude-codes) | [![docs.rs](https://docs.rs/claude-codes/badge.svg)](https://docs.rs/claude-codes) | [![CI](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml) | [![Feature Matrix](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml) |
| [`codex-codes`](./codex-codes/) | [![Crates.io](https://img.shields.io/crates/v/codex-codes.svg)](https://crates.io/crates/codex-codes) | [![docs.rs](https://docs.rs/codex-codes/badge.svg)](https://docs.rs/codex-codes) | [![CI](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml) | [![Feature Matrix](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml) |
| [`opencode-codes`](./opencode-codes/) | [![Crates.io](https://img.shields.io/crates/v/opencode-codes.svg)](https://crates.io/crates/opencode-codes) | [![docs.rs](https://docs.rs/opencode-codes/badge.svg)](https://docs.rs/opencode-codes) | [![CI](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml) | [![Feature Matrix](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml) |
| [`muse-codes`](./muse-codes/) | [![Crates.io](https://img.shields.io/crates/v/muse-codes.svg)](https://crates.io/crates/muse-codes) | [![docs.rs](https://docs.rs/muse-codes/badge.svg)](https://docs.rs/muse-codes) | [![CI](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml) | [![Feature Matrix](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml) |
| [`antigravity-codes`](./antigravity-codes/) | [![Crates.io](https://img.shields.io/crates/v/antigravity-codes.svg)](https://crates.io/crates/antigravity-codes) | [![docs.rs](https://docs.rs/antigravity-codes/badge.svg)](https://docs.rs/antigravity-codes) | [![CI](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/ci.yml) | [![Feature Matrix](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml/badge.svg)](https://github.com/meawoppl/rust-code-agent-sdks/actions/workflows/feature-matrix.yml) |

## Versioning

Each crate's version means **tested against**: the version names the newest
CLI release the crate's live integration suite has actually passed against —
never a target, never an aspiration. Installed CLIs typically release faster
than the wire changes, so the running binary may be newer than the pin; the
nightly drift checks watch the wire and the pin moves forward only when it
actually changes.

**Crate-side patch offsets.** When the crate needs a release of its own
between CLI releases (new helpers, docs), the patch number advances past the
tested CLI's and the bullet below spells the arithmetic out as
`tested + N crate-side`. The tested pins themselves (`TESTED_VERSION`,
the `Tested against:` lines) **never move on a crate-side release** — they
change only when the live suite passes against a newer CLI. Offsets can
temporarily occupy a version number the CLI hasn't shipped yet; when the CLI
catches up, the next tested release jumps to or past the CLI's number.
(Semver suffixes can't express this on crates.io: `-pre` versions sort
*before* the base and are skipped by default caret requirements, and build
metadata can't distinguish published versions — so explicit offset notes are
the least-bad scheme.)

- **`claude-codes`** — currently `claude-codes 2.1.234` (tested CLI + 2 crate-side patches), tested against Claude CLI `2.1.232`.
- **`codex-codes`** — currently `codex-codes 0.147.1` (tested CLI + 1 crate-side patch), tested against Codex CLI `0.147.0`.
- **`opencode-codes`** — currently `opencode-codes 1.18.19` (tested CLI + 1 crate-side patch), tested against opencode `1.18.18`.
- **`muse-codes`** — currently `muse-codes 0.1.8` (tested CLI + 8 crate-side patches), tested against Muse Code `0.1.0` (build `0.1.0-R708.1`; Meta has shipped no newer CLI).
- **`antigravity-codes`** — currently `antigravity-codes 0.1.10`, tested against google-antigravity `0.1.10` (the wheel its bundled harness was generated from).

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
| `auth` | PTY-driven login tooling (`LoginFlow`, `auth_status`) | No |

`types`, `sync-client`, and `async-client` are enabled by default (`auth` is
opt-in). For WASM or type-sharing use cases:

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

## Login & Auth Tooling

Each crate ships helpers for authenticating its CLI programmatically — the
mechanisms differ with each vendor's surface:

| | claude-codes (`auth` feature) | codex-codes | muse-codes |
|---|---|---|---|
| Status read | `auth_status()` (typed `claude auth status --json`: email, org, plan) | `account_read` (protocol) or `auth_local::auth_status_local()` (cheap file read: email + plan, best-effort) | `auth::credentials_present()` + typed `AuthFile` (presence + provider only) |
| Login flow | `LoginFlow` drives the Ink TUI under a PTY: `auth_url()` → user pastes code → `submit_code_and_wait()`; rejected codes retry via `retry_new_url()` (new PKCE) | Protocol-native: `account_login_start` (api-key / browser URL / device code), completion via the `account/login/completed` notification | `DeviceLoginFlow` wraps the plain-stdout device-code flow; `auth_set()` saves an API key over stdin |
| Cancellation | Drop kills the PTY child | `account_login_cancel` | `cancel()` / drop |

All presentables and outcomes are serde-shaped for relay surfaces, flows are
parkable handles across round-trips, and waits take caller-supplied
timeouts. opencode needs no CLI auth for the server endpoints this workspace
wraps.

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
  antigravity-codes/     # Antigravity localharness protobuf-JSON bindings
    src/                 # Types, handshake codec, process launcher, WebSocket client
    tests/               # Corpus, integration tests, descriptor snapshots
    test_cases/          # Captured and synthetic wire frames
    examples/            # stream_chat, custom_tool, capture_frames
```

[Installing and updating the wrapped CLI tools](./docs/installing-the-clis.md)
covers install/update/auth procedures for all four vendors.

See each crate's README for detailed usage:
- [claude-codes README](./claude-codes/README.md)
- [codex-codes README](./codex-codes/README.md)
- [opencode-codes README](./opencode-codes/README.md)
- [muse-codes README](./muse-codes/README.md)
- [antigravity-codes README](./antigravity-codes/README.md)

## License

Apache-2.0. See [LICENSE](LICENSE).
