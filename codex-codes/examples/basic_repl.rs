//! Interactive REPL for the Codex CLI using the async app-server client.
//!
//! Maintains a single persistent thread across the session.
//! Type your prompt and press Enter. Type "exit" to quit.

use codex_codes::{
    protocol::methods, AsyncClient, CommandApprovalDecision, CommandExecutionApprovalResponse,
    FileChangeApprovalDecision, FileChangeApprovalResponse, ServerMessage, ThreadStartParams,
    TurnStartParams, UserInput,
};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("\nCodex REPL (app-server)");
    println!("======================");
    println!("Type your queries and press Enter. Type 'exit' to quit.\n");

    let mut client = AsyncClient::start().await?;
    let thread = client.thread_start(&ThreadStartParams::default()).await?;
    let thread_id = thread.thread_id().to_string();
    println!("Thread: {}\n", thread_id);

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            break;
        }

        if input.is_empty() {
            continue;
        }

        // Start a new turn with the user's input
        client
            .turn_start(&TurnStartParams {
                thread_id: thread_id.clone(),
                input: vec![UserInput::Text {
                    text: input.to_string(),
                }],
                model: None,
                reasoning_effort: None,
                sandbox_policy: None,
            })
            .await?;

        println!("\n--- Response ---");

        // Stream events until the turn completes
        loop {
            let msg = match client.next_message().await? {
                Some(m) => m,
                None => {
                    eprintln!("[connection closed]");
                    return Ok(());
                }
            };

            match msg {
                ServerMessage::Notification { method, params } => match method.as_str() {
                    methods::AGENT_MESSAGE_DELTA => {
                        if let Some(ref p) = params {
                            if let Some(delta) = p.get("delta").and_then(|d| d.as_str()) {
                                print!("{}", delta);
                                io::stdout().flush()?;
                            }
                        }
                    }
                    methods::CMD_OUTPUT_DELTA => {
                        if let Some(ref p) = params {
                            if let Some(delta) = p.get("delta").and_then(|d| d.as_str()) {
                                print!("{}", delta);
                                io::stdout().flush()?;
                            }
                        }
                    }
                    methods::REASONING_DELTA => {
                        if let Some(ref p) = params {
                            if let Some(delta) = p.get("delta").and_then(|d| d.as_str()) {
                                print!("[thinking] {}", delta);
                            }
                        }
                    }
                    methods::ITEM_STARTED => {
                        if let Some(ref p) = params {
                            if let Some(item) = p.get("item") {
                                if let Some(ty) = item.get("type").and_then(|t| t.as_str()) {
                                    match ty {
                                        "commandExecution" | "command_execution" => {
                                            if let Some(cmd) =
                                                item.get("command").and_then(|c| c.as_str())
                                            {
                                                println!("\n[Command: {}]", cmd);
                                            }
                                        }
                                        "fileChange" | "file_change" => {
                                            println!("\n[File change]");
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                    methods::TURN_COMPLETED => {
                        println!();
                        break;
                    }
                    methods::ERROR => {
                        if let Some(ref p) = params {
                            if let Some(error) = p.get("error").and_then(|e| e.as_str()) {
                                eprintln!("\n[Error: {}]", error);
                            }
                        }
                    }
                    _ => {
                        log::debug!("Notification: {}", method);
                    }
                },
                ServerMessage::Request { id, method, params } => match method.as_str() {
                    methods::CMD_EXEC_APPROVAL => {
                        if let Some(ref p) = params {
                            if let Some(cmd) = p.get("command").and_then(|c| c.as_str()) {
                                println!("\n[Approving command: {}]", cmd);
                            }
                        }
                        client
                            .respond(
                                id,
                                &CommandExecutionApprovalResponse {
                                    decision: CommandApprovalDecision::Accept,
                                },
                            )
                            .await?;
                    }
                    methods::FILE_CHANGE_APPROVAL => {
                        println!("\n[Approving file change]");
                        client
                            .respond(
                                id,
                                &FileChangeApprovalResponse {
                                    decision: FileChangeApprovalDecision::Accept,
                                },
                            )
                            .await?;
                    }
                    _ => {
                        eprintln!("[unhandled server request: {}]", method);
                    }
                },
            }
        }

        println!("--- End ---\n");
    }

    client.shutdown().await?;
    Ok(())
}
