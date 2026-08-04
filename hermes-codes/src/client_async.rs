//! Asynchronous client for `hermes acp`.
//!
//! Spawns the Hermes ACP adapter and communicates over newline-delimited
//! JSON-RPC 2.0. ACP is **bidirectional**: while a prompt turn runs, the
//! agent streams `session/update` notifications AND sends requests the
//! client must answer (permission prompts; fs/terminal calls if those
//! capabilities were advertised).
//!
//! # Lifecycle
//!
//! 1. [`AsyncClient::start`] — spawns `hermes acp`, performs `initialize`
//! 2. [`AsyncClient::session_new`] — create a session
//! 3. [`AsyncClient::session_prompt`] — send user input; the call resolves
//!    with the final [`PromptResponse`] when the turn ends
//! 4. Between/during prompts, consume [`AsyncClient::next_message`] and
//!    answer [`ServerMessage::Request`]s via [`AsyncClient::respond`]
//! 5. The adapter is killed on [`Drop`]
//!
//! Because `session/prompt` blocks until the turn completes, drive it with
//! [`AsyncClient::request_raw`]-style concurrency only if you need streaming
//! updates mid-turn; the simple pattern is to call
//! [`session_prompt`](AsyncClient::session_prompt) and then drain buffered
//! messages — every notification and agent request that arrived during the
//! turn was answered or buffered by the response loop.

use crate::cli::HermesAcpBuilder;
use crate::error::{Error, Result};
use crate::jsonrpc::{
    JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, RequestId,
    JSONRPC_VERSION,
};
use crate::protocol::{methods, AgentNotification, AgentToClientRequest, ServerMessage};
use crate::protocol_generated::types::{
    AuthenticateRequest, AuthenticateResponse, CancelNotification, ClientCapabilities,
    CloseSessionRequest, CloseSessionResponse, FileSystemCapabilities, ForkSessionRequest,
    ForkSessionResponse, Implementation, InitializeRequest, InitializeResponse,
    ListSessionsRequest, ListSessionsResponse, LoadSessionRequest, LoadSessionResponse,
    NewSessionRequest, NewSessionResponse, PromptRequest, PromptResponse, ResumeSessionRequest,
    ResumeSessionResponse, SetSessionConfigOptionRequest, SetSessionConfigOptionResponse,
    SetSessionModeRequest, SetSessionModeResponse, SetSessionModelRequest, SetSessionModelResponse,
};
use log::warn;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::Child;

/// Buffer size for reading stdout (10 MB — session/update chunks can be large).
const STDOUT_BUFFER_SIZE: usize = 10 * 1024 * 1024;

/// Asynchronous client for a long-lived `hermes acp` process.
pub struct AsyncClient {
    child: Child,
    writer: BufWriter<tokio::process::ChildStdin>,
    reader: BufReader<tokio::process::ChildStdout>,
    _stderr_drain: tokio::task::JoinHandle<()>,
    next_id: AtomicI64,
    /// Incoming traffic that arrived while a request awaited its response.
    buffered: VecDeque<ServerMessage>,
}

impl AsyncClient {
    /// Create a client from an existing child process (stdin/stdout/stderr
    /// must all be piped). Does not perform the `initialize` handshake.
    pub fn new(mut child: Child) -> Result<Self> {
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::Protocol("Failed to get stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Protocol("Failed to get stdout".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| Error::Protocol("Failed to get stderr".to_string()))?;
        let stderr_drain = crate::stderr_drain::spawn_async(stderr);
        Ok(Self {
            child,
            writer: BufWriter::new(stdin),
            reader: BufReader::with_capacity(STDOUT_BUFFER_SIZE, stdout),
            _stderr_drain: stderr_drain,
            next_id: AtomicI64::new(1),
            buffered: VecDeque::new(),
        })
    }

    /// Spawn `hermes acp` and perform the `initialize` handshake with
    /// default client info and no fs/terminal capabilities.
    pub async fn start() -> Result<(Self, InitializeResponse)> {
        Self::start_with(HermesAcpBuilder::new()).await
    }

    /// [`start`](Self::start) with a custom process builder.
    pub async fn start_with(builder: HermesAcpBuilder) -> Result<(Self, InitializeResponse)> {
        let mut client = Self::new(builder.spawn().await?)?;
        let resp = client
            .initialize(&InitializeRequest {
                meta: None,
                client_capabilities: Some(ClientCapabilities {
                    meta: None,
                    auth: None,
                    fs: Some(FileSystemCapabilities {
                        meta: None,
                        read_text_file: Some(false),
                        write_text_file: Some(false),
                    }),
                    terminal: Some(false),
                }),
                client_info: Some(Implementation {
                    meta: None,
                    name: "hermes-codes".to_string(),
                    title: None,
                    version: env!("CARGO_PKG_VERSION").to_string(),
                }),
                protocol_version: crate::version::ACP_PROTOCOL_VERSION,
            })
            .await?;
        Ok((client, resp))
    }

    /// Send the `initialize` request. Must be the first request; ACP has no
    /// follow-up `initialized` notification.
    pub async fn initialize(&mut self, params: &InitializeRequest) -> Result<InitializeResponse> {
        self.request(methods::INITIALIZE, params).await
    }

    /// Send a JSON-RPC request and await its response, buffering any
    /// notifications and agent-to-client requests that arrive in between.
    ///
    /// NOTE: agent requests received while waiting are **buffered, not
    /// answered**. For prompts that may trigger permission requests, prefer
    /// [`session_prompt_with`](Self::session_prompt_with), which lets you
    /// answer them mid-turn.
    pub async fn request<P: Serialize, R: DeserializeOwned>(
        &mut self,
        method: &str,
        params: &P,
    ) -> Result<R> {
        let id = RequestId::Integer(self.next_id.fetch_add(1, Ordering::Relaxed));
        let req = JsonRpcRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: id.clone(),
            method: method.to_string(),
            params: Some(serde_json::to_value(params)?),
        };
        self.send_raw(&req).await?;
        loop {
            match self.read_message().await? {
                JsonRpcMessage::Response(resp) if resp.id == id => {
                    return Ok(serde_json::from_value(resp.result)?);
                }
                JsonRpcMessage::Error(err) if err.id == id => {
                    return Err(Error::JsonRpc {
                        code: err.error.code,
                        message: err.error.message,
                    });
                }
                JsonRpcMessage::Notification(n) => {
                    let typed = AgentNotification::from_envelope(&n.method, n.params)?;
                    self.buffered.push_back(ServerMessage::Notification(typed));
                }
                JsonRpcMessage::Request(r) => {
                    let typed = AgentToClientRequest::from_envelope(&r.method, r.params)?;
                    self.buffered.push_back(ServerMessage::Request {
                        id: r.id,
                        request: typed,
                    });
                }
                JsonRpcMessage::Response(resp) => {
                    warn!("[hermes-codes] response for unexpected id {}", resp.id);
                }
                JsonRpcMessage::Error(err) => {
                    warn!("[hermes-codes] error for unexpected id {}", err.id);
                }
            }
        }
    }

    /// Like [`request`](Self::request), but invokes `on_message` for every
    /// notification or agent request that arrives before the response —
    /// return `Some(result_json)` from the callback to answer an agent
    /// request immediately (required for permission prompts mid-turn).
    pub async fn request_with<P, R, F>(
        &mut self,
        method: &str,
        params: &P,
        mut on_message: F,
    ) -> Result<R>
    where
        P: Serialize,
        R: DeserializeOwned,
        F: FnMut(&ServerMessage) -> Option<serde_json::Value>,
    {
        let id = RequestId::Integer(self.next_id.fetch_add(1, Ordering::Relaxed));
        let req = JsonRpcRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: id.clone(),
            method: method.to_string(),
            params: Some(serde_json::to_value(params)?),
        };
        self.send_raw(&req).await?;
        loop {
            match self.read_message().await? {
                JsonRpcMessage::Response(resp) if resp.id == id => {
                    return Ok(serde_json::from_value(resp.result)?);
                }
                JsonRpcMessage::Error(err) if err.id == id => {
                    return Err(Error::JsonRpc {
                        code: err.error.code,
                        message: err.error.message,
                    });
                }
                JsonRpcMessage::Notification(n) => {
                    let typed = AgentNotification::from_envelope(&n.method, n.params)?;
                    let msg = ServerMessage::Notification(typed);
                    let _ = on_message(&msg);
                    self.buffered.push_back(msg);
                }
                JsonRpcMessage::Request(r) => {
                    let typed = AgentToClientRequest::from_envelope(&r.method, r.params)?;
                    let msg = ServerMessage::Request {
                        id: r.id.clone(),
                        request: typed,
                    };
                    if let Some(result) = on_message(&msg) {
                        self.send_raw(&JsonRpcResponse::new(r.id, result)).await?;
                    } else {
                        self.buffered.push_back(msg);
                    }
                }
                other => {
                    warn!("[hermes-codes] unexpected frame: {other:?}");
                }
            }
        }
    }

    /// Read the next buffered or incoming message. Returns `Ok(None)` when
    /// the adapter closes the connection.
    pub async fn next_message(&mut self) -> Result<Option<ServerMessage>> {
        if let Some(msg) = self.buffered.pop_front() {
            return Ok(Some(msg));
        }
        loop {
            let mut line = String::new();
            let n = self.reader.read_line(&mut line).await?;
            if n == 0 {
                return Ok(None);
            }
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<JsonRpcMessage>(line)? {
                JsonRpcMessage::Notification(notif) => {
                    let typed = AgentNotification::from_envelope(&notif.method, notif.params)?;
                    return Ok(Some(ServerMessage::Notification(typed)));
                }
                JsonRpcMessage::Request(req) => {
                    let typed = AgentToClientRequest::from_envelope(&req.method, req.params)?;
                    return Ok(Some(ServerMessage::Request {
                        id: req.id,
                        request: typed,
                    }));
                }
                other => {
                    warn!("[hermes-codes] dropping unpaired frame: {other:?}");
                }
            }
        }
    }

    /// Answer an agent-to-client request.
    pub async fn respond<R: Serialize>(&mut self, id: RequestId, result: &R) -> Result<()> {
        let resp = JsonRpcResponse::new(id, serde_json::to_value(result)?);
        self.send_raw(&resp).await
    }

    /// Answer an agent-to-client request with a JSON-RPC error.
    pub async fn respond_error(&mut self, id: RequestId, code: i64, message: &str) -> Result<()> {
        let err = crate::jsonrpc::JsonRpcError {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            error: crate::jsonrpc::JsonRpcErrorData {
                code,
                message: message.to_string(),
                data: None,
            },
        };
        self.send_raw(&err).await
    }

    /// Send a notification (no response expected).
    pub async fn notify<P: Serialize>(&mut self, method: &str, params: &P) -> Result<()> {
        let n = JsonRpcNotification {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.to_string(),
            params: Some(serde_json::to_value(params)?),
        };
        self.send_raw(&n).await
    }

    // ── Typed agent methods ────────────────────────────────────────────

    /// `authenticate` — needed when `initialize` advertised auth methods
    /// and no provider credentials are configured.
    pub async fn authenticate(
        &mut self,
        params: &AuthenticateRequest,
    ) -> Result<AuthenticateResponse> {
        self.request(methods::AUTHENTICATE, params).await
    }

    /// `session/new` — create a session (hermes: a fresh conversation).
    pub async fn session_new(&mut self, params: &NewSessionRequest) -> Result<NewSessionResponse> {
        self.request(methods::SESSION_NEW, params).await
    }

    /// `session/load` — replay a persisted session's history.
    pub async fn session_load(
        &mut self,
        params: &LoadSessionRequest,
    ) -> Result<LoadSessionResponse> {
        self.request(methods::SESSION_LOAD, params).await
    }

    /// `session/list` — enumerate persisted sessions.
    pub async fn session_list(
        &mut self,
        params: &ListSessionsRequest,
    ) -> Result<ListSessionsResponse> {
        self.request(methods::SESSION_LIST, params).await
    }

    /// `session/resume` — continue a persisted session without replay.
    pub async fn session_resume(
        &mut self,
        params: &ResumeSessionRequest,
    ) -> Result<ResumeSessionResponse> {
        self.request(methods::SESSION_RESUME, params).await
    }

    /// `session/fork` — branch a session into a new one.
    pub async fn session_fork(
        &mut self,
        params: &ForkSessionRequest,
    ) -> Result<ForkSessionResponse> {
        self.request(methods::SESSION_FORK, params).await
    }

    /// `session/close` — release a session.
    pub async fn session_close(
        &mut self,
        params: &CloseSessionRequest,
    ) -> Result<CloseSessionResponse> {
        self.request(methods::SESSION_CLOSE, params).await
    }

    /// `session/prompt` — run a prompt turn to completion. Agent requests
    /// arriving mid-turn are buffered (see [`request`](Self::request));
    /// use [`session_prompt_with`](Self::session_prompt_with) to answer them.
    pub async fn session_prompt(&mut self, params: &PromptRequest) -> Result<PromptResponse> {
        self.request(methods::SESSION_PROMPT, params).await
    }

    /// `session/prompt` with a mid-turn message callback — the way to
    /// stream `session/update`s and answer permission requests live.
    pub async fn session_prompt_with<F>(
        &mut self,
        params: &PromptRequest,
        on_message: F,
    ) -> Result<PromptResponse>
    where
        F: FnMut(&ServerMessage) -> Option<serde_json::Value>,
    {
        self.request_with(methods::SESSION_PROMPT, params, on_message)
            .await
    }

    /// `session/cancel` — notification; stops the current turn. The turn's
    /// `session/prompt` response then reports `cancelled`.
    pub async fn session_cancel(&mut self, params: &CancelNotification) -> Result<()> {
        self.notify(methods::SESSION_CANCEL, params).await
    }

    /// `session/set_mode`
    pub async fn session_set_mode(
        &mut self,
        params: &SetSessionModeRequest,
    ) -> Result<SetSessionModeResponse> {
        self.request(methods::SESSION_SET_MODE, params).await
    }

    /// `session/set_model`
    pub async fn session_set_model(
        &mut self,
        params: &SetSessionModelRequest,
    ) -> Result<SetSessionModelResponse> {
        self.request(methods::SESSION_SET_MODEL, params).await
    }

    /// `session/set_config_option`
    pub async fn session_set_config_option(
        &mut self,
        params: &SetSessionConfigOptionRequest,
    ) -> Result<SetSessionConfigOptionResponse> {
        self.request(methods::SESSION_SET_CONFIG_OPTION, params)
            .await
    }

    /// Kill the adapter process.
    pub async fn shutdown(&mut self) -> Result<()> {
        self.child.kill().await?;
        Ok(())
    }

    async fn send_raw<M: Serialize>(&mut self, msg: &M) -> Result<()> {
        let mut line = serde_json::to_string(msg)?;
        line.push('\n');
        self.writer.write_all(line.as_bytes()).await?;
        self.writer.flush().await?;
        Ok(())
    }

    async fn read_message(&mut self) -> Result<JsonRpcMessage> {
        loop {
            let mut line = String::new();
            let n = self.reader.read_line(&mut line).await?;
            if n == 0 {
                return Err(Error::ServerClosed);
            }
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            return Ok(serde_json::from_str(line)?);
        }
    }
}
