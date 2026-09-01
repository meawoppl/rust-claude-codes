# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.1.239] - 2026-09-01

Re-baseline against Claude CLI **2.1.239**: the full live integration suite
(28 tests) passes, so the tested pin (`TESTED_VERSION`, both README
`Tested against:` lines) and the crate version move to 2.1.239. Models the
2.1.232 → 2.1.239 stream-json drift (all additive) and repairs the schema
extractor against the CLI's new bundle style.

### Added

- `AssistantMessage.context_usage` (`ContextUsage` + `ContextCategory`,
  `ContextOverLimit`, `ContextMcpTool`, `ContextMemoryFile`, `ContextAgent`,
  `ContextSkill`) — the structured twin of the `/context` report — and
  `AssistantMessage.batch_tool_uses` (`BatchToolUse`).
- `ResultMessage.subagent_stats` (`SubagentStats` + `SubagentSpawnRequests`,
  `SubagentKillCounts`, `SubagentRefusalCounts`) — running totals for
  subagents started through the Agent tool.
- `InitMessage.effort` (the session's resolved effort level) and
  `InitMessage.cloud_session` (raw JSON snapshot on cloud-hosted sessions).
- `TaskStartedMessage.{is_backgrounded, spawn_depth}`,
  `CodeChangePublishedMessage.action`, `VcsStateChangedMessage.branch`,
  `ModelRefusalFallbackMessage.saw_cyber_refusal`, and
  `UserMessage.seeded_summon`.

### Fixed

- `scripts/extract_claude_sdk_schemas.py` (and with it the nightly drift
  check, which had been soft-skipping since the CLI's bundler switched
  styles): schemas are now also discovered in the free-function zod style
  (`ve(()=>_e({type:Tt("…")}))`) used by CLI 2.1.239+, alongside the
  zod-namespace style (`Se(()=>E.object({type:E.literal("…")}))`) of older
  binaries. The snapshot is re-baselined against 2.1.239 (43 wire labels).

## [2.1.234] - 2026-08-19

### Added

- **`version::tested_cli_version()`** — the tested-against CLI release,
  machine-readable from the published artifact (previously a private
  const only the runtime warning could see). CI keeps it in lockstep
  with the README's `Tested against:` line.

## [2.1.233] - 2026-08-19

### Added

- **`transcript` module** — the CLI's unpublished on-disk transcript
  path rule, exported so consumers stop growing private copies:
  `encode_project_dir` (`/` and `.` → `-`, measured against real
  transcript stores and pinned by tests, documented as lossy) and
  `transcript_path(home, cwd, session_id)`, which takes `home`
  explicitly so a test can never accidentally resolve into a real
  `~/.claude`. Extracted at agent-portal's request to delete its
  duplicate implementation.

## [2.1.232] - 2026-08-14

### Added

- Three fields CLI 2.1.232 added to the wire, each found by wirecheck's
  live subagent wrapping audit (present on the wire, dropped by the typed
  model): **`messaging_socket_path`** and **`terminal_slash_commands`**
  on the `system` init message, and
  **`usage.output_tokens_details.thinking_tokens`** on the `result`
  message (typed as `OutputTokensDetails`).

### Changed

- `TESTED_VERSION` and the crate version move to 2.1.232; the full live
  wirecheck claude tier (curated + cargo) passes against the installed
  CLI.

## [2.1.223] - 2026-08-05

### Changed

- **`LoginOutcome`, `TokenSource`, and `Osc52Status` now derive
  Serialize/Deserialize** — login outcomes are relay-shaped for web/launcher
  surfaces that ship them over the wire (requested by agent-portal's
  login-automation assessment). No wire or behavior change.

## [2.1.222] - 2026-08-04

Catches up to Claude CLI 2.1.222; snapshot baseline and `TESTED_VERSION`
move to 2.1.222, and the full live suite passes against the installed
binary. Also ships the complete login-tooling saga and session forking
(below) that accumulated unreleased.

### Added (2.1.222 drift)

- **`ModelRefusalFallbackMessage.scope`** — new open enum
  `RefusalFallbackScope` (`session` | `local` + `Unknown(String)`):
  `session` means the main thread fell back and the session model is
  swapped (also the meaning when the field is absent, i.e. every CLI
  before 2.1.222); `local` means a subagent / side-question / background
  fork fell back and only that response used the fallback model. (#272)

### Changed

- **Rejected-code retry now follows the CLI's real state machine** —
  recovered from the 2.1.220 binary: the error screen renders **no input
  component**, and its only affordance (Enter) restarts the OAuth flow with
  a **new PKCE challenge**. There is no same-challenge retry; a corrected
  code pasted after a rejection lands nowhere, silently. Accordingly:
  `submit_code` after a `CodeRejected` now returns `InvalidState` instead
  of writing into the void, and the new
  **`LoginFlow::retry_new_url(timeout)`** presses Enter, waits for
  `waiting_for_login` to re-render, and returns the **new** authorize URL
  to show the user (the old URL's code is dead). Live-verified end to end:
  reject → refused re-paste → new URL with rotated challenge in ~1 s →
  fresh input field accepts the next submission. (#270)

### Added

- **Bracketed-paste-mode telemetry**: channel lines now stamp
  `paste-mode@submit=on|off|never-advertised` — the TUI's advertised
  `?2004h/l` state at the moment the code was written, from the raw
  stream. Directly tests the late-write hypothesis (a TUI that drops paste
  mode while the user is off authorizing would receive a late frame as raw
  ESC keypresses) in production, where the 19–22 s human submit latency
  cannot be replicated by harnesses that write in milliseconds.
  `SUBMIT_PATH` → `…+paste-probe/v6`. (#269)

### Fixed

- **Login flows were structurally blind to child death.** `LoginFlow` held
  the PTY *slave* open for its whole life, so when the CLI exited, the
  master never saw EOF and the reader blocked forever — production attempt
  four's child died ~5 s after code submission and the flow spent the
  remaining 85 s polling a corpse, reporting benign absence on every
  channel. The slave is now dropped at spawn (the child owns its own fds),
  making master-side EOF fire on exit. (#267)

### Added

- **Child death is a first-class outcome** (#267):
  - New `Error::LoginChildExited { code, transcript }`, returned within
    ~1 s of death from `auth_url` and `submit_code_and_wait` (EOF-driven,
    with a `try_wait` backstop for a dead parent whose PTY is held open by
    an orphaned grandchild). Replaces the generic `Unknown`/`Protocol`
    errors on the exited-without-outcome paths — downstream match arms
    with a catch-all are unaffected.
  - **Pre-submit liveness check**: a death *before* the paste is reported
    as such ("BEFORE code submission — nothing was written"), timestamping
    death against the write — the discriminator between "the frame killed
    it" and "it was already gone".
  - Channel lines now carry `child=alive|exited(code)`; `SUBMIT_PATH`
    bumps to `…+exit-aware/v5`.
  - An exit that races a credentials write still resolves to success.
  Live-verified: kill-after-submit surfaces `LoginChildExited(code=143)`
  in ~570 ms with the masked input echo intact in the tail; kill-before
  is caught in microseconds; the rejection path is unchanged (~375 ms).

- **Write-path provenance**: `auth::SUBMIT_PATH` identifies the compiled
  code-submission mechanism and is stamped into the `LoginTimeout` channel
  line (`submit-path=bracketed-paste+lone-cr-150ms+term-forced/v3`) — which
  deployed binary is running becomes readable from one log line. Necessary
  because release builds inline the paste-frame bytes into immediates, so
  byte-grepping a binary for `ESC[200~` proves nothing in either
  direction. (#265)

### Changed

- **Login flows scrub session/credential env vars from the spawned CLI**
  (`CLAUDECODE`, `CLAUDE_CODE_ENTRYPOINT`, `CLAUDE_CODE_CHILD_SESSION`,
  `CLAUDE_CODE_SESSION_ID`, `CLAUDE_CODE_OAUTH_TOKEN`, `ANTHROPIC_API_KEY`,
  `ANTHROPIC_AUTH_TOKEN`), aligning with the session spawn path's
  `CLAUDECODE` scrubbing in `cli.rs` — a login child's purpose is minting
  fresh credentials, so it must not inherit the host's. Measured on 2.1.220
  that none of these break submission when present; this removes the axis,
  not a reproduced failure. `SUBMIT_PATH` bumps to
  `…+env-scrubbed/v4`. (#266)
- **Login flows now force `TERM=xterm-256color` on the spawned CLI.** The
  crate is the terminal on the other side of the PTY (it parses OSC 8/52
  and speaks bracketed paste), so it advertises a deterministic capability
  surface instead of inheriting the host process's TERM (server processes
  often have none). Measured on CLI 2.1.220, submission works under
  `TERM=dumb` and TERM-unset too — this eliminates an environment axis
  rather than fixing a reproduced failure. (#265)

### Fixed

- **Login code submission failed silently for every production-length
  code.** The TUI classifies any single PTY write of **≥ 64 bytes** as a
  paste and swallows a trailing CR into the paste payload instead of
  treating it as Enter (measured on CLI 2.1.220: 62-char code + CR submits;
  63-char + CR sits at the prompt forever). Real authorization codes are
  ~90+ chars, so no unframed single-chunk write could ever submit — this is
  what produced the silent 30s/90s timeouts in production, undetected
  because every test fixture happened to be under 64 bytes. `submit_code`
  now writes the code wrapped in **bracketed-paste framing**
  (`ESC[200~…ESC[201~` — the paste path the CLI explicitly enables via
  `ESC[?2004h`), then sends CR as its own write 150 ms later. Live-verified
  across a 36–120 byte matrix including both previously-silent boundary
  lengths; unit fixtures moved to production lengths (64/92/108/120) and a
  live integration test now submits 92- and 108-char codes, both of which
  any sub-64-byte fixture is structurally incapable of covering. (#263)

### Added

- **Three-source success detection for login flows**, driven by the prod
  finding that every *rejection* is detectable on screen but *success* may
  not be (the TUI masks secret material). `submit_code_and_wait` and
  `finish` now recover the minted token from: (1) visible screen text,
  (2) **OSC 52 clipboard-copy escapes** decoded from raw PTY bytes — exact,
  immune to wrapping/masking; confirmed emitted by CLI 2.1.220 — and
  (3) **credentials-store watching**: `.credentials.json` (under
  `CLAUDE_CONFIG_DIR` or `~/.claude`) created/updated after submission is
  authoritative success even with nothing observable on the PTY.
  `LoginOutcome` gains `token_source: Option<TokenSource>` and
  `credentials_updated: bool`. (#259)
- **Self-diagnosing login timeouts**: new `Error::LoginTimeout { transcript }`
  replaces `Error::Timeout` in `submit_code_and_wait`, carrying a per-channel
  status line (`screen` / `osc52` / `credentials`) plus the post-submission
  screen content — a silent failure now names the blind channel. (#259)

### Changed

- **Success-path OSC 52 telemetry**: `LoginOutcome` gains
  `osc52: Osc52Status` (`Absent` / `Unterminated` / `Undecodable` /
  `PresentNoToken` / `TokenRecovered`) and `copy_nudge_sent: bool`, and the
  `LoginTimeout` channel line gains `copy-nudge=sent|not-sent` — so a
  `token: None` success distinguishes "screen offers no copy affordance"
  from "affordance fired with a non-token payload", crate-side, where the
  PTY bytes are actually observable. The copy nudge is also recorded so any
  unexpected TUI consequence is attributable. (#261)
- **Copy-affordance nudge on confirmed success**: when the credentials store
  updates but no token is yet observable, `submit_code_and_wait` presses `c`
  (the TUI's "c to copy" affordance) once, giving the OSC 52 channel a chance
  to deliver the exact token bytes before the grace window closes — reducing
  the frequency of `token: None` successes that downstream must treat as
  re-login-after-redeploy mode. Best-effort: a stray keypress on screens
  without the affordance, in a flow that has already succeeded. (#260)
- **Login rejection detection widened**: the collapsed `Press Enter to retry`
  prompt now anchors `CodeRejected` as a fallback when the error wording
  doesn't contain `OAuth error` — live capture shows the renderer mangles
  wording (`Requstfailed withstatus code 400`), while the retry prompt is the
  rejection loop's structural constant. (#259)
- **`submit_code` hardening**: the code and its terminating CR are written as
  a single chunk, and an empty-after-trim code is refused with a clear error
  instead of pressing Enter on an empty field (a silent hang). (#259)

- **Login support tooling** behind a new `auth` feature. The CLI's login
  flows are interactive Ink TUIs (they hang forever on a pipe), so
  `auth::LoginFlow` drives them under a pseudo-terminal (`portable-pty`) and
  exposes the flow's human shape as an API: `start(LoginMode)` →
  `auth_url()` (lifted intact from the CLI's OSC 8 hyperlink) → user visits
  the URL and brings back a code → `submit_code()` → `finish()`, which
  returns the minted `sk-ant-oat01-…` token for `LoginMode::SetupToken`.
  Dropping the flow cancels it. `auth::auth_status()` types
  `claude auth status --json`. Live-tested: a real `setup-token` flow yields
  the PKCE authorize URL; `examples/login.rs` walks the full interactive
  loop. (#257)

- **`LoginFlow::submit_code_and_wait`**: outcome-driven completion that
  watches the CLI's *output* instead of its exit. Production use showed two
  hangs `finish()` cannot see: the CLI never exits on a rejected code (it
  prints `OAuth error … Press Enter to retry` and waits), and its Ink TUI
  positions words with cursor-column escapes instead of spaces, so
  ANSI-stripped text runs together (`Pastecodehereifprompted>`) and
  phrase-matching never fires. The new method returns as soon as a minted
  token appears, fails fast with the new `Error::CodeRejected { message }`
  when an OAuth error is printed (scanning only output after the current
  submission, so a retry can't trip on a prior attempt's error), and leaves
  the flow alive on rejection so a corrected code reaches the same PKCE
  session. Presence checks collapse whitespace; token extraction stays
  newline-bounded (collapsing would glue trailing prose onto the token) with
  a loud display-wrap detector instead of silent truncation. PTY width is
  raised to 1000 columns so wrapping is impossible at the source.

- **Session forking**: `ClaudeCliBuilder::fork_from(source_session_id)`
  assembles the full `--resume <src> --fork-session --session-id <new>`
  combination (generating a fresh UUID unless one is chained via
  `session_id`), and `fork_session(bool)` exposes the raw flag — which the
  builder previously could not emit at all. Covered by a live integration
  test proving the fork carries the source's history under the new session
  id. (#227)

### Changed

- **Breaking**: `UsageInfo.iterations` is now `Vec<UsageIteration>` (was
  `Vec<serde_json::Value>`), with typed `input_tokens`, `output_tokens`,
  optional per-iteration cache fields (`cache_read_input_tokens`,
  `cache_creation_input_tokens`, `cache_creation`), and the wire `type`
  exposed as `kind` (`"turn"` and `"message"` observed). (#250)
- **`UsageInfo` docs now state the counters are accumulated roll-ups** across
  the turn's API iterations — correct for cost, wrong for context occupancy
  (`cache_read_input_tokens` can exceed the context window several times over
  on tool-heavy turns). Use the last `iterations` entry for context
  estimates, as the CLI does. (#250)

## [2.1.220] - 2026-07-31

Version jumps 2.1.166 → 2.1.220 to re-align with the tested Claude CLI
version, per the crate's versioning convention.

### Added

- `RawAsyncClient` for newline-framed, undecoded access to the Claude
  stream-json protocol.
- `ClaudeInput::user_message_without_session` for messages associated with the
  current CLI process session.
- `ClaudeCliBuilder::working_directory` for setting the working directory of
  spawned Claude CLI processes.

### Changed

- On Windows, Claude CLI launches and version checks now use
  `CREATE_NO_WINDOW` to avoid opening console windows.

### Contributors

- Thanks to @ozitrance — a first-time contributor to this crate — for the
  entire feature set in this release: `RawAsyncClient`,
  `user_message_without_session`, `working_directory`, and the Windows
  `CREATE_NO_WINDOW` support (#243, #244, #245).

## [2.1.166] - 2026-07-27

### Changed

- **Tested Claude CLI version** bumped to 2.1.220 — no wire drift vs the
  2.1.219 snapshot, and the full integration suite passes against the
  installed binary.
- Shared dependencies and lint policy moved to the workspace root: `serde`,
  `serde_json`, `thiserror`, `tokio`, `log`, `which`, and dev `env_logger` are
  now `{ workspace = true }`, and the crate opts into `[workspace.lints]`
  (`unsafe_code = "deny"`). Aligns previously drifted versions: `tokio`
  1.47.1 → 1.49.0, `log` 0.4.27 → 0.4.29, `env_logger` 0.11.8 → 0.11.9.

## [2.1.165] - 2026-07-25

### Added

- **`SystemMessage::as_code_change_published()` / `as_vcs_state_changed()`**
  (plus `is_*` checks) — dedicated typed accessors following the
  `as_init()` pattern, so consumers reach `CodeChangePublishedMessage` /
  `VcsStateChangedMessage` without matching on `KnownSystemEvent` or poking
  raw JSON (#231). Note: the `vcs_state_changed` SDK frame is flat
  (`kind` + `cwd`) — the CLI's internal git/gh watcher event carries richer
  `commit`/`push`/`branch`/`pr` sections, but the emitter flattens each to
  one-or-more `kind` frames before they cross the wire.

## [2.1.164] - 2026-07-24

Catches up to CLI 2.1.219 — the Opus 5 release. Snapshot baseline and
`TESTED_VERSION` move to 2.1.219; the full integration suite passes against
the installed binary.

### Added

- **`ClaudeModel::Opus5`** — pinned variant for `claude-opus-5` (display name
  "Opus 5", knowledge cutoff May 2026). The floating `opus` alias resolves to
  `claude-opus-5` first-party as of CLI 2.1.219 (noted on the `Opus` variant
  doc). Model table refreshed from the 2.1.219 binary; the accepted floating
  aliases are unchanged.
- **`FastModeDisabledReason`** — open enum (`free`, `preference`,
  `extra_usage_disabled`, `network_error`, `unknown`, `not_first_party`,
  `disabled_by_env`, `model_not_allowed`, `sdk_opt_in_required`, `pending`,
  plus `Unknown(String)`) carried as the new optional
  `fast_mode_disabled_reason` on `ResultMessage` and `InitMessage`: why fast
  mode can't serve right now, complementing the existing `fast_mode_state`.
- **`InitMessage.mcp_server_errors`** — new `McpServerError` struct (`name`,
  `type`, `message`) recording `--mcp-config` entries that failed validation
  and were skipped.
- **`PluginInfo.version`** — installed plugin version on the init plugin
  list (caught by the live wire-fidelity audit, not the drift fingerprint —
  it is a nested field).

## [2.1.163] - 2026-07-22

Models the CLI 2.1.205 → 2.1.218 stream-json drift surfaced by the fixed
schema extractor (#223); the committed snapshot baseline and `TESTED_VERSION`
move to 2.1.218.

### Added

- **`ClaudeOutput::CommandLifecycle`** — typed variant for the new
  `command_lifecycle` wire type (fate of a queued command: queued → started →
  completed/cancelled/discarded), with `CommandLifecycleMessage` and the open
  `CommandLifecycleState` enum (`Unknown(String)` fallback).
- **`system/code_change_published`** — `CodeChangePublishedMessage`
  (`provider`, `url`, `repo`, `identifier`): the session is now associated
  with a published pull/merge request.
- **`system/vcs_state_changed`** — `VcsStateChangedMessage` with the open
  `VcsMutationKind` enum (`commit`/`push`/`merge`/`rebase` +
  `Unknown(String)`): a harness-observed command mutated repository state.
  Both new subtypes are wired through `SystemSubtype`, `KnownSystemEvent`,
  and the `typed_value` wrapping audit.
- **`AssistantMessage.aborted`** — true when the message was truncated by an
  interrupt/abort before the stream completed — and
  **`.resumed_from_incomplete_thinking`** — true when the turn continued a
  truncated thinking block (max-output-tokens recovery).
- **`ResultMessage.request_sent_wall_ms`** (fractional wall-clock ms) and
  **`.user_message_uuid`** (wire uuid of the user message the result answers).
- **`ToolProgressMessage.heartbeat`**, **`.subagent_type`**, and
  **`.subagent_retry`** (new `SubagentRetry` struct: attempt, max_retries,
  retry_delay_ms, error_status, error_category).
- **`UserMessage.tool_result_meta`** — new `ToolResultMeta` struct carrying
  the harness-stamped `non_execution_kind` for error tool results and any
  human-typed `user_feedback` deny comment.

## [2.1.162] - 2026-07-22

### Fixed

- **`ClaudeInput::interrupt()` now actually interrupts** (#218). It previously
  serialized to a bare `{"subtype":"interrupt"}`, which the CLI silently
  ignores (verified against 2.1.205 and 2.1.211) — the in-flight turn ran to
  completion. It now emits the required `control_request` envelope
  `{"type":"control_request","request_id":...,"request":{"subtype":"interrupt"}}`,
  which the CLI acknowledges with a `control_response` and cancels the turn
  immediately (verified live: ack in ~10ms, turn ends with
  `result subtype=error_during_execution`).

### Changed

- **Breaking**: `ClaudeInput::interrupt(request_id)` now takes the unique
  request id for the control envelope. `AsyncClient::interrupt()` /
  `SyncClient::interrupt()` generate an `interrupt-<uuid>` id and now return
  `Result<String>` (the id) so callers can correlate the CLI's
  `control_response` ack.
- **Breaking**: removed `SDKControlInterruptRequest` — its bare wire shape is
  a no-op against the CLI. Use `ClaudeInput::interrupt(request_id)` or the new
  `ControlRequestMessage::interrupt(request_id)` constructor instead.

### Added

- **`ControlRequestPayload::Interrupt`** variant and
  **`ControlRequestMessage::interrupt(request_id)`** constructor (mirroring
  `::initialize`), so the correctly-enveloped interrupt can be built typed.

## [2.1.161] - 2026-07-11

### Added

- **`ClaudeModel`** — convenience enum for every model selector the Claude CLI
  accepts, keyed by human-friendly names. Floating aliases (`Sonnet`, `Opus`,
  `Haiku`, `Fable`, `Best`, `OpusPlan`, and the `[1m]` context variants) plus
  the pinned registry models (`Fable5`, `Mythos5`, `Opus48` … `Haiku35`),
  extracted from the CLI 2.1.205 binary's model registry. `cli_arg()` returns
  the exact `--model` string, `display_name()` the human-readable name, and
  `Custom(String)` passes unknown models through verbatim.
  `ClaudeCliBuilder::model(ClaudeModel::Sonnet5)` works directly via
  `Into<String>`. Re-exported at the crate root.

## [2.1.160] - 2026-07-10

### Added

- **Claude Code CLI 2.1.205 output coverage.** Added typed coverage for new
  top-level SDK frames (`stream_event`, `tool_progress`, `auth_status`,
  `tool_use_summary`, `prompt_suggestion`, `conversation_reset`), 19 newer
  `system` subtypes, richer result/init/status/compact/user/assistant wrapper
  fields, and `get_usage` control-response quota payloads.
- **Rate limit schema updated to CLI 2.1.205.** `RateLimitInfo` now carries the
  full `rate_limit_event` wire schema:
  - New fields: `overage_resets_at`, `overage_in_use`, `surpassed_threshold`,
    `overage_period_monthly` / `overage_period_channel` (new
    `OveragePeriodUtilization` struct), `error_code` (new `RateLimitErrorCode`
    enum), `can_user_purchase_credits`, and
    `has_chargeable_saved_payment_method`.
  - `RateLimitWindow` gains `SevenDayOpus`, `SevenDaySonnet`,
    `SevenDayOverageIncluded`, and `Overage` variants.
  - `OverageStatus` gains `AllowedWarning`.
  - `OverageDisabledReason` expands from 2 to 12 typed variants matching the
    CLI enum.
  - Re-exported `OveragePeriodUtilization` and `RateLimitErrorCode` at the
    crate root.

- **`SubagentUsageRollup`** — session-level accumulator for the subagent
  token rollup the CLI renders as `<subagent_tokens>` / `<agent_count>` in its
  terminal `<usage>` block (resolves #169). Feed every `ClaudeOutput` through
  `observe()`; it gates on genuine `Task` results (`agentId`/`totalTokens`
  present), dedupes replayed frames by `agentId`, and totals tokens, agent
  count, tool uses, and duration. `UsageInfo` now documents explicitly that
  the `result` frame's usage covers the main agent only — the rollup is not
  carried on the wire. Re-exported at the crate root.

### Changed (breaking)

- `ClaudeOutput` gained additional variants, so exhaustive matches must handle
  the new top-level frame types.
- `ResultSubtype`, `TaskStatus`, and `TaskType` are now forward-compatible open
  enums with `Unknown(String)` fallbacks.
- `TaskStartedMessage.task_type`, `TaskStartedMessage.tool_use_id`, and
  `TaskProgressMessage.last_tool_name` are now optional to match CLI 2.1.205
  wire frames.
- `RateLimitInfo::is_using_overage` is now `Option<bool>` — the field is
  optional in the CLI wire schema and events omitting it previously failed to
  deserialize.
- `RateLimitWindow::Hourly` removed; the window no longer exists in the CLI
  schema. An `"hourly"` value now parses as `RateLimitWindow::Unknown`.

### Changed

- **Tested Claude CLI version** bumped to 2.1.205 — the full integration suite
  passes against the installed binary. One suite fix was required: CLI 2.1.205
  only enables `AskUserQuestion` in headless mode when a permission-prompt
  tool is configured (`--permission-prompt-tool`), so the round-trip test now
  spawns with one, matching the existing converge test.

## [2.1.159] - 2026-06-27

### Added

- **Typed subagent token accounting.** `Task`-tool result messages now expose
  the subagent's token / timing / tool-use rollup through typed fields instead
  of raw-JSON poking (resolves #168, #169):
  - New `SubagentResult` struct modeling the `Task` `tool_use_result` —
    `status`, `prompt`, `agent_id`, `agent_type`, `content`, `resolved_model`,
    `total_duration_ms`, `total_tokens`, `total_tool_use_count`, the nested
    per-model `usage` (`UsageInfo`), and an optional `SubagentToolStats`.
  - `UserMessage::subagent_result()` accessor parses the result leniently,
    returning `None` only when `tool_use_result` is absent.
  - `total_tokens` is the per-run `subagent_tokens` line item; summing it across
    a session's `Task` results yields the subagent token rollup the CLI renders
    in its terminal `<usage>` block. (The `stream-json` `result` frame's own
    `usage` does not carry the subagent rollup — the `Task` result is the source
    of truth.)
  - Re-exported `UsageInfo`, `ServerToolUse`, `SubagentResult`, and
    `SubagentToolStats` at the crate root.

### Changed

- Enriched the crate description, `keywords`, and `categories` for crates.io
  discoverability (agent / Claude Code / Anthropic / async terms).

## [2.1.158] - 2026-06-25

### Added

- **Subagent message coverage.** Typed every field a `Task`-tool /
  `local_agent` subagent session emits, verified against real captures:
  - New `system` subtypes `task_updated` (with `TaskUpdatedMessage` /
    `TaskPatch`) and `thinking_tokens` (with `ThinkingTokensMessage`), plus
    `SystemMessage::as_task_updated` / `as_thinking_tokens` accessors.
  - `TaskStartedMessage` and `TaskProgressMessage` gain `subagent_type`;
    `TaskStartedMessage` also gains `prompt`.
  - `AssistantMessage` gains `request_id`, `subagent_type`, `task_description`;
    `AssistantMessageContent` gains `message_type` (the API `type` field);
    `UserMessage` gains `subagent_type` and `task_description`.
  - `ToolUseBlock` gains a typed `caller` (`ToolCaller`).
  - `InitMessage` gains `analytics_disabled` and `product_feedback_disabled`.
  - `ResultMessage` gains `ttft_ms`, `ttft_stream_ms`, and `time_to_request_ms`.
- **Wire-fidelity audit** — `audit_frame` / `assert_fully_wrapped` / `FrameAudit`
  (exported from the crate root and `claude_codes::io`) check that a raw frame
  deserializes, round-trips losslessly, and — for `system` frames — resolves to
  a modeled subtype whose typed view captures every field.
- **`AsyncClient::receive_raw`** — read the next frame as a raw
  `serde_json::Value` before typed parsing, for auditing wire fidelity.
- New `test_cases/subagent_sessions/` captures and a `subagent_wrapping_tests`
  suite that audits every frame (fixtures always; a live subagent run under
  `integration-tests`).

### Changed

- **Tested Claude CLI version** bumped to 2.1.178 — the full integration suite,
  including the new live subagent run, passes against the installed binary.

## [2.1.157] - 2026-06-11

### Changed

- **Tested Claude CLI version** bumped from 2.1.150 to 2.1.170. The full
  integration suite passes against CLI 2.1.172; the pin sits two patch
  versions behind. The newer-than-tested CLI warning now triggers only
  above 2.1.170.

## [2.1.156] - 2026-06-10

### Added

- **`ContentBlock::Fallback`** — typed variant for model-fallback content
  blocks (`{"type": "fallback", "from": {"model": ...}, "to": {"model": ...}}`),
  emitted when the CLI switches the response to a fallback model mid-turn.
  Previously these deserialized as `ContentBlock::Unknown`. The wire shape was
  verified against the claude CLI 2.1.172 binary and real session transcripts;
  it carries no fields beyond the from/to models (no reason field exists).
  New types `FallbackBlock` and `FallbackModel` are exported from the crate
  root and `claude_codes::io`. (#160)

## [2.1.155] - 2026-06-08

### Changed (breaking)

- **`TextBlock::citations`** is now `Vec<Citation>` instead of
  `Vec<serde_json::Value>`. Consumers read typed fields (`citation_type`,
  `url`, `title`, `cited_text`, `document_index`, `document_title`) instead of
  JSON-poking each entry. Unmodeled location fields (start/end indices,
  `encrypted_index`, …) are preserved verbatim in `Citation::extra`, so the
  type round-trips losslessly across all citation shapes.

### Added

- **`Citation`** — typed content-block citation struct (re-exported from
  `claude_codes::io`).

## [2.1.154] - 2026-06-08

### Added

- **`ToolInput::from_named_input(name, input)`** — parses a tool-use `input`
  using the authoritative tool *name* from the `ToolUse` block instead of
  guessing the variant from field shape. Resolves the inherent ambiguity of
  the untagged `Deserialize` impl for structurally-identical inputs — most
  notably `WebSearch` vs `ToolSearch`, both bare `{ "query": String }`, where
  the untagged impl always picked `ToolSearch` and dropped the `WebSearch`
  query. Falls back to `Unknown` on shape mismatch and defers to the untagged
  impl for unmodeled (e.g. MCP) tool names.

## [2.1.153] - 2026-06-08

### Changed (breaking)

- **`ResultMessage::model_usage`** is now
  `Option<BTreeMap<String, ModelUsageEntry>>` instead of
  `Option<serde_json::Value>`. Consumers read typed per-model usage
  (`input_tokens`, `output_tokens`, `cache_read_input_tokens`,
  `cache_creation_input_tokens`, `cost_usd`, `web_search_requests`)
  keyed by model name, instead of poking the `Value`. Unmodeled wire
  keys are preserved in `ModelUsageEntry::extra`.

### Added

- **`ModelUsageEntry`** — typed per-model usage/cost record (re-exported
  from `claude_codes::io`).

## [2.1.152] - 2026-06-08

### Added

- **`CompactBoundaryMessage`** gains optional per-compaction stats:
  `summary: Option<String>` (also accepted under the `content` / `text`
  wire keys), `leaf_message_count: Option<u32>` (also accepted under
  `message_count`), and `duration_ms: Option<u64>`. Consumers can read
  these from the typed struct instead of poking `SystemMessage::data`.

## [2.1.151] - 2026-06-08

### Added

- **Round-trip regression test** — `test_task_messages_roundtrip_through_value`
  pushes `task_started` / `task_progress` / `task_notification` system
  messages through `from_str` → `to_value` → `from_value` → typed accessor,
  covering the proxy/relay path where a dropped or renamed field would
  otherwise surface only as a `None` downstream.

## [2.1.150] - 2026-05-29

### Changed

- Pinned the tested Claude CLI version forward to `2.1.150`. The full integration suite passes against the newer CLI with no protocol changes required.

## [2.1.142] - 2026-05-27

### Changed

- **`ToolPermissionRequest::answer_questions`** — Signature changed from `HashMap<String, String>` (keyed by caller-chosen string) to `&HashMap<usize, String>` (keyed by question index). The previous signature let callers — and the rustdoc example — accidentally key answers by `header` instead of the full question text, which makes the CLI render `"Your questions have been answered: ."` with an empty body and leaves Claude unable to see the choice. The new signature looks each index up in the request's `questions` array and uses `q.question` as the wire key, so the wrong-key footgun is impossible.
- **`answer_questions` now returns `AskUserQuestionResponseError`** instead of a bare `serde_json::Error`, distinguishing `WrongTool` / `ParseInput` / `QuestionIndexOutOfRange { index, total }` failure modes. Also validates that `tool_name == "AskUserQuestion"` before parsing the input.

### Added

- **`AskUserQuestionResponseError`** — Re-exported error type for the helper.

### Migration

If you wired up to the 2.1.141 `answer_questions(HashMap<String, String>)` form, swap to passing a `HashMap<usize, String>` keyed by question index (matches the natural UI shape of "the user picked option X for question 0"). No other callsite changes needed.

## [2.1.141] - 2026-05-17

### Added

- **`ToolPermissionRequest::answer_questions(answers, request_id)`** —
  typed helper for replying to an `AskUserQuestion` permission request.
  Parses `self.input` as `AskUserQuestionInput`, attaches the supplied
  `HashMap<String, String>` answers, and returns a `ControlResponse`
  whose `updatedInput` carries both the original `questions` array AND
  the new `answers`. Eliminates the `"undefined is not an object
  (evaluating 'q.map')"` failure mode in downstream viewers that read
  `tool_use_result.questions` and call `questions.map(...)` — that
  crash happens when the response payload is built by hand and the
  original `questions` are dropped. Existing `allow` / `allow_with`
  continue to work for non-AskUserQuestion approvals.

### Verified

- All 26 integration tests pass against the live Claude CLI, including
  `test_ask_user_question_answered_and_converges` which drives Claude
  through the new helper and confirms the agent's follow-up reply
  references the chosen answer (`"You picked Blue."`).

## [2.1.140] - 2026-05-13

### Added

- **`UserMessage.tool_use_result`** — `Option<serde_json::Value>` capturing the top-level structured tool result the CLI emits alongside `tool_result` content blocks (e.g. `{ questions, answers }` for `AskUserQuestion`, `{ stdout, stderr, exit_code }` for `Bash`). Previously dropped during deserialization, which broke proxies relaying user messages to viewers that read the field.
- **`UserMessage.timestamp`** — `Option<String>` capturing the CLI's ISO-8601 timestamp on echoed tool results.
- **`UserMessage::tool_use_result_as<T>()`** — Typed accessor for parsing `tool_use_result` into a caller-specified type when the tool is known (e.g. `tool_use_result_as::<AskUserQuestionInput>()`).
- **Integration regression test** — `test_ask_user_question_answered_and_converges` drives the full AskUserQuestion round-trip through the permission control protocol and asserts `tool_use_result` survives end-to-end without information loss.

### Changed

- Updated `TESTED_VERSION` to `2.1.140`

## [2.1.117] - 2026-04-15

### Added

- **`ContentBlock::Unknown(Value)`** — Fallback variant for forward compatibility with new content block types from the CLI. Prevents deserialization failures when encountering unknown block types (#104)
- **Typed content block variants** — `ServerToolUse`, `WebSearchToolResult`, `CodeExecutionToolResult`, `McpToolUse`, `McpToolResult`, `ContainerUpload` for the new server-side and MCP content blocks emitted by CLI 2.1.117 (#105, #106, #107)
- **`TextBlock.citations`** — `Vec<Value>` field for web search citations on text blocks (#108)
- **`ResultMessage` fields** — `api_error_status`, `stop_reason`, `terminal_reason`, `fast_mode_state`, `model_usage` (#109)
- **`UsageInfo` fields** — `cache_creation`, `inference_geo`, `iterations`, `speed` (#110)
- **`ServerToolUse.web_fetch_requests`** — Tracks web fetch request count (#110)
- **`InitMessage` fields** — `uuid`, `memory_paths`, `fast_mode_state` (#111)
- **`PluginInfo.source`** — Plugin registry source identifier (#111)
- **`AssistantMessageContent` fields** — `stop_details`, `context_management` (#112)
- **`AssistantUsage.inference_geo`** — Inference geography field (#112)
- **`UserMessage` fields** — `parent_tool_use_id`, `uuid` (#113)
- **New `ToolInput` types** — `MultiEdit`, `LS`, `NotebookRead`, `ScheduleWakeup`, `ToolSearch` with typed structs (#114)
- **`ClaudeCliBuilder.max_thinking_tokens()`** — Builder method and `CliFlag::MaxThinkingTokens` for extended thinking control (#115)
- **`ContentBlock.block_type()`** and **`ContentBlock.is_unknown()`** — Helper methods for content block introspection

### Changed

- Updated `TESTED_VERSION` to `2.1.117`
- `UsageInfo` fields now use `#[serde(default)]` for robustness
- `ServerToolUse` now derives `Default`

## [2.1.53] - 2026-03-17

### Added

- **Binary path resolution via `which`** — `ClaudeCliBuilder::spawn()`, `spawn_sync()`, and `build_command()` now resolve non-absolute binary paths using `which` at spawn time, producing a clear `BinaryNotFound` error instead of an opaque OS "file not found" (#102)
- **`Error::BinaryNotFound`** — New error variant for when the CLI binary isn't found on PATH

### Changed

- **`spawn_sync()` return type** — Now returns `crate::error::Result<Child>` instead of `std::io::Result<Child>` for consistent error handling
- **`build_command()` return type** — Now returns `Result<Command>` instead of `Command` to surface binary resolution errors

## [2.1.52] - 2026-03-12

### Added

- **`SDKControlInterruptRequest`** — Typed struct for the `{ "subtype": "interrupt" }` SDK control message, used to gracefully stop a running Claude session without killing the process
- **`ClaudeInput::interrupt()`** — Constructor for creating interrupt messages
- **`AsyncClient::interrupt()`** and **`SyncClient::interrupt()`** — Convenience methods to send an interrupt to the CLI subprocess

## [2.1.51] - 2026-02-27

### Changed

- **`Error::Deserialization`** now wraps `ParseError` instead of `String`, giving callers structured access to the raw input line, parsed JSON value, and error message
- **`ParseError`** gains a `raw_line: String` field containing the exact stdout line (works even when the input isn't valid JSON)

## [2.1.50] - 2026-02-27

### Fixed

- `RateLimitInfo.resets_at` and `RateLimitInfo.rate_limit_type` are now `Option` — Claude CLI can omit these fields in `rate_limit_event` messages with `status: "allowed"`

## [2.1.49] - 2026-02-25

### Changed

- **`SystemSubtype`** — enum replacing `String` for system message subtypes (`init`, `api_error`, etc.)
- **`ApiErrorType`** — enum replacing `String` for API error types (`authentication_error`, `overloaded_error`, etc.)
- **`RateLimitStatus`** — enum replacing `String` for rate limit statuses (`rate_limited`, `rate_limit_cleared`)
- **`RateLimitWindow`** — enum replacing `String` for rate limit windows (`minutely`, `daily`, etc.)
- **`PermissionType`** — enum replacing `String` for permission types (`addRules`, `setMode`)
- **`PermissionDestination`** — enum replacing `String` for permission destinations (`session`, `project`)
- **`PermissionBehavior`** — enum replacing `String` for permission behaviors (`allow`, `deny`)
- **`PermissionModeName`** — enum replacing `String` for permission mode names (`acceptEdits`, `bypassPermissions`)
- **`MessageRole`** — enum replacing `String` for message roles (`user`, `assistant`)
- **`CompactionTrigger`** — enum replacing `String` for compaction triggers (`auto`, `manual`)
- **`StopReason`** — enum replacing `String` for stop reasons (`end_turn`, `max_tokens`, `tool_use`)
- **`TodoStatus`** — enum replacing `String` for todo statuses (`pending`, `in_progress`, `completed`)
- **`OverageStatus`** — enum replacing `String` for overage billing status (`allowed`, `rejected`)
- **`OverageDisabledReason`** — enum replacing `String` for overage disabled reason (`org_level_disabled`, `out_of_credits`)
- **`ImageSourceType`** — enum replacing `String` for image encoding type (`base64`)
- **`MediaType`** — enum replacing `String` for image MIME types (`image/jpeg`, `image/png`, `image/gif`, `image/webp`)
- **`GrepOutputMode`** — enum replacing `String` for grep output mode (`content`, `files_with_matches`, `count`)
- **`SubagentType`** — enum replacing `String` for task subagent types (`Bash`, `Explore`, `Plan`, `general-purpose`)
- **`NotebookCellType`** — enum replacing `String` for notebook cell types (`code`, `markdown`)
- **`NotebookEditMode`** — enum replacing `String` for notebook edit modes (`replace`, `insert`, `delete`)
- **`ApiKeySource`** — enum replacing `String` for API key source in init messages (`none`)
- **`OutputStyle`** — enum replacing `String` for output style in init messages (`default`)
- **`InitPermissionMode`** — enum replacing `String` for permission mode in init messages (`default`)
- **`StatusMessageStatus`** — enum replacing `String` for status message status (`compacting`)

All enums include an `Unknown(String)` fallback variant for forward compatibility, plus `as_str()`, `Display`, and `From<&str>` implementations.

### Breaking

- Struct fields that were `String` are now typed enums — callers using `.as_deref()`, string comparisons, or `.to_string()` on these fields need to update to use the enum variants or `.as_str()` method

## [2.1.47] - 2026-02-24

### Added

- **`TaskStartedMessage`** — Typed struct for `task_started` system messages emitted when a background task (agent or bash) begins
- **`TaskProgressMessage`** — Typed struct for `task_progress` system messages with tool name, description, and cumulative usage stats
- **`TaskNotificationMessage`** — Typed struct for `task_notification` system messages emitted when a background task completes or fails
- **`TaskUsage`** — Cumulative usage statistics (`duration_ms`, `tool_uses`, `total_tokens`)
- **`TaskType`** enum — `LocalAgent` or `LocalBash`
- **`TaskStatus`** enum — `Completed` or `Failed`
- **`SystemMessage` helpers** — `is_task_started()`, `is_task_progress()`, `is_task_notification()` and corresponding `as_task_*()` methods

## [2.1.46] - 2026-02-20

### Fixed

- **`RateLimitInfo.overage_status`** now `Option<String>` — `allowed_warning` events omit this field, previously causing deserialization failures

### Added

- **`RateLimitInfo.utilization`** — `Option<f64>` capturing rate limit usage (0.0–1.0)

## [2.1.45] - 2026-02-18

### Added

- **Expanded `InitMessage` fields** - Added typed fields for `slash_commands`, `agents`, `plugins`, `skills`, `claude_code_version`, `api_key_source`, `output_style`, and `permission_mode`
- **`PluginInfo` struct** - Typed representation of plugin entries with `name` and `path` fields
- **`allow_recursion()` on `ClaudeCliBuilder`** - Enables spawning Claude CLI from within a Claude Code session by unsetting `CLAUDECODE` env var
- **`/clear` integration test** - Verifies session ID resets after `/clear` command

### Changed

- Updated `TESTED_VERSION` to `2.1.47`
- All integration tests now use `allow_recursion()` for reliable execution inside Claude Code sessions

## [2.1.20] - 2026-02-17

### Added

- **`RateLimitEvent` and `RateLimitInfo`** - Support for `rate_limit_event` messages from Claude CLI
- `ClaudeOutput::RateLimitEvent` variant with `is_rate_limit_event()` and `as_rate_limit_event()` helpers

## [2.1.19] - 2026-02-17

### Added

- **`CliFlag` enum** - Comprehensive enum covering all 41 Claude CLI flags for building launcher UIs and advanced configuration
- **`InputFormat` and `OutputFormat` enums** - Typed representations of `--input-format` and `--output-format` options
- **`PermissionMode::Delegate` and `PermissionMode::DontAsk`** - Added missing permission mode variants
- `CliFlag::as_flag()` - Returns the CLI flag string (e.g., `"--add-dir"`)
- `CliFlag::to_args()` - Converts a flag + value into CLI argument strings
- `CliFlag::all_flags()` - Returns all flag names with descriptions for enumeration

## [2.1.18] - 2026-01-26

### Changed

- Increase stdout buffer from 8KB to 10MB to handle large JSON messages

## [2.1.17] - 2026-01-25

### Added

- **Permission struct for "remember this decision" support** - New typed API for building permission responses that support Claude Code's "remember this decision" functionality.

  When responding to tool permission requests, you can now grant permissions so similar actions won't require approval in the future:

  ```rust
  use claude_codes::{ToolPermissionRequest, Permission};

  fn handle_permission(req: &ToolPermissionRequest, request_id: &str) -> ControlResponse {
      // Allow and remember this specific command for the session
      req.allow_and_remember(
          vec![Permission::allow_tool("Bash", "npm test")],
          request_id,
      )
  }
  ```

  Or accept Claude's suggested permission:

  ```rust
  // Use the first permission suggestion if available
  let response = req.allow_and_remember_suggestion(request_id)
      .unwrap_or_else(|| req.allow(request_id));
  ```

  Available `Permission` constructors:
  - `Permission::allow_tool(tool_name, rule_content)` - Allow a specific tool with a pattern (session-scoped)
  - `Permission::allow_tool_with_destination(tool_name, rule_content, destination)` - Allow with custom scope ("session" or "project")
  - `Permission::set_mode(mode, destination)` - Set a permission mode like "acceptEdits"
  - `Permission::from_suggestion(suggestion)` - Convert a `PermissionSuggestion` to a `Permission`

  **Migration from `allow_with_permissions`:**

  Before (manual JSON conversion):
  ```rust
  // Old approach - manually convert to JSON
  let perms_json: Vec<serde_json::Value> = suggestions
      .iter()
      .filter_map(|p| serde_json::to_value(p).ok())
      .collect();
  ControlResponse::from_result(
      &request_id,
      PermissionResult::allow_with_permissions(input, perms_json)
  )
  ```

  After (typed API):
  ```rust
  // New approach - use typed Permission API
  let permissions: Vec<Permission> = suggestions
      .iter()
      .map(Permission::from_suggestion)
      .collect();
  req.allow_and_remember(permissions, request_id)
  ```

- **`decision_reason` and `tool_use_id` fields on `ToolPermissionRequest`** - These fields are now exposed for consumers that need them when building custom permission handling logic. The `tool_use_id` is particularly useful for correlating permission requests with tool uses in the message stream.

- **`ClaudeOutput::Error` variant for Anthropic API errors** - New variant to capture API errors (500, 529 overloaded, rate limits, etc.) that were previously unparsed.

  ```rust
  use claude_codes::ClaudeOutput;

  match output {
      ClaudeOutput::Error(err) => {
          if err.is_overloaded() {
              println!("API overloaded, retrying...");
          } else if err.is_rate_limited() {
              println!("Rate limited: {}", err.error.message);
          } else {
              println!("API error: {}", err.error.message);
          }
      }
      // ... handle other variants
  }
  ```

  Helper methods on `AnthropicError`:
  - `is_overloaded()` - HTTP 529 overloaded error
  - `is_server_error()` - HTTP 500 server error
  - `is_rate_limited()` - HTTP 429 rate limit error
  - `is_authentication_error()` - HTTP 401 auth error
  - `is_invalid_request()` - HTTP 400 invalid request

  Helper methods on `ClaudeOutput`:
  - `is_api_error()` - Check if this is an error variant
  - `as_anthropic_error()` - Get the error if this is one

### Changed

- `allow_with_permissions` method documentation clarified to note it takes raw `Vec<Value>`. For type safety, prefer the new `allow_and_remember` method.

## [2.1.16] - 2026-01-22

### Fixed

- Fixed `PermissionSuggestion` struct to correctly handle both `setMode` and `addRules` suggestion types from Claude CLI.

## [2.1.15] - 2026-01-21

### Added

- Re-export `ContentBlock`, `ToolUseBlock`, and other io types at crate root
- Typed `UsageInfo` on `AssistantMessage` with `input_tokens`, `output_tokens`, and `cache_creation_input_tokens`
- Typed `PermissionSuggestion` for `ToolPermissionRequest` permission suggestions
- Typed `PermissionDenial` for `ResultMessage` permission denial details
- Typed `StatusDetails` and `SuggestionMetadata` for system status responses
- Typed system message subtypes (`init`, `status`, `compact_boundary`)
- Typed `ToolInput` definitions for all built-in tools (Read, Write, Edit, Bash, Glob, Grep, etc.)
- Helper methods on `ClaudeOutput`: `is_assistant_message()`, `is_result()`, `is_error()`, `as_assistant()`, `as_result()`, `as_system()`, `text_content()`, `tool_uses()`
- `errors` field on `ResultMessage` for capturing error details
- Real production message test captures for structured content

## [2.1.4] - 2026-01-10

### Added

- Tool approval protocol support with interactive permission request/response handling
- `ControlRequest` and `ControlResponse` types for the tool permission workflow
- `ToolPermissionRequest` with `allow()`, `deny()`, and `allow_with_permissions()` helpers

### Fixed

- `--session-id` flag no longer incorrectly added when using `--resume` or `--continue`

## [2.1.3] - 2026-01-09

### Changed

- Version sync with Claude CLI 2.1.3
- WASM support documentation for the `types` feature with `wasm32-unknown-unknown`

## [2.0.76] - 2026-01-04

### Changed

- Version sync with Claude CLI 2.0.76
- Fixed content deserialization to handle both string and array formats

### Fixed

- Removed debug `eprintln` statements from output

## [0.3.0] - 2025-08-30

### Changed

- **Breaking:** Reorganized to feature-based architecture with `sync-client`, `async-client`, and `types` features
- **Breaking:** Switched logging from `tracing` to `log` crate
- **Breaking:** Client modules moved to top-level `client_sync` and `client_async`
- `types` feature enables WASM-compatible type definitions without client dependencies

## [0.2.1] - 2025-08-28

### Added

- `ping()` method on `AsyncClient` and `SyncClient` for connectivity testing
- `parse_json_tolerant()` to handle ANSI escape codes in responses
- Integration tests for slash commands (`/help`, `/status`, `/cost`)

### Fixed

- `num_turns` field type to handle `-1` for slash commands

## [0.2.0] - 2025-08-26

### Added

- Image content block support (JPEG, PNG, GIF, WebP) with `user_message_with_image()`
- OAuth token and API key environment variable support (`CLAUDE_CODE_OAUTH_TOKEN`, `ANTHROPIC_API_KEY`)

### Changed

- **Breaking:** Session IDs use `UUID` type instead of `String`
- **Breaking:** `ClaudeInput::user_message()` now requires `UUID` for session_id

## [0.1.2] - 2025-08-25

### Added

- `resume_session()` and `resume_session_with_model()` on both clients
- Environment variable support for OAuth tokens and API keys
- Validation warnings for incorrect token/key prefixes

## [0.1.1] - 2025-08-25

### Added

- Session UUID versioning to track Claude Code sessions
- `session_uuid()` getter on both `AsyncClient` and `SyncClient`
- CLI builder generates UUID v4 by default

## [0.1.0] - 2025-08-25

### Added

- Comprehensive crate and module-level documentation
- `AsyncClient` and `SyncClient` API docs

### Changed

- Simplified licensing to Apache-2.0 only

## [0.0.5] - 2025-08-24

### Added

- `AsyncClient` with `query()` and `query_stream()` methods
- `SyncClient` for non-async contexts
- `ResponseStream` and `ResponseIterator` for iterative response processing
- `ResultMessage` with `UsageInfo` for token usage and cost tracking
- Claude CLI version checking with compatibility warnings
- Example programs: `basic_repl`, `async_client`, `sync_client`

### Changed

- Message types restructured to match Claude Code SDK (System, User, Assistant, Result)

## [0.0.1] - 2025-08-23

### Added

- Initial implementation of `claude-codes` crate
- `ClaudeInput` and `ClaudeOutput` enums for typed protocol messages
- `ClaudeCliBuilder` for streaming JSON mode
- Interactive testing binary for protocol debugging
- Automatic test case capture for failed deserializations
