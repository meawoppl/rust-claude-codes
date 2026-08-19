use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

use super::content_blocks::{ContentBlock, ToolUseBlock};
use super::control::{ControlRequest, ControlResponse};
use super::errors::{AnthropicError, ParseError};
use super::message_types::{AssistantMessage, SystemMessage, UserMessage};
use super::rate_limit::RateLimitEvent;
use super::result::ResultMessage;

/// Top-level enum for all possible Claude output messages
#[derive(Debug, Clone, Serialize)]
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

    /// Claude Code internal workflow-journal result event.
    #[serde(rename = "result")]
    TranscriptResult(TranscriptMessage),

    /// Control request from CLI (tool permissions, hooks, etc.)
    ControlRequest(ControlRequest),

    /// Control response from CLI (ack for initialization, etc.)
    ControlResponse(ControlResponse),

    /// API error from Anthropic (500, 529 overloaded, etc.)
    Error(AnthropicError),

    /// Rate limit status event
    RateLimitEvent(RateLimitEvent),

    /// Raw API stream event emitted with `--include-partial-messages`.
    StreamEvent(StreamEventMessage),

    /// Progress update for a running tool.
    ToolProgress(ToolProgressMessage),

    /// Fate of a queued command (slash command or queued user prompt).
    CommandLifecycle(CommandLifecycleMessage),

    /// Authentication status update.
    AuthStatus(AuthStatusMessage),

    /// Summary of preceding tool uses.
    ToolUseSummary(ToolUseSummaryMessage),

    /// Predicted next user prompt.
    PromptSuggestion(PromptSuggestionMessage),

    /// Conversation reset notification.
    ConversationReset(ConversationResetMessage),

    /// Claude Code internal transcript progress event.
    Progress(TranscriptMessage),

    /// Claude Code internal queue event.
    #[serde(rename = "queue-operation")]
    QueueOperation(TranscriptMessage),

    /// Claude Code internal PR link metadata event.
    #[serde(rename = "pr-link")]
    PrLink(TranscriptMessage),

    /// Claude Code internal file history snapshot event.
    #[serde(rename = "file-history-snapshot")]
    FileHistorySnapshot(TranscriptMessage),

    /// Claude Code internal session summary event.
    Summary(TranscriptMessage),

    /// Claude Code internal mode metadata event.
    Mode(TranscriptMessage),

    /// Claude Code internal permission mode metadata event.
    #[serde(rename = "permission-mode")]
    PermissionMode(TranscriptMessage),

    /// Claude Code internal attachment metadata event.
    Attachment(TranscriptMessage),

    /// Claude Code internal AI-generated title event.
    #[serde(rename = "ai-title")]
    AiTitle(TranscriptMessage),

    /// Claude Code internal last prompt pointer event.
    #[serde(rename = "last-prompt")]
    LastPrompt(TranscriptMessage),

    /// Claude Code internal session started event.
    Started(TranscriptMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEventMessage {
    pub event: Value,
    pub parent_tool_use_id: Option<String>,
    pub uuid: String,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttft_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProgressMessage {
    pub tool_use_id: String,
    pub tool_name: String,
    pub parent_tool_use_id: Option<String>,
    pub elapsed_time_seconds: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    pub uuid: String,
    pub session_id: String,
    /// True when this event was emitted only to keep the stream alive, not
    /// because the tool reported progress.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heartbeat: Option<bool>,
    /// Subagent type for progress from a `Task` tool's subagent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subagent_type: Option<String>,
    /// Present while a subagent API call is being retried after an error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subagent_retry: Option<SubagentRetry>,
}

/// Retry state carried on a [`ToolProgressMessage`] while a subagent API
/// call is retried after an error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentRetry {
    pub agent_id: String,
    pub attempt: u64,
    pub max_retries: u64,
    pub retry_delay_ms: u64,
    pub error_status: Option<u16>,
    pub error_category: String,
}

/// `command_lifecycle` message — the fate of a queued command (slash command
/// or queued user prompt): `queued` when the inbound message enters the
/// command queue, `started` when it drains into a turn, then exactly one
/// terminal state (`completed`, `cancelled`, or `discarded`). Commands
/// enqueued without a client-supplied uuid emit no lifecycle events. Not a
/// strict pairing — a terminal state may arrive for a `command_uuid` that
/// never emitted `started`, and internally-enqueued commands skip `queued`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandLifecycleMessage {
    /// The queued command's uuid — the client-supplied uuid on the inbound
    /// message (distinct from the universal per-frame `uuid`).
    pub command_uuid: String,
    pub state: CommandLifecycleState,
    pub uuid: String,
    pub session_id: String,
}

/// Lifecycle state carried by a [`CommandLifecycleMessage`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CommandLifecycleState {
    /// The inbound message entered the command queue.
    Queued,
    /// The command drained into a turn.
    Started,
    /// The turn that consumed the command ended cleanly.
    Completed,
    /// Removed by cancel, caught before dispatch, or consumed into a turn
    /// that was aborted or died on a hard failure.
    Cancelled,
    /// The session ended with the command still queued.
    Discarded,
    /// A state not yet known to this version of the crate.
    Unknown(String),
}

impl CommandLifecycleState {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Queued => "queued",
            Self::Started => "started",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Discarded => "discarded",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl std::fmt::Display for CommandLifecycleState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for CommandLifecycleState {
    fn from(s: &str) -> Self {
        match s {
            "queued" => Self::Queued,
            "started" => Self::Started,
            "completed" => Self::Completed,
            "cancelled" => Self::Cancelled,
            "discarded" => Self::Discarded,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for CommandLifecycleState {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for CommandLifecycleState {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatusMessage {
    #[serde(rename = "isAuthenticating")]
    pub is_authenticating: bool,
    pub output: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub uuid: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseSummaryMessage {
    pub summary: String,
    pub preceding_tool_use_ids: Vec<String>,
    pub uuid: String,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSuggestionMessage {
    pub suggestion: String,
    pub uuid: String,
    pub session_id: String,
}

/// Emitted when the conversation is reset mid-stream (e.g. `/clear` sent as
/// user input). **Identity semantics, measured live against CLI 2.1.232 —
/// two traps here:**
///
/// 1. **The session id rotates.** After this frame the same process re-inits
///    and every subsequent frame carries a NEW `session_id`; both the old
///    and new ids get real transcript files on disk. A consumer keying a
///    live stream's transcript by its first-seen session id will
///    mis-attribute everything after a reset — treat this frame as a
///    session-identity boundary and adopt the next `system` init's id.
/// 2. **`new_conversation_id` is not the successor session id.** In live
///    measurement it matched nothing: not the post-reset `session_id`, no
///    transcript file, never referenced by any later frame. Do not key on
///    it.
///
/// Pinned by `conversation_reset_rotates_the_session_id` in the live
/// integration tests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationResetMessage {
    /// A fresh id announced for the new conversation. **Not** observed to
    /// match the post-reset `session_id` or anything else on the wire or
    /// disk — see the type-level docs before using.
    pub new_conversation_id: String,
    pub uuid: String,
    /// The OLD session id — the identity being retired by this reset.
    pub session_id: String,
}

/// Raw preserved record for Claude Code transcript-only message types.
///
/// These events are emitted in `~/.claude/projects/**/*.jsonl`, not in the
/// public `--output-format stream-json` protocol. Keeping them typed at the
/// top level lets corpus tests parse real transcript files without losing the
/// original payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptMessage {
    #[serde(flatten)]
    pub data: Map<String, Value>,
}

impl ClaudeOutput {
    /// Get the message type as a string
    pub fn message_type(&self) -> String {
        match self {
            ClaudeOutput::System(_) => "system".to_string(),
            ClaudeOutput::User(_) => "user".to_string(),
            ClaudeOutput::Assistant(_) => "assistant".to_string(),
            ClaudeOutput::Result(_) => "result".to_string(),
            ClaudeOutput::TranscriptResult(_) => "result".to_string(),
            ClaudeOutput::ControlRequest(_) => "control_request".to_string(),
            ClaudeOutput::ControlResponse(_) => "control_response".to_string(),
            ClaudeOutput::Error(_) => "error".to_string(),
            ClaudeOutput::RateLimitEvent(_) => "rate_limit_event".to_string(),
            ClaudeOutput::StreamEvent(_) => "stream_event".to_string(),
            ClaudeOutput::ToolProgress(_) => "tool_progress".to_string(),
            ClaudeOutput::CommandLifecycle(_) => "command_lifecycle".to_string(),
            ClaudeOutput::AuthStatus(_) => "auth_status".to_string(),
            ClaudeOutput::ToolUseSummary(_) => "tool_use_summary".to_string(),
            ClaudeOutput::PromptSuggestion(_) => "prompt_suggestion".to_string(),
            ClaudeOutput::ConversationReset(_) => "conversation_reset".to_string(),
            ClaudeOutput::Progress(_) => "progress".to_string(),
            ClaudeOutput::QueueOperation(_) => "queue-operation".to_string(),
            ClaudeOutput::PrLink(_) => "pr-link".to_string(),
            ClaudeOutput::FileHistorySnapshot(_) => "file-history-snapshot".to_string(),
            ClaudeOutput::Summary(_) => "summary".to_string(),
            ClaudeOutput::Mode(_) => "mode".to_string(),
            ClaudeOutput::PermissionMode(_) => "permission-mode".to_string(),
            ClaudeOutput::Attachment(_) => "attachment".to_string(),
            ClaudeOutput::AiTitle(_) => "ai-title".to_string(),
            ClaudeOutput::LastPrompt(_) => "last-prompt".to_string(),
            ClaudeOutput::Started(_) => "started".to_string(),
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

    /// Check if this is an Anthropic API error
    pub fn is_api_error(&self) -> bool {
        matches!(self, ClaudeOutput::Error(_))
    }

    /// Get the control request if this is one
    pub fn as_control_request(&self) -> Option<&ControlRequest> {
        match self {
            ClaudeOutput::ControlRequest(req) => Some(req),
            _ => None,
        }
    }

    /// Get the Anthropic error if this is one
    ///
    /// # Example
    /// ```
    /// use claude_codes::ClaudeOutput;
    ///
    /// let json = r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#;
    /// let output: ClaudeOutput = serde_json::from_str(json).unwrap();
    ///
    /// if let Some(err) = output.as_anthropic_error() {
    ///     if err.is_overloaded() {
    ///         println!("API is overloaded, retrying...");
    ///     }
    /// }
    /// ```
    pub fn as_anthropic_error(&self) -> Option<&AnthropicError> {
        match self {
            ClaudeOutput::Error(err) => Some(err),
            _ => None,
        }
    }

    /// Check if this is a rate limit event
    pub fn is_rate_limit_event(&self) -> bool {
        matches!(self, ClaudeOutput::RateLimitEvent(_))
    }

    /// Get the rate limit event if this is one
    pub fn as_rate_limit_event(&self) -> Option<&RateLimitEvent> {
        match self {
            ClaudeOutput::RateLimitEvent(evt) => Some(evt),
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

    /// Check if this is a system init message
    ///
    /// # Example
    /// ```
    /// use claude_codes::ClaudeOutput;
    ///
    /// let json = r#"{"type":"system","subtype":"init","session_id":"abc"}"#;
    /// let output: ClaudeOutput = serde_json::from_str(json).unwrap();
    /// assert!(output.is_system_init());
    /// ```
    pub fn is_system_init(&self) -> bool {
        matches!(self, ClaudeOutput::System(sys) if sys.is_init())
    }

    /// Get the session ID from any message type that has one.
    ///
    /// Returns the session ID from System, Assistant, or Result messages.
    /// Returns `None` for User, ControlRequest, and ControlResponse messages.
    ///
    /// # Example
    /// ```
    /// use claude_codes::ClaudeOutput;
    ///
    /// let json = r#"{"type":"result","subtype":"success","is_error":false,
    ///     "duration_ms":100,"duration_api_ms":200,"num_turns":1,
    ///     "session_id":"my-session","total_cost_usd":0.01}"#;
    /// let output: ClaudeOutput = serde_json::from_str(json).unwrap();
    /// assert_eq!(output.session_id(), Some("my-session"));
    /// ```
    pub fn session_id(&self) -> Option<&str> {
        match self {
            ClaudeOutput::System(sys) => sys
                .data
                .get("session_id")
                .or_else(|| sys.data.get("sessionId"))
                .and_then(|v| v.as_str()),
            ClaudeOutput::Assistant(ass) => Some(&ass.session_id),
            ClaudeOutput::Result(res) => Some(&res.session_id),
            ClaudeOutput::TranscriptResult(msg) => msg
                .data
                .get("session_id")
                .or_else(|| msg.data.get("sessionId"))
                .and_then(|v| v.as_str()),
            ClaudeOutput::User(_) => None,
            ClaudeOutput::ControlRequest(_) => None,
            ClaudeOutput::ControlResponse(_) => None,
            ClaudeOutput::Error(_) => None,
            ClaudeOutput::RateLimitEvent(evt) => Some(&evt.session_id),
            ClaudeOutput::StreamEvent(msg) => Some(&msg.session_id),
            ClaudeOutput::ToolProgress(msg) => Some(&msg.session_id),
            ClaudeOutput::CommandLifecycle(msg) => Some(&msg.session_id),
            ClaudeOutput::AuthStatus(msg) => Some(&msg.session_id),
            ClaudeOutput::ToolUseSummary(msg) => Some(&msg.session_id),
            ClaudeOutput::PromptSuggestion(msg) => Some(&msg.session_id),
            ClaudeOutput::ConversationReset(msg) => Some(&msg.session_id),
            ClaudeOutput::Progress(msg)
            | ClaudeOutput::QueueOperation(msg)
            | ClaudeOutput::PrLink(msg)
            | ClaudeOutput::FileHistorySnapshot(msg)
            | ClaudeOutput::Summary(msg)
            | ClaudeOutput::Mode(msg)
            | ClaudeOutput::PermissionMode(msg)
            | ClaudeOutput::Attachment(msg)
            | ClaudeOutput::AiTitle(msg)
            | ClaudeOutput::LastPrompt(msg)
            | ClaudeOutput::Started(msg) => msg
                .data
                .get("session_id")
                .or_else(|| msg.data.get("sessionId"))
                .and_then(|v| v.as_str()),
        }
    }

    /// Get a specific tool use by name from an assistant message.
    ///
    /// Returns the first `ToolUseBlock` with the given name, or `None` if this
    /// is not an assistant message or doesn't contain the specified tool.
    ///
    /// # Example
    /// ```
    /// use claude_codes::ClaudeOutput;
    ///
    /// let json = r#"{"type":"assistant","message":{"id":"msg_1","role":"assistant",
    ///     "model":"claude-3","content":[{"type":"tool_use","id":"tu_1",
    ///     "name":"Bash","input":{"command":"ls"}}]},"session_id":"abc"}"#;
    /// let output: ClaudeOutput = serde_json::from_str(json).unwrap();
    ///
    /// if let Some(bash) = output.as_tool_use("Bash") {
    ///     assert_eq!(bash.name, "Bash");
    /// }
    /// ```
    pub fn as_tool_use(&self, tool_name: &str) -> Option<&ToolUseBlock> {
        match self {
            ClaudeOutput::Assistant(ass) => {
                ass.message.content.iter().find_map(|block| match block {
                    ContentBlock::ToolUse(tu) if tu.name == tool_name => Some(tu),
                    _ => None,
                })
            }
            _ => None,
        }
    }

    /// Get all tool uses from an assistant message.
    ///
    /// Returns an iterator over all `ToolUseBlock`s in the message, or an empty
    /// iterator if this is not an assistant message.
    ///
    /// # Example
    /// ```
    /// use claude_codes::ClaudeOutput;
    ///
    /// let json = r#"{"type":"assistant","message":{"id":"msg_1","role":"assistant",
    ///     "model":"claude-3","content":[
    ///         {"type":"tool_use","id":"tu_1","name":"Read","input":{"file_path":"/tmp/a"}},
    ///         {"type":"tool_use","id":"tu_2","name":"Write","input":{"file_path":"/tmp/b","content":"x"}}
    ///     ]},"session_id":"abc"}"#;
    /// let output: ClaudeOutput = serde_json::from_str(json).unwrap();
    ///
    /// let tools: Vec<_> = output.tool_uses().collect();
    /// assert_eq!(tools.len(), 2);
    /// ```
    pub fn tool_uses(&self) -> impl Iterator<Item = &ToolUseBlock> {
        let content = match self {
            ClaudeOutput::Assistant(ass) => Some(&ass.message.content),
            _ => None,
        };

        content
            .into_iter()
            .flat_map(|c| c.iter())
            .filter_map(|block| match block {
                ContentBlock::ToolUse(tu) => Some(tu),
                _ => None,
            })
    }

    /// Get text content from an assistant message.
    ///
    /// Returns the concatenated text from all text blocks in the message,
    /// or `None` if this is not an assistant message or has no text content.
    ///
    /// # Example
    /// ```
    /// use claude_codes::ClaudeOutput;
    ///
    /// let json = r#"{"type":"assistant","message":{"id":"msg_1","role":"assistant",
    ///     "model":"claude-3","content":[{"type":"text","text":"Hello, world!"}]},
    ///     "session_id":"abc"}"#;
    /// let output: ClaudeOutput = serde_json::from_str(json).unwrap();
    /// assert_eq!(output.text_content(), Some("Hello, world!".to_string()));
    /// ```
    pub fn text_content(&self) -> Option<String> {
        match self {
            ClaudeOutput::Assistant(ass) => {
                let texts: Vec<&str> = ass
                    .message
                    .content
                    .iter()
                    .filter_map(|block| match block {
                        ContentBlock::Text(t) => Some(t.text.as_str()),
                        _ => None,
                    })
                    .collect();

                if texts.is_empty() {
                    None
                } else {
                    Some(texts.join(""))
                }
            }
            _ => None,
        }
    }

    /// Get the assistant message if this is one.
    ///
    /// # Example
    /// ```
    /// use claude_codes::ClaudeOutput;
    ///
    /// let json = r#"{"type":"assistant","message":{"id":"msg_1","role":"assistant",
    ///     "model":"claude-3","content":[]},"session_id":"abc"}"#;
    /// let output: ClaudeOutput = serde_json::from_str(json).unwrap();
    ///
    /// if let Some(assistant) = output.as_assistant() {
    ///     assert_eq!(assistant.message.model, "claude-3");
    /// }
    /// ```
    pub fn as_assistant(&self) -> Option<&AssistantMessage> {
        match self {
            ClaudeOutput::Assistant(ass) => Some(ass),
            _ => None,
        }
    }

    /// Get the result message if this is one.
    ///
    /// # Example
    /// ```
    /// use claude_codes::ClaudeOutput;
    ///
    /// let json = r#"{"type":"result","subtype":"success","is_error":false,
    ///     "duration_ms":100,"duration_api_ms":200,"num_turns":1,
    ///     "session_id":"abc","total_cost_usd":0.01}"#;
    /// let output: ClaudeOutput = serde_json::from_str(json).unwrap();
    ///
    /// if let Some(result) = output.as_result() {
    ///     assert!(!result.is_error);
    /// }
    /// ```
    pub fn as_result(&self) -> Option<&ResultMessage> {
        match self {
            ClaudeOutput::Result(res) => Some(res),
            _ => None,
        }
    }

    /// Get the system message if this is one.
    pub fn as_system(&self) -> Option<&SystemMessage> {
        match self {
            ClaudeOutput::System(sys) => Some(sys),
            _ => None,
        }
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
            raw_line: s.to_string(),
            raw_json: None,
            error_message: format!("Invalid JSON: {}", e),
        })?;

        // Then try to parse that Value as ClaudeOutput
        serde_json::from_value::<ClaudeOutput>(value.clone()).map_err(|e| ParseError {
            raw_line: s.to_string(),
            raw_json: Some(value),
            error_message: e.to_string(),
        })
    }
}

impl<'de> Deserialize<'de> for ClaudeOutput {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        let message_type = value
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| serde::de::Error::missing_field("type"))?;
        let mut payload = value.clone();
        if let Some(obj) = payload.as_object_mut() {
            obj.remove("type");
        }

        fn parse<T, E>(value: Value) -> Result<T, E>
        where
            T: serde::de::DeserializeOwned,
            E: serde::de::Error,
        {
            serde_json::from_value(value).map_err(E::custom)
        }

        match message_type {
            "system" => parse(payload).map(Self::System),
            "user" => parse(payload).map(Self::User),
            "assistant" => parse(payload).map(Self::Assistant),
            "result" if value.get("subtype").is_some() => parse(payload).map(Self::Result),
            "result" => parse(payload).map(Self::TranscriptResult),
            "control_request" => parse(payload).map(Self::ControlRequest),
            "control_response" => parse(payload).map(Self::ControlResponse),
            "error" => parse(payload).map(Self::Error),
            "rate_limit_event" => parse(payload).map(Self::RateLimitEvent),
            "stream_event" => parse(payload).map(Self::StreamEvent),
            "tool_progress" => parse(payload).map(Self::ToolProgress),
            "command_lifecycle" => parse(payload).map(Self::CommandLifecycle),
            "auth_status" => parse(payload).map(Self::AuthStatus),
            "tool_use_summary" => parse(payload).map(Self::ToolUseSummary),
            "prompt_suggestion" => parse(payload).map(Self::PromptSuggestion),
            "conversation_reset" => parse(payload).map(Self::ConversationReset),
            "progress" => parse(payload).map(Self::Progress),
            "queue-operation" => parse(payload).map(Self::QueueOperation),
            "pr-link" => parse(payload).map(Self::PrLink),
            "file-history-snapshot" => parse(payload).map(Self::FileHistorySnapshot),
            "summary" => parse(payload).map(Self::Summary),
            "mode" => parse(payload).map(Self::Mode),
            "permission-mode" => parse(payload).map(Self::PermissionMode),
            "attachment" => parse(payload).map(Self::Attachment),
            "ai-title" => parse(payload).map(Self::AiTitle),
            "last-prompt" => parse(payload).map(Self::LastPrompt),
            "started" => parse(payload).map(Self::Started),
            other => Err(serde::de::Error::unknown_variant(
                other,
                &[
                    "system",
                    "user",
                    "assistant",
                    "result",
                    "control_request",
                    "control_response",
                    "error",
                    "rate_limit_event",
                    "stream_event",
                    "tool_progress",
                    "command_lifecycle",
                    "auth_status",
                    "tool_use_summary",
                    "prompt_suggestion",
                    "conversation_reset",
                    "progress",
                    "queue-operation",
                    "pr-link",
                    "file-history-snapshot",
                    "summary",
                    "mode",
                    "permission-mode",
                    "attachment",
                    "ai-title",
                    "last-prompt",
                    "started",
                ],
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_deserialize_new_top_level_message_types() {
        let cases = [
            (
                r#"{"type":"stream_event","event":{"type":"content_block_delta"},"parent_tool_use_id":null,"uuid":"u1","session_id":"s1","ttft_ms":12}"#,
                "stream_event",
            ),
            (
                r#"{"type":"tool_progress","tool_use_id":"toolu_1","tool_name":"Bash","parent_tool_use_id":null,"elapsed_time_seconds":1.25,"task_id":"task-1","uuid":"u2","session_id":"s2"}"#,
                "tool_progress",
            ),
            (
                r#"{"type":"auth_status","isAuthenticating":true,"output":["login"],"uuid":"u3","session_id":"s3"}"#,
                "auth_status",
            ),
            (
                r#"{"type":"tool_use_summary","summary":"read files","preceding_tool_use_ids":["toolu_1"],"uuid":"u4","session_id":"s4","timestamp":"2026-07-09T17:46:33Z"}"#,
                "tool_use_summary",
            ),
            (
                r#"{"type":"prompt_suggestion","suggestion":"Run tests","uuid":"u5","session_id":"s5"}"#,
                "prompt_suggestion",
            ),
            (
                r#"{"type":"conversation_reset","new_conversation_id":"new-session","uuid":"u6","session_id":"s6"}"#,
                "conversation_reset",
            ),
            (
                r#"{"type":"command_lifecycle","command_uuid":"cmd-1","state":"queued","uuid":"u7","session_id":"s7"}"#,
                "command_lifecycle",
            ),
        ];

        for (json, message_type) in cases {
            let output: ClaudeOutput = serde_json::from_str(json).unwrap();
            assert_eq!(output.message_type(), message_type);
            assert!(output.session_id().is_some());
        }
    }

    #[test]
    fn test_command_lifecycle_states_roundtrip() {
        for state in ["queued", "started", "completed", "cancelled", "discarded"] {
            let json = format!(
                r#"{{"type":"command_lifecycle","command_uuid":"cmd-1","state":"{}","uuid":"u1","session_id":"s1"}}"#,
                state
            );
            let output: ClaudeOutput = serde_json::from_str(&json).unwrap();
            let ClaudeOutput::CommandLifecycle(msg) = &output else {
                panic!("expected CommandLifecycle");
            };
            assert_eq!(msg.state.as_str(), state);
            assert!(!matches!(msg.state, CommandLifecycleState::Unknown(_)));
            assert_eq!(serde_json::to_string(&output).unwrap(), json);
        }

        // Unknown states survive decode and round-trip verbatim.
        let json = r#"{"type":"command_lifecycle","command_uuid":"cmd-2","state":"parked","uuid":"u2","session_id":"s2"}"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let ClaudeOutput::CommandLifecycle(msg) = &output else {
            panic!("expected CommandLifecycle");
        };
        assert_eq!(
            msg.state,
            CommandLifecycleState::Unknown("parked".to_string())
        );
        assert_eq!(serde_json::to_string(&output).unwrap(), json);
    }

    #[test]
    fn test_tool_progress_heartbeat_and_subagent_retry() {
        let json = r#"{"type":"tool_progress","tool_use_id":"toolu_1","tool_name":"Task","parent_tool_use_id":null,"elapsed_time_seconds":30,"uuid":"u1","session_id":"s1","heartbeat":true,"subagent_type":"Explore","subagent_retry":{"agent_id":"agent-1","attempt":2,"max_retries":5,"retry_delay_ms":4000,"error_status":529,"error_category":"overloaded"}}"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let ClaudeOutput::ToolProgress(msg) = &output else {
            panic!("expected ToolProgress");
        };
        assert_eq!(msg.heartbeat, Some(true));
        assert_eq!(msg.subagent_type.as_deref(), Some("Explore"));
        let retry = msg.subagent_retry.as_ref().unwrap();
        assert_eq!(retry.attempt, 2);
        assert_eq!(retry.error_status, Some(529));
        assert_eq!(retry.error_category, "overloaded");

        // Null error_status parses too.
        let json = r#"{"type":"tool_progress","tool_use_id":"toolu_2","tool_name":"Task","parent_tool_use_id":null,"elapsed_time_seconds":1,"uuid":"u2","session_id":"s2","subagent_retry":{"agent_id":"agent-2","attempt":1,"max_retries":3,"retry_delay_ms":500,"error_status":null,"error_category":"network"}}"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let ClaudeOutput::ToolProgress(msg) = &output else {
            panic!("expected ToolProgress");
        };
        assert_eq!(msg.subagent_retry.as_ref().unwrap().error_status, None);
    }

    #[test]
    fn test_deserialize_transcript_only_message_types() {
        let cases = [
            (
                r#"{"type":"progress","sessionId":"s1","content":"delta"}"#,
                "progress",
                Some("s1"),
            ),
            (
                r#"{"type":"queue-operation","sessionId":"s2","operation":"push"}"#,
                "queue-operation",
                Some("s2"),
            ),
            (
                r#"{"type":"pr-link","sessionId":"s3","url":"https://example.invalid/pr"}"#,
                "pr-link",
                Some("s3"),
            ),
            (
                r#"{"type":"file-history-snapshot","sessionId":"s4","files":[]}"#,
                "file-history-snapshot",
                Some("s4"),
            ),
            (r#"{"type":"summary","summary":"Done"}"#, "summary", None),
            (
                r#"{"type":"mode","mode":"normal","sessionId":"s5"}"#,
                "mode",
                Some("s5"),
            ),
            (
                r#"{"type":"permission-mode","permissionMode":"default","sessionId":"s6"}"#,
                "permission-mode",
                Some("s6"),
            ),
            (
                r#"{"type":"attachment","attachment":{"type":"task_reminder"},"sessionId":"s7"}"#,
                "attachment",
                Some("s7"),
            ),
            (
                r#"{"type":"ai-title","aiTitle":"Title","sessionId":"s8"}"#,
                "ai-title",
                Some("s8"),
            ),
            (
                r#"{"type":"last-prompt","lastPrompt":"prompt","sessionId":"s9"}"#,
                "last-prompt",
                Some("s9"),
            ),
            (
                r#"{"type":"started","sessionId":"s10"}"#,
                "started",
                Some("s10"),
            ),
            (
                r#"{"type":"result","key":"k","agentId":"a","result":{"ok":true}}"#,
                "result",
                None,
            ),
        ];

        for (json, message_type, session_id) in cases {
            let output: ClaudeOutput = serde_json::from_str(json).unwrap();
            assert_eq!(output.message_type(), message_type);
            assert_eq!(output.session_id(), session_id);
        }
    }

    #[test]
    fn test_transcript_assistant_accepts_camel_case_session_id() {
        let json = r#"{
            "type": "assistant",
            "sessionId": "camel-session",
            "message": {
                "id": "msg_123",
                "role": "assistant",
                "model": "claude-3-sonnet",
                "content": [{"type": "text", "text": "Hello"}]
            }
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.session_id(), Some("camel-session"));
    }

    #[test]
    fn test_is_system_init() {
        let init_json = r#"{
            "type": "system",
            "subtype": "init",
            "session_id": "test-session"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(init_json).unwrap();
        assert!(output.is_system_init());

        let status_json = r#"{
            "type": "system",
            "subtype": "status",
            "session_id": "test-session"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(status_json).unwrap();
        assert!(!output.is_system_init());
    }

    #[test]
    fn test_session_id() {
        // Result message
        let result_json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 1,
            "session_id": "result-session",
            "total_cost_usd": 0.01
        }"#;
        let output: ClaudeOutput = serde_json::from_str(result_json).unwrap();
        assert_eq!(output.session_id(), Some("result-session"));

        // Assistant message
        let assistant_json = r#"{
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "role": "assistant",
                "model": "claude-3",
                "content": []
            },
            "session_id": "assistant-session"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(assistant_json).unwrap();
        assert_eq!(output.session_id(), Some("assistant-session"));

        // System message
        let system_json = r#"{
            "type": "system",
            "subtype": "init",
            "session_id": "system-session"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(system_json).unwrap();
        assert_eq!(output.session_id(), Some("system-session"));
    }

    #[test]
    fn test_as_tool_use() {
        let json = r#"{
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "role": "assistant",
                "model": "claude-3",
                "content": [
                    {"type": "text", "text": "Let me run that command."},
                    {"type": "tool_use", "id": "tu_1", "name": "Bash", "input": {"command": "ls -la"}},
                    {"type": "tool_use", "id": "tu_2", "name": "Read", "input": {"file_path": "/tmp/test"}}
                ]
            },
            "session_id": "abc"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();

        // Find Bash tool
        let bash = output.as_tool_use("Bash");
        assert!(bash.is_some());
        assert_eq!(bash.unwrap().id, "tu_1");

        // Find Read tool
        let read = output.as_tool_use("Read");
        assert!(read.is_some());
        assert_eq!(read.unwrap().id, "tu_2");

        // Non-existent tool
        assert!(output.as_tool_use("Write").is_none());

        // Not an assistant message
        let result_json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 1,
            "session_id": "abc",
            "total_cost_usd": 0.01
        }"#;
        let result: ClaudeOutput = serde_json::from_str(result_json).unwrap();
        assert!(result.as_tool_use("Bash").is_none());
    }

    #[test]
    fn test_tool_uses() {
        let json = r#"{
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "role": "assistant",
                "model": "claude-3",
                "content": [
                    {"type": "text", "text": "Running commands..."},
                    {"type": "tool_use", "id": "tu_1", "name": "Bash", "input": {"command": "ls"}},
                    {"type": "tool_use", "id": "tu_2", "name": "Read", "input": {"file_path": "/tmp/a"}},
                    {"type": "tool_use", "id": "tu_3", "name": "Write", "input": {"file_path": "/tmp/b", "content": "x"}}
                ]
            },
            "session_id": "abc"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();

        let tools: Vec<_> = output.tool_uses().collect();
        assert_eq!(tools.len(), 3);
        assert_eq!(tools[0].name, "Bash");
        assert_eq!(tools[1].name, "Read");
        assert_eq!(tools[2].name, "Write");
    }

    #[test]
    fn test_text_content() {
        // Single text block
        let json = r#"{
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "role": "assistant",
                "model": "claude-3",
                "content": [{"type": "text", "text": "Hello, world!"}]
            },
            "session_id": "abc"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.text_content(), Some("Hello, world!".to_string()));

        // Multiple text blocks
        let json = r#"{
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "role": "assistant",
                "model": "claude-3",
                "content": [
                    {"type": "text", "text": "Hello, "},
                    {"type": "tool_use", "id": "tu_1", "name": "Bash", "input": {}},
                    {"type": "text", "text": "world!"}
                ]
            },
            "session_id": "abc"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.text_content(), Some("Hello, world!".to_string()));

        // No text blocks
        let json = r#"{
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "role": "assistant",
                "model": "claude-3",
                "content": [{"type": "tool_use", "id": "tu_1", "name": "Bash", "input": {}}]
            },
            "session_id": "abc"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.text_content(), None);

        // Not an assistant message
        let json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 1,
            "session_id": "abc",
            "total_cost_usd": 0.01
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.text_content(), None);
    }

    #[test]
    fn test_as_assistant() {
        let json = r#"{
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "role": "assistant",
                "model": "claude-sonnet-4",
                "content": []
            },
            "session_id": "abc"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();

        let assistant = output.as_assistant();
        assert!(assistant.is_some());
        assert_eq!(assistant.unwrap().message.model, "claude-sonnet-4");

        // Not an assistant
        let result_json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 1,
            "session_id": "abc",
            "total_cost_usd": 0.01
        }"#;
        let result: ClaudeOutput = serde_json::from_str(result_json).unwrap();
        assert!(result.as_assistant().is_none());
    }

    #[test]
    fn test_as_result() {
        let json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 5,
            "session_id": "abc",
            "total_cost_usd": 0.05
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();

        let result = output.as_result();
        assert!(result.is_some());
        assert_eq!(result.unwrap().num_turns, 5);
        assert_eq!(result.unwrap().total_cost_usd, 0.05);

        // Not a result
        let assistant_json = r#"{
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "role": "assistant",
                "model": "claude-3",
                "content": []
            },
            "session_id": "abc"
        }"#;
        let assistant: ClaudeOutput = serde_json::from_str(assistant_json).unwrap();
        assert!(assistant.as_result().is_none());
    }

    #[test]
    fn test_as_system() {
        let json = r#"{
            "type": "system",
            "subtype": "init",
            "session_id": "abc",
            "model": "claude-3"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();

        let system = output.as_system();
        assert!(system.is_some());
        assert!(system.unwrap().is_init());

        // Not a system message
        let result_json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 1,
            "session_id": "abc",
            "total_cost_usd": 0.01
        }"#;
        let result: ClaudeOutput = serde_json::from_str(result_json).unwrap();
        assert!(result.as_system().is_none());
    }
}
