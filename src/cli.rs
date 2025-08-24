//! Builder pattern for Claude CLI invocation
//!
//! This builder is configured to only support JSON streaming mode,
//! which provides the most control and visibility into Claude's responses.

use crate::error::{Error, Result};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::{Child, Command};
use tracing::debug;

/// Permission mode for Claude CLI
#[derive(Debug, Clone, Copy)]
pub enum PermissionMode {
    AcceptEdits,
    BypassPermissions,
    Default,
    Plan,
}

impl PermissionMode {
    fn as_str(&self) -> &'static str {
        match self {
            PermissionMode::AcceptEdits => "acceptEdits",
            PermissionMode::BypassPermissions => "bypassPermissions",
            PermissionMode::Default => "default",
            PermissionMode::Plan => "plan",
        }
    }
}

/// Builder for creating Claude CLI commands in JSON streaming mode
///
/// This builder automatically configures Claude to use:
/// - `--print` mode for non-interactive operation
/// - `--output-format stream-json` for streaming JSON responses
/// - `--input-format stream-json` for JSON input
/// - `--replay-user-messages` to echo back user messages
#[derive(Debug, Clone)]
pub struct ClaudeCliBuilder {
    command: PathBuf,
    prompt: Option<String>,
    debug: Option<String>,
    verbose: bool,
    dangerously_skip_permissions: bool,
    allowed_tools: Vec<String>,
    disallowed_tools: Vec<String>,
    mcp_config: Vec<String>,
    append_system_prompt: Option<String>,
    permission_mode: Option<PermissionMode>,
    continue_conversation: bool,
    resume: Option<String>,
    model: Option<String>,
    fallback_model: Option<String>,
    settings: Option<String>,
    add_dir: Vec<PathBuf>,
    ide: bool,
    strict_mcp_config: bool,
    session_id: Option<String>,
}

impl Default for ClaudeCliBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaudeCliBuilder {
    /// Create a new Claude CLI builder with JSON streaming mode pre-configured
    pub fn new() -> Self {
        Self {
            command: PathBuf::from("claude"),
            prompt: None,
            debug: None,
            verbose: false,
            dangerously_skip_permissions: false,
            allowed_tools: Vec::new(),
            disallowed_tools: Vec::new(),
            mcp_config: Vec::new(),
            append_system_prompt: None,
            permission_mode: None,
            continue_conversation: false,
            resume: None,
            model: None,
            fallback_model: None,
            settings: None,
            add_dir: Vec::new(),
            ide: false,
            strict_mcp_config: false,
            session_id: None,
        }
    }

    /// Set custom path to Claude binary
    pub fn command<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.command = path.into();
        self
    }

    /// Set the prompt for Claude
    pub fn prompt<S: Into<String>>(mut self, prompt: S) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Enable debug mode with optional filter
    pub fn debug<S: Into<String>>(mut self, filter: Option<S>) -> Self {
        self.debug = filter.map(|s| s.into());
        self
    }

    /// Enable verbose mode
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Skip all permission checks (dangerous!)
    pub fn dangerously_skip_permissions(mut self, skip: bool) -> Self {
        self.dangerously_skip_permissions = skip;
        self
    }

    /// Add allowed tools
    pub fn allowed_tools<I, S>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allowed_tools
            .extend(tools.into_iter().map(|s| s.into()));
        self
    }

    /// Add disallowed tools
    pub fn disallowed_tools<I, S>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.disallowed_tools
            .extend(tools.into_iter().map(|s| s.into()));
        self
    }

    /// Add MCP configuration
    pub fn mcp_config<I, S>(mut self, configs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.mcp_config
            .extend(configs.into_iter().map(|s| s.into()));
        self
    }

    /// Append a system prompt
    pub fn append_system_prompt<S: Into<String>>(mut self, prompt: S) -> Self {
        self.append_system_prompt = Some(prompt.into());
        self
    }

    /// Set permission mode
    pub fn permission_mode(mut self, mode: PermissionMode) -> Self {
        self.permission_mode = Some(mode);
        self
    }

    /// Continue the most recent conversation
    pub fn continue_conversation(mut self, continue_conv: bool) -> Self {
        self.continue_conversation = continue_conv;
        self
    }

    /// Resume a specific conversation
    pub fn resume<S: Into<String>>(mut self, session_id: Option<S>) -> Self {
        self.resume = session_id.map(|s| s.into());
        self
    }

    /// Set the model to use
    pub fn model<S: Into<String>>(mut self, model: S) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set fallback model for overload situations
    pub fn fallback_model<S: Into<String>>(mut self, model: S) -> Self {
        self.fallback_model = Some(model.into());
        self
    }

    /// Load settings from file or JSON
    pub fn settings<S: Into<String>>(mut self, settings: S) -> Self {
        self.settings = Some(settings.into());
        self
    }

    /// Add directories for tool access
    pub fn add_directories<I, P>(mut self, dirs: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        self.add_dir.extend(dirs.into_iter().map(|p| p.into()));
        self
    }

    /// Automatically connect to IDE
    pub fn ide(mut self, ide: bool) -> Self {
        self.ide = ide;
        self
    }

    /// Use only MCP servers from config
    pub fn strict_mcp_config(mut self, strict: bool) -> Self {
        self.strict_mcp_config = strict;
        self
    }

    /// Set a specific session ID
    pub fn session_id<S: Into<String>>(mut self, id: S) -> Self {
        self.session_id = Some(id.into());
        self
    }

    /// Build the command arguments (always includes JSON streaming flags)
    fn build_args(&self) -> Vec<String> {
        // Always add JSON streaming mode flags
        // Note: --print with stream-json requires --verbose
        let mut args = vec![
            "--print".to_string(),
            "--verbose".to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--input-format".to_string(),
            "stream-json".to_string(),
        ];

        if let Some(ref debug) = self.debug {
            args.push("--debug".to_string());
            if !debug.is_empty() {
                args.push(debug.clone());
            }
        }

        if self.dangerously_skip_permissions {
            args.push("--dangerously-skip-permissions".to_string());
        }

        if !self.allowed_tools.is_empty() {
            args.push("--allowed-tools".to_string());
            args.extend(self.allowed_tools.clone());
        }

        if !self.disallowed_tools.is_empty() {
            args.push("--disallowed-tools".to_string());
            args.extend(self.disallowed_tools.clone());
        }

        if !self.mcp_config.is_empty() {
            args.push("--mcp-config".to_string());
            args.extend(self.mcp_config.clone());
        }

        if let Some(ref prompt) = self.append_system_prompt {
            args.push("--append-system-prompt".to_string());
            args.push(prompt.clone());
        }

        if let Some(ref mode) = self.permission_mode {
            args.push("--permission-mode".to_string());
            args.push(mode.as_str().to_string());
        }

        if self.continue_conversation {
            args.push("--continue".to_string());
        }

        if let Some(ref session) = self.resume {
            args.push("--resume".to_string());
            args.push(session.clone());
        }

        if let Some(ref model) = self.model {
            args.push("--model".to_string());
            args.push(model.clone());
        }

        if let Some(ref model) = self.fallback_model {
            args.push("--fallback-model".to_string());
            args.push(model.clone());
        }

        if let Some(ref settings) = self.settings {
            args.push("--settings".to_string());
            args.push(settings.clone());
        }

        if !self.add_dir.is_empty() {
            args.push("--add-dir".to_string());
            for dir in &self.add_dir {
                args.push(dir.to_string_lossy().to_string());
            }
        }

        if self.ide {
            args.push("--ide".to_string());
        }

        if self.strict_mcp_config {
            args.push("--strict-mcp-config".to_string());
        }

        if let Some(ref id) = self.session_id {
            args.push("--session-id".to_string());
            args.push(id.clone());
        }

        // Add prompt as the last argument if provided
        if let Some(ref prompt) = self.prompt {
            args.push(prompt.clone());
        }

        args
    }

    /// Spawn the Claude process
    pub async fn spawn(self) -> Result<Child> {
        let args = self.build_args();

        // Log the full command being executed
        debug!(
            "[CLI] Executing command: {} {}",
            self.command.display(),
            args.join(" ")
        );
        eprintln!("Executing: {} {}", self.command.display(), args.join(" "));

        let child = Command::new(&self.command)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(Error::Io)?;

        Ok(child)
    }

    /// Build a Command without spawning (for testing or manual execution)
    pub fn build_command(self) -> Command {
        let args = self.build_args();
        let mut cmd = Command::new(&self.command);
        cmd.args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd
    }

    /// Spawn the Claude process using synchronous std::process
    pub fn spawn_sync(self) -> std::io::Result<std::process::Child> {
        let args = self.build_args();

        // Log the full command being executed
        debug!(
            "[CLI] Executing sync command: {} {}",
            self.command.display(),
            args.join(" ")
        );
        eprintln!("Executing: {} {}", self.command.display(), args.join(" "));

        std::process::Command::new(&self.command)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_flags_always_present() {
        let builder = ClaudeCliBuilder::new();
        let args = builder.build_args();

        // Verify all streaming flags are present by default
        assert!(args.contains(&"--print".to_string()));
        assert!(args.contains(&"--verbose".to_string())); // Required for --print with stream-json
        assert!(args.contains(&"--output-format".to_string()));
        assert!(args.contains(&"stream-json".to_string()));
        assert!(args.contains(&"--input-format".to_string()));
    }

    #[test]
    fn test_with_prompt() {
        let builder = ClaudeCliBuilder::new().prompt("Hello, Claude!");
        let args = builder.build_args();

        assert_eq!(args.last().unwrap(), "Hello, Claude!");
    }

    #[test]
    fn test_with_model() {
        let builder = ClaudeCliBuilder::new()
            .model("sonnet")
            .fallback_model("opus");
        let args = builder.build_args();

        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"sonnet".to_string()));
        assert!(args.contains(&"--fallback-model".to_string()));
        assert!(args.contains(&"opus".to_string()));
    }

    #[test]
    fn test_with_debug() {
        let builder = ClaudeCliBuilder::new().debug(Some("api"));
        let args = builder.build_args();

        assert!(args.contains(&"--debug".to_string()));
        assert!(args.contains(&"api".to_string()));
    }
}
