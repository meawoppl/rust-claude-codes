# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

- Initial support for `permission_suggestions` field in `ToolPermissionRequest`.

---

For older versions, see the [GitHub releases](https://github.com/meawoppl/rust-claude-codes/releases).
