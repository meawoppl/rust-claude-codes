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
    ping(&reporter).await;
    conversation_continuity(&reporter).await;
    fork_carries_history(&reporter).await;
    tool_use_blocks(&reporter).await;
    approval_handshake(&reporter).await;
}

/// Control-protocol liveness: a running client answers a ping.
async fn ping(reporter: &Reporter) {
    let started = reporter
        .start("ping", "control-protocol ping round trip")
        .await;
    let fut = async {
        let mut client = claude_codes::AsyncClient::with_defaults()
            .await
            .map_err(|e| e.to_string())?;
        let ok = client.ping().await;
        client.shutdown().await.map_err(|e| e.to_string())?;
        if ok {
            Ok("pong".to_string())
        } else {
            Err("ping returned false".to_string())
        }
    };
    report_timed(reporter, "ping", started, 60, fut).await;
}

/// Two queries in one process share the conversation: the second answer
/// must recall a token from the first.
async fn conversation_continuity(reporter: &Reporter) {
    let started = reporter
        .start(
            "conversation",
            "second turn recalls the first (same process)",
        )
        .await;
    let fut = async {
        let mut client = claude_codes::AsyncClient::with_defaults()
            .await
            .map_err(|e| e.to_string())?;
        client
            .query("Remember the word 'pineapple'. Reply with just OK.")
            .await
            .map_err(|e| e.to_string())?;
        let outputs = client
            .query("What word did I ask you to remember? Reply with just that word.")
            .await
            .map_err(|e| e.to_string())?;
        client.shutdown().await.map_err(|e| e.to_string())?;
        let text = assistant_text(&outputs);
        if text.to_lowercase().contains("pineapple") {
            Ok("second turn recalled the token".to_string())
        } else {
            Err(format!(
                "recall failed; assistant said: {}",
                &text[..text.len().min(80)]
            ))
        }
    };
    report_timed(reporter, "conversation", started, 180, fut).await;
}

/// `--fork-session`: a fork carries the source session's history under a
/// NEW session id.
async fn fork_carries_history(reporter: &Reporter) {
    let started = reporter
        .start(
            "fork_session",
            "fork carries history under a new session id",
        )
        .await;
    let fut = async {
        let source = uuid::Uuid::new_v4();
        let builder = claude_codes::ClaudeCliBuilder::new()
            .allow_recursion()
            .session_id(source);
        let mut client = claude_codes::AsyncClient::from_builder(builder)
            .await
            .map_err(|e| e.to_string())?;
        client
            .query("Remember the word 'kumquat'. Reply with just OK.")
            .await
            .map_err(|e| e.to_string())?;
        client.shutdown().await.map_err(|e| e.to_string())?;

        let fork_id = uuid::Uuid::new_v4();
        let builder = claude_codes::ClaudeCliBuilder::new()
            .allow_recursion()
            .fork_from(source.to_string())
            .session_id(fork_id);
        let mut fork = claude_codes::AsyncClient::from_builder(builder)
            .await
            .map_err(|e| e.to_string())?;
        let outputs = fork
            .query("What word did I ask you to remember? Reply with just that word.")
            .await
            .map_err(|e| e.to_string())?;
        let session = fork.session_uuid().map_err(|e| e.to_string())?;
        fork.shutdown().await.map_err(|e| e.to_string())?;
        let text = assistant_text(&outputs);
        if session == source {
            return Err("fork kept the SOURCE session id".to_string());
        }
        if !text.to_lowercase().contains("kumquat") {
            return Err(format!(
                "fork lost history; said: {}",
                &text[..text.len().min(80)]
            ));
        }
        Ok(format!("history carried into {session}"))
    };
    report_timed(reporter, "fork_session", started, 240, fut).await;
}

/// Tool use over the wire: a bash-running prompt produces typed ToolUse
/// content and a tool-result user frame before the Result.
async fn tool_use_blocks(reporter: &Reporter) {
    let started = reporter
        .start(
            "tool_use",
            "typed ToolUse block + tool-result frame round trip",
        )
        .await;
    let fut = async {
        let mut client = claude_codes::AsyncClient::with_defaults()
            .await
            .map_err(|e| e.to_string())?;
        let outputs = client
            .query("Run `echo wirecheck-tool-probe` with your Bash tool and reply with its output.")
            .await
            .map_err(|e| e.to_string())?;
        client.shutdown().await.map_err(|e| e.to_string())?;
        let mut saw_tool_use = false;
        let mut saw_tool_result = false;
        for out in &outputs {
            let v = serde_json::to_value(out).unwrap_or_default();
            let blocks = v
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_array())
                .cloned()
                .unwrap_or_default();
            for b in blocks {
                match b.get("type").and_then(|t| t.as_str()) {
                    Some("tool_use") => saw_tool_use = true,
                    Some("tool_result") => saw_tool_result = true,
                    _ => {}
                }
            }
        }
        if !saw_tool_use {
            return Err("no tool_use block in the stream".to_string());
        }
        if !saw_tool_result {
            return Err("tool_use without a tool_result frame".to_string());
        }
        Ok("tool_use and tool_result both present and typed".to_string())
    };
    report_timed(reporter, "tool_use", started, 180, fut).await;
}

/// The stdio permission-prompt handshake: enabling tool approval performs
/// the control-protocol initialization and is idempotent.
async fn approval_handshake(reporter: &Reporter) {
    let started = reporter
        .start(
            "approval_handshake",
            "can_use_tool control initialization succeeds and is idempotent",
        )
        .await;
    let fut = async {
        let child = claude_codes::ClaudeCliBuilder::new()
            .allow_recursion()
            .permission_prompt_tool("stdio")
            .spawn()
            .await
            .map_err(|e| e.to_string())?;
        let mut client = claude_codes::AsyncClient::new(child).map_err(|e| e.to_string())?;
        if client.is_tool_approval_enabled() {
            return Err("approval reported enabled before the handshake".to_string());
        }
        client
            .enable_tool_approval()
            .await
            .map_err(|e| e.to_string())?;
        if !client.is_tool_approval_enabled() {
            return Err("handshake succeeded but approval reads disabled".to_string());
        }
        // Second call must be a no-op, not a second handshake.
        client
            .enable_tool_approval()
            .await
            .map_err(|e| e.to_string())?;
        client.shutdown().await.map_err(|e| e.to_string())?;
        Ok("handshake ok, idempotent".to_string())
    };
    report_timed(reporter, "approval_handshake", started, 90, fut).await;
}

fn assistant_text(outputs: &[ClaudeOutput]) -> String {
    let mut text = String::new();
    for out in outputs {
        if let ClaudeOutput::Assistant(m) = out {
            text.push_str(&format!("{m:?}"));
        }
    }
    text
}

async fn report_timed(
    reporter: &Reporter,
    name: &'static str,
    started: std::time::Instant,
    timeout_s: u64,
    fut: impl std::future::Future<Output = Result<String, String>>,
) {
    match tokio::time::timeout(std::time::Duration::from_secs(timeout_s), fut).await {
        Ok(Ok(detail)) => {
            reporter
                .finish(name, started, CheckStatus::Pass, detail)
                .await
        }
        Ok(Err(e)) => reporter.finish(name, started, CheckStatus::Fail, e).await,
        Err(_) => {
            reporter
                .finish(
                    name,
                    started,
                    CheckStatus::Fail,
                    format!("timed out after {timeout_s}s"),
                )
                .await
        }
    }
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
