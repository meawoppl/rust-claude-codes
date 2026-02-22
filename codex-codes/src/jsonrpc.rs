//! JSON-RPC message types for the Codex app-server protocol.
//!
//! Based on the codex-rs `jsonrpc_lite` implementation. Note: the Codex app-server
//! does NOT include the `"jsonrpc": "2.0"` field in messages, despite following the
//! JSON-RPC 2.0 pattern.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A JSON-RPC request/response identifier.
///
/// Can be either a string or an integer, matching the codex-rs `RequestId` type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    String(String),
    Integer(i64),
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestId::String(s) => write!(f, "{}", s),
            RequestId::Integer(i) => write!(f, "{}", i),
        }
    }
}

/// A JSON-RPC request (client-to-server or server-to-client).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub id: RequestId,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// A JSON-RPC notification (no response expected).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// A JSON-RPC success response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub id: RequestId,
    pub result: Value,
}

/// The error payload within a JSON-RPC error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcErrorData {
    pub code: i64,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// A JSON-RPC error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub error: JsonRpcErrorData,
    pub id: RequestId,
}

/// Any JSON-RPC message on the wire.
///
/// Deserialized via untagged serde — the presence of `id`, `method`, `result`,
/// or `error` fields determines which variant is matched.
///
/// Variant ordering matters for untagged deserialization:
/// - Request has both `id` and `method`
/// - Response has `id` and `result`
/// - Error has `id` and `error`
/// - Notification has only `method` (no `id`)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
    Error(JsonRpcError),
    Notification(JsonRpcNotification),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_string() {
        let id: RequestId = serde_json::from_str(r#""req_1""#).unwrap();
        assert_eq!(id, RequestId::String("req_1".to_string()));
        assert_eq!(id.to_string(), "req_1");
    }

    #[test]
    fn test_request_id_integer() {
        let id: RequestId = serde_json::from_str("42").unwrap();
        assert_eq!(id, RequestId::Integer(42));
        assert_eq!(id.to_string(), "42");
    }

    #[test]
    fn test_request_roundtrip() {
        let req = JsonRpcRequest {
            id: RequestId::Integer(1),
            method: "thread/start".to_string(),
            params: Some(serde_json::json!({"instructions": "hello"})),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: JsonRpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, RequestId::Integer(1));
        assert_eq!(parsed.method, "thread/start");
    }

    #[test]
    fn test_request_no_params() {
        let json = r#"{"id":1,"method":"turn/interrupt"}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert!(req.params.is_none());

        // Serialized output should omit params
        let out = serde_json::to_string(&req).unwrap();
        assert!(!out.contains("params"));
    }

    #[test]
    fn test_notification_roundtrip() {
        let notif = JsonRpcNotification {
            method: "turn/started".to_string(),
            params: Some(serde_json::json!({"threadId": "th_1", "turnId": "t_1"})),
        };
        let json = serde_json::to_string(&notif).unwrap();
        let parsed: JsonRpcNotification = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.method, "turn/started");
    }

    #[test]
    fn test_response_roundtrip() {
        let resp = JsonRpcResponse {
            id: RequestId::Integer(1),
            result: serde_json::json!({"threadId": "th_abc"}),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, RequestId::Integer(1));
    }

    #[test]
    fn test_error_roundtrip() {
        let err = JsonRpcError {
            id: RequestId::Integer(1),
            error: JsonRpcErrorData {
                code: -32600,
                message: "Invalid request".to_string(),
                data: None,
            },
        };
        let json = serde_json::to_string(&err).unwrap();
        let parsed: JsonRpcError = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.error.code, -32600);
    }

    #[test]
    fn test_message_dispatch_request() {
        let json = r#"{"id":1,"method":"thread/start","params":{}}"#;
        let msg: JsonRpcMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, JsonRpcMessage::Request(_)));
    }

    #[test]
    fn test_message_dispatch_response() {
        let json = r#"{"id":1,"result":{"threadId":"th_1"}}"#;
        let msg: JsonRpcMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, JsonRpcMessage::Response(_)));
    }

    #[test]
    fn test_message_dispatch_error() {
        let json = r#"{"id":1,"error":{"code":-32600,"message":"bad"}}"#;
        let msg: JsonRpcMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, JsonRpcMessage::Error(_)));
    }

    #[test]
    fn test_message_dispatch_notification() {
        let json = r#"{"method":"turn/started","params":{"threadId":"th_1"}}"#;
        let msg: JsonRpcMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, JsonRpcMessage::Notification(_)));
    }

    #[test]
    fn test_no_jsonrpc_field() {
        // Verify we don't serialize a "jsonrpc" field
        let req = JsonRpcRequest {
            id: RequestId::Integer(1),
            method: "test".to_string(),
            params: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("jsonrpc"));
    }
}
