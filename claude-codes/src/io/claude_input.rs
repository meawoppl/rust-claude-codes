use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::content_blocks::{ContentBlock, ImageBlock, ImageSource, TextBlock};
use super::control::{ControlRequest, ControlResponse};
use super::message_types::{MessageContent, UserMessage};

/// Top-level enum for all possible Claude input messages.
///
/// Keep variants unboxed so pattern matches and constructors stay ergonomic.
#[allow(clippy::large_enum_variant)]
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
                role: super::MessageRole::User,
                content: vec![ContentBlock::Text(TextBlock {
                    text: text.into(),
                    citations: Vec::new(),
                })],
            },
            session_id: Some(session_id),
            parent_tool_use_id: None,
            uuid: None,
            timestamp: None,
            tool_use_result: None,
            subagent_type: None,
            task_description: None,
            origin: None,
            priority: None,
            is_synthetic: None,
            should_query: None,
            is_meta: None,
            is_visible_in_transcript_only: None,
            is_virtual: None,
            is_compact_summary: None,
            summarize_metadata: None,
            mcp_meta: None,
            tool_result_meta: None,
            source_tool_use_id: None,
            source_tool_assistant_uuid: None,
            image_paste_ids: None,
            client_platform: None,
            inbound_origin: None,
            is_replay: None,
            file_attachments: None,
        })
    }

    /// Create a user message with content blocks
    pub fn user_message_blocks(blocks: Vec<ContentBlock>, session_id: Uuid) -> Self {
        ClaudeInput::User(UserMessage {
            message: MessageContent {
                role: super::MessageRole::User,
                content: blocks,
            },
            session_id: Some(session_id),
            parent_tool_use_id: None,
            uuid: None,
            timestamp: None,
            tool_use_result: None,
            subagent_type: None,
            task_description: None,
            origin: None,
            priority: None,
            is_synthetic: None,
            should_query: None,
            is_meta: None,
            is_visible_in_transcript_only: None,
            is_virtual: None,
            is_compact_summary: None,
            summarize_metadata: None,
            mcp_meta: None,
            tool_result_meta: None,
            source_tool_use_id: None,
            source_tool_assistant_uuid: None,
            image_paste_ids: None,
            client_platform: None,
            inbound_origin: None,
            is_replay: None,
            file_attachments: None,
        })
    }

    /// Create an interrupt control message.
    ///
    /// Sends `{ "subtype": "interrupt" }` to the CLI subprocess's stdin,
    /// telling Claude to stop its current response and return control
    /// without killing the session.
    pub fn interrupt() -> Self {
        ClaudeInput::Raw(serde_json::to_value(super::SDKControlInterruptRequest::new()).unwrap())
    }

    /// Create a user message with an image and optional text
    /// Only supports JPEG, PNG, GIF, and WebP media types
    pub fn user_message_with_image(
        image_data: String,
        media_type: super::MediaType,
        text: Option<String>,
        session_id: Uuid,
    ) -> Result<Self, String> {
        // Validate media type
        match &media_type {
            super::MediaType::Jpeg
            | super::MediaType::Png
            | super::MediaType::Gif
            | super::MediaType::Webp => {}
            other => {
                return Err(format!(
                    "Invalid media type '{}'. Only JPEG, PNG, GIF, and WebP are supported.",
                    other
                ));
            }
        }

        let mut blocks = vec![ContentBlock::Image(ImageBlock {
            source: ImageSource {
                source_type: super::ImageSourceType::Base64,
                media_type,
                data: image_data,
            },
        })];

        if let Some(text_content) = text {
            blocks.push(ContentBlock::Text(TextBlock {
                text: text_content,
                citations: Vec::new(),
            }));
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
