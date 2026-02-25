use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::fmt;
use uuid::Uuid;

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
    TaskStarted,
    TaskProgress,
    TaskNotification,
    /// A subtype not yet known to this version of the crate.
    Unknown(String),
}

impl SystemSubtype {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Init => "init",
            Self::Status => "status",
            Self::CompactBoundary => "compact_boundary",
            Self::TaskStarted => "task_started",
            Self::TaskProgress => "task_progress",
            Self::TaskNotification => "task_notification",
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
            "task_started" => Self::TaskStarted,
            "task_progress" => Self::TaskProgress,
            "task_notification" => Self::TaskNotification,
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
    /// A source not yet known to this version of the crate.
    Unknown(String),
}

impl ApiKeySource {
    pub fn as_str(&self) -> &str {
        match self {
            Self::None => "none",
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
    /// A mode not yet known to this version of the crate.
    Unknown(String),
}

impl InitPermissionMode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Default => "default",
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
    /// A status not yet known to this version of the crate.
    Unknown(String),
}

impl StatusMessageStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Compacting => "compacting",
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

/// User message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub message: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(
        serialize_with = "serialize_optional_uuid",
        deserialize_with = "deserialize_optional_uuid"
    )]
    pub session_id: Option<Uuid>,
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
}

/// Plugin info from the init message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Plugin name
    pub name: String,
    /// Path to the plugin on disk
    pub path: String,
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
}

/// Compact boundary message - marks where context compaction occurred
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactBoundaryMessage {
    /// Session identifier
    pub session_id: String,
    /// Metadata about the compaction
    pub compact_metadata: CompactMetadata,
    /// Unique identifier for this message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

/// Metadata about context compaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactMetadata {
    /// Number of tokens before compaction
    pub pre_tokens: u64,
    /// What triggered the compaction
    pub trigger: CompactionTrigger,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    /// A sub-agent task (e.g., Explore, Plan).
    LocalAgent,
    /// A background bash command.
    LocalBash,
}

/// Completion status of a background task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Completed,
    Failed,
}

/// `task_started` system message — emitted once when a background task begins.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStartedMessage {
    pub session_id: String,
    pub task_id: String,
    pub task_type: TaskType,
    pub tool_use_id: String,
    pub description: String,
    pub uuid: String,
}

/// `task_progress` system message — emitted periodically as a background
/// agent task executes tools. Not emitted for `local_bash` tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgressMessage {
    pub session_id: String,
    pub task_id: String,
    pub tool_use_id: String,
    pub description: String,
    pub last_tool_name: String,
    pub usage: TaskUsage,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

/// Assistant message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub message: AssistantMessageContent,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_tool_use_id: Option<String>,
}

/// Nested message content for assistant messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessageContent {
    pub id: String,
    pub role: MessageRole,
    pub model: String,
    pub content: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<StopReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<AssistantUsage>,
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
            assert_eq!(task.task_type, super::TaskType::LocalBash);
            assert_eq!(task.tool_use_id, "toolu_011rfSTFumpJZdCCfzeD7jaS");
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
            assert_eq!(task.task_type, super::TaskType::LocalAgent);
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
            assert_eq!(progress.last_tool_name, "Read");
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
        } else {
            panic!("Expected System message");
        }
    }
}
