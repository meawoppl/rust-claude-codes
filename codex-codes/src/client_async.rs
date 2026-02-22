//! Asynchronous multi-turn client for the Codex app-server.
//!
//! Spawns `codex app-server --listen stdio://` and communicates over
//! newline-delimited JSON-RPC. The connection stays open for multiple
//! turns until explicitly shut down.

use crate::cli::AppServerBuilder;
use crate::error::{Error, Result};
use crate::jsonrpc::{JsonRpcError, JsonRpcMessage, JsonRpcRequest, JsonRpcResponse, RequestId};
use crate::protocol::{
    ServerMessage, ThreadArchiveParams, ThreadArchiveResponse, ThreadStartParams,
    ThreadStartResponse, TurnInterruptParams, TurnInterruptResponse, TurnStartParams,
    TurnStartResponse,
};
use log::{debug, error, warn};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, ChildStderr};

/// Buffer size for reading stdout (10MB).
const STDOUT_BUFFER_SIZE: usize = 10 * 1024 * 1024;

/// Asynchronous multi-turn client for the Codex app-server.
///
/// Communicates with a long-lived `codex app-server` process via
/// newline-delimited JSON-RPC over stdio.
pub struct AsyncClient {
    child: Child,
    writer: BufWriter<tokio::process::ChildStdin>,
    reader: BufReader<tokio::process::ChildStdout>,
    stderr: Option<BufReader<ChildStderr>>,
    next_id: AtomicI64,
    /// Buffered incoming messages (notifications/server requests) that arrived
    /// while waiting for a response to a client request.
    buffered: VecDeque<ServerMessage>,
}

impl AsyncClient {
    /// Start an app-server with default settings.
    pub async fn start() -> Result<Self> {
        Self::start_with(AppServerBuilder::new()).await
    }

    /// Start an app-server with a custom builder.
    pub async fn start_with(builder: AppServerBuilder) -> Result<Self> {
        crate::version::check_codex_version_async().await?;

        let mut child = builder.spawn().await?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::Protocol("Failed to get stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Protocol("Failed to get stdout".to_string()))?;
        let stderr = child.stderr.take().map(BufReader::new);

        Ok(Self {
            child,
            writer: BufWriter::new(stdin),
            reader: BufReader::with_capacity(STDOUT_BUFFER_SIZE, stdout),
            stderr,
            next_id: AtomicI64::new(1),
            buffered: VecDeque::new(),
        })
    }

    /// Send a JSON-RPC request and wait for the matching response.
    ///
    /// Any notifications or server requests that arrive before the response
    /// are buffered and can be retrieved via [`AsyncClient::next_message`].
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
                    self.buffered.push_back(ServerMessage::Notification {
                        method: notif.method,
                        params: notif.params,
                    });
                }
                JsonRpcMessage::Request(req) => {
                    self.buffered.push_back(ServerMessage::Request {
                        id: req.id,
                        method: req.method,
                        params: req.params,
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

    /// Start a new thread.
    pub async fn thread_start(
        &mut self,
        params: &ThreadStartParams,
    ) -> Result<ThreadStartResponse> {
        self.request(crate::protocol::methods::THREAD_START, params)
            .await
    }

    /// Start a new turn within a thread.
    pub async fn turn_start(&mut self, params: &TurnStartParams) -> Result<TurnStartResponse> {
        self.request(crate::protocol::methods::TURN_START, params)
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

    /// Respond to a server-to-client request (e.g., approval flow).
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
    /// Returns `None` if the connection is closed (EOF).
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
                    return Ok(Some(ServerMessage::Notification {
                        method: notif.method,
                        params: notif.params,
                    }));
                }
                JsonRpcMessage::Request(req) => {
                    return Ok(Some(ServerMessage::Request {
                        id: req.id,
                        method: req.method,
                        params: req.params,
                    }));
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

    /// Return an async event stream.
    pub fn events(&mut self) -> EventStream<'_> {
        EventStream { client: self }
    }

    /// Take the stderr reader (can only be called once).
    pub fn take_stderr(&mut self) -> Option<BufReader<ChildStderr>> {
        self.stderr.take()
    }

    /// Get the process ID.
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }

    /// Check if the child process is still running.
    pub fn is_alive(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }

    /// Shut down the app-server process.
    pub async fn shutdown(mut self) -> Result<()> {
        debug!("[CLIENT] Shutting down");
        self.child.kill().await.map_err(Error::Io)?;
        Ok(())
    }

    // -- internal --

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
                    return Err(Error::Deserialization(format!("{} (raw: {})", e, trimmed)));
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
