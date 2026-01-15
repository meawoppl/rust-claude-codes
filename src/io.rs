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
    pub usage: Option<serde_json::Value>,
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

// ============================================================================
// Task Notification Types
// ============================================================================

/// Status of a background task
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
    /// Task is still running
    Running,
    /// Unknown status (forward compatibility)
    #[serde(other)]
    Unknown,
}

/// A task notification embedded in user message text.
///
/// Claude Code emits these XML-like notifications when background tasks complete.
/// They are embedded in the text content of user messages.
///
/// # Example
///
/// ```
/// use claude_codes::TaskNotification;
///
/// let text = r#"<task-notification>
/// <task-id>b1c496c</task-id>
/// <output-file>/tmp/claude/tasks/b1c496c.output</output-file>
/// <status>completed</status>
/// <summary>Background command "git status" completed (exit code 0)</summary>
/// </task-notification>
/// Read the output file to retrieve the result."#;
///
/// if let Some(notification) = TaskNotification::parse(text) {
///     assert_eq!(notification.task_id, "b1c496c");
///     assert_eq!(notification.status, claude_codes::TaskStatus::Completed);
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskNotification {
    /// Unique identifier for the task
    pub task_id: String,
    /// Path to the file containing task output
    pub output_file: String,
    /// Current status of the task
    pub status: TaskStatus,
    /// Human-readable summary of the task result
    pub summary: String,
}

impl TaskNotification {
    /// Parse a task notification from text content.
    ///
    /// Returns `Some(TaskNotification)` if the text contains a valid
    /// `<task-notification>` block, otherwise returns `None`.
    ///
    /// # Example
    ///
    /// ```
    /// use claude_codes::TaskNotification;
    ///
    /// let text = "<task-notification>\n<task-id>abc123</task-id>\n<output-file>/tmp/out.txt</output-file>\n<status>completed</status>\n<summary>Done</summary>\n</task-notification>";
    /// let notification = TaskNotification::parse(text).unwrap();
    /// assert_eq!(notification.task_id, "abc123");
    /// ```
    pub fn parse(text: &str) -> Option<Self> {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        // Find the task-notification block
        let start_tag = "<task-notification>";
        let end_tag = "</task-notification>";

        let start_idx = text.find(start_tag)?;
        let end_idx = text.find(end_tag)?;

        if end_idx <= start_idx {
            return None;
        }

        let xml_content = &text[start_idx..end_idx + end_tag.len()];

        let mut reader = Reader::from_str(xml_content);
        reader.config_mut().trim_text(true);

        let mut task_id = None;
        let mut output_file = None;
        let mut status = None;
        let mut summary = None;

        let mut current_element: Option<String> = None;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    current_element = Some(
                        String::from_utf8_lossy(e.name().as_ref()).to_string(),
                    );
                }
                Ok(Event::Text(e)) => {
                    if let Some(ref elem) = current_element {
                        let text_content = e.unescape().ok()?.to_string();
                        match elem.as_str() {
                            "task-id" => task_id = Some(text_content),
                            "output-file" => output_file = Some(text_content),
                            "status" => status = Some(text_content),
                            "summary" => summary = Some(text_content),
                            _ => {}
                        }
                    }
                }
                Ok(Event::End(_)) => {
                    current_element = None;
                }
                Ok(Event::Eof) => break,
                Err(_) => return None,
                _ => {}
            }
        }

        Some(TaskNotification {
            task_id: task_id?,
            output_file: output_file?,
            status: match status?.as_str() {
                "completed" => TaskStatus::Completed,
                "failed" => TaskStatus::Failed,
                "running" => TaskStatus::Running,
                _ => TaskStatus::Unknown,
            },
            summary: summary?,
        })
    }

    /// Parse all task notifications from text content.
    ///
    /// Returns a vector of all `<task-notification>` blocks found in the text.
    /// Useful when a message might contain multiple notifications.
    pub fn parse_all(text: &str) -> Vec<Self> {
        let mut notifications = Vec::new();
        let mut search_start = 0;

        while let Some(start_idx) = text[search_start..].find("<task-notification>") {
            let absolute_start = search_start + start_idx;
            if let Some(end_idx) = text[absolute_start..].find("</task-notification>") {
                let absolute_end = absolute_start + end_idx + "</task-notification>".len();
                if let Some(notification) = Self::parse(&text[absolute_start..absolute_end]) {
                    notifications.push(notification);
                }
                search_start = absolute_end;
            } else {
                break;
            }
        }

        notifications
    }

    /// Check if the given text contains a task notification.
    pub fn contains_notification(text: &str) -> bool {
        text.contains("<task-notification>")
    }

    /// Extract the remaining text after removing task notifications.
    ///
    /// Returns the text with all `<task-notification>...</task-notification>` blocks removed.
    pub fn extract_remaining_text(text: &str) -> String {
        let mut result = text.to_string();
        while let Some(start_idx) = result.find("<task-notification>") {
            if let Some(end_offset) = result[start_idx..].find("</task-notification>") {
                let end_idx = start_idx + end_offset + "</task-notification>".len();
                result = format!("{}{}", &result[..start_idx], &result[end_idx..]);
            } else {
                break;
            }
        }
        result.trim().to_string()
    }
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

    #[serde(default)]
    pub permission_denials: Vec<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
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
    /// Suggested permissions (if any)
    #[serde(default)]
    pub permission_suggestions: Vec<Value>,
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

    // ============================================================================
    // Task Notification Tests
    // ============================================================================

    #[test]
    fn test_task_notification_parse_basic() {
        let text = r#"<task-notification>
<task-id>b1c496c</task-id>
<output-file>/tmp/claude/-home-meawoppl-repos-meter-sim/tasks/b1c496c.output</output-file>
<status>completed</status>
<summary>Background command "Commit merge" completed (exit code 0)</summary>
</task-notification>
Read the output file to retrieve the result: /tmp/claude/-home-meawoppl-repos-meter-sim/tasks/b1c496c.output"#;

        let notification = TaskNotification::parse(text).expect("Should parse notification");

        assert_eq!(notification.task_id, "b1c496c");
        assert_eq!(
            notification.output_file,
            "/tmp/claude/-home-meawoppl-repos-meter-sim/tasks/b1c496c.output"
        );
        assert_eq!(notification.status, TaskStatus::Completed);
        assert_eq!(
            notification.summary,
            "Background command \"Commit merge\" completed (exit code 0)"
        );
    }

    #[test]
    fn test_task_notification_parse_failed_status() {
        let text = r#"<task-notification>
<task-id>xyz789</task-id>
<output-file>/tmp/tasks/xyz789.output</output-file>
<status>failed</status>
<summary>Command failed with exit code 1</summary>
</task-notification>"#;

        let notification = TaskNotification::parse(text).expect("Should parse notification");
        assert_eq!(notification.status, TaskStatus::Failed);
    }

    #[test]
    fn test_task_notification_parse_running_status() {
        let text = r#"<task-notification>
<task-id>run123</task-id>
<output-file>/tmp/tasks/run123.output</output-file>
<status>running</status>
<summary>Task is still running</summary>
</task-notification>"#;

        let notification = TaskNotification::parse(text).expect("Should parse notification");
        assert_eq!(notification.status, TaskStatus::Running);
    }

    #[test]
    fn test_task_notification_parse_unknown_status() {
        let text = r#"<task-notification>
<task-id>unk456</task-id>
<output-file>/tmp/tasks/unk456.output</output-file>
<status>some_future_status</status>
<summary>Unknown status type</summary>
</task-notification>"#;

        let notification = TaskNotification::parse(text).expect("Should parse notification");
        assert_eq!(notification.status, TaskStatus::Unknown);
    }

    #[test]
    fn test_task_notification_parse_no_notification() {
        let text = "This is just regular text without any notification.";
        assert!(TaskNotification::parse(text).is_none());
    }

    #[test]
    fn test_task_notification_parse_incomplete() {
        let text = "<task-notification><task-id>abc</task-id></task-notification>";
        // Missing required fields, should return None
        assert!(TaskNotification::parse(text).is_none());
    }

    #[test]
    fn test_task_notification_contains_notification() {
        assert!(TaskNotification::contains_notification("<task-notification>...</task-notification>"));
        assert!(!TaskNotification::contains_notification("just regular text"));
    }

    #[test]
    fn test_task_notification_extract_remaining_text() {
        let text = r#"<task-notification>
<task-id>abc</task-id>
<output-file>/tmp/out.txt</output-file>
<status>completed</status>
<summary>Done</summary>
</task-notification>
Read the output file to retrieve the result: /tmp/out.txt"#;

        let remaining = TaskNotification::extract_remaining_text(text);
        assert_eq!(remaining, "Read the output file to retrieve the result: /tmp/out.txt");
    }

    #[test]
    fn test_task_notification_parse_all_multiple() {
        let text = r#"<task-notification>
<task-id>task1</task-id>
<output-file>/tmp/task1.out</output-file>
<status>completed</status>
<summary>First task done</summary>
</task-notification>
Some text in between
<task-notification>
<task-id>task2</task-id>
<output-file>/tmp/task2.out</output-file>
<status>failed</status>
<summary>Second task failed</summary>
</task-notification>"#;

        let notifications = TaskNotification::parse_all(text);
        assert_eq!(notifications.len(), 2);
        assert_eq!(notifications[0].task_id, "task1");
        assert_eq!(notifications[0].status, TaskStatus::Completed);
        assert_eq!(notifications[1].task_id, "task2");
        assert_eq!(notifications[1].status, TaskStatus::Failed);
    }

    #[test]
    fn test_task_status_serialization() {
        // Test that TaskStatus serializes to lowercase
        let completed = TaskStatus::Completed;
        let json = serde_json::to_string(&completed).unwrap();
        assert_eq!(json, "\"completed\"");

        let failed = TaskStatus::Failed;
        let json = serde_json::to_string(&failed).unwrap();
        assert_eq!(json, "\"failed\"");

        let running = TaskStatus::Running;
        let json = serde_json::to_string(&running).unwrap();
        assert_eq!(json, "\"running\"");
    }

    #[test]
    fn test_task_notification_serialization() {
        let notification = TaskNotification {
            task_id: "test123".to_string(),
            output_file: "/tmp/test.out".to_string(),
            status: TaskStatus::Completed,
            summary: "Test completed".to_string(),
        };

        let json = serde_json::to_string(&notification).unwrap();
        assert!(json.contains("\"task_id\":\"test123\""));
        assert!(json.contains("\"output_file\":\"/tmp/test.out\""));
        assert!(json.contains("\"status\":\"completed\""));
        assert!(json.contains("\"summary\":\"Test completed\""));
    }
}
