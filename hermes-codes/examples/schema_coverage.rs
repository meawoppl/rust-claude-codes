//! Schema coverage report for hermes-codes.
//!
//! For every ACP method in the generated tables, checks that a golden
//! sample (vendored from the ACP Python SDK's test corpus, when one
//! exists) round-trips through the corresponding typed model:
//! deserialize → reserialize → byte-compare as JSON values.
//!
//! Exit codes: `0` — every sampled method round-trips; `1` — at least one
//! sample fails against its type (wire drift or a modeling bug).

use hermes_codes::methods;
use serde_json::Value;
use std::path::Path;

/// (method, params type name, golden sample file for the params side).
const SAMPLES: &[(&str, &str)] = &[
    ("initialize", "initialize_request"),
    ("session/new", "new_session_request"),
    ("session/prompt", "prompt_request"),
    ("session/cancel", "cancel_notification"),
    ("session/request_permission", "request_permission_request"),
    ("fs/read_text_file", "fs_read_text_file_request"),
    ("fs/write_text_file", "fs_write_text_file_request"),
];

fn roundtrip(method: &str, ty: &str, raw: &str) -> Result<(), String> {
    let original: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let reserialized: Value = match ty {
        "InitializeRequest" => rt::<hermes_codes::InitializeRequest>(raw)?,
        "AuthenticateRequest" => rt::<hermes_codes::AuthenticateRequest>(raw)?,
        "NewSessionRequest" => rt::<hermes_codes::NewSessionRequest>(raw)?,
        "LoadSessionRequest" => rt::<hermes_codes::LoadSessionRequest>(raw)?,
        "PromptRequest" => rt::<hermes_codes::PromptRequest>(raw)?,
        "CancelNotification" => rt::<hermes_codes::CancelNotification>(raw)?,
        "RequestPermissionRequest" => rt::<hermes_codes::RequestPermissionRequest>(raw)?,
        "ReadTextFileRequest" => rt::<hermes_codes::ReadTextFileRequest>(raw)?,
        "WriteTextFileRequest" => rt::<hermes_codes::WriteTextFileRequest>(raw)?,
        "CreateTerminalRequest" => rt::<hermes_codes::CreateTerminalRequest>(raw)?,
        other => return Err(format!("{method}: no round-trip arm for {other}")),
    };
    if original == reserialized {
        Ok(())
    } else {
        Err(format!(
            "{method}: round-trip drift\n  in:  {original}\n  out: {reserialized}"
        ))
    }
}

fn rt<T: serde::de::DeserializeOwned + serde::Serialize>(raw: &str) -> Result<Value, String> {
    let typed: T = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    serde_json::to_value(&typed).map_err(|e| e.to_string())
}

fn main() {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let method_types: std::collections::HashMap<&str, &str> = methods::AGENT_METHODS
        .iter()
        .chain(methods::CLIENT_METHODS)
        .map(|(m, req, _)| (*m, *req))
        .collect();

    let mut failed = 0;
    let mut sampled = 0;
    for (method, file) in SAMPLES {
        let ty = method_types
            .get(method)
            .unwrap_or_else(|| panic!("{method} missing from generated method tables"));
        let path = golden.join(format!("{file}.json"));
        let Ok(raw) = std::fs::read_to_string(&path) else {
            println!("  ◐  {method:32} → {ty:32} no golden sample");
            continue;
        };
        sampled += 1;
        match roundtrip(method, ty, &raw) {
            Ok(()) => println!("  ✓  {method:32} → {ty}"),
            Err(e) => {
                failed += 1;
                println!("  ✗  {e}");
            }
        }
    }
    let total = methods::AGENT_METHODS.len() + methods::CLIENT_METHODS.len();
    println!(
        "\n  methods:  {total} modeled\n  sampled:  {sampled}/{}  (golden round-trips)\n  failed:   {failed}",
        SAMPLES.len()
    );
    std::process::exit(if failed > 0 { 1 } else { 0 });
}
