//! Example of using the synchronous client

use claude_codes::{ClaudeInput, SyncClient};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
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
                // You could handle different message types here
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
