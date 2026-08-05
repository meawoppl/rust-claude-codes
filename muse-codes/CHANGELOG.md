# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
