//! Example of using the asynchronous Codex app-server client.
//!
//! Starts a thread, sends a single turn, and prints streaming notifications
//! until the turn completes.

use codex_codes::{
    AsyncClient, Notification, ServerMessage, ThreadStartParams, TurnStartParams, UserInput,
};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    println!("Starting Codex app-server...");
    let mut client = AsyncClient::start().await?;

    // Start a thread
    let thread = client
        .thread_start(&serde_json::from_value::<ThreadStartParams>(serde_json::json!({})).unwrap())
        .await?;
    println!("Thread started: {}", thread.thread.id.as_str());

    // Start a turn with a question
    println!("\nSending query: What is the capital of France?\n");
    client
        .turn_start(&TurnStartParams {
            thread_id: thread.thread.id.clone(),
            tool_output: None,
            input: vec![UserInput::Text {
                text: "What is the capital of France?".to_string(),
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
        .await?;

    // Stream notifications until the turn completes
    let mut stream = client.events();
    while let Some(result) = stream.next().await {
        match result {
            Ok(msg) => {
                if handle_message(&msg) {
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }

    println!("\nDone.");
    client.shutdown().await?;
    Ok(())
}

/// Handle a server message. Returns true if the turn is complete.
fn handle_message(msg: &ServerMessage) -> bool {
    match msg {
        ServerMessage::Notification(n) => match n {
            Notification::AgentMessageDelta(d) => {
                print!("{}", d.delta);
                false
            }
            Notification::TurnStarted(_) => {
                println!("[turn started]");
                false
            }
            Notification::TurnCompleted(_) => {
                println!("\n[turn completed]");
                true
            }
            Notification::Error(e) => {
                eprintln!("[error] {}", e.error.message);
                false
            }
            other => {
                log::debug!("Notification: {}", other.method());
                false
            }
        },
        ServerMessage::Request { request, .. } => {
            eprintln!("[server request: {}] (unhandled)", request.method());
            false
        }
    }
}
