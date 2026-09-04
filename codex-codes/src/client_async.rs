//! Asynchronous multi-turn client for the Codex app-server.
//!
//! Spawns `codex app-server --listen stdio://` and communicates over
//! newline-delimited JSON-RPC. The connection stays open for multiple
//! turns until explicitly shut down.
//!
//! # Lifecycle
//!
//! 1. Create a client with [`AsyncClient::start`] (spawns and initializes the app-server)
//! 2. Call [`AsyncClient::thread_start`] to create a conversation session
//! 3. Call [`AsyncClient::turn_start`] to send user input
//! 4. Consume [`AsyncClient::next_message`] to stream notifications
//! 5. Handle approval requests via [`AsyncClient::respond`]
//! 6. Repeat steps 3-5 for follow-up turns
//! 7. The client kills the app-server on [`Drop`]
//!
//! # Example
//!
//! ```ignore
//! use codex_codes::{AsyncClient, ThreadStartParams, TurnStartParams, UserInput, ServerMessage};
//!
//! let mut client = AsyncClient::start().await?;
//! let thread = client.thread_start(&ThreadStartParams::default()).await?;
//!
//! client.turn_start(&TurnStartParams {
//!     thread_id: thread.thread_id().to_string(),
//!     input: vec![UserInput::Text { text: "Hello!".into() }],
//!     model: None,
//!     reasoning_effort: None,
//!     sandbox_policy: None,
//! }).await?;
//!
//! while let Some(msg) = client.next_message().await? {
//!     match msg {
//!         ServerMessage::Notification(n) => {
//!             if let codex_codes::Notification::TurnCompleted(_) = n { break; }
//!         }
//!         ServerMessage::Request { id, .. } => {
//!             client.respond(id, &serde_json::json!({"decision": "accept"})).await?;
//!         }
//!     }
//! }
//! ```

use crate::cli::AppServerBuilder;
use crate::error::{Error, ParseError, Result};
use crate::jsonrpc::{
    JsonRpcError, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, RequestId,
};
use crate::messages::{Notification, ServerMessage, ServerRequest};
use crate::protocol::{
    ClientInfo, InitializeParams, InitializeResponse, ThreadArchiveParams, ThreadArchiveResponse,
    ThreadDeleteParams, ThreadDeleteResponse, ThreadForkParams, ThreadForkResponse,
    ThreadItemsListParams, ThreadItemsListResponse, ThreadResumeParams, ThreadResumeResponse,
    ThreadRevertParams, ThreadRevertResponse, ThreadStartParams, ThreadStartResponse,
    ThreadTurnsListParams, ThreadTurnsListResponse, TurnInterruptParams, TurnInterruptResponse,
    TurnStartParams, TurnStartResponse, TurnSteerParams, TurnSteerResponse,
};
use crate::protocol_generated::types::{
    CancelLoginAccountParams, CancelLoginAccountResponse, GetAccountParams,
    GetAccountRateLimitsParams, GetAccountRateLimitsResponse, GetAccountResponse,
    GetAccountTokenUsageResponse, LoginAccountParams, LoginAccountResponse, LogoutAccountResponse,
};
use log::{debug, error, warn};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::Child;

/// Buffer size for reading stdout (10MB).
const STDOUT_BUFFER_SIZE: usize = 10 * 1024 * 1024;

/// Asynchronous multi-turn client for the Codex app-server.
///
/// Communicates with a long-lived `codex app-server` process via
/// newline-delimited JSON-RPC over stdio. Manages request/response
/// correlation and buffers incoming notifications that arrive while
/// waiting for RPC responses.
///
/// The client automatically kills the app-server process when dropped.
pub struct AsyncClient {
    child: Child,
    writer: BufWriter<tokio::process::ChildStdin>,
    reader: BufReader<tokio::process::ChildStdout>,
    /// Handle to the background task draining the child's stderr pipe.
    /// Kept alive for the lifetime of the client; the task exits on EOF
    /// when the child is killed.
    _stderr_drain: tokio::task::JoinHandle<()>,
    next_id: AtomicI64,
    /// Buffered incoming messages (notifications/server requests) that arrived
    /// while waiting for a response to a client request.
    buffered: VecDeque<ServerMessage>,
}

impl AsyncClient {
    /// Create a client from an existing Tokio child process.
    ///
    /// The child's stdin, stdout, and stderr must all be piped. This does not
    /// perform the app-server `initialize` handshake or a Codex version check.
    /// Use [`AppServerBuilder::build_command`] to retain the SDK's command-line
    /// and stdio configuration while customizing how the process is spawned.
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

        // The app-server emits ~200 KB/s of tracing to stderr. Without an
        // active reader, the ~64 KB kernel pipe fills almost instantly and
        // the child blocks. Drain in the background and route lines through
        // the `log` crate (see [`crate::stderr_drain`]).
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

    /// Start an app-server with default settings.
    ///
    /// Spawns `codex app-server --listen stdio://`, performs the required
    /// `initialize` handshake, and returns a connected client ready for
    /// `thread_start()`.
    ///
    /// # Errors
    ///
    /// Returns an error if the `codex` CLI is not installed, the version is
    /// incompatible, the process fails to start, or the initialization
    /// handshake fails.
    pub async fn start() -> Result<Self> {
        Self::start_with(AppServerBuilder::new()).await
    }

    /// Start an app-server with a custom [`AppServerBuilder`].
    ///
    /// Performs the required `initialize` handshake before returning.
    /// Use this to configure the binary path, working directory, environment,
    /// or CLI arguments.
    ///
    /// # Errors
    ///
    /// Returns an error if the process fails to start, stdio pipes
    /// cannot be established, or the initialization handshake fails.
    pub async fn start_with(builder: AppServerBuilder) -> Result<Self> {
        let mut client = Self::spawn(builder).await?;
        client
            .initialize(&InitializeParams {
                client_info: ClientInfo {
                    name: "codex-codes".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    title: None,
                },
                capabilities: None,
            })
            .await?;
        Ok(client)
    }

    /// Spawn an app-server without performing the `initialize` handshake.
    ///
    /// Use this if you need to send a custom [`InitializeParams`] (e.g., with
    /// specific capabilities). You **must** call [`AsyncClient::initialize`]
    /// before any other requests.
    pub async fn spawn(builder: AppServerBuilder) -> Result<Self> {
        crate::version::check_codex_version_async().await?;
        Self::new(builder.spawn().await?)
    }

    /// Send a JSON-RPC request and wait for the matching response.
    ///
    /// Any notifications or server requests that arrive before the response
    /// are buffered and can be retrieved via [`AsyncClient::next_message`].
    ///
    /// # Errors
    ///
    /// - [`Error::JsonRpc`] if the server returns a JSON-RPC error
    /// - [`Error::ServerClosed`] if the connection drops before a response arrives
    /// - [`Error::Json`] if response deserialization fails
    pub async fn request<P: Serialize, R: DeserializeOwned>(
        &mut self,
        method: &str,
        params: &P,
    ) -> Result<R> {
        let id = RequestId::Integer(self.next_id.fetch_add(1, Ordering::Relaxed));

        let req = JsonRpcRequest {
            id: id.clone(),
            method: method.to_string(),
            params: Some(serde_json::to_value(params).map_err(Error::Json)?),
        };

        self.send_raw(&req).await?;

        // Read lines until we get a response matching our id
        loop {
            let msg = self.read_message().await?;
            match msg {
                JsonRpcMessage::Response(resp) if resp.id == id => {
                    let result: R = serde_json::from_value(resp.result).map_err(Error::Json)?;
                    return Ok(result);
                }
                JsonRpcMessage::Error(err) if err.id == id => {
                    return Err(Error::JsonRpc {
                        code: err.error.code,
                        message: err.error.message,
                    });
                }
                // Buffer notifications and server requests
                JsonRpcMessage::Notification(notif) => {
                    let typed = Notification::from_envelope(&notif.method, notif.params)
                        .map_err(Error::Json)?;
                    self.buffered.push_back(ServerMessage::Notification(typed));
                }
                JsonRpcMessage::Request(req) => {
                    let typed = ServerRequest::from_envelope(&req.method, req.params)
                        .map_err(Error::Json)?;
                    self.buffered.push_back(ServerMessage::Request {
                        id: req.id,
                        request: typed,
                    });
                }
                // Response/error for a different id — unexpected
                JsonRpcMessage::Response(resp) => {
                    warn!(
                        "[CLIENT] Unexpected response for id={}, expected id={}",
                        resp.id, id
                    );
                }
                JsonRpcMessage::Error(err) => {
                    warn!(
                        "[CLIENT] Unexpected error for id={}, expected id={}",
                        err.id, id
                    );
                }
            }
        }
    }

    /// Start a new thread (conversation session).
    ///
    /// A thread must be created before any turns can be started. The returned
    /// [`ThreadStartResponse`] contains the `thread_id` needed for subsequent calls.
    pub async fn thread_start(
        &mut self,
        params: &ThreadStartParams,
    ) -> Result<ThreadStartResponse> {
        self.request(crate::protocol::methods::THREAD_START, params)
            .await
    }

    /// Resume a previously persisted thread by id.
    ///
    /// Replays the thread's history so turns can continue where they left off.
    pub async fn thread_resume(
        &mut self,
        params: &ThreadResumeParams,
    ) -> Result<ThreadResumeResponse> {
        self.request(crate::protocol::methods::THREAD_RESUME, params)
            .await
    }

    /// Fork an existing thread into a new independent thread.
    pub async fn thread_fork(&mut self, params: &ThreadForkParams) -> Result<ThreadForkResponse> {
        self.request(crate::protocol::methods::THREAD_FORK, params)
            .await
    }

    /// Start a new turn within a thread.
    ///
    /// Sends user input to the agent. After calling this, use [`AsyncClient::next_message`]
    /// to stream notifications until `turn/completed` arrives.
    pub async fn turn_start(&mut self, params: &TurnStartParams) -> Result<TurnStartResponse> {
        self.request(crate::protocol::methods::TURN_START, params)
            .await
    }

    /// Steer an active turn with additional user input (`turn/steer`) —
    /// appends to the running turn instead of starting a new one.
    pub async fn turn_steer(&mut self, params: &TurnSteerParams) -> Result<TurnSteerResponse> {
        self.request(crate::protocol::methods::TURN_STEER, params)
            .await
    }

    /// Page through a thread's items in canonical order (`thread/items/list`).
    /// Follow `next_cursor` for subsequent pages (0.148 upstream).
    pub async fn thread_items_list(
        &mut self,
        params: &ThreadItemsListParams,
    ) -> Result<ThreadItemsListResponse> {
        self.request(crate::protocol::methods::THREAD_ITEMS_LIST, params)
            .await
    }

    /// Page through a thread's turns (`thread/turns/list`); defaults to
    /// newest-first with summary item detail (0.148 upstream).
    pub async fn thread_turns_list(
        &mut self,
        params: &ThreadTurnsListParams,
    ) -> Result<ThreadTurnsListResponse> {
        self.request(crate::protocol::methods::THREAD_TURNS_LIST, params)
            .await
    }

    /// Replace a paginated thread's durable history with the prefix before
    /// one turn (`thread/revert`). Does not revert local file changes
    /// (0.148 upstream).
    pub async fn thread_revert(
        &mut self,
        params: &ThreadRevertParams,
    ) -> Result<ThreadRevertResponse> {
        self.request(crate::protocol::methods::THREAD_REVERT, params)
            .await
    }

    /// Interrupt an active turn.
    pub async fn turn_interrupt(
        &mut self,
        params: &TurnInterruptParams,
    ) -> Result<TurnInterruptResponse> {
        self.request(crate::protocol::methods::TURN_INTERRUPT, params)
            .await
    }

    /// Archive a thread.
    pub async fn thread_archive(
        &mut self,
        params: &ThreadArchiveParams,
    ) -> Result<ThreadArchiveResponse> {
        self.request(crate::protocol::methods::THREAD_ARCHIVE, params)
            .await
    }

    /// Delete a thread.
    pub async fn thread_delete(
        &mut self,
        params: &ThreadDeleteParams,
    ) -> Result<ThreadDeleteResponse> {
        self.request(crate::protocol::methods::THREAD_DELETE, params)
            .await
    }

    /// Perform the `initialize` handshake with the app-server.
    ///
    /// Sends `initialize` with the given params and then sends the
    /// `initialized` notification. This must be the first request after
    /// spawning the process.
    pub async fn initialize(&mut self, params: &InitializeParams) -> Result<InitializeResponse> {
        let resp: InitializeResponse = self
            .request(crate::protocol::methods::INITIALIZE, params)
            .await?;
        self.send_notification(crate::protocol::methods::INITIALIZED)
            .await?;
        Ok(resp)
    }

    /// Respond to a server-to-client request (e.g., approval flow).
    ///
    /// When the server sends a [`ServerMessage::Request`], it expects a response.
    /// Use this method with the request's `id` and a result payload. For command
    /// approval, pass a [`CommandExecutionApprovalResponse`](crate::CommandExecutionApprovalResponse).
    /// For file change approval, pass a [`FileChangeApprovalResponse`](crate::FileChangeApprovalResponse).
    pub async fn respond<R: Serialize>(&mut self, id: RequestId, result: &R) -> Result<()> {
        let resp = JsonRpcResponse {
            id,
            result: serde_json::to_value(result).map_err(Error::Json)?,
        };
        self.send_raw(&resp).await
    }

    /// Respond to a server-to-client request with an error.
    pub async fn respond_error(&mut self, id: RequestId, code: i64, message: &str) -> Result<()> {
        let err = JsonRpcError {
            id,
            error: crate::jsonrpc::JsonRpcErrorData {
                code,
                message: message.to_string(),
                data: None,
            },
        };
        self.send_raw(&err).await
    }

    /// Read the next incoming server message (notification or server request).
    ///
    /// Returns buffered messages first (from notifications that arrived during
    /// an [`AsyncClient::request`] call), then reads from the wire.
    ///
    /// Returns `Ok(None)` when the app-server closes the connection (EOF).
    ///
    /// # Typical notification methods
    ///
    /// | Method | Meaning |
    /// |--------|---------|
    /// | `turn/started` | Agent began processing |
    /// | `item/agentMessage/delta` | Streaming text chunk |
    /// | `item/commandExecution/outputDelta` | Command output chunk |
    /// | `item/started` / `item/completed` | Item lifecycle |
    /// | `turn/completed` | Agent finished the turn |
    /// | `error` | Server-side error |
    pub async fn next_message(&mut self) -> Result<Option<ServerMessage>> {
        // Drain buffered messages first
        if let Some(msg) = self.buffered.pop_front() {
            return Ok(Some(msg));
        }

        // Read from the wire
        loop {
            let msg = match self.read_message_opt().await? {
                Some(m) => m,
                None => return Ok(None),
            };

            match msg {
                JsonRpcMessage::Notification(notif) => {
                    let JsonRpcNotification { method, params } = notif;
                    let typed =
                        Notification::from_envelope(&method, params.clone()).map_err(|e| {
                            Error::Deserialization(ParseError::from_envelope(method, params, e))
                        })?;
                    return Ok(Some(ServerMessage::Notification(typed)));
                }
                JsonRpcMessage::Request(req) => {
                    let JsonRpcRequest { id, method, params } = req;
                    let typed =
                        ServerRequest::from_envelope(&method, params.clone()).map_err(|e| {
                            Error::Deserialization(ParseError::from_envelope(method, params, e))
                        })?;
                    return Ok(Some(ServerMessage::Request { id, request: typed }));
                }
                // Unexpected responses without a pending request
                JsonRpcMessage::Response(resp) => {
                    warn!(
                        "[CLIENT] Unexpected response (no pending request): id={}",
                        resp.id
                    );
                }
                JsonRpcMessage::Error(err) => {
                    warn!(
                        "[CLIENT] Unexpected error (no pending request): id={} code={}",
                        err.id, err.error.code
                    );
                }
            }
        }
    }

    /// Return an async event stream over [`ServerMessage`]s.
    ///
    /// Wraps [`AsyncClient::next_message`] in a stream-like API. Call
    /// [`EventStream::next`] in a loop, or [`EventStream::collect`] to
    /// gather all messages until EOF.
    pub fn events(&mut self) -> EventStream<'_> {
        EventStream { client: self }
    }

    /// Get the process ID.
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }

    // ── Account / auth methods ─────────────────────────────────────────

    /// `account/read` — the active account (plan, email, auth mode), or
    /// `account: null` when logged out.
    pub async fn account_read(&mut self, params: &GetAccountParams) -> Result<GetAccountResponse> {
        self.request(crate::protocol::methods::ACCOUNT_READ, params)
            .await
    }

    /// `account/login/start` — begin a login. The params select the mode
    /// (`apiKey` completes immediately; `chatgpt` returns an auth URL to
    /// open; `chatgptDeviceCode` returns a user code + verification URL).
    /// Browser/device modes complete asynchronously: watch for the
    /// `account/login/completed` notification, or cancel with
    /// [`account_login_cancel`](Self::account_login_cancel).
    pub async fn account_login_start(
        &mut self,
        params: &LoginAccountParams,
    ) -> Result<LoginAccountResponse> {
        self.request(crate::protocol::methods::ACCOUNT_LOGIN_START, params)
            .await
    }

    /// `account/login/cancel` — abort an in-flight browser/device login.
    pub async fn account_login_cancel(
        &mut self,
        params: &CancelLoginAccountParams,
    ) -> Result<CancelLoginAccountResponse> {
        self.request(crate::protocol::methods::ACCOUNT_LOGIN_CANCEL, params)
            .await
    }

    /// `account/logout` — remove the stored credential.
    pub async fn account_logout(&mut self) -> Result<LogoutAccountResponse> {
        self.request(
            crate::protocol::methods::ACCOUNT_LOGOUT,
            &serde_json::json!({}),
        )
        .await
    }

    /// `account/rateLimits/read` — current rate-limit windows.
    ///
    /// `params` declares the client's usage-read capabilities;
    /// `GetAccountRateLimitsParams::default()` serializes as `{}` and matches
    /// the pre-capability wire shape.
    pub async fn account_rate_limits_read(
        &mut self,
        params: GetAccountRateLimitsParams,
    ) -> Result<GetAccountRateLimitsResponse> {
        self.request(crate::protocol::methods::ACCOUNT_RATELIMITS_READ, &params)
            .await
    }

    /// `account/usage/read` — token-usage summary for the account.
    pub async fn account_usage_read(&mut self) -> Result<GetAccountTokenUsageResponse> {
        self.request(
            crate::protocol::methods::ACCOUNT_USAGE_READ,
            &serde_json::json!({}),
        )
        .await
    }

    /// Check if the child process is still running.
    pub fn is_alive(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }

    /// Shut down the app-server process.
    ///
    /// Consumes the client. If you don't call this explicitly, the
    /// [`Drop`] implementation will kill the process automatically.
    pub async fn shutdown(mut self) -> Result<()> {
        debug!("[CLIENT] Shutting down");
        self.child.kill().await.map_err(Error::Io)?;
        Ok(())
    }

    // -- internal --

    async fn send_notification(&mut self, method: &str) -> Result<()> {
        let notif = JsonRpcNotification {
            method: method.to_string(),
            params: None,
        };
        self.send_raw(&notif).await
    }

    async fn send_raw<T: Serialize>(&mut self, msg: &T) -> Result<()> {
        let json = serde_json::to_string(msg).map_err(Error::Json)?;
        debug!("[CLIENT] Sending: {}", json);
        self.writer
            .write_all(json.as_bytes())
            .await
            .map_err(Error::Io)?;
        self.writer.write_all(b"\n").await.map_err(Error::Io)?;
        self.writer.flush().await.map_err(Error::Io)?;
        Ok(())
    }

    async fn read_message(&mut self) -> Result<JsonRpcMessage> {
        self.read_message_opt().await?.ok_or(Error::ServerClosed)
    }

    async fn read_message_opt(&mut self) -> Result<Option<JsonRpcMessage>> {
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = self.reader.read_line(&mut line).await.map_err(Error::Io)?;

            if bytes_read == 0 {
                debug!("[CLIENT] Stream closed (EOF)");
                return Ok(None);
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            debug!("[CLIENT] Received: {}", trimmed);

            match serde_json::from_str::<JsonRpcMessage>(trimmed) {
                Ok(msg) => return Ok(Some(msg)),
                Err(e) => {
                    warn!(
                        "[CLIENT] Failed to deserialize message. \
                         Please report this at https://github.com/meawoppl/rust-code-agent-sdks/issues"
                    );
                    warn!("[CLIENT] Parse error: {}", e);
                    warn!("[CLIENT] Raw: {}", trimmed);
                    return Err(Error::Deserialization(ParseError::from_line(trimmed, e)));
                }
            }
        }
    }
}

impl Drop for AsyncClient {
    fn drop(&mut self) {
        if self.is_alive() {
            if let Err(e) = self.child.start_kill() {
                error!("Failed to kill app-server process on drop: {}", e);
            }
        }
    }
}

/// Async stream of [`ServerMessage`]s from an [`AsyncClient`].
pub struct EventStream<'a> {
    client: &'a mut AsyncClient,
}

impl EventStream<'_> {
    /// Get the next server message.
    pub async fn next(&mut self) -> Option<Result<ServerMessage>> {
        match self.client.next_message().await {
            Ok(Some(msg)) => Some(Ok(msg)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }

    /// Collect all remaining messages.
    pub async fn collect(mut self) -> Result<Vec<ServerMessage>> {
        let mut msgs = Vec::new();
        while let Some(result) = self.next().await {
            msgs.push(result?);
        }
        Ok(msgs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_size() {
        assert_eq!(STDOUT_BUFFER_SIZE, 10 * 1024 * 1024);
    }
}
