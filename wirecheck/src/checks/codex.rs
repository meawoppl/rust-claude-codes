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
            let detail = format!("logged_in={} method={:?}", s.logged_in, s.auth_mode);
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
                summary: None,
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
