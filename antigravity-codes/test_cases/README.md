# Test cases

Fixtures for `tests/deserialization_tests.rs`. Every file is one `OutputEvent`
frame, formatted with 2-space indentation.

## `events/` — captured

Frames recorded verbatim from a live `localharness` 0.1.10. This is the wire,
not a re-serialisation of it, which is what makes the "decoding loses nothing"
assertion meaningful: it compares every leaf of the original against the same
document round-tripped through the crate's types.

Capture more with:

```sh
ANTIGRAVITY_HARNESS_PATH=/path/to/localharness \
  cargo run -p antigravity-codes --example capture_frames -- ./antigravity-codes/test_cases/events "your prompt"
```

Note that the example writes *re-serialised* frames. For fixtures that carry
their evidentiary weight, capture the raw text instead — `RUST_LOG=trace` logs
every frame verbatim at the `antigravity_codes::client_raw` target.

### What this directory is missing

These captures come from a session with a deliberately rejected API key, so they
cover the handshake, initialize, prompt echo, the model-failure path, and the
trajectory lifecycle — but **not** a successful model turn. Anything downstream
of a real model response (streaming text deltas across many frames, harness-side
tool actions, subagent trajectories) is not represented here yet. Captures from
a session with a working key are welcome.

## `synthetic/` — hand-written

Frames for paths the rejected-key session never reaches: tool calls, lifecycle
hooks, policy decisions, usage updates, questions, tool confirmations, and the
richer step actions. These are written against the protobuf descriptor rather
than observed, so they prove the types decode a *correct* frame — not that the
harness emits exactly this shape.

Two are deliberately wrong:

- `unknown-future-frame.json` — an `OutputEvent` whose `oneof` arm this crate
  has never heard of. Must decode, with no arm set.
- `unknown-enum-value.json` — a `StepUpdate.state` value from a future harness.
  Must decode, retaining the value verbatim.

Both exist because the harness is versioned independently of this crate: a hard
decode failure on an upstream addition would turn every release into an outage.
