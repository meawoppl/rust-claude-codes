//! Login-flow drivers, one per CLI that supports programmatic login.
//!
//! Secrets never touch argv or logs: the muse API key goes over the SDK's
//! stdin path, and the claude flow's pasted code goes straight into the
//! PTY driver. State snapshots only ever carry URLs, display codes, and
//! outcome summaries.

use crate::state::{LoginState, Shared};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The claude login flow spans two HTTP requests (start → paste code), so
/// the blocking flow thread parks on this channel until the code arrives.
#[derive(Default)]
pub struct ClaudeFlowSlot {
    pub code_tx: Mutex<Option<std::sync::mpsc::Sender<String>>>,
}

async fn set_login(state: &Shared, agent: &'static str, login: LoginState) {
    let mut portal = state.write().await;
    if let Some(panel) = portal.agents.get_mut(agent) {
        panel.login = login;
    }
}

// ── muse ─────────────────────────────────────────────────────────────

/// Browser device-code flow: URL + confirmation code, then poll approval.
pub async fn muse_device(state: Shared) {
    set_login(&state, "muse", LoginState::Starting).await;
    let mut flow = match muse_codes::auth::DeviceLoginFlow::start().await {
        Ok(f) => f,
        Err(e) => {
            set_login(
                &state,
                "muse",
                LoginState::Failed {
                    error: e.to_string(),
                },
            )
            .await;
            return;
        }
    };
    let dc = match flow.device_code(Duration::from_secs(30)).await {
        Ok(dc) => dc,
        Err(e) => {
            set_login(
                &state,
                "muse",
                LoginState::Failed {
                    error: e.to_string(),
                },
            )
            .await;
            return;
        }
    };
    set_login(
        &state,
        "muse",
        LoginState::AwaitUser {
            url: dc.verification_url.clone(),
            code: Some(dc.code.clone()),
            needs_code_paste: false,
        },
    )
    .await;
    match flow.wait_approved(Duration::from_secs(600)).await {
        Ok(()) => {
            set_login(
                &state,
                "muse",
                LoginState::Done {
                    detail: "device flow approved; credentials stored".into(),
                },
            )
            .await;
        }
        Err(e) => {
            set_login(
                &state,
                "muse",
                LoginState::Failed {
                    error: e.to_string(),
                },
            )
            .await;
        }
    }
    crate::refresh_agent_auth(&state, "muse").await;
}

/// API-key path (key travels via the SDK's stdin mechanism, never argv).
pub async fn muse_api_key(state: Shared, key: String) {
    set_login(&state, "muse", LoginState::Waiting).await;
    match muse_codes::auth::auth_set(&key, None).await {
        Ok(()) => {
            set_login(
                &state,
                "muse",
                LoginState::Done {
                    detail: "API key stored via `muse auth set`".into(),
                },
            )
            .await;
        }
        Err(e) => {
            set_login(
                &state,
                "muse",
                LoginState::Failed {
                    error: e.to_string(),
                },
            )
            .await;
        }
    }
    crate::refresh_agent_auth(&state, "muse").await;
}

// ── claude ───────────────────────────────────────────────────────────

/// Start the claude visit-URL/paste-code flow. The PTY driver is
/// synchronous, so the whole flow lives on one blocking thread that parks
/// waiting for the pasted code.
pub async fn claude_start(state: Shared, slot: Arc<ClaudeFlowSlot>, mode: String) {
    use claude_codes::auth::{LoginFlow, LoginMode};
    let mode = match mode.as_str() {
        "console" => LoginMode::Console,
        "setup-token" => LoginMode::SetupToken,
        _ => LoginMode::ClaudeAi,
    };
    set_login(&state, "claude", LoginState::Starting).await;

    let (tx, rx) = std::sync::mpsc::channel::<String>();
    if let Ok(mut guard) = slot.code_tx.lock() {
        *guard = Some(tx);
    }

    let state_for_thread = state.clone();
    let outcome = tokio::task::spawn_blocking(move || {
        let mut flow = LoginFlow::start(mode).map_err(|e| e.to_string())?;
        let url = flow
            .auth_url(Duration::from_secs(60))
            .map_err(|e| e.to_string())?;
        // Publish the URL from inside the blocking thread so the page can
        // show it while we park on the code.
        {
            let mut portal = state_for_thread.blocking_write();
            if let Some(panel) = portal.agents.get_mut("claude") {
                panel.login = LoginState::AwaitUser {
                    url,
                    code: None,
                    needs_code_paste: true,
                };
            }
        }
        let code = rx
            .recv_timeout(Duration::from_secs(600))
            .map_err(|_| "timed out waiting for the pasted code".to_string())?;
        {
            let mut portal = state_for_thread.blocking_write();
            if let Some(panel) = portal.agents.get_mut("claude") {
                panel.login = LoginState::Waiting;
            }
        }
        let outcome = flow
            .submit_code_and_wait(&code, Duration::from_secs(120))
            .map_err(|e| e.to_string())?;
        // Summarize WITHOUT the token value: setup-token outcomes carry a
        // secret that must not reach state/logs; its presence is the news.
        Ok::<String, String>(format!(
            "login ok{}",
            if outcome.token.is_some() {
                " (token minted — retrieve via SDK)"
            } else {
                ""
            }
        ))
    })
    .await;

    match outcome {
        Ok(Ok(detail)) => set_login(&state, "claude", LoginState::Done { detail }).await,
        Ok(Err(e)) => set_login(&state, "claude", LoginState::Failed { error: e }).await,
        Err(e) => {
            set_login(
                &state,
                "claude",
                LoginState::Failed {
                    error: e.to_string(),
                },
            )
            .await
        }
    }
    if let Ok(mut guard) = slot.code_tx.lock() {
        *guard = None;
    }
    crate::refresh_agent_auth(&state, "claude").await;
}

/// Deliver the pasted code to the parked flow thread.
pub fn claude_submit_code(slot: &ClaudeFlowSlot, code: String) -> Result<(), &'static str> {
    let guard = slot.code_tx.lock().map_err(|_| "flow slot poisoned")?;
    match guard.as_ref() {
        Some(tx) => tx.send(code).map_err(|_| "flow thread already exited"),
        None => Err("no claude login flow is waiting for a code"),
    }
}
