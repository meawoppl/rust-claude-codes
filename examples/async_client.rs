//! Example of using the asynchronous client

use claude_codes::AsyncClient;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize tracing at warning level to reduce noise
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .init();

    // Create client with default settings
    let mut client = AsyncClient::with_defaults().await?;

    println!("Sending query: What is the capital of France?");
    println!("Waiting for response...\n");

    // Get responses as a stream
    let mut stream = client
        .query_stream("What is the capital of France?")
        .await?;

    // Process each response
    let mut response_count = 0;
    while let Some(result) = stream.next().await {
        response_count += 1;
        match result {
            Ok(output) => {
                println!("Response {}: {}", response_count, output.message_type());
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
