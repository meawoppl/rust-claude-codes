//! Exec-style event view over app-server notifications (issue #213).
//!
//! Downstream renderers (agent-portal's codex-session-lib was the filing
//! consumer) want a stable, serializable event stream in the shape of the
//! CLI's `codex exec` JSONL — without hand-rolling a synthetic struct per
//! notification and re-serializing typed items back into JSON.
//!
//! The serialized contract (settled with that consumer before adoption,
//! and matching real exec JSONL, which is FLAT — `thread.started` carries
//! `thread_id` top-level, `turn.completed` lifts `turn_id`/`status`/
//! `duration_ms`):
//!
//! - **Lifecycle events use dotted exec tags** (`thread.started`,
//!   `turn.completed`, `item.started`, …) with snake_case event-level
//!   fields lifted flat, plus the full typed payload riding along
//!   (`thread:`/`turn:`) for consumers that want more than the flat keys.
//! - **Everything else serializes as a forwarded notification**:
//!   `{"type": "<slash-form method>", "params": <inner payload>}` — e.g.
//!   `turn/diff/updated`, `item/agentMessage/delta`. The dot/slash split is
//!   load-bearing: dots are thread events, slashes are verbatim app-server
//!   notifications.
//! - **Item payloads embed the typed [`ThreadItem`]**, which serializes in
//!   the app-server's camelCase item shape (`"commandExecution"`, not
//!   exec's `"command_execution"`).
//! - **Turn errors arrive as `turn.failed`**, never as a bare `error` tag —
//!   that tag stays free for host-level errors (the portal claims it).
//! - **`item.updated` is absent by construction**: the exec format has the
//!   tag, but app-server 0.147 has no item-update notification to map from
//!   (`item/fileChange/patchUpdated` is the closest, and rides the slash
//!   passthrough). A consumer synthesizing `item.updated` locally keeps
//!   doing so.

use crate::messages::Notification;
use crate::protocol_generated::types::{Thread, ThreadItem, Turn, TurnError, TurnStatus};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

/// One renderable event. See the module docs for the serialized contract.
// Thread/ThreadItem payloads dwarf Raw. Like the Notification enum itself,
// this is a transient per-frame classification consumers unpack promptly;
// boxing would tax every construction/match site for no retained-memory win.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum ExecEvent {
    /// `thread.started` — `thread_id` flat (exec shape), full typed thread
    /// riding along.
    ThreadStarted { thread_id: String, thread: Thread },
    /// `turn.started`
    TurnStarted { thread_id: String, turn: Turn },
    /// `turn.completed` — `turn_id`/`status`/`duration_ms` lifted flat
    /// (exec shape); the full typed turn rides along.
    TurnCompleted { thread_id: String, turn: Turn },
    /// `turn.failed` — a turn-scoped `error` notification.
    TurnFailed {
        thread_id: String,
        turn_id: String,
        error: TurnError,
    },
    /// `item.started`
    ItemStarted {
        thread_id: String,
        started_at_ms: i64,
        item: ThreadItem,
    },
    /// `item.completed`
    ItemCompleted {
        thread_id: String,
        completed_at_ms: i64,
        item: ThreadItem,
    },
    /// Any other notification, serialized as the proxy-forwarded shape
    /// `{"type": "<method>", "params": …}` — slash tags preserved, params
    /// verbatim. Unknown future methods land here, never disappear.
    Raw {
        method: String,
        params: Option<Value>,
    },
}

impl ExecEvent {
    /// Map one notification onto its exec-style event. Never fails and
    /// never drops: lifecycle notifications get first-class variants,
    /// everything else rides [`ExecEvent::Raw`] under its wire method.
    pub fn from_notification(notification: Notification) -> ExecEvent {
        match notification {
            Notification::ThreadStarted(n) => ExecEvent::ThreadStarted {
                thread_id: n.thread.id.clone(),
                thread: n.thread,
            },
            Notification::TurnStarted(n) => ExecEvent::TurnStarted {
                thread_id: n.thread_id,
                turn: n.turn,
            },
            Notification::TurnCompleted(n) => ExecEvent::TurnCompleted {
                thread_id: n.thread_id,
                turn: n.turn,
            },
            Notification::Error(n) => ExecEvent::TurnFailed {
                thread_id: n.thread_id,
                turn_id: n.turn_id,
                error: n.error,
            },
            Notification::ItemStarted(n) => ExecEvent::ItemStarted {
                thread_id: n.thread_id,
                started_at_ms: n.started_at_ms,
                item: n.item,
            },
            Notification::ItemCompleted(n) => ExecEvent::ItemCompleted {
                thread_id: n.thread_id,
                completed_at_ms: n.completed_at_ms,
                item: n.item,
            },
            other => {
                let method = other.method().to_string();
                match other.into_envelope() {
                    Ok((_, params)) => ExecEvent::Raw { method, params },
                    // Serialization of our own typed structs failing would be
                    // a bindings bug; surface the method rather than nothing.
                    Err(_) => ExecEvent::Raw {
                        method,
                        params: None,
                    },
                }
            }
        }
    }

    /// The serialized `type` tag this event carries.
    pub fn tag(&self) -> &str {
        match self {
            ExecEvent::ThreadStarted { .. } => "thread.started",
            ExecEvent::TurnStarted { .. } => "turn.started",
            ExecEvent::TurnCompleted { .. } => "turn.completed",
            ExecEvent::TurnFailed { .. } => "turn.failed",
            ExecEvent::ItemStarted { .. } => "item.started",
            ExecEvent::ItemCompleted { .. } => "item.completed",
            ExecEvent::Raw { method, .. } => method,
        }
    }
}

/// Wire form of the dotted lifecycle events (everything except `Raw`,
/// whose tag is dynamic and therefore hand-serialized).
#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum LifecycleWire {
    #[serde(rename = "thread.started")]
    ThreadStarted { thread_id: String, thread: Thread },
    #[serde(rename = "turn.started")]
    TurnStarted { thread_id: String, turn: Turn },
    #[serde(rename = "turn.completed")]
    TurnCompleted {
        thread_id: String,
        turn_id: String,
        status: TurnStatus,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration_ms: Option<i64>,
        turn: Turn,
    },
    #[serde(rename = "turn.failed")]
    TurnFailed {
        thread_id: String,
        turn_id: String,
        error: TurnError,
    },
    #[serde(rename = "item.started")]
    ItemStarted {
        thread_id: String,
        started_at_ms: i64,
        item: ThreadItem,
    },
    #[serde(rename = "item.completed")]
    ItemCompleted {
        thread_id: String,
        completed_at_ms: i64,
        item: ThreadItem,
    },
}

impl Serialize for ExecEvent {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            ExecEvent::Raw { method, params } => {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", method)?;
                if let Some(params) = params {
                    map.serialize_entry("params", params)?;
                }
                map.end()
            }
            ExecEvent::TurnCompleted { thread_id, turn } => LifecycleWire::TurnCompleted {
                thread_id: thread_id.clone(),
                turn_id: turn.id.clone(),
                status: turn.status.clone(),
                duration_ms: turn.duration_ms,
                turn: turn.clone(),
            }
            .serialize(serializer),
            ExecEvent::ThreadStarted { thread_id, thread } => LifecycleWire::ThreadStarted {
                thread_id: thread_id.clone(),
                thread: thread.clone(),
            }
            .serialize(serializer),
            ExecEvent::TurnStarted { thread_id, turn } => LifecycleWire::TurnStarted {
                thread_id: thread_id.clone(),
                turn: turn.clone(),
            }
            .serialize(serializer),
            ExecEvent::TurnFailed {
                thread_id,
                turn_id,
                error,
            } => LifecycleWire::TurnFailed {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                error: error.clone(),
            }
            .serialize(serializer),
            ExecEvent::ItemStarted {
                thread_id,
                started_at_ms,
                item,
            } => LifecycleWire::ItemStarted {
                thread_id: thread_id.clone(),
                started_at_ms: *started_at_ms,
                item: item.clone(),
            }
            .serialize(serializer),
            ExecEvent::ItemCompleted {
                thread_id,
                completed_at_ms,
                item,
            } => LifecycleWire::ItemCompleted {
                thread_id: thread_id.clone(),
                completed_at_ms: *completed_at_ms,
                item: item.clone(),
            }
            .serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ExecEvent {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        let tag = value
            .get("type")
            .and_then(|t| t.as_str())
            .ok_or_else(|| D::Error::missing_field("type"))?;
        match tag {
            "thread.started" | "turn.started" | "turn.completed" | "turn.failed"
            | "item.started" | "item.completed" => {
                let wire: LifecycleWire =
                    serde_json::from_value(value).map_err(D::Error::custom)?;
                Ok(match wire {
                    LifecycleWire::ThreadStarted { thread_id, thread } => {
                        ExecEvent::ThreadStarted { thread_id, thread }
                    }
                    LifecycleWire::TurnStarted { thread_id, turn } => {
                        ExecEvent::TurnStarted { thread_id, turn }
                    }
                    LifecycleWire::TurnCompleted {
                        thread_id, turn, ..
                    } => ExecEvent::TurnCompleted { thread_id, turn },
                    LifecycleWire::TurnFailed {
                        thread_id,
                        turn_id,
                        error,
                    } => ExecEvent::TurnFailed {
                        thread_id,
                        turn_id,
                        error,
                    },
                    LifecycleWire::ItemStarted {
                        thread_id,
                        started_at_ms,
                        item,
                    } => ExecEvent::ItemStarted {
                        thread_id,
                        started_at_ms,
                        item,
                    },
                    LifecycleWire::ItemCompleted {
                        thread_id,
                        completed_at_ms,
                        item,
                    } => ExecEvent::ItemCompleted {
                        thread_id,
                        completed_at_ms,
                        item,
                    },
                })
            }
            method => Ok(ExecEvent::Raw {
                method: method.to_string(),
                params: value.get("params").cloned(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The consumer contract, pinned: `thread.started` carries `thread_id`
    /// flat; `turn.completed` lifts `turn_id`/`status`/`duration_ms` —
    /// the fields renderers (and real exec JSONL) put top-level.
    #[test]
    fn lifecycle_events_carry_the_consumer_fields_flat() {
        let n = Notification::from_envelope(
            "turn/completed",
            Some(serde_json::json!({
                "threadId": "t-1",
                "turn": {"id": "turn-9", "status": "completed", "durationMs": 42,
                          "items": [], "threadId": "t-1"}
            })),
        )
        .expect("typed");
        let v = serde_json::to_value(ExecEvent::from_notification(n)).expect("serialize");
        assert_eq!(v["type"], "turn.completed");
        assert_eq!(v["turn_id"], "turn-9");
        assert_eq!(v["status"], "completed");
        assert_eq!(v["duration_ms"], 42);
        assert_eq!(v["thread_id"], "t-1");
    }

    /// `thread.started` exposes the flat `thread_id` consumers key on.
    #[test]
    fn thread_started_is_flat() {
        let n = Notification::from_envelope(
            "thread/started",
            Some(serde_json::json!({
                "thread": {"id": "t-7", "status": {"type": "idle"}, "items": [],
                            "cliVersion": "0.147.0", "createdAt": 1, "updatedAt": 1,
                            "cwd": "/", "ephemeral": false, "originator": "test",
                            "preset": null, "modelSlug": "m", "turns": []}
            })),
        )
        .expect("typed");
        let v = serde_json::to_value(ExecEvent::from_notification(n)).expect("serialize");
        assert_eq!(v["type"], "thread.started");
        assert_eq!(v["thread_id"], "t-7");
        assert_eq!(v["thread"]["id"], "t-7");
    }

    /// Item payloads serialize in the app-server camelCase item shape —
    /// the settled answer to the `commandExecution` vs `command_execution`
    /// double-match downstreams carry today.
    #[test]
    fn item_events_use_dotted_tags_and_camel_item_types() {
        let n = Notification::from_envelope(
            "item/completed",
            Some(serde_json::json!({
                "threadId": "t-1",
                "completedAtMs": 5,
                "item": {"type": "agentMessage", "id": "i-1", "text": "hi"}
            })),
        )
        .expect("typed");
        let v = serde_json::to_value(ExecEvent::from_notification(n)).expect("serialize");
        assert_eq!(v["type"], "item.completed");
        assert_eq!(v["item"]["type"], "agentMessage");
    }

    /// Non-lifecycle notifications serialize as the proxy-forwarded shape:
    /// slash-form method as the tag, params verbatim — the dot/slash split
    /// consumers key on is preserved exactly, so passthrough renderers
    /// (`turn/diff/updated` etc.) work unchanged.
    #[test]
    fn forwarded_notifications_keep_slash_tags_and_params() {
        // Typed methods re-serialize through their param structs, so use a
        // real field and assert it survives under `params`.
        let n = Notification::from_envelope(
            "turn/diff/updated",
            Some(serde_json::json!({"threadId": "t-1", "turnId": "u-1", "diff": "+x"})),
        )
        .expect("routes");
        let v = serde_json::to_value(ExecEvent::from_notification(n)).expect("serialize");
        assert_eq!(
            v["type"], "turn/diff/updated",
            "slash tag survives verbatim"
        );
        assert_eq!(
            v["params"]["diff"], "+x",
            "typed params ride under 'params'"
        );

        // Unknown methods carry their params verbatim.
        let n = Notification::from_envelope("somefuture/thing", Some(serde_json::json!({"x": 1})))
            .expect("routes to Unknown");
        let v = serde_json::to_value(ExecEvent::from_notification(n)).expect("serialize");
        assert_eq!(v["type"], "somefuture/thing");
        assert_eq!(v["params"]["x"], 1);

        // And the other three passthrough renderers' tags stay slash-form.
        for method in [
            "item/fileChange/patchUpdated",
            "turn/plan/updated",
            "item/plan/delta",
            "item/agentMessage/delta",
        ] {
            let n =
                Notification::from_envelope(method, Some(serde_json::json!({}))).expect("routes");
            let v = serde_json::to_value(ExecEvent::from_notification(n)).expect("serialize");
            assert_eq!(v["type"], method, "slash tag survives verbatim");
        }
    }

    /// A turn error arrives as `turn.failed` with the typed error under
    /// `error` — the bare `error` tag is never emitted (it belongs to the
    /// host layer; the portal claims it for its own errors).
    #[test]
    fn turn_errors_are_turn_failed_never_bare_error() {
        let n = Notification::from_envelope(
            "error",
            Some(serde_json::json!({
                "threadId": "t-1", "turnId": "turn-1",
                "error": {"message": "boom"}
            })),
        )
        .expect("typed");
        let v = serde_json::to_value(ExecEvent::from_notification(n)).expect("serialize");
        assert_eq!(v["type"], "turn.failed");
        assert_eq!(v["error"]["message"], "boom");
    }

    /// Round trip: serialize → deserialize lands back in the same variant,
    /// including dynamic-tag Raw.
    #[test]
    fn events_round_trip_including_dynamic_raw_tags() {
        let raw = ExecEvent::Raw {
            method: "somefuture/thing".into(),
            params: Some(serde_json::json!({"x": 1})),
        };
        let v = serde_json::to_value(&raw).expect("serialize");
        let back: ExecEvent = serde_json::from_value(v).expect("deserialize");
        assert_eq!(back, raw);
    }
}
