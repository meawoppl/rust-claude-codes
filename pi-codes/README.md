# pi-codes

Typed Rust SDK for the [pi coding agent](https://github.com/earendil-works/pi)
(npm `@earendil-works/pi-coding-agent`): serde models of the
`pi --mode json` JSONL event stream and the `pi --mode rpc` stdin/stdout
command protocol, plus an async (Tokio) RPC client.

**Alpha.** Tested against pi 0.84.4 — but unlike the sibling crates,
the crate version does **not** yet follow the version-means-tested
convention: it starts at 0.0.1 while the API settles, and will jump to
the tested pi release once pinned forward. Expect breaking changes
between 0.0.x releases.

## What's covered

- **`--mode rpc`** — the headless command protocol: typed
  [`RpcCommand`]s (prompt/steer/follow-up, state, model, thinking,
  compaction, bash, session), the `{"type":"response"}` envelope with id
  correlation, and typed views of the important payloads
  ([`AgentState`], [`BashResult`], messages). Unmodeled commands pass
  through via `RpcCommand::Raw`.
- **`--mode json`** — the one-shot event stream: `PiEvent` with typed
  lifecycle/tool events and an `Unknown` fallback that preserves the
  payload, so newer CLIs degrade soft.
- **Messages** — `user` / `assistant` / `toolResult` / `bashExecution`
  roles with content blocks (text, thinking, toolCall, image) and usage
  accounting.

## Live testing

The integration tier (`cargo test -p pi-codes --features
integration-tests`) drives a real `pi --mode rpc` process and is
**credential-free**: state, model catalog, id correlation, the `bash`
command (a real shell round trip with typed results landing in
`get_messages`), and clean failure envelopes for unknown commands.
Model-turn coverage additionally needs a configured provider
(`pi auth check --provider <p>`).

Note: pi requires **Node 22+** (`fs.globSync`).

## Framing contract

RPC mode is strict JSONL: split records on `\n` only and tolerate a
trailing `\r`. U+2028/U+2029 are valid inside JSON strings and must not
split records — the Tokio line reader used here complies.
