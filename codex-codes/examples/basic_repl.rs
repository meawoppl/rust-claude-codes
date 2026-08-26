//! Interactive REPL for the Codex CLI using the async app-server client.
//!
//! Maintains a single persistent thread across the session.
//! Type your prompt and press Enter. Type "exit" to quit.

use codex_codes::{
    AsyncClient, CommandExecutionApprovalDecision, CommandExecutionRequestApprovalResponse,
    FileChangeApprovalDecision, FileChangeRequestApprovalResponse, Notification, ServerMessage,
    ServerRequest, ThreadItem, ThreadStartParams, TurnStartParams, UserInput,
};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("\nCodex REPL (app-server)");
    println!("======================");
    println!("Type your queries and press Enter. Type 'exit' to quit.\n");

    let mut client = AsyncClient::start().await?;
    let thread = client
        .thread_start(&serde_json::from_value::<ThreadStartParams>(serde_json::json!({})).unwrap())
        .await?;
    let thread_id = thread.thread.id.clone();
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
                ServerMessage::Notification(n) => match n {
                    Notification::AgentMessageDelta(d) => {
                        print!("{}", d.delta);
                        io::stdout().flush()?;
                    }
                    Notification::CmdOutputDelta(d) => {
                        print!("{}", d.delta);
                        io::stdout().flush()?;
                    }
                    Notification::ReasoningDelta(d) => {
                        print!("[thinking] {}", d.delta);
                    }
                    Notification::ItemStarted(item_event) => match item_event.item {
                        ThreadItem::CommandExecution { command, .. } => {
                            println!("\n[Command: {}]", command);
                        }
                        ThreadItem::FileChange { .. } => {
                            println!("\n[File change]");
                        }
                        _ => {}
                    },
                    Notification::TurnCompleted(_) => {
                        println!();
                        break;
                    }
                    Notification::Error(e) => {
                        eprintln!("\n[Error: {}]", e.error.message);
                    }
                    other => {
                        log::debug!("Notification: {}", other.method());
                    }
                },
                ServerMessage::Request { id, request } => match request {
                    ServerRequest::CmdExecApproval(p) => {
                        println!(
                            "\n[Approving command: {}]",
                            p.command.as_deref().unwrap_or("<no command>")
                        );
                        client
                            .respond(
                                id,
                                &CommandExecutionRequestApprovalResponse {
                                    decision: CommandExecutionApprovalDecision::Accept,
                                },
                            )
                            .await?;
                    }
                    ServerRequest::FileChangeApproval(_) => {
                        println!("\n[Approving file change]");
                        client
                            .respond(
                                id,
                                &FileChangeRequestApprovalResponse {
                                    decision: FileChangeApprovalDecision::Accept,
                                },
                            )
                            .await?;
                    }
                    ServerRequest::Unknown { method, .. } => {
                        eprintln!("[unhandled server request: {}]", method);
                    }
                    other => {
                        eprintln!("[unhandled server request: {}]", other.method());
                    }
                },
            }
        }

        println!("--- End ---\n");
    }

    client.shutdown().await?;
    Ok(())
}
