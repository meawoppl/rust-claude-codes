use codex_codes::io::items::{
    CommandExecutionStatus, FileChangeItem, PatchApplyStatus, PatchChangeKind, ThreadItem,
};
use codex_codes::protocol::{ConfigReadResponse, GetAccountRateLimitsResponse};
use codex_codes::{
    JsonRpcMessage, JsonRpcNotification, McpServerElicitationRequestParams, Notification,
    ParseError, ThreadEvent, ThreadItemsListResponse, ThreadRevertResponse,
    ThreadTurnsListResponse, TurnSteerResponse,
};

/// config/read round-trips snake_case wire fields and keeps unknown Config/AnalyticsConfig properties in the flatten maps.
#[test]
fn config_read_preserves_additional_properties() {
    let original = serde_json::json!({
        "config": {
            "analytics": {
                "enabled": true,
                "unknownAnalyticsSetting": "kept"
            },
            "sandbox_workspace_write": {
                "network_access": true,
                "writable_roots": ["/tmp/cache"]
            },
            "model_providers": {
                "local": {
                    "base_url": "http://localhost:11434/v1"
                }
            }
        },
        "origins": {}
    });

    let response: ConfigReadResponse =
        serde_json::from_value(original.clone()).expect("deserialize config/read response");

    assert_eq!(
        response.config.additional.get("model_providers"),
        original["config"].get("model_providers")
    );
    assert_eq!(
        response
            .config
            .analytics
            .as_ref()
            .and_then(|analytics| analytics.additional.get("unknownAnalyticsSetting")),
        Some(&serde_json::json!("kept"))
    );
    assert_eq!(
        response
            .config
            .sandbox_workspace_write
            .as_ref()
            .and_then(|sandbox| sandbox.writable_roots.as_deref()),
        Some(["/tmp/cache".to_string()].as_slice())
    );
    assert_eq!(
        serde_json::to_value(response).expect("serialize config/read response"),
        original
    );
}

/// GetAccountRateLimitsResponse decodes accountId and the backend-owned rateLimitUpsell banner, round-tripping both.
#[test]
fn account_rate_limits_include_account_and_upsell() {
    let original = serde_json::json!({
        "accountId": "workspace-123",
        "rateLimits": {},
        "rateLimitUpsell": {
            "message": "Upgrade for more usage"
        }
    });

    let response: GetAccountRateLimitsResponse =
        serde_json::from_value(original.clone()).expect("deserialize rate limits response");

    assert_eq!(response.account_id.as_deref(), Some("workspace-123"));
    assert_eq!(
        response.rate_limit_upsell.as_ref(),
        original.get("rateLimitUpsell")
    );
    assert_eq!(
        serde_json::to_value(response).expect("serialize rate limits response"),
        original
    );
}

/// MCP elicitation requests with the newer camelCase "openaiForm" mode decode to the OpenAiElicitationForm arm.
#[test]
fn mcp_elicitation_accepts_current_openai_form_mode() {
    let request: McpServerElicitationRequestParams = serde_json::from_value(serde_json::json!({
        "mode": "openaiForm",
        "message": "Choose an account",
        "requestedSchema": {"type": "object"}
    }))
    .expect("deserialize openaiForm elicitation");

    assert!(matches!(
        request,
        McpServerElicitationRequestParams::OpenAiElicitationForm { .. }
    ));
}

/// Response types behind the new sync helpers (turn/steer, items/turns list, thread/revert) decode minimal wire shapes.
#[test]
fn sync_helper_response_types_decode_current_wire_shapes() {
    let steer: TurnSteerResponse =
        serde_json::from_value(serde_json::json!({"turnId": "turn-1"})).unwrap();
    let items: ThreadItemsListResponse =
        serde_json::from_value(serde_json::json!({"data": []})).unwrap();
    let turns: ThreadTurnsListResponse =
        serde_json::from_value(serde_json::json!({"data": []})).unwrap();
    let revert: ThreadRevertResponse =
        serde_json::from_value(serde_json::json!({"thread": {}})).unwrap();

    assert_eq!(steer.turn_id, "turn-1");
    assert!(items.data.is_empty());
    assert!(turns.data.is_empty());
    assert_eq!(revert.thread, codex_codes::Thread::default());
}

/// Parse every line from a JSONL capture file into ThreadEvents,
/// panicking on any deserialization failure.
fn parse_capture(jsonl: &str) -> Vec<ThreadEvent> {
    jsonl
        .lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
        .map(|(i, line)| {
            serde_json::from_str::<ThreadEvent>(line)
                .unwrap_or_else(|e| panic!("Failed to parse line {}: {}\n  JSON: {}", i, e, line))
        })
        .collect()
}

/// Extract all ThreadItems from ItemCompleted events.
fn completed_items(events: &[ThreadEvent]) -> Vec<&ThreadItem> {
    events
        .iter()
        .filter_map(|e| match e {
            ThreadEvent::ItemCompleted(ic) => Some(&ic.item),
            _ => None,
        })
        .collect()
}

/// Every capture must start with thread.started and end with turn.completed.
fn assert_standard_envelope(events: &[ThreadEvent]) {
    assert!(
        events.len() >= 3,
        "Expected at least 3 events, got {}",
        events.len()
    );
    assert_eq!(events[0].event_type(), "thread.started");
    assert_eq!(events[1].event_type(), "turn.started");
    assert_eq!(
        events.last().unwrap().event_type(),
        "turn.completed",
        "Last event should be turn.completed"
    );

    // thread.started always carries a thread_id
    if let ThreadEvent::ThreadStarted(e) = &events[0] {
        assert!(!e.thread_id.is_empty(), "thread_id must not be empty");
    }

    // turn.completed always carries usage with non-zero tokens
    if let ThreadEvent::TurnCompleted(e) = events.last().unwrap() {
        assert!(e.usage.input_tokens > 0, "input_tokens should be > 0");
        assert!(e.usage.output_tokens > 0, "output_tokens should be > 0");
    }
}

// ── hello_world: simplest possible session ──────────────────────────

/// Every line of the captured hello-world exec stream parses into a typed event.
#[test]
fn test_hello_world_parses_all_lines() {
    let events = parse_capture(include_str!("../test_cases/captures/hello_world.jsonl"));
    assert_standard_envelope(&events);
    assert_eq!(events.len(), 5);
}

/// The hello-world capture contains both reasoning and agent-message items - the minimal turn vocabulary.
#[test]
fn test_hello_world_contains_reasoning_and_message() {
    let events = parse_capture(include_str!("../test_cases/captures/hello_world.jsonl"));
    let items = completed_items(&events);

    let reasoning_count = items
        .iter()
        .filter(|i| matches!(i, ThreadItem::Reasoning(_)))
        .count();
    let message_count = items
        .iter()
        .filter(|i| matches!(i, ThreadItem::AgentMessage(_)))
        .count();

    assert!(reasoning_count >= 1, "Expected at least one reasoning item");
    assert!(message_count >= 1, "Expected at least one agent message");

    // The final agent message should contain "hello world"
    let last_msg = items
        .iter()
        .rev()
        .find_map(|i| match i {
            ThreadItem::AgentMessage(m) => Some(&m.text),
            _ => None,
        })
        .expect("Should have an agent message");
    assert_eq!(last_msg, "hello world");
}

// ── list_files: command execution with item.started lifecycle ────────

/// Every line of the captured list-files exec stream parses into a typed event.
#[test]
fn test_list_files_parses_all_lines() {
    let events = parse_capture(include_str!("../test_cases/captures/list_files.jsonl"));
    assert_standard_envelope(&events);
    assert_eq!(events.len(), 8);
}

/// The list-files capture shows the full command lifecycle: item.started, command output, item.completed.
#[test]
fn test_list_files_command_lifecycle() {
    let events = parse_capture(include_str!("../test_cases/captures/list_files.jsonl"));

    // Find the item.started / item.completed pair for the command
    let started_cmd = events.iter().find_map(|e| match e {
        ThreadEvent::ItemStarted(is) => match &is.item {
            ThreadItem::CommandExecution(c) => Some(c),
            _ => None,
        },
        _ => None,
    });
    let completed_cmd = completed_items(&events).into_iter().find_map(|i| match i {
        ThreadItem::CommandExecution(c) => Some(c),
        _ => None,
    });

    let started = started_cmd.expect("Should have an item.started command");
    let completed = completed_cmd.expect("Should have an item.completed command");

    // Same item id
    assert_eq!(started.id, completed.id);

    // Started has in_progress status, no exit code, empty output
    assert_eq!(started.status, CommandExecutionStatus::InProgress);
    assert_eq!(started.exit_code, None);
    assert!(started
        .aggregated_output
        .as_deref()
        .unwrap_or("")
        .is_empty());

    // Completed has exit code 0, non-empty output
    assert_eq!(completed.status, CommandExecutionStatus::Completed);
    assert_eq!(completed.exit_code, Some(0));
    assert!(!completed
        .aggregated_output
        .as_deref()
        .unwrap_or("")
        .is_empty());
    assert!(completed.command.contains("ls"));
}

// ── file_create: command that creates a file ────────────────────────

/// Every line of the captured file-create exec stream parses into a typed event.
#[test]
fn test_file_create_parses_all_lines() {
    let events = parse_capture(include_str!("../test_cases/captures/file_create.jsonl"));
    assert_standard_envelope(&events);
    assert_eq!(events.len(), 8);
}

/// The file-create capture carries the command output text on the completed item.
#[test]
fn test_file_create_command_output() {
    let events = parse_capture(include_str!("../test_cases/captures/file_create.jsonl"));
    let cmd = completed_items(&events)
        .into_iter()
        .find_map(|i| match i {
            ThreadItem::CommandExecution(c) => Some(c),
            _ => None,
        })
        .expect("Should have a completed command");

    assert_eq!(cmd.exit_code, Some(0));
    assert_eq!(cmd.aggregated_output.as_deref(), Some("hello from codex"));
}

// ── failed_command: non-zero exit code ──────────────────────────────

/// Every line of the captured failed-command exec stream parses into a typed event.
#[test]
fn test_failed_command_parses_all_lines() {
    let events = parse_capture(include_str!("../test_cases/captures/failed_command.jsonl"));
    assert_standard_envelope(&events);
    assert_eq!(events.len(), 8);
}

/// A failed command reports failed status and its nonzero exit code on the completed item.
#[test]
fn test_failed_command_status_and_exit_code() {
    let events = parse_capture(include_str!("../test_cases/captures/failed_command.jsonl"));

    let started_cmd = events.iter().find_map(|e| match e {
        ThreadEvent::ItemStarted(is) => match &is.item {
            ThreadItem::CommandExecution(c) => Some(c),
            _ => None,
        },
        _ => None,
    });
    let completed_cmd = completed_items(&events).into_iter().find_map(|i| match i {
        ThreadItem::CommandExecution(c) => Some(c),
        _ => None,
    });

    let started = started_cmd.expect("Should have started command");
    let completed = completed_cmd.expect("Should have completed command");

    assert_eq!(started.status, CommandExecutionStatus::InProgress);
    assert_eq!(started.exit_code, None);

    assert_eq!(completed.status, CommandExecutionStatus::Failed);
    assert_eq!(completed.exit_code, Some(42));
    assert!(completed
        .aggregated_output
        .as_deref()
        .unwrap_or("")
        .is_empty());
}

// ── file_change: patch-based file modification ──────────────────────

/// Every line of the captured file-change exec stream parses into a typed event.
#[test]
fn test_file_change_parses_all_lines() {
    let events = parse_capture(include_str!("../test_cases/captures/file_change.jsonl"));
    assert_standard_envelope(&events);
    assert_eq!(events.len(), 12);
}

/// A fileChange item carries the changed paths and change kinds.
#[test]
fn test_file_change_item_fields() {
    let events = parse_capture(include_str!("../test_cases/captures/file_change.jsonl"));

    let fc: &FileChangeItem = completed_items(&events)
        .into_iter()
        .find_map(|i| match i {
            ThreadItem::FileChange(f) => Some(f),
            _ => None,
        })
        .expect("Should have a file_change item");

    assert_eq!(fc.status, PatchApplyStatus::Completed);
    assert_eq!(fc.changes.len(), 1);
    assert!(matches!(fc.changes[0].kind, PatchChangeKind::Update { .. }));
    assert!(fc.changes[0].path.contains("test.txt"));
}

/// The file-change capture also runs a verification command - both item kinds coexist in one turn.
#[test]
fn test_file_change_also_has_command_verification() {
    let events = parse_capture(include_str!("../test_cases/captures/file_change.jsonl"));
    let cmds: Vec<_> = completed_items(&events)
        .into_iter()
        .filter_map(|i| match i {
            ThreadItem::CommandExecution(c) => Some(c),
            _ => None,
        })
        .collect();

    assert!(!cmds.is_empty());
    assert!(cmds.iter().any(|c| c.command.contains("cat")));
    assert!(cmds.iter().any(|c| c
        .aggregated_output
        .as_deref()
        .unwrap_or("")
        .contains("new content")));
}

// ── multi_command: multiple sequential commands ─────────────────────

/// Every line of the captured multi-command exec stream parses into a typed event.
#[test]
fn test_multi_command_parses_all_lines() {
    let events = parse_capture(include_str!("../test_cases/captures/multi_command.jsonl"));
    assert_standard_envelope(&events);
    assert_eq!(events.len(), 12);
}

/// The multi-command capture executed all three requested commands.
#[test]
fn test_multi_command_three_commands_executed() {
    let events = parse_capture(include_str!("../test_cases/captures/multi_command.jsonl"));

    let cmds: Vec<_> = completed_items(&events)
        .into_iter()
        .filter_map(|i| match i {
            ThreadItem::CommandExecution(c) => Some(c),
            _ => None,
        })
        .collect();

    assert_eq!(cmds.len(), 3, "Expected exactly 3 completed commands");

    for (i, cmd) in cmds.iter().enumerate() {
        let step = format!("step{}", i + 1);
        assert!(
            cmd.command.contains(&step),
            "Command {} should contain '{}'",
            i,
            step
        );
        assert_eq!(cmd.exit_code, Some(0));
        assert_eq!(cmd.status, CommandExecutionStatus::Completed);
        assert!(
            cmd.aggregated_output
                .as_deref()
                .unwrap_or("")
                .contains(&step),
            "Output of command {} should contain '{}'",
            i,
            step
        );
    }
}

/// Every item.started in the multi-command capture has a matching item.completed - no orphaned lifecycle events.
#[test]
fn test_multi_command_started_events_match_completed() {
    let events = parse_capture(include_str!("../test_cases/captures/multi_command.jsonl"));

    let started_ids: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            ThreadEvent::ItemStarted(is) => match &is.item {
                ThreadItem::CommandExecution(c) => Some(c.id.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect();

    let completed_ids: Vec<_> = completed_items(&events)
        .into_iter()
        .filter_map(|i| match i {
            ThreadItem::CommandExecution(c) => Some(c.id.clone()),
            _ => None,
        })
        .collect();

    assert_eq!(started_ids.len(), 3);
    assert_eq!(completed_ids.len(), 3);
    assert_eq!(
        started_ids, completed_ids,
        "Every started command should have a matching completed event"
    );
}

// ── cross-capture: verify all captures share structural invariants ──

/// Thread ids are unique across all captures - the identity guarantee downstream keying relies on.
#[test]
fn test_all_captures_have_unique_thread_ids() {
    let captures = [
        include_str!("../test_cases/captures/hello_world.jsonl"),
        include_str!("../test_cases/captures/list_files.jsonl"),
        include_str!("../test_cases/captures/file_create.jsonl"),
        include_str!("../test_cases/captures/failed_command.jsonl"),
        include_str!("../test_cases/captures/file_change.jsonl"),
        include_str!("../test_cases/captures/multi_command.jsonl"),
    ];

    let thread_ids: Vec<String> = captures
        .iter()
        .map(|c| {
            let events = parse_capture(c);
            match &events[0] {
                ThreadEvent::ThreadStarted(e) => e.thread_id.clone(),
                _ => panic!("First event should be ThreadStarted"),
            }
        })
        .collect();

    let mut deduped = thread_ids.clone();
    deduped.sort();
    deduped.dedup();
    assert_eq!(
        thread_ids.len(),
        deduped.len(),
        "All captures should have unique thread IDs"
    );
}

/// Every capture reports cached-token usage - the accounting field consumers bill against.
#[test]
fn test_all_captures_have_cached_tokens() {
    let captures = [
        include_str!("../test_cases/captures/hello_world.jsonl"),
        include_str!("../test_cases/captures/list_files.jsonl"),
        include_str!("../test_cases/captures/file_create.jsonl"),
        include_str!("../test_cases/captures/failed_command.jsonl"),
        include_str!("../test_cases/captures/file_change.jsonl"),
        include_str!("../test_cases/captures/multi_command.jsonl"),
    ];

    for (i, capture) in captures.iter().enumerate() {
        let events = parse_capture(capture);
        if let ThreadEvent::TurnCompleted(tc) = events.last().unwrap() {
            assert!(
                tc.usage.cached_input_tokens > 0,
                "Capture {} should have cached_input_tokens > 0",
                i
            );
        }
    }
}

/// Item ids increase sequentially within a capture - the ordering guarantee for item lists.
#[test]
fn test_all_item_ids_are_sequential_within_capture() {
    let captures = [
        include_str!("../test_cases/captures/hello_world.jsonl"),
        include_str!("../test_cases/captures/list_files.jsonl"),
        include_str!("../test_cases/captures/file_create.jsonl"),
        include_str!("../test_cases/captures/failed_command.jsonl"),
        include_str!("../test_cases/captures/file_change.jsonl"),
        include_str!("../test_cases/captures/multi_command.jsonl"),
    ];

    for capture in &captures {
        let events = parse_capture(capture);
        let ids: Vec<String> = completed_items(&events)
            .into_iter()
            .map(|item| match item {
                ThreadItem::UserMessage(u) => u.id.clone(),
                ThreadItem::AgentMessage(m) => m.id.clone(),
                ThreadItem::Reasoning(r) => r.id.clone(),
                ThreadItem::CommandExecution(c) => c.id.clone(),
                ThreadItem::FileChange(f) => f.id.clone(),
                ThreadItem::McpToolCall(m) => m.id.clone(),
                ThreadItem::WebSearch(w) => w.id.clone(),
                ThreadItem::TodoList(t) => t.id.clone(),
                ThreadItem::Error(e) => e.id.clone(),
            })
            .collect();

        // IDs follow the pattern "item_N" with increasing N
        let mut seen: Vec<usize> = Vec::new();
        for id in &ids {
            assert!(
                id.starts_with("item_"),
                "ID '{}' should start with 'item_'",
                id
            );
            let n: usize = id[5..]
                .parse()
                .unwrap_or_else(|_| panic!("ID '{}' should have a numeric suffix", id));
            if !seen.contains(&n) {
                seen.push(n);
            }
        }

        for window in seen.windows(2) {
            assert!(
                window[1] > window[0],
                "Item IDs should be monotonically increasing, got {} after {}",
                window[1],
                window[0]
            );
        }
    }
}

// ── typed-decode failure plumbing (issue #128) ───────────────────────────
//
// When a wire frame's envelope parses but the typed-payload decode fails,
// callers must still be able to recover the original `method` + `params` for
// bug reports. These tests pin the exact code path used by
// `AsyncClient::next_message` / `SyncClient::next_message`: deserialize the
// line as a `JsonRpcMessage`, run `Notification::from_envelope` /
// `ServerRequest::from_envelope`, wrap the resulting `serde_json::Error` in a
// `ParseError`, and check that nothing was dropped on the floor.

/// An unmodeled notification surfaces as a parse error carrying its method and params - named, not silently dropped.
#[test]
fn parse_error_carries_method_and_params_for_unmodeled_notification_variant() {
    // Simulates a notification whose envelope is fine but whose params don't
    // match any modeled variant — e.g. a future `FileUpdateChange.type` value
    // the crate version doesn't yet know about.
    //
    // The crate uses an `Unknown { method, params }` fallback for unmodeled
    // *methods*, so to actually trigger a typed-decode failure on a *known*
    // method we send malformed params for it.
    let line = r#"{"method":"item/completed","params":{"item":42}}"#;
    let envelope: JsonRpcMessage = serde_json::from_str(line).expect("envelope parses");
    let JsonRpcMessage::Notification(JsonRpcNotification { method, params }) = envelope else {
        panic!("expected Notification, got: {:?}", envelope);
    };

    let err = Notification::from_envelope(&method, params.clone())
        .expect_err("malformed params must fail typed decode");
    let pe = ParseError::from_envelope(method.clone(), params.clone(), err);

    assert_eq!(pe.method.as_deref(), Some("item/completed"));
    assert_eq!(pe.raw_json, params);
    assert!(
        !pe.error_message.is_empty(),
        "error_message should be populated"
    );
    // raw_line is a wire-equivalent reconstruction, parseable back into an envelope.
    let echoed: JsonRpcMessage = serde_json::from_str(&pe.raw_line)
        .unwrap_or_else(|e| panic!("raw_line should re-parse as JsonRpcMessage: {}", e));
    if let JsonRpcMessage::Notification(n) = echoed {
        assert_eq!(n.method, "item/completed");
        assert_eq!(n.params.unwrap()["item"], 42);
    } else {
        panic!("raw_line should re-parse as a Notification");
    }
}

/// A malformed envelope keeps the raw line in the parse error so the bad wire bytes are recoverable.
#[test]
fn parse_error_from_invalid_envelope_keeps_raw_line_without_method() {
    // The bare-JSON failure path: line is not a valid JsonRpcMessage.
    let line = r#"{"completely":"unexpected"}"#;
    let err = serde_json::from_str::<JsonRpcMessage>(line).expect_err("should not parse");
    let pe = ParseError::from_line(line, err);
    assert!(pe.method.is_none());
    assert_eq!(pe.raw_line, line);
    // raw_json is populated because the line was valid JSON, just not a JsonRpcMessage.
    assert_eq!(pe.raw_json.as_ref().unwrap()["completely"], "unexpected");
}
