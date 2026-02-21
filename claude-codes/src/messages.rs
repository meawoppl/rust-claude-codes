//! Message types for the Claude Code protocol

use crate::types::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Base message trait for all protocol messages
pub trait Message: Serialize + for<'de> Deserialize<'de> {
    fn message_type(&self) -> &str;
}

/// Request message sent to Claude Code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    #[serde(rename = "type")]
    pub message_type: String,

    pub id: Id,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,

    pub payload: RequestPayload,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
}

/// Different types of request payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum RequestPayload {
    Initialize(InitializeRequest),
    Execute(ExecuteRequest),
    Complete(CompleteRequest),
    Cancel(CancelRequest),
    GetStatus(GetStatusRequest),
    Custom(Value),
}

/// Initialize a new session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<Vec<EnvironmentVariable>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<Capability>>,
}

/// Environment variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentVariable {
    pub name: String,
    pub value: String,
}

/// Execute a command or task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteRequest {
    pub command: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
}

/// Request completion suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteRequest {
    pub prompt: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<CompletionContext>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_suggestions: Option<usize>,
}

/// Context for completion requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor_position: Option<CursorPosition>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub surrounding_code: Option<String>,
}

/// Cursor position in a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: usize,
    pub column: usize,
}

/// Cancel a running operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelRequest {
    pub target_id: Id,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Get status of an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetStatusRequest {
    pub target_id: Id,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_details: Option<bool>,
}

/// Response message from Claude Code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    #[serde(rename = "type")]
    pub message_type: String,

    pub id: Id,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<Id>,

    pub status: Status,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<ResponsePayload>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorDetail>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
}

/// Different types of response payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result_type", rename_all = "snake_case")]
pub enum ResponsePayload {
    Initialize(InitializeResponse),
    Execute(ExecuteResponse),
    Complete(CompleteResponse),
    Status(StatusResponse),
    Stream(StreamResponse),
    Custom(Value),
}

/// Response to initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResponse {
    pub session_id: SessionId,
    pub version: String,
    pub capabilities: Vec<Capability>,
}

/// Response to execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_output: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_ms: Option<u64>,
}

/// Response to completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteResponse {
    pub suggestions: Vec<CompletionSuggestion>,
}

/// A single completion suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionSuggestion {
    pub text: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<SuggestionMetadata>,
}

/// Metadata for a completion suggestion.
///
/// This struct captures common metadata fields while allowing additional
/// custom fields through the `extra` field.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SuggestionMetadata {
    /// The source of the suggestion (e.g., "history", "model", "cache")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Priority level for the suggestion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,

    /// Category of the suggestion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    /// Any additional metadata fields
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, Value>,
}

/// Status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub status: Status,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<Progress>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<StatusDetails>,
}

/// Details for a status response.
///
/// This struct captures common status detail fields while allowing additional
/// custom fields through the `extra` field.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StatusDetails {
    /// Error message if the status indicates an error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Reason for the current status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Human-readable description of the status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Any additional detail fields
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, Value>,
}

/// Progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progress {
    pub current: usize,
    pub total: Option<usize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub percentage: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Streaming response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamResponse {
    pub chunk: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_id: Option<String>,

    pub is_final: bool,
}

/// Event message for notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    #[serde(rename = "type")]
    pub message_type: String,

    pub event_type: EventType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,

    pub payload: Value,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
}

/// Types of events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Log,
    Progress,
    StateChange,
    Error,
    Warning,
    Info,
    Debug,
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggestion_metadata_parsing() {
        let json = r#"{
            "source": "history",
            "priority": 5,
            "category": "command",
            "custom_field": "custom_value"
        }"#;

        let metadata: SuggestionMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(metadata.source, Some("history".to_string()));
        assert_eq!(metadata.priority, Some(5));
        assert_eq!(metadata.category, Some("command".to_string()));
        assert_eq!(
            metadata.extra.get("custom_field").unwrap(),
            &serde_json::json!("custom_value")
        );
    }

    #[test]
    fn test_suggestion_metadata_minimal() {
        let json = r#"{}"#;

        let metadata: SuggestionMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(metadata.source, None);
        assert_eq!(metadata.priority, None);
        assert!(metadata.extra.is_empty());
    }

    #[test]
    fn test_suggestion_metadata_roundtrip() {
        let mut extra = std::collections::HashMap::new();
        extra.insert("key".to_string(), serde_json::json!("value"));

        let metadata = SuggestionMetadata {
            source: Some("model".to_string()),
            priority: Some(10),
            category: None,
            extra,
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let parsed: SuggestionMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, metadata);
    }

    #[test]
    fn test_status_details_parsing() {
        let json = r#"{
            "error": "Connection failed",
            "reason": "timeout",
            "description": "The server did not respond in time",
            "retry_count": 3
        }"#;

        let details: StatusDetails = serde_json::from_str(json).unwrap();
        assert_eq!(details.error, Some("Connection failed".to_string()));
        assert_eq!(details.reason, Some("timeout".to_string()));
        assert_eq!(
            details.description,
            Some("The server did not respond in time".to_string())
        );
        assert_eq!(
            details.extra.get("retry_count").unwrap(),
            &serde_json::json!(3)
        );
    }

    #[test]
    fn test_status_details_minimal() {
        let json = r#"{}"#;

        let details: StatusDetails = serde_json::from_str(json).unwrap();
        assert_eq!(details.error, None);
        assert_eq!(details.reason, None);
        assert!(details.extra.is_empty());
    }

    #[test]
    fn test_status_details_roundtrip() {
        let details = StatusDetails {
            error: Some("Error message".to_string()),
            reason: None,
            description: Some("Description".to_string()),
            extra: std::collections::HashMap::new(),
        };

        let json = serde_json::to_string(&details).unwrap();
        let parsed: StatusDetails = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, details);
    }

    #[test]
    fn test_completion_suggestion_with_metadata() {
        let json = r#"{
            "text": "git status",
            "description": "Show repository status",
            "score": 0.95,
            "metadata": {
                "source": "history",
                "priority": 1
            }
        }"#;

        let suggestion: CompletionSuggestion = serde_json::from_str(json).unwrap();
        assert_eq!(suggestion.text, "git status");
        assert_eq!(
            suggestion.description,
            Some("Show repository status".to_string())
        );
        assert_eq!(suggestion.score, Some(0.95));
        assert!(suggestion.metadata.is_some());

        let meta = suggestion.metadata.unwrap();
        assert_eq!(meta.source, Some("history".to_string()));
        assert_eq!(meta.priority, Some(1));
    }

    #[test]
    fn test_status_response_with_details() {
        let json = r#"{
            "status": "in_progress",
            "progress": {
                "current": 50,
                "total": 100,
                "percentage": 0.5
            },
            "details": {
                "reason": "processing",
                "description": "Processing request"
            }
        }"#;

        let response: StatusResponse = serde_json::from_str(json).unwrap();
        assert!(matches!(response.status, Status::InProgress));
        assert!(response.progress.is_some());
        assert!(response.details.is_some());

        let details = response.details.unwrap();
        assert_eq!(details.reason, Some("processing".to_string()));
    }
}
