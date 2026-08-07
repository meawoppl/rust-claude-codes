//! opencode wire checks. No credentials needed for the lifecycle tier:
//! the suite spawns its own `opencode serve` via [`ManagedServer`], so
//! these checks pin the HTTP+SSE wire against a real server every run.

use crate::state::{CheckStatus, Reporter};
use opencode_codes::protocol_generated::types::SessionCreateParams;
use opencode_codes::OpencodeClient;
use std::time::Duration;

pub async fn run_suite(reporter: Reporter) {
    let started = reporter
        .start("binary", "opencode CLI present on PATH")
        .await;
    let version = tokio::process::Command::new("opencode")
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
        .start(
            "managed_server",
            "spawn `opencode serve` and reach it over HTTP",
        )
        .await;
    let server = match opencode_codes::server::ManagedServer::builder()
        .startup_timeout(Duration::from_secs(30))
        .spawn()
        .await
    {
        Ok(s) => {
            reporter
                .finish(
                    "managed_server",
                    started,
                    CheckStatus::Pass,
                    s.url().to_string(),
                )
                .await;
            s
        }
        Err(e) => {
            reporter
                .finish("managed_server", started, CheckStatus::Fail, e.to_string())
                .await;
            return;
        }
    };
    let client = match OpencodeClient::builder()
        .base_url(server.url())
        .timeout(Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            let started = reporter.start("session_lifecycle", "client builds").await;
            reporter
                .finish(
                    "session_lifecycle",
                    started,
                    CheckStatus::Fail,
                    e.to_string(),
                )
                .await;
            return;
        }
    };

    session_lifecycle(&reporter, &client).await;
    fork_session(&reporter, &client).await;
    event_stream(&reporter, &client).await;

    let started = reporter
        .start("server_shutdown", "managed server stops cleanly")
        .await;
    match server.stop().await {
        Ok(()) => {
            reporter
                .finish(
                    "server_shutdown",
                    started,
                    CheckStatus::Pass,
                    "stopped".into(),
                )
                .await;
        }
        Err(e) => {
            reporter
                .finish("server_shutdown", started, CheckStatus::Fail, e.to_string())
                .await;
        }
    }
}

fn params(title: &str) -> SessionCreateParams {
    SessionCreateParams {
        title: Some(title.to_string()),
        agent: None,
        metadata: None,
        model: None,
        parent_id: None,
        permission: None,
        workspace_id: None,
    }
}

/// create → list-messages(empty) → abort: the basic session wire cycle.
async fn session_lifecycle(reporter: &Reporter, client: &OpencodeClient) {
    let started = reporter
        .start(
            "session_lifecycle",
            "create → list (empty) → abort round trip",
        )
        .await;
    let fut = async {
        let session = client
            .create_session(&params("wirecheck lifecycle"))
            .await
            .map_err(|e| e.to_string())?;
        if !session.id.starts_with("ses") {
            return Err(format!("unexpected session id shape: {}", session.id));
        }
        let messages = client
            .list_messages(&session.id)
            .await
            .map_err(|e| e.to_string())?;
        if !messages.is_empty() {
            return Err(format!(
                "fresh session already has {} messages",
                messages.len()
            ));
        }
        let _: bool = client.abort(&session.id).await.map_err(|e| e.to_string())?;
        Ok(format!("session {} created, listed, aborted", session.id))
    };
    finish_timed(reporter, "session_lifecycle", started, 60, fut).await;
}

/// Forking returns a NEW session id tied to the same server.
async fn fork_session(reporter: &Reporter, client: &OpencodeClient) {
    let started = reporter
        .start("fork_session", "fork returns a distinct new session")
        .await;
    let fut = async {
        let source = client
            .create_session(&params("wirecheck fork source"))
            .await
            .map_err(|e| e.to_string())?;
        let fork = client
            .create_session(&SessionCreateParams {
                parent_id: Some(source.id.clone()),
                ..params("wirecheck fork")
            })
            .await
            .map_err(|e| e.to_string())?;
        if fork.id == source.id {
            return Err("fork returned the source id".to_string());
        }
        Ok(format!("{} → {}", source.id, fork.id))
    };
    finish_timed(reporter, "fork_session", started, 60, fut).await;
}

/// The SSE event stream connects and emits at least one well-formed frame.
async fn event_stream(reporter: &Reporter, client: &OpencodeClient) {
    let started = reporter
        .start(
            "event_stream",
            "SSE stream connects and yields a typed frame",
        )
        .await;
    let fut = async {
        let mut stream = client
            .event_stream(Default::default())
            .map_err(|e| e.to_string())?;
        // Creating a session while subscribed guarantees at least one event.
        let _ = client.create_session(&params("wirecheck sse ping")).await;
        match tokio::time::timeout(Duration::from_secs(20), stream.next()).await {
            Ok(Some(Ok(event))) => Ok(if event.is_connected() {
                "connected frame received".to_string()
            } else {
                "typed event frame received".to_string()
            }),
            Ok(Some(Err(e))) => Err(e.to_string()),
            Ok(None) => Err("stream ended without a frame".to_string()),
            Err(_) => Err("no frame within 20s".to_string()),
        }
    };
    finish_timed(reporter, "event_stream", started, 60, fut).await;
}

async fn finish_timed(
    reporter: &Reporter,
    name: &'static str,
    started: std::time::Instant,
    timeout_s: u64,
    fut: impl std::future::Future<Output = Result<String, String>>,
) {
    match tokio::time::timeout(Duration::from_secs(timeout_s), fut).await {
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
