//! Builder pattern for configuring and launching the Codex CLI process.
//!
//! This module provides [`CodexCliBuilder`] for constructing Codex CLI commands
//! with the correct flags for JSON streaming mode. The builder produces
//! `codex exec --json [flags...] -` commands that read prompts from stdin.

use crate::io::options::SandboxMode;
use log::debug;
use std::path::PathBuf;
use std::process::Stdio;

/// Builder for creating Codex CLI commands in JSON mode.
///
/// Produces commands of the form: `codex exec --json [flags...] -`
///
/// The trailing `-` tells Codex to read the prompt from stdin.
#[derive(Debug, Clone)]
pub struct CodexCliBuilder {
    command: PathBuf,
    model: Option<String>,
    sandbox: Option<SandboxMode>,
    working_directory: Option<PathBuf>,
    full_auto: bool,
    dangerously_bypass: bool,
    skip_git_repo_check: bool,
    ephemeral: bool,
    images: Vec<PathBuf>,
    add_dirs: Vec<PathBuf>,
    config_overrides: Vec<String>,
    output_schema: Option<String>,
    profile: Option<String>,
    enable_features: Vec<String>,
    disable_features: Vec<String>,
}

impl Default for CodexCliBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexCliBuilder {
    /// Create a new Codex CLI builder with default settings.
    pub fn new() -> Self {
        Self {
            command: PathBuf::from("codex"),
            model: None,
            sandbox: None,
            working_directory: None,
            full_auto: false,
            dangerously_bypass: false,
            skip_git_repo_check: false,
            ephemeral: false,
            images: Vec::new(),
            add_dirs: Vec::new(),
            config_overrides: Vec::new(),
            output_schema: None,
            profile: None,
            enable_features: Vec::new(),
            disable_features: Vec::new(),
        }
    }

    /// Set custom path to the codex binary.
    pub fn command<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.command = path.into();
        self
    }

    /// Set the model to use.
    pub fn model<S: Into<String>>(mut self, model: S) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the sandbox mode.
    pub fn sandbox(mut self, mode: SandboxMode) -> Self {
        self.sandbox = Some(mode);
        self
    }

    /// Set the working directory.
    pub fn working_directory<P: Into<PathBuf>>(mut self, dir: P) -> Self {
        self.working_directory = Some(dir.into());
        self
    }

    /// Enable full-auto mode (skip all approvals).
    pub fn full_auto(mut self, enabled: bool) -> Self {
        self.full_auto = enabled;
        self
    }

    /// Dangerously bypass all safety checks.
    pub fn dangerously_bypass(mut self, enabled: bool) -> Self {
        self.dangerously_bypass = enabled;
        self
    }

    /// Skip the git repository check.
    pub fn skip_git_repo_check(mut self, enabled: bool) -> Self {
        self.skip_git_repo_check = enabled;
        self
    }

    /// Run in ephemeral mode (no persistent state).
    pub fn ephemeral(mut self, enabled: bool) -> Self {
        self.ephemeral = enabled;
        self
    }

    /// Add image files to include with the prompt.
    pub fn images<I, P>(mut self, paths: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        self.images.extend(paths.into_iter().map(|p| p.into()));
        self
    }

    /// Add directories the agent can access.
    pub fn add_dirs<I, P>(mut self, dirs: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        self.add_dirs.extend(dirs.into_iter().map(|p| p.into()));
        self
    }

    /// Add configuration overrides (key=value pairs).
    pub fn config_overrides<I, S>(mut self, overrides: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.config_overrides
            .extend(overrides.into_iter().map(|s| s.into()));
        self
    }

    /// Set a JSON schema for structured output.
    pub fn output_schema<S: Into<String>>(mut self, schema: S) -> Self {
        self.output_schema = Some(schema.into());
        self
    }

    /// Set the configuration profile.
    pub fn profile<S: Into<String>>(mut self, profile: S) -> Self {
        self.profile = Some(profile.into());
        self
    }

    /// Enable specific features.
    pub fn enable_features<I, S>(mut self, features: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.enable_features
            .extend(features.into_iter().map(|s| s.into()));
        self
    }

    /// Disable specific features.
    pub fn disable_features<I, S>(mut self, features: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.disable_features
            .extend(features.into_iter().map(|s| s.into()));
        self
    }

    /// Build the command arguments.
    ///
    /// Always produces: `exec --json [flags...] -`
    fn build_args(&self) -> Vec<String> {
        let mut args = vec!["exec".to_string(), "--json".to_string()];

        if let Some(ref model) = self.model {
            args.push("--model".to_string());
            args.push(model.clone());
        }

        if let Some(ref sandbox) = self.sandbox {
            args.push("--sandbox".to_string());
            args.push(sandbox.as_cli_str().to_string());
        }

        if self.full_auto {
            args.push("--full-auto".to_string());
        }

        if self.dangerously_bypass {
            args.push("--dangerously-auto-approve".to_string());
        }

        if self.skip_git_repo_check {
            args.push("--skip-git-repo-check".to_string());
        }

        if self.ephemeral {
            args.push("--ephemeral".to_string());
        }

        for image in &self.images {
            args.push("--image".to_string());
            args.push(image.to_string_lossy().to_string());
        }

        for dir in &self.add_dirs {
            args.push("--add-dir".to_string());
            args.push(dir.to_string_lossy().to_string());
        }

        for config in &self.config_overrides {
            args.push("--config".to_string());
            args.push(config.clone());
        }

        if let Some(ref schema) = self.output_schema {
            args.push("--output-schema".to_string());
            args.push(schema.clone());
        }

        if let Some(ref profile) = self.profile {
            args.push("--profile".to_string());
            args.push(profile.clone());
        }

        for feature in &self.enable_features {
            args.push("--enable-feature".to_string());
            args.push(feature.clone());
        }

        for feature in &self.disable_features {
            args.push("--disable-feature".to_string());
            args.push(feature.clone());
        }

        // Trailing `-` tells codex to read prompt from stdin
        args.push("-".to_string());

        args
    }

    /// Spawn the Codex process asynchronously.
    #[cfg(feature = "async-client")]
    pub async fn spawn(self) -> crate::error::Result<tokio::process::Child> {
        let args = self.build_args();

        debug!(
            "[CLI] Executing async command: {} {}",
            self.command.display(),
            args.join(" ")
        );

        let mut cmd = tokio::process::Command::new(&self.command);
        cmd.args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(ref dir) = self.working_directory {
            cmd.current_dir(dir);
        }

        cmd.spawn().map_err(crate::error::Error::Io)
    }

    /// Spawn the Codex process synchronously.
    pub fn spawn_sync(self) -> std::io::Result<std::process::Child> {
        let args = self.build_args();

        debug!(
            "[CLI] Executing sync command: {} {}",
            self.command.display(),
            args.join(" ")
        );

        let mut cmd = std::process::Command::new(&self.command);
        cmd.args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(ref dir) = self.working_directory {
            cmd.current_dir(dir);
        }

        cmd.spawn()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_args() {
        let builder = CodexCliBuilder::new();
        let args = builder.build_args();

        assert_eq!(args[0], "exec");
        assert_eq!(args[1], "--json");
        assert_eq!(args.last().unwrap(), "-");
    }

    #[test]
    fn test_with_model() {
        let builder = CodexCliBuilder::new().model("o4-mini");
        let args = builder.build_args();

        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"o4-mini".to_string()));
    }

    #[test]
    fn test_with_sandbox() {
        let builder = CodexCliBuilder::new().sandbox(SandboxMode::ReadOnly);
        let args = builder.build_args();

        assert!(args.contains(&"--sandbox".to_string()));
        assert!(args.contains(&"read-only".to_string()));
    }

    #[test]
    fn test_full_auto() {
        let builder = CodexCliBuilder::new().full_auto(true);
        let args = builder.build_args();

        assert!(args.contains(&"--full-auto".to_string()));
    }

    #[test]
    fn test_multiple_images() {
        let builder = CodexCliBuilder::new().images(vec!["a.png", "b.png"]);
        let args = builder.build_args();

        let image_count = args.iter().filter(|a| *a == "--image").count();
        assert_eq!(image_count, 2);
    }

    #[test]
    fn test_trailing_dash_always_last() {
        let builder = CodexCliBuilder::new()
            .model("o4-mini")
            .full_auto(true)
            .ephemeral(true);
        let args = builder.build_args();

        assert_eq!(args.last().unwrap(), "-");
    }
}
