//! Hand-written protocol layer over the generated ACP types.
//!
//! Three things live here, deliberately outside codegen:
//!
//! 1. **Typed dispatch** of incoming traffic: ACP is bidirectional, so the
//!    agent sends the client both notifications ([`AgentNotification`]) and
//!    requests it expects answers to ([`AgentToClientRequest`] — fs access,
//!    terminal control, permission prompts).
//! 2. **Hermes extensions**: hermes-agent attaches data under ACP's
//!    `_meta.hermes` extensibility key ([`HermesMeta`],
//!    [`SessionProvenance`]) and advertises a terminal-setup auth method.
//! 3. Method-name re-exports from the generated tables.

use crate::protocol_generated::types::{
    CancelNotification, CreateTerminalRequest, KillTerminalRequest, ReadTextFileRequest,
    ReleaseTerminalRequest, RequestPermissionRequest, SessionNotification, TerminalOutputRequest,
    WaitForTerminalExitRequest, WriteTextFileRequest,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use crate::protocol_generated::methods;

/// Auth method id hermes advertises when its provider needs terminal setup
/// (run `hermes` interactively once to configure credentials).
pub const HERMES_SETUP_AUTH_METHOD_ID: &str = "hermes-setup";

/// A notification sent by the agent to the client.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum AgentNotification {
    /// `session/update` — the streaming firehose: message chunks, tool
    /// calls, plan updates, mode/model changes.
    SessionUpdate(SessionNotification),
    /// A method this crate doesn't model (forward compatibility).
    Unknown { method: String, params: Value },
}

impl AgentNotification {
    pub fn from_envelope(method: &str, params: Option<Value>) -> serde_json::Result<Self> {
        let params = params.unwrap_or(Value::Null);
        Ok(match method {
            methods::SESSION_UPDATE => {
                AgentNotification::SessionUpdate(serde_json::from_value(params)?)
            }
            _ => AgentNotification::Unknown {
                method: method.to_string(),
                params,
            },
        })
    }
}

/// A notification sent by the client to the agent.
#[derive(Debug, Clone, PartialEq)]
pub enum ClientNotification {
    /// `session/cancel` — stop the current prompt turn.
    SessionCancel(CancelNotification),
}

/// A request the agent sends to the client, expecting a response.
///
/// The client must answer every one of these (via
/// [`AsyncClient::respond`](crate::client_async::AsyncClient::respond) or
/// [`respond_error`](crate::client_async::AsyncClient::respond_error)):
/// hermes blocks the running turn on the answer.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentToClientRequest {
    /// `session/request_permission` — a tool call needs user authorization.
    RequestPermission(RequestPermissionRequest),
    /// `fs/read_text_file` — only sent if the client advertised the
    /// `fs.readTextFile` capability at initialize.
    ReadTextFile(ReadTextFileRequest),
    /// `fs/write_text_file` — only sent if the client advertised the
    /// `fs.writeTextFile` capability at initialize.
    WriteTextFile(WriteTextFileRequest),
    /// `terminal/create` — only sent if the client advertised the
    /// `terminal` capability at initialize.
    TerminalCreate(CreateTerminalRequest),
    /// `terminal/output`
    TerminalOutput(TerminalOutputRequest),
    /// `terminal/kill`
    TerminalKill(KillTerminalRequest),
    /// `terminal/release`
    TerminalRelease(ReleaseTerminalRequest),
    /// `terminal/wait_for_exit`
    TerminalWaitForExit(WaitForTerminalExitRequest),
    /// A method this crate doesn't model (forward compatibility). Respond
    /// with a `method_not_found` error unless you know better.
    Unknown { method: String, params: Value },
}

impl AgentToClientRequest {
    pub fn from_envelope(method: &str, params: Option<Value>) -> serde_json::Result<Self> {
        let params = params.unwrap_or(Value::Null);
        Ok(match method {
            methods::SESSION_REQUEST_PERMISSION => {
                AgentToClientRequest::RequestPermission(serde_json::from_value(params)?)
            }
            methods::FS_READ_TEXT_FILE => {
                AgentToClientRequest::ReadTextFile(serde_json::from_value(params)?)
            }
            methods::FS_WRITE_TEXT_FILE => {
                AgentToClientRequest::WriteTextFile(serde_json::from_value(params)?)
            }
            methods::TERMINAL_CREATE => {
                AgentToClientRequest::TerminalCreate(serde_json::from_value(params)?)
            }
            methods::TERMINAL_OUTPUT => {
                AgentToClientRequest::TerminalOutput(serde_json::from_value(params)?)
            }
            methods::TERMINAL_KILL => {
                AgentToClientRequest::TerminalKill(serde_json::from_value(params)?)
            }
            methods::TERMINAL_RELEASE => {
                AgentToClientRequest::TerminalRelease(serde_json::from_value(params)?)
            }
            methods::TERMINAL_WAIT_FOR_EXIT => {
                AgentToClientRequest::TerminalWaitForExit(serde_json::from_value(params)?)
            }
            _ => AgentToClientRequest::Unknown {
                method: method.to_string(),
                params,
            },
        })
    }
}

/// Anything the agent can send that isn't a response to one of our requests.
#[derive(Debug, Clone, PartialEq)]
pub enum ServerMessage {
    Notification(AgentNotification),
    Request {
        id: crate::jsonrpc::RequestId,
        request: AgentToClientRequest,
    },
}

// ──────────────────────────────────────────────────────────────────────────
// Hermes `_meta` extensions
// ──────────────────────────────────────────────────────────────────────────

/// Hermes-specific extension payloads under ACP's `_meta.hermes` key.
///
/// ACP reserves `_meta` for implementation extensions; hermes-agent nests
/// its data under the `hermes` member. Use [`HermesMeta::from_meta`] to
/// lift it out of any generated type's `meta` field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HermesMeta {
    /// Where a session came from when the adapter rotated or replayed it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_provenance: Option<SessionProvenance>,
    /// Marks a replayed compaction summary during `session/load`/`resume`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_summary: Option<Value>,
    /// Any other hermes extension members (forward compatibility).
    #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra: serde_json::Map<String, Value>,
}

impl HermesMeta {
    /// Extract `_meta.hermes` from a generated type's `meta` field.
    pub fn from_meta(meta: Option<&serde_json::Map<String, Value>>) -> Option<HermesMeta> {
        let hermes = meta?.get("hermes")?;
        serde_json::from_value(hermes.clone()).ok()
    }
}

/// `_meta.hermes.sessionProvenance` — how the ACP session maps onto the
/// underlying hermes conversation (rotations, resumes, replays).
///
/// Open-shaped by design: hermes evolves this faster than the ACP schema,
/// so only the stable members are named and the rest ride in `extra`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionProvenance {
    /// Why the adapter rotated to a new underlying conversation, when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation_reason: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra: serde_json::Map<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn hermes_meta_lifts_from_meta_field() {
        let meta: serde_json::Map<String, Value> = serde_json::from_value(json!({
            "hermes": {
                "sessionProvenance": {"rotationReason": "context_full", "sourceSessionId": "s1"},
                "futureThing": true
            },
            "otherVendor": {"x": 1}
        }))
        .unwrap();
        let h = HermesMeta::from_meta(Some(&meta)).expect("hermes member present");
        let prov = h.session_provenance.expect("provenance");
        assert_eq!(prov.rotation_reason.as_deref(), Some("context_full"));
        assert_eq!(prov.extra["sourceSessionId"], json!("s1"));
        assert_eq!(h.extra["futureThing"], json!(true));
        assert!(HermesMeta::from_meta(None).is_none());
    }

    #[test]
    fn notification_dispatch_and_unknown_fallback() {
        let n = AgentNotification::from_envelope(
            "session/update",
            Some(json!({
                "sessionId": "sess-1",
                "update": {"sessionUpdate": "agent_message_chunk",
                            "content": {"type": "text", "text": "hi"}}
            })),
        )
        .unwrap();
        assert!(matches!(n, AgentNotification::SessionUpdate(_)));
        let u =
            AgentNotification::from_envelope("hermes/experimental", Some(json!({"a": 1}))).unwrap();
        assert!(matches!(u, AgentNotification::Unknown { .. }));
    }

    #[test]
    fn agent_request_dispatch() {
        let r = AgentToClientRequest::from_envelope(
            "fs/read_text_file",
            Some(json!({"sessionId": "s", "path": "/tmp/x"})),
        )
        .unwrap();
        assert!(matches!(r, AgentToClientRequest::ReadTextFile(_)));
        let p = AgentToClientRequest::from_envelope(
            "session/request_permission",
            Some(json!({
                "sessionId": "s",
                "toolCall": {"toolCallId": "t1"},
                "options": [{"optionId": "allow", "name": "Allow", "kind": "allow_once"}]
            })),
        )
        .unwrap();
        assert!(matches!(p, AgentToClientRequest::RequestPermission(_)));
    }
}
