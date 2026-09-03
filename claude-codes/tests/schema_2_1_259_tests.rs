//! Coverage for the CLI 2.1.259 stream-json additive drift.
//!
//! 2.1.239 → 2.1.259 added top-level fields to several wire types without
//! removing any (see the drift report on issue #354). Each frame below carries
//! the new fields and is asserted **fully wrapped** — the typed model captures
//! every wire field with nothing left in an untyped escape hatch.

use claude_codes::{assert_fully_wrapped, ClaudeOutput};
use serde_json::json;

#[test]
fn assistant_carries_user_message_uuids_and_wire_tool_inputs() {
    let frame = json!({
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
        "session_id": "s1",
        "uuid": "u1",
        "parent_tool_use_id": null,
        "user_message_uuid": "um-2",
        "user_message_uuids": ["um-1", "um-2"],
        "wire_tool_inputs": {"toolu_1": {"path": "/tmp/x"}},
        "local_command_source": "<local-command-stdout>hi</local-command-stdout>"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::Assistant(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected assistant");
    };
    assert_eq!(msg.user_message_uuid.as_deref(), Some("um-2"));
    assert_eq!(msg.user_message_uuids, vec!["um-1", "um-2"]);
    assert!(msg.wire_tool_inputs.is_some());
    assert!(msg.local_command_source.is_some());
}

#[test]
fn stream_event_carries_user_message_uuids() {
    let frame = json!({
        "type": "stream_event",
        "event": {"type": "message_start"},
        "parent_tool_use_id": null,
        "uuid": "u1",
        "session_id": "s1",
        "user_message_uuid": "um-1",
        "user_message_uuids": ["um-1"]
    });
    assert_fully_wrapped(&frame);
}

#[test]
fn user_carries_client_composed() {
    let frame = json!({
        "type": "user",
        "message": {"role": "user", "content": []},
        "session_id": "7fbc568e-2bd6-45aa-b217-a1cf80004ba1",
        "client_composed": true
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::User(msg) = serde_json::from_value(frame).unwrap() else {
        panic!("expected user");
    };
    assert_eq!(msg.client_composed, Some(true));
}

#[test]
fn system_init_carries_footer_indicator_worker_epoch_powershell() {
    let frame = json!({
        "type": "system",
        "subtype": "init",
        "session_id": "s1",
        "uuid": "u1",
        "footer_indicator": {"text": "◆ test"},
        "worker_epoch": 7,
        "powershell_path": "C:\\pwsh.exe"
    });
    assert_fully_wrapped(&frame);
}

#[test]
fn system_init_powershell_path_null_round_trips() {
    // Windows-with-no-PowerShell emits an explicit null; the typed model keeps
    // the distinction between `Some(None)` (wire null) and absent.
    let frame = json!({
        "type": "system",
        "subtype": "init",
        "session_id": "s1",
        "uuid": "u1",
        "powershell_path": null
    });
    // A wire null that serializes away is not data loss (see wrap_audit docs).
    assert_fully_wrapped(&frame);
}

#[test]
fn system_task_started_carries_ambient() {
    let frame = json!({
        "type": "system",
        "subtype": "task_started",
        "session_id": "s1",
        "uuid": "u1",
        "task_id": "t1",
        "description": "background watcher",
        "ambient": true
    });
    assert_fully_wrapped(&frame);
}

#[test]
fn system_task_notification_carries_ambient_and_resource_links() {
    let frame = json!({
        "type": "system",
        "subtype": "task_notification",
        "session_id": "s1",
        "uuid": "u1",
        "task_id": "t1",
        "status": "completed",
        "summary": "done",
        "output_file": null,
        "ambient": false,
        "resource_links": [
            {"uri": "file:///tmp/out.txt", "name": "out.txt", "mimeType": "text/plain", "size": 12}
        ]
    });
    assert_fully_wrapped(&frame);
}

#[test]
fn rate_limit_event_carries_unified_windows_and_grace() {
    // Reproduces the live-suite frame that surfaced the nested `unifiedWindows`
    // drop the coarse (top-level-only) drift checker cannot see.
    let frame = json!({
        "type": "rate_limit_event",
        "session_id": "s1",
        "uuid": "u1",
        "rate_limit_info": {
            "status": "allowed",
            "resetsAt": 1788441600u64,
            "rateLimitType": "five_hour",
            "overageStatus": "rejected",
            "overageDisabledReason": "org_level_disabled",
            "isUsingOverage": false,
            "rateLimitGraceActive": true,
            "unifiedWindows": {
                "five_hour": {"resetsAt": 1788441600u64, "utilization": 0.01},
                "seven_day": {"resetsAt": 1788872400u64, "utilization": 0.2}
            }
        }
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::RateLimitEvent(evt) = serde_json::from_value(frame).unwrap() else {
        panic!("expected rate_limit_event");
    };
    let windows = evt.rate_limit_info.unified_windows.unwrap();
    assert_eq!(windows.five_hour.unwrap().utilization, 0.01);
    assert_eq!(windows.seven_day.unwrap().resets_at, 1788872400);
    assert_eq!(evt.rate_limit_info.rate_limit_grace_active, Some(true));
}

#[test]
fn system_code_change_published_carries_branch() {
    let frame = json!({
        "type": "system",
        "subtype": "code_change_published",
        "session_id": "s1",
        "uuid": "u1",
        "provider": "gerrit",
        "url": "https://example-review.googlesource.com/c/1234",
        "repo": "team/project",
        "identifier": "1234",
        "action": "pushed",
        "branch": "meawoppl/feature"
    });
    assert_fully_wrapped(&frame);

    let ClaudeOutput::System(sys) = serde_json::from_value(frame).unwrap() else {
        panic!("expected system");
    };
    let ccp = sys.as_code_change_published().unwrap();
    assert_eq!(ccp.branch.as_deref(), Some("meawoppl/feature"));
}
