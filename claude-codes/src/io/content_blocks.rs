use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::fmt;

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

/// Encoding type for image source data.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImageSourceType {
    /// Base64-encoded image data.
    Base64,
    /// A source type not yet known to this version of the crate.
    Unknown(String),
}

impl ImageSourceType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Base64 => "base64",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for ImageSourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for ImageSourceType {
    fn from(s: &str) -> Self {
        match s {
            "base64" => Self::Base64,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for ImageSourceType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ImageSourceType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// MIME type for image content.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MediaType {
    /// JPEG image.
    Jpeg,
    /// PNG image.
    Png,
    /// GIF image.
    Gif,
    /// WebP image.
    Webp,
    /// A media type not yet known to this version of the crate.
    Unknown(String),
}

impl MediaType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Gif => "image/gif",
            Self::Webp => "image/webp",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for MediaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for MediaType {
    fn from(s: &str) -> Self {
        match s {
            "image/jpeg" => Self::Jpeg,
            "image/png" => Self::Png,
            "image/gif" => Self::Gif,
            "image/webp" => Self::Webp,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for MediaType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for MediaType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// Image source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: ImageSourceType,
    pub media_type: MediaType,
    pub data: String,
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
