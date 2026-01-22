//! Core message types for Claude communication.
//!
//! This module defines the primary message structures used in the Claude protocol:
//!
//! - [`ClaudeInput`] - Messages sent to Claude
//! - [`ClaudeOutput`] - Messages received from Claude
//! - [`ContentBlock`] - Different types of content within messages
//!
//! # Message Flow
//!
//! 1. Create a [`ClaudeInput`] with your query
//! 2. Send it to Claude via a client
//! 3. Receive [`ClaudeOutput`] messages in response
//! 4. Handle different output types (System, Assistant, Result)
//!
//! # Example
//!
//! ```
//! use claude_codes::{ClaudeInput, ClaudeOutput};
//!
//! // Create an input message
//! let input = ClaudeInput::user_message("Hello, Claude!", uuid::Uuid::new_v4());
//!
//! // Parse an output message
//! let json = r#"{"type":"assistant","message":{"role":"assistant","content":[]}}"#;
//! match ClaudeOutput::parse_json(json) {
//!     Ok(output) => println!("Got: {}", output.message_type()),
//!     Err(e) => eprintln!("Parse error: {}", e),
//! }
//! ```

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::fmt;
use uuid::Uuid;

/// Serialize an optional UUID as a string
fn serialize_optional_uuid<S>(uuid: &Option<Uuid>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match uuid {
        Some(id) => serializer.serialize_str(&id.to_string()),
        None => serializer.serialize_none(),
    }
}

/// Deserialize an optional UUID from a string
fn deserialize_optional_uuid<'de, D>(deserializer: D) -> Result<Option<Uuid>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt_str: Option<String> = Option::deserialize(deserializer)?;
    match opt_str {
        Some(s) => Uuid::parse_str(&s)
            .map(Some)
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

/// Top-level enum for all possible Claude input messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeInput {
    /// User message input
    User(UserMessage),

    /// Control request (for initialization handshake)
    ControlRequest(ControlRequest),

    /// Control response (for tool permission responses)
    ControlResponse(ControlResponse),

    /// Raw JSON for untyped messages
    #[serde(untagged)]
    Raw(Value),
}

/// Error type for parsing failures that preserves the raw JSON
#[derive(Debug, Clone)]
pub struct ParseError {
    /// The raw JSON value that failed to parse
    pub raw_json: Value,
    /// The underlying serde error message
    pub error_message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to parse ClaudeOutput: {}", self.error_message)
    }
}

impl std::error::Error for ParseError {}

/// Top-level enum for all possible Claude output messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeOutput {
    /// System initialization message
    System(SystemMessage),

    /// User message echoed back
    User(UserMessage),

    /// Assistant response
    Assistant(AssistantMessage),

    /// Result message (completion of a query)
    Result(ResultMessage),

    /// Control request from CLI (tool permissions, hooks, etc.)
    ControlRequest(ControlRequest),

    /// Control response from CLI (ack for initialization, etc.)
    ControlResponse(ControlResponse),
}

/// User message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub message: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(
        serialize_with = "serialize_optional_uuid",
        deserialize_with = "deserialize_optional_uuid"
    )]
    pub session_id: Option<Uuid>,
}

/// Message content with role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContent {
    pub role: String,
    #[serde(deserialize_with = "deserialize_content_blocks")]
    pub content: Vec<ContentBlock>,
}

/// Deserialize content blocks that can be either a string or array
fn deserialize_content_blocks<'de, D>(deserializer: D) -> Result<Vec<ContentBlock>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Value = Value::deserialize(deserializer)?;
    match value {
        Value::String(s) => Ok(vec![ContentBlock::Text(TextBlock { text: s })]),
        Value::Array(_) => serde_json::from_value(value).map_err(serde::de::Error::custom),
        _ => Err(serde::de::Error::custom(
            "content must be a string or array",
        )),
    }
}

/// System message with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMessage {
    pub subtype: String,
    #[serde(flatten)]
    pub data: Value, // Captures all other fields
}

impl SystemMessage {
    /// Check if this is an init message
    pub fn is_init(&self) -> bool {
        self.subtype == "init"
    }

    /// Check if this is a status message
    pub fn is_status(&self) -> bool {
        self.subtype == "status"
    }

    /// Check if this is a compact_boundary message
    pub fn is_compact_boundary(&self) -> bool {
        self.subtype == "compact_boundary"
    }

    /// Try to parse as an init message
    pub fn as_init(&self) -> Option<InitMessage> {
        if self.subtype != "init" {
            return None;
        }
        serde_json::from_value(self.data.clone()).ok()
    }

    /// Try to parse as a status message
    pub fn as_status(&self) -> Option<StatusMessage> {
        if self.subtype != "status" {
            return None;
        }
        serde_json::from_value(self.data.clone()).ok()
    }

    /// Try to parse as a compact_boundary message
    pub fn as_compact_boundary(&self) -> Option<CompactBoundaryMessage> {
        if self.subtype != "compact_boundary" {
            return None;
        }
        serde_json::from_value(self.data.clone()).ok()
    }
}

/// Init system message data - sent at session start
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitMessage {
    /// Session identifier
    pub session_id: String,
    /// Current working directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Model being used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// List of available tools
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
    /// MCP servers configured
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<Value>,
}

/// Status system message - sent during operations like context compaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusMessage {
    /// Session identifier
    pub session_id: String,
    /// Current status (e.g., "compacting") or null when complete
    pub status: Option<String>,
    /// Unique identifier for this message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

/// Compact boundary message - marks where context compaction occurred
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactBoundaryMessage {
    /// Session identifier
    pub session_id: String,
    /// Metadata about the compaction
    pub compact_metadata: CompactMetadata,
    /// Unique identifier for this message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

/// Metadata about context compaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactMetadata {
    /// Number of tokens before compaction
    pub pre_tokens: u64,
    /// What triggered the compaction ("auto" or "manual")
    pub trigger: String,
}

/// Assistant message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub message: AssistantMessageContent,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_tool_use_id: Option<String>,
}

/// Nested message content for assistant messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessageContent {
    pub id: String,
    pub role: String,
    pub model: String,
    pub content: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<AssistantUsage>,
}

/// Usage information for assistant messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantUsage {
    /// Number of input tokens
    #[serde(default)]
    pub input_tokens: u32,

    /// Number of output tokens
    #[serde(default)]
    pub output_tokens: u32,

    /// Tokens used to create cache
    #[serde(default)]
    pub cache_creation_input_tokens: u32,

    /// Tokens read from cache
    #[serde(default)]
    pub cache_read_input_tokens: u32,

    /// Service tier used (e.g., "standard")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,

    /// Detailed cache creation breakdown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation: Option<CacheCreationDetails>,
}

/// Detailed cache creation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheCreationDetails {
    /// Ephemeral 1-hour input tokens
    #[serde(default)]
    pub ephemeral_1h_input_tokens: u32,

    /// Ephemeral 5-minute input tokens
    #[serde(default)]
    pub ephemeral_5m_input_tokens: u32,
}

/// Content blocks for messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text(TextBlock),
    Image(ImageBlock),
    Thinking(ThinkingBlock),
    ToolUse(ToolUseBlock),
    ToolResult(ToolResultBlock),
}

/// Text content block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBlock {
    pub text: String,
}

/// Image content block (follows Anthropic API structure)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageBlock {
    pub source: ImageSource,
}

/// Image source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String, // "base64"
    pub media_type: String, // e.g., "image/jpeg", "image/png"
    pub data: String,       // Base64-encoded image data
}

/// Thinking content block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingBlock {
    pub thinking: String,
    pub signature: String,
}

/// Tool use content block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseBlock {
    pub id: String,
    pub name: String,
    pub input: Value,
}

impl ToolUseBlock {
    /// Try to parse the input as a typed ToolInput.
    ///
    /// This attempts to deserialize the raw JSON input into a strongly-typed
    /// `ToolInput` enum variant. Returns `None` if parsing fails.
    ///
    /// # Example
    ///
    /// ```
    /// use claude_codes::{ToolUseBlock, ToolInput};
    /// use serde_json::json;
    ///
    /// let block = ToolUseBlock {
    ///     id: "toolu_123".to_string(),
    ///     name: "Bash".to_string(),
    ///     input: json!({"command": "ls -la"}),
    /// };
    ///
    /// if let Some(ToolInput::Bash(bash)) = block.typed_input() {
    ///     assert_eq!(bash.command, "ls -la");
    /// }
    /// ```
    pub fn typed_input(&self) -> Option<crate::tool_inputs::ToolInput> {
        serde_json::from_value(self.input.clone()).ok()
    }

    /// Parse the input as a typed ToolInput, returning an error on failure.
    ///
    /// Unlike `typed_input()`, this method returns the parsing error for debugging.
    pub fn try_typed_input(&self) -> Result<crate::tool_inputs::ToolInput, serde_json::Error> {
        serde_json::from_value(self.input.clone())
    }
}

/// Tool result content block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultBlock {
    pub tool_use_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<ToolResultContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Tool result content type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    Text(String),
    Structured(Vec<Value>),
}

/// Result message for completed queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultMessage {
    pub subtype: ResultSubtype,
    pub is_error: bool,
    pub duration_ms: u64,
    pub duration_api_ms: u64,
    pub num_turns: i32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,

    pub session_id: String,
    pub total_cost_usd: f64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageInfo>,

    /// Tools that were blocked due to permission denials during the session
    #[serde(default)]
    pub permission_denials: Vec<PermissionDenial>,

    /// Error messages when `is_error` is true.
    ///
    /// Contains human-readable error strings (e.g., "No conversation found with session ID: ...").
    /// This allows typed access to error conditions without needing to serialize to JSON and search.
    #[serde(default)]
    pub errors: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

/// A record of a tool permission that was denied during the session.
///
/// This is included in `ResultMessage.permission_denials` to provide a summary
/// of all permission denials that occurred.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PermissionDenial {
    /// The name of the tool that was blocked (e.g., "Bash", "Write")
    pub tool_name: String,

    /// The input that was passed to the tool
    pub tool_input: Value,

    /// The unique identifier for this tool use request
    pub tool_use_id: String,
}

/// Result subtypes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultSubtype {
    Success,
    ErrorMaxTurns,
    ErrorDuringExecution,
}

/// MCP Server configuration types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpServerConfig {
    Stdio(McpStdioServerConfig),
    Sse(McpSseServerConfig),
    Http(McpHttpServerConfig),
}

/// MCP stdio server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpStdioServerConfig {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<std::collections::HashMap<String, String>>,
}

/// MCP SSE server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSseServerConfig {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::HashMap<String, String>>,
}

/// MCP HTTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpHttpServerConfig {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::HashMap<String, String>>,
}

/// Permission mode for Claude operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    Default,
    AcceptEdits,
    BypassPermissions,
    Plan,
}

// ============================================================================
// Control Protocol Types (for bidirectional tool approval)
// ============================================================================

/// Control request from CLI (tool permission requests, hooks, etc.)
///
/// When using `--permission-prompt-tool stdio`, the CLI sends these requests
/// asking for approval before executing tools. The SDK must respond with a
/// [`ControlResponse`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlRequest {
    /// Unique identifier for this request (used to correlate responses)
    pub request_id: String,
    /// The request payload
    pub request: ControlRequestPayload,
}

/// Control request payload variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum ControlRequestPayload {
    /// Tool permission request - Claude wants to use a tool
    CanUseTool(ToolPermissionRequest),
    /// Hook callback request
    HookCallback(HookCallbackRequest),
    /// MCP message request
    McpMessage(McpMessageRequest),
    /// Initialize request (sent by SDK to CLI)
    Initialize(InitializeRequest),
}

/// A suggested permission for tool approval.
///
/// When Claude requests tool permission, it may include suggestions for
/// permissions that could be granted to avoid repeated prompts for similar
/// actions. The format varies based on the suggestion type:
///
/// - `setMode`: `{"type": "setMode", "mode": "acceptEdits", "destination": "session"}`
/// - `addRules`: `{"type": "addRules", "rules": [...], "behavior": "allow", "destination": "session"}`
///
/// Use the helper methods to access common fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PermissionSuggestion {
    /// The type of suggestion (e.g., "setMode", "addRules")
    #[serde(rename = "type")]
    pub suggestion_type: String,
    /// Where to apply this permission (e.g., "session", "project")
    pub destination: String,
    /// The permission mode (for setMode type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// The behavior (for addRules type, e.g., "allow")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behavior: Option<String>,
    /// The rules to add (for addRules type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<Value>>,
}

/// Tool permission request details
///
/// This is sent when Claude wants to use a tool. The SDK should evaluate
/// the request and respond with allow/deny using the ergonomic builder methods.
///
/// # Example
///
/// ```
/// use claude_codes::{ToolPermissionRequest, ControlResponse};
/// use serde_json::json;
///
/// fn handle_permission(req: &ToolPermissionRequest, request_id: &str) -> ControlResponse {
///     // Block dangerous bash commands
///     if req.tool_name == "Bash" {
///         if let Some(cmd) = req.input.get("command").and_then(|v| v.as_str()) {
///             if cmd.contains("rm -rf") {
///                 return req.deny("Dangerous command blocked", request_id);
///             }
///         }
///     }
///
///     // Allow everything else
///     req.allow(request_id)
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissionRequest {
    /// Name of the tool Claude wants to use (e.g., "Bash", "Write", "Read")
    pub tool_name: String,
    /// Input parameters for the tool
    pub input: Value,
    /// Suggested permissions that could be granted to avoid repeated prompts
    #[serde(default)]
    pub permission_suggestions: Vec<PermissionSuggestion>,
    /// Path that was blocked (if this is a retry after path-based denial)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_path: Option<String>,
}

impl ToolPermissionRequest {
    /// Allow the tool to execute with its original input.
    ///
    /// # Example
    /// ```
    /// # use claude_codes::ToolPermissionRequest;
    /// # use serde_json::json;
    /// let req = ToolPermissionRequest {
    ///     tool_name: "Read".to_string(),
    ///     input: json!({"file_path": "/tmp/test.txt"}),
    ///     permission_suggestions: vec![],
    ///     blocked_path: None,
    /// };
    /// let response = req.allow("req-123");
    /// ```
    pub fn allow(&self, request_id: &str) -> ControlResponse {
        ControlResponse::from_result(request_id, PermissionResult::allow(self.input.clone()))
    }

    /// Allow the tool to execute with modified input.
    ///
    /// Use this to sanitize or redirect tool inputs. For example, redirecting
    /// file writes to a safe directory.
    ///
    /// # Example
    /// ```
    /// # use claude_codes::ToolPermissionRequest;
    /// # use serde_json::json;
    /// let req = ToolPermissionRequest {
    ///     tool_name: "Write".to_string(),
    ///     input: json!({"file_path": "/etc/passwd", "content": "test"}),
    ///     permission_suggestions: vec![],
    ///     blocked_path: None,
    /// };
    /// // Redirect to safe location
    /// let safe_input = json!({"file_path": "/tmp/safe/passwd", "content": "test"});
    /// let response = req.allow_with(safe_input, "req-123");
    /// ```
    pub fn allow_with(&self, modified_input: Value, request_id: &str) -> ControlResponse {
        ControlResponse::from_result(request_id, PermissionResult::allow(modified_input))
    }

    /// Allow with updated permissions list.
    pub fn allow_with_permissions(
        &self,
        modified_input: Value,
        permissions: Vec<Value>,
        request_id: &str,
    ) -> ControlResponse {
        ControlResponse::from_result(
            request_id,
            PermissionResult::allow_with_permissions(modified_input, permissions),
        )
    }

    /// Deny the tool execution.
    ///
    /// The message will be shown to Claude, who may try a different approach.
    ///
    /// # Example
    /// ```
    /// # use claude_codes::ToolPermissionRequest;
    /// # use serde_json::json;
    /// let req = ToolPermissionRequest {
    ///     tool_name: "Bash".to_string(),
    ///     input: json!({"command": "sudo rm -rf /"}),
    ///     permission_suggestions: vec![],
    ///     blocked_path: None,
    /// };
    /// let response = req.deny("Dangerous command blocked by policy", "req-123");
    /// ```
    pub fn deny(&self, message: impl Into<String>, request_id: &str) -> ControlResponse {
        ControlResponse::from_result(request_id, PermissionResult::deny(message))
    }

    /// Deny the tool execution and stop the entire session.
    ///
    /// Use this for severe policy violations that should halt all processing.
    pub fn deny_and_stop(&self, message: impl Into<String>, request_id: &str) -> ControlResponse {
        ControlResponse::from_result(request_id, PermissionResult::deny_and_interrupt(message))
    }
}

/// Result of a permission decision
///
/// This type represents the decision made by the permission callback.
/// It can be serialized directly into the control response format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "behavior", rename_all = "snake_case")]
pub enum PermissionResult {
    /// Allow the tool to execute
    Allow {
        /// The (possibly modified) input to pass to the tool
        #[serde(rename = "updatedInput")]
        updated_input: Value,
        /// Optional updated permissions list
        #[serde(rename = "updatedPermissions", skip_serializing_if = "Option::is_none")]
        updated_permissions: Option<Vec<Value>>,
    },
    /// Deny the tool execution
    Deny {
        /// Message explaining why the tool was denied
        message: String,
        /// If true, stop the entire session
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        interrupt: bool,
    },
}

impl PermissionResult {
    /// Create an allow result with the given input
    pub fn allow(input: Value) -> Self {
        PermissionResult::Allow {
            updated_input: input,
            updated_permissions: None,
        }
    }

    /// Create an allow result with permissions
    pub fn allow_with_permissions(input: Value, permissions: Vec<Value>) -> Self {
        PermissionResult::Allow {
            updated_input: input,
            updated_permissions: Some(permissions),
        }
    }

    /// Create a deny result
    pub fn deny(message: impl Into<String>) -> Self {
        PermissionResult::Deny {
            message: message.into(),
            interrupt: false,
        }
    }

    /// Create a deny result that also interrupts the session
    pub fn deny_and_interrupt(message: impl Into<String>) -> Self {
        PermissionResult::Deny {
            message: message.into(),
            interrupt: true,
        }
    }
}

/// Hook callback request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookCallbackRequest {
    pub callback_id: String,
    pub input: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
}

/// MCP message request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpMessageRequest {
    pub server_name: String,
    pub message: Value,
}

/// Initialize request (SDK -> CLI)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<Value>,
}

/// Control response to CLI
///
/// Built using the ergonomic methods on [`ToolPermissionRequest`] or
/// constructed directly for other control request types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlResponse {
    /// The request ID this response corresponds to
    pub response: ControlResponsePayload,
}

impl ControlResponse {
    /// Create a success response from a PermissionResult
    ///
    /// This is the preferred way to construct permission responses.
    pub fn from_result(request_id: &str, result: PermissionResult) -> Self {
        // Serialize the PermissionResult to Value for the response
        let response_value = serde_json::to_value(&result)
            .expect("PermissionResult serialization should never fail");
        ControlResponse {
            response: ControlResponsePayload::Success {
                request_id: request_id.to_string(),
                response: Some(response_value),
            },
        }
    }

    /// Create a success response with the given payload (raw Value)
    pub fn success(request_id: &str, response_data: Value) -> Self {
        ControlResponse {
            response: ControlResponsePayload::Success {
                request_id: request_id.to_string(),
                response: Some(response_data),
            },
        }
    }

    /// Create an empty success response (for acks)
    pub fn success_empty(request_id: &str) -> Self {
        ControlResponse {
            response: ControlResponsePayload::Success {
                request_id: request_id.to_string(),
                response: None,
            },
        }
    }

    /// Create an error response
    pub fn error(request_id: &str, error_message: impl Into<String>) -> Self {
        ControlResponse {
            response: ControlResponsePayload::Error {
                request_id: request_id.to_string(),
                error: error_message.into(),
            },
        }
    }
}

/// Control response payload
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum ControlResponsePayload {
    Success {
        request_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        response: Option<Value>,
    },
    Error {
        request_id: String,
        error: String,
    },
}

/// Wrapper for outgoing control responses (includes type tag)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlResponseMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub response: ControlResponsePayload,
}

impl From<ControlResponse> for ControlResponseMessage {
    fn from(resp: ControlResponse) -> Self {
        ControlResponseMessage {
            message_type: "control_response".to_string(),
            response: resp.response,
        }
    }
}

/// Wrapper for outgoing control requests (includes type tag)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlRequestMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub request_id: String,
    pub request: ControlRequestPayload,
}

impl ControlRequestMessage {
    /// Create an initialization request to send to CLI
    pub fn initialize(request_id: impl Into<String>) -> Self {
        ControlRequestMessage {
            message_type: "control_request".to_string(),
            request_id: request_id.into(),
            request: ControlRequestPayload::Initialize(InitializeRequest { hooks: None }),
        }
    }

    /// Create an initialization request with hooks configuration
    pub fn initialize_with_hooks(request_id: impl Into<String>, hooks: Value) -> Self {
        ControlRequestMessage {
            message_type: "control_request".to_string(),
            request_id: request_id.into(),
            request: ControlRequestPayload::Initialize(InitializeRequest { hooks: Some(hooks) }),
        }
    }
}

/// Usage information for the request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfo {
    pub input_tokens: u32,
    pub cache_creation_input_tokens: u32,
    pub cache_read_input_tokens: u32,
    pub output_tokens: u32,
    pub server_tool_use: ServerToolUse,
    pub service_tier: String,
}

/// Server tool usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerToolUse {
    pub web_search_requests: u32,
}

impl ClaudeInput {
    /// Create a simple text user message
    pub fn user_message(text: impl Into<String>, session_id: Uuid) -> Self {
        ClaudeInput::User(UserMessage {
            message: MessageContent {
                role: "user".to_string(),
                content: vec![ContentBlock::Text(TextBlock { text: text.into() })],
            },
            session_id: Some(session_id),
        })
    }

    /// Create a user message with content blocks
    pub fn user_message_blocks(blocks: Vec<ContentBlock>, session_id: Uuid) -> Self {
        ClaudeInput::User(UserMessage {
            message: MessageContent {
                role: "user".to_string(),
                content: blocks,
            },
            session_id: Some(session_id),
        })
    }

    /// Create a user message with an image and optional text
    /// Only supports JPEG, PNG, GIF, and WebP media types
    pub fn user_message_with_image(
        image_data: String,
        media_type: String,
        text: Option<String>,
        session_id: Uuid,
    ) -> Result<Self, String> {
        // Validate media type
        let valid_types = ["image/jpeg", "image/png", "image/gif", "image/webp"];

        if !valid_types.contains(&media_type.as_str()) {
            return Err(format!(
                "Invalid media type '{}'. Only JPEG, PNG, GIF, and WebP are supported.",
                media_type
            ));
        }

        let mut blocks = vec![ContentBlock::Image(ImageBlock {
            source: ImageSource {
                source_type: "base64".to_string(),
                media_type,
                data: image_data,
            },
        })];

        if let Some(text_content) = text {
            blocks.push(ContentBlock::Text(TextBlock { text: text_content }));
        }

        Ok(Self::user_message_blocks(blocks, session_id))
    }
}

impl ClaudeOutput {
    /// Get the message type as a string
    pub fn message_type(&self) -> String {
        match self {
            ClaudeOutput::System(_) => "system".to_string(),
            ClaudeOutput::User(_) => "user".to_string(),
            ClaudeOutput::Assistant(_) => "assistant".to_string(),
            ClaudeOutput::Result(_) => "result".to_string(),
            ClaudeOutput::ControlRequest(_) => "control_request".to_string(),
            ClaudeOutput::ControlResponse(_) => "control_response".to_string(),
        }
    }

    /// Check if this is a control request (tool permission request)
    pub fn is_control_request(&self) -> bool {
        matches!(self, ClaudeOutput::ControlRequest(_))
    }

    /// Check if this is a control response
    pub fn is_control_response(&self) -> bool {
        matches!(self, ClaudeOutput::ControlResponse(_))
    }

    /// Get the control request if this is one
    pub fn as_control_request(&self) -> Option<&ControlRequest> {
        match self {
            ClaudeOutput::ControlRequest(req) => Some(req),
            _ => None,
        }
    }

    /// Check if this is a result with error
    pub fn is_error(&self) -> bool {
        matches!(self, ClaudeOutput::Result(r) if r.is_error)
    }

    /// Check if this is an assistant message
    pub fn is_assistant_message(&self) -> bool {
        matches!(self, ClaudeOutput::Assistant(_))
    }

    /// Check if this is a system message
    pub fn is_system_message(&self) -> bool {
        matches!(self, ClaudeOutput::System(_))
    }

    /// Check if this is a system init message
    ///
    /// # Example
    /// ```
    /// use claude_codes::ClaudeOutput;
    ///
    /// let json = r#"{"type":"system","subtype":"init","session_id":"abc"}"#;
    /// let output: ClaudeOutput = serde_json::from_str(json).unwrap();
    /// assert!(output.is_system_init());
    /// ```
    pub fn is_system_init(&self) -> bool {
        matches!(self, ClaudeOutput::System(sys) if sys.is_init())
    }

    /// Get the session ID from any message type that has one.
    ///
    /// Returns the session ID from System, Assistant, or Result messages.
    /// Returns `None` for User, ControlRequest, and ControlResponse messages.
    ///
    /// # Example
    /// ```
    /// use claude_codes::ClaudeOutput;
    ///
    /// let json = r#"{"type":"result","subtype":"success","is_error":false,
    ///     "duration_ms":100,"duration_api_ms":200,"num_turns":1,
    ///     "session_id":"my-session","total_cost_usd":0.01}"#;
    /// let output: ClaudeOutput = serde_json::from_str(json).unwrap();
    /// assert_eq!(output.session_id(), Some("my-session"));
    /// ```
    pub fn session_id(&self) -> Option<&str> {
        match self {
            ClaudeOutput::System(sys) => sys.data.get("session_id").and_then(|v| v.as_str()),
            ClaudeOutput::Assistant(ass) => Some(&ass.session_id),
            ClaudeOutput::Result(res) => Some(&res.session_id),
            ClaudeOutput::User(_) => None,
            ClaudeOutput::ControlRequest(_) => None,
            ClaudeOutput::ControlResponse(_) => None,
        }
    }

    /// Get a specific tool use by name from an assistant message.
    ///
    /// Returns the first `ToolUseBlock` with the given name, or `None` if this
    /// is not an assistant message or doesn't contain the specified tool.
    ///
    /// # Example
    /// ```
    /// use claude_codes::ClaudeOutput;
    ///
    /// let json = r#"{"type":"assistant","message":{"id":"msg_1","role":"assistant",
    ///     "model":"claude-3","content":[{"type":"tool_use","id":"tu_1",
    ///     "name":"Bash","input":{"command":"ls"}}]},"session_id":"abc"}"#;
    /// let output: ClaudeOutput = serde_json::from_str(json).unwrap();
    ///
    /// if let Some(bash) = output.as_tool_use("Bash") {
    ///     assert_eq!(bash.name, "Bash");
    /// }
    /// ```
    pub fn as_tool_use(&self, tool_name: &str) -> Option<&ToolUseBlock> {
        match self {
            ClaudeOutput::Assistant(ass) => {
                ass.message.content.iter().find_map(|block| match block {
                    ContentBlock::ToolUse(tu) if tu.name == tool_name => Some(tu),
                    _ => None,
                })
            }
            _ => None,
        }
    }

    /// Get all tool uses from an assistant message.
    ///
    /// Returns an iterator over all `ToolUseBlock`s in the message, or an empty
    /// iterator if this is not an assistant message.
    ///
    /// # Example
    /// ```
    /// use claude_codes::ClaudeOutput;
    ///
    /// let json = r#"{"type":"assistant","message":{"id":"msg_1","role":"assistant",
    ///     "model":"claude-3","content":[
    ///         {"type":"tool_use","id":"tu_1","name":"Read","input":{"file_path":"/tmp/a"}},
    ///         {"type":"tool_use","id":"tu_2","name":"Write","input":{"file_path":"/tmp/b","content":"x"}}
    ///     ]},"session_id":"abc"}"#;
    /// let output: ClaudeOutput = serde_json::from_str(json).unwrap();
    ///
    /// let tools: Vec<_> = output.tool_uses().collect();
    /// assert_eq!(tools.len(), 2);
    /// ```
    pub fn tool_uses(&self) -> impl Iterator<Item = &ToolUseBlock> {
        let content = match self {
            ClaudeOutput::Assistant(ass) => Some(&ass.message.content),
            _ => None,
        };

        content
            .into_iter()
            .flat_map(|c| c.iter())
            .filter_map(|block| match block {
                ContentBlock::ToolUse(tu) => Some(tu),
                _ => None,
            })
    }

    /// Get text content from an assistant message.
    ///
    /// Returns the concatenated text from all text blocks in the message,
    /// or `None` if this is not an assistant message or has no text content.
    ///
    /// # Example
    /// ```
    /// use claude_codes::ClaudeOutput;
    ///
    /// let json = r#"{"type":"assistant","message":{"id":"msg_1","role":"assistant",
    ///     "model":"claude-3","content":[{"type":"text","text":"Hello, world!"}]},
    ///     "session_id":"abc"}"#;
    /// let output: ClaudeOutput = serde_json::from_str(json).unwrap();
    /// assert_eq!(output.text_content(), Some("Hello, world!".to_string()));
    /// ```
    pub fn text_content(&self) -> Option<String> {
        match self {
            ClaudeOutput::Assistant(ass) => {
                let texts: Vec<&str> = ass
                    .message
                    .content
                    .iter()
                    .filter_map(|block| match block {
                        ContentBlock::Text(t) => Some(t.text.as_str()),
                        _ => None,
                    })
                    .collect();

                if texts.is_empty() {
                    None
                } else {
                    Some(texts.join(""))
                }
            }
            _ => None,
        }
    }

    /// Get the assistant message if this is one.
    ///
    /// # Example
    /// ```
    /// use claude_codes::ClaudeOutput;
    ///
    /// let json = r#"{"type":"assistant","message":{"id":"msg_1","role":"assistant",
    ///     "model":"claude-3","content":[]},"session_id":"abc"}"#;
    /// let output: ClaudeOutput = serde_json::from_str(json).unwrap();
    ///
    /// if let Some(assistant) = output.as_assistant() {
    ///     assert_eq!(assistant.message.model, "claude-3");
    /// }
    /// ```
    pub fn as_assistant(&self) -> Option<&AssistantMessage> {
        match self {
            ClaudeOutput::Assistant(ass) => Some(ass),
            _ => None,
        }
    }

    /// Get the result message if this is one.
    ///
    /// # Example
    /// ```
    /// use claude_codes::ClaudeOutput;
    ///
    /// let json = r#"{"type":"result","subtype":"success","is_error":false,
    ///     "duration_ms":100,"duration_api_ms":200,"num_turns":1,
    ///     "session_id":"abc","total_cost_usd":0.01}"#;
    /// let output: ClaudeOutput = serde_json::from_str(json).unwrap();
    ///
    /// if let Some(result) = output.as_result() {
    ///     assert!(!result.is_error);
    /// }
    /// ```
    pub fn as_result(&self) -> Option<&ResultMessage> {
        match self {
            ClaudeOutput::Result(res) => Some(res),
            _ => None,
        }
    }

    /// Get the system message if this is one.
    pub fn as_system(&self) -> Option<&SystemMessage> {
        match self {
            ClaudeOutput::System(sys) => Some(sys),
            _ => None,
        }
    }

    /// Parse a JSON string, handling potential ANSI escape codes and other prefixes
    /// This method will:
    /// 1. First try to parse as-is
    /// 2. If that fails, trim until it finds a '{' and try again
    pub fn parse_json_tolerant(s: &str) -> Result<ClaudeOutput, ParseError> {
        // First try to parse as-is
        match Self::parse_json(s) {
            Ok(output) => Ok(output),
            Err(first_error) => {
                // If that fails, look for the first '{' character
                if let Some(json_start) = s.find('{') {
                    let trimmed = &s[json_start..];
                    match Self::parse_json(trimmed) {
                        Ok(output) => Ok(output),
                        Err(_) => {
                            // Return the original error if both attempts fail
                            Err(first_error)
                        }
                    }
                } else {
                    Err(first_error)
                }
            }
        }
    }

    /// Parse a JSON string, returning ParseError with raw JSON if it doesn't match our types
    pub fn parse_json(s: &str) -> Result<ClaudeOutput, ParseError> {
        // First try to parse as a Value
        let value: Value = serde_json::from_str(s).map_err(|e| ParseError {
            raw_json: Value::String(s.to_string()),
            error_message: format!("Invalid JSON: {}", e),
        })?;

        // Then try to parse that Value as ClaudeOutput
        serde_json::from_value::<ClaudeOutput>(value.clone()).map_err(|e| ParseError {
            raw_json: value,
            error_message: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_user_message() {
        let session_uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let input = ClaudeInput::user_message("Hello, Claude!", session_uuid);
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"type\":\"user\""));
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"text\":\"Hello, Claude!\""));
        assert!(json.contains("550e8400-e29b-41d4-a716-446655440000"));
    }

    #[test]
    fn test_deserialize_assistant_message() {
        let json = r#"{
            "type": "assistant",
            "message": {
                "id": "msg_123",
                "role": "assistant",
                "model": "claude-3-sonnet",
                "content": [{"type": "text", "text": "Hello! How can I help you?"}]
            },
            "session_id": "123"
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert!(output.is_assistant_message());
    }

    #[test]
    fn test_deserialize_result_message() {
        let json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 1,
            "result": "Done",
            "session_id": "123",
            "total_cost_usd": 0.01,
            "permission_denials": []
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert!(!output.is_error());
    }

    #[test]
    fn test_deserialize_result_with_permission_denials() {
        let json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 2,
            "result": "Done",
            "session_id": "123",
            "total_cost_usd": 0.01,
            "permission_denials": [
                {
                    "tool_name": "Bash",
                    "tool_input": {"command": "rm -rf /", "description": "Delete everything"},
                    "tool_use_id": "toolu_123"
                }
            ]
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::Result(result) = output {
            assert_eq!(result.permission_denials.len(), 1);
            assert_eq!(result.permission_denials[0].tool_name, "Bash");
            assert_eq!(result.permission_denials[0].tool_use_id, "toolu_123");
            assert_eq!(
                result.permission_denials[0]
                    .tool_input
                    .get("command")
                    .unwrap(),
                "rm -rf /"
            );
        } else {
            panic!("Expected Result");
        }
    }

    #[test]
    fn test_permission_denial_roundtrip() {
        let denial = PermissionDenial {
            tool_name: "Write".to_string(),
            tool_input: serde_json::json!({"file_path": "/etc/passwd", "content": "bad"}),
            tool_use_id: "toolu_456".to_string(),
        };

        let json = serde_json::to_string(&denial).unwrap();
        assert!(json.contains("\"tool_name\":\"Write\""));
        assert!(json.contains("\"tool_use_id\":\"toolu_456\""));
        assert!(json.contains("/etc/passwd"));

        let parsed: PermissionDenial = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, denial);
    }

    // ============================================================================
    // Control Protocol Tests
    // ============================================================================

    #[test]
    fn test_deserialize_control_request_can_use_tool() {
        let json = r#"{
            "type": "control_request",
            "request_id": "perm-abc123",
            "request": {
                "subtype": "can_use_tool",
                "tool_name": "Write",
                "input": {
                    "file_path": "/home/user/hello.py",
                    "content": "print('hello')"
                },
                "permission_suggestions": [],
                "blocked_path": null
            }
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert!(output.is_control_request());

        if let ClaudeOutput::ControlRequest(req) = output {
            assert_eq!(req.request_id, "perm-abc123");
            if let ControlRequestPayload::CanUseTool(perm_req) = req.request {
                assert_eq!(perm_req.tool_name, "Write");
                assert_eq!(
                    perm_req.input.get("file_path").unwrap().as_str().unwrap(),
                    "/home/user/hello.py"
                );
            } else {
                panic!("Expected CanUseTool payload");
            }
        } else {
            panic!("Expected ControlRequest");
        }
    }

    #[test]
    fn test_deserialize_control_request_edit_tool_real() {
        // Real production message from Claude CLI
        let json = r#"{"type":"control_request","request_id":"f3cf357c-17d6-4eca-b498-dd17c7ac43dd","request":{"subtype":"can_use_tool","tool_name":"Edit","input":{"file_path":"/home/meawoppl/repos/cc-proxy/proxy/src/ui.rs","old_string":"/// Print hint to re-authenticate\npub fn print_reauth_hint() {\n    println!(\n        \"  {} Run: {} to re-authenticate\",\n        \"→\".bright_blue(),\n        \"claude-portal logout && claude-portal login\".bright_cyan()\n    );\n}","new_string":"/// Print hint to re-authenticate\npub fn print_reauth_hint() {\n    println!(\n        \"  {} Run: {} to re-authenticate\",\n        \"→\".bright_blue(),\n        \"claude-portal --reauth\".bright_cyan()\n    );\n}","replace_all":false},"permission_suggestions":[{"type":"setMode","mode":"acceptEdits","destination":"session"}],"tool_use_id":"toolu_015BDGtNiqNrRSJSDrWXNckW"}}"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert!(output.is_control_request());
        assert_eq!(output.message_type(), "control_request");

        if let ClaudeOutput::ControlRequest(req) = output {
            assert_eq!(req.request_id, "f3cf357c-17d6-4eca-b498-dd17c7ac43dd");
            if let ControlRequestPayload::CanUseTool(perm_req) = req.request {
                assert_eq!(perm_req.tool_name, "Edit");
                // Verify input contains the expected Edit fields
                assert_eq!(
                    perm_req.input.get("file_path").unwrap().as_str().unwrap(),
                    "/home/meawoppl/repos/cc-proxy/proxy/src/ui.rs"
                );
                assert!(perm_req.input.get("old_string").is_some());
                assert!(perm_req.input.get("new_string").is_some());
                assert_eq!(
                    perm_req
                        .input
                        .get("replace_all")
                        .unwrap()
                        .as_bool()
                        .unwrap(),
                    false
                );
            } else {
                panic!("Expected CanUseTool payload");
            }
        } else {
            panic!("Expected ControlRequest");
        }
    }

    #[test]
    fn test_tool_permission_request_allow() {
        let req = ToolPermissionRequest {
            tool_name: "Read".to_string(),
            input: serde_json::json!({"file_path": "/tmp/test.txt"}),
            permission_suggestions: vec![],
            blocked_path: None,
        };

        let response = req.allow("req-123");
        let message: ControlResponseMessage = response.into();

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"type\":\"control_response\""));
        assert!(json.contains("\"subtype\":\"success\""));
        assert!(json.contains("\"request_id\":\"req-123\""));
        assert!(json.contains("\"behavior\":\"allow\""));
        assert!(json.contains("\"updatedInput\""));
    }

    #[test]
    fn test_tool_permission_request_allow_with_modified_input() {
        let req = ToolPermissionRequest {
            tool_name: "Write".to_string(),
            input: serde_json::json!({"file_path": "/etc/passwd", "content": "test"}),
            permission_suggestions: vec![],
            blocked_path: None,
        };

        let modified_input = serde_json::json!({
            "file_path": "/tmp/safe/passwd",
            "content": "test"
        });
        let response = req.allow_with(modified_input, "req-456");
        let message: ControlResponseMessage = response.into();

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("/tmp/safe/passwd"));
        assert!(!json.contains("/etc/passwd"));
    }

    #[test]
    fn test_tool_permission_request_deny() {
        let req = ToolPermissionRequest {
            tool_name: "Bash".to_string(),
            input: serde_json::json!({"command": "sudo rm -rf /"}),
            permission_suggestions: vec![],
            blocked_path: None,
        };

        let response = req.deny("Dangerous command blocked", "req-789");
        let message: ControlResponseMessage = response.into();

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"behavior\":\"deny\""));
        assert!(json.contains("Dangerous command blocked"));
        assert!(!json.contains("\"interrupt\":true"));
    }

    #[test]
    fn test_tool_permission_request_deny_and_stop() {
        let req = ToolPermissionRequest {
            tool_name: "Bash".to_string(),
            input: serde_json::json!({"command": "rm -rf /"}),
            permission_suggestions: vec![],
            blocked_path: None,
        };

        let response = req.deny_and_stop("Security violation", "req-000");
        let message: ControlResponseMessage = response.into();

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"behavior\":\"deny\""));
        assert!(json.contains("\"interrupt\":true"));
    }

    #[test]
    fn test_permission_result_serialization() {
        // Test allow
        let allow = PermissionResult::allow(serde_json::json!({"test": "value"}));
        let json = serde_json::to_string(&allow).unwrap();
        assert!(json.contains("\"behavior\":\"allow\""));
        assert!(json.contains("\"updatedInput\""));

        // Test deny
        let deny = PermissionResult::deny("Not allowed");
        let json = serde_json::to_string(&deny).unwrap();
        assert!(json.contains("\"behavior\":\"deny\""));
        assert!(json.contains("\"message\":\"Not allowed\""));
        assert!(!json.contains("\"interrupt\""));

        // Test deny with interrupt
        let deny_stop = PermissionResult::deny_and_interrupt("Stop!");
        let json = serde_json::to_string(&deny_stop).unwrap();
        assert!(json.contains("\"interrupt\":true"));
    }

    #[test]
    fn test_control_request_message_initialize() {
        let init = ControlRequestMessage::initialize("init-1");

        let json = serde_json::to_string(&init).unwrap();
        assert!(json.contains("\"type\":\"control_request\""));
        assert!(json.contains("\"request_id\":\"init-1\""));
        assert!(json.contains("\"subtype\":\"initialize\""));
    }

    #[test]
    fn test_control_response_error() {
        let response = ControlResponse::error("req-err", "Something went wrong");
        let message: ControlResponseMessage = response.into();

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"subtype\":\"error\""));
        assert!(json.contains("\"error\":\"Something went wrong\""));
    }

    #[test]
    fn test_roundtrip_control_request() {
        // Test that we can serialize and deserialize control requests
        let original_json = r#"{
            "type": "control_request",
            "request_id": "test-123",
            "request": {
                "subtype": "can_use_tool",
                "tool_name": "Bash",
                "input": {"command": "ls -la"},
                "permission_suggestions": []
            }
        }"#;

        // Parse as ClaudeOutput
        let output: ClaudeOutput = serde_json::from_str(original_json).unwrap();

        // Serialize back and verify key parts are present
        let reserialized = serde_json::to_string(&output).unwrap();
        assert!(reserialized.contains("control_request"));
        assert!(reserialized.contains("test-123"));
        assert!(reserialized.contains("Bash"));
    }

    #[test]
    fn test_permission_suggestions_parsing() {
        // Test that permission_suggestions deserialize correctly with real protocol format
        let json = r#"{
            "type": "control_request",
            "request_id": "perm-456",
            "request": {
                "subtype": "can_use_tool",
                "tool_name": "Bash",
                "input": {"command": "npm test"},
                "permission_suggestions": [
                    {"type": "setMode", "mode": "acceptEdits", "destination": "session"},
                    {"type": "setMode", "mode": "bypassPermissions", "destination": "project"}
                ]
            }
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::ControlRequest(req) = output {
            if let ControlRequestPayload::CanUseTool(perm_req) = req.request {
                assert_eq!(perm_req.permission_suggestions.len(), 2);
                assert_eq!(
                    perm_req.permission_suggestions[0].suggestion_type,
                    "setMode"
                );
                assert_eq!(
                    perm_req.permission_suggestions[0].mode,
                    Some("acceptEdits".to_string())
                );
                assert_eq!(perm_req.permission_suggestions[0].destination, "session");
                assert_eq!(
                    perm_req.permission_suggestions[1].suggestion_type,
                    "setMode"
                );
                assert_eq!(
                    perm_req.permission_suggestions[1].mode,
                    Some("bypassPermissions".to_string())
                );
                assert_eq!(perm_req.permission_suggestions[1].destination, "project");
            } else {
                panic!("Expected CanUseTool payload");
            }
        } else {
            panic!("Expected ControlRequest");
        }
    }

    #[test]
    fn test_permission_suggestion_set_mode_roundtrip() {
        let suggestion = PermissionSuggestion {
            suggestion_type: "setMode".to_string(),
            destination: "session".to_string(),
            mode: Some("acceptEdits".to_string()),
            behavior: None,
            rules: None,
        };

        let json = serde_json::to_string(&suggestion).unwrap();
        assert!(json.contains("\"type\":\"setMode\""));
        assert!(json.contains("\"mode\":\"acceptEdits\""));
        assert!(json.contains("\"destination\":\"session\""));
        assert!(!json.contains("\"behavior\""));
        assert!(!json.contains("\"rules\""));

        let parsed: PermissionSuggestion = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, suggestion);
    }

    #[test]
    fn test_permission_suggestion_add_rules_roundtrip() {
        let suggestion = PermissionSuggestion {
            suggestion_type: "addRules".to_string(),
            destination: "session".to_string(),
            mode: None,
            behavior: Some("allow".to_string()),
            rules: Some(vec![serde_json::json!({
                "toolName": "Read",
                "ruleContent": "//tmp/**"
            })]),
        };

        let json = serde_json::to_string(&suggestion).unwrap();
        assert!(json.contains("\"type\":\"addRules\""));
        assert!(json.contains("\"behavior\":\"allow\""));
        assert!(json.contains("\"destination\":\"session\""));
        assert!(json.contains("\"rules\""));
        assert!(json.contains("\"toolName\":\"Read\""));
        assert!(!json.contains("\"mode\""));

        let parsed: PermissionSuggestion = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, suggestion);
    }

    #[test]
    fn test_permission_suggestion_add_rules_from_real_json() {
        // Real production message from Claude CLI
        let json = r#"{"type":"addRules","rules":[{"toolName":"Read","ruleContent":"//tmp/**"}],"behavior":"allow","destination":"session"}"#;

        let parsed: PermissionSuggestion = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.suggestion_type, "addRules");
        assert_eq!(parsed.destination, "session");
        assert_eq!(parsed.behavior, Some("allow".to_string()));
        assert!(parsed.rules.is_some());
        assert!(parsed.mode.is_none());
    }

    // ============================================================================
    // System Message Subtype Tests
    // ============================================================================

    #[test]
    fn test_system_message_init() {
        let json = r#"{
            "type": "system",
            "subtype": "init",
            "session_id": "test-session-123",
            "cwd": "/home/user/project",
            "model": "claude-sonnet-4",
            "tools": ["Bash", "Read", "Write"],
            "mcp_servers": []
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::System(sys) = output {
            assert!(sys.is_init());
            assert!(!sys.is_status());
            assert!(!sys.is_compact_boundary());

            let init = sys.as_init().expect("Should parse as init");
            assert_eq!(init.session_id, "test-session-123");
            assert_eq!(init.cwd, Some("/home/user/project".to_string()));
            assert_eq!(init.model, Some("claude-sonnet-4".to_string()));
            assert_eq!(init.tools, vec!["Bash", "Read", "Write"]);
        } else {
            panic!("Expected System message");
        }
    }

    #[test]
    fn test_system_message_status() {
        let json = r#"{
            "type": "system",
            "subtype": "status",
            "session_id": "879c1a88-3756-4092-aa95-0020c4ed9692",
            "status": "compacting",
            "uuid": "32eb9f9d-5ef7-47ff-8fce-bbe22fe7ed93"
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::System(sys) = output {
            assert!(sys.is_status());
            assert!(!sys.is_init());

            let status = sys.as_status().expect("Should parse as status");
            assert_eq!(status.session_id, "879c1a88-3756-4092-aa95-0020c4ed9692");
            assert_eq!(status.status, Some("compacting".to_string()));
            assert_eq!(
                status.uuid,
                Some("32eb9f9d-5ef7-47ff-8fce-bbe22fe7ed93".to_string())
            );
        } else {
            panic!("Expected System message");
        }
    }

    #[test]
    fn test_system_message_status_null() {
        let json = r#"{
            "type": "system",
            "subtype": "status",
            "session_id": "879c1a88-3756-4092-aa95-0020c4ed9692",
            "status": null,
            "uuid": "92d9637e-d00e-418e-acd2-a504e3861c6a"
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::System(sys) = output {
            let status = sys.as_status().expect("Should parse as status");
            assert_eq!(status.status, None);
        } else {
            panic!("Expected System message");
        }
    }

    #[test]
    fn test_system_message_compact_boundary() {
        let json = r#"{
            "type": "system",
            "subtype": "compact_boundary",
            "session_id": "879c1a88-3756-4092-aa95-0020c4ed9692",
            "compact_metadata": {
                "pre_tokens": 155285,
                "trigger": "auto"
            },
            "uuid": "a67780d5-74cb-48b1-9137-7a6e7cee45d7"
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::System(sys) = output {
            assert!(sys.is_compact_boundary());
            assert!(!sys.is_init());
            assert!(!sys.is_status());

            let compact = sys
                .as_compact_boundary()
                .expect("Should parse as compact_boundary");
            assert_eq!(compact.session_id, "879c1a88-3756-4092-aa95-0020c4ed9692");
            assert_eq!(compact.compact_metadata.pre_tokens, 155285);
            assert_eq!(compact.compact_metadata.trigger, "auto");
        } else {
            panic!("Expected System message");
        }
    }

    // ============================================================================
    // Helper Method Tests
    // ============================================================================

    #[test]
    fn test_is_system_init() {
        let init_json = r#"{
            "type": "system",
            "subtype": "init",
            "session_id": "test-session"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(init_json).unwrap();
        assert!(output.is_system_init());

        let status_json = r#"{
            "type": "system",
            "subtype": "status",
            "session_id": "test-session"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(status_json).unwrap();
        assert!(!output.is_system_init());
    }

    #[test]
    fn test_session_id() {
        // Result message
        let result_json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 1,
            "session_id": "result-session",
            "total_cost_usd": 0.01
        }"#;
        let output: ClaudeOutput = serde_json::from_str(result_json).unwrap();
        assert_eq!(output.session_id(), Some("result-session"));

        // Assistant message
        let assistant_json = r#"{
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "role": "assistant",
                "model": "claude-3",
                "content": []
            },
            "session_id": "assistant-session"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(assistant_json).unwrap();
        assert_eq!(output.session_id(), Some("assistant-session"));

        // System message
        let system_json = r#"{
            "type": "system",
            "subtype": "init",
            "session_id": "system-session"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(system_json).unwrap();
        assert_eq!(output.session_id(), Some("system-session"));
    }

    #[test]
    fn test_as_tool_use() {
        let json = r#"{
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "role": "assistant",
                "model": "claude-3",
                "content": [
                    {"type": "text", "text": "Let me run that command."},
                    {"type": "tool_use", "id": "tu_1", "name": "Bash", "input": {"command": "ls -la"}},
                    {"type": "tool_use", "id": "tu_2", "name": "Read", "input": {"file_path": "/tmp/test"}}
                ]
            },
            "session_id": "abc"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();

        // Find Bash tool
        let bash = output.as_tool_use("Bash");
        assert!(bash.is_some());
        assert_eq!(bash.unwrap().id, "tu_1");

        // Find Read tool
        let read = output.as_tool_use("Read");
        assert!(read.is_some());
        assert_eq!(read.unwrap().id, "tu_2");

        // Non-existent tool
        assert!(output.as_tool_use("Write").is_none());

        // Not an assistant message
        let result_json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 1,
            "session_id": "abc",
            "total_cost_usd": 0.01
        }"#;
        let result: ClaudeOutput = serde_json::from_str(result_json).unwrap();
        assert!(result.as_tool_use("Bash").is_none());
    }

    #[test]
    fn test_tool_uses() {
        let json = r#"{
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "role": "assistant",
                "model": "claude-3",
                "content": [
                    {"type": "text", "text": "Running commands..."},
                    {"type": "tool_use", "id": "tu_1", "name": "Bash", "input": {"command": "ls"}},
                    {"type": "tool_use", "id": "tu_2", "name": "Read", "input": {"file_path": "/tmp/a"}},
                    {"type": "tool_use", "id": "tu_3", "name": "Write", "input": {"file_path": "/tmp/b", "content": "x"}}
                ]
            },
            "session_id": "abc"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();

        let tools: Vec<_> = output.tool_uses().collect();
        assert_eq!(tools.len(), 3);
        assert_eq!(tools[0].name, "Bash");
        assert_eq!(tools[1].name, "Read");
        assert_eq!(tools[2].name, "Write");
    }

    #[test]
    fn test_text_content() {
        // Single text block
        let json = r#"{
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "role": "assistant",
                "model": "claude-3",
                "content": [{"type": "text", "text": "Hello, world!"}]
            },
            "session_id": "abc"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.text_content(), Some("Hello, world!".to_string()));

        // Multiple text blocks
        let json = r#"{
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "role": "assistant",
                "model": "claude-3",
                "content": [
                    {"type": "text", "text": "Hello, "},
                    {"type": "tool_use", "id": "tu_1", "name": "Bash", "input": {}},
                    {"type": "text", "text": "world!"}
                ]
            },
            "session_id": "abc"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.text_content(), Some("Hello, world!".to_string()));

        // No text blocks
        let json = r#"{
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "role": "assistant",
                "model": "claude-3",
                "content": [{"type": "tool_use", "id": "tu_1", "name": "Bash", "input": {}}]
            },
            "session_id": "abc"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.text_content(), None);

        // Not an assistant message
        let json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 1,
            "session_id": "abc",
            "total_cost_usd": 0.01
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.text_content(), None);
    }

    #[test]
    fn test_as_assistant() {
        let json = r#"{
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "role": "assistant",
                "model": "claude-sonnet-4",
                "content": []
            },
            "session_id": "abc"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();

        let assistant = output.as_assistant();
        assert!(assistant.is_some());
        assert_eq!(assistant.unwrap().message.model, "claude-sonnet-4");

        // Not an assistant
        let result_json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 1,
            "session_id": "abc",
            "total_cost_usd": 0.01
        }"#;
        let result: ClaudeOutput = serde_json::from_str(result_json).unwrap();
        assert!(result.as_assistant().is_none());
    }

    #[test]
    fn test_as_result() {
        let json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 5,
            "session_id": "abc",
            "total_cost_usd": 0.05
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();

        let result = output.as_result();
        assert!(result.is_some());
        assert_eq!(result.unwrap().num_turns, 5);
        assert_eq!(result.unwrap().total_cost_usd, 0.05);

        // Not a result
        let assistant_json = r#"{
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "role": "assistant",
                "model": "claude-3",
                "content": []
            },
            "session_id": "abc"
        }"#;
        let assistant: ClaudeOutput = serde_json::from_str(assistant_json).unwrap();
        assert!(assistant.as_result().is_none());
    }

    #[test]
    fn test_as_system() {
        let json = r#"{
            "type": "system",
            "subtype": "init",
            "session_id": "abc",
            "model": "claude-3"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();

        let system = output.as_system();
        assert!(system.is_some());
        assert!(system.unwrap().is_init());

        // Not a system message
        let result_json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 1,
            "session_id": "abc",
            "total_cost_usd": 0.01
        }"#;
        let result: ClaudeOutput = serde_json::from_str(result_json).unwrap();
        assert!(result.as_system().is_none());
    }

    // ============================================================================
    // ResultMessage Errors Field Tests
    // ============================================================================

    #[test]
    fn test_deserialize_result_message_with_errors() {
        let json = r#"{
            "type": "result",
            "subtype": "error_during_execution",
            "duration_ms": 0,
            "duration_api_ms": 0,
            "is_error": true,
            "num_turns": 0,
            "session_id": "27934753-425a-4182-892c-6b1c15050c3f",
            "total_cost_usd": 0,
            "errors": ["No conversation found with session ID: d56965c9-c855-4042-a8f5-f12bbb14d6f6"],
            "permission_denials": []
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert!(output.is_error());

        if let ClaudeOutput::Result(res) = output {
            assert!(res.is_error);
            assert_eq!(res.errors.len(), 1);
            assert!(res.errors[0].contains("No conversation found"));
        } else {
            panic!("Expected Result message");
        }
    }

    #[test]
    fn test_deserialize_result_message_errors_defaults_empty() {
        // Test that errors field defaults to empty Vec when not present
        let json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 1,
            "session_id": "123",
            "total_cost_usd": 0.01
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::Result(res) = output {
            assert!(res.errors.is_empty());
        } else {
            panic!("Expected Result message");
        }
    }

    #[test]
    fn test_result_message_errors_roundtrip() {
        let json = r#"{
            "type": "result",
            "subtype": "error_during_execution",
            "is_error": true,
            "duration_ms": 0,
            "duration_api_ms": 0,
            "num_turns": 0,
            "session_id": "test-session",
            "total_cost_usd": 0.0,
            "errors": ["Error 1", "Error 2"]
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let reserialized = serde_json::to_string(&output).unwrap();

        // Verify the errors are preserved
        assert!(reserialized.contains("Error 1"));
        assert!(reserialized.contains("Error 2"));
    }
}
