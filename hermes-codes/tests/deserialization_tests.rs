//! Golden-corpus round-trip tests.
//!
//! The fixtures in `tests/golden/` are vendored verbatim from the ACP
//! Python SDK's test corpus (Apache-2.0, agentclientprotocol/python-sdk
//! 0.9.0 — the exact SDK version hermes-agent pins), so every assertion
//! here runs against frames the reference implementation produces.
//!
//! Each case: parse the raw JSON → deserialize into the typed model →
//! reserialize → compare as `serde_json::Value`. Equality proves both
//! directions of the mapping preserve every field.

use serde_json::Value;
use std::path::PathBuf;

fn golden(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/golden/{name}.json"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn assert_roundtrip<T>(name: &str)
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    let raw = golden(name);
    let original: Value = serde_json::from_str(&raw).expect("golden file is valid JSON");
    let typed: T = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("{name}: failed to deserialize into typed model: {e}"));
    let back = serde_json::to_value(&typed).expect("reserialize");
    assert_eq!(original, back, "{name}: round-trip drift");
}

macro_rules! golden_roundtrip {
    ($($test:ident: $ty:ty => $file:literal;)+) => {
        $(
            #[test]
            fn $test() {
                assert_roundtrip::<$ty>($file);
            }
        )+
    };
}

golden_roundtrip! {
    initialize_request: hermes_codes::InitializeRequest => "initialize_request";
    initialize_response: hermes_codes::InitializeResponse => "initialize_response";
    new_session_request: hermes_codes::NewSessionRequest => "new_session_request";
    new_session_response: hermes_codes::NewSessionResponse => "new_session_response";
    prompt_request: hermes_codes::PromptRequest => "prompt_request";
    cancel_notification: hermes_codes::CancelNotification => "cancel_notification";
    request_permission_request: hermes_codes::RequestPermissionRequest => "request_permission_request";
    request_permission_response_selected: hermes_codes::RequestPermissionResponse => "request_permission_response_selected";
    fs_read_text_file_response: hermes_codes::ReadTextFileResponse => "fs_read_text_file_response";
    permission_outcome_selected: hermes_codes::RequestPermissionOutcome => "permission_outcome_selected";
    permission_outcome_cancelled: hermes_codes::RequestPermissionOutcome => "permission_outcome_cancelled";
    set_session_config_option_request: hermes_codes::SetSessionConfigOptionRequest => "set_session_config_option_request";
    session_update_agent_message_chunk: hermes_codes::SessionUpdate => "session_update_agent_message_chunk";
    session_update_agent_thought_chunk: hermes_codes::SessionUpdate => "session_update_agent_thought_chunk";
    session_update_user_message_chunk: hermes_codes::SessionUpdate => "session_update_user_message_chunk";
    session_update_plan: hermes_codes::SessionUpdate => "session_update_plan";
    session_update_tool_call: hermes_codes::SessionUpdate => "session_update_tool_call";
    session_update_tool_call_edit: hermes_codes::SessionUpdate => "session_update_tool_call_edit";
    session_update_tool_call_read: hermes_codes::SessionUpdate => "session_update_tool_call_read";
    session_update_tool_call_update_content: hermes_codes::SessionUpdate => "session_update_tool_call_update_content";
    session_update_tool_call_update_more_fields: hermes_codes::SessionUpdate => "session_update_tool_call_update_more_fields";
    session_update_tool_call_locations_rawinput: hermes_codes::SessionUpdate => "session_update_tool_call_locations_rawinput";
    session_update_config_option_update: hermes_codes::SessionUpdate => "session_update_config_option_update";
    fs_read_text_file_request: hermes_codes::ReadTextFileRequest => "fs_read_text_file_request";
    fs_write_text_file_request: hermes_codes::WriteTextFileRequest => "fs_write_text_file_request";
    content_text: hermes_codes::ContentBlock => "content_text";
    content_image: hermes_codes::ContentBlock => "content_image";
    content_audio: hermes_codes::ContentBlock => "content_audio";
    content_resource_text: hermes_codes::ContentBlock => "content_resource_text";
    content_resource_blob: hermes_codes::ContentBlock => "content_resource_blob";
    content_resource_link: hermes_codes::ContentBlock => "content_resource_link";
    tool_content_content_text: hermes_codes::ToolCallContent => "tool_content_content_text";
    tool_content_diff: hermes_codes::ToolCallContent => "tool_content_diff";
    tool_content_diff_no_old: hermes_codes::ToolCallContent => "tool_content_diff_no_old";
    tool_content_terminal: hermes_codes::ToolCallContent => "tool_content_terminal";
    set_session_config_option_response: hermes_codes::SetSessionConfigOptionResponse => "set_session_config_option_response";
}

/// Every golden file must at minimum parse as JSON — catches vendoring rot
/// for fixtures not yet covered by a typed round-trip above.
#[test]
fn all_golden_files_are_valid_json() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let mut count = 0;
    for entry in std::fs::read_dir(&dir).expect("golden dir") {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let raw = std::fs::read_to_string(&path).expect("read");
            serde_json::from_str::<Value>(&raw)
                .unwrap_or_else(|e| panic!("{}: invalid JSON: {e}", path.display()));
            count += 1;
        }
    }
    assert!(
        count >= 30,
        "expected the vendored corpus, found {count} files"
    );
}
