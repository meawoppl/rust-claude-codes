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
    pub metadata: Option<Value>,
}

/// Status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub status: Status,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<Progress>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
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
