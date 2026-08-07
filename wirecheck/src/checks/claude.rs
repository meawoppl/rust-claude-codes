//! Claude Code wire checks: binary, credential state, and a live
//! stream-json round trip through the typed async client — send a user
//! message, receive typed Assistant + Result frames, session id present.

use crate::state::{CheckStatus, Reporter};
use claude_codes::io::ClaudeOutput;

pub async fn run_suite(reporter: Reporter) {
    let started = reporter.start("binary", "claude CLI present on PATH").await;
    let version = tokio::process::Command::new("claude")
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
        .start("auth", "credential state via `claude auth status`")
        .await;
    let status = tokio::task::spawn_blocking(claude_codes::auth::auth_status).await;
    let logged_in = match status {
        Ok(Ok(s)) => {
            let detail = format!(
                "logged_in={} method={} provider={}",
                s.logged_in,
                s.auth_method.as_deref().unwrap_or("-"),
                s.api_provider.as_deref().unwrap_or("-"),
            );
            let st = if s.logged_in {
                CheckStatus::Pass
            } else {
                CheckStatus::Fail
            };
            reporter.finish("auth", started, st, detail).await;
            s.logged_in
        }
        other => {
            reporter
                .finish("auth", started, CheckStatus::Fail, format!("{other:?}"))
                .await;
            false
        }
    };

    if !logged_in {
        let started = reporter
            .start(
                "live_turn",
                "typed stream-json round trip (send → Assistant → Result)",
            )
            .await;
        reporter
            .finish(
                "live_turn",
                started,
                CheckStatus::Skipped,
                "not logged in — use the login flow above first".into(),
            )
            .await;
        return;
    }

    live_turn(&reporter).await;
}

/// One real turn through the typed client. Everything the wire returns
/// must deserialize into a typed [`ClaudeOutput`]; the client itself
/// errors on undecodable frames, so a clean query IS the strict audit.
async fn live_turn(reporter: &Reporter) {
    let started = reporter
        .start(
            "live_turn",
            "typed stream-json round trip (send → Assistant → Result)",
        )
        .await;
    let fut = async {
        let mut client = claude_codes::AsyncClient::with_defaults()
            .await
            .map_err(|e| e.to_string())?;
        let outputs = client
            .query("Reply with exactly: pong")
            .await
            .map_err(|e| e.to_string())?;
        let session = client.session_uuid().map_err(|e| e.to_string())?;
        let mut kinds = Vec::new();
        let mut assistant_text = String::new();
        let mut got_result = false;
        for out in &outputs {
            match out {
                ClaudeOutput::Assistant(m) => {
                    kinds.push("assistant");
                    assistant_text.push_str(&format!("{m:?}"));
                }
                ClaudeOutput::Result(_) => {
                    kinds.push("result");
                    got_result = true;
                }
                ClaudeOutput::System(_) => kinds.push("system"),
                ClaudeOutput::User(_) => kinds.push("user"),
                _ => kinds.push("other"),
            }
        }
        client.shutdown().await.map_err(|e| e.to_string())?;
        if !got_result {
            return Err(format!("no Result frame; frames: {kinds:?}"));
        }
        if !assistant_text.to_lowercase().contains("pong") {
            return Err("assistant reply did not contain the requested text".to_string());
        }
        Ok(format!("session {session}; frames: {kinds:?}"))
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
