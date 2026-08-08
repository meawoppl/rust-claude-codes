//! Corpus tests: every fixture must decode, and decoding must not lose data.
//!
//! The strong assertion here is **no field loss**. Decoding to [`OutputEvent`]
//! and serialising straight back must reproduce every leaf the harness sent. A
//! field this crate forgot to model would otherwise decode "successfully" by
//! silently dropping itself, and nothing else in the suite would notice.
//!
//! Two directories feed this:
//!
//! - `test_cases/events/` — frames captured verbatim from a live 0.1.10
//!   harness. These are the wire, not a re-serialisation of it.
//! - `test_cases/synthetic/` — hand-written frames for paths a session without
//!   a valid API key never reaches (tool calls, hooks, policy, usage), plus two
//!   forward-compatibility cases.

use std::path::{Path, PathBuf};

use antigravity_codes::protocol::OutputEvent;
use serde_json::Value;

fn fixtures(dir: &str) -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("test_cases")
        .join(dir);
    let mut paths: Vec<_> = std::fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("reading {}: {e}", root.display()))
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no fixtures in {}", root.display());
    paths
}

/// Collects every `(json-pointer, leaf value)` pair in a document.
fn leaves(value: &Value, path: String, out: &mut Vec<(String, Value)>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                leaves(v, format!("{path}/{k}"), out);
            }
        }
        Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                leaves(v, format!("{path}/{i}"), out);
            }
        }
        leaf => out.push((path, leaf.clone())),
    }
}

#[test]
fn every_captured_frame_decodes() {
    for path in fixtures("events") {
        let raw = std::fs::read_to_string(&path).unwrap();
        serde_json::from_str::<OutputEvent>(&raw)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }
}

#[test]
fn every_synthetic_frame_decodes() {
    for path in fixtures("synthetic") {
        let raw = std::fs::read_to_string(&path).unwrap();
        serde_json::from_str::<OutputEvent>(&raw)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }
}

#[test]
fn decoding_a_captured_frame_loses_nothing() {
    for path in fixtures("events") {
        let raw = std::fs::read_to_string(&path).unwrap();
        let original: Value = serde_json::from_str(&raw).unwrap();
        let typed: OutputEvent = serde_json::from_str(&raw).unwrap();
        let round_tripped = serde_json::to_value(&typed).unwrap();

        let mut expected = Vec::new();
        leaves(&original, String::new(), &mut expected);

        for (pointer, value) in expected {
            let actual = round_tripped.pointer(&pointer);
            assert_eq!(
                actual,
                Some(&value),
                "{}: `{pointer}` was dropped or changed by the round trip",
                path.display()
            );
        }
    }
}

#[test]
fn decoding_is_idempotent() {
    for dir in ["events", "synthetic"] {
        for path in fixtures(dir) {
            let raw = std::fs::read_to_string(&path).unwrap();
            let once: OutputEvent = serde_json::from_str(&raw).unwrap();
            let text = serde_json::to_string(&once).unwrap();
            let twice: OutputEvent = serde_json::from_str(&text).unwrap();
            assert_eq!(
                once,
                twice,
                "{} is not stable across a round trip",
                path.display()
            );
        }
    }
}

/// A frame whose `oneof` arm this crate has never heard of must still decode —
/// the harness is versioned independently, and a hard failure here would turn
/// every upstream addition into an outage.
#[test]
fn an_unknown_oneof_arm_decodes_to_no_arm() {
    let raw = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("test_cases/synthetic/unknown-future-frame.json"),
    )
    .unwrap();
    let event: OutputEvent = serde_json::from_str(&raw).unwrap();
    assert_eq!(event.sequence(), Some(22));
    assert!(event.into_event().is_none());
}

/// Likewise for an enum value added upstream: it is retained verbatim rather
/// than rejected.
#[test]
fn an_unknown_enum_value_is_retained() {
    use antigravity_codes::protocol::StepUpdateState;

    let raw = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("test_cases/synthetic/unknown-enum-value.json"),
    )
    .unwrap();
    let event: OutputEvent = serde_json::from_str(&raw).unwrap();
    let step = event.step_update.unwrap();
    assert_eq!(
        step.state,
        Some(StepUpdateState::Unknown("STATE_SOMETHING_NEW".into()))
    );
    assert_eq!(step.state.unwrap().as_str(), "STATE_SOMETHING_NEW");
}

#[test]
fn sixty_four_bit_counters_survive_as_numbers() {
    let raw = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("test_cases/synthetic/usage-update.json"),
    )
    .unwrap();
    let event: OutputEvent = serde_json::from_str(&raw).unwrap();
    let usage = event.usage_update.unwrap();
    assert_eq!(usage.total.as_ref().unwrap().total_token_count, Some(1539));
    assert_eq!(
        usage.agents[0].usage.as_ref().unwrap().prompt_token_count,
        Some(120)
    );
}
