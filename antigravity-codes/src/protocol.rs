//! The protobuf-JSON wire types, re-exported flat.
//!
//! Every type here is generated from the descriptor the shipped harness binary
//! was built with — see `scripts/codegen_antigravity.py`. The two envelopes
//! worth knowing are [`InputEvent`] (client to harness) and [`OutputEvent`]
//! (harness to client); the rest hang off those.
//!
//! # How a protobuf `oneof` is modelled
//!
//! The JSON mapping flattens a `oneof` — only the set arm appears, as an
//! ordinary member of the parent object. So the generated struct keeps one
//! `Option` per arm rather than a Rust `enum`, which is what makes an unknown
//! future arm decode cleanly instead of failing the whole frame. For matching,
//! each `oneof` also gets an owned view:
//!
//! ```
//! use antigravity_codes::protocol::{OutputEvent, OutputEventEvent, StepUpdate};
//!
//! let event = OutputEvent {
//!     step_update: Some(StepUpdate { text: Some("hi".into()), ..Default::default() }),
//!     ..Default::default()
//! };
//!
//! match event.into_event() {
//!     Some(OutputEventEvent::StepUpdate(step)) => assert_eq!(step.text.as_deref(), Some("hi")),
//!     other => panic!("unexpected arm: {other:?}"),
//! }
//! ```

pub use crate::protocol_generated::types::*;

impl InputEvent {
    /// A plain text turn: `InputEvent { user_input: Some(text) }`.
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            user_input: Some(text.into()),
            ..Default::default()
        }
    }

    /// Asks the harness to abandon the turn in flight.
    pub fn halt() -> Self {
        Self {
            halt_request: Some(true),
            ..Default::default()
        }
    }

    /// Asks the harness to end the session and flush its state to disk.
    pub fn session_end() -> Self {
        Self {
            session_end_request: Some(true),
            ..Default::default()
        }
    }

    /// The result of a client-side tool call, answering an [`OutputEvent`]'s
    /// [`ToolCall`].
    pub fn tool_response(response: ToolResponse) -> Self {
        Self {
            tool_response: Some(response),
            ..Default::default()
        }
    }

    /// A reply to a [`CallHookRequest`].
    pub fn hook_response(response: CallHookResponse) -> Self {
        Self {
            call_hook_response: Some(response),
            ..Default::default()
        }
    }

    /// A reply to a [`PolicyDecisionRequest`].
    pub fn policy_response(response: PolicyDecisionResponse) -> Self {
        Self {
            policy_decision_response: Some(response),
            ..Default::default()
        }
    }
}

impl ToolResponse {
    /// A successful result, carrying the tool's JSON-encoded return value.
    pub fn ok(id: impl Into<String>, response_json: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            response_json: Some(response_json.into()),
            ..Default::default()
        }
    }

    /// A failed result. The harness surfaces `message` to the model, and may
    /// route it through an `on_tool_error` hook first.
    pub fn error(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            error_message: Some(message.into()),
            ..Default::default()
        }
    }
}

impl StepUpdate {
    /// True once this step will not be updated again.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            Some(StepUpdateState::Done) | Some(StepUpdateState::Error)
        )
    }

    /// The text this step contributes, preferring the accumulated `text` the
    /// harness sends on completion over the incremental delta.
    pub fn text_or_delta(&self) -> Option<&str> {
        self.text
            .as_deref()
            .filter(|t| !t.is_empty())
            .or(self.text_delta.as_deref().filter(|t| !t.is_empty()))
    }
}

impl OutputEvent {
    /// The monotonically increasing sequence number the harness stamps on every
    /// frame, useful for ordering assertions and gap detection.
    pub fn sequence(&self) -> Option<i64> {
        self.seq_num
    }
}
