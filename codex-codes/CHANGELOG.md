# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.146.0] - 2026-07-31

### Changed

- **Tested Codex CLI version** bumped to 0.146.0. Verified live: the full
  `codex-codes` integration suite passes against the installed binary with
  `--test-threads=1`.

### Contributors

- Thanks to @ozitrance for the Codex-side `RawAsyncClient` contribution and
  Windows `CREATE_NO_WINDOW` process-launch support that landed in this release
  train.

## [0.145.1] - 2026-07-30

### Added

- `RawAsyncClient` for newline-framed, undecoded access to the Codex app-server
  protocol.
- **`threadSection/list` and `thread/section/move` client requests** — new
  method constants with generated `ThreadSection`,
  `ThreadSectionListParams`/`Response`, and
  `ThreadSectionMoveParams`/`Response` types.
- **New generated definitions**: `PluginDisabledReason`, `ThreadSearchSortKey`,
  `ExternalAgentConfigImportHistoryRecordSuccessParams` /
  `RecordTypeResultParams`; additive fields across `Thread`,
  `ThreadListParams`, `ThreadMetadataUpdateParams`, `PlanType`, and the
  plugin/import types.

### Changed

- On Windows, Codex app-server launches and version checks now use
  `CREATE_NO_WINDOW` to avoid opening console windows.
- **Re-snapshotted the Codex app-server schemas** against `openai/codex@main`
  (resolves the codex-schema-drift report, issue #241); both snapshots are
  byte-identical to upstream and schema coverage is 172/172 (100%).

## [0.145.0] - 2026-07-27

Version jumps 0.143.6 → 0.145.0 to re-align with the tested Codex CLI
version, per the crate's versioning convention.

### Changed

- **Tested Codex CLI version** bumped to 0.145.0. Verified live: the full
  integration suite passes against the installed binary (protocol-level
  suite in one environment, model-turn tests under live auth in another;
  one startup flake observed under sandboxed parallel runs — run the live
  suite with `--test-threads=1`).

### Added

- `AppServerBuilder::env` and `envs` for configuring the app-server process
  environment.
- `AppServerBuilder::build_command` and `build_command_sync`, plus
  `AsyncClient::new` and `SyncClient::new`, for customizing and spawning the
  app-server separately from client construction.

### Changed

- Shared dependencies and lint policy moved to the workspace root: `serde`,
  `serde_json`, `thiserror`, `tokio`, `log`, `which`, and dev `env_logger` /
  `jsonschema` are now `{ workspace = true }`, and the crate opts into
  `[workspace.lints]` (`unsafe_code = "deny"`). No dependency version changes.

## [0.143.6] - 2026-07-25

Re-snapshot of the app-server schema from `openai/codex@main`, resolving the
nightly drift report (#232). Snapshots are byte-identical to upstream again
and schema coverage is 170/170 (100%).

### Added

- **`externalAgentConfig/import/recordHistory` client request** — new method
  constant (`methods::EXTERNALAGENTCONFIG_IMPORT_RECORDHISTORY`) with
  generated `ExternalAgentConfigImportHistoryRecordParams`/`Response` types.
- **`BrowserUseRequirements`** definition (carried on `ConfigRequirements`
  as the new optional `browser_use` field).
- Additive fields across `AppToolSummary`, `ConfigBatchWriteParams`,
  `ExternalAgentConfigImportHistory`, `ExternalAgentConfigImportParams`,
  `PlanType`, `PluginShareContext`, `PluginShareSaveResponse`,
  `SkillInterface`, and `ThreadItem`.

## [0.143.5] - 2026-07-22

Re-snapshot of the app-server schema from `openai/codex@main`, resolving the
nightly drift report (#199). Snapshots are byte-identical to upstream again
and schema coverage is back to 100%.

### Added

- **`app/read` and `app/installed` client requests** — new method constants
  (`methods::APP_READ`, `methods::APP_INSTALLED`) with generated
  `AppsReadParams`/`AppsReadResponse` and
  `AppsInstalledParams`/`AppsInstalledResponse` types, plus supporting
  `ConnectorMetadata`, `InstalledApp`, and `AppToolSummary` definitions.
- **New generated definitions**: `CodexResponseHandoffMode`,
  `FeedbackRequirements`, `PathUri`, `ThreadRealtimeInitialItem`.
- Additive fields across `Account`, `ConfigRequirements`,
  `ConfiguredHookHandler`, `ContentItem`, `HookEventName`, `HookMetadata`,
  `InputModality`, `ManagedHooksRequirements`, `PluginListParams`,
  `PluginSummary`, `UserInput`, and the external-agent-config params.

### Changed

- **Breaking**: `ReviewDecision::Denied` is now a struct variant carrying a
  required `rejection: String` (wire shape
  `{"decision":{"denied":{"rejection":...}}}`). The
  `ExecCommandApprovalResponse::denied()` and
  `ApplyPatchApprovalResponse::denied()` constructors now take the rejection
  message as an argument.

### Removed

- **Breaking**: `AmazonBedrockCredentialSource` — removed upstream.

## [0.143.4] - 2026-07-16

Combines the `agent-portal` audit ergonomics (originally staged as an
unreleased 0.143.3) with a fresh upstream schema re-snapshot.

### Added

- **`ServerMessage::from_json_str` / `from_value`** — parse a raw app-server
  frame into a typed [`ServerMessage`] without a live client, reusing the same
  dispatch the client runs. Server-initiated notifications and requests decode
  to their typed variants (unknown methods route to `Unknown`); a JSON-RPC
  response/error frame returns `Error::Protocol`. Unblocks replay, recovery,
  and fixture-based tests downstream. (#209)
- **`Notification::turn_id() -> Option<&str>`** — read the turn id a
  notification is scoped to straight from its typed fields (`turn.id` for the
  turn-lifecycle notifications, the flat `turnId` for the streaming ones),
  instead of round-tripping through `into_envelope()` and poking
  `serde_json::Value`. This is the id `turn/interrupt` needs. (#205)
- **`Notification::thread_item() -> Option<&ThreadItem>`** — typed access to the
  item carried by `item/started` / `item/completed`, so event adapters don't
  reserialize items back into JSON. (#213)
- **`Default` for `ThreadStartParams`, `TurnStartParams`, `ThreadResumeParams`,
  and `ThreadForkParams`** — construct empty request params with
  `Params::default()` instead of spelling out every `None` field. (#206)
- **`accept()` / `decline()` / `cancel()` / `approved()` / `denied()` /
  `abort()` constructors** on the approval-response payloads
  (`FileChangeRequestApprovalResponse`,
  `CommandExecutionRequestApprovalResponse`, `ExecCommandApprovalResponse`,
  `ApplyPatchApprovalResponse`), so callers respond to approval requests
  without hand-rolling `serde_json` `decision` objects. (#212)
- **`Notification::ThreadEnvironmentConnected` / `ThreadEnvironmentDisconnected`**
  — typed variants for the new `thread/environment/connected` and
  `thread/environment/disconnected` server notifications (both carry
  `EnvironmentConnectionNotification`). Schema coverage back to 100%.

### Changed

- **Re-snapshotted the Codex app-server schemas** against `openai/codex@main`;
  both `codex_app_server_protocol{,.v2}.schemas.json` are byte-identical to
  upstream again and the typed structs/samples were regenerated. Pulls in 6 new
  definitions (`EnvironmentConnectionNotification`, `ScheduledTaskSummary` /
  `ScheduledTaskSchedule` / `ScheduledTaskWeekday`,
  `ExternalAgentImportedConnectorCandidate` / `ExternalAgentImportedConnectorSource`),
  additive token/memory fields (e.g. `cacheWriteInputTokens` on
  `TokenUsageBreakdown`, `memory` / `scheduledTasks` / `spendControlReached`),
  and drops the upstream-removed `templateId` from `McpToolCallAppContext`.

## [0.143.2] - 2026-07-11

### Added

- **`CodexModel`** — convenience enum for the Codex model catalog, keyed by
  human-friendly names (`Gpt56Sol`, `Gpt56Terra`, `Gpt56Luna`, `Gpt55`,
  `Gpt54`, `Gpt54Mini`, `Gpt52`, `CodexAutoReview`), taken from
  `openai/codex@main`'s bundled `models-manager/models.json` (2026-07-11).
  `cli_arg()` returns the slug for `codex -m` / `ThreadStartParams.model`,
  `display_name()` the catalog's display name, and `Custom(String)` passes
  unknown slugs through. Re-exported at the crate root.

## [0.143.1] - 2026-07-10

### Changed

- **Re-snapshotted the Codex app-server schemas** against `openai/codex@main`
  (resolves #195). `LoginAccountParams` and `LoginAccountResponse` gained an
  `amazonBedrock` account variant (`apiKey` + `region` on params). Regenerated
  `protocol_generated/{mod,types,samples}.rs`; schema coverage remains 165/165
  (100%) and the drift check reports byte-identical.

## [0.143.0] - 2026-06-27

### Added

- **`McpServerStartupFailureReason`** enum (`reauthenticationRequired`) and a new
  `failure_reason` field on `McpServerStatusUpdatedNotification`, distinguishing
  why an MCP server failed to start.
- **`ModelsRequirements`** / **`NewThreadModelDefaults`** types and a `models`
  field on `ConfigRequirements`, carrying default model / reasoning-effort /
  service-tier for new threads.
- **`PluginSource::Npm`** variant (`package`, optional `registry` / `version`)
  for npm-sourced plugins.
- New optional fields on existing types: `thread_id` on
  `McpServerOauthLoginCompletedNotification` and `McpServerOauthLoginParams`;
  `action_name`, `app_name`, and `template_id` on `McpToolCallAppContext`;
  `last_turn_id` on `ThreadForkParams`.

### Changed

- Re-snapshotted `codex_app_server_protocol{,.v2}.schemas.json` from
  `openai/codex@main` (commit `6509f314`, 2026-06-26), regenerating the typed
  structs (resolves the codex-schema-drift report, issue #172). Schema coverage
  remains 165/165 (100%).
- Bumped tested Codex CLI version to `0.143.0`.
- Enriched the crate description, `keywords`, and `categories` for crates.io
  discoverability (agent / Codex / OpenAI / async terms).

## [0.142.0] - 2026-06-24

### Added

- **`Notification::ExternalAgentConfigImportProgress`** — typed variant for the
  new `externalAgentConfig/import/progress` server notification, reporting
  per-item-type successes and failures while an external-agent config import
  runs.
- **`Notification::ModelSafetyBufferingUpdated`** — typed variant for the new
  `model/safetyBuffering/updated` server notification (transient, not persisted
  to rollout history; carries `showBufferingUi` and an optional `fasterModel`).
- **`account/rateLimitResetCredit/consume` request** — `ConsumeAccountRateLimitResetCreditParams`
  / `ConsumeAccountRateLimitResetCreditResponse` for redeeming an earned
  rate-limit reset credit (idempotency-key based; refetch `account/rateLimits/read`
  afterward).
- **`account/workspaceMessages/read` request** — `GetWorkspaceMessagesResponse`
  / `WorkspaceMessage` for fetching active ChatGPT workspace messages.
- **`externalAgentConfig/import/readHistories` request** —
  `ExternalAgentConfigImportHistoriesReadResponse` for reading prior
  external-agent config import histories.

### Changed

- Re-snapshotted `codex_app_server_protocol{,.v2}.schemas.json` from
  `openai/codex@main` (commit `134646ef`, 2026-06-24), regenerating the typed
  structs (resolves the codex-schema-drift report, issue #166). This pulls in
  25 new definitions and 36 changed ones across the v2 bundle, including richer
  external-agent config import progress/history reporting (per-type
  success/failure), workspace messages, transient model safety-buffering
  notifications, remote-control and multi-agent type additions, and expanded
  rate-limit reset-credit support. Schema coverage is back to 165/165 (100%).
- Bumped tested Codex CLI version to `0.142.0`.

## [0.137.4] - 2026-06-11

### Added

- **`thread/delete` request** — `thread_delete()` on both `AsyncClient` and
  `SyncClient`, with `ThreadDeleteParams` / `ThreadDeleteResponse`.
- **`Notification::ThreadDeleted`** — typed variant for the new
  `thread/deleted` server notification.
- **`ThreadItem::SubAgentActivity`** item variant and the
  `SubAgentActivityKind` enum (started / interacted / interrupted).
- **`AuthMode::BedrockApiKey`** auth-mode variant.

### Changed (breaking)

- **`ThreadSource`** is now an open transparent newtype `ThreadSource(pub
  String)` instead of a closed enum, matching upstream's move to a free-form
  string. Consumers matching on the old `User`/`Subagent` variants should
  compare against the string value instead.
- Re-snapshotted `codex_app_server_protocol{,.v2}.schemas.json` from
  `openai/codex@main` (commit `f4278010`, 2026-06-11), regenerating the typed
  structs (resolves the codex-schema-drift report). This is a strict superset
  of the rust-v0.139.0 release schema — everything in CLI 0.139.0 is modeled,
  plus the not-yet-released thread/delete, SubAgentActivity, ThreadSource,
  and BedrockApiKey changes. Schema coverage is back to 160/160 (100%).

## [0.137.3] - 2026-06-08

### Changed

- Release/version bump to publish alongside `claude-codes`; no library or
  schema changes since 0.137.2.

## [0.137.2] - 2026-06-08

### Changed

Re-snapshotted the Codex app-server schema from `openai/codex@main`; both
snapshots are byte-identical to upstream again. The only wire change:
`McpServerStatusUpdatedNotification` gains an optional
`thread_id: Option<String>` field. Schema coverage stays at 100%.

## [0.137.1] - 2026-06-08

### Added

Typed client helpers mirroring `thread_start`, so callers no longer need to
drop down to the low-level `request::<P, R>` primitive:

- `AsyncClient::thread_resume` / `SyncClient::thread_resume` (`thread/resume`)
- `AsyncClient::thread_fork` / `SyncClient::thread_fork` (`thread/fork`)

## [0.137.0] - 2026-06-07

### Changed (breaking)

Re-snapshotted the Codex app-server schema from `openai/codex@main` (Codex
CLI 0.137.0) and regenerated every wire type. The snapshot is now
byte-identical to upstream. 31 definitions were added, 4 removed
(`PermissionProfile`, `PermissionProfileFileSystemPermissions`,
`PermissionProfileNetworkPermissions`, `ProfileV2`), and 40 definition
bodies changed.

- `TurnStartParams` gains a `client_user_message_id: Option<String>` field;
  struct-literal construction must supply it.

### Added

Modeled the new protocol methods so schema coverage stays at 100%:

- Client requests: `account/usage/read`, `permissionProfile/list`,
  `plugin/installed`, `skills/extraRoots/set`, `thread/goal/get`,
  `thread/goal/set`, `thread/goal/clear`.
- Notifications: `thread/settings/updated` (`Notification::ThreadSettingsUpdated`)
  and `turn/moderationMetadata` (`Notification::TurnModerationMetadata`).

## [0.129.3] - 2026-05-17

### Changed (breaking)

The schema-driven codegen now owns every wire type. The hand-written
shadows in `protocol.rs` that pre-dated the codegen are gone, and the
allowlist mechanism that exempted them is gone with them. Names that
diverged from upstream now match what the schema defines, and consumers
move to those names. Notable renames:

- `CommandExecutionApprovalParams`/`Response` → `CommandExecutionRequestApprovalParams`/`Response`
- `FileChangeApprovalParams`/`Response`       → `FileChangeRequestApprovalParams`/`Response`
- `CommandApprovalDecision`                   → `CommandExecutionApprovalDecision`
- `CmdOutputDeltaNotification`                → `CommandExecutionOutputDeltaNotification`
- `ReasoningDeltaNotification`                → `ReasoningSummaryTextDeltaNotification`
- `McpServerStartupStatusUpdatedNotification` → `McpServerStatusUpdatedNotification`
- `RateLimits`                                → `RateLimitSnapshot`
- `TokenUsage`                                → `ThreadTokenUsage`
- `TokenCounts`                               → `TokenUsageBreakdown`

Field renames worth flagging:

- Approval-params id field `call_id`              → `item_id`
- `TurnStartParams.reasoning_effort`              → `effort`
- Several params types gained additional optional fields that exist
  upstream (`started_at_ms`, `grant_root`, `request_attestation`,
  `command_actions`, `proposed_execpolicy_amendment`, etc.).

`ThreadStartParams::default()` no longer exists; construct an empty
params payload via `serde_json::from_value(serde_json::json!({}))?` (or
list every `Option<…>` field explicitly).

`ThreadStartResponse::thread_id()` removed; access `.thread.id` directly.

`ThreadItem` now uses upstream's inline struct-variant shape
(`ThreadItem::CommandExecution { command, .. }`) rather than the
tuple-variant shape (`ThreadItem::CommandExecution(item)`). The
exec-protocol JSONL parser still uses the original tuple-variant layout
via `codex_codes::io::items::ThreadItem` (the `io` module is now
`pub mod`).

### Added

- **Codegen handles every schema shape** — extended `scripts/codegen_protocol.py`
  with handlers for bare-string newtypes, `oneOf` of pure string enums
  (single- or multi-value), `oneOf` mixing string enums with single-key
  object wrappers, `oneOf` of objects discriminated on a non-`type`
  key (e.g. `kind`), and top-level `anyOf` (untagged Rust enums). The
  number of opaque `pub struct Foo(pub Value)` fallback stubs in the
  generated output dropped from 28 to 0.
- **Schema-required fields tolerate missing wire payloads** — required
  fields whose Rust type already implements `Default` (`String`, `i64`,
  `bool`, `Vec`, `Option`, `Value`, `BTreeMap`) gain `#[serde(default)]`
  so codex's omit-when-empty behavior round-trips without losing types.
  Required fields whose type isn't `Default`-able stay strict.
- **`test_async_client_writes_compilable_quicksort`** — live integration
  test that drives the agent through writing `quicksort.rs`, handling
  every approval request, then verifying the produced source compiles
  with `rustc --edition 2021`.

### Removed

- The `HAND_WRITTEN` allowlist in `scripts/codegen_protocol.py` is gone.
  Any type that appears in the upstream schema is emitted by the codegen;
  any type that doesn't lives in `crate::io` or `crate::jsonrpc`.
- All hand-written wire-type definitions in `protocol.rs`. The module is
  now a re-export shim plus the JSON-RPC method-name constants.
- One stale unit test (`parse_error_carries_method_and_params_for_server_request_with_missing_field`)
  whose premise (missing required field is a deserialization error) no
  longer matches the codegen's permissive treatment of schema-required
  fields.

## [0.129.2] - 2026-05-16

### Added

- **`AppServerBuilder::config_override(key, value)`** — repeatable; appends a `-c key=value` *global* codex flag (placed before `app-server` since `-c` is parsed as a global option, not a subcommand arg). Closes [#135](https://github.com/meawoppl/rust-code-agent-sdks/issues/135). Unblocks consumers like agent-portal that need to pass e.g. `("sandbox_mode", "workspace-write")` or `("approval_policy", "on-request")` at spawn time — previously the only way to do this was to fork the crate or shell out around it.
- **`AppServerBuilder::extra_args(args)`** — appends raw additional args *after* `--listen stdio://` so they land as `app-server` subcommand args. The seam for any flag the SDK doesn't model yet (`--strict-config`, future `--session-source app-server`, etc.).

Both follow the existing `ClaudeCliBuilder` patterns in the sibling crate: `key: K, value: V` with `K: Into<String>`, `V: Into<String>` for the keyed variant; `<I, S: Into<String>>` for the iterable variant.

Values are passed to codex unparsed — codex tries TOML, falls back to raw string. Caller is responsible for any quoting/escaping (e.g. arrays: `r#"["disk-full-read-access"]"#`).

## [0.129.1] - 2026-05-15

### Added

- **Schema-driven codegen pipeline** — `scripts/codegen_protocol.py` reads the upstream JSON Schema bundles (`codex_app_server_protocol{,.v2}.schemas.json`), walks every reachable definition from `ServerNotification.oneOf`, `ClientRequest.oneOf`, and `ServerRequest.oneOf`, and emits fully-typed Rust structs / enums + a per-method sample registry into `src/protocol_generated/`.
- **`src/protocol_generated/types.rs`** — ~4.5k lines, hundreds of typed structs/enums for every wire type reachable from any modeled method. Re-exported as part of `codex_codes::protocol`.
- **`src/protocol_generated/samples.rs`** — one minimal-valid JSON sample per JSON-RPC method, used by the scorecard to assert each typed struct matches the schema's params definition.
- **ServerRequest dispatch expanded to all 10 approval-flow methods** — adds `ToolRequestUserInput`, `McpServerElicitationRequest`, `PermissionsRequestApproval`, `ItemToolCall`, `ChatgptAuthTokensRefresh`, `AttestationGenerate`, `ApplyPatchApproval`, `ExecCommandApproval` variants alongside the existing `CmdExecApproval` and `FileChangeApproval`.
- **Scorecard now tracks the ServerRequest envelope** in addition to ServerNotification and ClientRequest.

### Changed

- **`PatchChangeKind`** — Switched from a bare string enum to the internally-tagged shape codex actually emits: `{"type":"add"}`, `{"type":"delete"}`, `{"type":"update","move_path":...}`. Fixes [issue #128](https://github.com/meawoppl/rust-code-agent-sdks/issues/128)'s `unknown variant 'type'` reports. Test fixtures regenerated.
- **`FileUpdateChange`** — Added the required `diff: String` field that upstream sends. Defaulted to empty string for back-compat when parsing older payloads.
- **The 29 previously-`Value`-stub notification types** (`AccountUpdatedNotification`, `AppListUpdatedNotification`, `CommandExecOutputDeltaNotification`, the `thread/realtime/*` family, etc.) are now fully field-typed via the codegen output.

### Coverage scorecard

```
modeled:        149/149 (100%) — every server notification + client request + server request method
with sample:    149/149 (100%) — every modeled method's sample validates against the schema
```

## [0.129.0] - 2026-05-15

### Added

- **100% method coverage** of the Codex app-server v2 JSON Schema. Every method enumerated in `ServerNotification.oneOf` (63) and `ClientRequest.oneOf` (76) is now modeled — 139/139.
- **`cargo run --example schema_coverage`** scorecard tool that walks the upstream JSON Schema bundle, cross-references against the crate's typed surface, and reports `✓` (modeled + sample validates) / `◐` (modeled, no sample yet) / `⚠` (drift) / `✗` (missing) per method. Override the schema path with `CODEX_SCHEMA_PATH=/path/to/freshly-generated.json` to validate against a fresh schema.
- **Typed notification variants** for the 48 previously-unmodeled methods: `item/fileChange/patchUpdated`, `item/plan/delta`, `turn/plan/updated`, `turn/diff/updated`, `item/reasoning/summaryPartAdded`, `item/reasoning/textDelta`, `mcpServer/oauthLogin/completed`, `account/login/completed`, `account/updated`, `app/list/updated`, `command/exec/outputDelta`, `configWarning`, `deprecationNotice`, `externalAgentConfig/import/completed`, `fs/changed`, `fuzzyFileSearch/session{Completed,Updated}`, `guardianWarning`, `hook/{started,completed}`, `item/autoApprovalReview/{started,completed}`, `item/commandExecution/terminalInteraction`, `item/mcpToolCall/progress`, `model/{rerouted,verification}`, `process/{exited,outputDelta}`, `serverRequest/resolved`, `skills/changed`, `thread/{archived,closed,unarchived,compacted,goal/{updated,cleared},name/updated}`, the `thread/realtime/*` family (8 variants), `warning`, `windows/worldWritableWarning`, `windowsSandbox/setupCompleted`. All wired through `Notification::from_envelope()`, `into_envelope()`, and the strict typed-message audit.
- **Method-name constants** for all 70 previously-unmodeled client → server requests under `protocol::methods` (e.g. `THREAD_LIST`, `FS_WRITEFILE`, `COMMAND_EXEC`, `MCPSERVER_TOOL_CALL`, the `plugin/*`, `marketplace/*`, `experimentalFeature/*`, and `account/*` families).
- **`jsonschema` dev-dependency** powering the scorecard's wire-shape validation.
- **Workspace snapshot** of the upstream `codex_app_server_protocol.v2.schemas.json` at `codex-codes/tests/schemas/` so the scorecard runs offline.

### Changed

- Many new notification stubs use `#[serde(transparent)] pub struct Foo(pub Value)` so the wire shape is preserved end-to-end while field-level typing is deferred. Upgrade path is mechanical: replace the `Value` payload with named fields when callers need them; the dispatch surface doesn't change.

### Notes

- **Drift findings surfaced by the scorecard but not fixed here**:
  - `PatchChangeKind` in `io/items.rs` is still a bare string enum; upstream moved to an internally-tagged object enum with `{"type":"update","move_path":...}`. Fixing requires regenerating the test fixtures against a live Codex CLI. Root cause of [issue #128](https://github.com/meawoppl/rust-code-agent-sdks/issues/128)'s `unknown variant 'type'` reports.
  - `FileUpdateChange` is missing the required `diff: String` field upstream now sends.
- The scorecard reports `1/139 (1%)` with validating samples — only `error`. Sample registry is open to grow in follow-ups; each new sample drift-checks one more method end-to-end.

## [0.128.1] - 2026-05-15

### Added

- **`ParseError` struct** — Carries `raw_line`, `raw_json`, `error_message`, and an optional `method` for parsing failures, mirroring [`claude_codes::ParseError`](https://docs.rs/claude-codes/latest/claude_codes/struct.ParseError.html). Two constructors:
  - `ParseError::from_line(line, error)` — for bare-JSON / envelope-shape failures; populates `raw_json` if the line was valid JSON.
  - `ParseError::from_envelope(method, params, error)` — for typed-decode failures whose envelope parsed but whose `params` did not match; preserves the JSON-RPC `method` and `params`, and reconstructs a wire-equivalent `raw_line` for bug reports.
- **Regression tests in `tests/integration_tests.rs`** — three new tests that pin the exact code path used by `next_message`, including one reproducing the `missing field "callId"` failure mode from issue #128.

### Changed

- **`Error::Deserialization`** is now `Error::Deserialization(ParseError)` (was `Error::Deserialization(String)`). Code that matched the previous string payload should read `pe.error_message` / `pe.raw_line` / `pe.method` instead. Shipped as a patch — pre-1.0 crate, only this workspace's `cc-proxy` is a known downstream consumer.
- **`AsyncClient::next_message` / `SyncClient::next_message`** — On typed-decode failures (`Notification::from_envelope` / `ServerRequest::from_envelope`), the error now carries the original `method` and `params` via `Error::Deserialization(ParseError)` instead of dropping them in an opaque `Error::Json(serde_json::Error)`. Consumers can render the offending frame for bug reports without snooping `DEBUG`-level tracing for the raw line (fixes #128).

## [0.128.0] - 2026-05-14

Version jumps from 0.101.x into the 0.1xx range that tracks the Codex CLI it
targets (same convention as the sibling `claude-codes` crate, which mirrors
the Claude Code CLI version). Released as `0.128.0` rather than `0.130.0`
intentionally — the bindings have only been validated against a single live
turn so far and the underlying behavior is still evolving; sitting a couple
of patch numbers behind the CLI leaves room to bump in lockstep once the
typed surface is more battle-tested.


### Added

- **Typed message dispatch** — New module [`crate::messages`] introduces closed enums [`Notification`] and [`ServerRequest`] that wrap the per-method param structs. The clients now return the typed [`ServerMessage`] from `next_message()`/`events()` instead of the loose `{ method, params }` shape. Mirrors the [`ContentBlock`]-style dispatch in the sibling `claude-codes` crate: hand-written `Serialize`/`Deserialize` impls inspect the `method` discriminant, route known cases through their typed struct, and route unknown methods to an `Unknown { method, params }` fallback for forward compatibility. Known methods whose payload doesn't fit error loudly — the typing contract is enforced.
- **New typed notifications** — `AccountRateLimitsUpdatedNotification` (`account/rateLimits/updated`), `McpServerStartupStatusUpdatedNotification` (`mcpServer/startupStatus/updated`), `RemoteControlStatusChangedNotification` (`remoteControl/status/changed`). The app-server emits all three during normal operation; previously they fell through to the raw fallback.
- **`RateLimits` / `RateLimitWindow` / `TokenCounts`** — Supporting structs for the above and for the new nested `TokenUsage` shape.
- **`UserMessageItem` / `UserMessageContent`** — Added `ThreadItem::UserMessage` variant for the user-prompt item the app-server emits at the start of each turn (the exec JSONL protocol doesn't typically emit this).
- **Strict typed-message audit test** — `tests/live_client_tests.rs::test_typed_message_audit_strict` runs a real turn and asserts that every notification and server request resolves to a typed variant (no `Unknown`).

### Fixed

- **`ThreadStartedNotification`** — Was `{ thread_id: String }`; the wire actually sends `{ thread: ThreadInfo }` with the full info object.
- **`TurnStartedNotification` / `TurnCompletedNotification`** — Had a top-level `turn_id` field that doesn't exist on the wire (the id lives inside `turn.id`). Both now carry `{ thread_id, turn: Turn }`.
- **`ThreadStatusChangedNotification` / `ThreadStatus`** — Status was modeled as a bare string enum; the wire actually sends an internally-tagged enum (`{"type":"idle"}`, `{"type":"active","activeFlags":[]}`). `ThreadStatus` is now `#[serde(tag = "type")]` with the `Active` variant carrying `active_flags`.
- **`ThreadTokenUsageUpdatedNotification`** — Field was named `usage` but the wire sends `tokenUsage`. The notification also carries `turn_id` which wasn't modeled. Plus the inner `TokenUsage` is actually a wrapper of `{last, total, modelContextWindow}` over a `TokenCounts` struct with `inputTokens`, `outputTokens`, `cachedInputTokens`, `reasoningOutputTokens`, `totalTokens` — restructured accordingly.
- **`ItemStartedNotification` / `ItemCompletedNotification`** — Now include `started_at_ms` / `completed_at_ms` from the wire.
- **`CommandExecutionItem`** — Was snake_case-only (`aggregated_output`, `exit_code`); the app-server protocol uses camelCase. Now carries `#[serde(rename_all = "camelCase")]` with snake_case aliases so both protocols deserialize cleanly. `aggregated_output` is now `Option<String>` since the app-server sends `null` while a command is still in progress.

### Changed (breaking)

- **`ServerMessage` shape** — `ServerMessage::Notification { method, params }` is now `ServerMessage::Notification(Notification)`; `ServerMessage::Request { id, method, params }` is now `ServerMessage::Request { id, request: ServerRequest }`. Update call sites to match on the typed enum variants instead of method-string comparisons.
- **`CommandExecutionItem.aggregated_output`** — Type changed from `String` to `Option<String>` (see above).
- **`TokenUsage`** — Restructured from a flat counts struct to `{last, total, modelContextWindow}` over `TokenCounts`. Old direct field access (`usage.input_tokens`) becomes `usage.last.input_tokens` or `usage.total.input_tokens`.
- **`ThreadStatus`** — Now a `#[serde(tag = "type")]` enum; the `Active` variant has an `active_flags: Vec<Value>` field. Pattern matches need updating.

## [0.101.2] - 2026-05-14

### Fixed

- **Stderr pipe deadlock** — `AsyncClient` and `SyncClient` now drain the app-server's stderr in a background task/thread instead of leaving it pinned to an unread `BufReader`. The Codex CLI emits ~200 KB/s of tracing to stderr, which would fill the ~64 KB kernel pipe within a fraction of a second and block the child process — manifesting as the client hanging on the first non-trivial request. Drained lines are forwarded through the `log` crate at the level encoded in the line (`error!`/`warn!`/`debug!`/`trace!`), with ANSI color codes stripped. INFO tracing (the vast majority of volume) is routed to `trace!` so `RUST_LOG=info` stays quiet by default while WARN/ERROR remain visible.

### Removed

- **`AsyncClient::take_stderr()`** — Replaced by automatic background draining; the method is incompatible with the new design and is removed without a deprecation cycle (no known external callers).

## [0.101.1] - 2026-03-17

### Added

- **Binary path resolution via `which`** — `AppServerBuilder::spawn()` and `spawn_sync()` now resolve non-absolute binary paths using `which` at spawn time, producing a clear `BinaryNotFound` error instead of an opaque OS "file not found" (#102)
- **`Error::BinaryNotFound`** — New error variant for when the CLI binary isn't found on PATH

### Changed

- **`spawn_sync()` return type** — Now returns `crate::error::Result<Child>` instead of `std::io::Result<Child>` for consistent error handling

## [0.101.0] - 2026-02-23

### Added

- **`initialize` handshake** — `AsyncClient::start()` and `SyncClient::start()` now send the required `initialize` request followed by an `initialized` notification before returning, fixing compatibility with Codex CLI 0.104.0+ which requires this handshake before accepting other methods (#87)
- **`InitializeParams`, `InitializeResponse`, `ClientInfo`, `InitializeCapabilities`** — New protocol types for the initialization handshake
- **`AsyncClient::spawn()` / `SyncClient::spawn()`** — Low-level constructors that skip automatic initialization, for callers that need custom `InitializeParams`
- **`AsyncClient::initialize()` / `SyncClient::initialize()`** — Explicit initialization method for use with `spawn()`
- **`methods::INITIALIZE` / `methods::INITIALIZED`** — Method name constants for the initialization handshake
- **`ThreadInfo`** — New struct for thread metadata returned inside `ThreadStartResponse`
- **Integration tests** — Live client tests against a real Codex app-server process (behind `integration-tests` feature flag)

### Changed

- **`ThreadStartResponse`** — Updated to match the actual app-server wire format: now contains a `thread: ThreadInfo` field with the thread ID and metadata, plus optional `model` field. Use `response.thread_id()` to get the thread ID.

### Breaking

- `ThreadStartResponse.thread_id` field replaced by `ThreadStartResponse.thread_id()` method

## [0.100.1] - 2026-02-21

### Changed

- **Replaced `codex exec` with `codex app-server` JSON-RPC protocol** — The crate now wraps `codex app-server --listen stdio://` instead of the one-shot `codex exec --json -`. This enables multi-turn conversations, approval flows, and streaming notifications.

### Added

- **`jsonrpc` module** — JSON-RPC message types (`JsonRpcMessage`, `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`, `JsonRpcNotification`, `RequestId`) matching the app-server wire format (no `"jsonrpc":"2.0"` field)
- **`protocol` module** — App-server v2 protocol types including thread/turn lifecycle params, server notifications, approval flow types, and method name constants
- **`AppServerBuilder`** — Replaces `CodexCliBuilder` for spawning the long-lived app-server process
- **Multi-turn `AsyncClient`** — JSON-RPC client with `thread_start`, `turn_start`, `turn_interrupt`, `thread_archive`, `respond`, and `next_message` methods
- **Multi-turn `SyncClient`** — Blocking counterpart with the same API surface
- **Approval flow support** — `CommandExecutionApprovalParams/Response` and `FileChangeApprovalParams/Response` for handling server-to-client approval requests
- **Streaming delta notifications** — `AgentMessageDeltaNotification`, `CmdOutputDeltaNotification`, `FileChangeOutputDeltaNotification`, `ReasoningDeltaNotification`
- **camelCase serde aliases** on `ThreadItem` variants and status enums for app-server compatibility (snake_case exec format still supported)
- **`Declined` variant** on `CommandExecutionStatus` for commands rejected via approval flow
- **Comprehensive documentation** — Module-level docs with examples, lifecycle guides, error conditions, notification reference tables, and rustdoc examples across all public modules

### Removed

- `CodexCliBuilder` (replaced by `AppServerBuilder`)
- One-shot exec client API

## [0.100.0] - 2026-02-17

### Added

- Initial release of `codex-codes` crate
- Typed Rust bindings for the OpenAI Codex CLI JSON protocol
- `ThreadEvent` and `ThreadItem` types for parsing exec-format JSONL events
- `ThreadOptions` configuration types (`ApprovalMode`, `SandboxMode`, `WebSearchMode`)
- Sync and async clients wrapping `codex exec --json -`
- Feature flags: `types` (WASM-compatible), `sync-client`, `async-client`
- Integration tests with captured protocol message test cases
- Version compatibility checking against installed Codex CLI
