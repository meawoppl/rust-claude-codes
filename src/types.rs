//! Core types used in the Claude Code protocol

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a unique identifier for various entities
pub type Id = String;

/// Represents a session identifier
pub type SessionId = String;

/// Represents a task identifier
pub type TaskId = String;

/// Represents a conversation identifier
pub type ConversationId = String;

/// Status of a task or operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Priority levels for tasks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

/// Tool types available in Claude Code
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    Bash,
    Read,
    Write,
    Edit,
    Search,
    #[serde(rename = "web_search")]
    WebSearch,
    Other(String),
}

/// Represents metadata for a message or operation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Metadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Represents an error detail in responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Represents a capability or feature
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Capability {
    pub name: String,
    pub enabled: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}
