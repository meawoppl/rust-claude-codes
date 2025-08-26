//! Builder pattern for configuring and launching the Claude CLI process.
//!
//! This module provides [`ClaudeCliBuilder`] for constructing Claude CLI commands
//! with the correct flags for JSON streaming mode. The builder automatically configures:
//!
//! - JSON streaming input/output formats
//! - Non-interactive print mode
//! - Verbose output for proper streaming
//! - OAuth token and API key environment variables for authentication
//!
//! # Example
//!
//! ```no_run
//! use claude_codes::ClaudeCliBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Build and spawn an async Claude process
//! let child = ClaudeCliBuilder::new()
//!     .model("sonnet")
//!     .session_id("my-session")
//!     .spawn().await?;
//!     
//! // With OAuth authentication
//! let child = ClaudeCliBuilder::new()
//!     .model("opus")
//!     .oauth_token("sk-ant-oat-123456789")
//!     .spawn_sync()?;
//!
//! // Or with API key authentication
//! let child = ClaudeCliBuilder::new()
//!     .api_key("sk-ant-api-987654321")
//!     .spawn_sync()?;
//! # Ok(())
//! # }
//! ```

use crate::error::{Error, Result};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::{Child, Command};
use tracing::debug;
use uuid::Uuid;

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
    oauth_token: Option<String>,
    api_key: Option<String>,
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
            oauth_token: None,
            api_key: None,
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

    /// Set OAuth token for authentication (must start with "sk-ant-oat")
    pub fn oauth_token<S: Into<String>>(mut self, token: S) -> Self {
        let token_str = token.into();
        if !token_str.starts_with("sk-ant-oat") {
            eprintln!("Warning: OAuth token should start with 'sk-ant-oat'");
        }
        self.oauth_token = Some(token_str);
        self
    }

    /// Set API key for authentication (must start with "sk-ant-api")
    pub fn api_key<S: Into<String>>(mut self, key: S) -> Self {
        let key_str = key.into();
        if !key_str.starts_with("sk-ant-api") {
            eprintln!("Warning: API key should start with 'sk-ant-api'");
        }
        self.api_key = Some(key_str);
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

        // Always provide a session ID - use provided one or generate a UUID4
        args.push("--session-id".to_string());
        if let Some(ref id) = self.session_id {
            args.push(id.clone());
        } else {
            // Generate a UUID4 if no session ID was provided
            let uuid = Uuid::new_v4();
            debug!("[CLI] Generated session UUID: {}", uuid);
            args.push(uuid.to_string());
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

        let mut cmd = Command::new(&self.command);
        cmd.args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set OAuth token environment variable if provided
        if let Some(ref token) = self.oauth_token {
            cmd.env("CLAUDE_CODE_OAUTH_TOKEN", token);
            debug!("[CLI] Setting CLAUDE_CODE_OAUTH_TOKEN environment variable");
        }

        // Set API key environment variable if provided
        if let Some(ref key) = self.api_key {
            cmd.env("ANTHROPIC_API_KEY", key);
            debug!("[CLI] Setting ANTHROPIC_API_KEY environment variable");
        }

        let child = cmd.spawn().map_err(Error::Io)?;

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

        // Set OAuth token environment variable if provided
        if let Some(ref token) = self.oauth_token {
            cmd.env("CLAUDE_CODE_OAUTH_TOKEN", token);
        }

        // Set API key environment variable if provided
        if let Some(ref key) = self.api_key {
            cmd.env("ANTHROPIC_API_KEY", key);
        }

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

        let mut cmd = std::process::Command::new(&self.command);
        cmd.args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set OAuth token environment variable if provided
        if let Some(ref token) = self.oauth_token {
            cmd.env("CLAUDE_CODE_OAUTH_TOKEN", token);
            debug!("[CLI] Setting CLAUDE_CODE_OAUTH_TOKEN environment variable");
        }

        // Set API key environment variable if provided
        if let Some(ref key) = self.api_key {
            cmd.env("ANTHROPIC_API_KEY", key);
            debug!("[CLI] Setting ANTHROPIC_API_KEY environment variable");
        }

        cmd.spawn()
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

    #[test]
    fn test_with_oauth_token() {
        let valid_token = "sk-ant-oat-123456789";
        let builder = ClaudeCliBuilder::new().oauth_token(valid_token);

        // OAuth token is set as env var, not in args
        let args = builder.clone().build_args();
        assert!(!args.contains(&valid_token.to_string()));

        // Verify it's stored in the builder
        assert_eq!(builder.oauth_token, Some(valid_token.to_string()));
    }

    #[test]
    fn test_oauth_token_validation() {
        // Test with invalid prefix (should print warning but still accept)
        let invalid_token = "invalid-token-123";
        let builder = ClaudeCliBuilder::new().oauth_token(invalid_token);
        assert_eq!(builder.oauth_token, Some(invalid_token.to_string()));
    }

    #[test]
    fn test_with_api_key() {
        let valid_key = "sk-ant-api-987654321";
        let builder = ClaudeCliBuilder::new().api_key(valid_key);

        // API key is set as env var, not in args
        let args = builder.clone().build_args();
        assert!(!args.contains(&valid_key.to_string()));

        // Verify it's stored in the builder
        assert_eq!(builder.api_key, Some(valid_key.to_string()));
    }

    #[test]
    fn test_api_key_validation() {
        // Test with invalid prefix (should print warning but still accept)
        let invalid_key = "invalid-api-key";
        let builder = ClaudeCliBuilder::new().api_key(invalid_key);
        assert_eq!(builder.api_key, Some(invalid_key.to_string()));
    }

    #[test]
    fn test_both_auth_methods() {
        let oauth = "sk-ant-oat-123";
        let api_key = "sk-ant-api-456";
        let builder = ClaudeCliBuilder::new().oauth_token(oauth).api_key(api_key);

        assert_eq!(builder.oauth_token, Some(oauth.to_string()));
        assert_eq!(builder.api_key, Some(api_key.to_string()));
    }
}
