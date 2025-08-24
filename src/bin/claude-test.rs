//! Testing binary for Claude Code JSON communication
//!
//! This binary allows you to send queries to Claude and receive responses,
//! with automatic JSON serialization/deserialization.

use anyhow::{Context, Result};
use claude_codes::{ClaudeCliBuilder, ClaudeInput, ClaudeOutput, Protocol};
use serde::de::Error as SerdeError;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Child;
use tracing::{debug, error, info, warn};

/// Main entry point
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with simple format
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    let model = if args.len() > 1 {
        args[1].clone()
    } else {
        "sonnet".to_string()
    };

    info!("Starting Claude test client with model: {}", model);

    // Start Claude in JSON streaming mode
    let (mut claude, stderr) = start_claude(&model).await?;

    // Spawn a task to monitor stderr
    tokio::spawn(async move {
        let mut stderr = stderr;
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

    info!("Claude process started successfully");

    // Note: Claude doesn't send any messages before the first user input
    debug!("Ready to accept user input");

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

        // Send the query to Claude
        match send_query(&mut claude, input).await {
            Ok(()) => {
                // Expected flow after sending a message:
                // 1. System message (always sent with each response)
                // 2. User message echo (our message echoed back)
                // 3. Zero or more Assistant messages (the actual response)
                // 4. Result message (completion with metrics)

                println!("\n--- Waiting for response ---");

                let mut received_result = false;
                let mut received_system = false;
                let mut received_user = false;
                let mut assistant_count = 0;

                while !received_result {
                    match read_response(&mut claude).await {
                        Ok(output) => {
                            match &output {
                                ClaudeOutput::System(_) => {
                                    if received_system {
                                        warn!("Received multiple System messages in one response cycle");
                                    }
                                    received_system = true;
                                    debug!("Received System message");
                                }
                                ClaudeOutput::User(_) => {
                                    if received_user {
                                        warn!(
                                            "Received multiple User messages in one response cycle"
                                        );
                                    }
                                    received_user = true;
                                    debug!("Received User message echo");
                                }
                                ClaudeOutput::Assistant(_) => {
                                    assistant_count += 1;
                                    debug!("Received Assistant message #{}", assistant_count);
                                }
                                ClaudeOutput::Result(_) => {
                                    received_result = true;
                                    debug!("Received Result message - response complete");
                                }
                            }

                            handle_output(output);
                        }
                        Err(e) => {
                            error!("Failed to read response: {}", e);
                            eprintln!("Error reading response: {}", e);

                            // If we've received at least some response, continue
                            if received_system || assistant_count > 0 {
                                eprintln!("Partial response received before error");
                                break;
                            }
                            return Err(e);
                        }
                    }
                }

                // Log what we received for debugging
                debug!(
                    "Response complete - System: {}, User: {}, Assistant: {}, Result: {}",
                    received_system, received_user, assistant_count, received_result
                );

                println!(
                    "--- Response complete (received {} assistant messages) ---\n",
                    assistant_count
                );
            }
            Err(e) => {
                error!("Failed to send query: {}", e);
                eprintln!("Error sending query: {}", e);
                return Err(e);
            }
        }
    }

    // Clean up
    info!("Terminating Claude process...");
    claude.child.kill().await?;

    Ok(())
}

/// Claude process wrapper
struct ClaudeProcess {
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
}

/// Start the Claude process
async fn start_claude(
    model: &str,
) -> Result<(ClaudeProcess, BufReader<tokio::process::ChildStderr>)> {
    let mut child = ClaudeCliBuilder::new()
        .model(model)
        .spawn()
        .await
        .context("Failed to spawn Claude process")?;

    let stdin = child.stdin.take().context("Failed to get stdin handle")?;
    let stdout = BufReader::new(child.stdout.take().context("Failed to get stdout handle")?);
    let stderr = BufReader::new(child.stderr.take().context("Failed to get stderr handle")?);

    Ok((
        ClaudeProcess {
            child,
            stdin,
            stdout,
        },
        stderr,
    ))
}

/// Send a query to Claude
async fn send_query(claude: &mut ClaudeProcess, query: &str) -> Result<()> {
    info!("Sending query: {}", query);

    // Create the input message with default session ID
    let input = ClaudeInput::user_message(query, "default");

    // Serialize to JSON
    let json_line = Protocol::serialize(&input).context("Failed to serialize input")?;

    debug!("[OUTGOING] Sending JSON to Claude: {}", json_line.trim());

    // Send to Claude
    if let Err(e) = claude.stdin.write_all(json_line.as_bytes()).await {
        error!("Failed to write to stdin: {}", e);

        // Try to read any stdout that might have been produced
        let mut out_line = String::new();
        let _ = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            claude.stdout.read_line(&mut out_line),
        )
        .await;

        if !out_line.trim().is_empty() {
            error!("Claude stdout before failure: {}", out_line.trim());
        }

        return Err(anyhow::anyhow!("Failed to write to stdin: {}", e));
    }

    claude
        .stdin
        .flush()
        .await
        .context("Failed to flush stdin")?;

    Ok(())
}

/// Read a response from Claude
async fn read_response(claude: &mut ClaudeProcess) -> Result<ClaudeOutput> {
    let mut line = String::new();

    info!("Reading response from Claude...");

    // Read lines until we get a complete response
    loop {
        line.clear();
        let bytes_read = claude
            .stdout
            .read_line(&mut line)
            .await
            .context("Failed to read from stdout")?;

        if bytes_read == 0 {
            error!("Claude process closed unexpectedly");
            return Err(anyhow::anyhow!("Claude process terminated"));
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        debug!("[INCOMING] Received JSON from Claude: {}", trimmed);

        // Try to parse as ClaudeOutput
        match ClaudeOutput::parse_json(trimmed) {
            Ok(output) => {
                info!("Successfully parsed ClaudeOutput");
                debug!(
                    "[INCOMING] Parsed output type: {:?}",
                    std::mem::discriminant(&output)
                );

                // Non-streaming response, return immediately
                return Ok(output);
            }
            Err(parse_error) => {
                // Save failed test case with timestamp filename
                let error_msg = format!("{}", parse_error);
                let fake_serde_error = serde_json::Error::custom(&error_msg);
                let saved_file = save_test_case(trimmed, &fake_serde_error);

                // Print the raw JSON that failed to parse
                error!("[INCOMING] Failed to deserialize response: {}", parse_error);
                debug!("[INCOMING] Raw JSON that failed: {}", trimmed);
                eprintln!("\n=== DESERIALIZATION ERROR ===");
                eprintln!("Failed to parse response as ClaudeOutput");
                eprintln!("Error: {}", parse_error.error_message);
                eprintln!("\nRaw JSON received:");
                eprintln!(
                    "{}",
                    serde_json::to_string_pretty(&parse_error.raw_json)
                        .unwrap_or_else(|_| trimmed.to_string())
                );
                eprintln!("=============================\n");

                match saved_file {
                    Ok(filename) => eprintln!(
                        "✓ Test case saved: test_cases/failed_deserializations/{}",
                        filename
                    ),
                    Err(save_err) => eprintln!("✗ Failed to save test case: {}", save_err),
                }

                // Return an error
                return Err(anyhow::anyhow!(
                    "Failed to deserialize response: {}",
                    parse_error
                ));
            }
        }
    }
}

/// Save a failed deserialization as a test case
fn save_test_case(json: &str, error: &serde_json::Error) -> Result<String> {
    // Create test cases directory if it doesn't exist
    let test_dir = PathBuf::from("test_cases/failed_deserializations");
    fs::create_dir_all(&test_dir).context("Failed to create test_cases directory")?;

    // Generate a unique filename based on timestamp (YYMMDD_HHMMSS format)
    let mut filepath: PathBuf;
    let mut filename: String;

    loop {
        let timestamp = chrono::Local::now().format("%y%m%d_%H%M%S");
        let millis = chrono::Local::now().timestamp_subsec_millis();
        filename = format!("failed_{}_{:03}.json", timestamp, millis);
        filepath = test_dir.join(&filename);

        // If file doesn't exist, we can use this name
        if !filepath.exists() {
            break;
        }

        // Wait for a second to avoid collision
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    // Create a test case with metadata
    let test_case = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "error": error.to_string(),
        "raw_json": json,
        "pretty_json": serde_json::from_str::<serde_json::Value>(json)
            .ok()
            .and_then(|v| serde_json::to_string_pretty(&v).ok())
            .unwrap_or_else(|| json.to_string()),
    });

    // Write the test case
    let content = serde_json::to_string_pretty(&test_case)?;
    fs::write(&filepath, content)
        .with_context(|| format!("Failed to write test case to {:?}", filepath))?;

    info!("Saved test case to {:?}", filepath);
    Ok(filename)
}

/// Handle the output from Claude
fn handle_output(output: ClaudeOutput) {
    match output {
        ClaudeOutput::System(sys) => match sys.subtype.as_str() {
            "init" => {
                println!("\n[System Initialization]");
                debug!(
                    "System init data: {}",
                    serde_json::to_string_pretty(&sys.data).unwrap()
                );
            }
            "confirmation" => {
                debug!("System confirmation received");
            }
            _ => {
                println!("\n[System Message - {}]", sys.subtype);
                debug!(
                    "System data: {}",
                    serde_json::to_string_pretty(&sys.data).unwrap()
                );
            }
        },
        ClaudeOutput::User(msg) => {
            // Usually just an echo of what we sent
            debug!("User message echoed: session={:?}", msg.session_id);
        }
        ClaudeOutput::Assistant(msg) => {
            println!("\n[Assistant Response]");
            // Process content blocks from the nested message
            for block in &msg.message.content {
                match block {
                    claude_codes::io::ContentBlock::Text(text) => {
                        println!("{}", text.text);
                    }
                    claude_codes::io::ContentBlock::Thinking(thinking) => {
                        debug!("Claude's thinking: {}", thinking.thinking);
                    }
                    claude_codes::io::ContentBlock::ToolUse(tool) => {
                        println!("[Tool Request: {}]", tool.name);
                        println!("ID: {}", tool.id);
                        println!(
                            "Input: {}",
                            serde_json::to_string_pretty(&tool.input).unwrap()
                        );
                    }
                    claude_codes::io::ContentBlock::ToolResult(result) => {
                        println!("[Tool Result for {}]", result.tool_use_id);
                        if let Some(ref content) = result.content {
                            match content {
                                claude_codes::io::ToolResultContent::Text(text) => {
                                    println!("Result: {}", text);
                                }
                                claude_codes::io::ToolResultContent::Structured(data) => {
                                    println!(
                                        "Result: {}",
                                        serde_json::to_string_pretty(&data).unwrap()
                                    );
                                }
                            }
                        }
                    }
                }
            }
            debug!("Model: {}", msg.message.model);
        }
        ClaudeOutput::Result(result) => {
            println!("\n[Result - Query Complete]");
            println!("├─ Status: {:?}", result.subtype);
            println!(
                "├─ Duration: {}ms (API: {}ms)",
                result.duration_ms, result.duration_api_ms
            );
            if let Some(ref usage) = result.usage {
                println!(
                    "├─ Tokens: {} in, {} out",
                    usage.input_tokens, usage.output_tokens
                );
            }
            println!("├─ Cost: ${:.6}", result.total_cost_usd);
            println!("└─ Session: {}", result.session_id);

            if result.is_error {
                eprintln!("\n⚠️  ERROR: Query resulted in error state");
                if let Some(ref res) = result.result {
                    eprintln!("   Error details: {}", res);
                }
            }
        }
    }
}
