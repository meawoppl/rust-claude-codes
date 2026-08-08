//! Records every frame of a session to disk, for use as test fixtures.
//!
//! ```sh
//! export ANTIGRAVITY_HARNESS_PATH=/path/to/localharness
//! cargo run -p antigravity-codes --example capture_frames -- ./antigravity-codes/test_cases/events "hello"
//! ```
//!
//! Frames are written as `NNN-<kind>.json`. A frame that fails to decode is
//! still written, with a `-undecodable` suffix — those are the interesting
//! ones, and they belong in a bug report.

use std::path::PathBuf;

use antigravity_codes::protocol::{InputEvent, OutputEvent};
use antigravity_codes::{HarnessOptions, ModelBuilder, RawClient, Result};

fn kind_of(event: &OutputEvent) -> &'static str {
    if event.step_update.is_some() {
        "step-update"
    } else if event.trajectory_state_update.is_some() {
        "trajectory-state"
    } else if event.tool_call.is_some() {
        "tool-call"
    } else if event.initialize_conversation_response.is_some() {
        "initialize-response"
    } else if event.call_hook_request.is_some() {
        "call-hook-request"
    } else if event.policy_decision_request.is_some() {
        "policy-decision-request"
    } else if event.usage_update.is_some() {
        "usage-update"
    } else if event.session_end_response.is_some() {
        "session-end-response"
    } else {
        "unknown"
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let mut args = std::env::args().skip(1);
    let out: PathBuf = args.next().unwrap_or_else(|| "captures".into()).into();
    let prompt = args.next().unwrap_or_else(|| "Say hello.".into());
    std::fs::create_dir_all(&out)?;

    let mut client = RawClient::launch(
        HarnessOptions::new()
            .workspace(std::env::current_dir()?)
            .model(ModelBuilder::gemini(
                std::env::var("ANTIGRAVITY_MODEL").unwrap_or_else(|_| "gemini-flash-latest".into()),
                std::env::var("GEMINI_API_KEY").unwrap_or_default(),
            )),
    )
    .await?;

    // The initialize reply was consumed during launch; write it back out so the
    // capture is a complete session.
    let initialize = serde_json::to_value(client.initialize_response())
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    std::fs::write(
        out.join("000-initialize-response.json"),
        serde_json::to_string_pretty(&initialize).unwrap() + "\n",
    )?;

    client.send(&InputEvent::user(prompt)).await?;

    let mut n = 1;
    while let Some(event) = client.next_event().await? {
        let value = serde_json::to_value(&event).unwrap_or(serde_json::Value::Null);
        let name = format!("{n:03}-{}.json", kind_of(&event));
        std::fs::write(
            out.join(name),
            serde_json::to_string_pretty(&value).unwrap() + "\n",
        )?;
        n += 1;
        if event.trajectory_state_update.is_some() {
            let state = event
                .trajectory_state_update
                .as_ref()
                .and_then(|t| t.state.clone());
            println!("trajectory -> {state:?}");
            if matches!(state.as_ref().map(|s| s.as_str()), Some("STATE_FULLY_IDLE")) {
                break;
            }
        }
    }

    println!("wrote {n} frames to {}", out.display());
    client.shutdown().await
}
