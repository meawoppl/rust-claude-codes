//! Coverage for the CLI 2.1.266 stream-json additive drift.
//!
//! 2.1.263 → 2.1.266 added one `system` subtype (`dev_intent`) and top-level
//! fields to four wire types, all additive (no removals). Between the two
//! releases the CLI also tightened the assistant/user `message` field from
//! `z.unknown()` to a concrete Anthropic-API-shaped schema; that is a
//! passthrough shape the crate already types via [`ContentBlock`], so it needs
//! no new modeling and is not exercised here.
//!
//! Each frame below carries the new fields and is asserted **fully wrapped** —
//! the typed model captures every wire field with nothing left in an untyped
//! escape hatch.

use claude_codes::{
    assert_fully_wrapped, ClaudeOutput, DevIntentKind, KnownSystemEvent, RunnerExitPhase,
    SystemSubtype,
};
use serde_json::json;

/// The new `system/dev_intent` subtype is modeled, round-trips fully wrapped,
/// and exposes its `kind` through the typed accessors.
#[test]
fn system_dev_intent_fully_wrapped() {
    let frame = json!({
        "type": "system",
        "subtype": "dev_intent",
        "kind": "ios_app",
        "uuid": "u1",
        "session_id": "s1",
        "future_field": "preserved"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    assert_eq!(sys.subtype, SystemSubtype::DevIntent);
    assert!(sys.is_dev_intent());

    let direct = sys.as_dev_intent().expect("direct typed accessor");
    assert_eq!(direct.kind, DevIntentKind::IosApp);
    assert_eq!(direct.uuid, "u1");
    assert_eq!(direct.session_id, "s1");
    assert_eq!(direct.extra["future_field"], "preserved");

    let Some(KnownSystemEvent::DevIntent(known)) = sys.as_known_system_event() else {
        panic!("expected DevIntent event");
    };
    assert_eq!(known.kind.as_str(), "ios_app");
}

/// An unrecognized `dev_intent` kind falls back to `Unknown` rather than
/// failing the frame — the CLI documents the set as open.
#[test]
fn system_dev_intent_unknown_kind() {
    let frame = json!({
        "type": "system",
        "subtype": "dev_intent",
        "kind": "android_app",
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let direct = sys.as_dev_intent().unwrap();
    assert_eq!(
        direct.kind,
        DevIntentKind::Unknown("android_app".to_string())
    );
    assert_eq!(direct.kind.as_str(), "android_app");
}

/// An `assistant` frame carrying the 2.1.266 `historical` and
/// `wire_ingest_context` wrapper siblings round-trips without loss.
#[test]
fn assistant_carries_historical_and_wire_ingest_context() {
    let frame = json!({
        "type": "assistant",
        "message": {
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "claude-opus-4-8",
            "content": [{"type": "text", "text": "done"}],
            "stop_reason": null,
            "usage": {
                "input_tokens": 1,
                "output_tokens": 2,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0
            }
        },
        "session_id": "s1",
        "uuid": "u1",
        "parent_tool_use_id": null,
        "historical": true,
        "wire_ingest_context": {"toolu_1": {"cwd": "/repo"}}
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected assistant");
    };
    assert_eq!(msg.historical, Some(true));
    assert_eq!(msg.wire_ingest_context.unwrap()["toolu_1"]["cwd"], "/repo");

    // Older frames without the fields default to absent and do not serialize
    // the keys back.
    let old = json!({
        "type": "assistant",
        "message": {
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "claude-opus-4-8",
            "content": [],
            "stop_reason": null,
            "usage": {
                "input_tokens": 1,
                "output_tokens": 2,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0
            }
        },
        "session_id": "s1"
    });
    let ClaudeOutput::Assistant(msg) = serde_json::from_value(old).unwrap() else {
        panic!("expected assistant");
    };
    assert_eq!(msg.historical, None);
    assert!(msg.wire_ingest_context.is_none());
    let reserialized = serde_json::to_string(&msg).unwrap();
    assert!(!reserialized.contains("historical"));
    assert!(!reserialized.contains("wire_ingest_context"));
}

/// A `result` frame carrying the 2.1.266 `runner_exit` failure payload
/// round-trips, including a `null` exit code that rode a terminating signal.
#[test]
fn result_carries_runner_exit() {
    let frame = json!({
        "type": "result",
        "subtype": "error_during_execution",
        "is_error": true,
        "duration_ms": 5,
        "duration_api_ms": 0,
        "num_turns": 0,
        "session_id": "s1",
        "total_cost_usd": 0.0,
        "runner_exit": {"phase": "setup", "exit_code": null, "signal": "SIGKILL"}
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected result");
    };
    let runner_exit = res.runner_exit.expect("runner_exit present");
    assert_eq!(runner_exit.phase, RunnerExitPhase::Setup);
    assert_eq!(runner_exit.exit_code, None);
    assert_eq!(runner_exit.signal, Some("SIGKILL".to_string()));

    // The `run`-phase variant with a numeric exit code also round-trips.
    let run_frame = json!({
        "type": "result",
        "subtype": "error_during_execution",
        "is_error": true,
        "duration_ms": 5,
        "duration_api_ms": 0,
        "num_turns": 0,
        "session_id": "s1",
        "total_cost_usd": 0.0,
        "runner_exit": {"phase": "run", "exit_code": 137}
    });
    assert_fully_wrapped(&run_frame);
    let ClaudeOutput::Result(res) = serde_json::from_value(run_frame).unwrap() else {
        panic!("expected result");
    };
    let runner_exit = res.runner_exit.unwrap();
    assert_eq!(runner_exit.phase, RunnerExitPhase::Run);
    assert_eq!(runner_exit.exit_code, Some(137));
    assert_eq!(runner_exit.signal, None);
}

/// A `system/init` frame carrying the 2.1.266 `startup_timing` map round-trips
/// fully wrapped.
#[test]
fn system_init_carries_startup_timing() {
    let frame = json!({
        "type": "system",
        "subtype": "init",
        "session_id": "s1",
        "uuid": "u1",
        "startup_timing": {"phase_boot_ms": 42, "resume_hydrated_messages": 3}
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let init = sys.as_init().expect("typed init");
    assert_eq!(init.startup_timing.unwrap()["phase_boot_ms"], 42);
}

/// A `system/compact_boundary` frame carrying the 2.1.266 `historical` flag
/// round-trips fully wrapped.
#[test]
fn system_compact_boundary_carries_historical() {
    let frame = json!({
        "type": "system",
        "subtype": "compact_boundary",
        "session_id": "s1",
        "uuid": "u1",
        "compact_metadata": {"trigger": "auto", "pre_tokens": 1000},
        "historical": true
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected System");
    };
    let cb = sys.as_compact_boundary().expect("typed compact_boundary");
    assert_eq!(cb.historical, Some(true));
}

/// A `user` frame carrying the 2.1.266 `historical` flag round-trips fully
/// wrapped.
#[test]
fn user_carries_historical() {
    let frame = json!({
        "type": "user",
        "message": {"role": "user", "content": []},
        "session_id": "11111111-1111-1111-1111-111111111111",
        "historical": true
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::User(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected user");
    };
    assert_eq!(msg.historical, Some(true));
}
