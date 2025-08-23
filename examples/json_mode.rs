//! Example demonstrating JSON mode communication with Claude

use claude_codes::{ClaudeCliBuilder, ClaudeInput, ClaudeOutput, Protocol};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a Claude process (always in JSON streaming mode)
    let mut child = ClaudeCliBuilder::new().model("sonnet").spawn().await?;

    // Get handles to stdin and stdout
    let mut stdin = child.stdin.take().expect("Failed to get stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("Failed to get stdout"));

    // Send a message to Claude
    let input = ClaudeInput::user_message("What is 2 + 2?");
    let json_line = Protocol::serialize(&input)?;
    stdin.write_all(json_line.as_bytes()).await?;
    stdin.flush().await?;

    // Read responses
    let mut line = String::new();
    while stdout.read_line(&mut line).await? > 0 {
        // Try to parse as ClaudeOutput
        match serde_json::from_str::<ClaudeOutput>(&line) {
            Ok(output) => match output {
                ClaudeOutput::AssistantMessage(msg) => {
                    println!("Claude says: {}", msg.content);
                }
                ClaudeOutput::ToolUse(tool) => {
                    println!("Claude wants to use tool: {}", tool.tool_name);
                }
                ClaudeOutput::Error(err) => {
                    eprintln!("Error: {}", err.message);
                }
                _ => {
                    println!("Other output: {:?}", output);
                }
            },
            Err(_) => {
                // Could be a different message format or raw output
                if !line.trim().is_empty() {
                    println!("Raw output: {}", line.trim());
                }
            }
        }
        line.clear();
    }

    // Wait for the process to finish
    child.wait().await?;

    Ok(())
}
