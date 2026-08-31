//! Synchronous multi-turn client for the Codex app-server.
//!
//! Spawns `codex app-server --listen stdio://` and communicates over
//! newline-delimited JSON-RPC. The connection stays open for multiple
//! turns until explicitly shut down.
//!
//! This is the blocking counterpart to [`crate::client_async::AsyncClient`].
//! Prefer the async client for applications that already use tokio.
//!
//! # Lifecycle
//!
//! 1. Create a client with [`SyncClient::start`] (spawns and initializes the app-server)
//! 2. Call [`SyncClient::thread_start`] to create a conversation session
//! 3. Call [`SyncClient::turn_start`] to send user input
//! 4. Iterate over [`SyncClient::events`] until `turn/completed`
//! 5. Handle approval requests via [`SyncClient::respond`]
//! 6. Repeat steps 3-5 for follow-up turns
//!
//! # Example
//!
//! ```ignore
//! use codex_codes::{SyncClient, ThreadStartParams, TurnStartParams, UserInput, ServerMessage};
//!
//! let mut client = SyncClient::start()?;
//! let thread = client.thread_start(&ThreadStartParams::default())?;
//!
//! client.turn_start(&TurnStartParams {
//!     thread_id: thread.thread_id().to_string(),
//!     input: vec![UserInput::Text { text: "Hello!".into() }],
//!     model: None,
//!     reasoning_effort: None,
//!     sandbox_policy: None,
//! })?;
//!
//! for result in client.events() {
//!     match result? {
//!         ServerMessage::Notification(n) => {
//!             if let codex_codes::Notification::TurnCompleted(_) = n { break; }
//!         }
//!         _ => {}
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
use log::{debug, warn};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::Child;

/// Buffer size for reading stdout (10MB).
const STDOUT_BUFFER_SIZE: usize = 10 * 1024 * 1024;

/// Synchronous multi-turn client for the Codex app-server.
///
/// Communicates with a long-lived `codex app-server` process via
/// newline-delimited JSON-RPC over stdio. Manages request/response
/// correlation and buffers incoming notifications that arrive while
/// waiting for RPC responses.
///
/// The client automatically kills the app-server process when dropped.
pub struct SyncClient {
    child: Child,
    writer: BufWriter<std::process::ChildStdin>,
    reader: BufReader<std::process::ChildStdout>,
    /// Handle to the background thread draining the child's stderr pipe.
    /// Kept alive for the lifetime of the client; the thread exits on EOF
    /// when the child is killed.
    _stderr_drain: std::thread::JoinHandle<()>,
    next_id: i64,
    buffered: VecDeque<ServerMessage>,
}

impl SyncClient {
    /// Create a client from an existing child process.
    ///
    /// The child's stdin, stdout, and stderr must all be piped. This does not
    /// perform the app-server `initialize` handshake or a Codex version check.
    /// Use [`AppServerBuilder::build_command_sync`] to retain the SDK's
    /// command-line and stdio configuration while customizing how the process
    /// is spawned.
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
        let stderr_drain = crate::stderr_drain::spawn_sync(stderr);

        Ok(Self {
            child,
            writer: BufWriter::new(stdin),
            reader: BufReader::with_capacity(STDOUT_BUFFER_SIZE, stdout),
            _stderr_drain: stderr_drain,
            next_id: 1,
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
    pub fn start() -> Result<Self> {
        Self::start_with(AppServerBuilder::new())
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
    pub fn start_with(builder: AppServerBuilder) -> Result<Self> {
        let mut client = Self::spawn(builder)?;
        client.initialize(&InitializeParams {
            client_info: ClientInfo {
                name: "codex-codes".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: None,
            },
            capabilities: None,
        })?;
        Ok(client)
    }

    /// Spawn an app-server without performing the `initialize` handshake.
    ///
    /// Use this if you need to send a custom [`InitializeParams`] (e.g., with
    /// specific capabilities). You **must** call [`SyncClient::initialize`]
    /// before any other requests.
    pub fn spawn(builder: AppServerBuilder) -> Result<Self> {
        crate::version::check_codex_version()?;
        Self::new(builder.spawn_sync()?)
    }

    /// Send a JSON-RPC request and wait for the matching response.
    ///
    /// Any notifications or server requests that arrive before the response
    /// are buffered and can be retrieved via [`SyncClient::next_message`].
    ///
    /// # Errors
    ///
    /// - [`Error::JsonRpc`] if the server returns a JSON-RPC error
    /// - [`Error::ServerClosed`] if the connection drops before a response arrives
    /// - [`Error::Json`] if response deserialization fails
    pub fn request<P: Serialize, R: DeserializeOwned>(
        &mut self,
        method: &str,
        params: &P,
    ) -> Result<R> {
        let id = RequestId::Integer(self.next_id);
        self.next_id += 1;

        let req = JsonRpcRequest {
            id: id.clone(),
            method: method.to_string(),
            params: Some(serde_json::to_value(params).map_err(Error::Json)?),
        };

        self.send_raw(&req)?;

        loop {
            let msg = self.read_message()?;
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
    pub fn thread_start(&mut self, params: &ThreadStartParams) -> Result<ThreadStartResponse> {
        self.request(crate::protocol::methods::THREAD_START, params)
    }

    /// Resume a previously persisted thread by id.
    ///
    /// Replays the thread's history so turns can continue where they left off.
    pub fn thread_resume(&mut self, params: &ThreadResumeParams) -> Result<ThreadResumeResponse> {
        self.request(crate::protocol::methods::THREAD_RESUME, params)
    }

    /// Fork an existing thread into a new independent thread.
    pub fn thread_fork(&mut self, params: &ThreadForkParams) -> Result<ThreadForkResponse> {
        self.request(crate::protocol::methods::THREAD_FORK, params)
    }

    /// Start a new turn within a thread.
    ///
    /// Sends user input to the agent. After calling this, use [`SyncClient::events`]
    /// or [`SyncClient::next_message`] to consume notifications until `turn/completed`.
    pub fn turn_start(&mut self, params: &TurnStartParams) -> Result<TurnStartResponse> {
        self.request(crate::protocol::methods::TURN_START, params)
    }

    /// Steer an active turn with additional user input (`turn/steer`).
    pub fn turn_steer(&mut self, params: &TurnSteerParams) -> Result<TurnSteerResponse> {
        self.request(crate::protocol::methods::TURN_STEER, params)
    }

    /// Page through a thread's items in canonical order (`thread/items/list`).
    pub fn thread_items_list(
        &mut self,
        params: &ThreadItemsListParams,
    ) -> Result<ThreadItemsListResponse> {
        self.request(crate::protocol::methods::THREAD_ITEMS_LIST, params)
    }

    /// Page through a thread's turns (`thread/turns/list`).
    pub fn thread_turns_list(
        &mut self,
        params: &ThreadTurnsListParams,
    ) -> Result<ThreadTurnsListResponse> {
        self.request(crate::protocol::methods::THREAD_TURNS_LIST, params)
    }

    /// Revert a thread's durable history to before one turn (`thread/revert`).
    pub fn thread_revert(&mut self, params: &ThreadRevertParams) -> Result<ThreadRevertResponse> {
        self.request(crate::protocol::methods::THREAD_REVERT, params)
    }

    /// Interrupt an active turn.
    pub fn turn_interrupt(
        &mut self,
        params: &TurnInterruptParams,
    ) -> Result<TurnInterruptResponse> {
        self.request(crate::protocol::methods::TURN_INTERRUPT, params)
    }

    /// Archive a thread.
    pub fn thread_archive(
        &mut self,
        params: &ThreadArchiveParams,
    ) -> Result<ThreadArchiveResponse> {
        self.request(crate::protocol::methods::THREAD_ARCHIVE, params)
    }

    /// Delete a thread.
    pub fn thread_delete(&mut self, params: &ThreadDeleteParams) -> Result<ThreadDeleteResponse> {
        self.request(crate::protocol::methods::THREAD_DELETE, params)
    }

    /// Perform the `initialize` handshake with the app-server.
    ///
    /// Sends `initialize` with the given params and then sends the
    /// `initialized` notification. This must be the first request after
    /// spawning the process.
    pub fn initialize(&mut self, params: &InitializeParams) -> Result<InitializeResponse> {
        let resp: InitializeResponse =
            self.request(crate::protocol::methods::INITIALIZE, params)?;
        self.send_notification(crate::protocol::methods::INITIALIZED)?;
        Ok(resp)
    }

    /// Respond to a server-to-client request (e.g., approval flow).
    ///
    /// When the server sends a [`ServerMessage::Request`], it expects a response.
    /// Use this method with the request's `id` and a result payload. For command
    /// approval, pass a [`CommandExecutionApprovalResponse`](crate::CommandExecutionApprovalResponse).
    /// For file change approval, pass a [`FileChangeApprovalResponse`](crate::FileChangeApprovalResponse).
    pub fn respond<R: Serialize>(&mut self, id: RequestId, result: &R) -> Result<()> {
        let resp = JsonRpcResponse {
            id,
            result: serde_json::to_value(result).map_err(Error::Json)?,
        };
        self.send_raw(&resp)
    }

    /// Respond to a server-to-client request with an error.
    pub fn respond_error(&mut self, id: RequestId, code: i64, message: &str) -> Result<()> {
        let err = JsonRpcError {
            id,
            error: crate::jsonrpc::JsonRpcErrorData {
                code,
                message: message.to_string(),
                data: None,
            },
        };
        self.send_raw(&err)
    }

    /// Read the next incoming server message (notification or server request).
    ///
    /// Returns buffered messages first (from notifications that arrived during
    /// a [`SyncClient::request`] call), then reads from the wire.
    ///
    /// Returns `Ok(None)` when the app-server closes the connection (EOF).
    pub fn next_message(&mut self) -> Result<Option<ServerMessage>> {
        if let Some(msg) = self.buffered.pop_front() {
            return Ok(Some(msg));
        }

        loop {
            let msg = match self.read_message_opt()? {
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

    /// Return an iterator over [`ServerMessage`]s.
    ///
    /// The iterator yields `Result<ServerMessage>` and terminates when the
    /// connection closes (EOF). This is the idiomatic way to consume a turn's
    /// notifications in synchronous code.
    pub fn events(&mut self) -> EventIterator<'_> {
        EventIterator { client: self }
    }

    /// Shut down the child process.
    ///
    /// Kills the process if it's still running. Called automatically on [`Drop`].
    pub fn shutdown(&mut self) -> Result<()> {
        debug!("[CLIENT] Shutting down");
        match self.child.try_wait() {
            Ok(Some(_)) => Ok(()),
            Ok(None) => {
                self.child.kill().map_err(Error::Io)?;
                self.child.wait().map_err(Error::Io)?;
                Ok(())
            }
            Err(e) => Err(Error::Io(e)),
        }
    }

    // -- internal --

    fn send_notification(&mut self, method: &str) -> Result<()> {
        let notif = JsonRpcNotification {
            method: method.to_string(),
            params: None,
        };
        self.send_raw(&notif)
    }

    fn send_raw<T: Serialize>(&mut self, msg: &T) -> Result<()> {
        let json = serde_json::to_string(msg).map_err(Error::Json)?;
        debug!("[CLIENT] Sending: {}", json);
        self.writer.write_all(json.as_bytes()).map_err(Error::Io)?;
        self.writer.write_all(b"\n").map_err(Error::Io)?;
        self.writer.flush().map_err(Error::Io)?;
        Ok(())
    }

    fn read_message(&mut self) -> Result<JsonRpcMessage> {
        self.read_message_opt()?.ok_or(Error::ServerClosed)
    }

    fn read_message_opt(&mut self) -> Result<Option<JsonRpcMessage>> {
        loop {
            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(0) => {
                    debug!("[CLIENT] Stream closed (EOF)");
                    return Ok(None);
                }
                Ok(_) => {
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
                Err(e) => {
                    debug!("[CLIENT] Error reading stdout: {}", e);
                    return Err(Error::Io(e));
                }
            }
        }
    }
}

impl Drop for SyncClient {
    fn drop(&mut self) {
        if let Err(e) = self.shutdown() {
            debug!("[CLIENT] Error during shutdown: {}", e);
        }
    }
}

/// Iterator over [`ServerMessage`]s from a [`SyncClient`].
pub struct EventIterator<'a> {
    client: &'a mut SyncClient,
}

impl Iterator for EventIterator<'_> {
    type Item = Result<ServerMessage>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.client.next_message() {
            Ok(Some(msg)) => Some(Ok(msg)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
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
