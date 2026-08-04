//! JSON-RPC 2.0 message types for the ACP wire.
//!
//! ACP is newline-delimited JSON-RPC 2.0 over stdio. Unlike some agent
//! protocols, ACP **does** include the `"jsonrpc": "2.0"` member on every
//! message (matching the reference SDKs), and request ids may be strings,
//! numbers, or null.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The `jsonrpc` protocol marker every ACP frame carries.
pub const JSONRPC_VERSION: &str = "2.0";

fn jsonrpc_version() -> String {
    JSONRPC_VERSION.to_string()
}

/// A JSON-RPC request/response identifier: string, number, or null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    Null,
    Integer(i64),
    String(String),
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestId::Null => write!(f, "null"),
            RequestId::Integer(i) => write!(f, "{i}"),
            RequestId::String(s) => write!(f, "{s}"),
        }
    }
}

/// A JSON-RPC request (either direction — ACP is bidirectional).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    #[serde(default = "jsonrpc_version")]
    pub jsonrpc: String,
    pub id: RequestId,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// A JSON-RPC notification (no response expected).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    #[serde(default = "jsonrpc_version")]
    pub jsonrpc: String,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// A JSON-RPC success response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    #[serde(default = "jsonrpc_version")]
    pub jsonrpc: String,
    pub id: RequestId,
    pub result: Value,
}

impl JsonRpcResponse {
    pub fn new(id: RequestId, result: Value) -> Self {
        Self {
            jsonrpc: jsonrpc_version(),
            id,
            result,
        }
    }
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
    #[serde(default = "jsonrpc_version")]
    pub jsonrpc: String,
    pub id: RequestId,
    pub error: JsonRpcErrorData,
}

/// Any JSON-RPC message on the wire.
///
/// Variant ordering matters for untagged deserialization:
/// Request has `id` + `method`; Response has `id` + `result`; Error has
/// `id` + `error`; Notification has `method` and no `id`.
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
    fn frames_classify_by_shape() {
        let m: JsonRpcMessage =
            serde_json::from_str(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#)
                .unwrap();
        assert!(matches!(m, JsonRpcMessage::Request(_)));
        let m: JsonRpcMessage =
            serde_json::from_str(r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1}}"#)
                .unwrap();
        assert!(matches!(m, JsonRpcMessage::Response(_)));
        let m: JsonRpcMessage = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"nope"}}"#,
        )
        .unwrap();
        assert!(matches!(m, JsonRpcMessage::Error(_)));
        let m: JsonRpcMessage = serde_json::from_str(
            r#"{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"s"}}"#,
        )
        .unwrap();
        assert!(matches!(m, JsonRpcMessage::Notification(_)));
    }

    #[test]
    fn request_id_admits_null_number_string() {
        assert_eq!(
            serde_json::from_str::<RequestId>("null").unwrap(),
            RequestId::Null
        );
        assert_eq!(
            serde_json::from_str::<RequestId>("7").unwrap(),
            RequestId::Integer(7)
        );
        assert_eq!(
            serde_json::from_str::<RequestId>(r#""a""#).unwrap(),
            RequestId::String("a".into())
        );
    }
}
