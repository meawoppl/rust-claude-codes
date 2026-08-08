//! Finding, launching, and handshaking with the `localharness` binary.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

use crate::error::{Error, Result};
use crate::handshake::{decode_output_config, encode_input_config, frame};
use crate::protocol::{
    ClientInfo, CustomAgent, FilesystemWorkspace, GeminiAPIEndpoint, HarnessConfig,
    HarnessConfigSessionContinuationMode, HarnessSideTools, InputConfig, LifecycleHook,
    McpServerConfig, ModelConfig, ModelType, PolicyConfig, SystemInstructions, Tool,
    VertexEndpoint, Workspace,
};

/// The environment variable the reference client checks first, and so do we.
pub const HARNESS_PATH_ENV: &str = "ANTIGRAVITY_HARNESS_PATH";

/// How many lines of harness stderr to retain for error reporting.
const STDERR_TAIL_LINES: usize = 200;

/// Locates the `localharness` binary.
///
/// The binary is distributed **only** inside the platform wheels on PyPI — it
/// has no standalone release and no package manager will place it on `PATH` —
/// so discovery is, in order:
///
/// 1. `$ANTIGRAVITY_HARNESS_PATH`, if set.
/// 2. `localharness` on `PATH`.
///
/// A Python install is deliberately *not* probed here; if the binary came from
/// a wheel, point [`HarnessOptions::binary`] at
/// `<site-packages>/google/antigravity/bin/localharness`, or extract it
/// straight out of the wheel:
///
/// ```sh
/// pip download google-antigravity --no-deps -d /tmp/ag
/// unzip -j /tmp/ag/*.whl 'google/antigravity/bin/localharness' -d ~/.local/bin
/// ```
pub fn find_harness() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os(HARNESS_PATH_ENV) {
        let path = PathBuf::from(path);
        return match std::fs::metadata(&path) {
            Ok(_) => Ok(path),
            Err(source) => Err(Error::HarnessNotExecutable { path, source }),
        };
    }
    which::which("localharness").map_err(|_| Error::HarnessNotFound)
}

/// A model endpoint for [`HarnessOptions::model`].
///
/// At least one model is **required**: a harness initialised without one exits
/// immediately and drops the socket with no error frame.
#[derive(Debug, Clone)]
pub struct ModelBuilder(ModelConfig);

impl ModelBuilder {
    /// A model served by the Gemini Developer API, authenticated with an API key.
    pub fn gemini(name: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self(ModelConfig {
            name: Some(name.into()),
            types: vec![ModelType::Text],
            gemini_api_endpoint: Some(GeminiAPIEndpoint {
                api_key: Some(api_key.into()),
                ..Default::default()
            }),
            ..Default::default()
        })
    }

    /// A model served by Gemini Enterprise (formerly Vertex AI), authenticated
    /// with Application Default Credentials.
    pub fn vertex(
        name: impl Into<String>,
        project: impl Into<String>,
        location: impl Into<String>,
    ) -> Self {
        Self(ModelConfig {
            name: Some(name.into()),
            types: vec![ModelType::Text],
            vertex_endpoint: Some(VertexEndpoint {
                project: Some(project.into()),
                location: Some(location.into()),
                ..Default::default()
            }),
            ..Default::default()
        })
    }

    /// Declares what this model is used for. Defaults to [`ModelType::Text`].
    pub fn types(mut self, types: impl IntoIterator<Item = ModelType>) -> Self {
        self.0.types = types.into_iter().collect();
        self
    }

    /// Escape hatch for an endpoint shape this builder does not cover.
    pub fn from_config(config: ModelConfig) -> Self {
        Self(config)
    }

    /// The underlying wire config.
    pub fn build(self) -> ModelConfig {
        self.0
    }
}

/// How to launch and configure a harness session.
///
/// This carries both halves of startup: the [`InputConfig`] written over stdio
/// during the handshake, and the [`HarnessConfig`] sent as the first WebSocket
/// frame. Most callers only touch the latter.
#[derive(Debug, Clone, Default)]
pub struct HarnessOptions {
    binary: Option<PathBuf>,
    storage_directory: Option<PathBuf>,
    env: HashMap<String, String>,
    client_info: Option<ClientInfo>,
    config: HarnessConfig,
}

impl HarnessOptions {
    /// Default options. A workspace and at least one model still need setting.
    pub fn new() -> Self {
        Self::default()
    }

    /// Overrides binary discovery with an explicit path.
    pub fn binary(mut self, path: impl Into<PathBuf>) -> Self {
        self.binary = Some(path.into());
        self
    }

    /// Where the harness persists conversation state between runs.
    ///
    /// Leave unset for an ephemeral session.
    pub fn storage_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.storage_directory = Some(path.into());
        self
    }

    /// Adds a directory the agent is allowed to read and write.
    pub fn workspace(mut self, directory: impl AsRef<Path>) -> Self {
        self.config.workspaces.push(Workspace {
            filesystem_workspace: Some(FilesystemWorkspace {
                directory: Some(directory.as_ref().display().to_string()),
            }),
        });
        self
    }

    /// Adds a model. Call more than once to register several.
    pub fn model(mut self, model: ModelBuilder) -> Self {
        self.config.models.push(model.build());
        self
    }

    /// Sets the system prompt, replacing the harness's built-in identity.
    pub fn system_instructions(mut self, instructions: SystemInstructions) -> Self {
        self.config.system_instructions = Some(instructions);
        self
    }

    /// Resumes, or creates, a conversation by id.
    pub fn cascade_id(mut self, id: impl Into<String>) -> Self {
        self.config.cascade_id = Some(id.into());
        self.config.session_continuation_mode =
            Some(HarnessConfigSessionContinuationMode::CreateOrResume);
        self
    }

    /// Chooses how an existing `cascade_id` is treated.
    pub fn continuation_mode(mut self, mode: HarnessConfigSessionContinuationMode) -> Self {
        self.config.session_continuation_mode = Some(mode);
        self
    }

    /// Declares a tool the *client* executes. The harness will send a
    /// [`crate::protocol::ToolCall`] and wait for a
    /// [`crate::protocol::ToolResponse`].
    pub fn tool(mut self, tool: Tool) -> Self {
        self.config.tools.push(tool);
        self
    }

    /// Enables or disables the tools that run inside the harness itself
    /// (file edits, shell, search, and friends).
    pub fn harness_side_tools(mut self, tools: HarnessSideTools) -> Self {
        self.config.harness_side_tools = Some(tools);
        self
    }

    /// Registers an MCP server for the harness to connect to.
    pub fn mcp_server(mut self, server: McpServerConfig) -> Self {
        self.config.mcp_servers.push(server);
        self
    }

    /// Subscribes to a lifecycle hook. The harness will block the turn on a
    /// [`crate::protocol::CallHookRequest`] until the client answers.
    pub fn hook(mut self, hook: LifecycleHook) -> Self {
        self.config.enabled_hooks.push(hook);
        self
    }

    /// Registers a named subagent the model can delegate to.
    pub fn subagent(mut self, agent: CustomAgent) -> Self {
        self.config.custom_subagents.push(agent);
        self
    }

    /// Installs tool-permission rules evaluated inside the harness.
    pub fn policy(mut self, policy: PolicyConfig) -> Self {
        self.config.policy_config = Some(policy);
        self
    }

    /// Adds a directory of agent skills.
    pub fn skills_path(mut self, path: impl AsRef<Path>) -> Self {
        self.config
            .skills_paths
            .push(path.as_ref().display().to_string());
        self
    }

    /// Where generated artifacts and media are written.
    pub fn app_data_dir(mut self, path: impl AsRef<Path>) -> Self {
        self.config.app_data_dir = Some(path.as_ref().display().to_string());
        self
    }

    /// Sets an environment variable for the harness process.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Replaces the whole [`HarnessConfig`], for settings this builder does not
    /// surface. Anything set by earlier builder calls is discarded.
    pub fn harness_config(mut self, config: HarnessConfig) -> Self {
        self.config = config;
        self
    }

    /// The [`HarnessConfig`] as built so far.
    pub fn config(&self) -> &HarnessConfig {
        &self.config
    }

    fn input_config(&self) -> InputConfig {
        InputConfig {
            storage_directory: self
                .storage_directory
                .as_ref()
                .map(|p| p.display().to_string())
                .or(Some(String::new())),
            client_info: Some(self.client_info.clone().unwrap_or_else(default_client_info)),
            env: self.env.clone(),
            ..Default::default()
        }
    }
}

fn default_client_info() -> ClientInfo {
    ClientInfo {
        language: Some("rust".into()),
        version: Some(env!("CARGO_PKG_VERSION").into()),
        language_version: Some(String::new()),
        os: Some(std::env::consts::OS.into()),
        os_version: Some(String::new()),
    }
}

/// A running `localharness` process that has completed its stdio handshake.
///
/// Dropping this kills the process.
#[derive(Debug)]
pub struct Harness {
    child: Child,
    port: u16,
    api_key: String,
    stderr: Arc<Mutex<Vec<String>>>,
    /// Both pipes are held open for the life of the session. The harness
    /// treats EOF on stdin as "the client is gone" and exits — closing it after
    /// the handshake makes the WebSocket it just advertised unreachable.
    _stdin: tokio::process::ChildStdin,
    _stdout: tokio::process::ChildStdout,
}

impl Harness {
    /// Spawns the binary and performs the length-prefixed stdio handshake.
    pub async fn launch(options: &HarnessOptions) -> Result<Self> {
        let binary = match &options.binary {
            Some(path) => path.clone(),
            None => find_harness()?,
        };
        log::debug!("launching localharness at {}", binary.display());

        let mut command = Command::new(&binary);
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in &options.env {
            command.env(key, value);
        }
        let mut child = command
            .spawn()
            .map_err(|source| Error::HarnessNotExecutable {
                path: binary.clone(),
                source,
            })?;

        let stderr = Arc::new(Mutex::new(Vec::new()));
        if let Some(pipe) = child.stderr.take() {
            spawn_stderr_drain(pipe, Arc::clone(&stderr));
        }

        let mut stdin = child.stdin.take().expect("stdin was piped");
        let body = encode_input_config(&options.input_config());
        stdin.write_all(&frame(&body)).await?;
        stdin.flush().await?;

        let mut stdout = child.stdout.take().expect("stdout was piped");
        let mut len = [0u8; 4];
        if stdout.read_exact(&mut len).await.is_err() {
            return Err(Error::HandshakeFailed {
                stderr: drain_tail(&stderr),
            });
        }
        let mut buf = vec![0u8; u32::from_le_bytes(len) as usize];
        if stdout.read_exact(&mut buf).await.is_err() {
            return Err(Error::HandshakeFailed {
                stderr: drain_tail(&stderr),
            });
        }

        let config = decode_output_config(&buf)?;
        let port = config.port.unwrap_or_default();
        let api_key = config.api_key.unwrap_or_default();
        if port <= 0 || port > i32::from(u16::MAX) {
            return Err(Error::HandshakeFailed {
                stderr: format!("harness reported an unusable port {port}"),
            });
        }
        log::debug!("harness listening on port {port}");

        Ok(Self {
            child,
            port: port as u16,
            api_key,
            stderr,
            _stdin: stdin,
            _stdout: stdout,
        })
    }

    /// The loopback port the harness bound.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The per-process key the WebSocket upgrade must present.
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// The most recent harness stderr, which is where it reports model and
    /// configuration failures that never reach the wire.
    pub fn stderr_tail(&self) -> String {
        drain_tail(&self.stderr)
    }

    /// Waits for the process to exit, having asked it to stop.
    pub async fn shutdown(mut self) -> Result<()> {
        self.child.start_kill()?;
        self.child.wait().await?;
        Ok(())
    }
}

fn spawn_stderr_drain(pipe: tokio::process::ChildStderr, sink: Arc<Mutex<Vec<String>>>) {
    tokio::spawn(async move {
        use tokio::io::AsyncBufReadExt;
        let mut lines = BufReader::new(pipe).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            log::debug!("localharness: {line}");
            if let Ok(mut buf) = sink.lock() {
                if buf.len() == STDERR_TAIL_LINES {
                    buf.remove(0);
                }
                buf.push(line);
            }
        }
    });
}

fn drain_tail(sink: &Arc<Mutex<Vec<String>>>) -> String {
    sink.lock().map(|buf| buf.join("\n")).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_config_carries_client_info_and_env() {
        let options = HarnessOptions::new().env("FOO", "bar");
        let config = options.input_config();
        assert_eq!(config.env.get("FOO").map(String::as_str), Some("bar"));
        assert_eq!(
            config.client_info.unwrap().language.as_deref(),
            Some("rust")
        );
    }

    #[test]
    fn builder_accumulates_workspaces_and_models() {
        let options = HarnessOptions::new()
            .workspace("/tmp/a")
            .workspace("/tmp/b")
            .model(ModelBuilder::gemini("gemini-3-pro-preview", "k"));
        assert_eq!(options.config().workspaces.len(), 2);
        assert_eq!(options.config().models.len(), 1);
        assert_eq!(
            options.config().workspaces[0]
                .filesystem_workspace
                .as_ref()
                .unwrap()
                .directory
                .as_deref(),
            Some("/tmp/a")
        );
    }

    #[test]
    fn cascade_id_implies_create_or_resume() {
        let options = HarnessOptions::new().cascade_id("abc");
        assert_eq!(
            options.config().session_continuation_mode,
            Some(HarnessConfigSessionContinuationMode::CreateOrResume)
        );
    }

    #[test]
    fn missing_binary_is_reported_as_not_found() {
        let path = PathBuf::from("/nonexistent/localharness");
        let err = std::fs::metadata(&path).unwrap_err();
        let err = Error::HarnessNotExecutable { path, source: err };
        assert!(err.to_string().contains("is not usable"));
    }
}
