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

#[cfg(feature = "async-client")]
use crate::error::{Error, Result};
use log::debug;
use std::path::PathBuf;
use std::process::Stdio;
use uuid::Uuid;

/// Permission mode for Claude CLI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionMode {
    AcceptEdits,
    BypassPermissions,
    Default,
    Delegate,
    DontAsk,
    Plan,
}

impl PermissionMode {
    /// Get the CLI string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            PermissionMode::AcceptEdits => "acceptEdits",
            PermissionMode::BypassPermissions => "bypassPermissions",
            PermissionMode::Default => "default",
            PermissionMode::Delegate => "delegate",
            PermissionMode::DontAsk => "dontAsk",
            PermissionMode::Plan => "plan",
        }
    }
}

/// Input format for Claude CLI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    Text,
    StreamJson,
}

impl InputFormat {
    /// Get the CLI string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            InputFormat::Text => "text",
            InputFormat::StreamJson => "stream-json",
        }
    }
}

/// Output format for Claude CLI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
    StreamJson,
}

impl OutputFormat {
    /// Get the CLI string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputFormat::Text => "text",
            OutputFormat::Json => "json",
            OutputFormat::StreamJson => "stream-json",
        }
    }
}

/// Comprehensive enum of all Claude CLI flags.
///
/// This enum represents every flag available in the Claude CLI (`claude --help`).
/// Each variant carries the appropriate data type for its flag value.
///
/// Use `as_flag()` to get the CLI flag string (e.g., `"--model"`),
/// or `to_args()` to get the complete flag + value as CLI arguments.
///
/// # Example
/// ```
/// use claude_codes::CliFlag;
///
/// let flag = CliFlag::Model("sonnet".to_string());
/// assert_eq!(flag.as_flag(), "--model");
/// assert_eq!(flag.to_args(), vec!["--model", "sonnet"]);
/// ```
#[derive(Debug, Clone)]
pub enum CliFlag {
    /// Additional directories to allow tool access to
    AddDir(Vec<PathBuf>),
    /// Agent for the current session
    Agent(String),
    /// JSON object defining custom agents
    Agents(String),
    /// Enable bypassing all permission checks as an option
    AllowDangerouslySkipPermissions,
    /// Tool names to allow (e.g. "Bash(git:*) Edit")
    AllowedTools(Vec<String>),
    /// Append to the default system prompt
    AppendSystemPrompt(String),
    /// Beta headers for API requests (API key users only)
    Betas(Vec<String>),
    /// Enable Claude in Chrome integration
    Chrome,
    /// Continue the most recent conversation
    Continue,
    /// Bypass all permission checks
    DangerouslySkipPermissions,
    /// Enable debug mode with optional category filter
    Debug(Option<String>),
    /// Write debug logs to a specific file path
    DebugFile(PathBuf),
    /// Disable all skills/slash commands
    DisableSlashCommands,
    /// Tool names to deny (e.g. "Bash(git:*) Edit")
    DisallowedTools(Vec<String>),
    /// Automatic fallback model when default is overloaded
    FallbackModel(String),
    /// File resources to download at startup (format: file_id:relative_path)
    File(Vec<String>),
    /// Create a new session ID when resuming instead of reusing original
    ForkSession,
    /// Resume a session linked to a PR
    FromPr(Option<String>),
    /// Include partial message chunks as they arrive
    IncludePartialMessages,
    /// Input format (text or stream-json)
    InputFormat(InputFormat),
    /// JSON Schema for structured output validation
    JsonSchema(String),
    /// Maximum dollar amount for API calls
    MaxBudgetUsd(f64),
    /// Load MCP servers from JSON files or strings
    McpConfig(Vec<String>),
    /// Enable MCP debug mode (deprecated, use Debug instead)
    McpDebug,
    /// Model for the current session
    Model(String),
    /// Disable Claude in Chrome integration
    NoChrome,
    /// Disable session persistence
    NoSessionPersistence,
    /// Output format (text, json, or stream-json)
    OutputFormat(OutputFormat),
    /// Permission mode for the session
    PermissionMode(PermissionMode),
    /// Tool for handling permission prompts (e.g., "stdio")
    PermissionPromptTool(String),
    /// Load plugins from directories
    PluginDir(Vec<PathBuf>),
    /// Print response and exit
    Print,
    /// Re-emit user messages from stdin back on stdout
    ReplayUserMessages,
    /// Resume a conversation by session ID
    Resume(Option<String>),
    /// Use a specific session ID (UUID or tagged ID)
    SessionId(String),
    /// Comma-separated list of setting sources (user, project, local)
    SettingSources(String),
    /// Path to settings JSON file or JSON string
    Settings(String),
    /// Only use MCP servers from --mcp-config
    StrictMcpConfig,
    /// System prompt for the session
    SystemPrompt(String),
    /// Specify available tools from the built-in set
    Tools(Vec<String>),
    /// Override verbose mode setting
    Verbose,
}

impl CliFlag {
    /// Get the CLI flag string (e.g., `"--model"`)
    pub fn as_flag(&self) -> &'static str {
        match self {
            CliFlag::AddDir(_) => "--add-dir",
            CliFlag::Agent(_) => "--agent",
            CliFlag::Agents(_) => "--agents",
            CliFlag::AllowDangerouslySkipPermissions => "--allow-dangerously-skip-permissions",
            CliFlag::AllowedTools(_) => "--allowed-tools",
            CliFlag::AppendSystemPrompt(_) => "--append-system-prompt",
            CliFlag::Betas(_) => "--betas",
            CliFlag::Chrome => "--chrome",
            CliFlag::Continue => "--continue",
            CliFlag::DangerouslySkipPermissions => "--dangerously-skip-permissions",
            CliFlag::Debug(_) => "--debug",
            CliFlag::DebugFile(_) => "--debug-file",
            CliFlag::DisableSlashCommands => "--disable-slash-commands",
            CliFlag::DisallowedTools(_) => "--disallowed-tools",
            CliFlag::FallbackModel(_) => "--fallback-model",
            CliFlag::File(_) => "--file",
            CliFlag::ForkSession => "--fork-session",
            CliFlag::FromPr(_) => "--from-pr",
            CliFlag::IncludePartialMessages => "--include-partial-messages",
            CliFlag::InputFormat(_) => "--input-format",
            CliFlag::JsonSchema(_) => "--json-schema",
            CliFlag::MaxBudgetUsd(_) => "--max-budget-usd",
            CliFlag::McpConfig(_) => "--mcp-config",
            CliFlag::McpDebug => "--mcp-debug",
            CliFlag::Model(_) => "--model",
            CliFlag::NoChrome => "--no-chrome",
            CliFlag::NoSessionPersistence => "--no-session-persistence",
            CliFlag::OutputFormat(_) => "--output-format",
            CliFlag::PermissionMode(_) => "--permission-mode",
            CliFlag::PermissionPromptTool(_) => "--permission-prompt-tool",
            CliFlag::PluginDir(_) => "--plugin-dir",
            CliFlag::Print => "--print",
            CliFlag::ReplayUserMessages => "--replay-user-messages",
            CliFlag::Resume(_) => "--resume",
            CliFlag::SessionId(_) => "--session-id",
            CliFlag::SettingSources(_) => "--setting-sources",
            CliFlag::Settings(_) => "--settings",
            CliFlag::StrictMcpConfig => "--strict-mcp-config",
            CliFlag::SystemPrompt(_) => "--system-prompt",
            CliFlag::Tools(_) => "--tools",
            CliFlag::Verbose => "--verbose",
        }
    }

    /// Convert this flag into CLI arguments (flag + value)
    pub fn to_args(&self) -> Vec<String> {
        let flag = self.as_flag().to_string();
        match self {
            // Boolean flags (no value)
            CliFlag::AllowDangerouslySkipPermissions
            | CliFlag::Chrome
            | CliFlag::Continue
            | CliFlag::DangerouslySkipPermissions
            | CliFlag::DisableSlashCommands
            | CliFlag::ForkSession
            | CliFlag::IncludePartialMessages
            | CliFlag::McpDebug
            | CliFlag::NoChrome
            | CliFlag::NoSessionPersistence
            | CliFlag::Print
            | CliFlag::ReplayUserMessages
            | CliFlag::StrictMcpConfig
            | CliFlag::Verbose => vec![flag],

            // Optional value flags
            CliFlag::Debug(filter) => match filter {
                Some(f) => vec![flag, f.clone()],
                None => vec![flag],
            },
            CliFlag::FromPr(value) | CliFlag::Resume(value) => match value {
                Some(v) => vec![flag, v.clone()],
                None => vec![flag],
            },

            // Single string value flags
            CliFlag::Agent(v)
            | CliFlag::Agents(v)
            | CliFlag::AppendSystemPrompt(v)
            | CliFlag::FallbackModel(v)
            | CliFlag::JsonSchema(v)
            | CliFlag::Model(v)
            | CliFlag::PermissionPromptTool(v)
            | CliFlag::SessionId(v)
            | CliFlag::SettingSources(v)
            | CliFlag::Settings(v)
            | CliFlag::SystemPrompt(v) => vec![flag, v.clone()],

            // Format flags
            CliFlag::InputFormat(f) => vec![flag, f.as_str().to_string()],
            CliFlag::OutputFormat(f) => vec![flag, f.as_str().to_string()],
            CliFlag::PermissionMode(m) => vec![flag, m.as_str().to_string()],

            // Numeric flags
            CliFlag::MaxBudgetUsd(amount) => vec![flag, amount.to_string()],

            // Path flags
            CliFlag::DebugFile(p) => vec![flag, p.to_string_lossy().to_string()],

            // Multi-value string flags
            CliFlag::AllowedTools(items)
            | CliFlag::Betas(items)
            | CliFlag::DisallowedTools(items)
            | CliFlag::File(items)
            | CliFlag::McpConfig(items)
            | CliFlag::Tools(items) => {
                let mut args = vec![flag];
                args.extend(items.clone());
                args
            }

            // Multi-value path flags
            CliFlag::AddDir(paths) | CliFlag::PluginDir(paths) => {
                let mut args = vec![flag];
                args.extend(paths.iter().map(|p| p.to_string_lossy().to_string()));
                args
            }
        }
    }

    /// Returns all CLI flag names with their flag strings.
    ///
    /// Useful for enumerating available options in a UI or for validation.
    ///
    /// # Example
    /// ```
    /// use claude_codes::CliFlag;
    ///
    /// for (name, flag) in CliFlag::all_flags() {
    ///     println!("{}: {}", name, flag);
    /// }
    /// ```
    pub fn all_flags() -> Vec<(&'static str, &'static str)> {
        vec![
            ("AddDir", "--add-dir"),
            ("Agent", "--agent"),
            ("Agents", "--agents"),
            (
                "AllowDangerouslySkipPermissions",
                "--allow-dangerously-skip-permissions",
            ),
            ("AllowedTools", "--allowed-tools"),
            ("AppendSystemPrompt", "--append-system-prompt"),
            ("Betas", "--betas"),
            ("Chrome", "--chrome"),
            ("Continue", "--continue"),
            (
                "DangerouslySkipPermissions",
                "--dangerously-skip-permissions",
            ),
            ("Debug", "--debug"),
            ("DebugFile", "--debug-file"),
            ("DisableSlashCommands", "--disable-slash-commands"),
            ("DisallowedTools", "--disallowed-tools"),
            ("FallbackModel", "--fallback-model"),
            ("File", "--file"),
            ("ForkSession", "--fork-session"),
            ("FromPr", "--from-pr"),
            ("IncludePartialMessages", "--include-partial-messages"),
            ("InputFormat", "--input-format"),
            ("JsonSchema", "--json-schema"),
            ("MaxBudgetUsd", "--max-budget-usd"),
            ("McpConfig", "--mcp-config"),
            ("McpDebug", "--mcp-debug"),
            ("Model", "--model"),
            ("NoChrome", "--no-chrome"),
            ("NoSessionPersistence", "--no-session-persistence"),
            ("OutputFormat", "--output-format"),
            ("PermissionMode", "--permission-mode"),
            ("PermissionPromptTool", "--permission-prompt-tool"),
            ("PluginDir", "--plugin-dir"),
            ("Print", "--print"),
            ("ReplayUserMessages", "--replay-user-messages"),
            ("Resume", "--resume"),
            ("SessionId", "--session-id"),
            ("SettingSources", "--setting-sources"),
            ("Settings", "--settings"),
            ("StrictMcpConfig", "--strict-mcp-config"),
            ("SystemPrompt", "--system-prompt"),
            ("Tools", "--tools"),
            ("Verbose", "--verbose"),
        ]
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
    session_id: Option<Uuid>,
    oauth_token: Option<String>,
    api_key: Option<String>,
    /// Tool for handling permission prompts (e.g., "stdio" for bidirectional control)
    permission_prompt_tool: Option<String>,
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
            permission_prompt_tool: None,
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

    /// Set a specific session ID (must be a UUID)
    pub fn session_id(mut self, id: Uuid) -> Self {
        self.session_id = Some(id);
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

    /// Enable bidirectional tool permission protocol via stdio
    ///
    /// When enabled, Claude CLI will send permission requests via stdout
    /// and expect responses via stdin. Use "stdio" for standard I/O based
    /// permission handling.
    ///
    /// # Example
    /// ```
    /// use claude_codes::ClaudeCliBuilder;
    ///
    /// let builder = ClaudeCliBuilder::new()
    ///     .permission_prompt_tool("stdio")
    ///     .model("sonnet");
    /// ```
    pub fn permission_prompt_tool<S: Into<String>>(mut self, tool: S) -> Self {
        self.permission_prompt_tool = Some(tool.into());
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

        if let Some(ref tool) = self.permission_prompt_tool {
            args.push("--permission-prompt-tool".to_string());
            args.push(tool.clone());
        }

        // Only add --session-id when NOT resuming/continuing an existing session
        // (Claude CLI error: --session-id can only be used with --continue or --resume
        // if --fork-session is also specified)
        if self.resume.is_none() && !self.continue_conversation {
            args.push("--session-id".to_string());
            let session_uuid = self.session_id.unwrap_or_else(|| {
                let uuid = Uuid::new_v4();
                debug!("[CLI] Generated session UUID: {}", uuid);
                uuid
            });
            args.push(session_uuid.to_string());
        }

        // Add prompt as the last argument if provided
        if let Some(ref prompt) = self.prompt {
            args.push(prompt.clone());
        }

        args
    }

    /// Spawn the Claude process
    #[cfg(feature = "async-client")]
    pub async fn spawn(self) -> Result<tokio::process::Child> {
        let args = self.build_args();

        // Log the full command being executed
        debug!(
            "[CLI] Executing command: {} {}",
            self.command.display(),
            args.join(" ")
        );

        let mut cmd = tokio::process::Command::new(&self.command);
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
    #[cfg(feature = "async-client")]
    pub fn build_command(self) -> tokio::process::Command {
        let args = self.build_args();
        let mut cmd = tokio::process::Command::new(&self.command);
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

    #[test]
    fn test_permission_prompt_tool() {
        let builder = ClaudeCliBuilder::new().permission_prompt_tool("stdio");
        let args = builder.build_args();

        assert!(args.contains(&"--permission-prompt-tool".to_string()));
        assert!(args.contains(&"stdio".to_string()));
    }

    #[test]
    fn test_permission_prompt_tool_not_present_by_default() {
        let builder = ClaudeCliBuilder::new();
        let args = builder.build_args();

        assert!(!args.contains(&"--permission-prompt-tool".to_string()));
    }

    #[test]
    fn test_session_id_present_for_new_session() {
        let builder = ClaudeCliBuilder::new();
        let args = builder.build_args();

        assert!(
            args.contains(&"--session-id".to_string()),
            "New sessions should have --session-id"
        );
    }

    #[test]
    fn test_session_id_not_present_with_resume() {
        // When resuming a session, --session-id should NOT be added
        // (Claude CLI rejects --session-id + --resume without --fork-session)
        let builder = ClaudeCliBuilder::new().resume(Some("existing-uuid".to_string()));
        let args = builder.build_args();

        assert!(
            args.contains(&"--resume".to_string()),
            "Should have --resume flag"
        );
        assert!(
            !args.contains(&"--session-id".to_string()),
            "--session-id should NOT be present when resuming"
        );
    }

    #[test]
    fn test_session_id_not_present_with_continue() {
        // When continuing a session, --session-id should NOT be added
        let builder = ClaudeCliBuilder::new().continue_conversation(true);
        let args = builder.build_args();

        assert!(
            args.contains(&"--continue".to_string()),
            "Should have --continue flag"
        );
        assert!(
            !args.contains(&"--session-id".to_string()),
            "--session-id should NOT be present when continuing"
        );
    }
}
