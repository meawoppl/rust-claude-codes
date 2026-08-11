//! Typed models of the `muse exec --json` JSONL event stream.
//!
//! Every stdout line is one [`MuseRecord`] — an event-sourced journal
//! envelope with a lazily-typed payload. The envelope is fully typed; the
//! payload stays raw JSON on the record (so round-trips are byte-faithful
//! and unknown future payload types survive) and is lifted into a
//! [`MusePayload`] on demand via [`MuseRecord::typed_payload`].
//!
//! Shapes in this module are derived from **captured real output** of
//! Muse Code (see `test_cases/*.jsonl`), not from documentation — the wire
//! is the contract. Payload types not yet observed (the journal also
//! records approvals, edits, and subagent lifecycle under a live provider)
//! deserialize as [`MusePayload::Unknown`] rather than failing.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One line of the `muse exec --json` stream: the journal envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MuseRecord {
    /// Envelope schema version (observed: `1`).
    pub schema_version: u32,
    /// Unique record id (UUIDv7-style, monotonic within a stream).
    pub id: String,
    /// The stream this record belongs to.
    pub stream: StreamRef,
    /// 1-based position within `stream`.
    pub sequence: u64,
    /// Microseconds since the Unix epoch.
    pub recorded_at: u64,
    pub record_type: RecordType,
    pub durability: Durability,
    /// Id of the command that caused this record.
    pub causation_id: String,
    /// Dotted payload discriminator, e.g. `run.output.delta`.
    pub payload_type: String,
    /// Version of the payload's own schema (observed: `1`).
    pub payload_schema_version: u32,
    /// Raw payload — lift with [`MuseRecord::typed_payload`].
    pub payload: Value,
}

impl MuseRecord {
    /// Parse the payload into its typed form based on `payload_type`.
    ///
    /// Unknown payload types return [`MusePayload::Unknown`] carrying the
    /// raw value; a payload that fails to match its expected shape is a
    /// deserialization error (wire drift worth surfacing, not masking).
    pub fn typed_payload(&self) -> serde_json::Result<MusePayload> {
        MusePayload::from_parts(&self.payload_type, self.payload.clone())
    }
}

/// Reference to a journal stream.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StreamRef {
    pub kind: StreamKind,
    pub id: String,
}

/// Journal stream classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamKind {
    Session,
    Run,
    Task,
}

/// Journal record classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordType {
    /// Replay-exact state reconciliation (e.g. command acceptance).
    Reconciliation,
    /// Durable domain event.
    Event,
    /// Ephemeral progress/status (e.g. output deltas).
    Status,
}

/// Whether the record survives restart/replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Durability {
    Durable,
    Ephemeral,
}

/// Typed payload of a [`MuseRecord`], discriminated by `payload_type`.
#[derive(Debug, Clone, PartialEq)]
pub enum MusePayload {
    /// `runtime.command.accepted`
    CommandAccepted(CommandAccepted),
    /// `session.run.linked`
    SessionRunLinked(SessionRunLinked),
    /// `turn.input.user`
    TurnInputUser(TurnInputUser),
    /// `run.lifecycle.started`
    RunStarted(RunStarted),
    /// `run.model.configured`
    ModelConfigured(ModelConfigured),
    /// `run.output.delta`
    RunOutputDelta(RunOutputDelta),
    /// `tool.result`
    ToolResult(ToolResult),
    /// `run.terminal.completed` (and any future `run.terminal.*`)
    RunTerminal(RunTerminal),
    /// `task.stream.linked`
    TaskStreamLinked(TaskStreamLinked),
    /// `task.lifecycle.*`
    TaskLifecycle(TaskLifecycle),
    /// A payload type not yet known to this crate — preserved verbatim.
    Unknown {
        payload_type: String,
        payload: Value,
    },
}

impl MusePayload {
    pub fn from_parts(payload_type: &str, payload: Value) -> serde_json::Result<Self> {
        Ok(match payload_type {
            "runtime.command.accepted" => {
                MusePayload::CommandAccepted(serde_json::from_value(payload)?)
            }
            "session.run.linked" => MusePayload::SessionRunLinked(serde_json::from_value(payload)?),
            "turn.input.user" => MusePayload::TurnInputUser(serde_json::from_value(payload)?),
            "run.lifecycle.started" => MusePayload::RunStarted(serde_json::from_value(payload)?),
            "run.model.configured" => {
                MusePayload::ModelConfigured(serde_json::from_value(payload)?)
            }
            "tool.result" => MusePayload::ToolResult(serde_json::from_value(payload)?),
            "run.output.delta" => MusePayload::RunOutputDelta(serde_json::from_value(payload)?),
            t if t.starts_with("run.terminal.") => {
                MusePayload::RunTerminal(serde_json::from_value(payload)?)
            }
            "task.stream.linked" => MusePayload::TaskStreamLinked(serde_json::from_value(payload)?),
            t if t.starts_with("task.lifecycle.") => {
                MusePayload::TaskLifecycle(serde_json::from_value(payload)?)
            }
            other => MusePayload::Unknown {
                payload_type: other.to_string(),
                payload,
            },
        })
    }
}

/// `runtime.command.accepted` — the runtime took ownership of a submitted
/// command (`command_kind`, e.g. `turn.submit`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandAccepted {
    pub kind: String,
    pub command_id: String,
    pub command_kind: String,
    pub client_id: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra: serde_json::Map<String, Value>,
}

/// `session.run.linked` — a run stream was attached to the session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionRunLinked {
    pub kind: String,
    pub command_id: String,
    pub run_stream: StreamRef,
    #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra: serde_json::Map<String, Value>,
}

/// `turn.input.user` — the user prompt as the runtime recorded it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnInputUser {
    pub kind: String,
    pub command_id: String,
    pub prompt: String,
    pub run_stream: StreamRef,
    #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra: serde_json::Map<String, Value>,
}

/// `run.lifecycle.started`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunStarted {
    pub kind: String,
    pub command_id: String,
    pub prompt: String,
    pub run_stream: StreamRef,
    #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra: serde_json::Map<String, Value>,
}

/// `run.output.delta` — streamed model/agent output text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunOutputDelta {
    pub kind: String,
    pub command_id: String,
    pub run_stream: StreamRef,
    pub text: String,
    #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra: serde_json::Map<String, Value>,
}

/// `run.model.configured` — which model/profile/provider the run resolved
/// to (live providers only; the echo provider never emits it).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelConfigured {
    pub kind: String,
    pub command_id: String,
    pub run_stream: StreamRef,
    pub model_id: String,
    pub display_label: String,
    pub profile_id: String,
    pub provider_id: String,
    /// How the model was chosen (`startup` observed).
    pub source: String,
    #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra: serde_json::Map<String, Value>,
}

/// `tool.result` — outcome of one tool invocation (live providers only).
///
/// **No `task_id`**, but the wire models each tool call as its own task
/// (`task_kind: tool.<tool_name>`) and `correlation_facts.tool_name`
/// names it — match on that, latest-first. Recency-of-running-tasks
/// heuristics mis-attribute: the issuing tool task has already completed
/// when this record lands. `call_id` is the provider's call id, not a
/// task handle — see the README's known-wire-gaps section.
///
/// `correlation_facts` is absent on some tool results (e.g. compact `bash`
/// results like `{"items":5,"ok":true,"revision":4}` observed on
/// `3035c77c-efca...`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    pub kind: String,
    pub command_id: String,
    pub run_stream: StreamRef,
    /// Provider call id this result answers.
    pub call_id: String,
    /// Result text as shown to the model (including failure prose).
    pub text: String,
    /// Correlation summary — observed `{outcome, tool_name}`, open-shaped.
    /// Absent on some results; treat as `None` when missing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_facts: Option<Value>,
    /// Populated for file-editing tools; open-shaped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit_facts: Option<Value>,
    #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra: serde_json::Map<String, Value>,
}

/// `run.terminal.*` — the run reached a terminal state. `terminal` carries
/// the state (`completed` observed); `text` the final output; `reason` is
/// populated on abnormal endings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunTerminal {
    pub kind: String,
    pub command_id: String,
    pub run_stream: StreamRef,
    pub terminal: String,
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra: serde_json::Map<String, Value>,
}

/// `task.stream.linked` — a task stream was attached to a run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskStreamLinked {
    pub kind: String,
    pub command_id: String,
    pub run_stream: StreamRef,
    pub task_id: String,
    pub task_stream: StreamRef,
    #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra: serde_json::Map<String, Value>,
}

/// `task.lifecycle.*` — one step in a task's lifecycle state machine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskLifecycle {
    pub kind: String,
    pub command_id: String,
    pub run_stream: StreamRef,
    pub task_id: String,
    pub task_stream: StreamRef,
    pub event: TaskLifecycleEvent,
    #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra: serde_json::Map<String, Value>,
}

/// The `event` member of [`TaskLifecycle`], tagged by `kind`.
///
/// Observed lifecycle: `proposed → accepted → started → (scheduled →
/// side_effect_intent →) completed | failed`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TaskLifecycleEvent {
    Proposed {
        task_id: String,
        /// Dotted task class, e.g. `model.unknown.response` or
        /// `reminder.agent.plugin:<plugin>:<name>`.
        task_kind: String,
    },
    Accepted {
        task_id: String,
    },
    Started {
        task_id: String,
        /// Tracing span id (live providers attach one; echo does not).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        span_id: Option<String>,
    },
    Scheduled {
        task_id: String,
        idempotency_key: String,
    },
    SideEffectIntent {
        task_id: String,
        idempotency_key: String,
        operation: String,
        policy_decision: String,
        parent_task_id: Option<String>,
        cancellation_handle: Option<Value>,
    },
    /// Free-form progress (`message` + faceted `details`), e.g. model
    /// stream attempts.
    Status {
        task_id: String,
        message: String,
        details: Value,
    },
    /// Streamed task output chunk (e.g. tool stdout summaries).
    Output {
        task_id: String,
        chunk: String,
    },
    Completed {
        task_id: String,
    },
    Cancelled {
        task_id: String,
        reason: String,
    },
    Rejected {
        task_id: String,
        reason: String,
    },
    Failed {
        task_id: String,
        reason: String,
    },
    /// A lifecycle kind not yet known to this crate.
    #[serde(untagged)]
    Unknown(Value),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unknown_payload_type_is_preserved_not_error() {
        let p = MusePayload::from_parts("subagent.lifecycle.spawned", json!({"x": 1})).unwrap();
        match p {
            MusePayload::Unknown {
                payload_type,
                payload,
            } => {
                assert_eq!(payload_type, "subagent.lifecycle.spawned");
                assert_eq!(payload, json!({"x": 1}));
            }
            other => panic!("expected Unknown, got {other:?}"),
        }
    }

    #[test]
    fn task_lifecycle_failed_carries_reason() {
        let e: TaskLifecycleEvent = serde_json::from_value(json!({
            "kind": "failed",
            "task_id": "t1",
            "reason": "provider does not support base instructions"
        }))
        .unwrap();
        assert!(matches!(e, TaskLifecycleEvent::Failed { ref reason, .. }
            if reason.contains("base instructions")));
    }
}
