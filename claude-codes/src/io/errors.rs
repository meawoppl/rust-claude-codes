use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::fmt;

/// Error type for parsing failures that preserves the raw JSON
#[derive(Debug, Clone)]
pub struct ParseError {
    /// The raw JSON value that failed to parse
    pub raw_json: Value,
    /// The underlying serde error message
    pub error_message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to parse ClaudeOutput: {}", self.error_message)
    }
}

impl std::error::Error for ParseError {}

/// Known Anthropic API error types.
///
/// Maps to the `type` field inside an error response from the Anthropic API.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ApiErrorType {
    /// Internal server error (HTTP 500)
    ApiError,
    /// Service overloaded (HTTP 529)
    OverloadedError,
    /// Bad request (HTTP 400)
    InvalidRequestError,
    /// Invalid API key (HTTP 401)
    AuthenticationError,
    /// Too many requests (HTTP 429)
    RateLimitError,
    /// An error type not yet known to this version of the crate.
    Unknown(String),
}

impl ApiErrorType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::ApiError => "api_error",
            Self::OverloadedError => "overloaded_error",
            Self::InvalidRequestError => "invalid_request_error",
            Self::AuthenticationError => "authentication_error",
            Self::RateLimitError => "rate_limit_error",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for ApiErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for ApiErrorType {
    fn from(s: &str) -> Self {
        match s {
            "api_error" => Self::ApiError,
            "overloaded_error" => Self::OverloadedError,
            "invalid_request_error" => Self::InvalidRequestError,
            "authentication_error" => Self::AuthenticationError,
            "rate_limit_error" => Self::RateLimitError,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for ApiErrorType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ApiErrorType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// API error message from Anthropic.
///
/// When Claude Code encounters an API error (e.g., 500, 529 overloaded), it outputs
/// a JSON message with `type: "error"`. This struct captures that error information.
///
/// # Example JSON
///
/// ```json
/// {
///   "type": "error",
///   "error": {
///     "type": "api_error",
///     "message": "Internal server error"
///   },
///   "request_id": "req_011CXPC6BqUogB959LWEf52X"
/// }
/// ```
///
/// # Example
///
/// ```
/// use claude_codes::ClaudeOutput;
///
/// let json = r#"{"type":"error","error":{"type":"api_error","message":"Internal server error"},"request_id":"req_123"}"#;
/// let output: ClaudeOutput = serde_json::from_str(json).unwrap();
///
/// if let ClaudeOutput::Error(err) = output {
///     println!("Error type: {}", err.error.error_type);
///     println!("Message: {}", err.error.message);
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnthropicError {
    /// The nested error details
    pub error: AnthropicErrorDetails,
    /// The request ID for debugging/support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl AnthropicError {
    /// Check if this is an overloaded error (HTTP 529)
    pub fn is_overloaded(&self) -> bool {
        self.error.error_type == ApiErrorType::OverloadedError
    }

    /// Check if this is a server error (HTTP 500)
    pub fn is_server_error(&self) -> bool {
        self.error.error_type == ApiErrorType::ApiError
    }

    /// Check if this is an invalid request error (HTTP 400)
    pub fn is_invalid_request(&self) -> bool {
        self.error.error_type == ApiErrorType::InvalidRequestError
    }

    /// Check if this is an authentication error (HTTP 401)
    pub fn is_authentication_error(&self) -> bool {
        self.error.error_type == ApiErrorType::AuthenticationError
    }

    /// Check if this is a rate limit error (HTTP 429)
    pub fn is_rate_limited(&self) -> bool {
        self.error.error_type == ApiErrorType::RateLimitError
    }
}

/// Details of an Anthropic API error.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnthropicErrorDetails {
    /// The type of error
    #[serde(rename = "type")]
    pub error_type: ApiErrorType,
    /// Human-readable error message
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::ClaudeOutput;

    #[test]
    fn test_deserialize_anthropic_error() {
        let json = r#"{
            "type": "error",
            "error": {
                "type": "api_error",
                "message": "Internal server error"
            },
            "request_id": "req_011CXPC6BqUogB959LWEf52X"
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert!(output.is_api_error());
        assert_eq!(output.message_type(), "error");

        if let ClaudeOutput::Error(err) = output {
            assert_eq!(err.error.error_type, ApiErrorType::ApiError);
            assert_eq!(err.error.message, "Internal server error");
            assert_eq!(
                err.request_id,
                Some("req_011CXPC6BqUogB959LWEf52X".to_string())
            );
            assert!(err.is_server_error());
            assert!(!err.is_overloaded());
        } else {
            panic!("Expected Error variant");
        }
    }

    #[test]
    fn test_deserialize_anthropic_overloaded_error() {
        let json = r#"{
            "type": "error",
            "error": {
                "type": "overloaded_error",
                "message": "Overloaded"
            }
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();

        if let ClaudeOutput::Error(err) = output {
            assert!(err.is_overloaded());
            assert!(!err.is_server_error());
            assert!(err.request_id.is_none());
        } else {
            panic!("Expected Error variant");
        }
    }

    #[test]
    fn test_deserialize_anthropic_rate_limit_error() {
        let json = r#"{
            "type": "error",
            "error": {
                "type": "rate_limit_error",
                "message": "Rate limit exceeded"
            },
            "request_id": "req_456"
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();

        if let ClaudeOutput::Error(err) = output {
            assert!(err.is_rate_limited());
            assert!(!err.is_overloaded());
            assert!(!err.is_server_error());
        } else {
            panic!("Expected Error variant");
        }
    }

    #[test]
    fn test_deserialize_anthropic_authentication_error() {
        let json = r#"{
            "type": "error",
            "error": {
                "type": "authentication_error",
                "message": "Invalid API key"
            }
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();

        if let ClaudeOutput::Error(err) = output {
            assert!(err.is_authentication_error());
        } else {
            panic!("Expected Error variant");
        }
    }

    #[test]
    fn test_deserialize_anthropic_invalid_request_error() {
        let json = r#"{
            "type": "error",
            "error": {
                "type": "invalid_request_error",
                "message": "Invalid request body"
            }
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();

        if let ClaudeOutput::Error(err) = output {
            assert!(err.is_invalid_request());
        } else {
            panic!("Expected Error variant");
        }
    }

    #[test]
    fn test_anthropic_error_as_helper() {
        let json = r#"{"type":"error","error":{"type":"api_error","message":"Error"}}"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();

        let err = output.as_anthropic_error();
        assert!(err.is_some());
        assert_eq!(err.unwrap().error.error_type, ApiErrorType::ApiError);

        // Non-error should return None
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
        assert!(result.as_anthropic_error().is_none());
    }

    #[test]
    fn test_anthropic_error_roundtrip() {
        let error = AnthropicError {
            error: AnthropicErrorDetails {
                error_type: ApiErrorType::ApiError,
                message: "Test error".to_string(),
            },
            request_id: Some("req_123".to_string()),
        };

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("\"type\":\"api_error\""));
        assert!(json.contains("\"message\":\"Test error\""));
        assert!(json.contains("\"request_id\":\"req_123\""));

        let parsed: AnthropicError = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, error);
    }

    #[test]
    fn test_anthropic_error_session_id_is_none() {
        let json = r#"{"type":"error","error":{"type":"api_error","message":"Error"}}"#;
        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert!(output.session_id().is_none());
    }
}
