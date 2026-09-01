# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.84.4] - 2026-09-01

### Added

- Initial release, tested against pi 0.84.4
  (`@earendil-works/pi-coding-agent`; requires Node 22+):
  - `PiCliBuilder` — typed argv for `--mode json` / `--mode rpc`
    invocations (provider, model, session, tools, thinking, extras).
  - `rpc` — the headless command protocol: typed `RpcCommand`s with id
    correlation, the response envelope, `AgentState` / `BashResult` /
    message views, and `RpcCommand::Raw` passthrough for unmodeled
    commands.
  - `io` — `PiEvent` (lifecycle + tool events, `Unknown` preserves
    payloads) and `PiMessage` (user / assistant / toolResult /
    bashExecution) with content blocks and usage accounting.
  - `PiRpcClient` — async (Tokio) client honoring the strict LF-only
    JSONL framing contract.
  - Live integration tier that is credential-free: drives a real
    `pi --mode rpc` process through state, model catalog, bash round
    trips (typed results landing in `get_messages`), and clean failure
    envelopes.
