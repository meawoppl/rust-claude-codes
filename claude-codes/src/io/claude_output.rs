use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::content_blocks::{ContentBlock, ToolUseBlock};
use super::control::{ControlRequest, ControlResponse};
use super::errors::{AnthropicError, ParseError};
use super::message_types::{AssistantMessage, SystemMessage, UserMessage};
use super::rate_limit::RateLimitEvent;
use super::result::ResultMessage;

/// Top-level enum for all possible Claude output messages
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Control request from CLI (tool permissions, hooks, etc.)
    ControlRequest(ControlRequest),

    /// Control response from CLI (ack for initialization, etc.)
    ControlResponse(ControlResponse),

    /// API error from Anthropic (500, 529 overloaded, etc.)
    Error(AnthropicError),

    /// Rate limit status event
    RateLimitEvent(RateLimitEvent),
}

impl ClaudeOutput {
    /// Get the message type as a string
    pub fn message_type(&self) -> String {
        match self {
            ClaudeOutput::System(_) => "system".to_string(),
            ClaudeOutput::User(_) => "user".to_string(),
            ClaudeOutput::Assistant(_) => "assistant".to_string(),
            ClaudeOutput::Result(_) => "result".to_string(),
            ClaudeOutput::ControlRequest(_) => "control_request".to_string(),
            ClaudeOutput::ControlResponse(_) => "control_response".to_string(),
            ClaudeOutput::Error(_) => "error".to_string(),
            ClaudeOutput::RateLimitEvent(_) => "rate_limit_event".to_string(),
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
            ClaudeOutput::System(sys) => sys.data.get("session_id").and_then(|v| v.as_str()),
            ClaudeOutput::Assistant(ass) => Some(&ass.session_id),
            ClaudeOutput::Result(res) => Some(&res.session_id),
            ClaudeOutput::User(_) => None,
            ClaudeOutput::ControlRequest(_) => None,
            ClaudeOutput::ControlResponse(_) => None,
            ClaudeOutput::Error(_) => None,
            ClaudeOutput::RateLimitEvent(evt) => Some(&evt.session_id),
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
