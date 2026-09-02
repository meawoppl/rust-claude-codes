# pi-codes

Typed Rust SDK for the [pi coding agent](https://github.com/earendil-works/pi)
(npm `@earendil-works/pi-coding-agent`): serde models of the
`pi --mode json` JSONL event stream and the `pi --mode rpc` stdin/stdout
command protocol, plus an async (Tokio) RPC client.

Tested against pi 0.84.4. The crate version may carry a patch offset
above the CLI release for crate-side additions.

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

The model tier (gated on `OPENAI_API_KEY`) adds a streamed model turn
plus a conformance trio mirroring wirecheck's cross-harness checks:
the model reads a planted nonce file, writes a requested nonce to a
requested path, and runs a shell command with a disk-visible side
effect — write/bash verified on disk, never from the transcript.

## Measured wire notes (pi 0.84.4)

- **pi has no tool-approval gate.** read/bash/edit/write run without
  prompting; there is no `--yolo` equivalent because nothing needs
  bypassing. `--approve`/`--no-approve` only control trusting
  project-local config files (extensions, AGENTS.md).
- **`tool_execution_end` carries no `args` field** in RPC mode, though
  the documented `AgentEvent` type includes it. Pinned by the corpus;
  the decoder tolerates both.
- **`--mode json` hangs headless**: with no TTY it produced no output
  and never exited in our probes (with and without `--print`). Use
  `--mode rpc` for headless work — that is what `PiRpcClient` drives.
- The model may issue parallel tool calls in one assistant message; a
  racing read can fail with ENOENT and recover next turn. Tool errors
  arrive as `tool_execution_end { isError: true }` with the error text
  in `result.content`, not as stream errors.

## Framing contract

RPC mode is strict JSONL: split records on `\n` only and tolerate a
trailing `\r`. U+2028/U+2029 are valid inside JSON strings and must not
split records — the Tokio line reader used here complies.
