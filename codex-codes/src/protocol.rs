//! App-server v2 protocol types for the Codex CLI.
//!
//! These types represent the JSON-RPC request parameters, response payloads,
//! and notification bodies used by `codex app-server`. All wire types use
//! camelCase field names.

use crate::io::items::ThreadItem;
use crate::jsonrpc::RequestId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// User input
// ---------------------------------------------------------------------------

/// User input sent as part of a turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum UserInput {
    /// Text input.
    Text { text: String },
    /// Pre-encoded image (data URI).
    Image { data: String },
}

// ---------------------------------------------------------------------------
// Thread lifecycle requests
// ---------------------------------------------------------------------------

/// Parameters for `thread/start`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadStartParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Value>>,
}

/// Response from `thread/start`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadStartResponse {
    pub thread_id: String,
}

/// Parameters for `thread/archive`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadArchiveParams {
    pub thread_id: String,
}

/// Response from `thread/archive`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadArchiveResponse {}

// ---------------------------------------------------------------------------
// Turn lifecycle requests
// ---------------------------------------------------------------------------

/// Parameters for `turn/start`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnStartParams {
    pub thread_id: String,
    pub input: Vec<UserInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox_policy: Option<Value>,
}

/// Response from `turn/start`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnStartResponse {}

/// Parameters for `turn/interrupt`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnInterruptParams {
    pub thread_id: String,
}

/// Response from `turn/interrupt`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnInterruptResponse {}

// ---------------------------------------------------------------------------
// Turn status & data types
// ---------------------------------------------------------------------------

/// Status of a turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TurnStatus {
    Completed,
    Interrupted,
    Failed,
    InProgress,
}

/// Error information from a failed turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnError {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codex_error_info: Option<Value>,
}

/// A completed turn with its items and status.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Turn {
    pub id: String,
    #[serde(default)]
    pub items: Vec<ThreadItem>,
    pub status: TurnStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<TurnError>,
}

// ---------------------------------------------------------------------------
// Token usage
// ---------------------------------------------------------------------------

/// Token usage for a thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default)]
    pub cached_input_tokens: u64,
}

// ---------------------------------------------------------------------------
// Thread status
// ---------------------------------------------------------------------------

/// Status of a thread.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThreadStatus {
    NotLoaded,
    Idle,
    Active,
    SystemError,
}

// ---------------------------------------------------------------------------
// Server notifications
// ---------------------------------------------------------------------------

/// `thread/started` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadStartedNotification {
    pub thread_id: String,
}

/// `thread/status/changed` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadStatusChangedNotification {
    pub thread_id: String,
    pub status: ThreadStatus,
}

/// `turn/started` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnStartedNotification {
    pub thread_id: String,
    pub turn_id: String,
}

/// `turn/completed` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnCompletedNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub turn: Turn,
}

/// `item/started` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemStartedNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item: ThreadItem,
}

/// `item/completed` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemCompletedNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item: ThreadItem,
}

/// `item/agentMessage/delta` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessageDeltaNotification {
    pub thread_id: String,
    pub item_id: String,
    pub delta: String,
}

/// `item/commandExecution/outputDelta` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CmdOutputDeltaNotification {
    pub thread_id: String,
    pub item_id: String,
    pub delta: String,
}

/// `item/fileChange/outputDelta` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChangeOutputDeltaNotification {
    pub thread_id: String,
    pub item_id: String,
    pub delta: String,
}

/// `item/reasoning/summaryTextDelta` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningDeltaNotification {
    pub thread_id: String,
    pub item_id: String,
    pub delta: String,
}

/// `error` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorNotification {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(default)]
    pub will_retry: bool,
}

/// `thread/tokenUsage/updated` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadTokenUsageUpdatedNotification {
    pub thread_id: String,
    pub usage: TokenUsage,
}

// ---------------------------------------------------------------------------
// Approval flow types (server-to-client requests)
// ---------------------------------------------------------------------------

/// Decision for command execution approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandApprovalDecision {
    Accept,
    AcceptForSession,
    Decline,
    Cancel,
}

/// Parameters for `item/commandExecution/requestApproval` (server → client).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandExecutionApprovalParams {
    pub thread_id: String,
    pub turn_id: String,
    pub call_id: String,
    pub command: String,
    pub cwd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Response for `item/commandExecution/requestApproval`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandExecutionApprovalResponse {
    pub decision: CommandApprovalDecision,
}

/// Decision for file change approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileChangeApprovalDecision {
    Accept,
    AcceptForSession,
    Decline,
    Cancel,
}

/// Parameters for `item/fileChange/requestApproval` (server → client).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChangeApprovalParams {
    pub thread_id: String,
    pub turn_id: String,
    pub call_id: String,
    pub changes: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Response for `item/fileChange/requestApproval`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChangeApprovalResponse {
    pub decision: FileChangeApprovalDecision,
}

// ---------------------------------------------------------------------------
// Server message (what the client receives)
// ---------------------------------------------------------------------------

/// An incoming message from the app-server that the client should handle.
#[derive(Debug, Clone)]
pub enum ServerMessage {
    /// A notification (no response needed).
    Notification {
        method: String,
        params: Option<Value>,
    },
    /// A request from the server that needs a response (e.g., approval flow).
    Request {
        id: RequestId,
        method: String,
        params: Option<Value>,
    },
}

// ---------------------------------------------------------------------------
// Method name constants
// ---------------------------------------------------------------------------

/// JSON-RPC method names used by the app-server protocol.
pub mod methods {
    // Client → server requests
    pub const THREAD_START: &str = "thread/start";
    pub const THREAD_ARCHIVE: &str = "thread/archive";
    pub const TURN_START: &str = "turn/start";
    pub const TURN_INTERRUPT: &str = "turn/interrupt";
    pub const TURN_STEER: &str = "turn/steer";

    // Server → client notifications
    pub const THREAD_STARTED: &str = "thread/started";
    pub const THREAD_STATUS_CHANGED: &str = "thread/status/changed";
    pub const THREAD_TOKEN_USAGE_UPDATED: &str = "thread/tokenUsage/updated";
    pub const TURN_STARTED: &str = "turn/started";
    pub const TURN_COMPLETED: &str = "turn/completed";
    pub const ITEM_STARTED: &str = "item/started";
    pub const ITEM_COMPLETED: &str = "item/completed";
    pub const AGENT_MESSAGE_DELTA: &str = "item/agentMessage/delta";
    pub const CMD_OUTPUT_DELTA: &str = "item/commandExecution/outputDelta";
    pub const FILE_CHANGE_OUTPUT_DELTA: &str = "item/fileChange/outputDelta";
    pub const REASONING_DELTA: &str = "item/reasoning/summaryTextDelta";
    pub const ERROR: &str = "error";

    // Server → client requests (approval)
    pub const CMD_EXEC_APPROVAL: &str = "item/commandExecution/requestApproval";
    pub const FILE_CHANGE_APPROVAL: &str = "item/fileChange/requestApproval";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_input_text() {
        let input = UserInput::Text {
            text: "Hello".to_string(),
        };
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains(r#""type":"text""#));
        let parsed: UserInput = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, UserInput::Text { text } if text == "Hello"));
    }

    #[test]
    fn test_thread_start_params() {
        let params = ThreadStartParams {
            instructions: Some("Be helpful".to_string()),
            tools: None,
        };
        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("instructions"));
        assert!(!json.contains("tools"));
    }

    #[test]
    fn test_thread_start_response() {
        let json = r#"{"threadId":"th_abc123"}"#;
        let resp: ThreadStartResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.thread_id, "th_abc123");
    }

    #[test]
    fn test_turn_start_params() {
        let params = TurnStartParams {
            thread_id: "th_1".to_string(),
            input: vec![UserInput::Text {
                text: "What is 2+2?".to_string(),
            }],
            model: None,
            reasoning_effort: None,
            sandbox_policy: None,
        };
        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("threadId"));
        assert!(json.contains("input"));
    }

    #[test]
    fn test_turn_status() {
        let json = r#""completed""#;
        let status: TurnStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status, TurnStatus::Completed);
    }

    #[test]
    fn test_turn_completed_notification() {
        let json = r#"{
            "threadId": "th_1",
            "turnId": "t_1",
            "turn": {
                "id": "t_1",
                "items": [],
                "status": "completed"
            }
        }"#;
        let notif: TurnCompletedNotification = serde_json::from_str(json).unwrap();
        assert_eq!(notif.thread_id, "th_1");
        assert_eq!(notif.turn.status, TurnStatus::Completed);
    }

    #[test]
    fn test_agent_message_delta() {
        let json = r#"{"threadId":"th_1","itemId":"msg_1","delta":"Hello "}"#;
        let notif: AgentMessageDeltaNotification = serde_json::from_str(json).unwrap();
        assert_eq!(notif.delta, "Hello ");
    }

    #[test]
    fn test_command_approval_decision() {
        let json = r#""accept""#;
        let decision: CommandApprovalDecision = serde_json::from_str(json).unwrap();
        assert_eq!(decision, CommandApprovalDecision::Accept);

        let json = r#""acceptForSession""#;
        let decision: CommandApprovalDecision = serde_json::from_str(json).unwrap();
        assert_eq!(decision, CommandApprovalDecision::AcceptForSession);
    }

    #[test]
    fn test_command_approval_params() {
        let json = r#"{
            "threadId": "th_1",
            "turnId": "t_1",
            "callId": "call_1",
            "command": "rm -rf /tmp/test",
            "cwd": "/home/user"
        }"#;
        let params: CommandExecutionApprovalParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.command, "rm -rf /tmp/test");
    }

    #[test]
    fn test_error_notification() {
        let json = r#"{"error":"something failed","willRetry":true}"#;
        let notif: ErrorNotification = serde_json::from_str(json).unwrap();
        assert_eq!(notif.error, "something failed");
        assert!(notif.will_retry);
    }

    #[test]
    fn test_thread_status() {
        let json = r#""idle""#;
        let status: ThreadStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status, ThreadStatus::Idle);
    }

    #[test]
    fn test_token_usage() {
        let json = r#"{"inputTokens":100,"outputTokens":200,"cachedInputTokens":50}"#;
        let usage: TokenUsage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 200);
        assert_eq!(usage.cached_input_tokens, 50);
    }
}
