//! Builder for spawning the `hermes acp` process.
//!
//! The ACP adapter is an installed console script (`hermes acp`, also
//! exposed as `hermes-acp`); stdout carries JSON-RPC frames and all logging
//! goes to stderr. Environment is loaded by the adapter itself from
//! `~/.hermes/.env`, so no credential plumbing is needed here.

use crate::error::{Error, Result};
use std::path::PathBuf;
use std::process::Stdio;

/// Builder for the `hermes acp` child process.
#[derive(Debug, Clone)]
pub struct HermesAcpBuilder {
    binary: String,
    args: Vec<String>,
    working_directory: Option<PathBuf>,
    envs: Vec<(String, String)>,
}

impl Default for HermesAcpBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl HermesAcpBuilder {
    pub fn new() -> Self {
        Self {
            binary: "hermes".to_string(),
            args: vec!["acp".to_string()],
            working_directory: None,
            envs: Vec::new(),
        }
    }

    /// Use a specific binary instead of `hermes` from `PATH`. When pointing
    /// at the standalone entry point (`hermes-acp`), also call
    /// [`args`](Self::args) with an empty list to drop the `acp` subcommand.
    pub fn binary(mut self, path: impl Into<String>) -> Self {
        self.binary = path.into();
        self
    }

    /// Replace the argument list (default: `["acp"]`).
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args = args.into_iter().map(Into::into).collect();
        self
    }

    /// Set the child's working directory.
    pub fn working_directory(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_directory = Some(dir.into());
        self
    }

    /// Add an environment variable for the child.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.envs.push((key.into(), value.into()));
        self
    }

    /// Resolve the binary and build the tokio command with piped stdio.
    pub fn build_command(&self) -> Result<tokio::process::Command> {
        let program = which::which(&self.binary).map_err(|_| Error::BinaryNotFound {
            name: self.binary.clone(),
        })?;
        let mut cmd = tokio::process::Command::new(program);
        cmd.args(&self.args)
            .stdin(Stdio::piped())
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

    /// Spawn the ACP adapter process.
    pub async fn spawn(&self) -> Result<tokio::process::Child> {
        Ok(self.build_command()?.spawn()?)
    }
}
