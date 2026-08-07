//! Shared portal state: per-agent auth view, login-flow progress, and
//! check results. One `RwLock` guards it all — the portal is a local,
//! single-user tool and contention is nil.

use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type Shared = Arc<RwLock<Portal>>;

#[derive(Debug, Default, Serialize)]
pub struct Portal {
    pub agents: BTreeMap<&'static str, AgentPanel>,
}

/// Everything the page (and the querying agent) sees for one CLI.
#[derive(Debug, Default, Serialize)]
pub struct AgentPanel {
    /// CLI binary presence/version, refreshed on demand.
    pub binary: Option<String>,
    /// Human-readable credential state ("logged in (subscription)",
    /// "no credentials", ...). Never contains a secret.
    pub auth: Option<String>,
    pub logged_in: bool,
    /// Current login-flow progress, if one is running.
    pub login: LoginState,
    /// Latest check run, newest state per check name.
    pub checks: Vec<CheckResult>,
    /// True while a suite is executing (page shows a spinner).
    pub checks_running: bool,
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum LoginState {
    #[default]
    Idle,
    /// Flow started; waiting for the CLI to produce a URL/code.
    Starting,
    /// User action needed: open the URL (and for muse, confirm the code).
    AwaitUser {
        url: String,
        /// Muse device flows display a confirmation code; Claude's flow
        /// instead expects a code PASTED BACK via /api/login/claude/code.
        code: Option<String>,
        needs_code_paste: bool,
    },
    /// Code submitted / approval pending.
    Waiting,
    Done {
        detail: String,
    },
    Failed {
        error: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    pub name: &'static str,
    /// What property this check pins, in one sentence.
    pub what: &'static str,
    pub status: CheckStatus,
    /// Pass detail or failure explanation. Never contains secrets.
    pub detail: String,
    /// Milliseconds the check took, once finished.
    pub ms: Option<u128>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Running,
    Pass,
    Fail,
    /// Prerequisite missing (no credentials, binary absent) — distinct
    /// from Fail so a logged-out agent reads as "not checked" rather
    /// than "broken".
    Skipped,
}

impl Portal {
    pub fn new() -> Self {
        let mut agents = BTreeMap::new();
        agents.insert("claude", AgentPanel::default());
        agents.insert("codex", AgentPanel::default());
        agents.insert("muse", AgentPanel::default());
        Self { agents }
    }
}

/// Handle for check functions to report progress into the shared state.
#[derive(Clone)]
pub struct Reporter {
    pub state: Shared,
    pub agent: &'static str,
}

impl Reporter {
    /// Mark a check running; returns its start instant for the ms field.
    pub async fn start(&self, name: &'static str, what: &'static str) -> std::time::Instant {
        let mut portal = self.state.write().await;
        if let Some(panel) = portal.agents.get_mut(self.agent) {
            panel.checks.retain(|c| c.name != name);
            panel.checks.push(CheckResult {
                name,
                what,
                status: CheckStatus::Running,
                detail: String::new(),
                ms: None,
            });
        }
        std::time::Instant::now()
    }

    pub async fn finish(
        &self,
        name: &'static str,
        started: std::time::Instant,
        status: CheckStatus,
        detail: String,
    ) {
        let mut portal = self.state.write().await;
        if let Some(panel) = portal.agents.get_mut(self.agent) {
            if let Some(check) = panel.checks.iter_mut().find(|c| c.name == name) {
                check.status = status;
                check.detail = detail;
                check.ms = Some(started.elapsed().as_millis());
            }
        }
    }
}
