//! wirecheck — a local web portal that logs in to each wrapped agent CLI
//! and runs live wire-format checks against the real binaries.
//!
//! Binds 127.0.0.1 only; expose it through a session-owned tunnel (e.g.
//! `agent-portal forward 4477`). Results are equally consumable by a human
//! (the dashboard) and by an agent (`GET /api/state` is the whole truth as
//! JSON).

mod checks;
mod html;
mod login;
mod state;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Json};
use axum::routing::{get, post};
use serde::Deserialize;
use state::{LoginState, Portal, Shared};
use std::sync::Arc;

#[derive(Clone)]
struct Ctx {
    state: Shared,
    claude_flow: Arc<login::ClaudeFlowSlot>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("run") {
        headless_run(&args[1..]).await;
        return;
    }

    let ctx = Ctx {
        state: Arc::new(tokio::sync::RwLock::new(Portal::new())),
        claude_flow: Arc::new(login::ClaudeFlowSlot::default()),
    };
    refresh_all_auth(&ctx.state).await;

    let app = axum::Router::new()
        .route("/", get(|| async { Html(html::PAGE) }))
        .route("/api/state", get(api_state))
        .route("/api/refresh", post(api_refresh))
        .route("/api/checks/{agent}", post(api_run_checks))
        .route("/api/login/muse/device", post(api_muse_device))
        .route("/api/login/muse/apikey", post(api_muse_apikey))
        .route("/api/login/claude/start", post(api_claude_start))
        .route("/api/login/claude/code", post(api_claude_code))
        .with_state(ctx);

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 4477));
    tracing::info!("wirecheck portal on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind 127.0.0.1:4477");
    axum::serve(listener, app).await.expect("serve");
}

/// One-shot CI/scripting mode: `wirecheck run [agent ...]` runs the named
/// suites (default: all four) sequentially, prints the full state as JSON
/// on stdout, and exits 1 if any check failed — the integration-suite
/// entry point; the server is the same checks with a UI.
async fn headless_run(wanted: &[String]) {
    let all = ["claude", "codex", "muse", "opencode"];
    let agents: Vec<&'static str> = if wanted.is_empty() {
        all.to_vec()
    } else {
        all.iter()
            .copied()
            .filter(|a| wanted.iter().any(|w| w == a))
            .collect()
    };
    if agents.is_empty() {
        eprintln!("no known agents in {wanted:?}; known: {all:?}");
        std::process::exit(2);
    }
    let state: Shared = Arc::new(tokio::sync::RwLock::new(Portal::new()));
    refresh_all_auth(&state).await;
    for agent in &agents {
        eprintln!("── {agent} suite ──");
        let reporter = state::Reporter {
            state: state.clone(),
            agent,
        };
        match *agent {
            "muse" => checks::muse::run_suite(reporter).await,
            "claude" => checks::claude::run_suite(reporter).await,
            "opencode" => checks::opencode::run_suite(reporter).await,
            _ => checks::codex::run_suite(reporter).await,
        }
        let portal = state.read().await;
        if let Some(panel) = portal.agents.get(agent) {
            for c in &panel.checks {
                eprintln!("  [{:?}] {} — {}", c.status, c.name, c.detail);
            }
        }
    }
    let portal = state.read().await;
    println!(
        "{}",
        serde_json::to_string_pretty(&*portal).unwrap_or_else(|_| "{}".into())
    );
    let failed = portal
        .agents
        .iter()
        .filter(|(name, _)| agents.contains(name))
        .flat_map(|(_, p)| &p.checks)
        .filter(|c| c.status == state::CheckStatus::Fail)
        .count();
    if failed > 0 {
        eprintln!("{failed} check(s) FAILED");
        std::process::exit(1);
    }
}

async fn api_state(State(ctx): State<Ctx>) -> Json<serde_json::Value> {
    let portal = ctx.state.read().await;
    Json(serde_json::to_value(&*portal).unwrap_or(serde_json::Value::Null))
}

async fn api_refresh(State(ctx): State<Ctx>) -> StatusCode {
    refresh_all_auth(&ctx.state).await;
    StatusCode::NO_CONTENT
}

async fn api_run_checks(State(ctx): State<Ctx>, Path(agent): Path<String>) -> impl IntoResponse {
    let name: &'static str = match agent.as_str() {
        "muse" => "muse",
        "claude" => "claude",
        "codex" => "codex",
        "opencode" => "opencode",
        _ => return (StatusCode::NOT_FOUND, "unknown agent").into_response(),
    };
    {
        let mut portal = ctx.state.write().await;
        let Some(panel) = portal.agents.get_mut(name) else {
            return (StatusCode::NOT_FOUND, "unknown agent").into_response();
        };
        if panel.checks_running {
            return (StatusCode::CONFLICT, "suite already running").into_response();
        }
        panel.checks_running = true;
        panel.checks.clear();
    }
    let reporter = state::Reporter {
        state: ctx.state.clone(),
        agent: name,
    };
    let state_for_done = ctx.state.clone();
    tokio::spawn(async move {
        match name {
            "muse" => checks::muse::run_suite(reporter).await,
            "claude" => checks::claude::run_suite(reporter).await,
            "opencode" => checks::opencode::run_suite(reporter).await,
            _ => checks::codex::run_suite(reporter).await,
        }
        let mut portal = state_for_done.write().await;
        if let Some(panel) = portal.agents.get_mut(name) {
            panel.checks_running = false;
        }
    });
    StatusCode::ACCEPTED.into_response()
}

// ── login endpoints ──────────────────────────────────────────────────

async fn api_muse_device(State(ctx): State<Ctx>) -> StatusCode {
    tokio::spawn(login::muse_device(ctx.state.clone()));
    StatusCode::ACCEPTED
}

#[derive(Deserialize)]
struct KeyBody {
    key: String,
}

async fn api_muse_apikey(State(ctx): State<Ctx>, Json(body): Json<KeyBody>) -> StatusCode {
    tokio::spawn(login::muse_api_key(ctx.state.clone(), body.key));
    StatusCode::ACCEPTED
}

#[derive(Deserialize)]
struct StartBody {
    #[serde(default)]
    mode: String,
}

async fn api_claude_start(State(ctx): State<Ctx>, Json(body): Json<StartBody>) -> StatusCode {
    tokio::spawn(login::claude_start(
        ctx.state.clone(),
        ctx.claude_flow.clone(),
        body.mode,
    ));
    StatusCode::ACCEPTED
}

#[derive(Deserialize)]
struct CodeBody {
    code: String,
}

async fn api_claude_code(State(ctx): State<Ctx>, Json(body): Json<CodeBody>) -> impl IntoResponse {
    match login::claude_submit_code(&ctx.claude_flow, body.code) {
        Ok(()) => StatusCode::ACCEPTED.into_response(),
        Err(e) => (StatusCode::CONFLICT, e).into_response(),
    }
}

// ── auth refresh ─────────────────────────────────────────────────────

async fn refresh_all_auth(state: &Shared) {
    for agent in ["claude", "codex", "muse", "opencode"] {
        refresh_agent_auth(state, agent).await;
    }
}

/// Re-read binary version and credential state for one agent. Called at
/// startup, after every login flow, and from the refresh button.
pub async fn refresh_agent_auth(state: &Shared, agent: &'static str) {
    let binary = version_of(agent).await;
    let (auth, logged_in) = match agent {
        "claude" => match tokio::task::spawn_blocking(claude_codes::auth::auth_status).await {
            Ok(Ok(s)) => (
                format!(
                    "{} ({})",
                    if s.logged_in {
                        "logged in"
                    } else {
                        "no credentials"
                    },
                    s.auth_method.as_deref().unwrap_or("unknown method"),
                ),
                s.logged_in,
            ),
            other => (format!("status unavailable: {other:?}"), false),
        },
        "codex" => match codex_codes::auth_local::auth_status_local() {
            Ok(s) => (
                format!(
                    "{} ({:?})",
                    if s.logged_in {
                        "logged in"
                    } else {
                        "no credentials"
                    },
                    s.auth_mode,
                ),
                s.logged_in,
            ),
            Err(e) => (format!("status unavailable: {e}"), false),
        },
        "muse" => {
            let present = muse_codes::auth::credentials_present();
            (
                if present {
                    "credentials present".to_string()
                } else {
                    "no credentials".to_string()
                },
                present,
            )
        }
        _ => ("unknown agent".to_string(), false),
    };
    let mut portal = state.write().await;
    if let Some(panel) = portal.agents.get_mut(agent) {
        panel.binary = binary;
        panel.auth = Some(auth);
        panel.logged_in = logged_in;
        // A finished login flow's terminal state stays visible; anything
        // else resets so stale progress can't outlive a refresh.
        if !matches!(
            panel.login,
            LoginState::Done { .. } | LoginState::Failed { .. }
        ) {
            if let LoginState::AwaitUser { .. } | LoginState::Waiting | LoginState::Starting =
                panel.login
            {
                // leave in-flight flows alone
            } else {
                panel.login = LoginState::Idle;
            }
        }
    }
}

async fn version_of(agent: &str) -> Option<String> {
    let out = tokio::process::Command::new(agent)
        .arg("--version")
        .output()
        .await
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}
