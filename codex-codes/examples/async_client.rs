//! Example of using the asynchronous Codex app-server client.
//!
//! Starts a thread, sends a single turn, and prints streaming notifications
//! until the turn completes.

use codex_codes::{
    protocol::methods, AsyncClient, ServerMessage, ThreadStartParams, TurnStartParams, UserInput,
};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    println!("Starting Codex app-server...");
    let mut client = AsyncClient::start().await?;

    // Start a thread
    let thread = client.thread_start(&ThreadStartParams::default()).await?;
    println!("Thread started: {}", thread.thread_id);

    // Start a turn with a question
    println!("\nSending query: What is the capital of France?\n");
    client
        .turn_start(&TurnStartParams {
            thread_id: thread.thread_id.clone(),
            input: vec![UserInput::Text {
                text: "What is the capital of France?".to_string(),
            }],
            model: None,
            reasoning_effort: None,
            sandbox_policy: None,
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
        ServerMessage::Notification { method, params } => {
            match method.as_str() {
                methods::AGENT_MESSAGE_DELTA => {
                    if let Some(params) = params {
                        if let Some(delta) = params.get("delta").and_then(|d| d.as_str()) {
                            print!("{}", delta);
                        }
                    }
                }
                methods::TURN_STARTED => {
                    println!("[turn started]");
                }
                methods::TURN_COMPLETED => {
                    println!("\n[turn completed]");
                    return true;
                }
                methods::ITEM_STARTED => {
                    // Items starting — could inspect the item type
                }
                methods::ITEM_COMPLETED => {
                    // Items completing
                }
                methods::ERROR => {
                    if let Some(params) = params {
                        if let Some(error) = params.get("error").and_then(|e| e.as_str()) {
                            eprintln!("[error] {}", error);
                        }
                    }
                }
                _ => {
                    log::debug!("Notification: {}", method);
                }
            }
            false
        }
        ServerMessage::Request { method, .. } => {
            eprintln!("[server request: {}] (unhandled)", method);
            false
        }
    }
}
