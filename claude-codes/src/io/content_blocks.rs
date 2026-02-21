use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// Deserialize content blocks that can be either a string or array
pub(crate) fn deserialize_content_blocks<'de, D>(
    deserializer: D,
) -> Result<Vec<ContentBlock>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Value = Value::deserialize(deserializer)?;
    match value {
        Value::String(s) => Ok(vec![ContentBlock::Text(TextBlock { text: s })]),
        Value::Array(_) => serde_json::from_value(value).map_err(serde::de::Error::custom),
        _ => Err(serde::de::Error::custom(
            "content must be a string or array",
        )),
    }
}

/// Content blocks for messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text(TextBlock),
    Image(ImageBlock),
    Thinking(ThinkingBlock),
    ToolUse(ToolUseBlock),
    ToolResult(ToolResultBlock),
}

/// Text content block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBlock {
    pub text: String,
}

/// Image content block (follows Anthropic API structure)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageBlock {
    pub source: ImageSource,
}

/// Image source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String, // "base64"
    pub media_type: String, // e.g., "image/jpeg", "image/png"
    pub data: String,       // Base64-encoded image data
}

/// Thinking content block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingBlock {
    pub thinking: String,
    pub signature: String,
}

/// Tool use content block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseBlock {
    pub id: String,
    pub name: String,
    pub input: Value,
}

impl ToolUseBlock {
    /// Try to parse the input as a typed ToolInput.
    ///
    /// This attempts to deserialize the raw JSON input into a strongly-typed
    /// `ToolInput` enum variant. Returns `None` if parsing fails.
    ///
    /// # Example
    ///
    /// ```
    /// use claude_codes::{ToolUseBlock, ToolInput};
    /// use serde_json::json;
    ///
    /// let block = ToolUseBlock {
    ///     id: "toolu_123".to_string(),
    ///     name: "Bash".to_string(),
    ///     input: json!({"command": "ls -la"}),
    /// };
    ///
    /// if let Some(ToolInput::Bash(bash)) = block.typed_input() {
    ///     assert_eq!(bash.command, "ls -la");
    /// }
    /// ```
    pub fn typed_input(&self) -> Option<crate::tool_inputs::ToolInput> {
        serde_json::from_value(self.input.clone()).ok()
    }

    /// Parse the input as a typed ToolInput, returning an error on failure.
    ///
    /// Unlike `typed_input()`, this method returns the parsing error for debugging.
    pub fn try_typed_input(&self) -> Result<crate::tool_inputs::ToolInput, serde_json::Error> {
        serde_json::from_value(self.input.clone())
    }
}

/// Tool result content block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultBlock {
    pub tool_use_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<ToolResultContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Tool result content type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    Text(String),
    Structured(Vec<Value>),
}
