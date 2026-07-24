//! Testing binary for Claude Code JSON communication using AsyncClient with streaming
//!
//! This binary allows you to send queries to Claude and receive responses,
//! with automatic JSON serialization/deserialization using the new AsyncClient.

use anyhow::Result;
use claude_codes::{AsyncClient, ClaudeOutput};
use log::{debug, error, info};
use std::env;
use std::io::{self, Write};
use tokio::io::AsyncBufReadExt;

/// Main entry point
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging with simple format
    env_logger::init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    let model = if args.len() > 1 {
        args[1].clone()
    } else {
        "opus".to_string()
    };

    info!("Starting Claude test client with model: {}", model);

    // Create AsyncClient
    let mut client = match AsyncClient::with_model(&model).await {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create AsyncClient: {}", e);
            return Err(anyhow::anyhow!("Failed to create AsyncClient: {}", e));
        }
    };

    // Optionally spawn a task to monitor stderr
    if let Some(mut stderr) = client.take_stderr() {
        tokio::spawn(async move {
            let mut line = String::new();
            loop {
                line.clear();
                match stderr.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        if !line.trim().is_empty() {
                            error!("Claude stderr: {}", line.trim());
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

    info!("Claude client initialized successfully");

    println!("\nClaude Test Client");
    println!("=================");
    println!("Using model: {}", model);
    println!("Type your queries and press Enter. Type 'exit' to quit.");
    println!();

    // Main interaction loop
    loop {
        // Prompt for input
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        // Check for exit command
        if input.eq_ignore_ascii_case("exit") {
            info!("Exiting...");
            break;
        }

        // Skip empty inputs
        if input.is_empty() {
            continue;
        }

        // Send query and stream responses
        println!("\n--- Response ---");

        // Get a response stream
        let mut stream = match client.query_stream(input).await {
            Ok(stream) => stream,
            Err(e) => {
                error!("Failed to create query stream: {}", e);
                eprintln!("Error creating query stream: {}", e);
                continue;
            }
        };

        // Stream responses - the stream will terminate after Result message
        while let Some(result) = stream.next().await {
            match result {
                Ok(output) => {
                    debug!("Received {} message", output.message_type());
                    handle_output(output);
                }
                Err(e) => {
                    error!("Error receiving response: {}", e);
                    eprintln!("Error receiving response: {}", e);

                    // Check if client is still alive
                    if !client.is_alive() {
                        eprintln!("Claude process has terminated. Exiting...");
                        return Err(anyhow::anyhow!("Claude process terminated: {}", e));
                    }
                    break;
                }
            }
        }

        println!("--- Response complete ---\n");
    }

    // Clean up
    info!("Shutting down Claude client...");
    if let Err(e) = client.shutdown().await {
        error!("Failed to shutdown client cleanly: {}", e);
    }

    Ok(())
}

/// Handle the output from Claude
fn handle_output(output: ClaudeOutput) {
    match output {
        ClaudeOutput::System(sys) => match sys.subtype.as_str() {
            "init" => {
                debug!("System initialization");
                debug!(
                    "System init data: {}",
                    serde_json::to_string_pretty(&sys.data).unwrap_or_default()
                );
            }
            "confirmation" => {
                debug!("System confirmation received");
            }
            _ => {
                debug!("System message - {}", sys.subtype);
                debug!(
                    "System data: {}",
                    serde_json::to_string_pretty(&sys.data).unwrap_or_default()
                );
            }
        },
        ClaudeOutput::User(msg) => {
            // Usually just an echo of what we sent - don't print it
            debug!("User message echoed: session={:?}", msg.session_id);
        }
        ClaudeOutput::Assistant(msg) => {
            // Process content blocks from the nested message
            for block in &msg.message.content {
                match block {
                    claude_codes::io::ContentBlock::Text(text) => {
                        // Just print the text without labels for cleaner output
                        println!("{}", text.text);
                    }
                    claude_codes::io::ContentBlock::Thinking(thinking) => {
                        println!("\n[Thinking]\n{}\n", thinking.thinking);
                    }
                    claude_codes::io::ContentBlock::ToolUse(tool) => {
                        println!("\n[Tool Request: {}]", tool.name);
                        println!("ID: {}", tool.id);
                        if !tool.input.is_null() {
                            println!(
                                "Input: {}",
                                serde_json::to_string_pretty(&tool.input).unwrap_or_default()
                            );
                        }
                    }
                    claude_codes::io::ContentBlock::ToolResult(result) => {
                        println!("\n[Tool Result for {}]", result.tool_use_id);
                        if let Some(ref content) = result.content {
                            match content {
                                claude_codes::io::ToolResultContent::Text(text) => {
                                    println!("{}", text);
                                }
                                claude_codes::io::ToolResultContent::Structured(data) => {
                                    println!(
                                        "{}",
                                        serde_json::to_string_pretty(&data).unwrap_or_default()
                                    );
                                }
                            }
                        }
                    }
                    claude_codes::io::ContentBlock::Image(image) => {
                        println!(
                            "\n[Image: {} - data length: {}]",
                            image.source.media_type,
                            image.source.data.len()
                        );
                    }
                    claude_codes::io::ContentBlock::ServerToolUse(tool) => {
                        println!("\n[Server Tool: {} ({})]", tool.name, tool.id);
                    }
                    claude_codes::io::ContentBlock::WebSearchToolResult(result) => {
                        println!("\n[Web Search Result for {}]", result.tool_use_id);
                    }
                    claude_codes::io::ContentBlock::CodeExecutionToolResult(result) => {
                        println!("\n[Code Execution Result for {}]", result.tool_use_id);
                    }
                    claude_codes::io::ContentBlock::McpToolUse(tool) => {
                        println!("\n[MCP Tool: {} ({})]", tool.name, tool.id);
                    }
                    claude_codes::io::ContentBlock::McpToolResult(result) => {
                        println!("\n[MCP Tool Result for {}]", result.tool_use_id);
                    }
                    claude_codes::io::ContentBlock::ContainerUpload(_) => {
                        println!("\n[Container Upload]");
                    }
                    claude_codes::io::ContentBlock::Fallback(fallback) => {
                        println!(
                            "\n[Model fallback: {} -> {}]",
                            fallback.from.model, fallback.to.model
                        );
                    }
                    claude_codes::io::ContentBlock::Unknown(value) => {
                        println!(
                            "\n[Unknown block: {}]",
                            serde_json::to_string_pretty(value).unwrap_or_default()
                        );
                    }
                }
            }
        }
        ClaudeOutput::Result(result) => {
            // Only show result details in debug mode for cleaner output
            debug!("Query complete - Status: {:?}", result.subtype);
            debug!(
                "Duration: {}ms (API: {}ms)",
                result.duration_ms, result.duration_api_ms
            );

            if let Some(ref usage) = result.usage {
                info!(
                    "Usage: {} input tokens, {} output tokens",
                    usage.input_tokens, usage.output_tokens
                );
            }

            debug!("Cost: ${:.6}", result.total_cost_usd);
            debug!("Session: {}", result.session_id);

            // Only show errors prominently
            if result.is_error {
                eprintln!("\n⚠️  ERROR: Query resulted in error state");
                if let Some(ref res) = result.result {
                    eprintln!("   Error details: {}", res);
                }
            }
        }
        ClaudeOutput::ControlRequest(req) => {
            debug!("Control request received: {:?}", req.request_id);
        }
        ClaudeOutput::ControlResponse(resp) => {
            debug!("Control response received: {:?}", resp.response);
        }
        ClaudeOutput::Error(err) => {
            eprintln!("\n⚠️  API ERROR: {}", err.error.message);
            eprintln!("   Type: {}", err.error.error_type);
            if let Some(ref req_id) = err.request_id {
                eprintln!("   Request ID: {}", req_id);
            }
        }
        ClaudeOutput::RateLimitEvent(evt) => {
            debug!(
                "Rate limit event: status={}, type={:?}, resets_at={:?}",
                evt.rate_limit_info.status,
                evt.rate_limit_info.rate_limit_type,
                evt.rate_limit_info.resets_at
            );
        }
        ClaudeOutput::StreamEvent(evt) => {
            debug!("Stream event received for session {}", evt.session_id);
        }
        ClaudeOutput::ToolProgress(progress) => {
            debug!(
                "Tool progress: {} ({}) after {:.2}s",
                progress.tool_name, progress.tool_use_id, progress.elapsed_time_seconds
            );
        }
        ClaudeOutput::CommandLifecycle(lifecycle) => {
            debug!("Command {} is {}", lifecycle.command_uuid, lifecycle.state);
        }
        ClaudeOutput::AuthStatus(status) => {
            debug!(
                "Auth status: authenticating={}, lines={}",
                status.is_authenticating,
                status.output.len()
            );
        }
        ClaudeOutput::ToolUseSummary(summary) => {
            debug!(
                "Tool-use summary over {} tool(s): {}",
                summary.preceding_tool_use_ids.len(),
                summary.summary
            );
        }
        ClaudeOutput::PromptSuggestion(suggestion) => {
            debug!("Prompt suggestion: {}", suggestion.suggestion);
        }
        ClaudeOutput::ConversationReset(reset) => {
            debug!(
                "Conversation reset: {} -> {}",
                reset.session_id, reset.new_conversation_id
            );
        }
        ClaudeOutput::TranscriptResult(msg)
        | ClaudeOutput::Progress(msg)
        | ClaudeOutput::QueueOperation(msg)
        | ClaudeOutput::PrLink(msg)
        | ClaudeOutput::FileHistorySnapshot(msg)
        | ClaudeOutput::Summary(msg)
        | ClaudeOutput::Mode(msg)
        | ClaudeOutput::PermissionMode(msg)
        | ClaudeOutput::Attachment(msg)
        | ClaudeOutput::AiTitle(msg)
        | ClaudeOutput::LastPrompt(msg)
        | ClaudeOutput::Started(msg) => {
            debug!("Transcript-only message with {} field(s)", msg.data.len());
        }
    }
}
