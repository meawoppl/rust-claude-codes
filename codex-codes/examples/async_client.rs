//! Example of using the asynchronous Codex client.

use codex_codes::{AsyncClient, ThreadEvent, ThreadItem};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    println!("Sending query: What is the capital of France?");
    println!("Waiting for response...\n");

    let mut client = AsyncClient::exec("What is the capital of France?").await?;

    let mut stream = client.events();
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => handle_event(&event),
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }

    println!("\nDone.");
    Ok(())
}

fn handle_event(event: &ThreadEvent) {
    match event {
        ThreadEvent::ThreadStarted(e) => {
            println!("[thread.started] id={}", e.thread_id);
        }
        ThreadEvent::TurnStarted(_) => {
            println!("[turn.started]");
        }
        ThreadEvent::ItemStarted(_) => {}
        ThreadEvent::ItemUpdated(e) => match &e.item {
            ThreadItem::AgentMessage(msg) => {
                println!("  Agent: {}", msg.text);
            }
            ThreadItem::CommandExecution(cmd) => {
                println!("  Command: {}", cmd.command);
                for line in cmd.aggregated_output.lines().take(5) {
                    println!("    > {}", line);
                }
            }
            ThreadItem::FileChange(fc) => {
                for change in &fc.changes {
                    println!("  File change: {} ({:?})", change.path, change.kind);
                }
            }
            _ => {}
        },
        ThreadEvent::ItemCompleted(_) => {}
        ThreadEvent::TurnCompleted(e) => {
            println!(
                "\n[turn.completed] tokens: {} in / {} out",
                e.usage.input_tokens, e.usage.output_tokens
            );
        }
        ThreadEvent::TurnFailed(e) => {
            eprintln!("[turn.failed] {}", e.error.message);
        }
        ThreadEvent::Error(e) => {
            eprintln!("[error] {}", e.message);
        }
    }
}
