//! Run a crate's REAL integration tests (`cargo test --features
//! integration-tests`) as a subprocess and stream per-test results into
//! the shared state. This is the no-drift tier: nothing is ported or
//! duplicated — wirecheck executes the same test binaries CI and
//! developers run, and presents their outcomes.

use crate::state::{CheckResult, CheckStatus, Shared};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};

/// Which crate + extras each agent's cargo tier runs.
pub fn plan(agent: &str) -> Option<(&'static str, &'static [(&'static str, &'static str)])> {
    match agent {
        "claude" => Some(("claude-codes", &[])),
        "codex" => Some(("codex-codes", &[])),
        "muse" => Some(("muse-codes", &[])),
        // opencode's tests target a base URL; the caller pre-spawns a
        // managed server and passes it via env.
        "opencode" => Some(("opencode-codes", &[])),
        _ => None,
    }
}

pub async fn run(state: Shared, agent: &'static str) {
    let Some((package, _)) = plan(agent) else {
        return;
    };
    let descriptions = load_descriptions(package).await;

    // opencode's integration tests read OPENCODE_BASE_URL; spawn a managed
    // server for the duration so they run hermetically.
    let mut managed: Option<opencode_codes::server::ManagedServer> = None;
    let mut envs: Vec<(String, String)> = Vec::new();
    if agent == "opencode" {
        set_status(&state, agent, "starting managed opencode server…").await;
        match opencode_codes::server::ManagedServer::builder()
            .startup_timeout(std::time::Duration::from_secs(30))
            .spawn()
            .await
        {
            Ok(server) => {
                envs.push(("OPENCODE_BASE_URL".into(), server.url().to_string()));
                managed = Some(server);
            }
            Err(e) => {
                push_result(
                    &state,
                    agent,
                    CheckResult {
                        name: "managed server".into(),
                        what: String::new(),
                        status: CheckStatus::Fail,
                        detail: format!("could not spawn opencode serve: {e}"),
                        ms: None,
                    },
                )
                .await;
                finish(&state, agent, None).await;
                return;
            }
        }
    }

    set_status(&state, agent, "cargo test (compiling if needed)…").await;
    // One merged stream (2>&1): cargo announces each test binary with a
    // "Running tests/<file>.rs" header on STDERR while libtest results go
    // to stdout — the parser needs them interleaved in order to resolve
    // per-file descriptions.
    let mut cmd = tokio::process::Command::new("sh");
    cmd.arg("-c")
        .arg(format!(
            "cargo test -p {package} --features integration-tests -- --test-threads=1 2>&1"
        ))
        .current_dir(env!("CARGO_MANIFEST_DIR").trim_end_matches("/wirecheck"))
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    for (k, v) in &envs {
        cmd.env(k, v);
    }

    let started = std::time::Instant::now();
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            push_result(
                &state,
                agent,
                CheckResult {
                    name: "cargo test".into(),
                    what: String::new(),
                    status: CheckStatus::Fail,
                    detail: e.to_string(),
                    ms: None,
                },
            )
            .await;
            finish(&state, agent, managed).await;
            return;
        }
    };

    // libtest lines: `test path::to::name ... ok|FAILED|ignored`.
    // Failure details follow in `---- name stdout ----` blocks; collect the
    // whole stdout so they can be attached after the run.
    let stdout = child.stdout.take();
    let mut full = String::new();
    // Which test binary is currently reporting — libtest prints a
    // "Running tests/<file>.rs (…)" header before each — so descriptions
    // resolve per-file and same-named tests in different files can't
    // cross-wire.
    let mut current_file: Option<String> = None;
    if let Some(out) = stdout {
        let mut lines = BufReader::new(out).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            full.push_str(&line);
            full.push('\n');
            if let Some(rest) = line.trim_start().strip_prefix("Running ") {
                current_file = rest
                    .split_whitespace()
                    .next()
                    .and_then(|p| p.rsplit('/').next())
                    .and_then(|f| f.strip_suffix(".rs"))
                    .map(str::to_string);
            }
            if let Some(rest) = line.strip_prefix("test ") {
                if let Some((name, verdict)) = rest.rsplit_once(" ... ") {
                    // Skip doc-test headers and the summary line.
                    if name.contains("::") || !name.contains(' ') {
                        let status = match verdict.trim() {
                            v if v.starts_with("ok") => CheckStatus::Pass,
                            v if v.starts_with("ignored") => CheckStatus::Skipped,
                            _ => CheckStatus::Fail,
                        };
                        let short = name.rsplit("::").next().unwrap_or(name).to_string();
                        let what = current_file
                            .as_deref()
                            .and_then(|file| descriptions.get(file))
                            .and_then(|fns| fns.get(&short))
                            .cloned()
                            .unwrap_or_default();
                        set_status(&state, agent, &format!("ran {short}")).await;
                        push_result(
                            &state,
                            agent,
                            CheckResult {
                                name: short,
                                what,
                                status,
                                detail: String::new(),
                                ms: None,
                            },
                        )
                        .await;
                    }
                }
            }
        }
    }
    let status = child.wait().await;
    let elapsed = started.elapsed().as_millis();

    // Attach failure output blocks to their tests.
    attach_failures(&state, agent, &full).await;

    let summary = match &status {
        Ok(s) if s.success() => format!("all suites green in {}s", elapsed / 1000),
        Ok(_) => format!(
            "failures present ({}s) — details attached per test",
            elapsed / 1000
        ),
        Err(e) => format!("cargo did not run: {e}"),
    };
    set_status(&state, agent, &summary).await;
    finish(&state, agent, managed).await;
}

/// Test descriptions from the /// doc comments, extracted by the same
/// script CI uses to enforce them: `{file_stem: {fn_name: description}}`.
/// Missing script or crate entry degrades to bare names, never an error.
async fn load_descriptions(
    package: &str,
) -> std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>> {
    let repo_root = env!("CARGO_MANIFEST_DIR").trim_end_matches("/wirecheck");
    let out = tokio::process::Command::new("python3")
        .args(["scripts/check_test_annotations.py", "--emit-json"])
        .current_dir(repo_root)
        .output()
        .await;
    let Ok(out) = out else {
        return Default::default();
    };
    serde_json::from_slice::<serde_json::Value>(&out.stdout)
        .ok()
        .and_then(|v| serde_json::from_value(v.get(package)?.clone()).ok())
        .unwrap_or_default()
}

/// Pull `---- name stdout ----` blocks out of libtest output and attach
/// them (truncated) to the failing test's row.
async fn attach_failures(state: &Shared, agent: &str, full: &str) {
    let mut portal = state.write().await;
    let Some(panel) = portal.agents.get_mut(agent) else {
        return;
    };
    for test in &mut panel.cargo_tests {
        if test.status != CheckStatus::Fail || !test.detail.is_empty() {
            continue;
        }
        let marker = "---- ";
        for block in full.split(marker).skip(1) {
            if let Some((header, body)) = block.split_once(" ----\n") {
                if header.ends_with(&format!("::{} stdout", test.name))
                    || header == format!("{} stdout", test.name)
                {
                    let body = body.split("\n\n").next().unwrap_or(body);
                    test.detail = body.chars().take(600).collect();
                }
            }
        }
        if test.detail.is_empty() {
            test.detail = "failed — full output in the wirecheck server log".into();
        }
    }
}

async fn set_status(state: &Shared, agent: &str, msg: &str) {
    let mut portal = state.write().await;
    if let Some(panel) = portal.agents.get_mut(agent) {
        panel.cargo_status = Some(msg.to_string());
    }
}

async fn push_result(state: &Shared, agent: &str, result: CheckResult) {
    let mut portal = state.write().await;
    if let Some(panel) = portal.agents.get_mut(agent) {
        panel.cargo_tests.retain(|c| c.name != result.name);
        panel.cargo_tests.push(result);
    }
}

async fn finish(
    state: &Shared,
    agent: &str,
    managed: Option<opencode_codes::server::ManagedServer>,
) {
    if let Some(server) = managed {
        let _ = server.stop().await;
    }
    let mut portal = state.write().await;
    if let Some(panel) = portal.agents.get_mut(agent) {
        panel.cargo_running = false;
    }
}
