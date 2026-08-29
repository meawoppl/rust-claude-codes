//! Codex wire checks: binary, local credential snapshot, and a live
//! app-server session — initialize, thread_start, one turn driven to
//! `TurnCompleted` through the typed JSON-RPC client.

use crate::state::{CheckStatus, Reporter};
use codex_codes::{Notification, ServerMessage, ThreadStartParams, TurnStartParams, UserInput};

pub async fn run_suite(reporter: Reporter) {
    let started = reporter.start("binary", "codex CLI present on PATH").await;
    let version = tokio::process::Command::new("codex")
        .arg("--version")
        .output()
        .await;
    match &version {
        Ok(o) if o.status.success() => {
            reporter
                .finish(
                    "binary",
                    started,
                    CheckStatus::Pass,
                    String::from_utf8_lossy(&o.stdout).trim().to_string(),
                )
                .await;
        }
        other => {
            reporter
                .finish("binary", started, CheckStatus::Fail, format!("{other:?}"))
                .await;
            return;
        }
    }

    let started = reporter
        .start("auth", "local credential snapshot (~/.codex/auth.json)")
        .await;
    let logged_in = match codex_codes::auth_local::auth_status_local() {
        Ok(s) => {
            let detail = format!(
                "logged_in={} method={}",
                s.logged_in,
                s.auth_mode.as_deref().unwrap_or("-")
            );
            let st = if s.logged_in {
                CheckStatus::Pass
            } else {
                CheckStatus::Fail
            };
            reporter.finish("auth", started, st, detail).await;
            s.logged_in
        }
        Err(e) => {
            reporter
                .finish("auth", started, CheckStatus::Fail, e.to_string())
                .await;
            false
        }
    };

    if !logged_in {
        let started = reporter
            .start(
                "live_turn",
                "app-server turn to TurnCompleted through typed JSON-RPC",
            )
            .await;
        reporter
            .finish(
                "live_turn",
                started,
                CheckStatus::Skipped,
                "not logged in — run `codex login` on this host (browser callback flow)".into(),
            )
            .await;
        return;
    }

    live_turn(&reporter).await;
    thread_fork(&reporter).await;
    account_read_family(&reporter).await;
}

/// A fork gets a NEW thread id after a seeded turn — the history-carrying
/// wire behavior downstream session forks rest on.
async fn thread_fork(reporter: &Reporter) {
    let started = reporter
        .start(
            "thread_fork",
            "thread/fork returns a distinct thread after a seed turn",
        )
        .await;
    let fut = async {
        let mut client = codex_codes::AsyncClient::start()
            .await
            .map_err(|e| e.to_string())?;
        let source = client
            .thread_start(&ThreadStartParams::default())
            .await
            .map_err(|e| e.to_string())?;
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
            .map_err(|e| e.to_string())?;
        let mut n = 0usize;
        while let Some(msg) = client.next_message().await.map_err(|e| e.to_string())? {
            n += 1;
            match msg {
                ServerMessage::Notification(Notification::TurnCompleted(_)) => break,
                ServerMessage::Notification(_) => {}
                ServerMessage::Request { id, .. } => {
                    client
                        .respond(id, &serde_json::json!({"decision": "accept"}))
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
            if n > 200 {
                return Err("seed turn did not complete".to_string());
            }
        }
        let fork = client
            .thread_fork(
                &serde_json::from_value(serde_json::json!({ "threadId": source.thread.id }))
                    .map_err(|e| e.to_string())?,
            )
            .await
            .map_err(|e| e.to_string())?;
        client.shutdown().await.map_err(|e| e.to_string())?;
        if fork.thread.id.is_empty() || fork.thread.id == source.thread.id {
            return Err(format!("bad fork id {:?}", fork.thread.id));
        }
        Ok(format!("{} → {}", source.thread.id, fork.thread.id))
    };
    match tokio::time::timeout(std::time::Duration::from_secs(120), fut).await {
        Ok(Ok(detail)) => {
            reporter
                .finish("thread_fork", started, CheckStatus::Pass, detail)
                .await
        }
        Ok(Err(e)) => {
            reporter
                .finish("thread_fork", started, CheckStatus::Fail, e)
                .await
        }
        Err(_) => {
            reporter
                .finish(
                    "thread_fork",
                    started,
                    CheckStatus::Fail,
                    "timed out after 120s".into(),
                )
                .await
        }
    }
}

/// account/read family: typed requests go out and typed responses (or
/// well-formed JSON-RPC errors — the usage backend can reject a locally
/// valid token) come back.
async fn account_read_family(reporter: &Reporter) {
    let started = reporter
        .start(
            "account_reads",
            "account/read + rateLimits + usage answer typed or as JSON-RPC errors",
        )
        .await;
    let fut = async {
        let mut client = codex_codes::AsyncClient::start()
            .await
            .map_err(|e| e.to_string())?;
        client
            .account_read(&codex_codes::protocol_generated::types::GetAccountParams::default())
            .await
            .map_err(|e| format!("account/read: {e}"))?;
        for (name, result) in [
            (
                "rateLimits",
                client.account_rate_limits_read().await.map(|_| ()),
            ),
            ("usage", client.account_usage_read().await.map(|_| ())),
        ] {
            match result {
                Ok(()) => {}
                Err(codex_codes::Error::JsonRpc { code: -32603, .. }) => {}
                Err(other) => return Err(format!("account/{name}: transport failure: {other}")),
            }
        }
        client.shutdown().await.map_err(|e| e.to_string())?;
        Ok("all three account reads answered in protocol".to_string())
    };
    match tokio::time::timeout(std::time::Duration::from_secs(60), fut).await {
        Ok(Ok(detail)) => {
            reporter
                .finish("account_reads", started, CheckStatus::Pass, detail)
                .await
        }
        Ok(Err(e)) => {
            reporter
                .finish("account_reads", started, CheckStatus::Fail, e)
                .await
        }
        Err(_) => {
            reporter
                .finish(
                    "account_reads",
                    started,
                    CheckStatus::Fail,
                    "timed out after 60s".into(),
                )
                .await
        }
    }
}

async fn live_turn(reporter: &Reporter) {
    let started = reporter
        .start(
            "live_turn",
            "app-server turn to TurnCompleted through typed JSON-RPC",
        )
        .await;
    let fut = async {
        let mut client = codex_codes::AsyncClient::start()
            .await
            .map_err(|e| e.to_string())?;
        let thread = client
            .thread_start(&ThreadStartParams::default())
            .await
            .map_err(|e| e.to_string())?;
        if thread.thread.id.is_empty() {
            return Err("thread_start returned an empty thread id".to_string());
        }
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
            .map_err(|e| e.to_string())?;

        let mut notifications = 0usize;
        let mut answered = false;
        let mut completed = false;
        while let Some(msg) = client.next_message().await.map_err(|e| e.to_string())? {
            match msg {
                ServerMessage::Notification(Notification::AgentMessageDelta(d)) => {
                    notifications += 1;
                    if d.delta.contains('4') {
                        answered = true;
                    }
                }
                ServerMessage::Notification(Notification::TurnCompleted(_)) => {
                    completed = true;
                    break;
                }
                ServerMessage::Notification(_) => notifications += 1,
                ServerMessage::Request { id, .. } => {
                    // Auto-accept approvals: this is a 2+2 turn in a scratch
                    // thread; nothing it can ask for is consequential.
                    client
                        .respond(id, &serde_json::json!({"decision": "accept"}))
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
            if notifications > 500 {
                return Err("runaway notification stream (>500 before TurnCompleted)".to_string());
            }
        }
        client.shutdown().await.map_err(|e| e.to_string())?;
        if !completed {
            return Err("stream ended without TurnCompleted".to_string());
        }
        if !answered {
            return Err("no delta contained the expected answer".to_string());
        }
        Ok(format!(
            "thread {}; {notifications} typed notifications to completion",
            thread.thread.id
        ))
    };
    match tokio::time::timeout(std::time::Duration::from_secs(120), fut).await {
        Ok(Ok(detail)) => {
            reporter
                .finish("live_turn", started, CheckStatus::Pass, detail)
                .await;
        }
        Ok(Err(e)) => {
            reporter
                .finish("live_turn", started, CheckStatus::Fail, e)
                .await;
        }
        Err(_) => {
            reporter
                .finish(
                    "live_turn",
                    started,
                    CheckStatus::Fail,
                    "timed out after 120s".into(),
                )
                .await;
        }
    }
}
