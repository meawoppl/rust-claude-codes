use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Status of a command execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandExecutionStatus {
    InProgress,
    Completed,
    Failed,
}

/// A command execution item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecutionItem {
    pub id: String,
    pub command: String,
    pub aggregated_output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub status: CommandExecutionStatus,
}

/// Kind of patch change applied to a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchChangeKind {
    Add,
    Delete,
    Update,
}

/// A single file update within a file change item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUpdateChange {
    pub path: String,
    pub kind: PatchChangeKind,
}

/// Status of a patch apply operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchApplyStatus {
    Completed,
    Failed,
}

/// A file change item representing one or more file modifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChangeItem {
    pub id: String,
    pub changes: Vec<FileUpdateChange>,
    pub status: PatchApplyStatus,
}

/// Status of an MCP tool call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpToolCallStatus {
    InProgress,
    Completed,
    Failed,
}

/// Result of an MCP tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCallResult {
    pub content: Vec<Value>,
    pub structured_content: Value,
}

/// Error from an MCP tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCallError {
    pub message: String,
}

/// An MCP tool call item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCallItem {
    pub id: String,
    pub server: String,
    pub tool: String,
    pub arguments: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<McpToolCallResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpToolCallError>,
    pub status: McpToolCallStatus,
}

/// An agent message item containing text output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessageItem {
    pub id: String,
    pub text: String,
}

/// A reasoning item containing the model's chain-of-thought.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningItem {
    pub id: String,
    pub text: String,
}

/// A web search item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchItem {
    pub id: String,
    pub query: String,
}

/// An error item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorItem {
    pub id: String,
    pub message: String,
}

/// A single todo entry within a todo list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub text: String,
    pub completed: bool,
}

/// A todo list item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoListItem {
    pub id: String,
    pub items: Vec<TodoItem>,
}

/// All possible thread item types emitted by the Codex CLI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ThreadItem {
    AgentMessage(AgentMessageItem),
    Reasoning(ReasoningItem),
    CommandExecution(CommandExecutionItem),
    FileChange(FileChangeItem),
    McpToolCall(McpToolCallItem),
    WebSearch(WebSearchItem),
    TodoList(TodoListItem),
    Error(ErrorItem),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_agent_message() {
        let json = r#"{"type":"agent_message","id":"msg_1","text":"Hello world"}"#;
        let item: ThreadItem = serde_json::from_str(json).unwrap();
        assert!(matches!(item, ThreadItem::AgentMessage(ref m) if m.text == "Hello world"));
    }

    #[test]
    fn test_deserialize_command_execution() {
        let json = r#"{"type":"command_execution","id":"cmd_1","command":"ls -la","aggregated_output":"total 0","exit_code":0,"status":"completed"}"#;
        let item: ThreadItem = serde_json::from_str(json).unwrap();
        assert!(matches!(item, ThreadItem::CommandExecution(ref c) if c.exit_code == Some(0)));
    }

    #[test]
    fn test_deserialize_file_change() {
        let json = r#"{"type":"file_change","id":"fc_1","changes":[{"path":"src/main.rs","kind":"update"}],"status":"completed"}"#;
        let item: ThreadItem = serde_json::from_str(json).unwrap();
        assert!(
            matches!(item, ThreadItem::FileChange(ref f) if f.changes[0].kind == PatchChangeKind::Update)
        );
    }

    #[test]
    fn test_deserialize_todo_list() {
        let json = r#"{"type":"todo_list","id":"td_1","items":[{"text":"Fix bug","completed":false},{"text":"Write tests","completed":true}]}"#;
        let item: ThreadItem = serde_json::from_str(json).unwrap();
        assert!(matches!(item, ThreadItem::TodoList(ref t) if t.items.len() == 2));
    }

    #[test]
    fn test_deserialize_error() {
        let json = r#"{"type":"error","id":"err_1","message":"something went wrong"}"#;
        let item: ThreadItem = serde_json::from_str(json).unwrap();
        assert!(matches!(item, ThreadItem::Error(ref e) if e.message == "something went wrong"));
    }

    #[test]
    fn test_deserialize_reasoning() {
        let json = r#"{"type":"reasoning","id":"r_1","text":"Let me think about this..."}"#;
        let item: ThreadItem = serde_json::from_str(json).unwrap();
        assert!(matches!(item, ThreadItem::Reasoning(ref r) if r.text.contains("think")));
    }

    #[test]
    fn test_deserialize_web_search() {
        let json = r#"{"type":"web_search","id":"ws_1","query":"rust serde tutorial"}"#;
        let item: ThreadItem = serde_json::from_str(json).unwrap();
        assert!(matches!(item, ThreadItem::WebSearch(ref w) if w.query == "rust serde tutorial"));
    }

    #[test]
    fn test_deserialize_mcp_tool_call() {
        let json = r#"{"type":"mcp_tool_call","id":"mcp_1","server":"my-server","tool":"my-tool","arguments":{"key":"value"},"status":"completed","result":{"content":[],"structured_content":null}}"#;
        let item: ThreadItem = serde_json::from_str(json).unwrap();
        assert!(matches!(item, ThreadItem::McpToolCall(ref m) if m.tool == "my-tool"));
    }
}
