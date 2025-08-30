//! JSON Lines protocol implementation for Claude communication.
//!
//! This module provides the [`Protocol`] struct with methods for:
//! - Serializing messages to JSON Lines format
//! - Deserializing JSON Lines into typed messages
//!
//! The JSON Lines format means each message is a complete JSON object on a single line,
//! terminated by a newline character. This enables streaming communication where messages
//! can be processed as they arrive.
//!
//! # Example
//!
//! ```
//! use claude_codes::{Protocol, ClaudeInput};
//!
//! // Serialize a message
//! let input = ClaudeInput::user_message("Hello!", uuid::Uuid::new_v4());
//! let json_line = Protocol::serialize(&input)?;
//! assert!(json_line.ends_with('\n'));
//!
//! // Deserialize a message
//! let output = Protocol::deserialize::<serde_json::Value>(&json_line)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::error::{Error, Result};
use crate::messages::{Event, Request, Response};
use serde::{Deserialize, Serialize};

/// Protocol handler for Claude Code JSON lines communication
pub struct Protocol;

impl Protocol {
    /// Serialize a message to JSON lines format
    pub fn serialize<T: Serialize>(message: &T) -> Result<String> {
        let json = serde_json::to_string(message)?;
        Ok(format!("{}\n", json))
    }

    /// Deserialize a JSON line into a message
    pub fn deserialize<T: for<'de> Deserialize<'de>>(line: &str) -> Result<T> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Err(Error::Protocol("Empty line".to_string()));
        }
        Ok(serde_json::from_str(trimmed)?)
    }
}

/// Message envelope for routing different message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "message_class", rename_all = "snake_case")]
pub enum MessageEnvelope {
    Request(Request),
    Response(Response),
    Event(Event),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::*;

    #[test]
    fn test_serialize_deserialize() {
        let request = Request {
            message_type: "request".to_string(),
            id: "test-123".to_string(),
            session_id: Some("session-456".to_string()),
            payload: RequestPayload::Initialize(InitializeRequest {
                working_directory: Some("/home/user".to_string()),
                environment: None,
                capabilities: None,
            }),
            metadata: None,
        };

        let serialized = Protocol::serialize(&request).unwrap();
        assert!(serialized.ends_with('\n'));

        let deserialized: Request = Protocol::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.id, request.id);
    }

    #[test]
    fn test_empty_line_error() {
        let result: Result<Request> = Protocol::deserialize("");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_json_error() {
        let result: Result<Request> = Protocol::deserialize("not valid json");
        assert!(result.is_err());
    }
}
