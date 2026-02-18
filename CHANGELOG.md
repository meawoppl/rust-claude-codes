# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
