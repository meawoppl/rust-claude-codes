# Test cases

Fixtures for `tests/deserialization_tests.rs` and `tests/step_assembly_tests.rs`.
Every file is one `OutputEvent` frame, formatted with 2-space indentation.

## `events/` — captured verbatim

Frames recorded from a live `localharness` 0.1.10, re-indented but **not**
re-encoded. That distinction is the whole point of this directory: a frame that
has been decoded and re-encoded can only contain fields the crate already
models, so a corpus built from re-serialised frames agrees with itself by
construction and can never catch a field the types are missing. Only raw frames
make `decoding_a_captured_frame_loses_nothing` mean anything.

(This was not hypothetical. The first pass at the capture example wrote decoded
frames, and the one place it accidentally wrote a *differently shaped* value —
the bare `InitializeConversationResponse` instead of the `OutputEvent` carrying
it — was caught immediately by that test. `RawClient::next_frame` and
`initialize_frame` exist so captures stay verbatim.)

Four sessions, distinguished by the leading digit:

| Prefix | Session | What it covers |
|---|---|---|
| `0xx` | Rejected API key | Handshake, initialize, prompt echo, in-band error step, `STATE_FULLY_IDLE` carrying a failure |
| `1xx` | Live model, no tools | A full successful turn: twelve `stepUpdate` frames of incremental `textDelta`, then the settled `text`, plus a `usageUpdate` |
| `2xx` | Live model, read-only tools | An agentic turn — `listDirectory` → `viewFile` → answer, across four step indices, with per-step `usageUpdate` frames |
| `3xx` | Dynamic policy rule | A real `policyDecisionRequest`, raised by a `PolicyRule` with `is_dynamic` set |

The `3xx` session also documents a behaviour worth knowing: the harness **blocks
the turn indefinitely** on a policy request until the client answers. That
session was recorded with `RawClient`, which does not answer, so it simply stops
after the request — which is the evidence.

Capture more with:

```sh
ANTIGRAVITY_HARNESS_PATH=/path/to/localharness GEMINI_API_KEY=... \
  cargo run -p antigravity-codes --example capture_frames -- ./captures "your prompt"
```

`ANTIGRAVITY_TOOLS=all`, `ANTIGRAVITY_SUBAGENTS=1`, and `ANTIGRAVITY_POLICY=1`
widen what a session will exercise.

### Not captured, and why

Recording an agentic turn costs many model calls, and the Gemini free tier
allows **20 requests per minute** — which a single multi-step turn can exhaust
on its own, made worse by the harness retrying internally on `429`. Sessions
that were observed working live but could not be re-recorded verbatim within
that budget:

- **Write tools** — `editFile` and `runCommand` frames. A session that created a
  file, edited another, and ran `wc -l` was observed completing correctly (45
  frames), but only via the earlier re-serialising recorder. Covered
  synthetically for now.
- **Subagent delegation** — observed producing two concurrent trajectories, the
  main one a 32-hex cascade id and the subagent a UUID, each numbering steps
  from zero. Modelled in `synthetic/subagent-session/` from that observation.

Both are worth re-recording on an account without the free-tier cap.

## `synthetic/` — hand-written

Frames for paths the captured sessions do not reach: client-side tool calls,
lifecycle hooks, questions, tool confirmations, and the write actions above.
Written against the protobuf descriptor rather than observed, so they prove the
types decode a *correct* frame — not that the harness emits exactly this shape.

Two of those paths *are* known to work end to end even though the committed
fixture is synthetic: `examples/custom_tool.rs` runs a live session that
provokes a real `callHookRequest` (`LIFECYCLE_HOOK_PRE_TOOL`) and a real
`toolCall`, answers both, and the model consumes the tool's result.

And where a real capture later arrived, it confirmed the synthetic shape:
`synthetic/policy-decision-request.json` was written blind from the descriptor,
and the captured `3xx` frame matches it field for field, differing only in that
the harness sends `serverName: ""` explicitly and omits `callId`.

### `synthetic/subagent-session/`

A whole replayable session rather than a single frame, consumed by
`step_assembly_tests.rs`. It is the only fixture exercising concurrent
trajectories, which is what makes `(trajectory_id, step_index)` — rather than
the index alone — the necessary key for step bookkeeping.

### Deliberately wrong

- `unknown-future-frame.json` — an `OutputEvent` whose `oneof` arm this crate
  has never heard of. Must decode, with no arm set.
- `unknown-enum-value.json` — a `StepUpdate.state` value from a future harness.
  Must decode, retaining the value verbatim.

Both exist because the harness is versioned independently of this crate: a hard
decode failure on an upstream addition would turn every release into an outage.
