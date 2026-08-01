//! Live integration tests against a running opencode server.
//!
//! Gated behind the `integration-tests` feature so they never run in a plain
//! `cargo test`. They drive the crate's own [`OpencodeClient`] and, when the
//! `server` feature is on, [`ManagedServer`].
//!
//! ```bash
//! # Against an already-running server (default http://127.0.0.1:41999):
//! cargo test -p opencode-codes --features integration-tests -- --nocapture
//!
//! # Point at a different instance:
//! OPENCODE_BASE_URL=http://127.0.0.1:4096 \
//!   cargo test -p opencode-codes --features integration-tests
//!
//! # Include the self-spawned ManagedServer test:
//! cargo test -p opencode-codes --features "integration-tests server"
//! ```
//!
//! Every test is written to survive a provider-less server: assertions target
//! transport behaviour and wire typing (a session gets an id, a listing is a
//! typed `Vec`, an abort answers with a `bool`, the SSE stream emits `data:`
//! frames), never a model's textual output.
#![cfg(feature = "integration-tests")]

use std::time::Duration;

use opencode_codes::client_async::OpencodeClient;
use opencode_codes::protocol_generated::types::{
    Part, PermissionReplyParams, PromptAsyncParams, PromptAsyncParamsPartsItem,
    SessionCreateParams, TextPartInput,
};

/// Base URL of the server under test. Defaults to the local instance the
/// fixtures were captured from; override with `OPENCODE_BASE_URL`.
fn base_url() -> String {
    std::env::var("OPENCODE_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:41999".to_string())
}

/// A client pointed at [`base_url`], reading optional basic-auth from the
/// environment and applying a generous per-request timeout.
fn client() -> OpencodeClient {
    OpencodeClient::builder()
        .base_url(base_url())
        .auth_from_env()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("client builds")
}

/// A minimal, titled create-session request.
fn create_params(title: &str) -> SessionCreateParams {
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

/// A one-text-part prompt body.
fn text_prompt(text: &str) -> PromptAsyncParams {
    PromptAsyncParams {
        agent: None,
        format: None,
        message_id: None,
        model: None,
        no_reply: None,
        parts: vec![PromptAsyncParamsPartsItem::Text(TextPartInput {
            id: None,
            ignored: None,
            metadata: None,
            synthetic: None,
            text: text.to_string(),
            time: None,
            type_: String::new(),
        })],
        system: None,
        tools: None,
        variant: None,
    }
}

/// create -> list -> abort against the live server, asserting only on typing.
#[tokio::test]
async fn create_list_abort_loop() {
    let client = client();

    let session = client
        .create_session(&create_params("opencode-codes create_list_abort"))
        .await
        .expect("create session");
    assert!(
        session.id.starts_with("ses"),
        "session id should be a ses… handle, got {}",
        session.id
    );
    assert_eq!(session.version, "1.18.5", "server version drifted");

    // A brand-new session has no messages yet.
    let messages = client
        .list_messages(&session.id)
        .await
        .expect("list messages");
    assert!(
        messages.is_empty(),
        "expected an empty transcript for a fresh session, got {}",
        messages.len()
    );

    // Abort answers with a boolean even when nothing was running.
    let _aborted: bool = client.abort(&session.id).await.expect("abort");
}

/// Fork a session — the fork must be a NEW `ses…` handle distinct from the
/// source, with the source left intact.
#[tokio::test]
async fn fork_session_returns_new_session() {
    let client = client();

    let source = client
        .create_session(&create_params("opencode-codes fork source"))
        .await
        .expect("create source session");

    let fork = client.fork_session(&source.id).await.expect("fork session");

    assert!(
        fork.id.starts_with("ses"),
        "fork id should be a ses… handle, got {}",
        fork.id
    );
    assert_ne!(fork.id, source.id, "fork must get a NEW session id");

    // Source is untouched and still addressable.
    let messages = client
        .list_messages(&source.id)
        .await
        .expect("source still listable after fork");
    assert!(messages.is_empty());
}

/// Submit a prompt, then reconcile via the authoritative message listing.
///
/// The user message the server records is deterministic and provider-free, so
/// we assert it reappears; the assistant's reply (if any) is model output and is
/// deliberately not required.
#[tokio::test]
async fn prompt_then_reconcile_user_message() {
    let client = client();

    let session = client
        .create_session(&create_params("opencode-codes reconcile probe"))
        .await
        .expect("create session");

    let marker = "opencode-codes reconciliation marker";
    // prompt_async accepts the work (HTTP 204) and returns unit.
    client
        .prompt_async(&session.id, &text_prompt(marker))
        .await
        .expect("prompt accepted");

    // SSE is best-effort; the message listing is the source of truth. Poll it.
    let mut found = false;
    for _ in 0..20 {
        let messages = client
            .list_messages(&session.id)
            .await
            .expect("list messages");
        found = messages.iter().flat_map(|m| &m.parts).any(|p| match p {
            Part::Text(t) => t.text == marker,
            _ => false,
        });
        if found {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    assert!(
        found,
        "prompt's user text part never reconciled into the message listing"
    );
}

/// A reply addressed to an unknown permission id must surface as HTTP 404,
/// exercising the two-channel permission correlation contract's error path.
#[tokio::test]
async fn unknown_permission_reply_is_404() {
    let client = client();

    let session = client
        .create_session(&create_params("opencode-codes permission probe"))
        .await
        .expect("create session");

    let err = client
        .respond_permission(
            &session.id,
            "per_does_not_exist_00000000000000",
            &PermissionReplyParams {
                response: "reject".into(),
            },
        )
        .await
        .expect_err("stale permission id must fail");

    match err {
        opencode_codes::Error::Http { status, .. } => {
            assert_eq!(status, 404, "expected 404 for unknown permission id")
        }
        other => panic!("expected HTTP 404, got {other:?}"),
    }
}

/// The `GET /event` SSE stream must emit at least one `data:` frame promptly —
/// opencode sends a `server.connected` event on connect. Driven with a raw
/// tokio TCP read so it needs no SSE dependency in the test crate.
///
/// Skipped when `OPENCODE_SERVER_PASSWORD` is set, since this raw probe does not
/// perform HTTP basic auth.
#[tokio::test]
async fn event_stream_emits_frames() {
    if std::env::var("OPENCODE_SERVER_PASSWORD")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
    {
        eprintln!("skipping raw SSE probe: OPENCODE_SERVER_PASSWORD is set");
        return;
    }

    let event_url = client().transport().event_url();
    let authority = event_url
        .strip_prefix("http://")
        .expect("event url should be plain http for the raw probe")
        .split('/')
        .next()
        .expect("event url has an authority")
        .to_string();

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = tokio::net::TcpStream::connect(&authority)
        .await
        .expect("connect to event stream");
    let request = format!(
        "GET /event HTTP/1.1\r\nHost: {authority}\r\nAccept: text/event-stream\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .await
        .expect("send SSE request");

    let mut buf = vec![0u8; 8192];
    let mut acc = String::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        assert!(
            !remaining.is_zero(),
            "SSE stream produced no data: frame within the timeout; saw:\n{acc}"
        );
        let n = match tokio::time::timeout(remaining, stream.read(&mut buf)).await {
            Ok(Ok(0)) => panic!("SSE stream closed before emitting a frame; saw:\n{acc}"),
            Ok(Ok(n)) => n,
            Ok(Err(e)) => panic!("SSE stream read error: {e}"),
            Err(_) => panic!("SSE stream timed out; saw:\n{acc}"),
        };
        acc.push_str(&String::from_utf8_lossy(&buf[..n]));
        if let Some(idx) = acc.find("data:") {
            // Confirm the frame body is JSON carrying a `type` discriminant.
            let rest = &acc[idx + "data:".len()..];
            if let Some(end) = rest.find('\n') {
                let payload = rest[..end].trim();
                let value: serde_json::Value =
                    serde_json::from_str(payload).expect("SSE data frame is JSON");
                assert!(
                    value
                        .get("type")
                        .and_then(serde_json::Value::as_str)
                        .is_some(),
                    "SSE event frame lacks a `type` field: {payload}"
                );
                break;
            }
        }
    }
}

/// End-to-end against a server this test spawns and tears down itself.
///
/// Requires the `server` feature (`ManagedServer`). Tolerant of environments
/// without the `opencode` binary on `PATH`: a spawn failure is reported and the
/// test returns rather than failing.
#[cfg(feature = "server")]
#[tokio::test]
async fn managed_server_create_list_abort() {
    use opencode_codes::server::ManagedServer;

    // `opencode` is looked up on PATH by default; override with OPENCODE_BINARY
    // (e.g. an npm-local install) when it is not on PATH.
    let binary = std::env::var("OPENCODE_BINARY").unwrap_or_else(|_| "opencode".to_string());
    let server = match ManagedServer::builder()
        .binary(binary)
        .startup_timeout(Duration::from_secs(20))
        .spawn()
        .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("skipping ManagedServer test: could not spawn opencode ({e})");
            return;
        }
    };

    let client = OpencodeClient::builder()
        .base_url(server.url())
        .timeout(Duration::from_secs(30))
        .build()
        .expect("client builds");

    let session = client
        .create_session(&create_params("opencode-codes managed probe"))
        .await
        .expect("create session on managed server");
    assert!(session.id.starts_with("ses"));

    let messages = client
        .list_messages(&session.id)
        .await
        .expect("list messages");
    assert!(messages.is_empty());

    let _aborted: bool = client.abort(&session.id).await.expect("abort");

    server.stop().await.expect("stop managed server");
}
