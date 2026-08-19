//! Exec-style event view over app-server notifications (issue #213).
//!
//! Downstream renderers (agent-portal's codex-session-lib was the filing
//! consumer) want a stable, serializable event stream in the shape of the
//! CLI's `codex exec` JSONL — without hand-rolling a synthetic struct per
//! notification and re-serializing typed items back into JSON. This module
//! is that adapter: [`ExecEvent::from_notification`] maps the lifecycle
//! notifications renderers actually draw onto tagged, serde-stable events,
//! and passes everything else through as [`ExecEvent::Raw`] with its wire
//! method and params preserved — unknown future notifications degrade to
//! raw, never disappear.

use crate::messages::Notification;
use crate::protocol_generated::types::{Thread, ThreadItem, ThreadTokenUsage, Turn, TurnError};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One renderable event, tagged in `codex exec` JSONL style.
///
/// Typed payloads (`ThreadItem`, `Turn`, …) are embedded directly — they
/// already serialize to their wire shapes, so consumers get stable JSON
/// without a re-serialization layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExecEvent {
    #[serde(rename = "thread.started")]
    ThreadStarted { thread: Thread },
    #[serde(rename = "turn.started")]
    TurnStarted { thread_id: String, turn: Turn },
    #[serde(rename = "turn.completed")]
    TurnCompleted { thread_id: String, turn: Turn },
    /// A turn-scoped error (`error` notification) — the closest app-server
    /// analog to exec's `turn.failed`.
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
    /// Streamed agent-message text for the active item.
    #[serde(rename = "item.agentMessage.delta")]
    AgentMessageDelta {
        thread_id: String,
        item_id: String,
        delta: String,
    },
    /// Per-turn token accounting (`thread/tokenUsage/updated`).
    #[serde(rename = "thread.tokenUsage")]
    TokenUsage {
        thread_id: String,
        turn_id: String,
        token_usage: ThreadTokenUsage,
    },
    /// Any notification without a first-class mapping above — including
    /// methods newer than these bindings. The wire method and params are
    /// preserved verbatim so nothing is droppable-by-default.
    #[serde(rename = "raw")]
    Raw {
        method: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        params: Option<Value>,
    },
}

impl ExecEvent {
    /// Map one notification onto its exec-style event. Never fails and
    /// never drops: lifecycle notifications get first-class variants,
    /// everything else round-trips through [`ExecEvent::Raw`].
    pub fn from_notification(notification: Notification) -> ExecEvent {
        match notification {
            Notification::ThreadStarted(n) => ExecEvent::ThreadStarted { thread: n.thread },
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
            Notification::AgentMessageDelta(n) => ExecEvent::AgentMessageDelta {
                thread_id: n.thread_id,
                item_id: n.item_id,
                delta: n.delta,
            },
            Notification::ThreadTokenUsageUpdated(n) => ExecEvent::TokenUsage {
                thread_id: n.thread_id,
                turn_id: n.turn_id,
                token_usage: n.token_usage,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lifecycle notifications map to exec-tagged events whose JSON shape
    /// is stable (`type` tag in exec JSONL style).
    #[test]
    fn item_completed_maps_to_exec_tagged_json() {
        let n = Notification::from_envelope(
            "item/completed",
            Some(serde_json::json!({
                "threadId": "t-1",
                "completedAtMs": 5,
                "item": {"type": "agentMessage", "id": "i-1", "text": "hi"}
            })),
        )
        .expect("typed notification");
        let event = ExecEvent::from_notification(n);
        let v = serde_json::to_value(&event).expect("serialize");
        assert_eq!(v["type"], "item.completed");
        assert_eq!(v["thread_id"], "t-1");
        assert_eq!(v["item"]["type"], "agentMessage");
        assert_eq!(v["item"]["text"], "hi");
    }

    /// Unknown methods pass through with method + params preserved — a
    /// newer CLI's notification degrades to raw, never disappears.
    #[test]
    fn unknown_notification_passes_through_raw() {
        let n = Notification::from_envelope("somefuture/thing", Some(serde_json::json!({"x": 1})))
            .expect("unknown routes to Unknown without error");
        let event = ExecEvent::from_notification(n);
        match &event {
            ExecEvent::Raw { method, params } => {
                assert_eq!(method, "somefuture/thing");
                assert_eq!(params.as_ref().unwrap()["x"], 1);
            }
            other => panic!("expected Raw, got {other:?}"),
        }
        let v = serde_json::to_value(&event).expect("serialize");
        assert_eq!(v["type"], "raw");
    }

    /// A typed-but-unmapped notification (no first-class exec variant) also
    /// rides Raw, keeping its wire method — nothing is droppable-by-default.
    #[test]
    fn typed_but_unmapped_notification_rides_raw_with_its_method() {
        let n = Notification::from_envelope(
            "thread/reverted",
            Some(serde_json::json!({"threadId": "t-9"})),
        )
        .expect("typed");
        match ExecEvent::from_notification(n) {
            ExecEvent::Raw { method, params } => {
                assert_eq!(method, "thread/reverted");
                assert_eq!(params.unwrap()["threadId"], "t-9");
            }
            other => panic!("expected Raw, got {other:?}"),
        }
    }
}
