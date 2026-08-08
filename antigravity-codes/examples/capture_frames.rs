//! Records every frame of a session to disk, verbatim, for use as test fixtures.
//!
//! ```sh
//! export ANTIGRAVITY_HARNESS_PATH=/path/to/localharness
//! export GEMINI_API_KEY=...
//! cargo run -p antigravity-codes --example capture_frames -- ./captures "hello"
//! ```
//!
//! Frames are written as `NNN-<kind>.json`, re-indented but **not** re-encoded:
//! the field set is exactly what the harness sent. That distinction is the whole
//! point — a corpus built from decoded-then-re-encoded frames can only contain
//! fields the crate already models, so it agrees with itself by construction and
//! can never catch a field the types are missing.
//!
//! Options, all via the environment:
//!
//! | Variable | Effect |
//! |---|---|
//! | `ANTIGRAVITY_MODEL` | Model to use (default `gemini-flash-latest`) |
//! | `ANTIGRAVITY_TOOLS` | `read-only` (default), `all`, or `none` |
//! | `ANTIGRAVITY_SUBAGENTS` | Set to let the agent delegate to a subagent |
//! | `ANTIGRAVITY_POLICY` | Set to install a dynamic policy rule on `run_command` |

use std::path::PathBuf;

use antigravity_codes::protocol::{
    HarnessSideTools, InputEvent, OutputEvent, PolicyConfig, PolicyDecision, PolicyRule,
    SubagentsConfig,
};
use antigravity_codes::{HarnessOptions, ModelBuilder, RawClient, Result};

fn kind_of(event: &OutputEvent) -> String {
    let base = if event.step_update.is_some() {
        "step-update"
    } else if event.trajectory_state_update.is_some() {
        "trajectory-state"
    } else if event.tool_call.is_some() {
        "tool-call"
    } else if event.initialize_conversation_response.is_some() {
        "initialize-response"
    } else if event.call_hook_request.is_some() {
        "call-hook-request"
    } else if event.policy_decision_request.is_some() {
        "policy-decision-request"
    } else if event.usage_update.is_some() {
        "usage-update"
    } else if event.session_end_response.is_some() {
        "session-end-response"
    } else {
        "unknown"
    };

    // Name the action a step carries, so a directory of captures is greppable.
    let action = event.step_update.as_ref().and_then(|s| {
        [
            ("list-directory", s.list_directory.is_some()),
            ("find-file", s.find_file.is_some()),
            ("search-directory", s.search_directory.is_some()),
            ("view-file", s.view_file.is_some()),
            ("create-file", s.create_file.is_some()),
            ("edit-file", s.edit_file.is_some()),
            ("run-command", s.run_command.is_some()),
            ("invoke-subagent", s.invoke_subagent.is_some()),
            ("generate-image", s.generate_image.is_some()),
            ("search-web", s.search_web.is_some()),
            ("read-url-content", s.read_url_content.is_some()),
            ("mcp-tool", s.mcp_tool.is_some()),
            ("custom-tool", s.custom_tool.is_some()),
            ("finish", s.finish.is_some()),
            ("error", s.error.is_some()),
            ("questions-request", s.questions_request.is_some()),
            ("tool-confirmation", s.tool_confirmation_request.is_some()),
        ]
        .into_iter()
        .find(|(_, present)| *present)
        .map(|(name, _)| name)
    });

    match action {
        Some(action) => format!("{base}-{action}"),
        None => base.to_string(),
    }
}

fn options() -> HarnessOptions {
    let mut tools = match std::env::var("ANTIGRAVITY_TOOLS").as_deref() {
        Ok("all") => HarnessSideTools::all(),
        Ok("none") => HarnessSideTools::none(),
        _ => HarnessSideTools::read_only(),
    };
    if std::env::var("ANTIGRAVITY_SUBAGENTS").is_ok() {
        tools.subagents = Some(SubagentsConfig {
            enabled: Some(true),
        });
    }

    let mut options = HarnessOptions::new()
        .workspace(std::env::current_dir().expect("a readable working directory"))
        .model(ModelBuilder::gemini(
            std::env::var("ANTIGRAVITY_MODEL").unwrap_or_else(|_| "gemini-flash-latest".into()),
            std::env::var("GEMINI_API_KEY").unwrap_or_default(),
        ))
        .harness_side_tools(tools);

    if std::env::var("ANTIGRAVITY_POLICY").is_ok() {
        // `is_dynamic` is what makes the harness ask the *client* rather than
        // deciding locally, which is the frame worth capturing.
        options = options.policy(PolicyConfig {
            rules: vec![PolicyRule {
                tool: Some("run_command".into()),
                name: Some("ask-before-shell".into()),
                rule_id: Some("shell-guard".into()),
                decision: Some(PolicyDecision::AskUser),
                is_dynamic: Some(true),
                ..Default::default()
            }],
        });
    }

    options
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let mut args = std::env::args().skip(1);
    let out: PathBuf = args.next().unwrap_or_else(|| "captures".into()).into();
    let prompt = args.next().unwrap_or_else(|| "Say hello.".into());
    std::fs::create_dir_all(&out)?;

    let mut client = RawClient::launch(options()).await?;

    // `launch` consumed the initialize frame, but the client kept the raw text
    // so the capture is a complete session rather than one starting mid-stream.
    let initialize: serde_json::Value =
        serde_json::from_str(client.initialize_frame()).unwrap_or(serde_json::Value::Null);
    write(&out, "000-initialize-response.json", &initialize)?;

    client.send(&InputEvent::user(prompt)).await?;

    let mut n = 1;
    while let Some((raw, event)) = client.next_frame().await? {
        // Parsed as a `Value`, not as an `OutputEvent`, so nothing the crate
        // does not model is dropped on the way to disk.
        let value: serde_json::Value =
            serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null);
        write(&out, &format!("{n:03}-{}.json", kind_of(&event)), &value)?;
        n += 1;

        if let Some(state) = event
            .trajectory_state_update
            .as_ref()
            .and_then(|t| t.state.clone())
        {
            println!("trajectory -> {state}");
            if state.as_str() == "STATE_FULLY_IDLE" {
                break;
            }
        }
    }

    println!("wrote {n} frames to {}", out.display());
    client.shutdown().await
}

fn write(dir: &std::path::Path, name: &str, value: &serde_json::Value) -> std::io::Result<()> {
    let body =
        serde_json::to_string_pretty(value).map_err(|e| std::io::Error::other(e.to_string()))?;
    std::fs::write(dir.join(name), body + "\n")
}
