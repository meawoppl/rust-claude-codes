//! Synchronous multi-turn client for the Codex app-server.
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
/// newline-delimited JSON-RPC over stdio.
pub struct SyncClient {
    child: Child,
    writer: BufWriter<std::process::ChildStdin>,
    reader: BufReader<std::process::ChildStdout>,
    next_id: i64,
    buffered: VecDeque<ServerMessage>,
}

impl SyncClient {
    /// Start an app-server with default settings.
    pub fn start() -> Result<Self> {
        Self::start_with(AppServerBuilder::new())
    }

    /// Start an app-server with a custom builder.
    pub fn start_with(builder: AppServerBuilder) -> Result<Self> {
        crate::version::check_codex_version()?;

        let mut child = builder.spawn_sync().map_err(Error::Io)?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::Protocol("Failed to get stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Protocol("Failed to get stdout".to_string()))?;

        Ok(Self {
            child,
            writer: BufWriter::new(stdin),
            reader: BufReader::with_capacity(STDOUT_BUFFER_SIZE, stdout),
            next_id: 1,
            buffered: VecDeque::new(),
        })
    }

    /// Send a JSON-RPC request and wait for the matching response.
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
    pub fn thread_start(&mut self, params: &ThreadStartParams) -> Result<ThreadStartResponse> {
        self.request(crate::protocol::methods::THREAD_START, params)
    }

    /// Start a new turn within a thread.
    pub fn turn_start(&mut self, params: &TurnStartParams) -> Result<TurnStartResponse> {
        self.request(crate::protocol::methods::TURN_START, params)
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

    /// Respond to a server-to-client request (e.g., approval flow).
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
    /// Returns `None` if the connection is closed (EOF).
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

    /// Return an iterator over server messages.
    pub fn events(&mut self) -> EventIterator<'_> {
        EventIterator { client: self }
    }

    /// Shut down the child process.
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
                            return Err(Error::Deserialization(format!(
                                "{} (raw: {})",
                                e, trimmed
                            )));
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
