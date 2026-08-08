# Test cases

Fixtures for `tests/deserialization_tests.rs`. Every file is one `OutputEvent`
frame, formatted with 2-space indentation.

## `events/` — captured

Frames recorded verbatim from a live `localharness` 0.1.10. This is the wire,
not a re-serialisation of it, which is what makes the "decoding loses nothing"
assertion meaningful: it compares every leaf of the original against the same
document round-tripped through the crate's types.

Three sessions, distinguished by the leading digit:

| Prefix | Session | What it covers |
|---|---|---|
| `0xx` | Rejected API key | Handshake, initialize, prompt echo, in-band error step, `STATE_FULLY_IDLE` carrying a failure |
| `1xx` | Live model, no tools | A full successful turn: twelve `stepUpdate` frames of incremental `textDelta`, then the settled `text`, plus a `usageUpdate` |
| `2xx` | Live model, read-only tools | An agentic turn — `listDirectory` → `viewFile` → answer, across four step indices, with per-step `usageUpdate` frames |

The `2xx` session is the one that exercises the interesting shapes: multiple
concurrent step indices on one trajectory, action payloads nested inside a
`stepUpdate`, and `STATE_ACTIVE` → `STATE_DONE` transitions per step.

Capture more with:

```sh
ANTIGRAVITY_HARNESS_PATH=/path/to/localharness GEMINI_API_KEY=... \
  cargo run -p antigravity-codes --example capture_frames -- ./captures "your prompt"
```

Note that the example writes *re-serialised* frames, which would make the
no-field-loss test tautological. For fixtures that carry their evidentiary
weight, capture the raw text instead — `RUST_LOG=antigravity_codes=trace` logs
every frame verbatim as it arrives.

### Still not represented

- **Subagent trajectories.** Needs a session with `subagents` enabled and a
  prompt that provokes delegation.
- **Harness-side write tools** — `editFile`, `createFile`, `runCommand`. The
  captures use the read-only default; these are covered synthetically.
- **`policyDecisionRequest`.** Needs a dynamic policy rule configured.

## `synthetic/` — hand-written

Frames for paths the captured sessions do not reach: client-side tool calls,
lifecycle hooks, policy decisions, questions, tool confirmations, and the write
actions above. These are written against the protobuf descriptor rather than
observed, so they prove the types decode a *correct* frame — not that the
harness emits exactly this shape.

Two of those paths *are* known to work end to end even though the committed
fixture is synthetic: `examples/custom_tool.rs` runs a live session that
provokes a real `callHookRequest` (`LIFECYCLE_HOOK_PRE_TOOL`) and a real
`toolCall`, answers both, and the model consumes the tool's result. That
exercise is what the synthetic fixtures are modelled on.

Two files are deliberately wrong:

- `unknown-future-frame.json` — an `OutputEvent` whose `oneof` arm this crate
  has never heard of. Must decode, with no arm set.
- `unknown-enum-value.json` — a `StepUpdate.state` value from a future harness.
  Must decode, retaining the value verbatim.

Both exist because the harness is versioned independently of this crate: a hard
decode failure on an upstream addition would turn every release into an outage.
