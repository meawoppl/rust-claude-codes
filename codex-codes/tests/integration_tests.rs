use codex_codes::{
    CommandExecutionStatus, FileChangeItem, PatchApplyStatus, PatchChangeKind, ThreadEvent,
    ThreadItem,
};

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

#[test]
fn test_hello_world_parses_all_lines() {
    let events = parse_capture(include_str!("../test_cases/captures/hello_world.jsonl"));
    assert_standard_envelope(&events);
    assert_eq!(events.len(), 5);
}

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

#[test]
fn test_list_files_parses_all_lines() {
    let events = parse_capture(include_str!("../test_cases/captures/list_files.jsonl"));
    assert_standard_envelope(&events);
    assert_eq!(events.len(), 8);
}

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
    assert!(started.aggregated_output.is_empty());

    // Completed has exit code 0, non-empty output
    assert_eq!(completed.status, CommandExecutionStatus::Completed);
    assert_eq!(completed.exit_code, Some(0));
    assert!(!completed.aggregated_output.is_empty());
    assert!(completed.command.contains("ls"));
}

// ── file_create: command that creates a file ────────────────────────

#[test]
fn test_file_create_parses_all_lines() {
    let events = parse_capture(include_str!("../test_cases/captures/file_create.jsonl"));
    assert_standard_envelope(&events);
    assert_eq!(events.len(), 8);
}

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
    assert_eq!(cmd.aggregated_output, "hello from codex");
}

// ── failed_command: non-zero exit code ──────────────────────────────

#[test]
fn test_failed_command_parses_all_lines() {
    let events = parse_capture(include_str!("../test_cases/captures/failed_command.jsonl"));
    assert_standard_envelope(&events);
    assert_eq!(events.len(), 8);
}

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
    assert!(completed.aggregated_output.is_empty());
}

// ── file_change: patch-based file modification ──────────────────────

#[test]
fn test_file_change_parses_all_lines() {
    let events = parse_capture(include_str!("../test_cases/captures/file_change.jsonl"));
    assert_standard_envelope(&events);
    assert_eq!(events.len(), 12);
}

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
    assert_eq!(fc.changes[0].kind, PatchChangeKind::Update);
    assert!(fc.changes[0].path.contains("test.txt"));
}

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
    assert!(cmds
        .iter()
        .any(|c| c.aggregated_output.contains("new content")));
}

// ── multi_command: multiple sequential commands ─────────────────────

#[test]
fn test_multi_command_parses_all_lines() {
    let events = parse_capture(include_str!("../test_cases/captures/multi_command.jsonl"));
    assert_standard_envelope(&events);
    assert_eq!(events.len(), 12);
}

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
            cmd.aggregated_output.contains(&step),
            "Output of command {} should contain '{}'",
            i,
            step
        );
    }
}

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
