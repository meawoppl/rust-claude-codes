//! Example of using the synchronous Codex app-server client.
//!
//! Starts a thread, sends a single turn, and prints streaming notifications
//! until the turn completes.

use codex_codes::{
    protocol::methods, ServerMessage, SyncClient, ThreadStartParams, TurnStartParams, UserInput,
};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    println!("Starting Codex app-server...");
    let mut client = SyncClient::start()?;

    // Start a thread
    let thread = client.thread_start(&ThreadStartParams::default())?;
    println!("Thread started: {}", thread.thread_id);

    // Start a turn with a question
    println!("\nSending query: What is the capital of France?\n");
    client.turn_start(&TurnStartParams {
        thread_id: thread.thread_id.clone(),
        input: vec![UserInput::Text {
            text: "What is the capital of France?".to_string(),
        }],
        model: None,
        reasoning_effort: None,
        sandbox_policy: None,
    })?;

    // Iterate notifications until the turn completes
    for result in client.events() {
        match result {
            Ok(msg) => match &msg {
                ServerMessage::Notification { method, params } => match method.as_str() {
                    methods::AGENT_MESSAGE_DELTA => {
                        if let Some(params) = params {
                            if let Some(delta) = params.get("delta").and_then(|d| d.as_str()) {
                                print!("{}", delta);
                            }
                        }
                    }
                    methods::TURN_COMPLETED => {
                        println!("\n[turn completed]");
                        break;
                    }
                    methods::ERROR => {
                        if let Some(params) = params {
                            if let Some(error) = params.get("error").and_then(|e| e.as_str()) {
                                eprintln!("[error] {}", error);
                            }
                        }
                    }
                    _ => {}
                },
                ServerMessage::Request { method, .. } => {
                    eprintln!("[server request: {}] (unhandled)", method);
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
