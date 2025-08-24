//! Example of using the synchronous client

use claude_codes::{ClaudeInput, SyncClient};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize tracing at warning level to reduce noise
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .init();

    // Create client with default settings
    let mut client = SyncClient::with_defaults()?;

    // Send a simple query
    let input = ClaudeInput::user_message("What is 2 + 2?", "test-session");

    println!("Sending query: What is 2 + 2?");
    println!("Waiting for response...\n");

    // Get responses as an iterator
    let responses = client.query_stream(input)?;

    // Process each response
    for (i, response) in responses.enumerate() {
        match response {
            Ok(output) => {
                println!("Response {}: {}", i + 1, output.message_type());
                // Show assistant messages
                if let claude_codes::ClaudeOutput::Assistant(msg) = &output {
                    for content in &msg.message.content {
                        if let claude_codes::io::ContentBlock::Text(text) = content {
                            println!("  Claude says: {}", text.text);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error receiving response: {}", e);
                break;
            }
        }
    }

    println!("\nClient session complete");
    Ok(())
}
