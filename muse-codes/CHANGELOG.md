# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2026-08-19

### Changed

- **Tested pin moves to Muse Code 0.2.1 (`0.2.1-R1215.1`)** — surfaced by
  a live drift report from agent-portal (host running 0.2.1 against
  models captured from 0.1.0), the direction nobody was watching: the
  HOST ahead of the SDK. The crate version jumps to match, per the
  version-means-tested convention. Verified live: 12/13 wirecheck checks
  green unchanged (identity counter trap, kind-match correlation,
  envelope invariants all hold on 0.2.1); the echo fingerprint is
  byte-identical.
- **New committed capture `meta_tool_use_0_2_1.jsonl`** pinning the two
  observed 0.2.1 behaviors: `task.lifecycle.failed` in the live
  vocabulary (already typed, never before captured), and
  `correlation_facts` ABSENT on tool results the binary rejects before
  execution (the model's stringified-scalar bug tripping strict serde).
  Known-wire-gaps documents the absence: such results cannot be
  kind-matched and need the running-task fallback.

## [0.1.8] - 2026-08-19

### Added

- **`version::tested_cli_version()` / `tested_cli_build()`** — the
  workspace-uniform accessors over the existing `TESTED_MUSE_VERSION` /
  `TESTED_MUSE_BUILD` pins, so every crate answers the same question the
  same way.

### Changed

- **`ToolResult.correlation_facts` is typed** (`ToolCorrelationFacts
  { tool_name, outcome, extra }`, fixes #310): the two fields consumers
  key on — `tool_name` drives the `tool.<name>`-task attribution match,
  `outcome` classifies the result — are now fields instead of `Value`
  pokes, with `ToolResult::outcome()` / `tool_name()` accessors. Unknown
  keys round-trip through a flattened `extra`, pinned by the corpus
  byte-faithful round-trip tests. Filed by agent-portal to replace its
  `correlation_facts.get("outcome")` poke.

## [0.1.7] - 2026-08-11

### Added

- **`CommandResult` typed binding for `tool.result` bash/command output** (fixes #294) — `ToolResult.text` for the `bash` tool is a JSON string; new `CommandResult { chunk_id, command, description, exit_code, terminal_status, output, original_output_bytes, original_output_tokens, truncated, extra }` plus `ToolResult::is_command_tool()`, `command_result() -> Option<CommandResult>` and `try_command_result() -> Result<CommandResult, _>`. Docs note the dual emission (`tool.result` + `task.lifecycle.output` chunk) and that `tool.result` is authoritative for de-dupe.

## [0.1.6] - 2026-08-11

### Fixed

- **`tool.result` tolerates missing `correlation_facts`** — the live wire now
  omits `correlation_facts` on some tool results (compact `bash` results
  like `{"items":5,"ok":true,"revision":4}` from `3035c77c-efca...`).
  `ToolResult.correlation_facts` is now `Option<Value>` with `#[serde(default)]`
  so `typed_payload()` no longer errors with `missing field correlation_facts`
  (fixes #293, #297, #298, #299). Existing captures still round-trip with
  `Some({outcome, tool_name})`.

## [0.1.5] - 2026-08-06

### Added

- **Full `muse exec` flag parity** — `MuseExecBuilder` now covers the
  entire flag surface of Muse Code 0.1.0, verified flag-by-flag against
  the real binary: `prompt_file`, `api_key_stdin`, `parallel_tool_calls`,
  `agents`, `image` (repeatable), `workspace`, `worktree` (typed
  `WorktreeMode`) with `worktree_base`/`worktree_existing`, the context
  compaction trio, `max_model_steps`, `max_tool_output_bytes`,
  `allow_workspace_switch`, `user_input_auto_resolve`,
  `subagent_worktree_isolation`, `disable_web_tools`,
  `no_foreign_personal_context`, `no_session_log`, and the safety group
  (`yolo`, `trust_workspace`, `disable_approval`, `disable_sandbox`,
  `sandbox_network`, `disable_write`, `disable_shell`,
  `enable_shell_tool`).
- **Measured CLI constraints documented on the methods and pinned by live
  tests**: `--parallel-tool-calls`/`--api-key-stdin`/`--image` are
  meta-provider-only (echo rejects at startup);
  `--allow-workspace-switch` requires `--session-id`; `--no-session-log`
  conflicts with `--session-id` ("a session id needs retained logging") —
  so multi-turn continuity requires the session log. `--agents` is
  accepted by `exec` despite appearing only in the top-level help.

### Changed

- `MuseExecBuilder` implements `Default` (binary `muse` from `PATH`,
  empty prompt); `--api-key-stdin` pipes the child's stdin so the caller
  can write the key.

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
