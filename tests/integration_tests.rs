//! Integration tests for Claude CLI interactions
//!
//! These tests require a real Claude CLI installation and are only run
//! when the `integration-tests` feature is enabled.
//!
//! Run with: `cargo test --features integration-tests`

#![cfg(feature = "integration-tests")]

use claude_codes::io::ContentBlock;
use claude_codes::{AsyncClient, ClaudeInput, ClaudeOutput, SyncClient};
use uuid::Uuid;

/// Test that we can check Claude CLI version
#[tokio::test]
async fn test_claude_cli_version() {
    use claude_codes::version::check_claude_version_async;

    // This just checks that Claude is installed and accessible
    check_claude_version_async()
        .await
        .expect("Failed to check Claude version");

    println!("Claude CLI version check passed");
}

/// Test basic async client connection and query
#[tokio::test]
async fn test_async_client_basic_query() {
    // Create a client with defaults
    let mut client = AsyncClient::with_defaults()
        .await
        .expect("Failed to create async client");

    // Query with a simple math question
    let mut stream = client
        .query_stream("What is 2 + 2? Reply with just the number.")
        .await
        .expect("Failed to send query");

    // Collect responses
    let mut found_answer = false;
    let mut message_count = 0;

    while let Some(result) = stream.next().await {
        match result {
            Ok(output) => {
                message_count += 1;
                // Check for assistant response containing "4"
                if let ClaudeOutput::Assistant(msg) = &output {
                    for content in &msg.message.content {
                        if let claude_codes::io::ContentBlock::Text(text) = content {
                            if text.text.contains("4") {
                                found_answer = true;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }

        // Stop after finding answer or too many messages
        if found_answer || message_count > 10 {
            break;
        }
    }

    assert!(message_count > 0, "Should have received messages");
    assert!(found_answer, "Should have received answer '4'");
}

/// Test sync client with a simple query
#[test]
fn test_sync_client_basic_query() {
    // Create a sync client
    let mut client = SyncClient::with_defaults().expect("Failed to create sync client");

    // Build input
    let session_id = Uuid::new_v4();
    let input = ClaudeInput::user_message(
        "What is 10 divided by 2? Reply with just the number.",
        session_id,
    );

    // Send query and get responses
    let responses = client.query(input).expect("Failed to query");

    // Check responses
    let mut found_answer = false;
    for response in &responses {
        if let ClaudeOutput::Assistant(msg) = response {
            for content in &msg.message.content {
                if let claude_codes::io::ContentBlock::Text(text) = content {
                    if text.text.contains("5") {
                        found_answer = true;
                    }
                }
            }
        }
    }

    assert!(!responses.is_empty(), "Should have received responses");
    assert!(found_answer, "Should have received answer '5'");
}

/// Test async client with multiple queries in sequence
#[tokio::test]
async fn test_async_client_conversation() {
    let mut client = AsyncClient::with_defaults()
        .await
        .expect("Failed to create async client");

    // First query
    let mut stream1 = client
        .query_stream("Remember the number 42. What number did I ask you to remember?")
        .await
        .expect("Failed to send first query");

    let mut found_42_first = false;
    while let Some(result) = stream1.next().await {
        if let Ok(ClaudeOutput::Assistant(msg)) = result {
            for content in &msg.message.content {
                if let claude_codes::io::ContentBlock::Text(text) = content {
                    if text.text.contains("42") {
                        found_42_first = true;
                    }
                }
            }
        }
    }

    assert!(
        found_42_first,
        "Should have received response mentioning 42"
    );

    // Second query in same session
    let mut stream2 = client
        .query_stream("What was that number again?")
        .await
        .expect("Failed to send second query");

    let mut found_42_second = false;
    while let Some(result) = stream2.next().await {
        if let Ok(ClaudeOutput::Assistant(msg)) = result {
            for content in &msg.message.content {
                if let claude_codes::io::ContentBlock::Text(text) = content {
                    if text.text.contains("42") {
                        found_42_second = true;
                    }
                }
            }
        }
    }

    assert!(
        found_42_second,
        "Should remember 42 from earlier in conversation"
    );
}

/// Test handling various message types
#[tokio::test]
async fn test_message_types() {
    let mut client = AsyncClient::with_defaults()
        .await
        .expect("Failed to create async client");

    let mut stream = client
        .query_stream("Hello! Please respond briefly.")
        .await
        .expect("Failed to send query");

    let mut message_types = std::collections::HashSet::new();
    let mut count = 0;

    while let Some(result) = stream.next().await {
        if let Ok(output) = result {
            message_types.insert(output.message_type().to_string());
            count += 1;
        }

        // Stop after collecting several messages
        if count > 5 {
            break;
        }
    }

    // We should have received at least system and assistant messages
    assert!(count > 0, "Should have received messages");
    assert!(
        message_types.contains("system") || message_types.contains("assistant"),
        "Should have received system or assistant messages"
    );
}

/// Test with custom session ID using the builder
#[tokio::test]
async fn test_with_custom_session() {
    use claude_codes::ClaudeCliBuilder;

    // Use a proper UUID for the session ID
    let session_uuid = Uuid::new_v4();

    let builder = ClaudeCliBuilder::new()
        .model("sonnet")
        .session_id(session_uuid);

    let mut client = AsyncClient::from_builder(builder)
        .await
        .expect("Failed to create client with builder");

    let mut stream = client
        .query_stream("What is 1 + 1?")
        .await
        .expect("Failed to query");

    let mut received_response = false;
    let mut message_count = 0;

    while let Some(result) = stream.next().await {
        message_count += 1;
        if let Ok(ClaudeOutput::Assistant(_)) = result {
            received_response = true;
            break;
        }
        // Stop after too many messages to avoid infinite loop
        if message_count > 10 {
            break;
        }
    }

    assert!(received_response, "Should have received assistant response");
}

/// Test tool use - listing directory and file operations
#[tokio::test]
async fn test_tool_use_blocks() {
    let mut client = AsyncClient::with_defaults()
        .await
        .expect("Failed to create async client");

    // Ask Claude to list the current directory
    let mut stream = client
        .query_stream("Please list the files in the current directory using ls")
        .await
        .expect("Failed to send query");

    let mut tool_use_blocks = Vec::new();
    let mut tool_result_blocks = Vec::new();
    let mut text_blocks = Vec::new();
    let mut thinking_blocks = Vec::new();
    let mut message_count = 0;

    while let Some(result) = stream.next().await {
        message_count += 1;

        match result {
            Ok(output) => {
                // Check for different message types that might contain tool use
                match &output {
                    ClaudeOutput::Assistant(msg) => {
                        for content in &msg.message.content {
                            match content {
                                claude_codes::io::ContentBlock::Text(text) => {
                                    text_blocks.push(text.text.clone());
                                }
                                claude_codes::io::ContentBlock::ToolUse(tool) => {
                                    tool_use_blocks.push((tool.id.clone(), tool.name.clone()));
                                }
                                claude_codes::io::ContentBlock::Thinking(thinking) => {
                                    thinking_blocks.push(thinking.thinking.clone());
                                }
                                claude_codes::io::ContentBlock::ToolResult(_) => {
                                    // Tool results shouldn't appear in assistant messages
                                    panic!(
                                        "Found ToolResult in Assistant message - this is wrong!"
                                    );
                                }
                                claude_codes::io::ContentBlock::Image(_) => {
                                    // Images might appear in assistant messages for generated images
                                }
                            }
                        }
                    }
                    ClaudeOutput::User(msg) => {
                        // Tool results appear in user messages (echoed back)
                        for content in &msg.message.content {
                            match content {
                                claude_codes::io::ContentBlock::ToolResult(result) => {
                                    tool_result_blocks.push((
                                        result.tool_use_id.clone(),
                                        result.is_error.unwrap_or(false),
                                    ));
                                }
                                claude_codes::io::ContentBlock::ToolUse(_) => {
                                    // Tool use shouldn't appear in user messages
                                    panic!("Found ToolUse in User message - this is wrong!");
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }

        // Stop after collecting enough messages
        if message_count > 15 {
            break;
        }
    }

    println!("Tool use blocks: {:?}", tool_use_blocks);
    println!("Tool result blocks: {:?}", tool_result_blocks);
    println!("Text blocks count: {}", text_blocks.len());
    println!("Thinking blocks: {:?}", thinking_blocks);

    // Verify we got tool use blocks
    assert!(
        !tool_use_blocks.is_empty(),
        "Should have received at least one ToolUse block"
    );

    // Verify we got tool results
    assert!(
        !tool_result_blocks.is_empty(),
        "Should have received at least one ToolResult block"
    );

    // Verify the tool IDs match between use and result
    for (use_id, _) in &tool_use_blocks {
        assert!(
            tool_result_blocks
                .iter()
                .any(|(result_id, _)| result_id == use_id),
            "Tool use ID {} should have a corresponding result",
            use_id
        );
    }
}

/// Test file editing tool use
#[tokio::test]
async fn test_file_edit_tool_use() {
    let mut client = AsyncClient::with_defaults()
        .await
        .expect("Failed to create async client");

    // Create a test file first
    let test_file = "/tmp/claude_test_file.txt";
    std::fs::write(test_file, "Hello World").expect("Failed to create test file");

    // Ask Claude to edit the file
    let query = format!(
        "Please read the file at {} and tell me what it says. Then append ' - Modified by Claude' to it.",
        test_file
    );

    let mut stream = client
        .query_stream(&query)
        .await
        .expect("Failed to send query");

    let mut tool_uses = Vec::new();
    let mut message_count = 0;

    while let Some(result) = stream.next().await {
        message_count += 1;

        if let Ok(ClaudeOutput::Assistant(msg)) = result {
            for content in &msg.message.content {
                match content {
                    claude_codes::io::ContentBlock::ToolUse(tool) => {
                        println!("Tool use: name={}, input={:?}", tool.name, tool.input);
                        tool_uses.push(tool.name.clone());
                    }
                    claude_codes::io::ContentBlock::Text(text) => {
                        if text.text.len() < 200 {
                            println!("Text: {}", text.text);
                        }
                    }
                    _ => {}
                }
            }
        }

        if message_count > 20 {
            break;
        }
    }

    println!("Tools used: {:?}", tool_uses);

    // Clean up
    let _ = std::fs::remove_file(test_file);

    assert!(message_count > 0, "Should have received messages");
}

/// Test capturing raw tool blocks for deserialization testing
#[tokio::test]
async fn test_capture_tool_blocks_for_testing() {
    use std::fs;
    use std::path::Path;

    let mut client = AsyncClient::with_defaults()
        .await
        .expect("Failed to create async client");

    // Ask for multiple tool uses to get variety
    let mut stream = client
        .query_stream(
            "Please do the following:\n\
            1. List files in /tmp\n\
            2. Show the current date\n\
            3. Check if /etc/passwd exists",
        )
        .await
        .expect("Failed to send query");

    let captures_dir = Path::new("test_cases/tool_use_captures");
    fs::create_dir_all(captures_dir).ok();

    let mut capture_count = 0;
    let mut message_count = 0;

    while let Some(result) = stream.next().await {
        message_count += 1;

        match result {
            Ok(output) => {
                // Serialize the entire message for analysis
                if let Ok(json) = serde_json::to_string_pretty(&output) {
                    // Save messages that might contain tool use
                    if json.contains("tool") || json.contains("Tool") {
                        let filename = format!("tool_msg_{}.json", capture_count);
                        let filepath = captures_dir.join(filename);
                        fs::write(&filepath, &json).ok();
                        println!("Captured potential tool message to {:?}", filepath);
                        capture_count += 1;
                    }
                }

                // Log what we're seeing
                if let ClaudeOutput::Assistant(msg) = &output {
                    for content in &msg.message.content {
                        if let claude_codes::io::ContentBlock::ToolUse(tool) = content {
                            println!("=== TOOL USE FOUND ===");
                            println!("Name: {}", tool.name);
                            println!("ID: {}", tool.id);
                            println!("Input: {:?}", tool.input);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Parse error (might be new message type): {}", e);
                // Save error details for analysis
                let filename = format!("error_msg_{}.txt", capture_count);
                let filepath = captures_dir.join(filename);
                fs::write(&filepath, format!("Error: {}", e)).ok();
                println!("Captured error to {:?}", filepath);
                capture_count += 1;
            }
        }

        if message_count > 25 {
            break;
        }
    }

    println!("Captured {} potential tool messages", capture_count);
    assert!(message_count > 0, "Should have received messages");
}

/// Test image content blocks
#[tokio::test]
async fn test_image_content_blocks() {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use std::fs;

    // Read and encode the test image
    let image_path = "tests/test_data/hello-claude.png";
    let image_data = fs::read(image_path).expect("Failed to read test image");
    let base64_image = STANDARD.encode(&image_data);

    // Create client
    let mut client = AsyncClient::with_defaults()
        .await
        .expect("Failed to create async client");

    // Send message with image
    let session_id = Uuid::new_v4();
    let input = ClaudeInput::user_message_with_image(
        base64_image.clone(),
        "image/png".to_string(),
        Some("What do you see in this image?".to_string()),
        session_id,
    )
    .expect("Failed to create image message");

    // Verify serialization includes image block
    let serialized = serde_json::to_string(&input).expect("Failed to serialize");
    assert!(
        serialized.contains("\"type\":\"image\""),
        "Should contain image type"
    );
    assert!(
        serialized.contains("\"source\""),
        "Should contain source field"
    );
    assert!(
        serialized.contains("\"media_type\":\"image/png\""),
        "Should contain media type"
    );
    assert!(
        serialized.contains("\"type\":\"base64\""),
        "Should contain source type"
    );

    // Send to Claude and collect responses
    client
        .send(&input)
        .await
        .expect("Failed to send image message");

    let mut found_image_description = false;
    let mut message_count = 0;
    let mut image_blocks_in_response = 0;

    loop {
        match client.receive().await {
            Ok(output) => {
                message_count += 1;

                // Check if assistant response mentions image content
                if let ClaudeOutput::Assistant(msg) = &output {
                    for content in &msg.message.content {
                        match content {
                            claude_codes::io::ContentBlock::Text(text) => {
                                // Claude should describe what it sees
                                if text.text.to_lowercase().contains("image")
                                    || text.text.to_lowercase().contains("hello")
                                    || text.text.to_lowercase().contains("text")
                                    || text.text.to_lowercase().contains("see")
                                {
                                    found_image_description = true;
                                    println!("Claude's description: {}", text.text);
                                }
                            }
                            claude_codes::io::ContentBlock::Image(_) => {
                                // Images in responses would be interesting
                                image_blocks_in_response += 1;
                            }
                            _ => {}
                        }
                    }
                }

                // Stop on result message
                if matches!(output, ClaudeOutput::Result(_)) {
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error receiving: {}", e);
                break;
            }
        }

        // Safety limit
        if message_count > 20 {
            break;
        }
    }

    println!("Found image description: {}", found_image_description);
    println!("Image blocks in response: {}", image_blocks_in_response);

    assert!(message_count > 0, "Should have received messages");

    assert!(
        found_image_description,
        "Claude should have described the image content"
    );
}

/// Test mixed content blocks (text + image)
#[tokio::test]
async fn test_mixed_content_blocks() {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    // Create a small test image programmatically (1x1 red pixel PNG)
    let red_pixel_png = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77,
        0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0x99, 0x63, 0xF8, 0xCF,
        0xC0, 0x00, 0x00, 0x00, 0x03, 0x00, 0x01, 0x5B, 0x79, 0x53, 0x0D, 0x00, 0x00, 0x00, 0x00,
        0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82, // IEND chunk
    ];

    let base64_image = STANDARD.encode(&red_pixel_png);

    // Create mixed content blocks
    let blocks = vec![
        ContentBlock::Text(claude_codes::io::TextBlock {
            text: "Here's a question with an image:".to_string(),
        }),
        ContentBlock::Image(claude_codes::io::ImageBlock {
            source: claude_codes::io::ImageSource {
                source_type: "base64".to_string(),
                media_type: "image/png".to_string(),
                data: base64_image,
            },
        }),
        ContentBlock::Text(claude_codes::io::TextBlock {
            text: "What color is this pixel?".to_string(),
        }),
    ];

    let session_id = Uuid::new_v4();
    let input = ClaudeInput::user_message_blocks(blocks, session_id);

    // Verify serialization
    let serialized = serde_json::to_string(&input).expect("Failed to serialize");
    assert!(
        serialized.contains("\"type\":\"text\""),
        "Should contain text blocks"
    );
    assert!(
        serialized.contains("\"type\":\"image\""),
        "Should contain image block"
    );

    // Verify deserialization round-trip
    let deserialized: ClaudeInput =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    if let ClaudeInput::User(user_msg) = deserialized {
        assert_eq!(
            user_msg.message.content.len(),
            3,
            "Should have 3 content blocks"
        );

        // Verify block types
        assert!(
            matches!(&user_msg.message.content[0], ContentBlock::Text(_)),
            "First block should be text"
        );

        assert!(
            matches!(&user_msg.message.content[1], ContentBlock::Image(_)),
            "Second block should be image"
        );

        assert!(
            matches!(&user_msg.message.content[2], ContentBlock::Text(_)),
            "Third block should be text"
        );
    } else {
        panic!("Expected User message");
    }

    println!("Mixed content blocks test passed");
}

/// Test media type validation
#[test]
fn test_media_type_validation() {
    let session_id = Uuid::new_v4();
    let fake_data = "fake_base64_data".to_string();

    // Valid media types should work
    let valid_types = vec!["image/jpeg", "image/png", "image/gif", "image/webp"];
    for media_type in valid_types {
        let result = ClaudeInput::user_message_with_image(
            fake_data.clone(),
            media_type.to_string(),
            None,
            session_id,
        );
        assert!(result.is_ok(), "Media type {} should be valid", media_type);
    }

    // Invalid media types should fail
    let invalid_types = vec![
        "image/bmp",
        "image/tiff",
        "video/mp4",
        "text/plain",
        "application/pdf",
    ];
    for media_type in invalid_types {
        let result = ClaudeInput::user_message_with_image(
            fake_data.clone(),
            media_type.to_string(),
            None,
            session_id,
        );
        assert!(
            result.is_err(),
            "Media type {} should be invalid",
            media_type
        );
        if let Err(msg) = result {
            assert!(msg.contains("Only JPEG, PNG, GIF, and WebP are supported"));
        }
    }
}
