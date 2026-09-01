//! The `pi --mode rpc` command protocol: JSON objects on stdin (one per
//! line, LF-delimited), `{"type":"response",...}` acknowledgements and
//! streamed events on stdout.
//!
//! Framing contract (from `docs/rpc.md`): split records on `\n` ONLY,
//! tolerate a trailing `\r` — never use a splitter that also breaks on
//! U+2028/U+2029, which are legal inside JSON strings.

use crate::io::{Model, PiMessage};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// An image attached to a `prompt`/`steer`/`follow_up` command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageContent {
    /// Always `"image"`.
    #[serde(rename = "type")]
    pub content_type: String,
    /// Base64-encoded bytes.
    pub data: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
}

/// How a `prompt` sent mid-stream should queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamingBehavior {
    /// Deliver after the current assistant turn's tool calls, before the
    /// next LLM call.
    #[serde(rename = "steer")]
    Steer,
    /// Deliver only once the agent stops.
    #[serde(rename = "followUp")]
    FollowUp,
}

/// A client → agent command. Serializes with the wire's `type` tag; the
/// optional `id` correlates the matching [`RpcResponse`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RpcCommand {
    Prompt {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        images: Option<Vec<ImageContent>>,
        #[serde(rename = "streamingBehavior", skip_serializing_if = "Option::is_none")]
        streaming_behavior: Option<StreamingBehavior>,
    },
    Steer {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        images: Option<Vec<ImageContent>>,
    },
    FollowUp {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        images: Option<Vec<ImageContent>>,
    },
    Abort {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    ClearQueue {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    NewSession {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    GetState {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    GetMessages {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    SetModel {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        provider: String,
        #[serde(rename = "modelId")]
        model_id: String,
    },
    CycleModel {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    GetAvailableModels {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    SetThinkingLevel {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        level: String,
    },
    CycleThinkingLevel {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    GetAvailableThinkingLevels {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    SetSteeringMode {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        mode: String,
    },
    SetFollowUpMode {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        mode: String,
    },
    Compact {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        instructions: Option<String>,
    },
    SetAutoCompaction {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        enabled: bool,
    },
    SetAutoRetry {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        enabled: bool,
    },
    AbortRetry {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    /// Run a shell command outside the model; output joins context on the
    /// NEXT prompt. Streams `bash_execution_update` events carrying this
    /// command's `id`.
    Bash {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        command: String,
    },
    AbortBash {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    GetSessionStats {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    ExportHtml {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
    },
    SetSessionName {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        name: String,
    },
    /// Anything this crate doesn't model yet — serialized as its raw
    /// object so new commands work without a crate release.
    #[serde(untagged)]
    Raw(Value),
}

impl RpcCommand {
    /// Serialize to one LF-terminated wire line.
    pub fn to_line(&self) -> Result<String, serde_json::Error> {
        Ok(format!("{}\n", serde_json::to_string(self)?))
    }
}

/// The `{"type":"response"}` acknowledgement for one command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Which command this answers (`"parse"` for unparseable input).
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Typed view of `get_state` response data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AgentState {
    /// `None` when no provider resolved; may also be a placeholder with
    /// `id: "unknown"` — see [`Model`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<Model>,
    #[serde(rename = "thinkingLevel", default)]
    pub thinking_level: String,
    #[serde(rename = "isStreaming", default)]
    pub is_streaming: bool,
    #[serde(rename = "isCompacting", default)]
    pub is_compacting: bool,
    #[serde(rename = "steeringMode", default)]
    pub steering_mode: String,
    #[serde(rename = "followUpMode", default)]
    pub follow_up_mode: String,
    #[serde(
        rename = "sessionFile",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub session_file: Option<String>,
    #[serde(rename = "sessionId", default)]
    pub session_id: String,
    #[serde(
        rename = "sessionName",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub session_name: Option<String>,
    #[serde(rename = "autoCompactionEnabled", default)]
    pub auto_compaction_enabled: bool,
    #[serde(rename = "messageCount", default)]
    pub message_count: i64,
    #[serde(rename = "pendingMessageCount", default)]
    pub pending_message_count: i64,
    #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra: serde_json::Map<String, Value>,
}

/// Typed view of `bash` response data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BashResult {
    #[serde(default)]
    pub output: String,
    #[serde(rename = "exitCode", default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i64>,
    #[serde(default)]
    pub cancelled: bool,
    #[serde(default)]
    pub truncated: bool,
    #[serde(
        rename = "fullOutputPath",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub full_output_path: Option<String>,
}

/// Typed view of `get_messages` response data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Messages {
    #[serde(default)]
    pub messages: Vec<PiMessage>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_serialize_with_snake_case_type_tags() {
        let c = RpcCommand::GetState {
            id: Some("r1".into()),
        };
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["type"], "get_state");
        assert_eq!(v["id"], "r1");

        let c = RpcCommand::Prompt {
            id: None,
            message: "hi".into(),
            images: None,
            streaming_behavior: Some(StreamingBehavior::Steer),
        };
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["type"], "prompt");
        assert_eq!(v["streamingBehavior"], "steer");
        assert!(v.get("images").is_none());
    }

    #[test]
    fn response_envelope_parses_success_and_error() {
        let r: RpcResponse = serde_json::from_str(
            r#"{"id":"r1","type":"response","command":"get_state","success":true,"data":{}}"#,
        )
        .unwrap();
        assert!(r.success);
        assert_eq!(r.command, "get_state");

        let r: RpcResponse = serde_json::from_str(
            r#"{"type":"response","command":"set_model","success":false,"error":"Model not found: invalid/model"}"#,
        )
        .unwrap();
        assert!(!r.success);
        assert!(r.error.unwrap().contains("Model not found"));
    }

    #[test]
    fn agent_state_parses_credential_free_placeholder() {
        let s: AgentState = serde_json::from_value(serde_json::json!({
            "model": {"id":"unknown","name":"unknown","api":"unknown","provider":"unknown",
                      "baseUrl":"","reasoning":false,"input":[],
                      "cost":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0},
                      "contextWindow":0,"maxTokens":0},
            "thinkingLevel":"off","isStreaming":false,"isCompacting":false,
            "steeringMode":"one-at-a-time","followUpMode":"one-at-a-time",
            "sessionId":"01a05ec0-4e0e-7dd4-a225-2e78cccd0059",
            "autoCompactionEnabled":true,"messageCount":0,"pendingMessageCount":0
        }))
        .unwrap();
        assert_eq!(s.model.unwrap().context_window, 0);
        assert_eq!(s.message_count, 0);
    }

    #[test]
    fn raw_command_passes_through_unmodeled_types() {
        let c = RpcCommand::Raw(serde_json::json!({"type": "get_tree", "id": "r9"}));
        let line = c.to_line().unwrap();
        assert!(line.ends_with('\n'));
        assert!(line.contains("\"get_tree\""));
    }
}
