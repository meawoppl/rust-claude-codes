# muse-codes

Typed Rust SDK for [Meta's Muse Code](https://dev.meta.ai/docs) terminal
coding agent.

Muse Code's headless mode (`muse exec --json`) emits an event-sourced JSONL
journal on stdout: envelope records covering command intake, run lifecycle,
task lifecycle, and streamed output. This crate types that stream and ships
an async Tokio client for driving headless runs.

Tested against Muse Code 0.1.0 (`0.1.0-R708.1`).

## Captured, not guessed

Muse publishes no schema and no official SDK, so the wire is the contract:

- All models derive from **committed captures of real CLI output**
  (`test_cases/*.jsonl`), taken via the credential-free `--provider echo`
  mode, which exercises the full headless event vocabulary without model
  calls or a Meta login.
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

## Testing

- **Corpus tests**: every committed capture line must parse, lift into a
  known typed payload, round-trip byte-faithfully, and satisfy envelope
  invariants (schema version, monotone sequences, durability classes).
- **Live test** (`--features integration-tests`): drives a real
  echo-provider run through the typed client to its terminal record —
  needs `muse` on `PATH`, no credentials.
