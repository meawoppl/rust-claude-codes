# muse-codes

Typed Rust SDK for [Meta's Muse Code](https://dev.meta.ai/docs) terminal
coding agent.

Muse Code's headless mode (`muse exec --json`) emits an event-sourced JSONL
journal on stdout: envelope records covering command intake, run lifecycle,
task lifecycle, and streamed output. This crate types that stream and ships
an async Tokio client for driving headless runs.

Tested against Muse Code 0.1.0 (`0.1.0-R708.1`). The crate version may
carry a patch offset above the CLI release for crate-side additions.

## Captured, not guessed

Muse publishes no schema and no official SDK, so the wire is the contract:

- All models derive from **committed captures of real CLI output**
  (`test_cases/*.jsonl`), taken via the credential-free `--provider echo`
  mode plus live Muse Spark runs (basic, tool-use, multi-subagent) for
  the provider-only vocabulary (`run.model.configured`, `tool.result`,
  streamed/cancelled/rejected/status lifecycle events).
- The envelope keeps its payload as raw JSON (byte-faithful round-trips);
  [`MuseRecord::typed_payload`] lifts it into a typed `MusePayload`, and
  payload types not yet observed (the journal also records approvals,
  edits, and subagent lifecycle under a live provider) come back as
  `MusePayload::Unknown` instead of failing.
- A nightly workflow re-captures echo runs against the freshly installed
  CLI, fingerprints `payload_type → field set`, and files an issue on any
  drift from the committed snapshot
  (`tests/schemas/muse_stream_fingerprint.txt`).

## Features

| Feature | Description | WASM-compatible |
|---------|-------------|-----------------|
| `types` | Journal envelope + payload models (serde only) | Yes |
| `async-client` | Tokio client spawning `muse exec --json` | No |

`default = ["types", "async-client"]`.

## Quick start

```rust,ignore
use muse_codes::{ExecRun, MuseExecBuilder, MusePayload, Provider};

let run = ExecRun::spawn(
    &MuseExecBuilder::new("summarize this repo").provider(Provider::Meta),
).await?;

let terminal = run.wait_terminal(|record| {
    if let Ok(MusePayload::RunOutputDelta(d)) = record.typed_payload() {
        print!("{}", d.text);
    }
}).await?;
println!("\nterminal: {}", terminal.terminal);
```

The Meta provider needs credentials (`muse login`, `META_API_KEY`, or
`~/.config/muse/auth.json`); `Provider::Echo` runs without any.

## Auth helpers

No PTY needed — Muse's auth surface is automation-friendly:

```rust,ignore
use muse_codes::auth::{auth_set, credentials_present, DeviceLoginFlow};

// API key path (CI):
auth_set(&std::env::var("MY_META_KEY")?, None).await?;

// Or the browser device-code path:
let mut flow = DeviceLoginFlow::start().await?;
let dc = flow.device_code(std::time::Duration::from_secs(20)).await?;
println!("Open {} and confirm code {}", dc.verification_url, dc.code);
flow.wait_approved(std::time::Duration::from_secs(300)).await?;
assert!(credentials_present());
```

## Record identity — read this before persisting or rendering

Journal record `id`s are **UUID-shaped counters, not UUIDs**. They restart
at the same value for every session, so two sessions emit byte-identical id
lists; and `sequence` restarts on every turn. Neither is a safe handle
alone.

- Unique key: the composite **`(stream.id, id)`**
- Turn grouping: `causation_id`
- Ordering **within** a turn: `sequence`

Treat `id` as stream-local *everywhere* — dedup, correlation maps, and
frontend list keys included (a render key of `id` alone would alias rows
across sessions). `stream.id` is the only cross-session-unique handle;
supply your own via [`MuseExecBuilder::session_id`] and it is adopted
verbatim.

## Known wire gaps (Muse Code 0.1.0)

Things the stream does *not* carry, which consumers must work around:

- **`tool.result` has no `task_id`.** Tool outcomes cannot be attributed to
  the task that issued them from the record alone. A consumer building a
  task tree must guess — attaching the result to the most recently started
  non-terminal task works for every capture in `test_cases/`, but would
  mis-attribute under genuinely concurrent tasks. The `call_id` it does
  carry is a provider call id, not a task handle. Fixing this properly
  requires the field upstream; don't grow a smarter heuristic to compensate.
- **No usage/token accounting** appears anywhere in the observed stream.
- **No approval round-trip**: policy decisions are journaled after the fact
  (`side_effect_intent.policy_decision`), never asked, so headless runs
  cannot be gated interactively.

## Testing

- **Corpus tests**: every committed capture line must parse, lift into a
  known typed payload, round-trip byte-faithfully, and satisfy envelope
  invariants (schema version, monotone sequences, durability classes).
- **Live test** (`--features integration-tests`): drives a real
  echo-provider run through the typed client to its terminal record —
  needs `muse` on `PATH`, no credentials.
