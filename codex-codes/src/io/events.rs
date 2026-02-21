use serde::{Deserialize, Serialize};

use super::items::ThreadItem;

/// Token usage statistics for a completed turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
}

/// Error information from a thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadError {
    pub message: String,
}

/// Event indicating a thread has started.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadStartedEvent {
    pub thread_id: String,
}

/// Event indicating a turn has started.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnStartedEvent {}

/// Event indicating a turn has completed successfully.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnCompletedEvent {
    pub usage: Usage,
}

/// Event indicating a turn has failed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnFailedEvent {
    pub error: ThreadError,
}

/// Event indicating an item has started processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemStartedEvent {
    pub item: ThreadItem,
}

/// Event indicating an item has been updated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemUpdatedEvent {
    pub item: ThreadItem,
}

/// Event indicating an item has completed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemCompletedEvent {
    pub item: ThreadItem,
}

/// A thread-level error event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadErrorEvent {
    pub message: String,
}

/// All possible events emitted during a Codex thread execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ThreadEvent {
    #[serde(rename = "thread.started")]
    ThreadStarted(ThreadStartedEvent),
    #[serde(rename = "turn.started")]
    TurnStarted(TurnStartedEvent),
    #[serde(rename = "turn.completed")]
    TurnCompleted(TurnCompletedEvent),
    #[serde(rename = "turn.failed")]
    TurnFailed(TurnFailedEvent),
    #[serde(rename = "item.started")]
    ItemStarted(ItemStartedEvent),
    #[serde(rename = "item.updated")]
    ItemUpdated(ItemUpdatedEvent),
    #[serde(rename = "item.completed")]
    ItemCompleted(ItemCompletedEvent),
    Error(ThreadErrorEvent),
}

impl ThreadEvent {
    /// Returns the event type string.
    pub fn event_type(&self) -> &str {
        match self {
            ThreadEvent::ThreadStarted(_) => "thread.started",
            ThreadEvent::TurnStarted(_) => "turn.started",
            ThreadEvent::TurnCompleted(_) => "turn.completed",
            ThreadEvent::TurnFailed(_) => "turn.failed",
            ThreadEvent::ItemStarted(_) => "item.started",
            ThreadEvent::ItemUpdated(_) => "item.updated",
            ThreadEvent::ItemCompleted(_) => "item.completed",
            ThreadEvent::Error(_) => "error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_thread_started() {
        let json = r#"{"type":"thread.started","thread_id":"th_abc123"}"#;
        let event: ThreadEvent = serde_json::from_str(json).unwrap();
        assert!(matches!(event, ThreadEvent::ThreadStarted(ref e) if e.thread_id == "th_abc123"));
        assert_eq!(event.event_type(), "thread.started");
    }

    #[test]
    fn test_deserialize_turn_started() {
        let json = r#"{"type":"turn.started"}"#;
        let event: ThreadEvent = serde_json::from_str(json).unwrap();
        assert!(matches!(event, ThreadEvent::TurnStarted(_)));
    }

    #[test]
    fn test_deserialize_turn_completed() {
        let json = r#"{"type":"turn.completed","usage":{"input_tokens":100,"cached_input_tokens":50,"output_tokens":200}}"#;
        let event: ThreadEvent = serde_json::from_str(json).unwrap();
        if let ThreadEvent::TurnCompleted(e) = &event {
            assert_eq!(e.usage.input_tokens, 100);
            assert_eq!(e.usage.cached_input_tokens, 50);
            assert_eq!(e.usage.output_tokens, 200);
        } else {
            panic!("Expected TurnCompleted");
        }
    }

    #[test]
    fn test_deserialize_turn_failed() {
        let json = r#"{"type":"turn.failed","error":{"message":"rate limited"}}"#;
        let event: ThreadEvent = serde_json::from_str(json).unwrap();
        assert!(
            matches!(event, ThreadEvent::TurnFailed(ref e) if e.error.message == "rate limited")
        );
    }

    #[test]
    fn test_deserialize_item_started() {
        let json = r#"{"type":"item.started","item":{"type":"agent_message","id":"msg_1","text":"Starting..."}}"#;
        let event: ThreadEvent = serde_json::from_str(json).unwrap();
        assert!(matches!(event, ThreadEvent::ItemStarted(_)));
    }

    #[test]
    fn test_deserialize_error_event() {
        let json = r#"{"type":"error","message":"connection lost"}"#;
        let event: ThreadEvent = serde_json::from_str(json).unwrap();
        assert!(matches!(event, ThreadEvent::Error(ref e) if e.message == "connection lost"));
    }
}
