//! Lossless-deserialization drift tripwire for captured opencode wire fixtures.
//!
//! Every fixture under `test_cases/` was captured live from opencode 1.18.5
//! (see `test_cases/README.md` for provenance). Each one is deserialized into
//! the generated protocol type, serialized back to a [`serde_json::Value`], and
//! the re-serialized shape is checked to be a subset of the original wire JSON.
//!
//! "Subset" tolerates the two intentional lossy behaviours of the generated
//! types: `Option` fields that were `None` are skipped, and internally-tagged
//! enum discriminant fields (`type`, `role`) are emitted once by the enum rather
//! than by the inner struct. It does NOT tolerate the type inventing, renaming,
//! or mis-typing a field — any such drift makes the re-serialized value carry a
//! key or value the wire never sent, which fails the check.
//!
//! Because the event enum is internally tagged without an `Unknown` fallback,
//! an added/renamed server event type also fails deserialization outright,
//! making the event-stream test a strong forward-compatibility tripwire.

use opencode_codes::protocol_generated::types::{
    Event, MessageWithParts, NotFoundError, Session, SessionStatus,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// Compare two JSON scalars, treating all numbers as `f64` so that an integer
/// `0` on the wire matches a re-serialized floating `0.0` from an `f64` field.
fn scalars_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => match (x.as_f64(), y.as_f64()) {
            (Some(xf), Some(yf)) => xf == yf,
            _ => x == y,
        },
        _ => a == b,
    }
}

/// Assert that `back` (the re-serialized typed value) is contained within
/// `orig` (the raw wire JSON): every key/element `back` carries must be present
/// in `orig` with an equal (subset-compatible) value. Keys present only in
/// `orig` are allowed — they correspond to skipped `None` options.
fn assert_subset(back: &Value, orig: &Value, path: &str) {
    match (back, orig) {
        (Value::Object(bm), Value::Object(om)) => {
            for (k, bv) in bm {
                let ov = om.get(k).unwrap_or_else(|| {
                    panic!("re-serialized value invented key `{path}.{k}` absent from wire JSON")
                });
                assert_subset(bv, ov, &format!("{path}.{k}"));
            }
        }
        (Value::Array(ba), Value::Array(oa)) => {
            assert_eq!(
                ba.len(),
                oa.len(),
                "array length drift at `{path}`: type produced {}, wire had {}",
                ba.len(),
                oa.len()
            );
            for (i, (bv, ov)) in ba.iter().zip(oa.iter()).enumerate() {
                assert_subset(bv, ov, &format!("{path}[{i}]"));
            }
        }
        _ => assert!(
            scalars_eq(back, orig),
            "value drift at `{path}`: type produced {back}, wire had {orig}"
        ),
    }
}

/// Deserialize `json` into `T`, then assert the re-serialized form is a subset
/// of the original. Returns the parsed value for further assertions.
fn roundtrip<T: DeserializeOwned + Serialize>(label: &str, json: &str) -> T {
    let orig: Value = serde_json::from_str(json)
        .unwrap_or_else(|e| panic!("fixture `{label}` is not valid JSON: {e}"));
    let parsed: T = serde_json::from_str(json)
        .unwrap_or_else(|e| panic!("fixture `{label}` failed to deserialize into type: {e}"));
    let back = serde_json::to_value(&parsed)
        .unwrap_or_else(|e| panic!("fixture `{label}` failed to re-serialize: {e}"));
    assert_subset(&back, &orig, label);
    parsed
}

#[test]
fn session_create_roundtrips() {
    let session: Session = roundtrip(
        "session_create",
        include_str!("../test_cases/rest/session_create.json"),
    );
    assert!(session.id.starts_with("ses_"));
    assert_eq!(session.project_id, "global");
    assert_eq!(session.version, "1.18.5");
}

#[test]
fn session_status_empty_roundtrips() {
    let map: BTreeMap<String, SessionStatus> = roundtrip(
        "session_status_empty",
        include_str!("../test_cases/rest/session_status_empty.json"),
    );
    assert!(map.is_empty());
}

#[test]
fn session_status_busy_roundtrips() {
    let map: BTreeMap<String, SessionStatus> = roundtrip(
        "session_status_busy",
        include_str!("../test_cases/rest/session_status_busy.json"),
    );
    assert_eq!(map.len(), 1);
    let status = map.values().next().unwrap();
    assert!(matches!(status, SessionStatus::Busy));
}

#[test]
fn messages_empty_roundtrips() {
    let msgs: Vec<MessageWithParts> = roundtrip(
        "messages_empty",
        include_str!("../test_cases/rest/messages_empty.json"),
    );
    assert!(msgs.is_empty());
}

#[test]
fn messages_after_prompt_roundtrips() {
    let msgs: Vec<MessageWithParts> = roundtrip(
        "messages_after_prompt",
        include_str!("../test_cases/rest/messages_after_prompt.json"),
    );
    assert!(
        msgs.len() >= 2,
        "expected at least a user + assistant message, got {}",
        msgs.len()
    );
    // The assistant turn must carry at least one text part with content.
    let has_text_part = msgs
        .iter()
        .flat_map(|m| &m.parts)
        .any(|p| matches!(p, opencode_codes::protocol_generated::types::Part::Text(_)));
    assert!(has_text_part, "no text part found across captured messages");
}

#[test]
fn abort_response_roundtrips() {
    let aborted: bool = roundtrip(
        "abort_response",
        include_str!("../test_cases/rest/abort_response.json"),
    );
    assert!(aborted);
}

#[test]
fn session_not_found_error_roundtrips() {
    let err: NotFoundError = roundtrip(
        "session_not_found",
        include_str!("../test_cases/errors/session_not_found.json"),
    );
    assert_eq!(err.name, "NotFoundError");
    assert!(err.data.message.contains("Session not found"));
}

#[test]
fn event_stream_every_frame_roundtrips() {
    let stream = include_str!("../test_cases/events/event_stream.jsonl");
    let mut count = 0usize;
    let mut seen_types = std::collections::BTreeSet::new();
    for (i, line) in stream.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let label = format!("event_stream[{i}]");
        let orig: Value =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("{label} is not valid JSON: {e}"));
        if let Some(t) = orig.get("type").and_then(Value::as_str) {
            seen_types.insert(t.to_string());
        }
        let _event: Event = roundtrip(&label, line);
        count += 1;
    }
    assert!(count > 0, "event stream fixture was empty");
    // The captured stream exercised these distinct server event types; if the
    // generated enum ever loses one of these variants, deserialization above
    // fails first, but assert their presence to document the coverage.
    for expected in [
        "server.connected",
        "session.updated",
        "message.updated",
        "message.part.updated",
        "message.part.delta",
        "session.status",
        "session.idle",
    ] {
        assert!(
            seen_types.contains(expected),
            "fixture regression: event type `{expected}` no longer present in capture"
        );
    }
}
