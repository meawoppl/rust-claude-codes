use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::content_blocks::{ContentBlock, ImageBlock, ImageSource, TextBlock};
use super::control::{ControlRequest, ControlResponse};
use super::message_types::{MessageContent, UserMessage};

/// Top-level enum for all possible Claude input messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeInput {
    /// User message input
    User(UserMessage),

    /// Control request (for initialization handshake)
    ControlRequest(ControlRequest),

    /// Control response (for tool permission responses)
    ControlResponse(ControlResponse),

    /// Raw JSON for untyped messages
    #[serde(untagged)]
    Raw(Value),
}

impl ClaudeInput {
    /// Create a simple text user message
    pub fn user_message(text: impl Into<String>, session_id: Uuid) -> Self {
        ClaudeInput::User(UserMessage {
            message: MessageContent {
                role: "user".to_string(),
                content: vec![ContentBlock::Text(TextBlock { text: text.into() })],
            },
            session_id: Some(session_id),
        })
    }

    /// Create a user message with content blocks
    pub fn user_message_blocks(blocks: Vec<ContentBlock>, session_id: Uuid) -> Self {
        ClaudeInput::User(UserMessage {
            message: MessageContent {
                role: "user".to_string(),
                content: blocks,
            },
            session_id: Some(session_id),
        })
    }

    /// Create a user message with an image and optional text
    /// Only supports JPEG, PNG, GIF, and WebP media types
    pub fn user_message_with_image(
        image_data: String,
        media_type: String,
        text: Option<String>,
        session_id: Uuid,
    ) -> Result<Self, String> {
        // Validate media type
        let valid_types = ["image/jpeg", "image/png", "image/gif", "image/webp"];

        if !valid_types.contains(&media_type.as_str()) {
            return Err(format!(
                "Invalid media type '{}'. Only JPEG, PNG, GIF, and WebP are supported.",
                media_type
            ));
        }

        let mut blocks = vec![ContentBlock::Image(ImageBlock {
            source: ImageSource {
                source_type: "base64".to_string(),
                media_type,
                data: image_data,
            },
        })];

        if let Some(text_content) = text {
            blocks.push(ContentBlock::Text(TextBlock { text: text_content }));
        }

        Ok(Self::user_message_blocks(blocks, session_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_user_message() {
        let session_uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let input = ClaudeInput::user_message("Hello, Claude!", session_uuid);
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"type\":\"user\""));
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"text\":\"Hello, Claude!\""));
        assert!(json.contains("550e8400-e29b-41d4-a716-446655440000"));
    }
}
