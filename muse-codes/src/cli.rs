//! Builder for spawning headless `muse exec --json` runs.

use crate::error::{Error, Result};
use std::path::PathBuf;
use std::process::Stdio;

/// Provider mode for a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    /// The Meta provider (default; requires credentials — `muse login`,
    /// `META_API_KEY`, or `~/.config/muse/auth.json`).
    Meta,
    /// Credential-free echo provider — exercises the full event stream
    /// without model calls. What this crate's committed captures use.
    Echo,
}

impl Provider {
    fn as_str(self) -> &'static str {
        match self {
            Provider::Meta => "meta",
            Provider::Echo => "echo",
        }
    }
}

/// Builder for one `muse exec --json` invocation.
#[derive(Debug, Clone)]
pub struct MuseExecBuilder {
    binary: String,
    prompt: String,
    provider: Option<Provider>,
    preset: Option<String>,
    model: Option<String>,
    reasoning_effort: Option<String>,
    base_url: Option<String>,
    working_directory: Option<PathBuf>,
    envs: Vec<(String, String)>,
}

impl MuseExecBuilder {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            binary: "muse".to_string(),
            prompt: prompt.into(),
            provider: None,
            preset: None,
            model: None,
            reasoning_effort: None,
            base_url: None,
            working_directory: None,
            envs: Vec::new(),
        }
    }

    /// Use a specific binary instead of `muse` from `PATH`.
    pub fn binary(mut self, path: impl Into<String>) -> Self {
        self.binary = path.into();
        self
    }

    pub fn provider(mut self, provider: Provider) -> Self {
        self.provider = Some(provider);
        self
    }

    /// Built-in preset (`native-basic`, `miniswe`).
    pub fn preset(mut self, preset: impl Into<String>) -> Self {
        self.preset = Some(preset.into());
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Meta reasoning effort (`none|minimal|low|medium|high|xhigh|ultra`).
    /// Not supported with [`Provider::Echo`].
    pub fn reasoning_effort(mut self, effort: impl Into<String>) -> Self {
        self.reasoning_effort = Some(effort.into());
        self
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    pub fn working_directory(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_directory = Some(dir.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.envs.push((key.into(), value.into()));
        self
    }

    /// Resolve the binary and assemble the command with piped stdio.
    pub fn build_command(&self) -> Result<tokio::process::Command> {
        let program = which::which(&self.binary).map_err(|_| Error::BinaryNotFound {
            name: self.binary.clone(),
        })?;
        let mut cmd = tokio::process::Command::new(program);
        cmd.arg("exec").arg("--json");
        if let Some(p) = self.provider {
            cmd.args(["--provider", p.as_str()]);
        }
        if let Some(p) = &self.preset {
            cmd.args(["--preset", p]);
        }
        if let Some(m) = &self.model {
            cmd.args(["--model", m]);
        }
        if let Some(e) = &self.reasoning_effort {
            cmd.args(["--reasoning-effort", e]);
        }
        if let Some(u) = &self.base_url {
            cmd.args(["--base-url", u]);
        }
        cmd.arg(&self.prompt)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if let Some(dir) = &self.working_directory {
            cmd.current_dir(dir);
        }
        for (k, v) in &self.envs {
            cmd.env(k, v);
        }
        Ok(cmd)
    }

    /// Spawn the run.
    pub async fn spawn(&self) -> Result<tokio::process::Child> {
        Ok(self.build_command()?.spawn()?)
    }
}
