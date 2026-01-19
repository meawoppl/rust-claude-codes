//! Integration tests for typed tool inputs.
//!
//! These tests verify that real Claude CLI tool use messages can be deserialized
//! and their inputs can be parsed into strongly-typed structures.

use claude_codes::{
    BashInput, ClaudeOutput, ToolInput, ToolUseBlock,
};
use claude_codes::io::ContentBlock;
use serde_json::json;

// ============================================================================
// Tests using captured real messages from test_cases/tool_use_captures/
// ============================================================================

/// Test parsing the system init message (tool_msg_0.json)
#[test]
fn test_parse_system_init_message() {
    let json_str = include_str!("../test_cases/tool_use_captures/tool_msg_0.json");
    let output: ClaudeOutput = serde_json::from_str(json_str).expect("Failed to parse system init message");

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
            println!("System init message parsed successfully with {} tools", tools_array.len());
        }
        _ => panic!("Expected System message, got {:?}", output.message_type()),
    }
}

/// Test parsing assistant message with Bash tool use (tool_msg_1.json)
#[test]
fn test_parse_bash_tool_use_message() {
    let json_str = include_str!("../test_cases/tool_use_captures/tool_msg_1.json");
    let output: ClaudeOutput = serde_json::from_str(json_str).expect("Failed to parse assistant message");

    match output {
        ClaudeOutput::Assistant(msg) => {
            assert_eq!(msg.message.role, "assistant");
            assert_eq!(msg.message.content.len(), 1);

            if let ContentBlock::ToolUse(tool_use) = &msg.message.content[0] {
                assert_eq!(tool_use.name, "Bash");
                assert_eq!(tool_use.id, "toolu_01F7wwFsuQE8bTbjP4Aig5Ab");

                // Test typed_input() method
                let typed = tool_use.typed_input().expect("Failed to get typed input");
                match typed {
                    ToolInput::Bash(bash) => {
                        assert_eq!(bash.command, "ls -la /tmp");
                        assert_eq!(bash.description, Some("List files in /tmp directory".to_string()));
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
                assert_eq!(bash.description, Some("Show current date and time".to_string()));
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
        // Check stop_reason is present
        assert_eq!(msg.message.stop_reason, Some("tool_use".to_string()));

        if let ContentBlock::ToolUse(tool_use) = &msg.message.content[0] {
            let typed = tool_use.typed_input().unwrap();
            if let ToolInput::Bash(bash) = typed {
                assert!(bash.command.contains("test -f /etc/passwd"));
                assert_eq!(bash.description, Some("Check if /etc/passwd exists".to_string()));
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
            assert_eq!(result.tool_use_id, "toolu_01F7wwFsuQE8bTbjP4Aig5Ab");
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
            assert_eq!(result.tool_use_id, "toolu_011U4kgQ9oshx2Z86oR12AtY");
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

        // Parse the first denial's tool_input as BashInput
        let denial1 = &result.permission_denials[0];
        let tool_name = denial1.get("tool_name").unwrap().as_str().unwrap();
        assert_eq!(tool_name, "Bash");

        let tool_input = denial1.get("tool_input").unwrap();
        let bash: BashInput = serde_json::from_value(tool_input.clone()).expect("Failed to parse tool_input");
        assert_eq!(bash.command, "ls -la /tmp");
        assert_eq!(bash.description, Some("List files in /tmp directory".to_string()));

        // Parse the second denial
        let denial2 = &result.permission_denials[1];
        let tool_input2 = denial2.get("tool_input").unwrap();
        let bash2: BashInput = serde_json::from_value(tool_input2.clone()).unwrap();
        assert!(bash2.command.contains("test -f /etc/passwd"));

        println!("Parsed result with {} permission denials", result.permission_denials.len());
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
    assert_eq!(question.questions[0].question, "Which database should we use?");
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
