//! Top-level I/O types for Claude communication

use serde::{Deserialize, Serialize};
use serde_json::Value;

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

    /// Raw JSON for untyped messages
    #[serde(untagged)]
    Raw(Value),
}

/// User message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub message: Value, // MessageParam in SDK
    pub session_id: String,
}

/// System initialization message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMessage {
    pub subtype: String, // "init"

    #[serde(rename = "apiKeySource")]
    pub api_key_source: String,

    pub cwd: String,
    pub session_id: String,
    pub tools: Vec<String>,
    pub mcp_servers: Vec<McpServer>,
    pub model: String,

    #[serde(rename = "permissionMode")]
    pub permission_mode: PermissionMode,
}

/// MCP Server info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub name: String,
    pub status: String,
}

/// Assistant message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub message: Value, // Message type in SDK
    pub session_id: String,
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
    /// Create a simple user message input
    pub fn user_message(message: impl Into<String>, session_id: impl Into<String>) -> Self {
        ClaudeInput::User(UserMessage {
            message: Value::String(message.into()),
            session_id: session_id.into(),
        })
    }
}

impl ClaudeOutput {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_user_message() {
        let input = ClaudeInput::user_message("Hello, Claude!", "session-123");
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("Hello, Claude!"));
        assert!(json.contains("session-123"));
    }

    #[test]
    fn test_deserialize_assistant_message() {
        let json = r#"{
            "type": "assistant",
            "message": {"content": "Hello! How can I help you?"},
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
