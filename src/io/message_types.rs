use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use uuid::Uuid;

use super::content_blocks::{deserialize_content_blocks, ContentBlock};

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
    pub role: String,
    #[serde(deserialize_with = "deserialize_content_blocks")]
    pub content: Vec<ContentBlock>,
}

/// System message with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMessage {
    pub subtype: String,
    #[serde(flatten)]
    pub data: Value, // Captures all other fields
}

impl SystemMessage {
    /// Check if this is an init message
    pub fn is_init(&self) -> bool {
        self.subtype == "init"
    }

    /// Check if this is a status message
    pub fn is_status(&self) -> bool {
        self.subtype == "status"
    }

    /// Check if this is a compact_boundary message
    pub fn is_compact_boundary(&self) -> bool {
        self.subtype == "compact_boundary"
    }

    /// Try to parse as an init message
    pub fn as_init(&self) -> Option<InitMessage> {
        if self.subtype != "init" {
            return None;
        }
        serde_json::from_value(self.data.clone()).ok()
    }

    /// Try to parse as a status message
    pub fn as_status(&self) -> Option<StatusMessage> {
        if self.subtype != "status" {
            return None;
        }
        serde_json::from_value(self.data.clone()).ok()
    }

    /// Try to parse as a compact_boundary message
    pub fn as_compact_boundary(&self) -> Option<CompactBoundaryMessage> {
        if self.subtype != "compact_boundary" {
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
    /// How the API key was sourced (e.g., "none")
    #[serde(skip_serializing_if = "Option::is_none", rename = "apiKeySource")]
    pub api_key_source: Option<String>,
    /// Output style (e.g., "default")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_style: Option<String>,
    /// Permission mode (e.g., "default")
    #[serde(skip_serializing_if = "Option::is_none", rename = "permissionMode")]
    pub permission_mode: Option<String>,
}

/// Status system message - sent during operations like context compaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusMessage {
    /// Session identifier
    pub session_id: String,
    /// Current status (e.g., "compacting") or null when complete
    pub status: Option<String>,
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
    /// What triggered the compaction ("auto" or "manual")
    pub trigger: String,
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
    pub role: String,
    pub model: String,
    pub content: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
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
            assert_eq!(init.api_key_source, Some("none".to_string()));
            assert_eq!(init.output_style, Some("default".to_string()));
            assert_eq!(init.permission_mode, Some("default".to_string()));
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
            assert_eq!(status.status, Some("compacting".to_string()));
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
            assert_eq!(compact.compact_metadata.trigger, "auto");
        } else {
            panic!("Expected System message");
        }
    }
}
