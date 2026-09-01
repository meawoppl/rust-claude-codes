//! Typed models of pi's wire vocabulary: the messages and events shared
//! by `--mode json` (one-shot JSONL event stream) and `--mode rpc`
//! (command protocol; see [`crate::rpc`]).
//!
//! Shapes mirror the documented TypeScript types
//! (`packages/ai/src/types.ts`, `packages/agent/src/types.ts`, and
//! `packages/coding-agent/docs/{json,rpc}.md` in earendil-works/pi).
//! Unknown event types are preserved verbatim in [`PiEvent::Unknown`]
//! rather than erroring, so a newer CLI degrades soft.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Per-request/response token accounting on assistant and tool messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Usage {
    #[serde(default)]
    pub input: f64,
    #[serde(default)]
    pub output: f64,
    #[serde(rename = "cacheRead", default)]
    pub cache_read: f64,
    #[serde(rename = "cacheWrite", default)]
    pub cache_write: f64,
    #[serde(
        rename = "totalTokens",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub total_tokens: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<UsageCost>,
    #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra: serde_json::Map<String, Value>,
}

/// Dollar cost breakdown inside [`Usage`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UsageCost {
    #[serde(default)]
    pub input: f64,
    #[serde(default)]
    pub output: f64,
    #[serde(rename = "cacheRead", default)]
    pub cache_read: f64,
    #[serde(rename = "cacheWrite", default)]
    pub cache_write: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
}

/// A model catalog entry (`get_state`/`set_model`/`get_available_models`).
/// With no provider configured, `get_state` returns a placeholder with
/// `id: "unknown"` and zeroed limits — treat `contextWindow == 0` as
/// "no real model resolved".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Model {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub api: String,
    #[serde(default)]
    pub provider: String,
    #[serde(rename = "baseUrl", default)]
    pub base_url: String,
    #[serde(default)]
    pub reasoning: bool,
    #[serde(default)]
    pub input: Vec<String>,
    #[serde(rename = "contextWindow", default)]
    pub context_window: i64,
    #[serde(rename = "maxTokens", default)]
    pub max_tokens: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<UsageCost>,
    #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra: serde_json::Map<String, Value>,
}

/// One block of assistant content: text, thinking, or a tool call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Thinking {
        thinking: String,
    },
    ToolCall {
        id: String,
        name: String,
        arguments: Value,
    },
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
}

/// A conversation message, discriminated on `role`. This is pi's
/// `AgentMessage` union plus the RPC-only `bashExecution` role.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "camelCase")]
pub enum PiMessage {
    User {
        /// A plain string or an array of content blocks — both are legal.
        content: Value,
        #[serde(default)]
        timestamp: f64,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        attachments: Vec<Value>,
        #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
        extra: serde_json::Map<String, Value>,
    },
    Assistant {
        content: Vec<ContentBlock>,
        #[serde(default)]
        api: String,
        #[serde(default)]
        provider: String,
        #[serde(default)]
        model: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        usage: Option<Usage>,
        /// `stop`, `length`, `toolUse`, `error`, `aborted`, or
        /// (harness-v2) `deferred`. Left open for forward compat.
        #[serde(rename = "stopReason", default)]
        stop_reason: String,
        #[serde(default)]
        timestamp: f64,
        #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
        extra: serde_json::Map<String, Value>,
    },
    ToolResult {
        #[serde(rename = "toolCallId", default)]
        tool_call_id: String,
        #[serde(rename = "toolName", default)]
        tool_name: String,
        content: Value,
        /// Nested LLM work performed by the tool; contributes to session
        /// totals when present.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        usage: Option<Usage>,
        #[serde(rename = "isError", default)]
        is_error: bool,
        #[serde(default)]
        timestamp: f64,
        #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
        extra: serde_json::Map<String, Value>,
    },
    /// Created by the RPC `bash` command, never by LLM tool calls. On the
    /// next prompt it is rewritten into a `user` message for the model.
    BashExecution {
        #[serde(default)]
        command: String,
        #[serde(default)]
        output: String,
        #[serde(rename = "exitCode", default, skip_serializing_if = "Option::is_none")]
        exit_code: Option<i64>,
        #[serde(default)]
        cancelled: bool,
        #[serde(default)]
        truncated: bool,
        #[serde(
            rename = "fullOutputPath",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        full_output_path: Option<String>,
        #[serde(default)]
        timestamp: f64,
        #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
        extra: serde_json::Map<String, Value>,
    },
}

/// One event from the `--mode json` / `--mode rpc` stdout stream,
/// discriminated on `type`. Known lifecycle events are typed with their
/// primary fields lifted; every variant keeps its full payload reachable
/// (unmodeled fields land in `extra`), and unknown `type`s fall back to
/// [`PiEvent::Unknown`] instead of erroring.
#[derive(Debug, Clone, PartialEq)]
pub enum PiEvent {
    /// `agent_start`
    AgentStart,
    /// `agent_end` — carries the turn's messages.
    AgentEnd { messages: Vec<PiMessage> },
    /// `turn_start`
    TurnStart,
    /// `turn_end`
    TurnEnd {
        message: Box<PiMessage>,
        tool_results: Vec<PiMessage>,
    },
    /// `message_start`
    MessageStart { message: Box<PiMessage> },
    /// `message_update` — streaming delta; in `--mode json` the
    /// cumulative message snapshot is omitted and only the incremental
    /// `assistantMessageEvent` (kept raw here) arrives.
    MessageUpdate {
        message: Option<Box<PiMessage>>,
        usage: Option<Usage>,
        assistant_message_event: Value,
    },
    /// `message_end`
    MessageEnd { message: Box<PiMessage> },
    /// `tool_execution_start`
    ToolExecutionStart {
        tool_call_id: String,
        tool_name: String,
        args: Value,
    },
    /// `tool_execution_update`
    ToolExecutionUpdate {
        tool_call_id: String,
        tool_name: String,
        args: Value,
        partial_result: Value,
    },
    /// `tool_execution_end`
    ToolExecutionEnd {
        tool_call_id: String,
        tool_name: String,
        result: Value,
        is_error: bool,
    },
    /// Any other `type` (session events like `queue_update`,
    /// `compaction_start`/`compaction_end`, `bash_execution_update`,
    /// extension UI requests, and whatever future CLIs add). The full
    /// object is preserved.
    Unknown { event_type: String, payload: Value },
}

impl PiEvent {
    /// The wire `type` string.
    pub fn event_type(&self) -> &str {
        match self {
            PiEvent::AgentStart => "agent_start",
            PiEvent::AgentEnd { .. } => "agent_end",
            PiEvent::TurnStart => "turn_start",
            PiEvent::TurnEnd { .. } => "turn_end",
            PiEvent::MessageStart { .. } => "message_start",
            PiEvent::MessageUpdate { .. } => "message_update",
            PiEvent::MessageEnd { .. } => "message_end",
            PiEvent::ToolExecutionStart { .. } => "tool_execution_start",
            PiEvent::ToolExecutionUpdate { .. } => "tool_execution_update",
            PiEvent::ToolExecutionEnd { .. } => "tool_execution_end",
            PiEvent::Unknown { event_type, .. } => event_type,
        }
    }

    /// Parse one stdout JSON object into a typed event.
    pub fn from_value(v: Value) -> Result<Self, serde_json::Error> {
        let t = v
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        Ok(match t.as_str() {
            "agent_start" => PiEvent::AgentStart,
            "agent_end" => PiEvent::AgentEnd {
                messages: serde_json::from_value(
                    v.get("messages").cloned().unwrap_or(Value::Array(vec![])),
                )?,
            },
            "turn_start" => PiEvent::TurnStart,
            "turn_end" => PiEvent::TurnEnd {
                message: serde_json::from_value(v.get("message").cloned().unwrap_or(Value::Null))?,
                tool_results: serde_json::from_value(
                    v.get("toolResults")
                        .cloned()
                        .unwrap_or(Value::Array(vec![])),
                )?,
            },
            "message_start" => PiEvent::MessageStart {
                message: serde_json::from_value(v.get("message").cloned().unwrap_or(Value::Null))?,
            },
            "message_update" => PiEvent::MessageUpdate {
                message: match v.get("message") {
                    Some(m) => Some(serde_json::from_value(m.clone())?),
                    None => None,
                },
                usage: match v.get("usage") {
                    Some(u) => Some(serde_json::from_value(u.clone())?),
                    None => None,
                },
                assistant_message_event: v
                    .get("assistantMessageEvent")
                    .cloned()
                    .unwrap_or(Value::Null),
            },
            "message_end" => PiEvent::MessageEnd {
                message: serde_json::from_value(v.get("message").cloned().unwrap_or(Value::Null))?,
            },
            "tool_execution_start" => PiEvent::ToolExecutionStart {
                tool_call_id: str_field(&v, "toolCallId"),
                tool_name: str_field(&v, "toolName"),
                args: v.get("args").cloned().unwrap_or(Value::Null),
            },
            "tool_execution_update" => PiEvent::ToolExecutionUpdate {
                tool_call_id: str_field(&v, "toolCallId"),
                tool_name: str_field(&v, "toolName"),
                args: v.get("args").cloned().unwrap_or(Value::Null),
                partial_result: v.get("partialResult").cloned().unwrap_or(Value::Null),
            },
            "tool_execution_end" => PiEvent::ToolExecutionEnd {
                tool_call_id: str_field(&v, "toolCallId"),
                tool_name: str_field(&v, "toolName"),
                result: v.get("result").cloned().unwrap_or(Value::Null),
                is_error: v
                    .get("isError")
                    .and_then(Value::as_bool)
                    .unwrap_or_default(),
            },
            _ => PiEvent::Unknown {
                event_type: t,
                payload: v,
            },
        })
    }
}

fn str_field(v: &Value, k: &str) -> String {
    v.get(k)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assistant_message_from_docs_parses() {
        let m: PiMessage = serde_json::from_value(serde_json::json!({
            "role": "assistant",
            "content": [
                {"type": "text", "text": "Hello! How can I help?"},
                {"type": "thinking", "thinking": "User is greeting me..."},
                {"type": "toolCall", "id": "call_123", "name": "bash", "arguments": {"command": "ls"}}
            ],
            "api": "anthropic-messages",
            "provider": "anthropic",
            "model": "claude-sonnet-4-20250514",
            "usage": {
                "input": 100, "output": 50, "cacheRead": 0, "cacheWrite": 0,
                "cost": {"input": 0.0003, "output": 0.00075, "cacheRead": 0, "cacheWrite": 0, "total": 0.00105}
            },
            "stopReason": "stop",
            "timestamp": 1733234567890i64
        }))
        .unwrap();
        let PiMessage::Assistant {
            content,
            stop_reason,
            ..
        } = m
        else {
            panic!("wrong role")
        };
        assert_eq!(content.len(), 3);
        assert_eq!(stop_reason, "stop");
        assert!(matches!(content[2], ContentBlock::ToolCall { .. }));
    }

    #[test]
    fn bash_execution_message_round_trips() {
        let v = serde_json::json!({
            "role": "bashExecution",
            "command": "ls -la",
            "output": "total 48",
            "exitCode": 0,
            "cancelled": false,
            "truncated": false,
            "timestamp": 1733234567890i64
        });
        let m: PiMessage = serde_json::from_value(v).unwrap();
        assert!(matches!(m, PiMessage::BashExecution { .. }));
    }

    #[test]
    fn unknown_event_type_is_preserved_not_error() {
        let e = PiEvent::from_value(serde_json::json!({
            "type": "queue_update", "steering": [], "followUp": []
        }))
        .unwrap();
        let PiEvent::Unknown {
            event_type,
            payload,
        } = e
        else {
            panic!("expected Unknown")
        };
        assert_eq!(event_type, "queue_update");
        assert!(payload.get("steering").is_some());
    }

    #[test]
    fn tool_execution_events_lift_fields() {
        let e = PiEvent::from_value(serde_json::json!({
            "type": "tool_execution_end",
            "toolCallId": "call_9", "toolName": "bash",
            "result": {"ok": true}, "isError": false
        }))
        .unwrap();
        let PiEvent::ToolExecutionEnd {
            tool_call_id,
            tool_name,
            is_error,
            ..
        } = e
        else {
            panic!("wrong variant")
        };
        assert_eq!(
            (tool_call_id.as_str(), tool_name.as_str()),
            ("call_9", "bash")
        );
        assert!(!is_error);
    }
}
