//! Example of using the synchronous Codex app-server client.
//!
//! Starts a thread, sends a single turn, and prints streaming notifications
//! until the turn completes.

use codex_codes::{
    Notification, ServerMessage, SyncClient, ThreadStartParams, TurnStartParams, UserInput,
};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    println!("Starting Codex app-server...");
    let mut client = SyncClient::start()?;

    // Start a thread
    let thread = client.thread_start(
        &serde_json::from_value::<ThreadStartParams>(serde_json::json!({})).unwrap(),
    )?;
    println!("Thread started: {}", thread.thread.id.as_str());

    // Start a turn with a question
    println!("\nSending query: What is the capital of France?\n");
    client.turn_start(&TurnStartParams {
        thread_id: thread.thread.id.clone(),
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
    })?;

    // Iterate notifications until the turn completes
    for result in client.events() {
        match result {
            Ok(msg) => match msg {
                ServerMessage::Notification(Notification::AgentMessageDelta(d)) => {
                    print!("{}", d.delta);
                }
                ServerMessage::Notification(Notification::TurnCompleted(_)) => {
                    println!("\n[turn completed]");
                    break;
                }
                ServerMessage::Notification(Notification::Error(e)) => {
                    eprintln!("[error] {}", e.error.message);
                }
                ServerMessage::Notification(_) => {}
                ServerMessage::Request { request, .. } => {
                    eprintln!("[server request: {}] (unhandled)", request.method());
                }
            },
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }

    println!("\nDone.");
    Ok(())
}
