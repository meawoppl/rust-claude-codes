# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.20.0] - 2026-08-04

Initial release, tracking hermes-agent 0.20.0.

### Added

- **Generated ACP protocol types** (135 definitions) from the schema
  variant hermes-agent actually speaks: `schema.unstable.json` /
  `meta.unstable.json` at zed-industries/agent-client-protocol `v0.11.2`,
  the tag the pinned Python SDK (`agent-client-protocol==0.9.0`) generates
  from. Includes the unstable method family (`session/fork`, `resume`,
  `close`, `set_model`) hermes exposes. Codegen
  (`scripts/codegen_acp.py`) is deterministic and CI-enforced; every
  method↔type mapping is validated against the snapshot at generation
  time.
- **JSON-RPC 2.0 layer** (`jsonrpc` module) with string/number/null
  request ids and shape-based frame classification.
- **`AsyncClient`** (feature `async-client`): spawns `hermes acp`,
  performs the `initialize` handshake, typed helpers for all 13 agent
  methods, `session_prompt_with` for mid-turn streaming + answering the
  agent's bidirectional requests (`session/request_permission`, `fs/*`,
  `terminal/*`), stderr drained through the `log` crate.
- **Hermes `_meta` extensions**: `HermesMeta` / `SessionProvenance`
  lifted from any type's `_meta.hermes` member, plus the `hermes-setup`
  auth-method id.
- **Golden-corpus tests**: 37 fixtures vendored from the ACP Python SDK's
  test suite (Apache-2.0, exact pinned version) — deserialize →
  reserialize → byte-compare round-trips, including every `session/update`
  variant shape.
- **Drift automation**: nightly workflow re-derives the full provenance
  chain (hermes pin → PyPI sdist schema ref → zed schema files) and files
  an issue on any divergence; a companion job fails CI if the committed
  bindings don't match a fresh codegen run.
- **`types` feature is `wasm32-unknown-unknown` compatible**, matching
  the workspace's WASM division.
