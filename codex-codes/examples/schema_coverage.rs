//! Codex app-server protocol coverage scorecard.
//!
//! Walks the upstream JSON Schema bundle (`codex_app_server_protocol.v2.schemas.json`)
//! and reports, for every method in `ServerNotification.oneOf` and
//! `ClientRequest.oneOf`:
//!
//!   ✓  modeled in `codex-codes` AND a hand-rolled sample validates against
//!      the schema's params definition (catches wire drift)
//!   ◐  modeled in `codex-codes` but we don't have a sample yet (so we don't
//!      know whether our struct's serde shape matches the wire — grow the
//!      sample registry below to fix)
//!   ✗  not modeled in `codex-codes` at all
//!
//! ## Run
//!
//! ```
//! cargo run --example schema_coverage
//! ```
//!
//! Uses the snapshot schema at `tests/schemas/codex_app_server_protocol.v2.schemas.json`
//! by default; override with `CODEX_SCHEMA_PATH=/path/to/freshly-generated.json`
//! (e.g. what `codex app-server generate-json-schema --out DIR` writes).
//!
//! ## Exit codes
//!
//! * `0` — no drift detected on any *validated* sample (unmodeled methods are
//!   informational, not failures)
//! * `1` — at least one modeled sample failed to validate; the wire shape has
//!   drifted from our typed struct
//!
//! ## Adding samples
//!
//! Each modeled method should have an entry in `samples::server_notification`
//! or `samples::client_request` that returns a representative serialized
//! payload constructed from real Rust types. New variants without samples
//! show up as ◐; that's a backlog item, not a failure.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use jsonschema::Validator;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    /// Method modeled and a sample serialized JSON validates against the schema.
    Validated,
    /// Method modeled but no sample registered.
    NoSample,
    /// Method modeled, sample serialized, but did NOT match the schema.
    Drift,
    /// Method not modeled at all.
    NotModeled,
}

impl Status {
    fn glyph(self) -> &'static str {
        match self {
            Status::Validated => "✓",
            Status::NoSample => "◐",
            Status::Drift => "⚠",
            Status::NotModeled => "✗",
        }
    }

    fn descr(self) -> &'static str {
        match self {
            Status::Validated => "modeled, sample validates",
            Status::NoSample => "modeled, no sample yet",
            Status::Drift => "modeled, sample does NOT validate (DRIFT)",
            Status::NotModeled => "not modeled",
        }
    }
}

#[derive(Debug)]
struct Row {
    method: String,
    params_def: String,
    status: Status,
    errors: Vec<String>,
}

fn main() -> ExitCode {
    let path = std::env::var("CODEX_SCHEMA_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/schemas/codex_app_server_protocol.v2.schemas.json")
        });

    let bundle: Value = match std::fs::read_to_string(&path)
        .map_err(|e| format!("read: {}", e))
        .and_then(|s| serde_json::from_str(&s).map_err(|e| format!("parse: {}", e)))
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: could not load schema at {}: {}", path.display(), e);
            return ExitCode::from(2);
        }
    };

    let modeled_notifications = samples::modeled_notification_methods();
    let modeled_requests = samples::modeled_client_request_methods();
    let modeled_server_reqs = samples::modeled_server_request_methods();
    let notif_samples = samples::server_notification_samples();
    let req_samples = samples::client_request_samples();
    let server_req_samples = samples::server_request_samples();

    let server_rows = walk_envelope(
        &bundle,
        "ServerNotification",
        &modeled_notifications,
        &notif_samples,
    );
    let request_rows = walk_envelope(&bundle, "ClientRequest", &modeled_requests, &req_samples);

    // ServerRequest envelope only exists in the full (non-flat) bundle. Try to
    // load it from a sibling file in the same dir; if not present, walk it
    // out of the generated samples list instead (matching against any
    // *Params definition present in the flat bundle).
    let server_req_rows =
        walk_server_request(&bundle, &path, &modeled_server_reqs, &server_req_samples);

    print_report(&path, &server_rows, &request_rows, &server_req_rows);

    let drift_any = server_rows
        .iter()
        .chain(&request_rows)
        .chain(&server_req_rows)
        .any(|r| r.status == Status::Drift);
    if drift_any {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// Walk the `ServerRequest.oneOf` envelope from the FULL schema bundle (which
/// keeps the approval-flow methods enumerated), then validate each sample
/// against the FLAT (v2) bundle's per-type definitions.
fn walk_server_request(
    v2_bundle: &Value,
    v2_path: &std::path::Path,
    modeled: &std::collections::HashSet<&'static str>,
    sample_registry: &BTreeMap<&'static str, Value>,
) -> Vec<Row> {
    // Try sibling: replace `.v2.schemas.json` with `.schemas.json`.
    let v2_fname = v2_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let full_fname = v2_fname.replace(".v2.", ".");
    let full_path = v2_path.with_file_name(full_fname);

    let envelope_source = if full_path.exists() {
        match std::fs::read_to_string(&full_path)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        {
            Some(v) => v,
            None => return Vec::new(),
        }
    } else {
        // Build a synthetic envelope from the registered server-request method
        // names so the scorecard still reports SOMETHING.
        let mut variants = Vec::new();
        for method in modeled {
            variants.push(serde_json::json!({
                "properties": {"method": {"enum": [method]}}
            }));
        }
        serde_json::json!({"definitions": {"ServerRequest": {"oneOf": variants}}})
    };

    let mut rows = walk_envelope(&envelope_source, "ServerRequest", modeled, sample_registry);
    // Re-resolve params definitions against the v2 bundle (the validation
    // target), since some *Params types only live in the full bundle.
    for row in rows.iter_mut() {
        if row.params_def != "<no params>"
            && !v2_bundle["definitions"][&row.params_def].is_object()
            && envelope_source["definitions"][&row.params_def].is_object()
        {
            // The full bundle has it, the v2 doesn't — validate against the
            // full bundle's definitions table by re-running the validator with
            // the full bundle.
            if let Some(sample) = sample_registry.get(row.method.as_str()) {
                match validate_against_def(&envelope_source, &row.params_def, sample) {
                    Ok(()) => row.status = Status::Validated,
                    Err(errs) => {
                        row.status = Status::Drift;
                        row.errors = errs;
                    }
                }
            }
        }
    }
    rows
}

fn walk_envelope(
    bundle: &Value,
    envelope_name: &str,
    modeled: &std::collections::HashSet<&'static str>,
    sample_registry: &BTreeMap<&'static str, Value>,
) -> Vec<Row> {
    let envelope = match &bundle["definitions"][envelope_name] {
        Value::Object(o) => o,
        _ => {
            eprintln!("warning: schema has no definition for {}", envelope_name);
            return Vec::new();
        }
    };
    let Some(Value::Array(variants)) = envelope.get("oneOf") else {
        eprintln!(
            "warning: definitions.{}.oneOf missing or not an array",
            envelope_name
        );
        return Vec::new();
    };

    let mut rows: Vec<Row> = variants
        .iter()
        .filter_map(extract_envelope_variant)
        .collect();

    // Sort by method for stable output.
    rows.sort_by(|a, b| a.method.cmp(&b.method));

    for row in rows.iter_mut() {
        if !modeled.contains(row.method.as_str()) {
            row.status = Status::NotModeled;
            continue;
        }
        // Methods that take no params are trivially validated as long as
        // the sample is null / empty / absent.
        if row.params_def == "<no params>" {
            row.status = Status::Validated;
            continue;
        }
        match sample_registry.get(row.method.as_str()) {
            None => {
                row.status = Status::NoSample;
            }
            Some(sample) => match validate_against_def(bundle, &row.params_def, sample) {
                Ok(()) => row.status = Status::Validated,
                Err(errs) => {
                    row.status = Status::Drift;
                    row.errors = errs;
                }
            },
        }
    }

    rows
}

fn extract_envelope_variant(v: &Value) -> Option<Row> {
    let obj = v.as_object()?;
    let props = obj.get("properties")?.as_object()?;

    // method: { enum: ["..."] }
    let method = props
        .get("method")?
        .get("enum")?
        .as_array()?
        .first()?
        .as_str()?
        .to_string();

    // params: { $ref: "#/definitions/SomeType" }  -- some variants omit params.
    let params_def = props
        .get("params")
        .and_then(|p| p.get("$ref"))
        .and_then(|r| r.as_str())
        .and_then(|s| s.strip_prefix("#/definitions/"))
        .unwrap_or("<no params>")
        .to_string();

    Some(Row {
        method,
        params_def,
        status: Status::NotModeled,
        errors: Vec::new(),
    })
}

/// Compile a single-definition schema against the full bundle's `definitions`
/// table so `$ref` lookups resolve, then validate `sample`.
fn validate_against_def(bundle: &Value, def_name: &str, sample: &Value) -> Result<(), Vec<String>> {
    let wrapped = serde_json::json!({
        "$ref": format!("#/definitions/{}", def_name),
        "definitions": bundle["definitions"].clone(),
    });
    let validator = match Validator::new(&wrapped) {
        Ok(v) => v,
        Err(e) => return Err(vec![format!("compile error: {}", e)]),
    };
    let errs: Vec<String> = validator
        .iter_errors(sample)
        .map(|e| format!("{} ({})", e, e.instance_path()))
        .collect();
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}

fn print_report(path: &std::path::Path, server: &[Row], requests: &[Row], approvals: &[Row]) {
    println!("Codex app-server protocol coverage — codex-codes scorecard");
    println!("============================================================");
    println!("schema: {}", path.display());
    println!();
    print_section("Server → Client Notifications", server);
    print_section("Client → Server Requests", requests);
    print_section("Server → Client Requests (Approval Flow)", approvals);

    let total = server.len() + requests.len() + approvals.len();
    let modeled = server
        .iter()
        .chain(requests)
        .chain(approvals)
        .filter(|r| {
            matches!(
                r.status,
                Status::Validated | Status::NoSample | Status::Drift
            )
        })
        .count();
    let validated = server
        .iter()
        .chain(requests)
        .chain(approvals)
        .filter(|r| r.status == Status::Validated)
        .count();
    let drift = server
        .iter()
        .chain(requests)
        .chain(approvals)
        .filter(|r| r.status == Status::Drift)
        .count();
    println!("Overall:");
    println!(
        "  modeled:        {modeled}/{total} ({:.0}%)",
        100.0 * modeled as f32 / total.max(1) as f32
    );
    println!(
        "  with sample:    {validated}/{modeled} ({:.0}%)",
        100.0 * validated as f32 / modeled.max(1) as f32
    );
    if drift > 0 {
        println!("  DRIFT:          {drift}");
    }
}

fn print_section(title: &str, rows: &[Row]) {
    let total = rows.len();
    let modeled = rows
        .iter()
        .filter(|r| {
            matches!(
                r.status,
                Status::Validated | Status::NoSample | Status::Drift
            )
        })
        .count();
    let pct = 100.0 * modeled as f32 / total.max(1) as f32;
    println!("{title} ({modeled}/{total} modeled, {pct:.0}%)");
    println!("{}", "-".repeat(title.len() + 40));

    for r in rows {
        println!(
            "  {}  {:<46} → {:<40} {}",
            r.status.glyph(),
            r.method,
            r.params_def,
            r.status.descr()
        );
        for e in &r.errors {
            println!("     {}", e);
        }
    }
    println!();
}

/// Registry of which methods we model and representative wire samples.
///
/// Adding a sample is the cheap way to grow our drift-detection coverage:
/// instantiate the typed struct with realistic field values, return it as a
/// JSON `Value`. The schema validator then checks the wire shape matches.
mod samples {
    use codex_codes::protocol::methods;
    use codex_codes::{ErrorNotification, TurnError};
    use serde_json::Value;
    use std::collections::{BTreeMap, HashSet};

    /// Method strings our `Notification` enum has a typed variant for. Must
    /// stay in sync with [`codex_codes::messages::Notification`].
    pub(super) fn modeled_notification_methods() -> HashSet<&'static str> {
        [
            methods::THREAD_STARTED,
            methods::THREAD_STATUS_CHANGED,
            methods::THREAD_TOKEN_USAGE_UPDATED,
            methods::TURN_STARTED,
            methods::TURN_COMPLETED,
            methods::ITEM_STARTED,
            methods::ITEM_COMPLETED,
            methods::AGENT_MESSAGE_DELTA,
            methods::CMD_OUTPUT_DELTA,
            methods::FILE_CHANGE_OUTPUT_DELTA,
            methods::REASONING_DELTA,
            methods::ERROR,
            methods::ACCOUNT_RATE_LIMITS_UPDATED,
            methods::MCP_SERVER_STARTUP_STATUS_UPDATED,
            methods::MCP_SERVER_OAUTH_LOGIN_COMPLETED,
            methods::REMOTE_CONTROL_STATUS_CHANGED,
            methods::FILE_CHANGE_PATCH_UPDATED,
            methods::PLAN_DELTA,
            methods::TURN_PLAN_UPDATED,
            methods::TURN_DIFF_UPDATED,
            methods::REASONING_SUMMARY_PART_ADDED,
            methods::REASONING_TEXT_DELTA,
            methods::ACCOUNT_LOGIN_COMPLETED,
            methods::DEPRECATION_NOTICE,
            methods::GUARDIAN_WARNING,
            methods::WARNING,
            methods::THREAD_ARCHIVED,
            methods::THREAD_CLOSED,
            methods::THREAD_UNARCHIVED,
            methods::THREAD_GOAL_CLEARED,
            methods::THREAD_NAME_UPDATED,
            methods::SKILLS_CHANGED,
            methods::FS_CHANGED,
            methods::CONFIG_WARNING,
            methods::ACCOUNT_UPDATED,
            methods::APP_LIST_UPDATED,
            methods::COMMAND_EXEC_OUTPUT_DELTA,
            methods::EXTERNAL_AGENT_CONFIG_IMPORT_COMPLETED,
            methods::FUZZY_FILE_SEARCH_SESSION_COMPLETED,
            methods::FUZZY_FILE_SEARCH_SESSION_UPDATED,
            methods::HOOK_COMPLETED,
            methods::HOOK_STARTED,
            methods::ITEM_AUTO_APPROVAL_REVIEW_COMPLETED,
            methods::ITEM_AUTO_APPROVAL_REVIEW_STARTED,
            methods::ITEM_COMMAND_EXEC_TERMINAL_INTERACTION,
            methods::ITEM_MCP_TOOL_CALL_PROGRESS,
            methods::MODEL_REROUTED,
            methods::MODEL_VERIFICATION,
            methods::PROCESS_EXITED,
            methods::PROCESS_OUTPUT_DELTA,
            methods::SERVER_REQUEST_RESOLVED,
            methods::THREAD_COMPACTED,
            methods::THREAD_GOAL_UPDATED,
            methods::THREAD_REALTIME_CLOSED,
            methods::THREAD_REALTIME_ERROR,
            methods::THREAD_REALTIME_ITEM_ADDED,
            methods::THREAD_REALTIME_OUTPUT_AUDIO_DELTA,
            methods::THREAD_REALTIME_SDP,
            methods::THREAD_REALTIME_STARTED,
            methods::THREAD_REALTIME_TRANSCRIPT_DELTA,
            methods::THREAD_REALTIME_TRANSCRIPT_DONE,
            methods::WINDOWS_WORLD_WRITABLE_WARNING,
            methods::WINDOWS_SANDBOX_SETUP_COMPLETED,
            methods::THREAD_SETTINGS_UPDATED,
            methods::TURN_MODERATION_METADATA,
            methods::THREAD_DELETED,
            methods::EXTERNAL_AGENT_CONFIG_IMPORT_PROGRESS,
            methods::MODEL_SAFETY_BUFFERING_UPDATED,
            methods::THREAD_ENVIRONMENT_CONNECTED,
            methods::THREAD_ENVIRONMENT_DISCONNECTED,
        ]
        .into_iter()
        .collect()
    }

    /// Method strings our client request surface (Params/Response pairs) covers.
    /// Must stay in sync with `client_async.rs` / `client_sync.rs`.
    pub(super) fn modeled_client_request_methods() -> HashSet<&'static str> {
        [
            methods::INITIALIZE,
            methods::INITIALIZED,
            methods::THREAD_START,
            methods::THREAD_ARCHIVE,
            methods::THREAD_DELETE,
            methods::TURN_START,
            methods::TURN_INTERRUPT,
            methods::TURN_STEER,
            methods::THREAD_RESUME,
            methods::THREAD_FORK,
            methods::THREAD_UNSUBSCRIBE,
            methods::THREAD_NAME_SET,
            methods::THREAD_METADATA_UPDATE,
            methods::THREAD_UNARCHIVE,
            methods::THREAD_COMPACT_START,
            methods::THREAD_SHELLCOMMAND,
            methods::THREAD_APPROVEGUARDIANDENIEDACTION,
            methods::THREAD_ROLLBACK,
            methods::THREAD_LIST,
            methods::THREAD_LOADED_LIST,
            methods::THREAD_READ,
            methods::THREAD_INJECT_ITEMS,
            methods::THREAD_ITEMS_LIST,
            methods::THREAD_TURNS_LIST,
            methods::THREAD_REVERT,
            methods::SKILLS_LIST,
            methods::HOOKS_LIST,
            methods::MARKETPLACE_ADD,
            methods::MARKETPLACE_REMOVE,
            methods::MARKETPLACE_UPGRADE,
            methods::PLUGIN_LIST,
            methods::PLUGIN_READ,
            methods::PLUGIN_SKILL_READ,
            methods::PLUGIN_SHARE_SAVE,
            methods::PLUGIN_SHARE_UPDATETARGETS,
            methods::PLUGIN_SHARE_LIST,
            methods::PLUGIN_SHARE_CHECKOUT,
            methods::PLUGIN_SHARE_DELETE,
            methods::APP_LIST,
            methods::APP_READ,
            methods::APP_INSTALLED,
            methods::FS_READFILE,
            methods::FS_WRITEFILE,
            methods::FS_CREATEDIRECTORY,
            methods::FS_GETMETADATA,
            methods::FS_READDIRECTORY,
            methods::FS_REMOVE,
            methods::FS_COPY,
            methods::FS_WATCH,
            methods::FS_UNWATCH,
            methods::SKILLS_CONFIG_WRITE,
            methods::PLUGIN_INSTALL,
            methods::PLUGIN_UNINSTALL,
            methods::REVIEW_START,
            methods::MODEL_LIST,
            methods::MODELPROVIDER_CAPABILITIES_READ,
            methods::EXPERIMENTALFEATURE_LIST,
            methods::EXPERIMENTALFEATURE_ENABLEMENT_SET,
            methods::MCPSERVER_OAUTH_LOGIN,
            methods::CONFIG_MCPSERVER_RELOAD,
            methods::MCPSERVERSTATUS_LIST,
            methods::MCPSERVER_RESOURCE_READ,
            methods::MCPSERVER_TOOL_CALL,
            methods::WINDOWSSANDBOX_SETUPSTART,
            methods::WINDOWSSANDBOX_READINESS,
            methods::ACCOUNT_LOGIN_START,
            methods::ACCOUNT_LOGIN_CANCEL,
            methods::ACCOUNT_LOGOUT,
            methods::ACCOUNT_RATELIMITS_READ,
            methods::ACCOUNT_SENDADDCREDITSNUDGEEMAIL,
            methods::FEEDBACK_UPLOAD,
            methods::COMMAND_EXEC,
            methods::COMMAND_EXEC_WRITE,
            methods::COMMAND_EXEC_TERMINATE,
            methods::COMMAND_EXEC_RESIZE,
            methods::CONFIG_READ,
            methods::EXTERNALAGENTCONFIG_DETECT,
            methods::EXTERNALAGENTCONFIG_IMPORT,
            methods::CONFIG_VALUE_WRITE,
            methods::CONFIG_BATCHWRITE,
            methods::CONFIGREQUIREMENTS_READ,
            methods::ACCOUNT_READ,
            methods::FUZZYFILESEARCH,
            methods::ACCOUNT_USAGE_READ,
            methods::PERMISSION_PROFILE_LIST,
            methods::PLUGIN_INSTALLED,
            methods::SKILLS_EXTRA_ROOTS_SET,
            methods::THREAD_GOAL_GET,
            methods::THREAD_GOAL_SET,
            methods::THREAD_GOAL_CLEAR,
            methods::THREADSECTION_LIST,
            methods::THREADSECTION_CREATE,
            methods::THREADSECTION_UPDATE,
            methods::THREADSECTION_DELETE,
            methods::THREAD_SECTION_MOVE,
            methods::ACCOUNT_RATELIMITRESETCREDIT_CONSUME,
            methods::ACCOUNT_WORKSPACEMESSAGES_READ,
            methods::EXTERNALAGENTCONFIG_IMPORT_READHISTORIES,
            methods::EXTERNALAGENTCONFIG_IMPORT_RECORDHISTORY,
        ]
        .into_iter()
        .collect()
    }

    /// Hand-rolled wire samples for server-notification params. The map's key
    /// is the JSON-RPC `method`; the value is what `serde_json::to_value` on
    /// the corresponding typed struct produces.
    ///
    /// **Seed set only** — grow this as we add typed variants. Methods absent
    /// from this map show up as ◐ ("modeled, no sample") in the report.
    pub(super) fn server_notification_samples() -> BTreeMap<&'static str, Value> {
        let mut m: BTreeMap<&'static str, Value> = BTreeMap::new();
        // Generated registry — one minimal valid sample per method.
        for (method, sample) in
            codex_codes::protocol_generated::samples::server_notification_samples()
        {
            m.insert(method, sample);
        }
        // Hand-tuned override for `error` with realistic field values.
        m.insert(
            methods::ERROR,
            serde_json::to_value(ErrorNotification {
                error: TurnError {
                    message: "something blew up".into(),
                    additional_details: None,
                    codex_error_info: None,
                    misalignment: None,
                },
                thread_id: "th_abc".into(),
                turn_id: "tn_xyz".into(),
                will_retry: false,
            })
            .expect("ErrorNotification serializes"),
        );
        m
    }

    /// Client-request samples sourced from the generated registry.
    pub(super) fn client_request_samples() -> BTreeMap<&'static str, Value> {
        let mut m: BTreeMap<&'static str, Value> = BTreeMap::new();
        for (method, sample) in codex_codes::protocol_generated::samples::client_request_samples() {
            m.insert(method, sample);
        }
        m.insert(
            methods::THREAD_ITEMS_LIST,
            serde_json::to_value(codex_codes::ThreadItemsListParams {
                thread_id: "thread-1".into(),
                ..Default::default()
            })
            .expect("ThreadItemsListParams serializes"),
        );
        m.insert(
            methods::THREAD_TURNS_LIST,
            serde_json::to_value(codex_codes::ThreadTurnsListParams {
                thread_id: "thread-1".into(),
                ..Default::default()
            })
            .expect("ThreadTurnsListParams serializes"),
        );
        m.insert(
            methods::THREAD_REVERT,
            serde_json::to_value(codex_codes::ThreadRevertParams {
                before_turn_id: "turn-2".into(),
                thread_id: "thread-1".into(),
            })
            .expect("ThreadRevertParams serializes"),
        );
        m
    }

    /// Server-request (approval-flow) samples sourced from the generated registry.
    pub(super) fn server_request_samples() -> BTreeMap<&'static str, Value> {
        let mut m: BTreeMap<&'static str, Value> = BTreeMap::new();
        for (method, sample) in codex_codes::protocol_generated::samples::server_request_samples() {
            m.insert(method, sample);
        }
        m
    }

    /// Methods our crate models for the server-request (approval) envelope.
    pub(super) fn modeled_server_request_methods() -> HashSet<&'static str> {
        codex_codes::protocol_generated::samples::server_request_samples()
            .into_iter()
            .map(|(m, _)| m)
            .collect()
    }
}
