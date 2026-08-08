# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.18.14] - 2026-08-07

### Changed

- Track opencode CLI 1.18.14 (nightly OpenAPI drift check green across
  the span — no wire changes; version pin moved forward).
- The version-drift assertion now compares against `CARGO_PKG_VERSION`
  instead of a second hardcoded copy of the version, so the pin cannot
  fall out of sync with `Cargo.toml`.
- The in-crate live SSE test honors `OPENCODE_BASE_URL` (matching the
  integration tests), so harnesses running it against a managed server
  on a random port work.

### Added

- **`OpencodeClient::fork_session`** — wraps `POST /session/{sessionID}/fork`
  (operation `session.fork`), branching a session's whole history into a new
  server-assigned session. Verified against a live 1.18.10 server. (#227)

### Changed

- On Windows, managed `opencode serve` launches now use `CREATE_NO_WINDOW` to
  avoid opening console windows.

## [1.18.5] - 2026-07-25

Initial release. Wraps the opencode local HTTP + Server-Sent Events server,
mirroring the sibling crates' conventions. The version tracks the opencode
release train.

### Added

- `types` / `async-client` / `server` / `integration-tests` feature tiers
  (`types` builds on `wasm32`).
- `error` module with the `Error` enum and `Result` alias; transport and SSE
  variants gated behind `async-client`, the server-lifecycle variant behind
  `server`.
- `protocol_generated` module: serde wire types generated from the opencode
  OpenAPI 3.1 schema.
- `http` module: `HttpTransport` REST client with base-URL normalization, HTTP
  Basic auth, directory/workspace scoping, and typed URL builders for the six
  hand-wrapped endpoints.
- `sse` module: self-reconnecting `GET /event` reader (`EventStream`) with a
  configurable exponential-backoff `RetryConfig`.
- `client_async` module: high-level `OpencodeClient` covering session create,
  `prompt_async`, message listing, abort, and permission reply, plus a builder
  and a raw request escape hatch.
- `server` module: managed `opencode serve` launcher that picks a free port,
  waits for health, and tears the process group down on drop.
- OpenAPI 3.1 snapshot of opencode 1.18.5 at
  `tests/schemas/opencode_openapi.json` as the drift-check ground truth.

### Changed

- Shared dependencies and lint policy moved to the workspace root: `serde`,
  `serde_json`, `thiserror`, `tokio`, and dev `jsonschema` are now
  `{ workspace = true }`, and the crate opts into `[workspace.lints]`
  (`unsafe_code = "deny"`) with a scoped `#[allow]` on the vetted
  process-group signal FFI in `server.rs`. No dependency version changes.
