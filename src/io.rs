//! Top-level I/O types for Claude communication

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Top-level enum for all possible Claude input messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeInput {
    /// User message input
    UserMessage(UserMessageInput),

    /// Tool result response
    ToolResult(ToolResultInput),

    /// System control message
    SystemControl(SystemControlInput),

    /// Configuration update
    ConfigUpdate(ConfigUpdateInput),

    /// Session management
    SessionManagement(SessionManagementInput),

    /// Raw JSON for untyped messages
    Raw(Value),
}

/// Top-level enum for all possible Claude output messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeOutput {
    /// Assistant response
    AssistantMessage(AssistantMessageOutput),

    /// Tool use request
    ToolUse(ToolUseOutput),

    /// Status update
    StatusUpdate(StatusUpdateOutput),

    /// Error message
    Error(ErrorOutput),

    /// Metadata/info message
    Metadata(MetadataOutput),

    /// Stream chunk
    StreamChunk(StreamChunkOutput),

    /// Session info
    SessionInfo(SessionInfoOutput),

    /// Result message (completion of a query)
    Result(ResultOutput),

    /// Raw JSON for untyped messages
    Raw(Value),
}

/// User message input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessageInput {
    pub message: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<Attachment>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// Tool result input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultInput {
    pub tool_use_id: String,
    pub result: ToolResult,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// System control input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemControlInput {
    pub action: SystemAction,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
}

/// Configuration update input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigUpdateInput {
    pub config: Value,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<ConfigScope>,
}

/// Session management input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionManagementInput {
    pub action: SessionAction,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
}

/// Assistant message output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessageOutput {
    pub content: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// Tool use request output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseOutput {
    pub tool_use_id: String,
    pub tool_name: String,
    pub parameters: Value,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Status update output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdateOutput {
    pub status: StatusType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<ProgressInfo>,
}

/// Error output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorOutput {
    pub error_type: String,
    pub message: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

/// Metadata output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataOutput {
    pub key: String,
    pub value: Value,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
}

/// Stream chunk output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunkOutput {
    pub delta: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_final: Option<bool>,
}

/// Session info output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfoOutput {
    pub session_id: String,
    pub status: SessionStatus,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// Result output for completed queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultOutput {
    pub subtype: String,
    pub is_error: bool,
    pub duration_ms: u64,
    pub duration_api_ms: u64,
    pub num_turns: u32,
    pub result: String,
    pub session_id: String,
    pub total_cost_usd: f64,
    pub usage: UsageInfo,
    pub permission_denials: Vec<Value>,
    pub uuid: String,
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

/// Attachment for messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub file_type: String,
    pub file_path: Option<String>,
    pub content: Option<String>,
    pub encoding: Option<String>,
}

/// Tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResult {
    Text(String),
    Structured(Value),
}

/// System actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemAction {
    Pause,
    Resume,
    Cancel,
    Reset,
    Ping,
    Custom(String),
}

/// Configuration scope
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigScope {
    Session,
    Global,
    Project,
}

/// Session actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionAction {
    Create,
    Resume,
    End,
    List,
    Clear,
}

/// Status types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusType {
    Idle,
    Processing,
    WaitingForInput,
    ToolUse,
    Thinking,
    Responding,
    Error,
}

/// Progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressInfo {
    pub current: usize,
    pub total: Option<usize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub percentage: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Session status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Paused,
    Ended,
    Error,
}

impl ClaudeInput {
    /// Create a simple user message input
    pub fn user_message(message: impl Into<String>) -> Self {
        ClaudeInput::UserMessage(UserMessageInput {
            message: message.into(),
            conversation_id: None,
            attachments: None,
            metadata: None,
        })
    }

    /// Create a tool result input
    pub fn tool_result(tool_use_id: impl Into<String>, result: ToolResult) -> Self {
        ClaudeInput::ToolResult(ToolResultInput {
            tool_use_id: tool_use_id.into(),
            result,
            error: None,
        })
    }
}

impl ClaudeOutput {
    /// Check if this is an error output
    pub fn is_error(&self) -> bool {
        matches!(self, ClaudeOutput::Error(_))
    }

    /// Check if this is a tool use request
    pub fn is_tool_use(&self) -> bool {
        matches!(self, ClaudeOutput::ToolUse(_))
    }

    /// Check if this is an assistant message
    pub fn is_assistant_message(&self) -> bool {
        matches!(self, ClaudeOutput::AssistantMessage(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_user_message() {
        let input = ClaudeInput::user_message("Hello, Claude!");
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("user_message"));
        assert!(json.contains("Hello, Claude!"));
    }

    #[test]
    fn test_deserialize_assistant_message() {
        let json = r#"{
            "type": "assistant_message",
            "content": "Hello! How can I help you?",
            "conversation_id": "123"
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert!(output.is_assistant_message());
    }

    #[test]
    fn test_tool_result() {
        let result = ClaudeInput::tool_result("tool-123", ToolResult::Text("Success".to_string()));
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("tool_result"));
        assert!(json.contains("tool-123"));
    }
}
