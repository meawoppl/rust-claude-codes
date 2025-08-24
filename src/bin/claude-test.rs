//! Testing binary for Claude Code JSON communication
//!
//! This binary allows you to send queries to Claude and receive responses,
//! with automatic JSON serialization/deserialization.

use anyhow::{Context, Result};
use claude_codes::{ClaudeCliBuilder, ClaudeInput, ClaudeOutput, Protocol};
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
    println!("Claude Test Client");
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
                // Read and process the response
                match read_response(&mut claude).await {
                    Ok(output) => {
                        handle_output(output);
                    }
                    Err(e) => {
                        error!("Failed to read response: {}", e);
                        eprintln!("Error reading response: {}", e);
                        return Err(e);
                    }
                }
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

    // Create the input message with a dummy session ID for now
    let input = ClaudeInput::user_message(query, "test-session");

    // Serialize to JSON
    let json_line = Protocol::serialize(&input).context("Failed to serialize input")?;

    debug!("Sending JSON: {}", json_line.trim());

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

        debug!("Received line: {}", trimmed);

        // Try to parse as ClaudeOutput
        match serde_json::from_str::<ClaudeOutput>(trimmed) {
            Ok(output) => {
                info!("Successfully parsed ClaudeOutput");

                // Non-streaming response, return immediately
                return Ok(output);
            }
            Err(e) => {
                // Save failed test case
                save_test_case(trimmed, &e)?;

                // Print the raw JSON that failed to parse
                error!("Failed to deserialize response: {}", e);
                eprintln!("\n=== DESERIALIZATION ERROR ===");
                eprintln!("Failed to parse response as ClaudeOutput");
                eprintln!("Error: {}", e);
                eprintln!("\nRaw JSON received:");
                eprintln!("{}", trimmed);
                eprintln!("=============================\n");
                eprintln!("Test case saved to test_cases/failed_deserializations/");

                // Return an error
                return Err(anyhow::anyhow!(
                    "Failed to deserialize response: {}. Raw JSON: {}",
                    e,
                    trimmed
                ));
            }
        }
    }
}

/// Save a failed deserialization as a test case
fn save_test_case(json: &str, error: &serde_json::Error) -> Result<()> {
    // Create test cases directory if it doesn't exist
    let test_dir = PathBuf::from("test_cases/failed_deserializations");
    fs::create_dir_all(&test_dir).context("Failed to create test_cases directory")?;

    // Generate a unique filename based on timestamp and hash
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S_%f");
    let hash = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        json.hash(&mut hasher);
        hasher.finish()
    };

    let filename = format!("case_{}_{:016x}.json", timestamp, hash);
    let filepath = test_dir.join(&filename);

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
    Ok(())
}

/// Handle the output from Claude
fn handle_output(output: ClaudeOutput) {
    match output {
        ClaudeOutput::System(sys) => {
            println!("\n[System Init]");
            println!("Model: {}", sys.model);
            println!("Session: {}", sys.session_id);
            println!("Permission Mode: {:?}", sys.permission_mode);
            println!("Tools: {} available", sys.tools.len());
            println!();
        }
        ClaudeOutput::User(msg) => {
            debug!("User message echoed: session={}", msg.session_id);
        }
        ClaudeOutput::Assistant(msg) => {
            println!("\nClaude: ");
            // The message field is a JSON Value, extract content if possible
            if let Some(content) = msg.message.get("content").and_then(|v| v.as_str()) {
                println!("{}", content);
            } else {
                println!("{}", serde_json::to_string_pretty(&msg.message).unwrap());
            }
            println!();
        }
        ClaudeOutput::Result(result) => {
            println!("\n[Query Complete]");
            if let Some(ref res) = result.result {
                println!("Result: {}", res);
            }
            println!("Status: {:?}", result.subtype);
            println!(
                "Duration: {}ms (API: {}ms)",
                result.duration_ms, result.duration_api_ms
            );
            if let Some(ref usage) = result.usage {
                println!(
                    "Tokens: {} in, {} out",
                    usage.input_tokens, usage.output_tokens
                );
            }
            println!("Cost: ${:.6}", result.total_cost_usd);
            if result.is_error {
                eprintln!("ERROR: Query resulted in error state");
            }
            println!();
        }
        ClaudeOutput::Raw(value) => {
            warn!("Received raw/untyped output");
            println!("\n[Raw Output]");
            println!("{}", serde_json::to_string_pretty(&value).unwrap());
            println!();
        }
    }
}
