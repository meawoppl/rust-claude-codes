//! Async client for `pi --mode rpc`: spawns the CLI, writes one command
//! per LF-terminated line, and demultiplexes stdout into command
//! responses and streamed events.
//!
//! Framing follows the protocol contract: records split on `\n` only
//! (tokio's line reader complies; it never splits on U+2028/U+2029).

use crate::cli::{Mode, PiCliBuilder};
use crate::error::{Error, Result};
use crate::io::PiEvent;
use crate::rpc::{RpcCommand, RpcResponse};
use std::collections::VecDeque;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};

/// A running `pi --mode rpc` process.
pub struct PiRpcClient {
    child: Child,
    stdin: ChildStdin,
    stdout: tokio::io::Lines<BufReader<ChildStdout>>,
    /// Events that arrived while waiting for a command response.
    pending_events: VecDeque<PiEvent>,
    next_id: u64,
}

impl PiRpcClient {
    /// Spawn `pi --mode rpc` from a builder. The builder's mode is forced
    /// to [`Mode::Rpc`].
    pub async fn spawn(builder: PiCliBuilder) -> Result<Self> {
        let program = which::which("pi").map_err(|_| Error::BinaryNotFound)?;
        let mut cmd = tokio::process::Command::new(program);
        cmd.args(builder.mode(Mode::Rpc).assembled_args())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        let mut child = cmd.spawn()?;
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout).lines(),
            pending_events: VecDeque::new(),
            next_id: 0,
        })
    }

    /// Spawn with defaults: an ephemeral (`--no-session`) RPC server.
    pub async fn start() -> Result<Self> {
        Self::spawn(PiCliBuilder::new().no_session(true)).await
    }

    fn fresh_id(&mut self) -> String {
        self.next_id += 1;
        format!("pi-codes-{}", self.next_id)
    }

    /// Send one command and wait for its response. Events that arrive in
    /// between are buffered for [`next_event`](Self::next_event).
    ///
    /// The command's `id` is used for correlation; commands built without
    /// one get a client-generated id (the wire response echoes it).
    pub async fn request(&mut self, mut command: RpcCommand) -> Result<RpcResponse> {
        let id = match command_id(&command) {
            Some(id) => id,
            None => {
                let id = self.fresh_id();
                set_command_id(&mut command, &id);
                id
            }
        };
        let line = command.to_line()?;
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.flush().await?;

        loop {
            let v = self.next_value().await?;
            if v.get("type").and_then(serde_json::Value::as_str) == Some("response") {
                let resp: RpcResponse =
                    serde_json::from_value(v.clone()).map_err(|source| Error::Deserialization {
                        source,
                        raw_line: v.to_string(),
                    })?;
                // Correlate on id when the wire echoes one; a response
                // without an id answers the oldest in-flight command,
                // which for this sequential client is ours.
                match &resp.id {
                    Some(rid) if rid != &id => continue,
                    _ => return Ok(resp),
                }
            }
            self.pending_events
                .push_back(PiEvent::from_value(v.clone()).map_err(|source| {
                    Error::Deserialization {
                        source,
                        raw_line: v.to_string(),
                    }
                })?);
        }
    }

    /// Convenience: send and fail on `success: false`.
    pub async fn request_ok(&mut self, command: RpcCommand) -> Result<RpcResponse> {
        let resp = self.request(command).await?;
        if resp.success {
            Ok(resp)
        } else {
            Err(Error::CommandFailed(
                resp.error.unwrap_or_else(|| resp.command.clone()),
            ))
        }
    }

    /// Next streamed event: buffered ones first, then the wire. Returns
    /// `Ok(None)` on clean EOF (process exit).
    pub async fn next_event(&mut self) -> Result<Option<PiEvent>> {
        if let Some(e) = self.pending_events.pop_front() {
            return Ok(Some(e));
        }
        match self.try_next_value().await? {
            None => Ok(None),
            Some(v) => Ok(Some(PiEvent::from_value(v.clone()).map_err(|source| {
                Error::Deserialization {
                    source,
                    raw_line: v.to_string(),
                }
            })?)),
        }
    }

    async fn next_value(&mut self) -> Result<serde_json::Value> {
        match self.try_next_value().await? {
            Some(v) => Ok(v),
            None => {
                let mut err = String::new();
                if let Some(stderr) = self.child.stderr.take() {
                    use tokio::io::AsyncReadExt;
                    let mut r = BufReader::new(stderr);
                    let _ = r.read_to_string(&mut err).await;
                }
                Err(Error::ProcessExited(err.trim().to_string()))
            }
        }
    }

    async fn try_next_value(&mut self) -> Result<Option<serde_json::Value>> {
        loop {
            let Some(line) = self.stdout.next_line().await? else {
                return Ok(None);
            };
            let line = line.trim_end_matches('\r');
            if line.trim().is_empty() {
                continue;
            }
            return serde_json::from_str(line)
                .map(Some)
                .map_err(|source| Error::Deserialization {
                    source,
                    raw_line: line.to_string(),
                });
        }
    }

    /// Kill the process and wait for exit.
    pub async fn shutdown(mut self) -> Result<()> {
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
        Ok(())
    }
}

fn command_id(c: &RpcCommand) -> Option<String> {
    match serde_json::to_value(c) {
        Ok(v) => v
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        Err(_) => None,
    }
}

fn set_command_id(c: &mut RpcCommand, id: &str) {
    // Round-trip through JSON: every modeled command carries an optional
    // `id`, and Raw commands are plain objects.
    if let Ok(mut v) = serde_json::to_value(&*c) {
        if let Some(obj) = v.as_object_mut() {
            obj.insert("id".into(), serde_json::Value::String(id.to_string()));
            if let Ok(with_id) = serde_json::from_value::<RpcCommand>(v) {
                *c = with_id;
            }
        }
    }
}
