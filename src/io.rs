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
use tracing::debug;
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
    pub content: Vec<ContentBlock>,
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

/// Result message for completed queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultMessage {
    pub subtype: ResultSubtype,
    pub is_error: bool,
    pub duration_ms: u64,
    pub duration_api_ms: u64,
    pub num_turns: u32,

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

    /// Parse a JSON string, returning ParseError with raw JSON if it doesn't match our types
    pub fn parse_json(s: &str) -> Result<ClaudeOutput, ParseError> {
        debug!("[IO] Attempting to parse JSON: {}", s);

        // First try to parse as a Value
        let value: Value = serde_json::from_str(s).map_err(|e| {
            debug!("[IO] Failed to parse as JSON Value: {}", e);
            ParseError {
                raw_json: Value::String(s.to_string()),
                error_message: format!("Invalid JSON: {}", e),
            }
        })?;

        debug!("[IO] Successfully parsed as JSON Value, attempting to deserialize as ClaudeOutput");

        // Then try to parse that Value as ClaudeOutput
        serde_json::from_value::<ClaudeOutput>(value.clone()).map_err(|e| {
            debug!("[IO] Failed to deserialize as ClaudeOutput: {}", e);
            ParseError {
                raw_json: value,
                error_message: e.to_string(),
            }
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
}
