//! Integration tests for Codex app-server interactions.
//!
//! These tests require a real Codex CLI installation and are only run
//! when the `integration-tests` feature is enabled.
//!
//! Run with: `cargo test -p codex-codes --features integration-tests --test live_client_tests`

#![cfg(feature = "integration-tests")]

use codex_codes::protocol::methods;
use codex_codes::{
    AsyncClient, ClientInfo, InitializeCapabilities, InitializeParams, ServerMessage, SyncClient,
    ThreadStartParams, TurnStartParams, UserInput,
};

// ── Version check ───────────────────────────────────────────────────

#[tokio::test]
async fn test_codex_cli_version() {
    codex_codes::version::check_codex_version_async()
        .await
        .expect("Failed to check Codex CLI version");
}

// ── Async client: initialize + thread_start ─────────────────────────

#[tokio::test]
async fn test_async_client_start_and_thread_start() {
    let mut client = AsyncClient::start()
        .await
        .expect("Failed to start app-server");

    let thread = client
        .thread_start(&ThreadStartParams::default())
        .await
        .expect("Failed to start thread");

    assert!(
        !thread.thread_id().is_empty(),
        "thread_id must not be empty"
    );

    client.shutdown().await.expect("Failed to shutdown");
}

// ── Async client: full turn lifecycle ───────────────────────────────

#[tokio::test]
async fn test_async_client_basic_turn() {
    let mut client = AsyncClient::start()
        .await
        .expect("Failed to start app-server");

    let thread = client
        .thread_start(&ThreadStartParams::default())
        .await
        .expect("Failed to start thread");

    client
        .turn_start(&TurnStartParams {
            thread_id: thread.thread_id().to_string(),
            input: vec![UserInput::Text {
                text: "What is 2 + 2? Reply with just the number.".to_string(),
            }],
            model: None,
            reasoning_effort: None,
            sandbox_policy: None,
        })
        .await
        .expect("Failed to start turn");

    let mut found_answer = false;
    let mut turn_completed = false;
    let mut message_count = 0;

    while let Some(msg) = client.next_message().await.expect("Failed to read message") {
        message_count += 1;

        match &msg {
            ServerMessage::Notification { method, params } => {
                if method == methods::AGENT_MESSAGE_DELTA {
                    if let Some(params) = params {
                        if let Some(delta) = params.get("delta").and_then(|d| d.as_str()) {
                            if delta.contains('4') {
                                found_answer = true;
                            }
                        }
                    }
                }
                if method == methods::TURN_COMPLETED {
                    turn_completed = true;
                    break;
                }
            }
            ServerMessage::Request { id, .. } => {
                // Auto-accept any approval requests
                client
                    .respond(id.clone(), &serde_json::json!({"decision": "accept"}))
                    .await
                    .expect("Failed to respond");
            }
        }

        if message_count > 100 {
            break;
        }
    }

    assert!(turn_completed, "Turn should have completed");
    assert!(found_answer, "Response should contain '4'");

    client.shutdown().await.expect("Failed to shutdown");
}

// ── Async client: custom initialization ─────────────────────────────

#[tokio::test]
async fn test_async_client_custom_initialize() {
    use codex_codes::AppServerBuilder;

    let mut client = AsyncClient::spawn(AppServerBuilder::new())
        .await
        .expect("Failed to spawn app-server");

    let resp = client
        .initialize(&InitializeParams {
            client_info: ClientInfo {
                name: "codex-codes-test".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: Some("Integration Test".to_string()),
            },
            capabilities: Some(InitializeCapabilities {
                experimental_api: false,
                opt_out_notification_methods: None,
            }),
        })
        .await
        .expect("Failed to initialize");

    assert!(
        !resp.user_agent.is_empty(),
        "user_agent should not be empty"
    );

    // Verify we can use the client after custom initialization
    let thread = client
        .thread_start(&ThreadStartParams::default())
        .await
        .expect("Failed to start thread after custom init");

    assert!(!thread.thread_id().is_empty());

    client.shutdown().await.expect("Failed to shutdown");
}

// ── Sync client: initialize + thread_start ──────────────────────────

#[test]
fn test_sync_client_start_and_thread_start() {
    let mut client = SyncClient::start().expect("Failed to start app-server");

    let thread = client
        .thread_start(&ThreadStartParams::default())
        .expect("Failed to start thread");

    assert!(
        !thread.thread_id().is_empty(),
        "thread_id must not be empty"
    );
}

// ── Sync client: full turn lifecycle ────────────────────────────────

#[test]
fn test_sync_client_basic_turn() {
    let mut client = SyncClient::start().expect("Failed to start app-server");

    let thread = client
        .thread_start(&ThreadStartParams::default())
        .expect("Failed to start thread");

    client
        .turn_start(&TurnStartParams {
            thread_id: thread.thread_id().to_string(),
            input: vec![UserInput::Text {
                text: "What is 2 + 2? Reply with just the number.".to_string(),
            }],
            model: None,
            reasoning_effort: None,
            sandbox_policy: None,
        })
        .expect("Failed to start turn");

    let mut found_answer = false;
    let mut turn_completed = false;
    let mut message_count = 0;

    for result in client.events() {
        let msg = result.expect("Failed to read message");
        message_count += 1;

        match &msg {
            ServerMessage::Notification { method, params } => {
                if method == methods::AGENT_MESSAGE_DELTA {
                    if let Some(params) = params {
                        if let Some(delta) = params.get("delta").and_then(|d| d.as_str()) {
                            if delta.contains('4') {
                                found_answer = true;
                            }
                        }
                    }
                }
                if method == methods::TURN_COMPLETED {
                    turn_completed = true;
                    break;
                }
            }
            ServerMessage::Request { .. } => {}
        }

        if message_count > 100 {
            break;
        }
    }

    assert!(turn_completed, "Turn should have completed");
    assert!(found_answer, "Response should contain '4'");
}

// ── Async client: multi-turn conversation ───────────────────────────

#[tokio::test]
async fn test_async_client_multi_turn() {
    let mut client = AsyncClient::start()
        .await
        .expect("Failed to start app-server");

    let thread = client
        .thread_start(&ThreadStartParams::default())
        .await
        .expect("Failed to start thread");

    // First turn: establish context
    client
        .turn_start(&TurnStartParams {
            thread_id: thread.thread_id().to_string(),
            input: vec![UserInput::Text {
                text: "Remember the number 42. Just say OK.".to_string(),
            }],
            model: None,
            reasoning_effort: None,
            sandbox_policy: None,
        })
        .await
        .expect("Failed to start first turn");

    // Drain until turn completes
    let mut message_count = 0;
    while let Some(msg) = client.next_message().await.expect("read") {
        message_count += 1;
        if let ServerMessage::Notification { method, .. } = &msg {
            if method == methods::TURN_COMPLETED {
                break;
            }
        }
        if let ServerMessage::Request { id, .. } = msg {
            client
                .respond(id, &serde_json::json!({"decision": "accept"}))
                .await
                .ok();
        }
        if message_count > 100 {
            break;
        }
    }

    // Second turn: check context is maintained
    client
        .turn_start(&TurnStartParams {
            thread_id: thread.thread_id().to_string(),
            input: vec![UserInput::Text {
                text: "What number did I ask you to remember? Reply with just the number."
                    .to_string(),
            }],
            model: None,
            reasoning_effort: None,
            sandbox_policy: None,
        })
        .await
        .expect("Failed to start second turn");

    let mut found_42 = false;
    let mut message_count = 0;
    while let Some(msg) = client.next_message().await.expect("read") {
        message_count += 1;
        match &msg {
            ServerMessage::Notification { method, params } => {
                if method == methods::AGENT_MESSAGE_DELTA {
                    if let Some(params) = params {
                        if let Some(delta) = params.get("delta").and_then(|d| d.as_str()) {
                            if delta.contains("42") {
                                found_42 = true;
                            }
                        }
                    }
                }
                if method == methods::TURN_COMPLETED {
                    break;
                }
            }
            ServerMessage::Request { id, .. } => {
                client
                    .respond(id.clone(), &serde_json::json!({"decision": "accept"}))
                    .await
                    .ok();
            }
        }
        if message_count > 100 {
            break;
        }
    }

    assert!(found_42, "Agent should remember 42 from the first turn");

    client.shutdown().await.expect("Failed to shutdown");
}

// ── Async client: event stream API ──────────────────────────────────

#[tokio::test]
async fn test_async_client_event_stream() {
    let mut client = AsyncClient::start()
        .await
        .expect("Failed to start app-server");

    let thread = client
        .thread_start(&ThreadStartParams::default())
        .await
        .expect("Failed to start thread");

    client
        .turn_start(&TurnStartParams {
            thread_id: thread.thread_id().to_string(),
            input: vec![UserInput::Text {
                text: "Say hello.".to_string(),
            }],
            model: None,
            reasoning_effort: None,
            sandbox_policy: None,
        })
        .await
        .expect("Failed to start turn");

    let mut stream = client.events();
    let mut got_turn_started = false;
    let mut got_turn_completed = false;
    let mut message_count = 0;

    while let Some(result) = stream.next().await {
        let msg = result.expect("Failed to read event");
        message_count += 1;

        if let ServerMessage::Notification { method, .. } = &msg {
            if method == methods::TURN_STARTED {
                got_turn_started = true;
            }
            if method == methods::TURN_COMPLETED {
                got_turn_completed = true;
                break;
            }
        }

        if message_count > 100 {
            break;
        }
    }

    assert!(got_turn_started, "Should have received turn/started");
    assert!(got_turn_completed, "Should have received turn/completed");
}
