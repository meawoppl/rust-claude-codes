//! Interactive REPL for the Codex CLI using the async client.
//!
//! Each query spawns a fresh `codex exec --json -` process.
//! Type your prompt and press Enter. Type "exit" to quit.

use codex_codes::{AsyncClient, CodexCliBuilder, ThreadEvent, ThreadItem};
use log::{debug, error};
use std::env;
use std::io::{self, Write};
use tokio::io::AsyncBufReadExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let model = args.get(1).cloned();

    println!("\nCodex REPL");
    println!("==========");
    if let Some(ref m) = model {
        println!("Using model: {}", m);
    }
    println!("Type your queries and press Enter. Type 'exit' to quit.\n");

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

        let mut builder = CodexCliBuilder::new().full_auto(true);
        if let Some(ref m) = model {
            builder = builder.model(m.as_str());
        }

        let mut client = match AsyncClient::from_builder(builder, input).await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to start Codex: {}", e);
                continue;
            }
        };

        // Drain stderr in the background
        if let Some(mut stderr) = client.take_stderr() {
            tokio::spawn(async move {
                let mut line = String::new();
                loop {
                    line.clear();
                    match stderr.read_line(&mut line).await {
                        Ok(0) => break,
                        Ok(_) => {
                            if !line.trim().is_empty() {
                                error!("Codex stderr: {}", line.trim());
                            }
                        }
                        Err(e) => {
                            error!("Error reading stderr: {}", e);
                            break;
                        }
                    }
                }
            });
        }

        println!("\n--- Response ---");

        let mut stream = client.events();
        while let Some(result) = stream.next().await {
            match result {
                Ok(event) => {
                    debug!("Event: {:?}", std::mem::discriminant(&event));
                    handle_event(&event);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    break;
                }
            }
        }

        println!("--- End ---\n");
    }

    Ok(())
}

fn handle_event(event: &ThreadEvent) {
    match event {
        ThreadEvent::ThreadStarted(e) => {
            debug!("[thread.started] id={}", e.thread_id);
        }
        ThreadEvent::TurnStarted(_) => {
            debug!("[turn.started]");
        }
        ThreadEvent::ItemStarted(_) => {}
        ThreadEvent::ItemUpdated(e) => match &e.item {
            ThreadItem::AgentMessage(msg) => {
                println!("{}", msg.text);
            }
            ThreadItem::CommandExecution(cmd) => {
                println!("\n[Command: {}]", cmd.command);
                for line in cmd.aggregated_output.lines() {
                    println!("  {}", line);
                }
                match cmd.status {
                    codex_codes::CommandExecutionStatus::Completed => {
                        if let Some(code) = cmd.exit_code {
                            if code != 0 {
                                println!("  (exit code: {})", code);
                            }
                        }
                    }
                    codex_codes::CommandExecutionStatus::Failed => {
                        println!("  (FAILED, exit code: {:?})", cmd.exit_code);
                    }
                    _ => {}
                }
            }
            ThreadItem::FileChange(fc) => {
                for change in &fc.changes {
                    println!("\n[File: {} ({:?})]", change.path, change.kind);
                }
            }
            ThreadItem::Reasoning(r) => {
                println!("\n[Thinking]\n{}", r.text);
            }
            ThreadItem::McpToolCall(mcp) => {
                println!("\n[MCP: {}::{}]", mcp.server, mcp.tool);
            }
            ThreadItem::WebSearch(ws) => {
                println!("\n[Web search: {}]", ws.query);
            }
            ThreadItem::TodoList(todo) => {
                println!("\n[Todo list: {} items]", todo.items.len());
                for item in &todo.items {
                    let check = if item.completed { "x" } else { " " };
                    println!("  [{}] {}", check, item.text);
                }
            }
            ThreadItem::Error(err) => {
                eprintln!("\n[Error: {}]", err.message);
            }
        },
        ThreadEvent::ItemCompleted(_) => {}
        ThreadEvent::TurnCompleted(e) => {
            println!(
                "\n[Tokens: {} in / {} out]",
                e.usage.input_tokens, e.usage.output_tokens
            );
        }
        ThreadEvent::TurnFailed(e) => {
            eprintln!("\n[Turn failed: {}]", e.error.message);
        }
        ThreadEvent::Error(e) => {
            eprintln!("\n[Error: {}]", e.message);
        }
    }
}
