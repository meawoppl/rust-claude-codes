//! Integration tests for typed tool inputs.
//!
//! These tests verify that real Claude CLI tool use messages can be deserialized
//! and their inputs can be parsed into strongly-typed structures.

use claude_codes::io::ContentBlock;
use claude_codes::{BashInput, ClaudeOutput, ToolInput, ToolUseBlock};
use serde_json::json;

// ============================================================================
// Tests using captured real messages from test_cases/tool_use_captures/
// ============================================================================

/// Test parsing the system init message (tool_msg_0.json)
#[test]
fn test_parse_system_init_message() {
    let json_str = include_str!("../test_cases/tool_use_captures/tool_msg_0.json");
    let output: ClaudeOutput =
        serde_json::from_str(json_str).expect("Failed to parse system init message");

    match output {
        ClaudeOutput::System(msg) => {
            assert_eq!(msg.subtype, "init");
            // Check that tools list is present
            let tools = msg.data.get("tools").expect("Missing tools");
            assert!(tools.is_array());
            let tools_array = tools.as_array().unwrap();
            assert!(tools_array.iter().any(|t| t.as_str() == Some("Bash")));
            assert!(tools_array.iter().any(|t| t.as_str() == Some("Read")));
            assert!(tools_array.iter().any(|t| t.as_str() == Some("Write")));
            println!(
                "System init message parsed successfully with {} tools",
                tools_array.len()
            );
        }
        _ => panic!("Expected System message, got {:?}", output.message_type()),
    }
}

/// Test parsing assistant message with Bash tool use (tool_msg_1.json)
#[test]
fn test_parse_bash_tool_use_message() {
    let json_str = include_str!("../test_cases/tool_use_captures/tool_msg_1.json");
    let output: ClaudeOutput =
        serde_json::from_str(json_str).expect("Failed to parse assistant message");

    match output {
        ClaudeOutput::Assistant(msg) => {
            assert_eq!(msg.message.role, "assistant");
            assert_eq!(msg.message.content.len(), 1);

            if let ContentBlock::ToolUse(tool_use) = &msg.message.content[0] {
                assert_eq!(tool_use.name, "Bash");
                // Note: tool_use.id changes per session, just verify it starts with expected prefix
                assert!(tool_use.id.starts_with("toolu_"));

                // Test typed_input() method
                let typed = tool_use.typed_input().expect("Failed to get typed input");
                match typed {
                    ToolInput::Bash(bash) => {
                        assert_eq!(bash.command, "ls -la /tmp");
                        assert_eq!(
                            bash.description,
                            Some("List files in /tmp directory".to_string())
                        );
                    }
                    _ => panic!("Expected Bash input, got {:?}", typed.tool_name()),
                }
            } else {
                panic!("Expected ToolUse content block");
            }
        }
        _ => panic!("Expected Assistant message"),
    }
}

/// Test parsing assistant message with date command (tool_msg_2.json)
#[test]
fn test_parse_bash_date_command() {
    let json_str = include_str!("../test_cases/tool_use_captures/tool_msg_2.json");
    let output: ClaudeOutput = serde_json::from_str(json_str).expect("Failed to parse");

    if let ClaudeOutput::Assistant(msg) = output {
        if let ContentBlock::ToolUse(tool_use) = &msg.message.content[0] {
            let typed = tool_use.typed_input().unwrap();
            if let ToolInput::Bash(bash) = typed {
                assert_eq!(bash.command, "date");
                // Description may vary slightly per session
                assert!(bash.description.is_some());
                assert!(bash
                    .description
                    .as_ref()
                    .unwrap()
                    .to_lowercase()
                    .contains("date"));
            } else {
                panic!("Expected Bash");
            }
        }
    }
}

/// Test parsing assistant message with complex bash command (tool_msg_3.json)
#[test]
fn test_parse_bash_complex_command() {
    let json_str = include_str!("../test_cases/tool_use_captures/tool_msg_3.json");
    let output: ClaudeOutput = serde_json::from_str(json_str).expect("Failed to parse");

    if let ClaudeOutput::Assistant(msg) = output {
        // Note: stop_reason may or may not be present depending on API version
        // Just verify the message parses correctly

        if let ContentBlock::ToolUse(tool_use) = &msg.message.content[0] {
            let typed = tool_use.typed_input().unwrap();
            if let ToolInput::Bash(bash) = typed {
                assert!(bash.command.contains("test -f /etc/passwd"));
                // Description may vary, just check it exists and mentions passwd
                assert!(bash.description.is_some());
                assert!(bash
                    .description
                    .as_ref()
                    .unwrap()
                    .to_lowercase()
                    .contains("passwd"));
            }
        }
    }
}

/// Test parsing tool result (error) message (tool_msg_4.json)
#[test]
fn test_parse_tool_result_error() {
    let json_str = include_str!("../test_cases/tool_use_captures/tool_msg_4.json");
    let output: ClaudeOutput = serde_json::from_str(json_str).expect("Failed to parse");

    if let ClaudeOutput::User(msg) = output {
        if let ContentBlock::ToolResult(result) = &msg.message.content[0] {
            // tool_use_id changes per session, just verify format
            assert!(result.tool_use_id.starts_with("toolu_"));
            assert_eq!(result.is_error, Some(true));
        }
    }
}

/// Test parsing tool result (success) message (tool_msg_5.json)
#[test]
fn test_parse_tool_result_success() {
    let json_str = include_str!("../test_cases/tool_use_captures/tool_msg_5.json");
    let output: ClaudeOutput = serde_json::from_str(json_str).expect("Failed to parse");

    if let ClaudeOutput::User(msg) = output {
        if let ContentBlock::ToolResult(result) = &msg.message.content[0] {
            // tool_use_id changes per session, just verify format
            assert!(result.tool_use_id.starts_with("toolu_"));
            assert_eq!(result.is_error, Some(false));
        }
    }
}

/// Test parsing result message with permission_denials (tool_msg_7.json)
#[test]
fn test_parse_result_with_permission_denials() {
    let json_str = include_str!("../test_cases/tool_use_captures/tool_msg_7.json");
    let output: ClaudeOutput = serde_json::from_str(json_str).expect("Failed to parse");

    if let ClaudeOutput::Result(result) = output {
        assert!(!result.is_error);
        assert_eq!(result.num_turns, 4);
        assert_eq!(result.permission_denials.len(), 2);

        // Access the first denial's fields directly (now typed)
        let denial1 = &result.permission_denials[0];
        assert_eq!(denial1.tool_name, "Bash");
        // tool_use_id changes per session, just verify format
        assert!(denial1.tool_use_id.starts_with("toolu_"));

        // Parse the tool_input as BashInput
        let bash: BashInput =
            serde_json::from_value(denial1.tool_input.clone()).expect("Failed to parse tool_input");
        assert_eq!(bash.command, "ls -la /tmp");
        // Description may vary slightly
        assert!(bash.description.is_some());

        // Access the second denial
        let denial2 = &result.permission_denials[1];
        assert_eq!(denial2.tool_name, "Bash");
        assert!(denial2.tool_use_id.starts_with("toolu_"));

        let bash2: BashInput = serde_json::from_value(denial2.tool_input.clone()).unwrap();
        assert!(bash2.command.contains("test -f /etc/passwd"));

        println!(
            "Parsed result with {} permission denials",
            result.permission_denials.len()
        );
    }
}

/// Test parsing result message with uuid field (tool_msg_8.json)
#[test]
fn test_parse_result_with_uuid() {
    let json_str = include_str!("../test_cases/tool_use_captures/tool_msg_8.json");
    let output: ClaudeOutput = serde_json::from_str(json_str).expect("Failed to parse");

    if let ClaudeOutput::Result(result) = output {
        assert!(!result.is_error);
        assert_eq!(result.num_turns, 4);
        assert_eq!(result.permission_denials.len(), 2);

        // Verify the new uuid field is parsed
        assert!(result.uuid.is_some());
        let uuid = result.uuid.as_ref().unwrap();
        assert!(uuid.contains("-"), "UUID should contain hyphens");
        println!("Parsed result with uuid: {}", uuid);

        // Verify errors array (empty in this case)
        assert!(result.errors.is_empty());

        // Verify usage info
        assert!(result.usage.is_some());
    } else {
        panic!("Expected Result message");
    }
}

// ============================================================================
// Tests for ToolInput enum deserialization
// ============================================================================

#[test]
fn test_tool_input_bash_deserialization() {
    let json = json!({
        "command": "git status",
        "description": "Check git status"
    });

    let input: ToolInput = serde_json::from_value(json).unwrap();
    assert!(matches!(input, ToolInput::Bash(_)));
    assert_eq!(input.tool_name(), Some("Bash"));

    let bash = input.as_bash().unwrap();
    assert_eq!(bash.command, "git status");
}

#[test]
fn test_tool_input_read_deserialization() {
    let json = json!({
        "file_path": "/home/user/code.rs",
        "offset": 100,
        "limit": 50
    });

    let input: ToolInput = serde_json::from_value(json).unwrap();
    assert!(matches!(input, ToolInput::Read(_)));

    let read = input.as_read().unwrap();
    assert_eq!(read.file_path, "/home/user/code.rs");
    assert_eq!(read.offset, Some(100));
    assert_eq!(read.limit, Some(50));
}

#[test]
fn test_tool_input_write_deserialization() {
    let json = json!({
        "file_path": "/tmp/output.txt",
        "content": "Hello, world!"
    });

    let input: ToolInput = serde_json::from_value(json).unwrap();
    assert!(matches!(input, ToolInput::Write(_)));

    let write = input.as_write().unwrap();
    assert_eq!(write.file_path, "/tmp/output.txt");
    assert_eq!(write.content, "Hello, world!");
}

#[test]
fn test_tool_input_edit_deserialization() {
    let json = json!({
        "file_path": "/home/user/code.rs",
        "old_string": "fn old_name()",
        "new_string": "fn new_name()",
        "replace_all": true
    });

    let input: ToolInput = serde_json::from_value(json).unwrap();
    assert!(matches!(input, ToolInput::Edit(_)));

    let edit = input.as_edit().unwrap();
    assert_eq!(edit.file_path, "/home/user/code.rs");
    assert_eq!(edit.old_string, "fn old_name()");
    assert_eq!(edit.new_string, "fn new_name()");
    assert_eq!(edit.replace_all, Some(true));
}

#[test]
fn test_tool_input_glob_deserialization() {
    let json = json!({
        "pattern": "**/*.rs",
        "path": "/home/user/project"
    });

    let input: ToolInput = serde_json::from_value(json).unwrap();
    assert!(matches!(input, ToolInput::Glob(_)));

    let glob = input.as_glob().unwrap();
    assert_eq!(glob.pattern, "**/*.rs");
    assert_eq!(glob.path, Some("/home/user/project".to_string()));
}

#[test]
fn test_tool_input_grep_deserialization() {
    let json = json!({
        "pattern": "fn\\s+\\w+",
        "path": "/home/user/project",
        "type": "rust",
        "-i": true,
        "-C": 3,
        "output_mode": "content"
    });

    let input: ToolInput = serde_json::from_value(json).unwrap();
    assert!(matches!(input, ToolInput::Grep(_)));

    let grep = input.as_grep().unwrap();
    assert_eq!(grep.pattern, "fn\\s+\\w+");
    assert_eq!(grep.file_type, Some("rust".to_string()));
    assert_eq!(grep.case_insensitive, Some(true));
    assert_eq!(grep.context, Some(3));
}

#[test]
fn test_tool_input_task_deserialization() {
    let json = json!({
        "description": "Search codebase",
        "prompt": "Find all usages of the foo function",
        "subagent_type": "Explore",
        "run_in_background": true
    });

    let input: ToolInput = serde_json::from_value(json).unwrap();
    assert!(matches!(input, ToolInput::Task(_)));

    let task = input.as_task().unwrap();
    assert_eq!(task.description, "Search codebase");
    assert_eq!(task.prompt, "Find all usages of the foo function");
    assert_eq!(task.subagent_type, "Explore");
    assert_eq!(task.run_in_background, Some(true));
}

#[test]
fn test_tool_input_web_fetch_deserialization() {
    let json = json!({
        "url": "https://docs.rs/serde/latest",
        "prompt": "Extract the main documentation"
    });

    let input: ToolInput = serde_json::from_value(json).unwrap();
    assert!(matches!(input, ToolInput::WebFetch(_)));

    let fetch = input.as_web_fetch().unwrap();
    assert_eq!(fetch.url, "https://docs.rs/serde/latest");
    assert_eq!(fetch.prompt, "Extract the main documentation");
}

#[test]
fn test_tool_input_web_search_deserialization() {
    let json = json!({
        "query": "rust serde tutorial 2026",
        "allowed_domains": ["docs.rs", "crates.io"]
    });

    let input: ToolInput = serde_json::from_value(json).unwrap();
    assert!(matches!(input, ToolInput::WebSearch(_)));

    let search = input.as_web_search().unwrap();
    assert_eq!(search.query, "rust serde tutorial 2026");
}

#[test]
fn test_tool_input_todo_write_deserialization() {
    let json = json!({
        "todos": [
            {
                "content": "Implement feature",
                "status": "in_progress",
                "activeForm": "Implementing feature"
            },
            {
                "content": "Write tests",
                "status": "pending",
                "activeForm": "Writing tests"
            }
        ]
    });

    let input: ToolInput = serde_json::from_value(json).unwrap();
    assert!(matches!(input, ToolInput::TodoWrite(_)));

    let todo = input.as_todo_write().unwrap();
    assert_eq!(todo.todos.len(), 2);
    assert_eq!(todo.todos[0].content, "Implement feature");
    assert_eq!(todo.todos[0].status, "in_progress");
}

#[test]
fn test_tool_input_ask_user_question_deserialization() {
    let json = json!({
        "questions": [
            {
                "question": "Which database should we use?",
                "header": "Database",
                "options": [
                    {"label": "PostgreSQL", "description": "Robust relational database"},
                    {"label": "SQLite", "description": "Lightweight embedded database"}
                ],
                "multiSelect": false
            }
        ]
    });

    let input: ToolInput = serde_json::from_value(json).unwrap();
    assert!(matches!(input, ToolInput::AskUserQuestion(_)));

    let question = input.as_ask_user_question().unwrap();
    assert_eq!(question.questions.len(), 1);
    assert_eq!(
        question.questions[0].question,
        "Which database should we use?"
    );
    assert_eq!(question.questions[0].options.len(), 2);
}

#[test]
fn test_tool_input_unknown_custom_tool() {
    // Simulates a custom MCP tool with unknown structure
    let json = json!({
        "custom_field": "custom_value",
        "another_field": 123,
        "nested": {
            "foo": "bar"
        }
    });

    let input: ToolInput = serde_json::from_value(json).unwrap();
    assert!(matches!(input, ToolInput::Unknown(_)));
    assert_eq!(input.tool_name(), None);
    assert!(input.is_unknown());

    let unknown = input.as_unknown().unwrap();
    assert_eq!(unknown.get("custom_field").unwrap(), "custom_value");
}

// ============================================================================
// ToolUseBlock helper method tests
// ============================================================================

#[test]
fn test_tool_use_block_typed_input() {
    let block = ToolUseBlock {
        id: "toolu_123".to_string(),
        name: "Bash".to_string(),
        input: json!({
            "command": "cargo build",
            "description": "Build the project"
        }),
    };

    let typed = block.typed_input().expect("Should parse");
    assert!(matches!(typed, ToolInput::Bash(_)));

    if let ToolInput::Bash(bash) = typed {
        assert_eq!(bash.command, "cargo build");
    }
}

#[test]
fn test_tool_use_block_try_typed_input_error() {
    let block = ToolUseBlock {
        id: "toolu_456".to_string(),
        name: "SomeCustomTool".to_string(),
        input: json!({
            "weird_field": [1, 2, 3]
        }),
    };

    // Should succeed but return Unknown variant
    let typed = block.try_typed_input().expect("Should parse as Unknown");
    assert!(matches!(typed, ToolInput::Unknown(_)));
}

// ============================================================================
// Tests for new helper methods with real message captures
// ============================================================================

/// Test assistant message with usage info and parent_tool_use_id
#[test]
fn test_parse_assistant_with_usage_and_parent() {
    let json_str = include_str!("../test_cases/tool_use_captures/assistant_with_usage.json");
    let output: ClaudeOutput =
        serde_json::from_str(json_str).expect("Failed to parse assistant message with usage");

    // Test is_assistant_message()
    assert!(output.is_assistant_message());

    // Test session_id() helper
    assert_eq!(
        output.session_id(),
        Some("08cd4ce5-1ce0-4dd4-8e7c-8b69712c514e")
    );

    // Test as_assistant() helper
    let assistant = output.as_assistant().expect("Should be assistant");
    assert_eq!(assistant.message.role, "assistant");
    assert_eq!(assistant.message.model, "claude-haiku-4-5-20251001");

    // Test tool_uses() helper
    let tools: Vec<_> = output.tool_uses().collect();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "Bash");

    // Test as_tool_use() helper
    let bash = output.as_tool_use("Bash").expect("Should find Bash tool");
    assert_eq!(bash.name, "Bash");

    // Test typed_input on the tool
    let typed = bash.typed_input().expect("Should get typed input");
    if let ToolInput::Bash(b) = typed {
        assert!(b.command.contains("grep"));
    } else {
        panic!("Expected Bash input");
    }

    // Test text_content() - should be None since it's only tool_use
    assert!(output.text_content().is_none());

    // Check parent_tool_use_id is preserved
    assert_eq!(
        assistant.parent_tool_use_id,
        Some("toolu_012hZhNyfdf6Y156ryHJSbxd".to_string())
    );

    // Check usage is preserved
    let usage = assistant.message.usage.as_ref().expect("Should have usage");
    assert_eq!(usage.input_tokens, 5);
    assert_eq!(usage.output_tokens, 250);
}

/// Test tool_result with error content (string form)
#[test]
fn test_parse_tool_result_error_message() {
    let json_str = include_str!("../test_cases/tool_use_captures/tool_result_error.json");
    let output: ClaudeOutput =
        serde_json::from_str(json_str).expect("Failed to parse tool result error");

    // Should not be assistant
    assert!(!output.is_assistant_message());

    // Test session_id() returns None for user messages
    assert!(output.session_id().is_none());

    // Test is_error() - should be false since this is a user message, not result
    assert!(!output.is_error());

    // Verify it's a user message with tool_result content
    if let ClaudeOutput::User(user) = output {
        assert_eq!(user.message.role, "user");
        assert_eq!(user.message.content.len(), 1);

        if let ContentBlock::ToolResult(result) = &user.message.content[0] {
            assert_eq!(result.is_error, Some(true));
            assert_eq!(result.tool_use_id, "toolu_01F5A3vbYenHhtdEV9Zt7arW");
            // Check error message content - ToolResultContent is an enum
            if let Some(claude_codes::ToolResultContent::Text(text)) = &result.content {
                assert!(text.contains("InputValidationError"));
            } else {
                panic!("Expected Text content in tool result");
            }
        } else {
            panic!("Expected ToolResult content block");
        }
    } else {
        panic!("Expected User message");
    }
}

/// Test tool_result with structured/array-form content (for WASM consumers - issue #42)
#[test]
fn test_parse_tool_result_structured_content() {
    let json_str = include_str!("../test_cases/tool_use_captures/tool_result_structured.json");
    let output: ClaudeOutput = serde_json::from_str(json_str)
        .expect("Failed to parse tool result with structured content");

    // Verify it's a user message with tool_result content
    if let ClaudeOutput::User(user) = output {
        assert_eq!(user.message.role, "user");
        assert_eq!(user.message.content.len(), 1);

        if let ContentBlock::ToolResult(result) = &user.message.content[0] {
            assert_eq!(result.tool_use_id, "toolu_01ABC123def456");
            // Check structured content - ToolResultContent is an enum with Structured variant
            if let Some(claude_codes::ToolResultContent::Structured(structured)) = &result.content {
                // Structured content is Vec<Value>
                assert!(!structured.is_empty());
                // First element should have type: "text"
                let first = &structured[0];
                assert_eq!(first.get("type").and_then(|v| v.as_str()), Some("text"));
                // Verify it contains actual text content
                let text = first.get("text").and_then(|v| v.as_str());
                assert!(text.is_some());
                assert!(text.unwrap().contains("React"));
            } else {
                panic!(
                    "Expected Structured content in tool result, got: {:?}",
                    result.content
                );
            }
        } else {
            panic!("Expected ToolResult content block");
        }
    } else {
        panic!("Expected User message");
    }
}

/// Test tool_result with multiple text items in structured content (real message from production)
#[test]
fn test_parse_tool_result_multi_text_structured() {
    let json_str = include_str!("../test_cases/tool_use_captures/tool_result_multi_text.json");
    let output: ClaudeOutput = serde_json::from_str(json_str)
        .expect("Failed to parse tool result with multi-text content");

    // Verify it's a user message with tool_result content
    if let ClaudeOutput::User(user) = output {
        assert_eq!(user.message.role, "user");
        assert_eq!(user.message.content.len(), 1);

        if let ContentBlock::ToolResult(result) = &user.message.content[0] {
            assert_eq!(result.tool_use_id, "toolu_012hZhNyfdf6Y156ryHJSbxd");
            // Check structured content has multiple text items
            if let Some(claude_codes::ToolResultContent::Structured(structured)) = &result.content {
                // Should have 2 text items (the main analysis and the agentId)
                assert_eq!(structured.len(), 2);

                // First element should have type: "text" with the analysis
                let first = &structured[0];
                assert_eq!(first.get("type").and_then(|v| v.as_str()), Some("text"));
                let text = first.get("text").and_then(|v| v.as_str()).unwrap();
                assert!(text.contains("ADSBView.tsx Component Analysis"));

                // Second element should have the agentId
                let second = &structured[1];
                assert_eq!(second.get("type").and_then(|v| v.as_str()), Some("text"));
                let agent_text = second.get("text").and_then(|v| v.as_str()).unwrap();
                assert!(agent_text.contains("agentId"));
            } else {
                panic!(
                    "Expected Structured content in tool result, got: {:?}",
                    result.content
                );
            }
        } else {
            panic!("Expected ToolResult content block");
        }
    } else {
        panic!("Expected User message");
    }
}

// ============================================================================
// Roundtrip serialization tests
// ============================================================================

#[test]
fn test_bash_input_roundtrip() {
    let original = BashInput {
        command: "echo hello".to_string(),
        description: Some("Print hello".to_string()),
        timeout: Some(5000),
        run_in_background: Some(false),
    };

    let json = serde_json::to_value(&original).unwrap();
    let parsed: BashInput = serde_json::from_value(json).unwrap();

    assert_eq!(original, parsed);
}

#[test]
fn test_tool_input_enum_roundtrip() {
    let original = ToolInput::Bash(BashInput {
        command: "ls -la".to_string(),
        description: Some("List files".to_string()),
        timeout: None,
        run_in_background: None,
    });

    let json = serde_json::to_value(&original).unwrap();
    let parsed: ToolInput = serde_json::from_value(json).unwrap();

    assert_eq!(original, parsed);
}
