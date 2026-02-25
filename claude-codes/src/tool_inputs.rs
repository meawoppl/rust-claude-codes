//! Typed tool input definitions for Claude Code tools.
//!
//! This module provides strongly-typed structs for the input parameters of each
//! Claude Code tool. Using these types instead of raw `serde_json::Value` provides:
//!
//! - Compile-time type checking
//! - IDE autocompletion and documentation
//! - Self-documenting API
//!
//! # Example
//!
//! ```
//! use claude_codes::{ToolInput, BashInput};
//!
//! // Parse a tool input from JSON
//! let json = serde_json::json!({
//!     "command": "ls -la",
//!     "description": "List files in current directory"
//! });
//!
//! let input: ToolInput = serde_json::from_value(json).unwrap();
//! if let ToolInput::Bash(bash) = input {
//!     assert_eq!(bash.command, "ls -la");
//! }
//! ```

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Individual Tool Input Structs
// ============================================================================

/// Input for the Bash tool - executes shell commands.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BashInput {
    /// The bash command to execute (required)
    pub command: String,

    /// Human-readable description of what the command does
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Timeout in milliseconds (max 600000)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,

    /// Whether to run the command in the background
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_in_background: Option<bool>,
}

/// Input for the Read tool - reads file contents.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReadInput {
    /// The absolute path to the file to read
    pub file_path: String,

    /// The line number to start reading from (1-indexed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,

    /// The number of lines to read
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
}

/// Input for the Write tool - writes content to a file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WriteInput {
    /// The absolute path to the file to write
    pub file_path: String,

    /// The content to write to the file
    pub content: String,
}

/// Input for the Edit tool - performs string replacements in files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditInput {
    /// The absolute path to the file to modify
    pub file_path: String,

    /// The text to replace
    pub old_string: String,

    /// The text to replace it with
    pub new_string: String,

    /// Replace all occurrences of old_string (default false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace_all: Option<bool>,
}

/// Input for the Glob tool - finds files matching a pattern.
///
/// The `deny_unknown_fields` attribute ensures Glob only matches exact
/// Glob inputs and doesn't accidentally match Grep inputs (which share
/// the `pattern` field but have additional Grep-specific fields).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GlobInput {
    /// The glob pattern to match files against (e.g., "**/*.rs")
    pub pattern: String,

    /// The directory to search in (defaults to current working directory)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Input for the Grep tool - searches file contents.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GrepInput {
    /// The regular expression pattern to search for
    pub pattern: String,

    /// File or directory to search in
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Glob pattern to filter files (e.g., "*.js")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glob: Option<String>,

    /// File type to search (e.g., "js", "py", "rust")
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub file_type: Option<String>,

    /// Case insensitive search
    #[serde(rename = "-i", skip_serializing_if = "Option::is_none")]
    pub case_insensitive: Option<bool>,

    /// Show line numbers in output
    #[serde(rename = "-n", skip_serializing_if = "Option::is_none")]
    pub line_numbers: Option<bool>,

    /// Number of lines to show after each match
    #[serde(rename = "-A", skip_serializing_if = "Option::is_none")]
    pub after_context: Option<u32>,

    /// Number of lines to show before each match
    #[serde(rename = "-B", skip_serializing_if = "Option::is_none")]
    pub before_context: Option<u32>,

    /// Number of lines to show before and after each match
    #[serde(rename = "-C", skip_serializing_if = "Option::is_none")]
    pub context: Option<u32>,

    /// Output mode: content, files_with_matches, or count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_mode: Option<GrepOutputMode>,

    /// Enable multiline mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiline: Option<bool>,

    /// Limit output to first N lines/entries
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_limit: Option<u32>,

    /// Skip first N lines/entries
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
}

/// Input for the Task tool - launches subagents.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskInput {
    /// A short (3-5 word) description of the task
    pub description: String,

    /// The task for the agent to perform
    pub prompt: String,

    /// The type of specialized agent to use
    pub subagent_type: SubagentType,

    /// Whether to run the agent in the background
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_in_background: Option<bool>,

    /// Optional model to use for this agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Maximum number of agentic turns
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,

    /// Optional agent ID to resume from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume: Option<String>,
}

/// Input for the WebFetch tool - fetches and processes web content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebFetchInput {
    /// The URL to fetch content from
    pub url: String,

    /// The prompt to run on the fetched content
    pub prompt: String,
}

/// Input for the WebSearch tool - searches the web.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebSearchInput {
    /// The search query to use
    pub query: String,

    /// Only include search results from these domains
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_domains: Option<Vec<String>>,

    /// Never include search results from these domains
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_domains: Option<Vec<String>>,
}

/// Input for the TodoWrite tool - manages task lists.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TodoWriteInput {
    /// The updated todo list
    pub todos: Vec<TodoItem>,
}

/// Status of a todo item.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
    /// A status not yet known to this version of the crate.
    Unknown(String),
}

impl TodoStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for TodoStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for TodoStatus {
    fn from(s: &str) -> Self {
        match s {
            "pending" => Self::Pending,
            "in_progress" => Self::InProgress,
            "completed" => Self::Completed,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for TodoStatus {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for TodoStatus {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// Output mode for the Grep tool.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GrepOutputMode {
    /// Show matching lines with context.
    Content,
    /// Show only file paths containing matches.
    FilesWithMatches,
    /// Show match counts per file.
    Count,
    /// A mode not yet known to this version of the crate.
    Unknown(String),
}

impl GrepOutputMode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Content => "content",
            Self::FilesWithMatches => "files_with_matches",
            Self::Count => "count",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for GrepOutputMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for GrepOutputMode {
    fn from(s: &str) -> Self {
        match s {
            "content" => Self::Content,
            "files_with_matches" => Self::FilesWithMatches,
            "count" => Self::Count,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for GrepOutputMode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for GrepOutputMode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// Type of specialized subagent for the Task tool.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubagentType {
    /// Command execution specialist.
    Bash,
    /// Fast codebase exploration agent.
    Explore,
    /// Software architect agent for planning.
    Plan,
    /// General-purpose agent.
    GeneralPurpose,
    /// A subagent type not yet known to this version of the crate.
    Unknown(String),
}

impl SubagentType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Bash => "Bash",
            Self::Explore => "Explore",
            Self::Plan => "Plan",
            Self::GeneralPurpose => "general-purpose",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for SubagentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for SubagentType {
    fn from(s: &str) -> Self {
        match s {
            "Bash" => Self::Bash,
            "Explore" => Self::Explore,
            "Plan" => Self::Plan,
            "general-purpose" => Self::GeneralPurpose,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for SubagentType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for SubagentType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// Type of Jupyter notebook cell.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NotebookCellType {
    /// Code cell.
    Code,
    /// Markdown cell.
    Markdown,
    /// A cell type not yet known to this version of the crate.
    Unknown(String),
}

impl NotebookCellType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Code => "code",
            Self::Markdown => "markdown",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for NotebookCellType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for NotebookCellType {
    fn from(s: &str) -> Self {
        match s {
            "code" => Self::Code,
            "markdown" => Self::Markdown,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for NotebookCellType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for NotebookCellType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// Type of edit to perform on a notebook cell.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NotebookEditMode {
    /// Replace the cell's content.
    Replace,
    /// Insert a new cell.
    Insert,
    /// Delete the cell.
    Delete,
    /// An edit mode not yet known to this version of the crate.
    Unknown(String),
}

impl NotebookEditMode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Replace => "replace",
            Self::Insert => "insert",
            Self::Delete => "delete",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for NotebookEditMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for NotebookEditMode {
    fn from(s: &str) -> Self {
        match s {
            "replace" => Self::Replace,
            "insert" => Self::Insert,
            "delete" => Self::Delete,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for NotebookEditMode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for NotebookEditMode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// A single todo item in a task list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TodoItem {
    /// The task description (imperative form)
    pub content: String,

    /// Current status
    pub status: TodoStatus,

    /// The present continuous form shown during execution
    #[serde(rename = "activeForm")]
    pub active_form: String,
}

/// Input for the AskUserQuestion tool - asks the user questions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AskUserQuestionInput {
    /// Questions to ask the user (1-4 questions)
    pub questions: Vec<Question>,

    /// User answers collected by the permission component
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answers: Option<HashMap<String, String>>,

    /// Optional metadata for tracking and analytics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<QuestionMetadata>,
}

/// A question to ask the user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Question {
    /// The complete question to ask the user
    pub question: String,

    /// Very short label displayed as a chip/tag (max 12 chars)
    pub header: String,

    /// The available choices for this question (2-4 options)
    pub options: Vec<QuestionOption>,

    /// Whether multiple options can be selected
    #[serde(rename = "multiSelect", default)]
    pub multi_select: bool,
}

/// An option for a question.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuestionOption {
    /// The display text for this option
    pub label: String,

    /// Explanation of what this option means
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Metadata for questions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuestionMetadata {
    /// Optional identifier for the source of this question
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Input for the NotebookEdit tool - edits Jupyter notebooks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NotebookEditInput {
    /// The absolute path to the Jupyter notebook file
    pub notebook_path: String,

    /// The new source for the cell
    pub new_source: String,

    /// The ID of the cell to edit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cell_id: Option<String>,

    /// The type of the cell (code or markdown)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cell_type: Option<NotebookCellType>,

    /// The type of edit to make (replace, insert, delete)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edit_mode: Option<NotebookEditMode>,
}

/// Input for the TaskOutput tool - retrieves output from background tasks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskOutputInput {
    /// The task ID to get output from
    pub task_id: String,

    /// Whether to wait for completion (default true)
    #[serde(default = "default_true")]
    pub block: bool,

    /// Max wait time in ms (default 30000, max 600000)
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    30000
}

/// Input for the KillShell tool - kills a running background shell.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KillShellInput {
    /// The ID of the background shell to kill
    pub shell_id: String,
}

/// Input for the Skill tool - executes a skill.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillInput {
    /// The skill name (e.g., "commit", "review-pr")
    pub skill: String,

    /// Optional arguments for the skill
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
}

/// Input for the EnterPlanMode tool - enters planning mode.
///
/// This is an empty struct as EnterPlanMode takes no parameters.
/// The `deny_unknown_fields` attribute ensures this only matches
/// empty JSON objects `{}`, not arbitrary JSON.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct EnterPlanModeInput {}

/// Input for the ExitPlanMode tool - exits planning mode.
///
/// The `deny_unknown_fields` attribute ensures this only matches JSON objects
/// that contain known fields (or are empty), not arbitrary JSON.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct ExitPlanModeInput {
    /// Prompt-based permissions needed to implement the plan
    #[serde(rename = "allowedPrompts", skip_serializing_if = "Option::is_none")]
    pub allowed_prompts: Option<Vec<AllowedPrompt>>,

    /// Whether to push the plan to a remote Claude.ai session
    #[serde(rename = "pushToRemote", skip_serializing_if = "Option::is_none")]
    pub push_to_remote: Option<bool>,

    /// The remote session ID if pushed to remote
    #[serde(rename = "remoteSessionId", skip_serializing_if = "Option::is_none")]
    pub remote_session_id: Option<String>,

    /// The remote session URL if pushed to remote
    #[serde(rename = "remoteSessionUrl", skip_serializing_if = "Option::is_none")]
    pub remote_session_url: Option<String>,

    /// The remote session title if pushed to remote
    #[serde(rename = "remoteSessionTitle", skip_serializing_if = "Option::is_none")]
    pub remote_session_title: Option<String>,

    /// The plan content from plan mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
}

/// An allowed prompt permission for plan mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AllowedPrompt {
    /// The tool this prompt applies to
    pub tool: String,

    /// Semantic description of the action
    pub prompt: String,
}

// ============================================================================
// ToolInput Enum - Unified type for all tool inputs
// ============================================================================

/// Unified enum representing input for any Claude Code tool.
///
/// This enum uses `#[serde(untagged)]` to automatically deserialize based on
/// the structure of the JSON. The `Unknown` variant serves as a fallback for:
/// - New tools added in future Claude CLI versions
/// - Custom MCP tools provided by users
/// - Any tool input that doesn't match known schemas
///
/// # Example
///
/// ```
/// use claude_codes::ToolInput;
///
/// // Known tool - deserializes to specific variant
/// let bash_json = serde_json::json!({"command": "ls"});
/// let input: ToolInput = serde_json::from_value(bash_json).unwrap();
/// assert!(matches!(input, ToolInput::Bash(_)));
///
/// // Unknown tool - falls back to Unknown variant
/// let custom_json = serde_json::json!({"custom_field": "value"});
/// let input: ToolInput = serde_json::from_value(custom_json).unwrap();
/// assert!(matches!(input, ToolInput::Unknown(_)));
/// ```
///
/// # Note on Ordering
///
/// The variants are ordered from most specific (most required fields) to least
/// specific to ensure correct deserialization with `#[serde(untagged)]`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ToolInput {
    /// Edit tool - has unique field combination (file_path, old_string, new_string)
    Edit(EditInput),

    /// Write tool - file_path + content
    Write(WriteInput),

    /// AskUserQuestion tool - has questions array
    AskUserQuestion(AskUserQuestionInput),

    /// TodoWrite tool - has todos array
    TodoWrite(TodoWriteInput),

    /// Task tool - description + prompt + subagent_type
    Task(TaskInput),

    /// NotebookEdit tool - notebook_path + new_source
    NotebookEdit(NotebookEditInput),

    /// WebFetch tool - url + prompt
    WebFetch(WebFetchInput),

    /// TaskOutput tool - task_id + block + timeout
    TaskOutput(TaskOutputInput),

    /// Bash tool - has command field
    Bash(BashInput),

    /// Read tool - has file_path
    Read(ReadInput),

    /// Glob tool - has pattern field (with deny_unknown_fields, must come before Grep)
    Glob(GlobInput),

    /// Grep tool - has pattern field plus many optional fields
    Grep(GrepInput),

    /// WebSearch tool - has query field
    WebSearch(WebSearchInput),

    /// KillShell tool - has shell_id
    KillShell(KillShellInput),

    /// Skill tool - has skill field
    Skill(SkillInput),

    /// ExitPlanMode tool
    ExitPlanMode(ExitPlanModeInput),

    /// EnterPlanMode tool (empty input)
    EnterPlanMode(EnterPlanModeInput),

    /// Unknown tool input - fallback for custom/new tools
    ///
    /// This variant captures any tool input that doesn't match the known schemas.
    /// Use this for:
    /// - MCP tools provided by users
    /// - New tools in future Claude CLI versions
    /// - Any custom tool integration
    Unknown(Value),
}

impl ToolInput {
    /// Returns the tool name if it can be determined from the input type.
    ///
    /// For `Unknown` variants, returns `None` since the tool name cannot be
    /// determined from the input structure alone.
    pub fn tool_name(&self) -> Option<&'static str> {
        match self {
            ToolInput::Bash(_) => Some("Bash"),
            ToolInput::Read(_) => Some("Read"),
            ToolInput::Write(_) => Some("Write"),
            ToolInput::Edit(_) => Some("Edit"),
            ToolInput::Glob(_) => Some("Glob"),
            ToolInput::Grep(_) => Some("Grep"),
            ToolInput::Task(_) => Some("Task"),
            ToolInput::WebFetch(_) => Some("WebFetch"),
            ToolInput::WebSearch(_) => Some("WebSearch"),
            ToolInput::TodoWrite(_) => Some("TodoWrite"),
            ToolInput::AskUserQuestion(_) => Some("AskUserQuestion"),
            ToolInput::NotebookEdit(_) => Some("NotebookEdit"),
            ToolInput::TaskOutput(_) => Some("TaskOutput"),
            ToolInput::KillShell(_) => Some("KillShell"),
            ToolInput::Skill(_) => Some("Skill"),
            ToolInput::EnterPlanMode(_) => Some("EnterPlanMode"),
            ToolInput::ExitPlanMode(_) => Some("ExitPlanMode"),
            ToolInput::Unknown(_) => None,
        }
    }

    /// Try to get the input as a Bash input.
    pub fn as_bash(&self) -> Option<&BashInput> {
        match self {
            ToolInput::Bash(input) => Some(input),
            _ => None,
        }
    }

    /// Try to get the input as a Read input.
    pub fn as_read(&self) -> Option<&ReadInput> {
        match self {
            ToolInput::Read(input) => Some(input),
            _ => None,
        }
    }

    /// Try to get the input as a Write input.
    pub fn as_write(&self) -> Option<&WriteInput> {
        match self {
            ToolInput::Write(input) => Some(input),
            _ => None,
        }
    }

    /// Try to get the input as an Edit input.
    pub fn as_edit(&self) -> Option<&EditInput> {
        match self {
            ToolInput::Edit(input) => Some(input),
            _ => None,
        }
    }

    /// Try to get the input as a Glob input.
    pub fn as_glob(&self) -> Option<&GlobInput> {
        match self {
            ToolInput::Glob(input) => Some(input),
            _ => None,
        }
    }

    /// Try to get the input as a Grep input.
    pub fn as_grep(&self) -> Option<&GrepInput> {
        match self {
            ToolInput::Grep(input) => Some(input),
            _ => None,
        }
    }

    /// Try to get the input as a Task input.
    pub fn as_task(&self) -> Option<&TaskInput> {
        match self {
            ToolInput::Task(input) => Some(input),
            _ => None,
        }
    }

    /// Try to get the input as a WebFetch input.
    pub fn as_web_fetch(&self) -> Option<&WebFetchInput> {
        match self {
            ToolInput::WebFetch(input) => Some(input),
            _ => None,
        }
    }

    /// Try to get the input as a WebSearch input.
    pub fn as_web_search(&self) -> Option<&WebSearchInput> {
        match self {
            ToolInput::WebSearch(input) => Some(input),
            _ => None,
        }
    }

    /// Try to get the input as a TodoWrite input.
    pub fn as_todo_write(&self) -> Option<&TodoWriteInput> {
        match self {
            ToolInput::TodoWrite(input) => Some(input),
            _ => None,
        }
    }

    /// Try to get the input as an AskUserQuestion input.
    pub fn as_ask_user_question(&self) -> Option<&AskUserQuestionInput> {
        match self {
            ToolInput::AskUserQuestion(input) => Some(input),
            _ => None,
        }
    }

    /// Try to get the input as a NotebookEdit input.
    pub fn as_notebook_edit(&self) -> Option<&NotebookEditInput> {
        match self {
            ToolInput::NotebookEdit(input) => Some(input),
            _ => None,
        }
    }

    /// Try to get the input as a TaskOutput input.
    pub fn as_task_output(&self) -> Option<&TaskOutputInput> {
        match self {
            ToolInput::TaskOutput(input) => Some(input),
            _ => None,
        }
    }

    /// Try to get the input as a KillShell input.
    pub fn as_kill_shell(&self) -> Option<&KillShellInput> {
        match self {
            ToolInput::KillShell(input) => Some(input),
            _ => None,
        }
    }

    /// Try to get the input as a Skill input.
    pub fn as_skill(&self) -> Option<&SkillInput> {
        match self {
            ToolInput::Skill(input) => Some(input),
            _ => None,
        }
    }

    /// Try to get the input as an unknown Value.
    pub fn as_unknown(&self) -> Option<&Value> {
        match self {
            ToolInput::Unknown(value) => Some(value),
            _ => None,
        }
    }

    /// Check if this is an unknown tool input.
    pub fn is_unknown(&self) -> bool {
        matches!(self, ToolInput::Unknown(_))
    }
}

// ============================================================================
// Conversion implementations
// ============================================================================

impl From<BashInput> for ToolInput {
    fn from(input: BashInput) -> Self {
        ToolInput::Bash(input)
    }
}

impl From<ReadInput> for ToolInput {
    fn from(input: ReadInput) -> Self {
        ToolInput::Read(input)
    }
}

impl From<WriteInput> for ToolInput {
    fn from(input: WriteInput) -> Self {
        ToolInput::Write(input)
    }
}

impl From<EditInput> for ToolInput {
    fn from(input: EditInput) -> Self {
        ToolInput::Edit(input)
    }
}

impl From<GlobInput> for ToolInput {
    fn from(input: GlobInput) -> Self {
        ToolInput::Glob(input)
    }
}

impl From<GrepInput> for ToolInput {
    fn from(input: GrepInput) -> Self {
        ToolInput::Grep(input)
    }
}

impl From<TaskInput> for ToolInput {
    fn from(input: TaskInput) -> Self {
        ToolInput::Task(input)
    }
}

impl From<WebFetchInput> for ToolInput {
    fn from(input: WebFetchInput) -> Self {
        ToolInput::WebFetch(input)
    }
}

impl From<WebSearchInput> for ToolInput {
    fn from(input: WebSearchInput) -> Self {
        ToolInput::WebSearch(input)
    }
}

impl From<TodoWriteInput> for ToolInput {
    fn from(input: TodoWriteInput) -> Self {
        ToolInput::TodoWrite(input)
    }
}

impl From<AskUserQuestionInput> for ToolInput {
    fn from(input: AskUserQuestionInput) -> Self {
        ToolInput::AskUserQuestion(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bash_input_parsing() {
        let json = serde_json::json!({
            "command": "ls -la",
            "description": "List files",
            "timeout": 5000,
            "run_in_background": false
        });

        let input: BashInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.command, "ls -la");
        assert_eq!(input.description, Some("List files".to_string()));
        assert_eq!(input.timeout, Some(5000));
        assert_eq!(input.run_in_background, Some(false));
    }

    #[test]
    fn test_bash_input_minimal() {
        let json = serde_json::json!({
            "command": "echo hello"
        });

        let input: BashInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.command, "echo hello");
        assert_eq!(input.description, None);
        assert_eq!(input.timeout, None);
    }

    #[test]
    fn test_read_input_parsing() {
        let json = serde_json::json!({
            "file_path": "/home/user/test.rs",
            "offset": 10,
            "limit": 100
        });

        let input: ReadInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.file_path, "/home/user/test.rs");
        assert_eq!(input.offset, Some(10));
        assert_eq!(input.limit, Some(100));
    }

    #[test]
    fn test_write_input_parsing() {
        let json = serde_json::json!({
            "file_path": "/tmp/test.txt",
            "content": "Hello, world!"
        });

        let input: WriteInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.file_path, "/tmp/test.txt");
        assert_eq!(input.content, "Hello, world!");
    }

    #[test]
    fn test_edit_input_parsing() {
        let json = serde_json::json!({
            "file_path": "/home/user/code.rs",
            "old_string": "fn old()",
            "new_string": "fn new()",
            "replace_all": true
        });

        let input: EditInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.file_path, "/home/user/code.rs");
        assert_eq!(input.old_string, "fn old()");
        assert_eq!(input.new_string, "fn new()");
        assert_eq!(input.replace_all, Some(true));
    }

    #[test]
    fn test_glob_input_parsing() {
        let json = serde_json::json!({
            "pattern": "**/*.rs",
            "path": "/home/user/project"
        });

        let input: GlobInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.pattern, "**/*.rs");
        assert_eq!(input.path, Some("/home/user/project".to_string()));
    }

    #[test]
    fn test_grep_input_parsing() {
        let json = serde_json::json!({
            "pattern": "fn\\s+\\w+",
            "path": "/home/user/project",
            "type": "rust",
            "-i": true,
            "-C": 3
        });

        let input: GrepInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.pattern, "fn\\s+\\w+");
        assert_eq!(input.file_type, Some("rust".to_string()));
        assert_eq!(input.case_insensitive, Some(true));
        assert_eq!(input.context, Some(3));
    }

    #[test]
    fn test_task_input_parsing() {
        let json = serde_json::json!({
            "description": "Search codebase",
            "prompt": "Find all usages of foo()",
            "subagent_type": "Explore",
            "run_in_background": true
        });

        let input: TaskInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.description, "Search codebase");
        assert_eq!(input.prompt, "Find all usages of foo()");
        assert_eq!(input.subagent_type, SubagentType::Explore);
        assert_eq!(input.run_in_background, Some(true));
    }

    #[test]
    fn test_web_fetch_input_parsing() {
        let json = serde_json::json!({
            "url": "https://example.com",
            "prompt": "Extract the main content"
        });

        let input: WebFetchInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.url, "https://example.com");
        assert_eq!(input.prompt, "Extract the main content");
    }

    #[test]
    fn test_web_search_input_parsing() {
        let json = serde_json::json!({
            "query": "rust serde tutorial",
            "allowed_domains": ["docs.rs", "crates.io"]
        });

        let input: WebSearchInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.query, "rust serde tutorial");
        assert_eq!(
            input.allowed_domains,
            Some(vec!["docs.rs".to_string(), "crates.io".to_string()])
        );
    }

    #[test]
    fn test_todo_write_input_parsing() {
        let json = serde_json::json!({
            "todos": [
                {
                    "content": "Fix the bug",
                    "status": "in_progress",
                    "activeForm": "Fixing the bug"
                },
                {
                    "content": "Write tests",
                    "status": "pending",
                    "activeForm": "Writing tests"
                }
            ]
        });

        let input: TodoWriteInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.todos.len(), 2);
        assert_eq!(input.todos[0].content, "Fix the bug");
        assert_eq!(input.todos[0].status, TodoStatus::InProgress);
        assert_eq!(input.todos[1].status, TodoStatus::Pending);
    }

    #[test]
    fn test_ask_user_question_input_parsing() {
        let json = serde_json::json!({
            "questions": [
                {
                    "question": "Which framework?",
                    "header": "Framework",
                    "options": [
                        {"label": "React", "description": "Popular UI library"},
                        {"label": "Vue", "description": "Progressive framework"}
                    ],
                    "multiSelect": false
                }
            ]
        });

        let input: AskUserQuestionInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.questions.len(), 1);
        assert_eq!(input.questions[0].question, "Which framework?");
        assert_eq!(input.questions[0].options.len(), 2);
        assert_eq!(input.questions[0].options[0].label, "React");
    }

    #[test]
    fn test_tool_input_enum_bash() {
        let json = serde_json::json!({
            "command": "ls -la"
        });

        let input: ToolInput = serde_json::from_value(json).unwrap();
        assert!(matches!(input, ToolInput::Bash(_)));
        assert_eq!(input.tool_name(), Some("Bash"));
        assert!(input.as_bash().is_some());
    }

    #[test]
    fn test_tool_input_enum_edit() {
        let json = serde_json::json!({
            "file_path": "/test.rs",
            "old_string": "old",
            "new_string": "new"
        });

        let input: ToolInput = serde_json::from_value(json).unwrap();
        assert!(matches!(input, ToolInput::Edit(_)));
        assert_eq!(input.tool_name(), Some("Edit"));
    }

    #[test]
    fn test_tool_input_enum_unknown() {
        // Custom MCP tool with unknown structure
        let json = serde_json::json!({
            "custom_field": "custom_value",
            "another_field": 42
        });

        let input: ToolInput = serde_json::from_value(json).unwrap();
        assert!(matches!(input, ToolInput::Unknown(_)));
        assert_eq!(input.tool_name(), None);
        assert!(input.is_unknown());

        let unknown = input.as_unknown().unwrap();
        assert_eq!(unknown.get("custom_field").unwrap(), "custom_value");
    }

    #[test]
    fn test_tool_input_roundtrip() {
        let original = BashInput {
            command: "echo test".to_string(),
            description: Some("Test command".to_string()),
            timeout: Some(5000),
            run_in_background: None,
        };

        let tool_input: ToolInput = original.clone().into();
        let json = serde_json::to_value(&tool_input).unwrap();
        let parsed: ToolInput = serde_json::from_value(json).unwrap();

        if let ToolInput::Bash(bash) = parsed {
            assert_eq!(bash.command, original.command);
            assert_eq!(bash.description, original.description);
        } else {
            panic!("Expected Bash variant");
        }
    }

    #[test]
    fn test_notebook_edit_input_parsing() {
        let json = serde_json::json!({
            "notebook_path": "/home/user/notebook.ipynb",
            "new_source": "print('hello')",
            "cell_id": "abc123",
            "cell_type": "code",
            "edit_mode": "replace"
        });

        let input: NotebookEditInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.notebook_path, "/home/user/notebook.ipynb");
        assert_eq!(input.new_source, "print('hello')");
        assert_eq!(input.cell_id, Some("abc123".to_string()));
    }

    #[test]
    fn test_task_output_input_parsing() {
        let json = serde_json::json!({
            "task_id": "task-123",
            "block": false,
            "timeout": 60000
        });

        let input: TaskOutputInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.task_id, "task-123");
        assert!(!input.block);
        assert_eq!(input.timeout, 60000);
    }

    #[test]
    fn test_skill_input_parsing() {
        let json = serde_json::json!({
            "skill": "commit",
            "args": "-m 'Fix bug'"
        });

        let input: SkillInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.skill, "commit");
        assert_eq!(input.args, Some("-m 'Fix bug'".to_string()));
    }
}
