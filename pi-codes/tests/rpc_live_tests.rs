//! Live tests against a real `pi --mode rpc` process.
//!
//! The credential-free tier needs only the pi binary on PATH (Node 22+);
//! no provider keys. Run with:
//! `cargo test -p pi-codes --features integration-tests`

#![cfg(feature = "integration-tests")]

use pi_codes::{PiRpcClient, RpcCommand};

/// The RPC server starts ephemeral and answers get_state with a session id and idle flags.
#[tokio::test]
async fn get_state_answers_credential_free() {
    let mut client = PiRpcClient::start().await.expect("spawn pi --mode rpc");
    let resp = client
        .request_ok(RpcCommand::GetState { id: None })
        .await
        .expect("get_state");
    let state: pi_codes::AgentState =
        serde_json::from_value(resp.data.expect("state data")).expect("typed AgentState");
    assert!(!state.session_id.is_empty(), "session id present");
    assert!(!state.is_streaming, "fresh server is idle");
    client.shutdown().await.unwrap();
}

/// Response ids echo the request id for correlation.
#[tokio::test]
async fn response_id_correlates() {
    let mut client = PiRpcClient::start().await.expect("spawn");
    let resp = client
        .request(RpcCommand::GetAvailableModels {
            id: Some("corr-1".into()),
        })
        .await
        .expect("get_available_models");
    assert_eq!(resp.id.as_deref(), Some("corr-1"));
    assert!(resp.success);
    client.shutdown().await.unwrap();
}

/// The bash RPC command runs a real shell command and returns typed output; no model involved.
#[tokio::test]
async fn bash_command_round_trips() {
    let mut client = PiRpcClient::start().await.expect("spawn");
    let resp = client
        .request_ok(RpcCommand::Bash {
            id: None,
            command: "echo pi-codes-live-probe".into(),
        })
        .await
        .expect("bash");
    let result: pi_codes::BashResult =
        serde_json::from_value(resp.data.expect("bash data")).expect("typed BashResult");
    assert_eq!(result.exit_code, Some(0));
    assert!(result.output.contains("pi-codes-live-probe"));
    assert!(!result.cancelled);
    client.shutdown().await.unwrap();
}

/// After bash, get_messages carries a typed bashExecution message.
#[tokio::test]
async fn bash_lands_in_messages_typed() {
    let mut client = PiRpcClient::start().await.expect("spawn");
    client
        .request_ok(RpcCommand::Bash {
            id: None,
            command: "echo typed-message-probe".into(),
        })
        .await
        .expect("bash");
    let resp = client
        .request_ok(RpcCommand::GetMessages { id: None })
        .await
        .expect("get_messages");
    let msgs: pi_codes::rpc::Messages =
        serde_json::from_value(resp.data.expect("messages data")).expect("typed messages");
    assert!(
        msgs.messages.iter().any(
            |m| matches!(m, pi_codes::PiMessage::BashExecution { output, .. }
                if output.contains("typed-message-probe"))
        ),
        "bashExecution message present and typed"
    );
    client.shutdown().await.unwrap();
}

/// An unknown/unmodeled command sent via Raw still gets a response envelope (success or a clean error).
#[tokio::test]
async fn raw_command_gets_response_envelope() {
    let mut client = PiRpcClient::start().await.expect("spawn");
    let resp = client
        .request(RpcCommand::Raw(serde_json::json!({
            "type": "get_session_stats", "id": "raw-1"
        })))
        .await
        .expect("raw get_session_stats");
    assert_eq!(resp.id.as_deref(), Some("raw-1"));
    assert_eq!(resp.command, "get_session_stats");
    client.shutdown().await.unwrap();
}

/// A garbage command type produces the documented parse/unknown failure, not a crash or silence.
#[tokio::test]
async fn unknown_command_fails_cleanly() {
    let mut client = PiRpcClient::start().await.expect("spawn");
    let resp = client
        .request(RpcCommand::Raw(serde_json::json!({
            "type": "no_such_command_xyz", "id": "bad-1"
        })))
        .await
        .expect("response for bad command");
    assert!(!resp.success, "unknown command must not succeed");
    assert!(resp.error.is_some());
    client.shutdown().await.unwrap();
}

/// Full model turn through RPC (needs OPENAI_API_KEY; skips cleanly without): prompt → typed
/// events stream to agent_end, assistant reply echoes a nonce, no decode failures anywhere.
#[tokio::test]
async fn model_turn_streams_typed_events() {
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("skipping: OPENAI_API_KEY not set");
        return;
    }
    let mut client = PiRpcClient::spawn(
        pi_codes::PiCliBuilder::new()
            .no_session(true)
            .provider("openai")
            .model("gpt-4.1-mini"),
    )
    .await
    .expect("spawn");
    let resp = client
        .request_ok(RpcCommand::Prompt {
            id: None,
            message: "Reply with exactly: pi-codes-model-probe".into(),
            images: None,
            streaming_behavior: None,
        })
        .await
        .expect("prompt accepted");
    assert!(resp.success);

    let mut kinds = Vec::new();
    let mut answer = String::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    loop {
        assert!(std::time::Instant::now() < deadline, "turn did not finish");
        let ev = tokio::time::timeout(std::time::Duration::from_secs(60), client.next_event())
            .await
            .expect("event within 60s")
            .expect("stream healthy")
            .expect("stream open");
        kinds.push(ev.event_type().to_string());
        match &ev {
            pi_codes::PiEvent::MessageEnd { message } => {
                if let pi_codes::PiMessage::Assistant {
                    content,
                    stop_reason,
                    extra,
                    ..
                } = message.as_ref()
                {
                    for block in content {
                        if let pi_codes::ContentBlock::Text { text } = block {
                            answer.push_str(text);
                        }
                    }
                    // Provider failures arrive as a typed assistant
                    // message with stopReason "error" — fail with the
                    // provider's own words (e.g. an out-of-credits key).
                    if stop_reason == "error" {
                        panic!(
                            "provider errored: {}",
                            extra
                                .get("errorMessage")
                                .and_then(|e| e.as_str())
                                .unwrap_or("(no errorMessage)")
                        );
                    }
                }
            }
            pi_codes::PiEvent::AgentEnd { .. } => break,
            _ => {}
        }
    }
    client.shutdown().await.unwrap();
    assert!(
        answer.contains("pi-codes-model-probe"),
        "assistant echoed the nonce; got: {answer:?}; events: {kinds:?}"
    );
    assert!(
        kinds.contains(&"agent_start".to_string()),
        "events: {kinds:?}"
    );
    assert!(
        kinds.contains(&"message_update".to_string()),
        "streaming deltas present"
    );
}

// ── Conformance tier (mirrors wirecheck's cross-harness checks) ──────
// UUID nonces the model can't fake; write/bash verified on DISK, not
// from the transcript. All gated on OPENAI_API_KEY.

fn conformance_client_prompt(workdir: &std::path::Path) -> pi_codes::PiCliBuilder {
    pi_codes::PiCliBuilder::new()
        .no_session(true)
        .provider("openai")
        .model("gpt-4.1-mini")
        .working_directory(workdir)
}

async fn run_conformance_turn(workdir: &std::path::Path, prompt: &str) -> String {
    let mut client = PiRpcClient::spawn(conformance_client_prompt(workdir))
        .await
        .expect("spawn");
    client
        .request_ok(RpcCommand::Prompt {
            id: None,
            message: prompt.into(),
            images: None,
            streaming_behavior: None,
        })
        .await
        .expect("prompt accepted");
    let mut answer = String::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
    loop {
        assert!(std::time::Instant::now() < deadline, "turn did not finish");
        let ev = tokio::time::timeout(std::time::Duration::from_secs(90), client.next_event())
            .await
            .expect("event within 90s")
            .expect("stream healthy")
            .expect("stream open");
        match &ev {
            pi_codes::PiEvent::MessageEnd { message } => {
                if let pi_codes::PiMessage::Assistant {
                    content,
                    stop_reason,
                    extra,
                    ..
                } = message.as_ref()
                {
                    if stop_reason == "error" {
                        panic!(
                            "provider errored: {}",
                            extra
                                .get("errorMessage")
                                .and_then(|e| e.as_str())
                                .unwrap_or("(no errorMessage)")
                        );
                    }
                    for block in content {
                        if let pi_codes::ContentBlock::Text { text } = block {
                            answer.push_str(text);
                        }
                    }
                }
            }
            pi_codes::PiEvent::AgentEnd { .. } => break,
            _ => {}
        }
    }
    client.shutdown().await.unwrap();
    answer
}

fn conformance_workdir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "pi-codes-conform-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn nonce() -> String {
    // Two independent entropy sources; no uuid dep needed in tests.
    format!(
        "{:x}-{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    )
}

/// Model reads a planted file with its read tool and reports the nonce inside (needs OPENAI_API_KEY; skips without).
#[tokio::test]
async fn conform_model_reads_planted_file() {
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("skipping: OPENAI_API_KEY not set");
        return;
    }
    let dir = conformance_workdir();
    let n = nonce();
    let secret = dir.join("secret.txt");
    std::fs::write(&secret, format!("{n}\n")).unwrap();
    let answer = run_conformance_turn(
        &dir,
        &format!(
            "A file exists at {} — read it with your read tool and reply \
             with the token it contains.",
            secret.display()
        ),
    )
    .await;
    assert!(
        answer.contains(&n),
        "planted token not in answer: {answer:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Model writes a requested nonce to a requested path with its write tool; verified on disk (needs OPENAI_API_KEY; skips without).
#[tokio::test]
async fn conform_model_writes_requested_nonce() {
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("skipping: OPENAI_API_KEY not set");
        return;
    }
    let dir = conformance_workdir();
    let n = nonce();
    let target = dir.join("written.txt");
    run_conformance_turn(
        &dir,
        &format!(
            "Using your write tool, create a file at {} whose contents is \
             exactly this token: {n}",
            target.display()
        ),
    )
    .await;
    let on_disk = std::fs::read_to_string(&target).expect("file written");
    assert!(on_disk.contains(&n), "nonce missing on disk: {on_disk:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Model runs a shell command with a disk-visible side effect via its bash tool (needs OPENAI_API_KEY; skips without).
#[tokio::test]
async fn conform_model_bash_disk_side_effect() {
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("skipping: OPENAI_API_KEY not set");
        return;
    }
    let dir = conformance_workdir();
    let n = nonce();
    let target = dir.join("bash_out.txt");
    run_conformance_turn(
        &dir,
        &format!(
            "Using your bash tool, run exactly this command: echo {n} > {}",
            target.display()
        ),
    )
    .await;
    let on_disk = std::fs::read_to_string(&target).expect("command ran");
    assert!(on_disk.contains(&n), "nonce missing on disk: {on_disk:?}");
    let _ = std::fs::remove_dir_all(&dir);
}
