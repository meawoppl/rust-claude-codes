use serde::{Deserialize, Serialize};
use serde_json::Value;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::ClaudeOutput;

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

        assert!(reserialized.contains("Error 1"));
        assert!(reserialized.contains("Error 2"));
    }
}
