# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.4] - 2026-08-06

### Added

- **`MuseExecBuilder::session_id`** — run under a caller-supplied
  `--session-id`, the basis of multi-turn continuity: each turn is its own
  process, and reusing the id continues the session. The id is adopted
  verbatim as `stream.id` on every record, which is also what makes the
  `(stream.id, id)` identity composite trustworthy.
- **`ExecRun::pid`** — the child's OS process id, for supervisors that
  signal the process group directly.
- **Live tests pinning two measured behaviors** that consumers must not get
  wrong: `sequence` restarts per turn (never key across turns on it), record
  ids do **not** repeat within a session but **do** repeat across sessions
  (UUID-shaped counters — only `(stream.id, id)` is unique), and a run
  SIGKILLed mid-flight leaves the session store usable for the next turn.

## [0.1.3] - 2026-08-05

### Changed

- **`DeviceCode` now derives Serialize/Deserialize** — the verification
  URL + code presentable is relay-shaped for remote UIs. No behavior
  change.

## [0.1.2] - 2026-08-05

Extends the corpus and models with the **live-provider vocabulary**,
captured from real Muse Spark runs (basic, tool-use, and multi-subagent)
now that credentials exist. Echo remains the CI capture path; the meta
captures are committed corpus.

### Added

- **`run.model.configured`** (`ModelConfigured`): resolved model /
  display label / profile / provider / source for the run.
- **`tool.result`** (`ToolResult`): per-invocation outcome — `call_id`,
  result `text`, open-shaped `correlation_facts` (`{outcome, tool_name}`
  observed) and `edit_facts` for file-editing tools.
- **Four new `task.lifecycle` event kinds**: `status` (message + faceted
  `details`, e.g. model stream attempts), `output` (streamed chunk),
  `cancelled` (`reason`), `rejected` (`reason`); `started` gains an
  optional `span_id` (live providers attach one).
- Three live-capture corpus files (`meta_basic`, `meta_tool_use`,
  `meta_subagents`) — 20 payload types now round-trip in tests.

## [0.1.1] - 2026-08-05

### Added

- **Login support tooling** (`auth` module, feature `async-client`). Muse's
  auth surface needs no PTY: `auth_set` wraps
  `muse auth set --api-key-stdin` (secret over stdin, never argv);
  `DeviceLoginFlow` wraps the plain-stdout `muse login` OAuth device-code
  flow (`device_code()` extracts the verification URL + code from captured
  wire shapes, `wait_approved()`/`cancel()` manage the poll);
  `logout()`; `credentials_present()` (checks `META_API_KEY` then the
  saved file's providers map — `muse logout` empties the map but keeps
  the file); typed `AuthFile`/`ProviderCredential` models of
  `~/.config/muse/auth.json` (schema_version 1, observed). Live-tested:
  real device flow yields URL+code then cancels; auth_set/logout
  round-trip in a sandboxed HOME.

## [0.1.0] - 2026-08-05

Initial release, tracking Muse Code 0.1.0 (`0.1.0-R708.1`) — released by
Meta today; this crate models its headless `muse exec --json` stream from
captured real output (Muse publishes no schema or SDK).

### Added

- **Journal envelope model** (`MuseRecord`): `schema_version`, UUIDv7-style
  `id`, `stream` refs (session/run/task), strictly-increasing `sequence`,
  microsecond `recorded_at`, `record_type`
  (reconciliation/event/status), `durability` (durable/ephemeral),
  `causation_id`, and a raw payload lifted on demand — byte-faithful
  round-trips by construction.
- **Typed payloads** for the full observed headless vocabulary (14 types):
  command intake, session/run linking, user turn input, run lifecycle and
  output deltas, `run.terminal.*`, task stream linking, and the
  `task.lifecycle.*` state machine (`proposed → accepted → started →
  scheduled → side_effect_intent → completed | failed`) with a kind-tagged
  event enum. Unobserved payload types (approvals, edits, subagent
  lifecycle under a live provider) surface as `MusePayload::Unknown`
  rather than failing.
- **`ExecRun` / `MuseExecBuilder`** (feature `async-client`): spawn
  headless runs (provider, preset, model, reasoning effort, base URL),
  stream typed records, `wait_terminal` with per-record callback, stderr
  collected for error context.
- **Committed capture corpus** (`test_cases/`) from credential-free
  echo-provider runs, with corpus tests enforcing parse/type/round-trip
  and envelope invariants; live integration test drives a real run
  end-to-end.
- **Drift automation**: nightly workflow installs the current Muse Code
  CLI, re-captures echo runs, fingerprints `payload_type → field set`
  (plus per-lifecycle-event sub-fingerprints), and files an issue on any
  divergence from `tests/schemas/muse_stream_fingerprint.txt`.
- **`types` feature is `wasm32-unknown-unknown` compatible**, matching the
  workspace's WASM division.
