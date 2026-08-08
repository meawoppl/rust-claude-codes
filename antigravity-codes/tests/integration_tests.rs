//! Tests that drive a real `localharness` process.
//!
//! Enable with `--features integration-tests`, and point
//! `ANTIGRAVITY_HARNESS_PATH` at a binary (or put `localharness` on `PATH`):
//!
//! ```sh
//! pip download google-antigravity --no-deps -d /tmp/ag
//! unzip -o -j /tmp/ag/*.whl 'google/antigravity/bin/localharness' -d /tmp/ag
//! ANTIGRAVITY_HARNESS_PATH=/tmp/ag/localharness \
//!   cargo test -p antigravity-codes --features integration-tests
//! ```
//!
//! Most of these run **without** a Gemini API key. The harness performs the
//! handshake, the initialize exchange, and the whole turn lifecycle locally,
//! and only fails when it reaches out to the model — which makes the failure
//! path itself a precise, offline test of trajectory handling. The one test
//! that needs a real key skips itself when `GEMINI_API_KEY` is unset.

#![cfg(feature = "integration-tests")]

use antigravity_codes::protocol::{InputEvent, StepUpdateTarget};
use antigravity_codes::{Client, Error, HarnessOptions, ModelBuilder, RawClient, Result};

/// A key the Gemini endpoint will reject, for tests that never mean to reach it.
const REJECTED_KEY: &str = "not-a-real-key";

fn harness_available() -> bool {
    let available = antigravity_codes::process::find_harness().is_ok();
    if !available {
        eprintln!("skipping: no localharness binary (set ANTIGRAVITY_HARNESS_PATH)");
    }
    available
}

fn workspace() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("antigravity-codes-tests");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn options(api_key: &str) -> HarnessOptions {
    HarnessOptions::new()
        .workspace(workspace())
        .model(ModelBuilder::gemini("gemini-3-pro-preview", api_key))
}

#[tokio::test]
async fn launching_completes_the_handshake_and_initialize() -> Result<()> {
    if !harness_available() {
        return Ok(());
    }
    let client = RawClient::launch(options(REJECTED_KEY)).await?;

    let cascade_id = client
        .cascade_id()
        .expect("initialize reply carries a cascade id");
    assert_eq!(
        cascade_id.len(),
        32,
        "cascade id is a 128-bit hex string: {cascade_id}"
    );
    assert!(client.harness().port() > 0);
    assert!(!client.harness().api_key().is_empty());
    assert!(!client.is_closed());

    client.shutdown().await
}

#[tokio::test]
async fn a_session_with_no_model_is_rejected_with_stderr_attached() {
    if !harness_available() {
        return;
    }
    // The harness treats "no model configured" as fatal: it exits and drops the
    // socket without ever sending an error frame, so the only diagnosis
    // available is the process's stderr. Asserting that the crate surfaces it
    // is the difference between a usable error and "connection closed".
    let result = RawClient::launch(HarnessOptions::new().workspace(workspace())).await;

    match result {
        Err(Error::HandshakeFailed { stderr }) => {
            assert!(
                !stderr.is_empty(),
                "the failure should carry the harness's stderr"
            );
        }
        Err(other) => panic!("expected a handshake failure, got {other:?}"),
        Ok(_) => panic!("a session with no model should not initialise"),
    }
}

#[tokio::test]
async fn a_turn_streams_steps_and_reports_model_failure() -> Result<()> {
    if !harness_available() {
        return Ok(());
    }
    let mut client = Client::launch(options(REJECTED_KEY)).await?;

    let mut steps = Vec::new();
    let mut turn = client.send("list the files here").await?;
    let outcome = loop {
        match turn.next_step().await {
            Ok(Some(step)) => steps.push(step),
            Ok(None) => break Ok(()),
            Err(e) => break Err(e),
        }
    };

    // The harness echoes the prompt back as the first step, addressed to the
    // model rather than the user.
    let echo = steps.first().expect("at least the prompt echo arrives");
    assert_eq!(echo.text, "list the files here");
    assert_eq!(echo.target, StepUpdateTarget::Model);
    assert!(echo.is_final());

    // …and the turn ends by reporting why the model call failed.
    match outcome {
        Err(Error::Turn { message }) => {
            assert!(
                message.contains("API key not valid") || message.contains("API_KEY_INVALID"),
                "unexpected turn failure: {message}"
            );
        }
        other => panic!("expected the rejected key to fail the turn, got {other:?}"),
    }

    client.shutdown().await
}

#[tokio::test]
async fn the_conversation_id_round_trips_through_storage() -> Result<()> {
    if !harness_available() {
        return Ok(());
    }
    let storage = workspace().join("resume");
    std::fs::create_dir_all(&storage).unwrap();

    let first = RawClient::launch(options(REJECTED_KEY).storage_directory(&storage)).await?;
    let cascade_id = first.cascade_id().unwrap().to_string();
    first.shutdown().await?;

    let second = RawClient::launch(
        options(REJECTED_KEY)
            .storage_directory(&storage)
            .cascade_id(&cascade_id),
    )
    .await?;
    assert_eq!(second.cascade_id(), Some(cascade_id.as_str()));
    second.shutdown().await
}

#[tokio::test]
async fn halting_is_accepted_mid_session() -> Result<()> {
    if !harness_available() {
        return Ok(());
    }
    let mut client = RawClient::launch(options(REJECTED_KEY)).await?;
    client.send(&InputEvent::user("count to a million")).await?;
    client.send(&InputEvent::halt()).await?;

    // Drain until the harness settles; the point is that neither frame is
    // rejected and the socket survives both.
    while let Some(event) = client.next_event().await? {
        if event.trajectory_state_update.is_some() {
            break;
        }
    }
    client.shutdown().await
}

#[tokio::test]
async fn a_real_key_produces_a_real_reply() -> Result<()> {
    if !harness_available() {
        return Ok(());
    }
    let Ok(api_key) = std::env::var("GEMINI_API_KEY") else {
        eprintln!("skipping: GEMINI_API_KEY is not set");
        return Ok(());
    };

    let mut client = Client::launch(options(&api_key)).await?;
    let mut turn = client.send("Reply with exactly the word: pong").await?;
    let text = turn.collect_text().await?;

    assert!(
        text.to_lowercase().contains("pong"),
        "the model should have answered the prompt, got: {text}"
    );
    assert!(client.usage().is_some(), "a completed turn reports usage");

    client.shutdown().await
}
