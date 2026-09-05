//! Coverage for the CLI 2.1.261 stream-json additive drift.
//!
//! 2.1.259 → 2.1.261 added one `system` subtype (`cloud_session_delta`) and
//! top-level fields to four wire types without removing any (see the drift
//! report on issue #361). Each frame below carries the new fields and is
//! asserted **fully wrapped** — the typed model captures every wire field
//! with nothing left in an untyped escape hatch.

use claude_codes::{assert_fully_wrapped, ClaudeOutput, KnownSystemEvent, SystemSubtype};
use serde_json::json;

/// Pins that an `assistant` frame's 2.1.260 `narration_block_indexes`
/// wrapper sibling round-trips without loss and defaults to empty.
#[test]
fn assistant_carries_narration_block_indexes() {
    let frame = json!({
        "type": "assistant",
        "message": {
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "claude-opus-4-8",
            "content": [
                {"type": "thinking", "thinking": "", "signature": "sig-narration"},
                {"type": "text", "text": "done"}
            ],
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
        "narration_block_indexes": [0]
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected assistant");
    };
    assert_eq!(msg.narration_block_indexes, vec![0]);

    let reserialized = serde_json::to_string(&msg).unwrap();
    assert!(reserialized.contains("\"narration_block_indexes\":[0]"));

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
    assert!(msg.narration_block_indexes.is_empty());
    let reserialized = serde_json::to_string(&msg).unwrap();
    assert!(!reserialized.contains("narration_block_indexes"));
}

/// Pins that a `result/success` frame's 2.1.260 stream-timing fields
/// (`first_content_frame_ms`, `first_stream_post_ms`,
/// `first_stream_post_ack_ms`, `first_stream_post_wall_ms`) round-trip.
#[test]
fn result_carries_first_stream_post_timings() {
    let frame = json!({
        "type": "result",
        "subtype": "success",
        "is_error": false,
        "duration_ms": 1200,
        "duration_api_ms": 900,
        "num_turns": 1,
        "result": "ok",
        "session_id": "s1",
        "total_cost_usd": 0.01,
        "usage": {"input_tokens": 1, "output_tokens": 2},
        "permission_denials": [],
        "uuid": "u1",
        "ttft_ms": 300,
        "request_sent_wall_ms": 1753212345678.25,
        "first_content_frame_ms": 310,
        "first_stream_post_ms": 12,
        "first_stream_post_ack_ms": 40,
        "first_stream_post_wall_ms": 1753212345690.5
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Result(res) = serde_json::from_value(frame).unwrap() else {
        panic!("expected result");
    };
    assert_eq!(res.first_content_frame_ms, Some(310));
    assert_eq!(res.first_stream_post_ms, Some(12));
    assert_eq!(res.first_stream_post_ack_ms, Some(40));
    assert_eq!(res.first_stream_post_wall_ms, Some(1753212345690.5));

    let reserialized = serde_json::to_string(&res).unwrap();
    assert!(reserialized.contains("\"first_content_frame_ms\":310"));
    assert!(reserialized.contains("\"first_stream_post_wall_ms\":1753212345690.5"));
}

/// Pins that a `system/api_retry` frame's 2.1.261 `no_response` block
/// (first-byte-timeout retries) round-trips, and is absent otherwise.
#[test]
fn api_retry_carries_no_response_timing() {
    let frame = json!({
        "type": "system",
        "subtype": "api_retry",
        "attempt": 1,
        "max_retries": 1,
        "retry_delay_ms": 0,
        "error_status": null,
        "error": "no response headers within 30000ms",
        "no_response": {"waited_ms": 30000, "retry_wait_ms": 60000},
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected system");
    };
    assert_eq!(sys.subtype, SystemSubtype::ApiRetry);
    let Some(KnownSystemEvent::ApiRetry(retry)) = sys.as_known_system_event() else {
        panic!("expected ApiRetry event");
    };
    let no_response = retry.no_response.expect("no_response present");
    assert_eq!(no_response.waited_ms, 30000);
    assert_eq!(no_response.retry_wait_ms, 60000);

    let ordinary = json!({
        "type": "system",
        "subtype": "api_retry",
        "attempt": 2,
        "max_retries": 10,
        "retry_delay_ms": 2000,
        "error_status": 529,
        "error": "overloaded",
        "uuid": "u2",
        "session_id": "s1"
    });
    let ClaudeOutput::System(sys) = serde_json::from_value(ordinary).unwrap() else {
        panic!("expected system");
    };
    let Some(KnownSystemEvent::ApiRetry(retry)) = sys.as_known_system_event() else {
        panic!("expected ApiRetry event");
    };
    assert!(retry.no_response.is_none());
    assert!(!serde_json::to_string(&retry)
        .unwrap()
        .contains("no_response"));
}

/// Pins that a `system/thinking_tokens` frame's 2.1.261 `user_message_uuid`
/// send-binding key round-trips.
#[test]
fn thinking_tokens_carries_user_message_uuid() {
    let frame = json!({
        "type": "system",
        "subtype": "thinking_tokens",
        "estimated_tokens": 420,
        "estimated_tokens_delta": 20,
        "user_message_uuid": "um-1",
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected system");
    };
    let Some(KnownSystemEvent::ThinkingTokens(tokens)) = sys.as_known_system_event() else {
        panic!("expected ThinkingTokens event");
    };
    assert_eq!(tokens.user_message_uuid.as_deref(), Some("um-1"));
    assert_eq!(tokens.estimated_tokens, 420);
}

/// Pins the new 2.1.260 `system/cloud_session_delta` subtype: a typed view,
/// a `KnownSystemEvent` variant, and the raw `cloud_session` block held
/// verbatim (its shape is internal and evolving).
#[test]
fn cloud_session_delta_is_a_known_system_event() {
    let frame = json!({
        "type": "system",
        "subtype": "cloud_session_delta",
        "seq": 1,
        "changed": ["serving"],
        "cloud_session": {
            "id": "session_abc",
            "view_url": "https://example.invalid/s/abc",
            "device": {"status": "bound", "device_id": "dev-1"},
            "serving": {"state": "on", "policy": "parity"}
        },
        "uuid": "u1",
        "session_id": "s1"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected system");
    };
    assert_eq!(sys.subtype, SystemSubtype::CloudSessionDelta);
    assert_eq!(sys.subtype.as_str(), "cloud_session_delta");

    let delta = sys.as_cloud_session_delta().expect("typed view");
    assert_eq!(delta.seq, 1);
    assert_eq!(delta.changed, vec!["serving"]);
    assert_eq!(delta.cloud_session["serving"]["state"], "on");

    let Some(KnownSystemEvent::CloudSessionDelta(known)) = sys.as_known_system_event() else {
        panic!("expected CloudSessionDelta event");
    };
    assert_eq!(known.cloud_session["id"], "session_abc");
}
