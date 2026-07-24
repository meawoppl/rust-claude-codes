use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::fmt;
use uuid::Uuid;

use super::claude_output::ClaudeOutput;
use super::content_blocks::{deserialize_content_blocks, ContentBlock};

/// Known system message subtypes.
///
/// The Claude CLI emits system messages with a `subtype` field indicating what
/// kind of system event occurred. This enum captures the known subtypes while
/// preserving unknown values via the `Unknown` variant for forward compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SystemSubtype {
    Init,
    Status,
    CompactBoundary,
    ThinkingTokens,
    TaskStarted,
    TaskProgress,
    TaskUpdated,
    TaskNotification,
    ApiRetry,
    ControlRequestProgress,
    ModelRefusalFallback,
    ModelRefusalNoFallback,
    LocalCommandOutput,
    HookStarted,
    HookProgress,
    HookResponse,
    PluginInstall,
    BackgroundTasksChanged,
    SessionStateChanged,
    WorkerShuttingDown,
    CommandsChanged,
    Notification,
    FilesPersisted,
    MemoryRecall,
    ElicitationComplete,
    PermissionDenied,
    MirrorError,
    Informational,
    CodeChangePublished,
    VcsStateChanged,
    /// A subtype not yet known to this version of the crate.
    Unknown(String),
}

impl SystemSubtype {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Init => "init",
            Self::Status => "status",
            Self::CompactBoundary => "compact_boundary",
            Self::ThinkingTokens => "thinking_tokens",
            Self::TaskStarted => "task_started",
            Self::TaskProgress => "task_progress",
            Self::TaskUpdated => "task_updated",
            Self::TaskNotification => "task_notification",
            Self::ApiRetry => "api_retry",
            Self::ControlRequestProgress => "control_request_progress",
            Self::ModelRefusalFallback => "model_refusal_fallback",
            Self::ModelRefusalNoFallback => "model_refusal_no_fallback",
            Self::LocalCommandOutput => "local_command_output",
            Self::HookStarted => "hook_started",
            Self::HookProgress => "hook_progress",
            Self::HookResponse => "hook_response",
            Self::PluginInstall => "plugin_install",
            Self::BackgroundTasksChanged => "background_tasks_changed",
            Self::SessionStateChanged => "session_state_changed",
            Self::WorkerShuttingDown => "worker_shutting_down",
            Self::CommandsChanged => "commands_changed",
            Self::Notification => "notification",
            Self::FilesPersisted => "files_persisted",
            Self::MemoryRecall => "memory_recall",
            Self::ElicitationComplete => "elicitation_complete",
            Self::PermissionDenied => "permission_denied",
            Self::MirrorError => "mirror_error",
            Self::Informational => "informational",
            Self::CodeChangePublished => "code_change_published",
            Self::VcsStateChanged => "vcs_state_changed",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for SystemSubtype {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for SystemSubtype {
    fn from(s: &str) -> Self {
        match s {
            "init" => Self::Init,
            "status" => Self::Status,
            "compact_boundary" => Self::CompactBoundary,
            "thinking_tokens" => Self::ThinkingTokens,
            "task_started" => Self::TaskStarted,
            "task_progress" => Self::TaskProgress,
            "task_updated" => Self::TaskUpdated,
            "task_notification" => Self::TaskNotification,
            "api_retry" => Self::ApiRetry,
            "control_request_progress" => Self::ControlRequestProgress,
            "model_refusal_fallback" => Self::ModelRefusalFallback,
            "model_refusal_no_fallback" => Self::ModelRefusalNoFallback,
            "local_command_output" => Self::LocalCommandOutput,
            "hook_started" => Self::HookStarted,
            "hook_progress" => Self::HookProgress,
            "hook_response" => Self::HookResponse,
            "plugin_install" => Self::PluginInstall,
            "background_tasks_changed" => Self::BackgroundTasksChanged,
            "session_state_changed" => Self::SessionStateChanged,
            "worker_shutting_down" => Self::WorkerShuttingDown,
            "commands_changed" => Self::CommandsChanged,
            "notification" => Self::Notification,
            "files_persisted" => Self::FilesPersisted,
            "memory_recall" => Self::MemoryRecall,
            "elicitation_complete" => Self::ElicitationComplete,
            "permission_denied" => Self::PermissionDenied,
            "mirror_error" => Self::MirrorError,
            "informational" => Self::Informational,
            "code_change_published" => Self::CodeChangePublished,
            "vcs_state_changed" => Self::VcsStateChanged,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for SystemSubtype {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for SystemSubtype {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// Known message roles.
///
/// Used in `MessageContent` and `AssistantMessageContent` to indicate the
/// speaker of a message.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MessageRole {
    User,
    Assistant,
    /// A role not yet known to this version of the crate.
    Unknown(String),
}

impl MessageRole {
    pub fn as_str(&self) -> &str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for MessageRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for MessageRole {
    fn from(s: &str) -> Self {
        match s {
            "user" => Self::User,
            "assistant" => Self::Assistant,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for MessageRole {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for MessageRole {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// What triggered a context compaction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CompactionTrigger {
    /// Automatic compaction triggered by token limit.
    Auto,
    /// User-initiated compaction (e.g., /compact command).
    Manual,
    /// A trigger not yet known to this version of the crate.
    Unknown(String),
}

impl CompactionTrigger {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Auto => "auto",
            Self::Manual => "manual",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for CompactionTrigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for CompactionTrigger {
    fn from(s: &str) -> Self {
        match s {
            "auto" => Self::Auto,
            "manual" => Self::Manual,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for CompactionTrigger {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for CompactionTrigger {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// Reason why the assistant stopped generating.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StopReason {
    /// The assistant reached a natural end of its turn.
    EndTurn,
    /// The response hit the maximum token limit.
    MaxTokens,
    /// The assistant wants to use a tool.
    ToolUse,
    /// A stop reason not yet known to this version of the crate.
    Unknown(String),
}

impl StopReason {
    pub fn as_str(&self) -> &str {
        match self {
            Self::EndTurn => "end_turn",
            Self::MaxTokens => "max_tokens",
            Self::ToolUse => "tool_use",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for StopReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for StopReason {
    fn from(s: &str) -> Self {
        match s {
            "end_turn" => Self::EndTurn,
            "max_tokens" => Self::MaxTokens,
            "tool_use" => Self::ToolUse,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for StopReason {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for StopReason {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// How the API key was sourced for the session.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ApiKeySource {
    /// No API key provided.
    None,
    User,
    Project,
    Org,
    Temporary,
    Oauth,
    /// A source not yet known to this version of the crate.
    Unknown(String),
}

impl ApiKeySource {
    pub fn as_str(&self) -> &str {
        match self {
            Self::None => "none",
            Self::User => "user",
            Self::Project => "project",
            Self::Org => "org",
            Self::Temporary => "temporary",
            Self::Oauth => "oauth",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for ApiKeySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for ApiKeySource {
    fn from(s: &str) -> Self {
        match s {
            "none" => Self::None,
            "user" => Self::User,
            "project" => Self::Project,
            "org" => Self::Org,
            "temporary" => Self::Temporary,
            "oauth" => Self::Oauth,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for ApiKeySource {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ApiKeySource {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// Output formatting style for the session.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OutputStyle {
    /// Default output style.
    Default,
    /// A style not yet known to this version of the crate.
    Unknown(String),
}

impl OutputStyle {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Default => "default",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for OutputStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for OutputStyle {
    fn from(s: &str) -> Self {
        match s {
            "default" => Self::Default,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for OutputStyle {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for OutputStyle {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// Permission mode reported in init messages.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InitPermissionMode {
    /// Default permission mode.
    Default,
    AcceptEdits,
    BypassPermissions,
    Plan,
    DontAsk,
    Auto,
    /// A mode not yet known to this version of the crate.
    Unknown(String),
}

impl InitPermissionMode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Default => "default",
            Self::AcceptEdits => "acceptEdits",
            Self::BypassPermissions => "bypassPermissions",
            Self::Plan => "plan",
            Self::DontAsk => "dontAsk",
            Self::Auto => "auto",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for InitPermissionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for InitPermissionMode {
    fn from(s: &str) -> Self {
        match s {
            "default" => Self::Default,
            "acceptEdits" => Self::AcceptEdits,
            "bypassPermissions" => Self::BypassPermissions,
            "plan" => Self::Plan,
            "dontAsk" => Self::DontAsk,
            "auto" => Self::Auto,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for InitPermissionMode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for InitPermissionMode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// Status of an ongoing operation (e.g., context compaction).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StatusMessageStatus {
    /// Context compaction is in progress.
    Compacting,
    /// The CLI is issuing a request.
    Requesting,
    /// A status not yet known to this version of the crate.
    Unknown(String),
}

impl StatusMessageStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Compacting => "compacting",
            Self::Requesting => "requesting",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for StatusMessageStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for StatusMessageStatus {
    fn from(s: &str) -> Self {
        match s {
            "compacting" => Self::Compacting,
            "requesting" => Self::Requesting,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for StatusMessageStatus {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for StatusMessageStatus {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// Serialize an optional UUID as a string
pub(crate) fn serialize_optional_uuid<S>(
    uuid: &Option<Uuid>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match uuid {
        Some(id) => serializer.serialize_str(&id.to_string()),
        None => serializer.serialize_none(),
    }
}

/// Deserialize an optional UUID from a string
pub(crate) fn deserialize_optional_uuid<'de, D>(deserializer: D) -> Result<Option<Uuid>, D::Error>
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

/// Message provenance. The `kind` field is the stable discriminator; variant
/// specific fields are preserved in `extra` for forward-compatible access.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageOrigin {
    pub kind: String,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// Metadata attached when user-visible transcript content summarizes prior messages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SummarizeMetadata {
    pub messages_summarized: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_context: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
}

/// MCP metadata passed through on user-message wrappers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpMeta {
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<Value>,
}

/// Display metadata for a `tool_result` block carried on the user wrapper.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolResultMeta {
    /// The `tool_use_id` of the matching `tool_result` block.
    pub id: String,
    /// Harness-stamped reason an `is_error: true` result did not carry the
    /// tool's own execution output (`user-rejected`, `permission-rule`,
    /// `automode-*`, `interrupted`, `cancelled`). Open set — treat
    /// unrecognized values as valid reasons; absent means the tool ran to
    /// completion.
    pub non_execution_kind: String,
    /// The deny comment a human typed at a permission prompt, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_feedback: Option<String>,
}

/// User message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub message: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none", alias = "sessionId")]
    #[serde(
        serialize_with = "serialize_optional_uuid",
        deserialize_with = "deserialize_optional_uuid"
    )]
    pub session_id: Option<Uuid>,
    /// Parent tool use ID for nested agent messages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_tool_use_id: Option<String>,
    /// Message-level unique identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    /// CLI-emitted ISO-8601 timestamp for the message (present on echoed tool results).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    /// Structured tool result data echoed by the CLI alongside the `tool_result`
    /// content block. The shape depends on which tool produced it (e.g. for
    /// `AskUserQuestion` it is `{ questions, answers }`; for `Bash` it is
    /// `{ stdout, stderr, exit_code, ... }`). Stored as raw JSON to preserve
    /// wire fidelity; use [`UserMessage::tool_use_result_as`] to parse into a
    /// typed shape when you know which tool was invoked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_result: Option<serde_json::Value>,
    /// Subagent type, when this user message is the prompt echoed into a
    /// `local_agent` subagent (e.g. `general-purpose`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_type: Option<String>,
    /// Short description of the subagent task, present alongside `subagent_type`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<MessageOrigin>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "isSynthetic")]
    pub is_synthetic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "shouldQuery")]
    pub should_query: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_meta: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_visible_in_transcript_only: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_virtual: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_compact_summary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summarize_metadata: Option<SummarizeMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_meta: Option<McpMeta>,
    /// Display metadata for this message's `tool_result` blocks, keyed by
    /// `tool_use_id`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result_meta: Option<Vec<ToolResultMeta>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_tool_use_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_tool_assistant_uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_paste_ids: Option<Vec<u64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inbound_origin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "isReplay")]
    pub is_replay: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_attachments: Option<Vec<Value>>,
}

impl UserMessage {
    /// Parse the `tool_use_result` field into a caller-specified type.
    ///
    /// Returns `None` if `tool_use_result` is absent, otherwise returns the
    /// deserialization result. The caller must know which tool produced the
    /// result and supply a matching type — e.g. for `AskUserQuestion` use
    /// [`AskUserQuestionInput`](crate::AskUserQuestionInput), whose
    /// `questions` + `answers` fields match the wire result shape.
    pub fn tool_use_result_as<T: serde::de::DeserializeOwned>(
        &self,
    ) -> Option<Result<T, serde_json::Error>> {
        self.tool_use_result
            .as_ref()
            .map(|v| serde_json::from_value(v.clone()))
    }

    /// Parse the `tool_use_result` as a subagent (`Task`) run result.
    ///
    /// When this user message echoes the result of a `Task` tool call, the CLI
    /// attaches a structured `tool_use_result` carrying the subagent's token,
    /// timing, and tool-use accounting. Returns `None` when the field is absent
    /// or does not parse as a [`SubagentResult`].
    ///
    /// Summing [`SubagentResult::total_tokens`] across every `Task` result in a
    /// session yields the subagent token rollup the CLI renders as
    /// `subagent_tokens` in its terminal `<usage>` block.
    pub fn subagent_result(&self) -> Option<SubagentResult> {
        self.tool_use_result
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

/// Token, timing, and tool-use accounting for a completed subagent (`Task`) run.
///
/// The Claude CLI echoes this object in the `tool_use_result` of a `Task` tool's
/// result message. It is the typed source of truth for subagent token
/// attribution: the per-run [`total_tokens`](Self::total_tokens),
/// [`total_duration_ms`](Self::total_duration_ms), and
/// [`total_tool_use_count`](Self::total_tool_use_count) correspond to the
/// `subagent_tokens` / `duration_ms` / `tool_uses` line items the CLI renders in
/// its human-readable `<usage>` block, and [`usage`](Self::usage) carries the
/// full per-model token breakdown for the run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentResult {
    /// Completion status of the subagent run (e.g. `"completed"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// The prompt the subagent was launched with.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Stable identifier of the spawned subagent.
    #[serde(rename = "agentId", skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// Subagent type that ran (e.g. `general-purpose`, `Explore`).
    #[serde(rename = "agentType", skip_serializing_if = "Option::is_none")]
    pub agent_type: Option<String>,
    /// Final content blocks the subagent returned.
    #[serde(
        default,
        deserialize_with = "deserialize_content_blocks",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub content: Vec<ContentBlock>,
    /// Model the subagent actually resolved to (e.g. `claude-sonnet-4-6`).
    #[serde(rename = "resolvedModel", skip_serializing_if = "Option::is_none")]
    pub resolved_model: Option<String>,
    /// Wall-clock duration of the subagent run, in milliseconds.
    #[serde(rename = "totalDurationMs", skip_serializing_if = "Option::is_none")]
    pub total_duration_ms: Option<u64>,
    /// Total tokens consumed by the subagent — the `subagent_tokens` rollup line.
    #[serde(rename = "totalTokens", skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
    /// Number of tool invocations the subagent made.
    #[serde(rename = "totalToolUseCount", skip_serializing_if = "Option::is_none")]
    pub total_tool_use_count: Option<u64>,
    /// Detailed token / cache usage for the subagent run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<super::result::UsageInfo>,
    /// Per-category tool-use counts, present for some agent types (e.g. `Explore`).
    #[serde(rename = "toolStats", skip_serializing_if = "Option::is_none")]
    pub tool_stats: Option<SubagentToolStats>,
}

/// Per-category tool-use counts for a subagent run, from `tool_use_result.toolStats`.
///
/// The `extra` field captures any counters the CLI adds that aren't modeled here,
/// so new wire fields deserialize without error.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentToolStats {
    #[serde(default)]
    pub read_count: u64,
    #[serde(default)]
    pub search_count: u64,
    #[serde(default)]
    pub bash_count: u64,
    #[serde(default)]
    pub edit_file_count: u64,
    #[serde(default)]
    pub lines_added: u64,
    #[serde(default)]
    pub lines_removed: u64,
    #[serde(default)]
    pub other_tool_count: u64,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// Session-level subagent token rollup — the `<subagent_tokens>` /
/// `<agent_count>` line items the Claude CLI renders in its terminal
/// `<usage>` block.
///
/// The `stream-json` protocol does **not** carry this rollup on the `result`
/// frame's `usage` (confirmed against the CLI binary — the terminal renderer
/// computes it from `Task` tool results). Consumers that need it must
/// accumulate it the same way: feed every session message through
/// [`observe`](Self::observe) and read the totals at any point.
///
/// A `Task` result observed twice under the same `agentId` (e.g. a replayed
/// frame on resume) is counted once. Results with no `agentId` are counted
/// every time they are observed.
///
/// # Example
///
/// ```
/// use claude_codes::{ClaudeOutput, SubagentUsageRollup};
///
/// let mut rollup = SubagentUsageRollup::default();
/// let json = r#"{"type":"user","message":{"role":"user","content":[]},"session_id":"7fbc568e-2bd6-45aa-b217-a1cf80004ba1","tool_use_result":{"status":"completed","agentId":"ab52f22445470d454","totalDurationMs":1853,"totalTokens":10201,"totalToolUseCount":0}}"#;
/// let output: ClaudeOutput = serde_json::from_str(json).unwrap();
/// rollup.observe(&output);
/// assert_eq!(rollup.subagent_tokens, 10201);
/// assert_eq!(rollup.agent_count, 1);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SubagentUsageRollup {
    /// Total tokens consumed by subagents — sum of
    /// [`SubagentResult::total_tokens`] over every observed `Task` result.
    pub subagent_tokens: u64,
    /// Number of subagent runs observed (`<agent_count>`).
    pub agent_count: u32,
    /// Total subagent tool invocations — sum of `total_tool_use_count`.
    pub tool_uses: u64,
    /// Total subagent wall-clock milliseconds — sum of `total_duration_ms`.
    pub duration_ms: u64,
    seen_agent_ids: std::collections::BTreeSet<String>,
}

impl SubagentUsageRollup {
    /// Accumulate `output` into the rollup if it is a `Task` tool result.
    ///
    /// Returns `true` when the message contributed to the totals. Non-user
    /// messages, user messages without a `tool_use_result`, results from
    /// other tools, and duplicate `agentId`s are all ignored.
    pub fn observe(&mut self, output: &ClaudeOutput) -> bool {
        match output {
            ClaudeOutput::User(user) => self.observe_user(user),
            _ => false,
        }
    }

    /// Accumulate a user message's `Task` tool result, if it carries one.
    ///
    /// Every [`SubagentResult`] field is optional, so any JSON object in
    /// `tool_use_result` parses as one (e.g. a `Bash` or `ToolSearch`
    /// result). Only results carrying an `agentId` or a `totalTokens`
    /// line item are treated as genuine `Task` results.
    pub fn observe_user(&mut self, user: &UserMessage) -> bool {
        let Some(result) = user.subagent_result() else {
            return false;
        };
        if result.total_tokens.is_none() && result.agent_id.is_none() {
            return false;
        }
        if let Some(agent_id) = &result.agent_id {
            if !self.seen_agent_ids.insert(agent_id.clone()) {
                return false;
            }
        }
        self.agent_count += 1;
        self.subagent_tokens += result.total_tokens.unwrap_or(0);
        self.tool_uses += result.total_tool_use_count.unwrap_or(0);
        self.duration_ms += result.total_duration_ms.unwrap_or(0);
        true
    }
}

/// Message content with role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContent {
    pub role: MessageRole,
    #[serde(deserialize_with = "deserialize_content_blocks")]
    pub content: Vec<ContentBlock>,
}

/// System message with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMessage {
    pub subtype: SystemSubtype,
    #[serde(flatten)]
    pub data: Value, // Captures all other fields
}

impl SystemMessage {
    /// Check if this is an init message
    pub fn is_init(&self) -> bool {
        self.subtype == SystemSubtype::Init
    }

    /// Check if this is a status message
    pub fn is_status(&self) -> bool {
        self.subtype == SystemSubtype::Status
    }

    /// Check if this is a compact_boundary message
    pub fn is_compact_boundary(&self) -> bool {
        self.subtype == SystemSubtype::CompactBoundary
    }

    /// Try to parse as an init message
    pub fn as_init(&self) -> Option<InitMessage> {
        if self.subtype != SystemSubtype::Init {
            return None;
        }
        serde_json::from_value(self.data.clone()).ok()
    }

    /// Try to parse as a status message
    pub fn as_status(&self) -> Option<StatusMessage> {
        if self.subtype != SystemSubtype::Status {
            return None;
        }
        serde_json::from_value(self.data.clone()).ok()
    }

    /// Try to parse as a compact_boundary message
    pub fn as_compact_boundary(&self) -> Option<CompactBoundaryMessage> {
        if self.subtype != SystemSubtype::CompactBoundary {
            return None;
        }
        serde_json::from_value(self.data.clone()).ok()
    }

    /// Check if this is a task_started message
    pub fn is_task_started(&self) -> bool {
        self.subtype == SystemSubtype::TaskStarted
    }

    /// Check if this is a task_progress message
    pub fn is_task_progress(&self) -> bool {
        self.subtype == SystemSubtype::TaskProgress
    }

    /// Check if this is a task_notification message
    pub fn is_task_notification(&self) -> bool {
        self.subtype == SystemSubtype::TaskNotification
    }

    /// Try to parse as a task_started message
    pub fn as_task_started(&self) -> Option<TaskStartedMessage> {
        if self.subtype != SystemSubtype::TaskStarted {
            return None;
        }
        serde_json::from_value(self.data.clone()).ok()
    }

    /// Try to parse as a task_progress message
    pub fn as_task_progress(&self) -> Option<TaskProgressMessage> {
        if self.subtype != SystemSubtype::TaskProgress {
            return None;
        }
        serde_json::from_value(self.data.clone()).ok()
    }

    /// Try to parse as a task_notification message
    pub fn as_task_notification(&self) -> Option<TaskNotificationMessage> {
        if self.subtype != SystemSubtype::TaskNotification {
            return None;
        }
        serde_json::from_value(self.data.clone()).ok()
    }

    /// Check if this is a task_updated message
    pub fn is_task_updated(&self) -> bool {
        self.subtype == SystemSubtype::TaskUpdated
    }

    /// Try to parse as a task_updated message
    pub fn as_task_updated(&self) -> Option<TaskUpdatedMessage> {
        if self.subtype != SystemSubtype::TaskUpdated {
            return None;
        }
        serde_json::from_value(self.data.clone()).ok()
    }

    /// Check if this is a thinking_tokens message
    pub fn is_thinking_tokens(&self) -> bool {
        self.subtype == SystemSubtype::ThinkingTokens
    }

    /// Try to parse as a thinking_tokens message
    pub fn as_thinking_tokens(&self) -> Option<ThinkingTokensMessage> {
        if self.subtype != SystemSubtype::ThinkingTokens {
            return None;
        }
        serde_json::from_value(self.data.clone()).ok()
    }

    /// Parse any typed system subtype known to this crate version.
    pub fn as_known_system_event(&self) -> Option<KnownSystemEvent> {
        macro_rules! parse {
            ($variant:ident, $ty:ty) => {
                serde_json::from_value::<$ty>(self.data.clone())
                    .ok()
                    .map(KnownSystemEvent::$variant)
            };
        }

        match self.subtype {
            SystemSubtype::Init => parse!(Init, InitMessage),
            SystemSubtype::Status => parse!(Status, StatusMessage),
            SystemSubtype::CompactBoundary => parse!(CompactBoundary, CompactBoundaryMessage),
            SystemSubtype::ThinkingTokens => parse!(ThinkingTokens, ThinkingTokensMessage),
            SystemSubtype::TaskStarted => parse!(TaskStarted, TaskStartedMessage),
            SystemSubtype::TaskProgress => parse!(TaskProgress, TaskProgressMessage),
            SystemSubtype::TaskUpdated => parse!(TaskUpdated, TaskUpdatedMessage),
            SystemSubtype::TaskNotification => parse!(TaskNotification, TaskNotificationMessage),
            SystemSubtype::ApiRetry => parse!(ApiRetry, ApiRetryMessage),
            SystemSubtype::ControlRequestProgress => {
                parse!(ControlRequestProgress, ControlRequestProgressMessage)
            }
            SystemSubtype::ModelRefusalFallback => {
                parse!(ModelRefusalFallback, ModelRefusalFallbackMessage)
            }
            SystemSubtype::ModelRefusalNoFallback => {
                parse!(ModelRefusalNoFallback, ModelRefusalNoFallbackMessage)
            }
            SystemSubtype::LocalCommandOutput => {
                parse!(LocalCommandOutput, LocalCommandOutputMessage)
            }
            SystemSubtype::HookStarted => parse!(HookStarted, HookStartedMessage),
            SystemSubtype::HookProgress => parse!(HookProgress, HookProgressMessage),
            SystemSubtype::HookResponse => parse!(HookResponse, HookResponseMessage),
            SystemSubtype::PluginInstall => parse!(PluginInstall, PluginInstallMessage),
            SystemSubtype::BackgroundTasksChanged => {
                parse!(BackgroundTasksChanged, BackgroundTasksChangedMessage)
            }
            SystemSubtype::SessionStateChanged => {
                parse!(SessionStateChanged, SessionStateChangedMessage)
            }
            SystemSubtype::WorkerShuttingDown => {
                parse!(WorkerShuttingDown, WorkerShuttingDownMessage)
            }
            SystemSubtype::CommandsChanged => parse!(CommandsChanged, CommandsChangedMessage),
            SystemSubtype::Notification => parse!(Notification, NotificationMessage),
            SystemSubtype::FilesPersisted => parse!(FilesPersisted, FilesPersistedMessage),
            SystemSubtype::MemoryRecall => parse!(MemoryRecall, MemoryRecallMessage),
            SystemSubtype::ElicitationComplete => {
                parse!(ElicitationComplete, ElicitationCompleteMessage)
            }
            SystemSubtype::PermissionDenied => parse!(PermissionDenied, PermissionDeniedMessage),
            SystemSubtype::MirrorError => parse!(MirrorError, MirrorErrorMessage),
            SystemSubtype::Informational => parse!(Informational, InformationalMessage),
            SystemSubtype::CodeChangePublished => {
                parse!(CodeChangePublished, CodeChangePublishedMessage)
            }
            SystemSubtype::VcsStateChanged => parse!(VcsStateChanged, VcsStateChangedMessage),
            SystemSubtype::Unknown(_) => None,
        }
    }

    /// Re-serialize this system message's payload through the typed view that
    /// matches its `subtype`, returning the result as JSON.
    ///
    /// Used by the wrapping audit ([`crate::io::audit_frame`]) to verify that a
    /// subtype's dedicated struct captures every wire field: the audit compares
    /// this against the raw [`SystemMessage::data`]. Returns `None` for subtypes
    /// this crate version has no dedicated struct for (including
    /// [`SystemSubtype::Unknown`]) — those are reported as not fully wrapped.
    pub fn typed_value(&self) -> Option<Value> {
        fn reserialize<T: Serialize>(parsed: Option<T>) -> Option<Value> {
            parsed.and_then(|v| serde_json::to_value(v).ok())
        }
        match self.subtype {
            SystemSubtype::Init => reserialize(self.as_init()),
            SystemSubtype::Status => reserialize(self.as_status()),
            SystemSubtype::CompactBoundary => reserialize(self.as_compact_boundary()),
            SystemSubtype::ThinkingTokens => reserialize(self.as_thinking_tokens()),
            SystemSubtype::TaskStarted => reserialize(self.as_task_started()),
            SystemSubtype::TaskProgress => reserialize(self.as_task_progress()),
            SystemSubtype::TaskUpdated => reserialize(self.as_task_updated()),
            SystemSubtype::TaskNotification => reserialize(self.as_task_notification()),
            SystemSubtype::ApiRetry => reserialize(parse_system::<ApiRetryMessage>(self)),
            SystemSubtype::ControlRequestProgress => {
                reserialize(parse_system::<ControlRequestProgressMessage>(self))
            }
            SystemSubtype::ModelRefusalFallback => {
                reserialize(parse_system::<ModelRefusalFallbackMessage>(self))
            }
            SystemSubtype::ModelRefusalNoFallback => {
                reserialize(parse_system::<ModelRefusalNoFallbackMessage>(self))
            }
            SystemSubtype::LocalCommandOutput => {
                reserialize(parse_system::<LocalCommandOutputMessage>(self))
            }
            SystemSubtype::HookStarted => reserialize(parse_system::<HookStartedMessage>(self)),
            SystemSubtype::HookProgress => reserialize(parse_system::<HookProgressMessage>(self)),
            SystemSubtype::HookResponse => reserialize(parse_system::<HookResponseMessage>(self)),
            SystemSubtype::PluginInstall => reserialize(parse_system::<PluginInstallMessage>(self)),
            SystemSubtype::BackgroundTasksChanged => {
                reserialize(parse_system::<BackgroundTasksChangedMessage>(self))
            }
            SystemSubtype::SessionStateChanged => {
                reserialize(parse_system::<SessionStateChangedMessage>(self))
            }
            SystemSubtype::WorkerShuttingDown => {
                reserialize(parse_system::<WorkerShuttingDownMessage>(self))
            }
            SystemSubtype::CommandsChanged => {
                reserialize(parse_system::<CommandsChangedMessage>(self))
            }
            SystemSubtype::Notification => reserialize(parse_system::<NotificationMessage>(self)),
            SystemSubtype::FilesPersisted => {
                reserialize(parse_system::<FilesPersistedMessage>(self))
            }
            SystemSubtype::MemoryRecall => reserialize(parse_system::<MemoryRecallMessage>(self)),
            SystemSubtype::ElicitationComplete => {
                reserialize(parse_system::<ElicitationCompleteMessage>(self))
            }
            SystemSubtype::PermissionDenied => {
                reserialize(parse_system::<PermissionDeniedMessage>(self))
            }
            SystemSubtype::MirrorError => reserialize(parse_system::<MirrorErrorMessage>(self)),
            SystemSubtype::Informational => reserialize(parse_system::<InformationalMessage>(self)),
            SystemSubtype::CodeChangePublished => {
                reserialize(parse_system::<CodeChangePublishedMessage>(self))
            }
            SystemSubtype::VcsStateChanged => {
                reserialize(parse_system::<VcsStateChangedMessage>(self))
            }
            SystemSubtype::Unknown(_) => None,
        }
    }
}

fn parse_system<T: serde::de::DeserializeOwned>(message: &SystemMessage) -> Option<T> {
    serde_json::from_value(message.data.clone()).ok()
}

/// Owned typed view over any known system message subtype.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KnownSystemEvent {
    Init(InitMessage),
    Status(StatusMessage),
    CompactBoundary(CompactBoundaryMessage),
    ThinkingTokens(ThinkingTokensMessage),
    TaskStarted(TaskStartedMessage),
    TaskProgress(TaskProgressMessage),
    TaskUpdated(TaskUpdatedMessage),
    TaskNotification(TaskNotificationMessage),
    ApiRetry(ApiRetryMessage),
    ControlRequestProgress(ControlRequestProgressMessage),
    ModelRefusalFallback(ModelRefusalFallbackMessage),
    ModelRefusalNoFallback(ModelRefusalNoFallbackMessage),
    LocalCommandOutput(LocalCommandOutputMessage),
    HookStarted(HookStartedMessage),
    HookProgress(HookProgressMessage),
    HookResponse(HookResponseMessage),
    PluginInstall(PluginInstallMessage),
    BackgroundTasksChanged(BackgroundTasksChangedMessage),
    SessionStateChanged(SessionStateChangedMessage),
    WorkerShuttingDown(WorkerShuttingDownMessage),
    CommandsChanged(CommandsChangedMessage),
    Notification(NotificationMessage),
    FilesPersisted(FilesPersistedMessage),
    MemoryRecall(MemoryRecallMessage),
    ElicitationComplete(ElicitationCompleteMessage),
    PermissionDenied(PermissionDeniedMessage),
    MirrorError(MirrorErrorMessage),
    Informational(InformationalMessage),
    CodeChangePublished(CodeChangePublishedMessage),
    VcsStateChanged(VcsStateChangedMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRetryMessage {
    pub attempt: u64,
    pub max_retries: u64,
    pub retry_delay_ms: u64,
    pub error_status: Option<u16>,
    pub error: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlRequestProgressMessage {
    pub request_id: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_delay_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_status: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRefusalFallbackMessage {
    pub trigger: String,
    pub direction: String,
    pub original_model: String,
    pub fallback_model: String,
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_refusal_category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_refusal_explanation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retracted_message_uuids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused_user_message_uuid: Option<String>,
    pub content: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRefusalNoFallbackMessage {
    pub original_model: String,
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_refusal_category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_refusal_explanation: Option<String>,
    pub content: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalCommandOutputMessage {
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookStartedMessage {
    pub hook_id: String,
    pub hook_name: String,
    pub hook_event: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookProgressMessage {
    pub hook_id: String,
    pub hook_name: String,
    pub hook_event: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResponseMessage {
    pub hook_id: String,
    pub hook_name: String,
    pub hook_event: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub outcome: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInstallMessage {
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundTasksChangedMessage {
    pub tasks: Vec<BackgroundTaskInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundTaskInfo {
    pub task_id: String,
    pub task_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStateChangedMessage {
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerShuttingDownMessage {
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandsChangedMessage {
    pub commands: Vec<CommandInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInfo {
    pub name: String,
    pub description: String,
    #[serde(rename = "argumentHint")]
    pub argument_hint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aliases: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationMessage {
    pub key: String,
    pub text: String,
    pub priority: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesPersistedMessage {
    pub files: Vec<PersistedFile>,
    pub failed: Vec<FailedPersistedFile>,
    pub processed_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedFile {
    pub filename: String,
    pub file_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedPersistedFile {
    pub filename: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecallMessage {
    pub mode: String,
    pub memories: Vec<MemoryRecallItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecallItem {
    pub path: String,
    pub scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationCompleteMessage {
    pub mcp_server_name: String,
    pub elicitation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionDeniedMessage {
    pub tool_name: String,
    pub tool_use_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_reason_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_reason: Option<String>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorErrorMessage {
    pub error: String,
    pub key: MirrorErrorKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorErrorKey {
    #[serde(rename = "projectKey")]
    pub project_key: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subpath: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InformationalMessage {
    pub content: String,
    pub level: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prevent_continuation: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// Plugin info from the init message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Plugin name
    pub name: String,
    /// Path to the plugin on disk
    pub path: String,
    /// Plugin registry source (e.g., "rust-analyzer-lsp@claude-plugins-official")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Installed plugin version (e.g., "1.0.0"). Added in CLI 2.1.219.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Plugin load diagnostic reported by system init.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginDiagnostic {
    pub plugin: String,
    #[serde(rename = "type")]
    pub diagnostic_type: String,
    pub message: String,
}

/// Memory paths reported by system init.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryPaths {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// An MCP server config entry that failed validation, reported by system
/// init (e.g. a `url` entry with no `type`). The affected server is skipped
/// and absent from `InitMessage::mcp_servers`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpServerError {
    pub name: String,
    /// Stable error category.
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

/// Init system message data - sent at session start
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitMessage {
    /// Session identifier
    pub session_id: String,
    /// Current working directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Model being used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// List of available tools
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
    /// MCP servers configured
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<Value>,
    /// Available slash commands (e.g., "compact", "cost", "review")
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slash_commands: Vec<String>,
    /// Available agent types (e.g., "Bash", "Explore", "Plan")
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<String>,
    /// Installed plugins
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plugins: Vec<PluginInfo>,
    /// Installed skills
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<Value>,
    /// Claude Code CLI version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_code_version: Option<String>,
    /// How the API key was sourced
    #[serde(skip_serializing_if = "Option::is_none", rename = "apiKeySource")]
    pub api_key_source: Option<ApiKeySource>,
    /// Output style
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_style: Option<OutputStyle>,
    /// Permission mode
    #[serde(skip_serializing_if = "Option::is_none", rename = "permissionMode")]
    pub permission_mode: Option<InitPermissionMode>,

    /// Message-level unique identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,

    /// Memory storage paths (e.g., {"auto": "/path/to/memory/"})
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_paths: Option<MemoryPaths>,

    /// Fast mode toggle state (e.g., "off")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fast_mode_state: Option<String>,

    /// Why fast mode can't serve right now. Absent when nothing blocks it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fast_mode_disabled_reason: Option<super::result::FastModeDisabledReason>,

    /// MCP server config entries (from `--mcp-config`) that failed validation
    /// and were skipped. Affected servers are absent from `mcp_servers`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_server_errors: Option<Vec<McpServerError>>,

    /// Whether analytics collection is disabled for this session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub analytics_disabled: Option<bool>,

    /// Whether product-feedback prompts are disabled for this session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_feedback_disabled: Option<bool>,

    /// API beta flags active for the session.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub betas: Vec<String>,

    /// Open-set protocol capability names supported by this CLI.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,

    /// Plugin load errors.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plugin_errors: Vec<PluginDiagnostic>,

    /// Plugin load warnings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plugin_warnings: Vec<PluginDiagnostic>,
}

/// Status system message - sent during operations like context compaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusMessage {
    /// Session identifier
    pub session_id: String,
    /// Current status (e.g., compacting) or null when complete
    pub status: Option<StatusMessageStatus>,
    /// Unique identifier for this message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    /// Current permission mode when changed mid-session.
    #[serde(skip_serializing_if = "Option::is_none", rename = "permissionMode")]
    pub permission_mode: Option<InitPermissionMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compact_result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compact_error: Option<String>,
}

/// Compact boundary message - marks where context compaction occurred
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactBoundaryMessage {
    /// Session identifier
    pub session_id: String,
    /// Metadata about the compaction
    pub compact_metadata: CompactMetadata,
    /// Human-readable summary of what was compacted, when the CLI emits one.
    ///
    /// Also accepted under the `content` / `text` wire keys.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "content",
        alias = "text"
    )]
    pub summary: Option<String>,
    /// Number of messages summarized in this compaction pass, when present.
    ///
    /// Also accepted under the `message_count` wire key.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "message_count"
    )]
    pub leaf_message_count: Option<u32>,
    /// Wall-clock duration of the compaction pass in milliseconds, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Unique identifier for this message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    /// Logical parent across the compaction boundary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logical_parent_uuid: Option<Option<String>>,
}

/// Metadata about context compaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactMetadata {
    /// Number of tokens before compaction
    pub pre_tokens: u64,
    /// What triggered the compaction
    pub trigger: CompactionTrigger,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cumulative_dropped_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_context: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub messages_summarized: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precomputed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_compact_discovered_tools: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preserved_segment: Option<PreservedSegment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preserved_messages: Option<PreservedMessages>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PreservedSegment {
    pub head_uuid: String,
    pub anchor_uuid: String,
    pub tail_uuid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PreservedMessages {
    pub anchor_uuid: String,
    pub uuids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub all_uuids: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Task system message types (task_started, task_progress, task_notification)
// ---------------------------------------------------------------------------

/// Cumulative usage statistics for a background task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskUsage {
    /// Wall-clock milliseconds since the task started.
    pub duration_ms: u64,
    /// Total number of tool calls made so far.
    pub tool_uses: u64,
    /// Total tokens consumed so far.
    pub total_tokens: u64,
}

/// The kind of background task.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TaskType {
    /// A sub-agent task (e.g., Explore, Plan).
    LocalAgent,
    /// A background bash command.
    LocalBash,
    /// A local workflow task.
    LocalWorkflow,
    /// A task type not yet known to this version of the crate.
    Unknown(String),
}

impl TaskType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::LocalAgent => "local_agent",
            Self::LocalBash => "local_bash",
            Self::LocalWorkflow => "local_workflow",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for TaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for TaskType {
    fn from(s: &str) -> Self {
        match s {
            "local_agent" => Self::LocalAgent,
            "local_bash" => Self::LocalBash,
            "local_workflow" => Self::LocalWorkflow,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for TaskType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for TaskType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// Completion status of a background task.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Killed,
    Paused,
    Stopped,
    Unknown(String),
}

impl TaskStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Killed => "killed",
            Self::Paused => "paused",
            Self::Stopped => "stopped",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for TaskStatus {
    fn from(s: &str) -> Self {
        match s {
            "pending" => Self::Pending,
            "running" => Self::Running,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            "killed" => Self::Killed,
            "paused" => Self::Paused,
            "stopped" => Self::Stopped,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for TaskStatus {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for TaskStatus {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// `task_started` system message — emitted once when a background task begins.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStartedMessage {
    pub session_id: String,
    pub task_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_type: Option<TaskType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
    pub description: String,
    /// The subagent type for `local_agent` tasks (e.g. `general-purpose`,
    /// `Explore`). Absent for `local_bash` tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subagent_type: Option<String>,
    /// The prompt handed to the subagent. Present for `local_agent` tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_transcript: Option<bool>,
    pub uuid: String,
}

/// `task_updated` system message — emitted when a background task's state
/// changes (e.g. transitions to `completed`). Carries a partial `patch` of the
/// fields that changed rather than the full task record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskUpdatedMessage {
    pub session_id: String,
    pub task_id: String,
    pub patch: TaskPatch,
    pub uuid: String,
}

/// The partial update carried by a [`TaskUpdatedMessage`]. Every field is
/// optional because the CLI only sends the keys that changed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<TaskStatus>,
    /// Wall-clock epoch milliseconds when the task finished, when the patch
    /// reports completion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_paused_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_backgrounded: Option<bool>,
}

/// `thinking_tokens` system message — emitted as the model streams extended
/// thinking, reporting the running estimate of thinking tokens consumed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingTokensMessage {
    pub session_id: String,
    /// Running estimate of total thinking tokens for the current turn.
    pub estimated_tokens: u64,
    /// Increase in the estimate since the previous `thinking_tokens` event.
    pub estimated_tokens_delta: u64,
    pub uuid: String,
}

/// `task_progress` system message — emitted periodically as a background
/// agent task executes tools. Not emitted for `local_bash` tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgressMessage {
    pub session_id: String,
    pub task_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_tool_name: Option<String>,
    pub usage: TaskUsage,
    /// Subagent type for `local_agent` tasks (e.g. `Explore`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subagent_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub uuid: String,
}

/// `task_notification` system message — emitted once when a background
/// task completes or fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNotificationMessage {
    pub session_id: String,
    pub task_id: String,
    pub status: TaskStatus,
    pub summary: String,
    pub output_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TaskUsage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_transcript: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

/// API error category attached to assistant wrapper frames.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssistantErrorKind {
    AuthenticationFailed,
    OauthOrgNotAllowed,
    BillingError,
    RateLimit,
    Overloaded,
    InvalidRequest,
    ModelNotFound,
    ServerError,
    UnknownError,
    MaxOutputTokens,
    Unknown(String),
}

impl AssistantErrorKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::AuthenticationFailed => "authentication_failed",
            Self::OauthOrgNotAllowed => "oauth_org_not_allowed",
            Self::BillingError => "billing_error",
            Self::RateLimit => "rate_limit",
            Self::Overloaded => "overloaded",
            Self::InvalidRequest => "invalid_request",
            Self::ModelNotFound => "model_not_found",
            Self::ServerError => "server_error",
            Self::UnknownError => "unknown",
            Self::MaxOutputTokens => "max_output_tokens",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for AssistantErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for AssistantErrorKind {
    fn from(s: &str) -> Self {
        match s {
            "authentication_failed" => Self::AuthenticationFailed,
            "oauth_org_not_allowed" => Self::OauthOrgNotAllowed,
            "billing_error" => Self::BillingError,
            "rate_limit" => Self::RateLimit,
            "overloaded" => Self::Overloaded,
            "invalid_request" => Self::InvalidRequest,
            "model_not_found" => Self::ModelNotFound,
            "server_error" => Self::ServerError,
            "unknown" => Self::UnknownError,
            "max_output_tokens" => Self::MaxOutputTokens,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for AssistantErrorKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for AssistantErrorKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// `code_change_published` system message — the session is now associated
/// with a published code change (a pull/merge request). Fires on creation and
/// whenever the session contributes to an existing one, so bind on every
/// event; re-emission for the same URL is possible and idempotent. Values are
/// scraped from captured command output — treat them as a binding hint and
/// verify against the forge before routing authenticated requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChangePublishedMessage {
    /// Forge classification derived from the URL's shape (`github`,
    /// `github-enterprise`, `gitlab`, `bitbucket` today). Open set — treat an
    /// unknown value as a valid provider, never as an error.
    pub provider: String,
    /// Web URL of the pull/merge request. Unverified.
    pub url: String,
    /// Repository path from the URL (`owner/name` on GitHub; may carry more
    /// segments on GitLab).
    pub repo: String,
    /// Provider-native change identifier — the PR/MR number as a string.
    pub identifier: String,
    pub uuid: String,
    pub session_id: String,
}

/// `vcs_state_changed` system message — a harness-observed shell command
/// mutated repository state. A cache-invalidation signal, deliberately
/// payload-free beyond classification: consumers re-read state (branch, head,
/// PR status) instead of decoding the event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcsStateChangedMessage {
    /// What class of mutation was observed. New kinds may be added — treat an
    /// unrecognized kind exactly like a recognized one (something changed).
    pub kind: VcsMutationKind,
    /// The session's working directory — a hint, not necessarily the mutated
    /// repo's path (`git -C` or an inner `cd` mutates elsewhere).
    pub cwd: String,
    pub uuid: String,
    pub session_id: String,
}

/// Mutation class carried by a [`VcsStateChangedMessage`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VcsMutationKind {
    Commit,
    Push,
    Merge,
    Rebase,
    /// A kind not yet known to this version of the crate.
    Unknown(String),
}

impl VcsMutationKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Commit => "commit",
            Self::Push => "push",
            Self::Merge => "merge",
            Self::Rebase => "rebase",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for VcsMutationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for VcsMutationKind {
    fn from(s: &str) -> Self {
        match s {
            "commit" => Self::Commit,
            "push" => Self::Push,
            "merge" => Self::Merge,
            "rebase" => Self::Rebase,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for VcsMutationKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for VcsMutationKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// Display metadata for a tool-use block carried on the assistant wrapper.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolUseMeta {
    pub id: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

/// Assistant message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub message: AssistantMessageContent,
    #[serde(alias = "sessionId")]
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_tool_use_id: Option<String>,
    /// Anthropic API request id that produced this message (e.g. `req_...`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Subagent type, when this assistant message was produced inside a
    /// `local_agent` subagent (e.g. `general-purpose`, `Explore`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_type: Option<String>,
    /// Short description of the subagent task, present alongside `subagent_type`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AssistantErrorKind>,
    /// True when this message was truncated by an interrupt/abort before the
    /// stream completed — `stop_reason` was never received and the content
    /// may end mid-word. Absent on normally completed messages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aborted: Option<bool>,
    /// True when this turn continued the preceding truncated assistant turn
    /// inside its trailing signed thinking block (max-output-tokens
    /// recovery). Histories replayed through the bridge must carry the flag
    /// back so the normalizer keeps the run's prefix on the wire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resumed_from_incomplete_thinking: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supersedes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_use_meta: Vec<ToolUseMeta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_meta: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_virtual: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_api_error_message: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_error_status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advisor_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution_agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution_skill: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution_plugin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution_mcp_server: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution_mcp_tool: Option<String>,
}

/// Nested message content for assistant messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessageContent {
    pub id: String,
    /// The Anthropic API message type — always `"message"`.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub message_type: Option<String>,
    pub role: MessageRole,
    pub model: String,
    pub content: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<StopReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<AssistantUsage>,
    /// Details about why generation stopped
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_details: Option<Value>,
    /// Context management metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_management: Option<Value>,
}

/// Usage information for assistant messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantUsage {
    /// Number of input tokens
    #[serde(default)]
    pub input_tokens: u32,

    /// Number of output tokens
    #[serde(default)]
    pub output_tokens: u32,

    /// Tokens used to create cache
    #[serde(default)]
    pub cache_creation_input_tokens: u32,

    /// Tokens read from cache
    #[serde(default)]
    pub cache_read_input_tokens: u32,

    /// Service tier used (e.g., "standard")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,

    /// Detailed cache creation breakdown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation: Option<CacheCreationDetails>,

    /// Inference geography (e.g., "not_available")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inference_geo: Option<String>,
}

/// Detailed cache creation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheCreationDetails {
    /// Ephemeral 1-hour input tokens
    #[serde(default)]
    pub ephemeral_1h_input_tokens: u32,

    /// Ephemeral 5-minute input tokens
    #[serde(default)]
    pub ephemeral_5m_input_tokens: u32,
}

#[cfg(test)]
mod tests {
    use crate::io::ClaudeOutput;

    #[test]
    fn test_subagent_usage_rollup_accumulates_task_results() {
        use super::SubagentUsageRollup;

        let mut rollup = SubagentUsageRollup::default();

        let task_result = r#"{"type":"user","message":{"role":"user","content":[]},"session_id":"7fbc568e-2bd6-45aa-b217-a1cf80004ba1","tool_use_result":{"status":"completed","prompt":"Compute 6 times 7.","agentId":"ab52f22445470d454","agentType":"general-purpose","resolvedModel":"claude-sonnet-4-6","totalDurationMs":1853,"totalTokens":10201,"totalToolUseCount":3}}"#;
        let output: ClaudeOutput = serde_json::from_str(task_result).unwrap();
        assert!(rollup.observe(&output));
        assert_eq!(rollup.subagent_tokens, 10201);
        assert_eq!(rollup.agent_count, 1);
        assert_eq!(rollup.tool_uses, 3);
        assert_eq!(rollup.duration_ms, 1853);

        // Replayed frame with the same agentId is counted once.
        assert!(!rollup.observe(&output));
        assert_eq!(rollup.agent_count, 1);
        assert_eq!(rollup.subagent_tokens, 10201);

        // A second agent accumulates.
        let second = r#"{"type":"user","message":{"role":"user","content":[]},"session_id":"7fbc568e-2bd6-45aa-b217-a1cf80004ba1","tool_use_result":{"status":"completed","agentId":"ffff00001111","totalDurationMs":100,"totalTokens":500,"totalToolUseCount":1}}"#;
        let output: ClaudeOutput = serde_json::from_str(second).unwrap();
        assert!(rollup.observe(&output));
        assert_eq!(rollup.agent_count, 2);
        assert_eq!(rollup.subagent_tokens, 10701);
    }

    #[test]
    fn test_subagent_usage_rollup_ignores_non_task_results() {
        use super::SubagentUsageRollup;

        let mut rollup = SubagentUsageRollup::default();

        // A ToolSearch tool_use_result parses as an all-None SubagentResult;
        // it must not count as a subagent.
        let tool_search = r#"{"type":"user","message":{"role":"user","content":[]},"session_id":"7fbc568e-2bd6-45aa-b217-a1cf80004ba1","tool_use_result":{"matches":["TaskCreate"],"query":"select:TaskCreate","total_deferred_tools":27}}"#;
        let output: ClaudeOutput = serde_json::from_str(tool_search).unwrap();
        assert!(!rollup.observe(&output));

        // Plain user message without tool_use_result.
        let plain = r#"{"type":"user","message":{"role":"user","content":[]},"session_id":"7fbc568e-2bd6-45aa-b217-a1cf80004ba1"}"#;
        let output: ClaudeOutput = serde_json::from_str(plain).unwrap();
        assert!(!rollup.observe(&output));

        // Non-user frames are ignored.
        let system = r#"{"type":"system","subtype":"status","status":null,"session_id":"7fbc568e-2bd6-45aa-b217-a1cf80004ba1"}"#;
        let output: ClaudeOutput = serde_json::from_str(system).unwrap();
        assert!(!rollup.observe(&output));

        assert_eq!(rollup, SubagentUsageRollup::default());
    }

    #[test]
    fn test_subagent_usage_rollup_over_captured_session() {
        use super::SubagentUsageRollup;

        let mut rollup = SubagentUsageRollup::default();
        let fixture =
            include_str!("../../test_cases/subagent_sessions/general_purpose_compute.jsonl");
        for line in fixture.lines().filter(|l| !l.trim().is_empty()) {
            if let Ok(output) = serde_json::from_str::<ClaudeOutput>(line) {
                rollup.observe(&output);
            }
        }
        assert_eq!(rollup.agent_count, 1);
        assert_eq!(rollup.subagent_tokens, 10201);
    }

    #[test]
    fn test_system_message_init() {
        let json = r#"{
            "type": "system",
            "subtype": "init",
            "session_id": "test-session-123",
            "cwd": "/home/user/project",
            "model": "claude-sonnet-4",
            "tools": ["Bash", "Read", "Write"],
            "mcp_servers": [],
            "slash_commands": ["compact", "cost", "review"],
            "agents": ["Bash", "Explore", "Plan"],
            "plugins": [{"name": "rust-analyzer-lsp", "path": "/home/user/.claude/plugins/rust-analyzer-lsp/1.0.0"}],
            "skills": [],
            "claude_code_version": "2.1.15",
            "apiKeySource": "none",
            "output_style": "default",
            "permissionMode": "default"
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::System(sys) = output {
            assert!(sys.is_init());
            assert!(!sys.is_status());
            assert!(!sys.is_compact_boundary());

            let init = sys.as_init().expect("Should parse as init");
            assert_eq!(init.session_id, "test-session-123");
            assert_eq!(init.cwd, Some("/home/user/project".to_string()));
            assert_eq!(init.model, Some("claude-sonnet-4".to_string()));
            assert_eq!(init.tools, vec!["Bash", "Read", "Write"]);
            assert_eq!(init.slash_commands, vec!["compact", "cost", "review"]);
            assert_eq!(init.agents, vec!["Bash", "Explore", "Plan"]);
            assert_eq!(init.plugins.len(), 1);
            assert_eq!(init.plugins[0].name, "rust-analyzer-lsp");
            assert_eq!(init.claude_code_version, Some("2.1.15".to_string()));
            assert_eq!(init.api_key_source, Some(super::ApiKeySource::None));
            assert_eq!(init.output_style, Some(super::OutputStyle::Default));
            assert_eq!(
                init.permission_mode,
                Some(super::InitPermissionMode::Default)
            );
        } else {
            panic!("Expected System message");
        }
    }

    #[test]
    fn test_system_message_init_from_real_capture() {
        let json = include_str!("../../test_cases/tool_use_captures/tool_msg_0.json");
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::System(sys) = output {
            let init = sys.as_init().expect("Should parse real init capture");
            assert_eq!(init.slash_commands.len(), 8);
            assert!(init.slash_commands.contains(&"compact".to_string()));
            assert!(init.slash_commands.contains(&"review".to_string()));
            assert_eq!(init.agents.len(), 5);
            assert!(init.agents.contains(&"Bash".to_string()));
            assert!(init.agents.contains(&"Explore".to_string()));
            assert_eq!(init.plugins.len(), 1);
            assert_eq!(init.plugins[0].name, "rust-analyzer-lsp");
            assert_eq!(init.claude_code_version, Some("2.1.15".to_string()));
        } else {
            panic!("Expected System message");
        }
    }

    #[test]
    fn test_system_message_status() {
        let json = r#"{
            "type": "system",
            "subtype": "status",
            "session_id": "879c1a88-3756-4092-aa95-0020c4ed9692",
            "status": "compacting",
            "uuid": "32eb9f9d-5ef7-47ff-8fce-bbe22fe7ed93"
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::System(sys) = output {
            assert!(sys.is_status());
            assert!(!sys.is_init());

            let status = sys.as_status().expect("Should parse as status");
            assert_eq!(status.session_id, "879c1a88-3756-4092-aa95-0020c4ed9692");
            assert_eq!(status.status, Some(super::StatusMessageStatus::Compacting));
            assert_eq!(
                status.uuid,
                Some("32eb9f9d-5ef7-47ff-8fce-bbe22fe7ed93".to_string())
            );
        } else {
            panic!("Expected System message");
        }
    }

    #[test]
    fn test_system_message_status_null() {
        let json = r#"{
            "type": "system",
            "subtype": "status",
            "session_id": "879c1a88-3756-4092-aa95-0020c4ed9692",
            "status": null,
            "uuid": "92d9637e-d00e-418e-acd2-a504e3861c6a"
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::System(sys) = output {
            let status = sys.as_status().expect("Should parse as status");
            assert_eq!(status.status, None);
        } else {
            panic!("Expected System message");
        }
    }

    #[test]
    fn test_system_message_task_started() {
        let json = r#"{
            "type": "system",
            "subtype": "task_started",
            "session_id": "9abbc466-dad0-4b8e-b6b0-cad5eb7a16b9",
            "task_id": "b6daf3f",
            "task_type": "local_bash",
            "tool_use_id": "toolu_011rfSTFumpJZdCCfzeD7jaS",
            "description": "Wait for CI on PR #12",
            "uuid": "c4243261-c128-4747-b8c3-5e1c7c10eeb8"
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::System(sys) = output {
            assert!(sys.is_task_started());
            assert!(!sys.is_task_progress());
            assert!(!sys.is_task_notification());

            let task = sys.as_task_started().expect("Should parse as task_started");
            assert_eq!(task.session_id, "9abbc466-dad0-4b8e-b6b0-cad5eb7a16b9");
            assert_eq!(task.task_id, "b6daf3f");
            assert_eq!(task.task_type, Some(super::TaskType::LocalBash));
            assert_eq!(
                task.tool_use_id.as_deref(),
                Some("toolu_011rfSTFumpJZdCCfzeD7jaS")
            );
            assert_eq!(task.description, "Wait for CI on PR #12");
        } else {
            panic!("Expected System message");
        }
    }

    #[test]
    fn test_system_message_task_started_agent() {
        let json = r#"{
            "type": "system",
            "subtype": "task_started",
            "session_id": "bff4f716-17c1-4255-ab7b-eea9d33824e3",
            "task_id": "a4a7e0906e5fc64cc",
            "task_type": "local_agent",
            "tool_use_id": "toolu_01SFz9FwZ1cYgCSy8vRM7wep",
            "description": "Explore Scene/ArrayScene duplication",
            "uuid": "85a39f5a-e4d4-47f7-9a6d-1125f1a8035f"
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::System(sys) = output {
            let task = sys.as_task_started().expect("Should parse as task_started");
            assert_eq!(task.task_type, Some(super::TaskType::LocalAgent));
            assert_eq!(task.task_id, "a4a7e0906e5fc64cc");
        } else {
            panic!("Expected System message");
        }
    }

    #[test]
    fn test_system_message_task_progress() {
        let json = r#"{
            "type": "system",
            "subtype": "task_progress",
            "session_id": "bff4f716-17c1-4255-ab7b-eea9d33824e3",
            "task_id": "a4a7e0906e5fc64cc",
            "tool_use_id": "toolu_01SFz9FwZ1cYgCSy8vRM7wep",
            "description": "Reading src/jplephem/chebyshev.rs",
            "last_tool_name": "Read",
            "usage": {
                "duration_ms": 13996,
                "tool_uses": 9,
                "total_tokens": 38779
            },
            "uuid": "85a39f5a-e4d4-47f7-9a6d-1125f1a8035f"
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::System(sys) = output {
            assert!(sys.is_task_progress());
            assert!(!sys.is_task_started());

            let progress = sys
                .as_task_progress()
                .expect("Should parse as task_progress");
            assert_eq!(progress.task_id, "a4a7e0906e5fc64cc");
            assert_eq!(progress.description, "Reading src/jplephem/chebyshev.rs");
            assert_eq!(progress.last_tool_name.as_deref(), Some("Read"));
            assert_eq!(progress.usage.duration_ms, 13996);
            assert_eq!(progress.usage.tool_uses, 9);
            assert_eq!(progress.usage.total_tokens, 38779);
        } else {
            panic!("Expected System message");
        }
    }

    #[test]
    fn test_system_message_task_notification_completed() {
        let json = r#"{
            "type": "system",
            "subtype": "task_notification",
            "session_id": "bff4f716-17c1-4255-ab7b-eea9d33824e3",
            "task_id": "a0ba761e9dc9c316f",
            "tool_use_id": "toolu_01Ho6XVXFLVNjTQ9YqowdBXW",
            "status": "completed",
            "summary": "Agent \"Write Hipparcos data source doc\" completed",
            "output_file": "",
            "usage": {
                "duration_ms": 172300,
                "tool_uses": 11,
                "total_tokens": 42005
            },
            "uuid": "269f49b9-218d-4c8d-9f7e-3a5383a0c5b2"
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::System(sys) = output {
            assert!(sys.is_task_notification());

            let notif = sys
                .as_task_notification()
                .expect("Should parse as task_notification");
            assert_eq!(notif.status, super::TaskStatus::Completed);
            assert_eq!(
                notif.summary,
                "Agent \"Write Hipparcos data source doc\" completed"
            );
            assert_eq!(notif.output_file, Some("".to_string()));
            assert_eq!(
                notif.tool_use_id,
                Some("toolu_01Ho6XVXFLVNjTQ9YqowdBXW".to_string())
            );
            let usage = notif.usage.expect("Should have usage");
            assert_eq!(usage.duration_ms, 172300);
            assert_eq!(usage.tool_uses, 11);
            assert_eq!(usage.total_tokens, 42005);
        } else {
            panic!("Expected System message");
        }
    }

    #[test]
    fn test_system_message_task_notification_failed_no_usage() {
        let json = r#"{
            "type": "system",
            "subtype": "task_notification",
            "session_id": "ea629737-3c36-48a8-a1c4-ad761ad35784",
            "task_id": "b98f6a3",
            "status": "failed",
            "summary": "Background command \"Run FSM calibration\" failed with exit code 1",
            "output_file": "/tmp/claude-1000/tasks/b98f6a3.output"
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::System(sys) = output {
            let notif = sys
                .as_task_notification()
                .expect("Should parse as task_notification");
            assert_eq!(notif.status, super::TaskStatus::Failed);
            assert!(notif.tool_use_id.is_none());
            assert!(notif.usage.is_none());
            assert_eq!(
                notif.output_file,
                Some("/tmp/claude-1000/tasks/b98f6a3.output".to_string())
            );
        } else {
            panic!("Expected System message");
        }
    }

    /// Task system messages survive a `to_value` → `from_value` round-trip
    /// with their typed accessors still resolving. Mirrors the proxy/relay
    /// path where output is reparsed from a `serde_json::Value` rather than
    /// straight from the CLI's stdout, so a silently dropped or renamed field
    /// surfaces here instead of as a `None` downstream.
    #[test]
    fn test_task_messages_roundtrip_through_value() {
        let cases = [
            r#"{"type":"system","subtype":"task_started","session_id":"s1",
                "task_id":"t1","task_type":"local_bash","tool_use_id":"tu1",
                "description":"Sleep 3s","uuid":"u1"}"#,
            r#"{"type":"system","subtype":"task_progress","session_id":"s1",
                "task_id":"t1","tool_use_id":"tu1","description":"Running ls",
                "last_tool_name":"Bash",
                "usage":{"duration_ms":100,"tool_uses":1,"total_tokens":500},
                "uuid":"u2"}"#,
            r#"{"type":"system","subtype":"task_notification","session_id":"s1",
                "task_id":"t1","tool_use_id":"tu1","status":"completed",
                "summary":"done","output_file":"",
                "usage":{"duration_ms":100,"tool_uses":1,"total_tokens":500},
                "uuid":"u3"}"#,
        ];

        for json in cases {
            let output: ClaudeOutput = serde_json::from_str(json).unwrap();
            let value = serde_json::to_value(&output).unwrap();
            let reparsed: ClaudeOutput = serde_json::from_value(value).unwrap();

            let ClaudeOutput::System(sys) = reparsed else {
                panic!("Expected System variant after round-trip");
            };

            match sys.subtype {
                super::SystemSubtype::TaskStarted => {
                    assert!(
                        sys.as_task_started().is_some(),
                        "as_task_started failed after round-trip"
                    );
                }
                super::SystemSubtype::TaskProgress => {
                    assert!(
                        sys.as_task_progress().is_some(),
                        "as_task_progress failed after round-trip"
                    );
                }
                super::SystemSubtype::TaskNotification => {
                    assert!(
                        sys.as_task_notification().is_some(),
                        "as_task_notification failed after round-trip"
                    );
                }
                other => panic!("unexpected subtype after round-trip: {other:?}"),
            }
        }
    }

    #[test]
    fn test_system_message_compact_boundary() {
        let json = r#"{
            "type": "system",
            "subtype": "compact_boundary",
            "session_id": "879c1a88-3756-4092-aa95-0020c4ed9692",
            "compact_metadata": {
                "pre_tokens": 155285,
                "trigger": "auto"
            },
            "uuid": "a67780d5-74cb-48b1-9137-7a6e7cee45d7"
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::System(sys) = output {
            assert!(sys.is_compact_boundary());
            assert!(!sys.is_init());
            assert!(!sys.is_status());

            let compact = sys
                .as_compact_boundary()
                .expect("Should parse as compact_boundary");
            assert_eq!(compact.session_id, "879c1a88-3756-4092-aa95-0020c4ed9692");
            assert_eq!(compact.compact_metadata.pre_tokens, 155285);
            assert_eq!(
                compact.compact_metadata.trigger,
                super::CompactionTrigger::Auto
            );
            // Per-compaction stats are optional and absent here.
            assert!(compact.summary.is_none());
            assert!(compact.leaf_message_count.is_none());
            assert!(compact.duration_ms.is_none());
        } else {
            panic!("Expected System message");
        }
    }

    #[test]
    fn test_compact_boundary_with_summary_stats() {
        // Canonical keys.
        let json = r#"{
            "type": "system",
            "subtype": "compact_boundary",
            "session_id": "s1",
            "compact_metadata": { "pre_tokens": 1000, "trigger": "manual" },
            "summary": "Summarized the earlier exploration.",
            "leaf_message_count": 42,
            "duration_ms": 1234,
            "uuid": "u1"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let ClaudeOutput::System(sys) = output else {
            panic!("Expected System message");
        };
        let compact = sys.as_compact_boundary().expect("compact_boundary");
        assert_eq!(
            compact.summary.as_deref(),
            Some("Summarized the earlier exploration.")
        );
        assert_eq!(compact.leaf_message_count, Some(42));
        assert_eq!(compact.duration_ms, Some(1234));

        // Alternate wire keys (`content` for summary, `message_count` for count)
        // deserialize into the same fields.
        let json_alt = r#"{
            "type": "system",
            "subtype": "compact_boundary",
            "session_id": "s2",
            "compact_metadata": { "pre_tokens": 2000, "trigger": "auto" },
            "content": "alt-key summary",
            "message_count": 7
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json_alt).unwrap();
        let ClaudeOutput::System(sys) = output else {
            panic!("Expected System message");
        };
        let compact = sys.as_compact_boundary().expect("compact_boundary");
        assert_eq!(compact.summary.as_deref(), Some("alt-key summary"));
        assert_eq!(compact.leaf_message_count, Some(7));
    }

    #[test]
    fn test_init_message_with_new_fields() {
        let json = r#"{
            "type": "system",
            "subtype": "init",
            "session_id": "test-session",
            "cwd": "/home/user",
            "model": "claude-opus-4-7",
            "tools": ["Bash"],
            "mcp_servers": [],
            "permissionMode": "default",
            "apiKeySource": "none",
            "uuid": "44841a0d-182d-493a-86b5-79800d3d9665",
            "memory_paths": {"auto": "/home/user/.claude/projects/memory/"},
            "fast_mode_state": "off",
            "plugins": [{"name": "lsp", "path": "/plugins/lsp", "source": "lsp@official"}],
            "claude_code_version": "2.1.117"
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::System(sys) = output {
            let init = sys.as_init().expect("Should parse as init");
            assert_eq!(
                init.uuid.as_deref(),
                Some("44841a0d-182d-493a-86b5-79800d3d9665")
            );
            assert!(init.memory_paths.is_some());
            assert_eq!(init.fast_mode_state.as_deref(), Some("off"));
            assert_eq!(init.plugins[0].source.as_deref(), Some("lsp@official"));
            assert_eq!(init.claude_code_version.as_deref(), Some("2.1.117"));
        } else {
            panic!("Expected System message");
        }
    }

    #[test]
    fn test_assistant_message_with_new_fields() {
        let json = r#"{
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "type": "message",
                "role": "assistant",
                "model": "claude-opus-4-7",
                "content": [{"type": "text", "text": "Hello"}],
                "stop_reason": "end_turn",
                "stop_details": null,
                "context_management": null,
                "usage": {
                    "input_tokens": 100,
                    "output_tokens": 10,
                    "cache_creation_input_tokens": 50,
                    "cache_read_input_tokens": 0,
                    "service_tier": "standard",
                    "inference_geo": "not_available"
                }
            },
            "session_id": "abc",
            "uuid": "msg-uuid-123"
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::Assistant(asst) = output {
            assert_eq!(asst.message.stop_details, None);
            assert_eq!(asst.message.context_management, None);
            let usage = asst.message.usage.unwrap();
            assert_eq!(usage.inference_geo.as_deref(), Some("not_available"));
        } else {
            panic!("Expected Assistant message");
        }
    }

    #[test]
    fn test_user_message_with_new_fields() {
        let json = r#"{
            "type": "user",
            "message": {
                "role": "user",
                "content": [{"type": "text", "text": "Hello"}]
            },
            "session_id": "9abbc466-dad0-4b8e-b6b0-cad5eb7a16b9",
            "parent_tool_use_id": "toolu_123",
            "uuid": "user-msg-456"
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::User(user) = output {
            assert_eq!(user.parent_tool_use_id.as_deref(), Some("toolu_123"));
            assert_eq!(user.uuid.as_deref(), Some("user-msg-456"));
        } else {
            panic!("Expected User message");
        }
    }

    /// Real wire payload captured from the CLI after answering an
    /// AskUserQuestion via the permission control protocol. The top-level
    /// `tool_use_result` and `timestamp` fields must round-trip without loss —
    /// proxies using this crate to relay messages to a viewer rely on those
    /// fields being preserved (the viewer reads `tool_use_result.answers`).
    #[test]
    fn test_user_message_preserves_tool_use_result_and_timestamp() {
        let json = r#"{
            "type":"user",
            "message":{"role":"user","content":[{"type":"tool_result","content":"User has answered your questions: . You can now continue with the user's answers in mind.","tool_use_id":"toolu_01331duMqP2PrRaqR2yWa8e4"}]},
            "parent_tool_use_id":null,
            "session_id":"622ae0c3-3d50-4fa7-9ee0-69d691238c6d",
            "uuid":"8ef6e997-a849-4d15-bed3-2837c3d3f4cd",
            "timestamp":"2026-05-12T23:12:04.121Z",
            "tool_use_result":{"questions":[{"question":"Which color do you prefer?","header":"Color","options":[{"label":"Red","description":"A warm color"},{"label":"Blue","description":"A cool color"}],"multiSelect":false}],"answers":{"Color":"Blue"}}
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let user = match output {
            ClaudeOutput::User(u) => u,
            other => panic!("Expected User message, got {:?}", other.message_type()),
        };

        assert_eq!(user.timestamp.as_deref(), Some("2026-05-12T23:12:04.121Z"));
        let raw = user
            .tool_use_result
            .as_ref()
            .expect("tool_use_result must be captured");
        assert_eq!(raw["answers"]["Color"], "Blue");
        assert_eq!(raw["questions"][0]["header"], "Color");

        // Round-trip: re-serialize and confirm tool_use_result + timestamp
        // survive — the bug we're guarding against is that the proxy silently
        // drops these fields when relaying user messages.
        let reser: serde_json::Value = serde_json::to_value(&user).unwrap();
        assert_eq!(reser["timestamp"], "2026-05-12T23:12:04.121Z");
        assert_eq!(reser["tool_use_result"]["answers"]["Color"], "Blue");
        assert_eq!(
            reser["tool_use_result"]["questions"][0]["question"],
            "Which color do you prefer?"
        );

        // Typed accessor: AskUserQuestionInput has the same shape as the
        // AskUserQuestion tool_use_result.
        let typed: crate::AskUserQuestionInput = user
            .tool_use_result_as::<crate::AskUserQuestionInput>()
            .expect("tool_use_result present")
            .expect("AskUserQuestionInput parses");
        assert_eq!(typed.questions.len(), 1);
        assert_eq!(typed.questions[0].header, "Color");
        let answers = typed.answers.expect("answers populated");
        assert_eq!(answers.get("Color").map(String::as_str), Some("Blue"));
    }

    /// User messages without `tool_use_result` / `timestamp` must still
    /// deserialize fine and serialize back without spuriously emitting nulls.
    #[test]
    fn test_user_message_without_tool_use_result_omits_field() {
        let json = r#"{
            "type":"user",
            "message":{"role":"user","content":[{"type":"text","text":"hello"}]},
            "session_id":"622ae0c3-3d50-4fa7-9ee0-69d691238c6d"
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let user = match output {
            ClaudeOutput::User(u) => u,
            _ => panic!("Expected User message"),
        };
        assert!(user.tool_use_result.is_none());
        assert!(user.timestamp.is_none());

        let reser = serde_json::to_value(&user).unwrap();
        assert!(reser.get("tool_use_result").is_none());
        assert!(reser.get("timestamp").is_none());
    }

    /// A `Task` tool result must expose subagent token / timing / tool-use
    /// accounting through the typed [`UserMessage::subagent_result`] accessor,
    /// including the nested per-model `usage` breakdown and `toolStats`.
    #[test]
    fn test_subagent_result_exposes_token_accounting() {
        let json = r#"{
            "type":"user",
            "message":{"role":"user","content":[{"tool_use_id":"toolu_01","type":"tool_result","content":[{"type":"text","text":"21"}]}]},
            "session_id":"d3fc5942-75e5-4aa1-a87d-b9484a176541",
            "tool_use_result":{
                "status":"completed",
                "prompt":"Count the .rs files.",
                "agentId":"ac4f0276e9d4b6232",
                "agentType":"Explore",
                "content":[{"type":"text","text":"21"}],
                "resolvedModel":"claude-haiku-4-5-20251001",
                "totalDurationMs":6869,
                "totalTokens":7834,
                "totalToolUseCount":1,
                "usage":{"input_tokens":6,"cache_creation_input_tokens":125,"cache_read_input_tokens":7699,"output_tokens":4,"service_tier":"standard"},
                "toolStats":{"readCount":0,"searchCount":0,"bashCount":1,"editFileCount":0,"linesAdded":0,"linesRemoved":0,"otherToolCount":0}
            }
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let user = match output {
            ClaudeOutput::User(u) => u,
            _ => panic!("Expected User message"),
        };

        let result = user.subagent_result().expect("subagent result parses");
        assert_eq!(result.agent_type.as_deref(), Some("Explore"));
        assert_eq!(
            result.resolved_model.as_deref(),
            Some("claude-haiku-4-5-20251001")
        );
        assert_eq!(result.total_tokens, Some(7834));
        assert_eq!(result.total_duration_ms, Some(6869));
        assert_eq!(result.total_tool_use_count, Some(1));

        let usage = result.usage.expect("nested usage present");
        assert_eq!(usage.input_tokens, 6);
        assert_eq!(usage.cache_read_input_tokens, 7699);

        let stats = result.tool_stats.expect("toolStats present");
        assert_eq!(stats.bash_count, 1);
    }

    /// `tool_use_result` shapes that aren't subagent runs (e.g. AskUserQuestion)
    /// parse leniently into the all-`Option` [`SubagentResult`] with empty
    /// accounting rather than failing, so callers can probe without panicking.
    #[test]
    fn test_subagent_result_absent_for_non_task_result() {
        let json = r#"{
            "type":"user",
            "message":{"role":"user","content":[{"type":"text","text":"hi"}]},
            "session_id":"622ae0c3-3d50-4fa7-9ee0-69d691238c6d",
            "tool_use_result":{"questions":[],"answers":{"Color":"Blue"}}
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let user = match output {
            ClaudeOutput::User(u) => u,
            _ => panic!("Expected User message"),
        };

        let result = user.subagent_result().expect("lenient parse");
        assert_eq!(result.total_tokens, None);
        assert_eq!(result.agent_type, None);
    }

    #[test]
    fn test_init_fast_mode_reason_and_mcp_server_errors_fully_wrapped() {
        use serde_json::Value;

        let raw: Value = serde_json::from_str(
            r#"{
            "type":"system","subtype":"init","session_id":"s1","uuid":"u1",
            "fast_mode_state":"off",
            "fast_mode_disabled_reason":"not_first_party",
            "mcp_server_errors":[{"name":"broken","type":"invalid_config","message":"url entry with no type"}]
        }"#,
        )
        .unwrap();
        crate::io::assert_fully_wrapped(&raw);

        let output: ClaudeOutput = serde_json::from_value(raw).unwrap();
        let ClaudeOutput::System(sys) = output else {
            panic!("expected System");
        };
        let init = sys.as_init().expect("parses as init");
        assert_eq!(
            init.fast_mode_disabled_reason,
            Some(crate::FastModeDisabledReason::NotFirstParty)
        );
        let errs = init.mcp_server_errors.unwrap();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].name, "broken");
        assert_eq!(errs[0].error_type, "invalid_config");
    }

    #[test]
    fn test_code_change_published_fully_wrapped() {
        use super::{KnownSystemEvent, SystemSubtype};
        use serde_json::Value;

        let raw: Value = serde_json::from_str(
            r#"{
            "type":"system","subtype":"code_change_published",
            "provider":"github","url":"https://github.com/owner/repo/pull/42",
            "repo":"owner/repo","identifier":"42",
            "uuid":"u1","session_id":"s1"
        }"#,
        )
        .unwrap();
        crate::io::assert_fully_wrapped(&raw);

        let output: ClaudeOutput = serde_json::from_value(raw).unwrap();
        let ClaudeOutput::System(sys) = output else {
            panic!("expected System");
        };
        assert_eq!(sys.subtype, SystemSubtype::CodeChangePublished);
        let Some(KnownSystemEvent::CodeChangePublished(msg)) = sys.as_known_system_event() else {
            panic!("expected CodeChangePublished event");
        };
        assert_eq!(msg.provider, "github");
        assert_eq!(msg.repo, "owner/repo");
        assert_eq!(msg.identifier, "42");
    }

    #[test]
    fn test_vcs_state_changed_fully_wrapped() {
        use super::{KnownSystemEvent, VcsMutationKind};
        use serde_json::Value;

        for kind in ["commit", "push", "merge", "rebase"] {
            let raw: Value = serde_json::from_str(&format!(
                r#"{{"type":"system","subtype":"vcs_state_changed","kind":"{}","cwd":"/repo","uuid":"u1","session_id":"s1"}}"#,
                kind
            ))
            .unwrap();
            crate::io::assert_fully_wrapped(&raw);

            let output: ClaudeOutput = serde_json::from_value(raw).unwrap();
            let ClaudeOutput::System(sys) = output else {
                panic!("expected System");
            };
            let Some(KnownSystemEvent::VcsStateChanged(msg)) = sys.as_known_system_event() else {
                panic!("expected VcsStateChanged event");
            };
            assert_eq!(msg.kind.as_str(), kind);
            assert!(!matches!(msg.kind, VcsMutationKind::Unknown(_)));
        }

        // Unknown kinds are valid per the wire contract.
        let raw: Value = serde_json::from_str(
            r#"{"type":"system","subtype":"vcs_state_changed","kind":"tag","cwd":"/repo","uuid":"u2","session_id":"s2"}"#,
        )
        .unwrap();
        crate::io::assert_fully_wrapped(&raw);
        let output: ClaudeOutput = serde_json::from_value(raw).unwrap();
        let ClaudeOutput::System(sys) = output else {
            panic!("expected System");
        };
        let Some(KnownSystemEvent::VcsStateChanged(msg)) = sys.as_known_system_event() else {
            panic!("expected VcsStateChanged event");
        };
        assert_eq!(msg.kind, VcsMutationKind::Unknown("tag".to_string()));
    }

    #[test]
    fn test_assistant_aborted_and_resume_flags_roundtrip() {
        let json = r#"{
            "type":"assistant",
            "message":{"id":"msg_1","role":"assistant","model":"claude-3","content":[{"type":"text","text":"partial"}]},
            "session_id":"s1",
            "aborted":true,
            "resumed_from_incomplete_thinking":true
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let ClaudeOutput::Assistant(msg) = &output else {
            panic!("expected Assistant");
        };
        assert_eq!(msg.aborted, Some(true));
        assert_eq!(msg.resumed_from_incomplete_thinking, Some(true));
        let reserialized = serde_json::to_string(&output).unwrap();
        assert!(reserialized.contains("\"aborted\":true"));
        assert!(reserialized.contains("\"resumed_from_incomplete_thinking\":true"));

        // Absent flags stay absent on the wire.
        let json = r#"{
            "type":"assistant",
            "message":{"id":"msg_2","role":"assistant","model":"claude-3","content":[]},
            "session_id":"s2"
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let reserialized = serde_json::to_string(&output).unwrap();
        assert!(!reserialized.contains("aborted"));
        assert!(!reserialized.contains("resumed_from_incomplete_thinking"));
    }

    #[test]
    fn test_user_tool_result_meta_roundtrip() {
        let json = r#"{
            "type":"user",
            "message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"denied"}]},
            "session_id":"622ae0c3-3d50-4fa7-9ee0-69d691238c6d",
            "tool_result_meta":[
                {"id":"toolu_1","non_execution_kind":"user-rejected","user_feedback":"use the staging db"},
                {"id":"toolu_2","non_execution_kind":"permission-rule"}
            ]
        }"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let ClaudeOutput::User(user) = &output else {
            panic!("expected User");
        };
        let meta = user.tool_result_meta.as_ref().unwrap();
        assert_eq!(meta.len(), 2);
        assert_eq!(meta[0].non_execution_kind, "user-rejected");
        assert_eq!(meta[0].user_feedback.as_deref(), Some("use the staging db"));
        assert_eq!(meta[1].user_feedback, None);

        let reserialized = serde_json::to_string(&output).unwrap();
        assert!(reserialized.contains("\"non_execution_kind\":\"user-rejected\""));
        assert!(!reserialized.contains("\"user_feedback\":null"));
    }
}
