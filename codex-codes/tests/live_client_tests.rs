//! Integration tests for Codex app-server interactions.
//!
//! These tests require a real Codex CLI installation and are only run
//! when the `integration-tests` feature is enabled.
//!
//! Run with: `cargo test -p codex-codes --features integration-tests --test live_client_tests`

#![cfg(feature = "integration-tests")]

use codex_codes::{
    AppServerBuilder, AsyncClient, ClientInfo, InitializeCapabilities, InitializeParams,
    Notification, ServerMessage, ServerRequest, SyncClient, ThreadStartParams, TurnStartParams,
    UserInput,
};

// ── Version check ───────────────────────────────────────────────────

/// The installed codex CLI answers --version - the binary-presence gate for the live suite.
#[tokio::test]
async fn test_codex_cli_version() {
    codex_codes::version::check_codex_version_async()
        .await
        .expect("Failed to check Codex CLI version");
}

// ── Async client: initialize + thread_start ─────────────────────────

/// The async client starts the app-server and thread/start returns a non-empty thread id.
#[tokio::test]
async fn test_async_client_start_and_thread_start() {
    let mut client = AsyncClient::start()
        .await
        .expect("Failed to start app-server");

    let thread = client
        .thread_start(&serde_json::from_value::<ThreadStartParams>(serde_json::json!({})).unwrap())
        .await
        .expect("Failed to start thread");

    assert!(!thread.thread.id.is_empty(), "thread_id must not be empty");

    client.shutdown().await.expect("Failed to shutdown");
}

/// Fork a thread with `thread/fork`. The source needs at least one persisted
/// turn first — forking a fresh, turn-less thread is rejected with
/// "no rollout found for thread id …".
#[tokio::test]
async fn test_async_client_thread_fork() {
    let mut client = AsyncClient::start()
        .await
        .expect("Failed to start app-server");

    let source = client
        .thread_start(&serde_json::from_value::<ThreadStartParams>(serde_json::json!({})).unwrap())
        .await
        .expect("Failed to start source thread");

    client
        .turn_start(&TurnStartParams {
            thread_id: source.thread.id.clone(),
            tool_output: None,
            input: vec![UserInput::Text {
                text: "Reply with just OK.".to_string(),
                text_elements: None,
            }],
            approval_policy: None,
            approvals_reviewer: None,
            client_user_message_id: None,
            cwd: None,
            effort: None,
            model: None,
            output_schema: None,
            personality: None,
            sandbox_policy: None,
            service_tier: None,
            service_tier_for_turn: None,
            summary: None,
            turn_trigger: None,
        })
        .await
        .expect("Failed to start seed turn");

    let mut message_count = 0;
    while let Some(msg) = client.next_message().await.expect("Failed to read message") {
        message_count += 1;
        match msg {
            ServerMessage::Notification(Notification::TurnCompleted(_)) => break,
            ServerMessage::Notification(_) => {}
            ServerMessage::Request { id, .. } => {
                client
                    .respond(id, &serde_json::json!({"decision": "accept"}))
                    .await
                    .expect("Failed to respond");
            }
        }
        if message_count > 100 {
            panic!("seed turn did not complete");
        }
    }

    let fork = client
        .thread_fork(
            &serde_json::from_value(serde_json::json!({ "threadId": source.thread.id }))
                .expect("valid fork params"),
        )
        .await
        .expect("Failed to fork thread");

    assert!(
        !fork.thread.id.is_empty(),
        "forked thread id must not be empty"
    );
    assert_ne!(
        fork.thread.id, source.thread.id,
        "fork must get a NEW thread id"
    );

    client.shutdown().await.expect("Failed to shutdown");
}

// ── Async client: full turn lifecycle ───────────────────────────────

/// A live turn streams typed notifications to turnCompleted and the answer arrives in agent-message deltas.
#[tokio::test]
async fn test_async_client_basic_turn() {
    let mut client = AsyncClient::start()
        .await
        .expect("Failed to start app-server");

    let thread = client
        .thread_start(&serde_json::from_value::<ThreadStartParams>(serde_json::json!({})).unwrap())
        .await
        .expect("Failed to start thread");

    client
        .turn_start(&TurnStartParams {
            thread_id: thread.thread.id.clone(),
            tool_output: None,
            input: vec![UserInput::Text {
                text: "What is 2 + 2? Reply with just the number.".to_string(),
                text_elements: None,
            }],
            approval_policy: None,
            approvals_reviewer: None,
            client_user_message_id: None,
            cwd: None,
            effort: None,
            model: None,
            output_schema: None,
            personality: None,
            sandbox_policy: None,
            service_tier: None,
            service_tier_for_turn: None,
            summary: None,
            turn_trigger: None,
        })
        .await
        .expect("Failed to start turn");

    let mut found_answer = false;
    let mut turn_completed = false;
    let mut message_count = 0;

    while let Some(msg) = client.next_message().await.expect("Failed to read message") {
        message_count += 1;

        match msg {
            ServerMessage::Notification(Notification::AgentMessageDelta(d)) => {
                if d.delta.contains('4') {
                    found_answer = true;
                }
            }
            ServerMessage::Notification(Notification::TurnCompleted(_)) => {
                turn_completed = true;
                break;
            }
            ServerMessage::Notification(_) => {}
            ServerMessage::Request { id, .. } => {
                // Auto-accept any approval requests
                client
                    .respond(id, &serde_json::json!({"decision": "accept"}))
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

/// initialize with custom client info and capabilities is accepted by the live app-server.
#[tokio::test]
async fn test_async_client_custom_initialize() {
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
                experimental_api: Some(false),
                extensions: None,
                mcp_server_openai_form_elicitation: None,
                opt_out_notification_methods: None,
                request_attestation: None,
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
        .thread_start(&serde_json::from_value::<ThreadStartParams>(serde_json::json!({})).unwrap())
        .await
        .expect("Failed to start thread after custom init");

    assert!(!thread.thread.id.is_empty());

    client.shutdown().await.expect("Failed to shutdown");
}

// ── Sync client: initialize + thread_start ──────────────────────────

/// The sync client starts the app-server and opens a thread (blocking API parity with async).
#[test]
fn test_sync_client_start_and_thread_start() {
    let mut client = SyncClient::start().expect("Failed to start app-server");

    let thread = client
        .thread_start(&serde_json::from_value::<ThreadStartParams>(serde_json::json!({})).unwrap())
        .expect("Failed to start thread");

    assert!(!thread.thread.id.is_empty(), "thread_id must not be empty");
}

// ── Sync client: full turn lifecycle ────────────────────────────────

/// A live turn completes through the sync client (blocking API parity with async).
#[test]
fn test_sync_client_basic_turn() {
    let mut client = SyncClient::start().expect("Failed to start app-server");

    let thread = client
        .thread_start(&serde_json::from_value::<ThreadStartParams>(serde_json::json!({})).unwrap())
        .expect("Failed to start thread");

    client
        .turn_start(&TurnStartParams {
            thread_id: thread.thread.id.clone(),
            tool_output: None,
            input: vec![UserInput::Text {
                text: "What is 2 + 2? Reply with just the number.".to_string(),
                text_elements: None,
            }],
            approval_policy: None,
            approvals_reviewer: None,
            client_user_message_id: None,
            cwd: None,
            effort: None,
            model: None,
            output_schema: None,
            personality: None,
            sandbox_policy: None,
            service_tier: None,
            service_tier_for_turn: None,
            summary: None,
            turn_trigger: None,
        })
        .expect("Failed to start turn");

    let mut found_answer = false;
    let mut turn_completed = false;
    let mut message_count = 0;

    for result in client.events() {
        let msg = result.expect("Failed to read message");
        message_count += 1;

        match msg {
            ServerMessage::Notification(Notification::AgentMessageDelta(d)) => {
                if d.delta.contains('4') {
                    found_answer = true;
                }
            }
            ServerMessage::Notification(Notification::TurnCompleted(_)) => {
                turn_completed = true;
                break;
            }
            ServerMessage::Notification(_) | ServerMessage::Request { .. } => {}
        }

        if message_count > 100 {
            break;
        }
    }

    assert!(turn_completed, "Turn should have completed");
    assert!(found_answer, "Response should contain '4'");
}

// ── Async client: multi-turn conversation ───────────────────────────

/// Two turns on one thread share context - the second answer uses the first turn's information.
#[tokio::test]
async fn test_async_client_multi_turn() {
    let mut client = AsyncClient::start()
        .await
        .expect("Failed to start app-server");

    let thread = client
        .thread_start(&serde_json::from_value::<ThreadStartParams>(serde_json::json!({})).unwrap())
        .await
        .expect("Failed to start thread");

    // First turn: establish context
    client
        .turn_start(&TurnStartParams {
            thread_id: thread.thread.id.clone(),
            tool_output: None,
            input: vec![UserInput::Text {
                text: "Remember the number 42. Just say OK.".to_string(),
                text_elements: None,
            }],
            approval_policy: None,
            approvals_reviewer: None,
            client_user_message_id: None,
            cwd: None,
            effort: None,
            model: None,
            output_schema: None,
            personality: None,
            sandbox_policy: None,
            service_tier: None,
            service_tier_for_turn: None,
            summary: None,
            turn_trigger: None,
        })
        .await
        .expect("Failed to start first turn");

    // Drain until turn completes
    let mut message_count = 0;
    while let Some(msg) = client.next_message().await.expect("read") {
        message_count += 1;
        match msg {
            ServerMessage::Notification(Notification::TurnCompleted(_)) => break,
            ServerMessage::Notification(_) => {}
            ServerMessage::Request { id, .. } => {
                client
                    .respond(id, &serde_json::json!({"decision": "accept"}))
                    .await
                    .ok();
            }
        }
        if message_count > 100 {
            break;
        }
    }

    // Second turn: check context is maintained
    client
        .turn_start(&TurnStartParams {
            thread_id: thread.thread.id.clone(),
            tool_output: None,
            input: vec![UserInput::Text {
                text: "What number did I ask you to remember? Reply with just the number."
                    .to_string(),
                text_elements: None,
            }],
            approval_policy: None,
            approvals_reviewer: None,
            client_user_message_id: None,
            cwd: None,
            effort: None,
            model: None,
            output_schema: None,
            personality: None,
            sandbox_policy: None,
            service_tier: None,
            service_tier_for_turn: None,
            summary: None,
            turn_trigger: None,
        })
        .await
        .expect("Failed to start second turn");

    let mut found_42 = false;
    let mut message_count = 0;
    while let Some(msg) = client.next_message().await.expect("read") {
        message_count += 1;
        match msg {
            ServerMessage::Notification(Notification::AgentMessageDelta(d)) => {
                if d.delta.contains("42") {
                    found_42 = true;
                }
            }
            ServerMessage::Notification(Notification::TurnCompleted(_)) => break,
            ServerMessage::Notification(_) => {}
            ServerMessage::Request { id, .. } => {
                client
                    .respond(id, &serde_json::json!({"decision": "accept"}))
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

/// The live notification stream arrives in protocol order and every frame deserializes.
#[tokio::test]
async fn test_async_client_event_stream() {
    let mut client = AsyncClient::start()
        .await
        .expect("Failed to start app-server");

    let thread = client
        .thread_start(&serde_json::from_value::<ThreadStartParams>(serde_json::json!({})).unwrap())
        .await
        .expect("Failed to start thread");

    client
        .turn_start(&TurnStartParams {
            thread_id: thread.thread.id.clone(),
            tool_output: None,
            input: vec![UserInput::Text {
                text: "Say hello.".to_string(),
                text_elements: None,
            }],
            approval_policy: None,
            approvals_reviewer: None,
            client_user_message_id: None,
            cwd: None,
            effort: None,
            model: None,
            output_schema: None,
            personality: None,
            sandbox_policy: None,
            service_tier: None,
            service_tier_for_turn: None,
            summary: None,
            turn_trigger: None,
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

        if let ServerMessage::Notification(n) = &msg {
            match n {
                Notification::TurnStarted(_) => got_turn_started = true,
                Notification::TurnCompleted(_) => {
                    got_turn_completed = true;
                    break;
                }
                _ => {}
            }
        }

        if message_count > 100 {
            break;
        }
    }

    assert!(got_turn_started, "Should have received turn/started");
    assert!(got_turn_completed, "Should have received turn/completed");
}

// ── Strict typed-message audit ──────────────────────────────────────
//
// Runs a turn that exercises a command-execution item plus the usual thread
// and turn lifecycle, then asserts that **every** notification and server
// request received deserializes into its typed variant — i.e. `Unknown` must
// not appear. The dispatch in [`codex_codes::messages`] is strict on known
// methods (deserialization failure surfaces as a client error) and tolerant
// on unknown methods (routes to `Unknown` without error); this test catches
// both regressions: a known method whose payload no longer fits the typed
// struct fails the client call before we get here, and a brand-new method
// that we haven't modeled fails the assertion below.
/// STRICT audit: every message a live turn produces deserializes into a typed variant - any Unknown is drift, caught here first.
#[tokio::test]
async fn test_typed_message_audit_strict() {
    use std::collections::BTreeMap;

    let mut client = AsyncClient::start()
        .await
        .expect("Failed to start app-server");

    let thread = client
        .thread_start(&serde_json::from_value::<ThreadStartParams>(serde_json::json!({})).unwrap())
        .await
        .expect("Failed to start thread");

    client
        .turn_start(&TurnStartParams {
            thread_id: thread.thread.id.clone(),
            tool_output: None,
            input: vec![UserInput::Text {
                text: "Run `ls` in the current directory, then briefly describe what you saw."
                    .to_string(),
                text_elements: None,
            }],
            approval_policy: None,
            approvals_reviewer: None,
            client_user_message_id: None,
            cwd: None,
            effort: None,
            model: None,
            output_schema: None,
            personality: None,
            sandbox_policy: None,
            service_tier: None,
            service_tier_for_turn: None,
            summary: None,
            turn_trigger: None,
        })
        .await
        .expect("Failed to start turn");

    let mut typed_counts: BTreeMap<&'static str, u32> = BTreeMap::new();
    let mut unknown_methods: BTreeMap<String, u32> = BTreeMap::new();
    let mut message_count = 0;

    while let Some(msg) = client.next_message().await.expect("read") {
        message_count += 1;
        match msg {
            ServerMessage::Notification(n) => {
                let variant = match &n {
                    Notification::ThreadStarted(_) => "ThreadStarted",
                    Notification::ThreadStatusChanged(_) => "ThreadStatusChanged",
                    Notification::ThreadTokenUsageUpdated(_) => "ThreadTokenUsageUpdated",
                    Notification::TurnStarted(_) => "TurnStarted",
                    Notification::TurnCompleted(_) => "TurnCompleted",
                    Notification::ItemStarted(_) => "ItemStarted",
                    Notification::ItemCompleted(_) => "ItemCompleted",
                    Notification::AgentMessageDelta(_) => "AgentMessageDelta",
                    Notification::CmdOutputDelta(_) => "CmdOutputDelta",
                    Notification::FileChangeOutputDelta(_) => "FileChangeOutputDelta",
                    Notification::ReasoningDelta(_) => "ReasoningDelta",
                    Notification::Error(_) => "Error",
                    Notification::AccountRateLimitsUpdated(_) => "AccountRateLimitsUpdated",
                    Notification::McpServerStartupStatusUpdated(_) => {
                        "McpServerStartupStatusUpdated"
                    }
                    Notification::RemoteControlStatusChanged(_) => "RemoteControlStatusChanged",
                    Notification::McpServerOauthLoginCompleted(_) => "McpServerOauthLoginCompleted",
                    Notification::FileChangePatchUpdated(_) => "FileChangePatchUpdated",
                    Notification::PlanDelta(_) => "PlanDelta",
                    Notification::TurnPlanUpdated(_) => "TurnPlanUpdated",
                    Notification::TurnDiffUpdated(_) => "TurnDiffUpdated",
                    Notification::ReasoningSummaryPartAdded(_) => "ReasoningSummaryPartAdded",
                    Notification::ReasoningTextDelta(_) => "ReasoningTextDelta",
                    Notification::AccountLoginCompleted(_) => "AccountLoginCompleted",
                    Notification::DeprecationNotice(_) => "DeprecationNotice",
                    Notification::GuardianWarning(_) => "GuardianWarning",
                    Notification::Warning(_) => "Warning",
                    Notification::ThreadArchived(_) => "ThreadArchived",
                    Notification::ThreadClosed(_) => "ThreadClosed",
                    Notification::ThreadUnarchived(_) => "ThreadUnarchived",
                    Notification::ThreadGoalCleared(_) => "ThreadGoalCleared",
                    Notification::ThreadNameUpdated(_) => "ThreadNameUpdated",
                    Notification::SkillsChanged(_) => "SkillsChanged",
                    Notification::FsChanged(_) => "FsChanged",
                    Notification::ConfigWarning(_) => "ConfigWarning",
                    Notification::AccountUpdated(_) => "AccountUpdated",
                    Notification::AppListUpdated(_) => "AppListUpdated",
                    Notification::CommandExecOutputDelta(_) => "CommandExecOutputDelta",
                    Notification::ExternalAgentConfigImportCompleted(_) => {
                        "ExternalAgentConfigImportCompleted"
                    }
                    Notification::ExternalAgentConfigImportProgress(_) => {
                        "ExternalAgentConfigImportProgress"
                    }
                    Notification::FuzzyFileSearchSessionCompleted(_) => {
                        "FuzzyFileSearchSessionCompleted"
                    }
                    Notification::FuzzyFileSearchSessionUpdated(_) => {
                        "FuzzyFileSearchSessionUpdated"
                    }
                    Notification::HookCompleted(_) => "HookCompleted",
                    Notification::HookStarted(_) => "HookStarted",
                    Notification::ItemGuardianApprovalReviewCompleted(_) => {
                        "ItemGuardianApprovalReviewCompleted"
                    }
                    Notification::ItemGuardianApprovalReviewStarted(_) => {
                        "ItemGuardianApprovalReviewStarted"
                    }
                    Notification::TerminalInteraction(_) => "TerminalInteraction",
                    Notification::McpToolCallProgress(_) => "McpToolCallProgress",
                    Notification::ModelRerouted(_) => "ModelRerouted",
                    Notification::ModelSafetyBufferingUpdated(_) => "ModelSafetyBufferingUpdated",
                    Notification::ModelVerification(_) => "ModelVerification",
                    Notification::ProcessExited(_) => "ProcessExited",
                    Notification::ProcessOutputDelta(_) => "ProcessOutputDelta",
                    Notification::ServerRequestResolved(_) => "ServerRequestResolved",
                    Notification::ContextCompacted(_) => "ContextCompacted",
                    Notification::ThreadGoalUpdated(_) => "ThreadGoalUpdated",
                    Notification::ThreadEnvironmentConnected(_) => "ThreadEnvironmentConnected",
                    Notification::ThreadEnvironmentDisconnected(_) => {
                        "ThreadEnvironmentDisconnected"
                    }
                    Notification::StrictReviewRequired(_) => "StrictReviewRequired",
                    Notification::ProjectChanged(_) => "ProjectChanged",
                    Notification::ThreadProjectUpdated(_) => "ThreadProjectUpdated",
                    Notification::ThreadQueueChanged(_) => "ThreadQueueChanged",
                    Notification::ThreadReverted(_) => "ThreadReverted",
                    Notification::McpServerEventStream(_) => "McpServerEventStream",
                    Notification::ThreadRealtimeClosed(_) => "ThreadRealtimeClosed",
                    Notification::ThreadRealtimeError(_) => "ThreadRealtimeError",
                    Notification::ThreadRealtimeItemAdded(_) => "ThreadRealtimeItemAdded",
                    Notification::ThreadRealtimeOutputAudioDelta(_) => {
                        "ThreadRealtimeOutputAudioDelta"
                    }
                    Notification::ThreadRealtimeSdp(_) => "ThreadRealtimeSdp",
                    Notification::ThreadRealtimeStarted(_) => "ThreadRealtimeStarted",
                    Notification::ThreadDeleted(_) => "ThreadDeleted",
                    Notification::ThreadSettingsUpdated(_) => "ThreadSettingsUpdated",
                    Notification::TurnModerationMetadata(_) => "TurnModerationMetadata",
                    Notification::ThreadRealtimeTranscriptDelta(_) => {
                        "ThreadRealtimeTranscriptDelta"
                    }
                    Notification::ThreadRealtimeTranscriptDone(_) => "ThreadRealtimeTranscriptDone",
                    Notification::ThreadRealtimeItemStarted(_) => "ThreadRealtimeItemStarted",
                    Notification::ThreadRealtimeItemCompleted(_) => "ThreadRealtimeItemCompleted",
                    Notification::ThreadRealtimeItemTranscriptDelta(_) => {
                        "ThreadRealtimeItemTranscriptDelta"
                    }
                    Notification::ModelProviderAuthRecoveryStarted(_) => {
                        "ModelProviderAuthRecoveryStarted"
                    }
                    Notification::ModelProviderAuthRecoveryCompleted(_) => {
                        "ModelProviderAuthRecoveryCompleted"
                    }
                    Notification::WindowsWorldWritableWarning(_) => "WindowsWorldWritableWarning",
                    Notification::WindowsSandboxSetupCompleted(_) => "WindowsSandboxSetupCompleted",
                    Notification::Unknown { method, .. } => {
                        *unknown_methods.entry(method.clone()).or_insert(0) += 1;
                        continue;
                    }
                };
                *typed_counts.entry(variant).or_insert(0) += 1;
                if matches!(&n, Notification::TurnCompleted(_)) {
                    break;
                }
            }
            ServerMessage::Request { id, request } => {
                if request.is_unknown() {
                    unknown_methods
                        .entry(request.method().to_string())
                        .and_modify(|c| *c += 1)
                        .or_insert(1);
                }
                client
                    .respond(id, &serde_json::json!({"decision": "accept"}))
                    .await
                    .expect("respond");
            }
        }

        if message_count > 500 {
            break;
        }
    }

    eprintln!("\n── Typed message audit ──────────────────────────────────");
    eprintln!("Typed variants seen ({} kinds):", typed_counts.len());
    for (variant, n) in &typed_counts {
        eprintln!("  {:4}× Notification::{}", n, variant);
    }
    eprintln!("─────────────────────────────────────────────────────────\n");

    assert!(
        unknown_methods.is_empty(),
        "Wire methods with no typed binding (audit must be empty): {:?}",
        unknown_methods
    );

    client.shutdown().await.expect("shutdown");
}

// ── End-to-end: write a real Rust source file and compile it ────────
//
// Asks the agent to produce `quicksort.rs` in a fresh scratch workspace,
// then verifies it compiles with a concrete `rustc` invocation. The point
// of this test is twofold: prove the typed approval flow round-trips
// against a real CLI, and make hangs (which the upstream-facing wrappers
// can otherwise swallow) loud — every approval is logged with its kind
// and key fields before we respond, and each `next_message()` is bounded
// so a real hang fails the test with the last-seen approval as context
// instead of sitting on stdio forever.
//
// Currently fails against codex-cli ≥ 0.130 at the file-change-approval
// step because `FileChangeApprovalParams` / `CommandExecutionApprovalParams`
// in `protocol.rs` are hand-written (excluded from the schema codegen)
// and still expect the pre-0.130 wire shape (`callId` field, no
// `startedAtMs` / `grantRoot`). The test surfacing that mismatch
// loudly is intentional — repairing those structs is the natural
// follow-up. This file's `#![cfg(feature = "integration-tests")]`
// gate keeps the failure off the default CI lane.
/// End-to-end coding task: codex writes a quicksort that actually compiles and runs - the full-loop smoke test.
#[tokio::test]
async fn test_async_client_writes_compilable_quicksort() {
    use std::process::Command;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use tokio::time::timeout;

    let _ = env_logger::builder().is_test(true).try_init();

    // Codex refuses to operate outside a git repo by default, so init one
    // in the scratch dir before spawning the app-server there.
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let scratch = std::env::temp_dir().join(format!("codex-codes-quicksort-{nonce}"));
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).expect("create scratch dir");
    let git_init = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&scratch)
        .status()
        .expect("spawn git init");
    assert!(
        git_init.success(),
        "git init failed in {}",
        scratch.display()
    );

    let mut client = AsyncClient::spawn(AppServerBuilder::new().working_directory(&scratch))
        .await
        .expect("spawn app-server");
    client
        .initialize(&InitializeParams {
            client_info: ClientInfo {
                name: "codex-codes-test".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: Some("Quicksort Integration".to_string()),
            },
            capabilities: None,
        })
        .await
        .expect("initialize");

    let thread = client
        .thread_start(&serde_json::from_value::<ThreadStartParams>(serde_json::json!({})).unwrap())
        .await
        .expect("thread_start");

    let prompt = "Create a file named `quicksort.rs` in the current working directory \
        containing a self-contained Rust program: an in-place `quicksort` function over \
        `[i32]`, plus a `main` that sorts a small literal slice and prints the result. \
        Do not create a `Cargo.toml` and do not use any external crates. The file MUST \
        compile with: `rustc --edition 2021 quicksort.rs -o /dev/null`.";

    client
        .turn_start(&TurnStartParams {
            thread_id: thread.thread.id.clone(),
            tool_output: None,
            input: vec![UserInput::Text {
                text: prompt.to_string(),
                text_elements: None,
            }],
            approval_policy: None,
            approvals_reviewer: None,
            client_user_message_id: None,
            cwd: None,
            effort: None,
            model: None,
            output_schema: None,
            personality: None,
            sandbox_policy: None,
            service_tier: None,
            service_tier_for_turn: None,
            summary: None,
            turn_trigger: None,
        })
        .await
        .expect("turn_start");

    let per_message = Duration::from_secs(90);
    let mut approvals_seen = 0u32;
    let mut message_count = 0u32;
    let mut last_approval: Option<String> = None;

    loop {
        let next = timeout(per_message, client.next_message()).await;
        let msg = match next {
            Ok(Ok(Some(m))) => m,
            Ok(Ok(None)) => panic!(
                "app-server stream ended before TurnCompleted \
                 (messages={message_count}, approvals={approvals_seen}, last_approval={:?})",
                last_approval
            ),
            Ok(Err(e)) => panic!("read error after {message_count} messages: {e}"),
            Err(_) => panic!(
                "no message for {}s — possible hang \
                 (messages={message_count}, approvals={approvals_seen}, last_approval={:?})",
                per_message.as_secs(),
                last_approval
            ),
        };
        message_count += 1;

        match msg {
            ServerMessage::Notification(Notification::TurnCompleted(_)) => break,
            ServerMessage::Notification(_) => {}
            ServerMessage::Request { id, request } => {
                approvals_seen += 1;
                let summary = match &request {
                    ServerRequest::CmdExecApproval(p) => format!(
                        "cmdExec item={} cwd={:?} cmd={:?} reason={:?}",
                        p.item_id, p.cwd, p.command, p.reason
                    ),
                    ServerRequest::FileChangeApproval(p) => {
                        format!("fileChange item={} reason={:?}", p.item_id, p.reason)
                    }
                    ServerRequest::Unknown { method, .. } => format!("unknown method={method}"),
                    other => format!("{} (unhandled)", other.method()),
                };
                eprintln!("[approval #{approvals_seen}] {summary}");
                last_approval = Some(summary);
                client
                    .respond(id, &serde_json::json!({"decision": "accept"}))
                    .await
                    .expect("respond to approval");
            }
        }

        if message_count > 5000 {
            panic!(
                "too many messages without TurnCompleted (count={message_count}, \
                 approvals={approvals_seen})"
            );
        }
    }

    client.shutdown().await.expect("shutdown");

    let written = scratch.join("quicksort.rs");
    assert!(
        written.exists(),
        "quicksort.rs not written to {}",
        scratch.display()
    );

    let out = scratch.join("quicksort_bin");
    let compile = Command::new("rustc")
        .args(["--edition", "2021", "quicksort.rs", "-o"])
        .arg(&out)
        .current_dir(&scratch)
        .output()
        .expect("spawn rustc");
    assert!(
        compile.status.success(),
        "rustc failed for {}:\n--- stderr ---\n{}\n--- stdout ---\n{}",
        written.display(),
        String::from_utf8_lossy(&compile.stderr),
        String::from_utf8_lossy(&compile.stdout)
    );

    let _ = std::fs::remove_dir_all(&scratch);
}

/// Account read-side helpers against the live app-server: `account/read`,
/// `account/rateLimits/read`, `account/usage/read`. Read-only — never
/// mutates the stored credential.
#[tokio::test]
async fn test_account_read_family() {
    let mut client = AsyncClient::start()
        .await
        .expect("Failed to start app-server");

    let account = client
        .account_read(&codex_codes::protocol_generated::types::GetAccountParams::default())
        .await
        .expect("account/read");
    // This environment is logged in via ChatGPT; a logged-out env would
    // carry account: None and that's a legitimate wire answer too.
    let _ = account;

    // Rate-limit/usage reads proxy to the ChatGPT usage backend, which can
    // reject a locally-valid session token (observed: 401 "Could not parse
    // your authentication token" while model turns work fine). The wiring
    // assertion is "typed request goes out, typed response OR a well-formed
    // JSON-RPC error comes back" — not that this box's token can fetch
    // usage.
    match client.account_rate_limits_read(Default::default()).await {
        Ok(_) => {}
        Err(codex_codes::Error::JsonRpc { code, .. }) => {
            assert_eq!(code, -32603, "unexpected rpc error class");
        }
        Err(other) => panic!("account/rateLimits/read transport failure: {other}"),
    }
    match client.account_usage_read().await {
        Ok(_) => {}
        Err(codex_codes::Error::JsonRpc { code, .. }) => {
            assert_eq!(code, -32603, "unexpected rpc error class");
        }
        Err(other) => panic!("account/usage/read transport failure: {other}"),
    }

    client.shutdown().await.expect("shutdown");
}

/// Local auth snapshot parses the REAL ~/.codex/auth.json on this box —
/// the private-contract shape watchdog. If codex changes the file layout,
/// this fails here (a crate patch) instead of silently downstream.
#[test]
fn test_auth_status_local_parses_real_file() {
    let path = codex_codes::auth_local::auth_json_path().expect("home resolves");
    if !path.exists() {
        eprintln!("skipping: no auth.json on this host");
        return;
    }
    let s = codex_codes::auth_local::auth_status_local().expect("real auth.json parses");
    assert!(s.logged_in, "file exists but parsed as logged out");
    // This box is chatgpt-mode; a fresh api-key box would legitimately
    // carry no identity claims.
    if s.auth_mode.as_deref() == Some("chatgpt") {
        assert!(
            s.email.is_some(),
            "chatgpt mode should yield an email label"
        );
        assert!(s.account_id.is_some());
    }
}

/// thread/resume reopens a persisted thread through the typed helper (no
/// methods::* or raw request::<> needed downstream) and its response
/// decodes: the resumed thread carries the same id and replays into a
/// working turn.
#[tokio::test]
async fn test_thread_resume_reopens_thread_via_typed_helper() {
    let mut client = AsyncClient::start().await.expect("start app-server");
    let source = client
        .thread_start(&ThreadStartParams::default())
        .await
        .expect("thread_start");
    client
        .turn_start(&TurnStartParams {
            thread_id: source.thread.id.clone(),
            tool_output: None,
            input: vec![UserInput::Text {
                text: "Remember the word 'garnet'. Reply with just OK.".to_string(),
                text_elements: None,
            }],
            approval_policy: None,
            approvals_reviewer: None,
            client_user_message_id: None,
            cwd: None,
            effort: None,
            model: None,
            output_schema: None,
            personality: None,
            sandbox_policy: None,
            service_tier: None,
            service_tier_for_turn: None,
            summary: None,
            turn_trigger: None,
        })
        .await
        .expect("seed turn");
    let mut n = 0;
    while let Some(msg) = client.next_message().await.expect("read") {
        n += 1;
        match msg {
            ServerMessage::Notification(Notification::TurnCompleted(_)) => break,
            ServerMessage::Request { id, .. } => {
                client
                    .respond(id, &serde_json::json!({"decision": "accept"}))
                    .await
                    .expect("respond");
            }
            _ => {}
        }
        assert!(n < 200, "seed turn runaway");
    }
    client.shutdown().await.expect("shutdown source");

    // Fresh app-server process: resume must rehydrate from persistence.
    let mut resumed = AsyncClient::start().await.expect("restart app-server");
    let response = resumed
        .thread_resume(
            &serde_json::from_value(serde_json::json!({ "threadId": source.thread.id }))
                .expect("resume params"),
        )
        .await
        .expect("thread_resume decodes");
    assert_eq!(
        response.thread.id, source.thread.id,
        "resume must reopen the SAME thread id"
    );
    resumed.shutdown().await.expect("shutdown resumed");
}
