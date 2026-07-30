//! Raw asynchronous transport for the Codex app-server.
//!
//! This client only frames newline-delimited messages. It does not decode JSON
//! or correlate JSON-RPC requests and responses.

use crate::cli::AppServerBuilder;
use crate::error::{Error, Result};
use log::{debug, error};
use serde::Serialize;
use std::process::ExitStatus;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::Child;

const STDOUT_BUFFER_SIZE: usize = 10 * 1024 * 1024;

pub struct RawAsyncClient {
    child: Child,
    writer: BufWriter<tokio::process::ChildStdin>,
    reader: BufReader<tokio::process::ChildStdout>,
    _stderr_drain: tokio::task::JoinHandle<()>,
}

impl RawAsyncClient {
    pub async fn start() -> Result<Self> {
        Self::start_with(AppServerBuilder::new()).await
    }

    pub async fn start_with(builder: AppServerBuilder) -> Result<Self> {
        crate::version::check_codex_version_async().await?;
        Self::new(builder.spawn().await?)
    }

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

        Ok(Self {
            child,
            writer: BufWriter::new(stdin),
            reader: BufReader::with_capacity(STDOUT_BUFFER_SIZE, stdout),
            _stderr_drain: crate::stderr_drain::spawn_async(stderr),
        })
    }

    /// Serialize and write one typed SDK message.
    pub async fn send<T: Serialize>(&mut self, message: &T) -> Result<()> {
        let line = serde_json::to_string(message).map_err(Error::Json)?;
        self.write_line(&line).await
    }

    async fn write_line(&mut self, line: &str) -> Result<()> {
        let line = single_line(line)?;
        debug!("[RAW CLIENT] Sending {} bytes", line.len());
        self.writer
            .write_all(line.as_bytes())
            .await
            .map_err(Error::Io)?;
        self.writer.write_all(b"\n").await.map_err(Error::Io)?;
        self.writer.flush().await.map_err(Error::Io)
    }

    /// Read one complete JSONL frame without inspecting its contents.
    pub async fn next_line(&mut self) -> Result<Option<String>> {
        let mut line = String::new();
        loop {
            line.clear();
            if self.reader.read_line(&mut line).await.map_err(Error::Io)? == 0 {
                return Ok(None);
            }
            remove_line_ending(&mut line);
            if line.trim().is_empty() {
                continue;
            }
            debug!("[RAW CLIENT] Received {} bytes", line.len());
            return Ok(Some(line));
        }
    }

    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }

    pub fn is_alive(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }

    pub async fn wait_for_exit(&mut self) -> Result<ExitStatus> {
        self.child.wait().await.map_err(Error::Io)
    }

    pub async fn shutdown(mut self) -> Result<()> {
        self.child.kill().await.map_err(Error::Io)
    }
}

impl Drop for RawAsyncClient {
    fn drop(&mut self) {
        if self.is_alive() {
            if let Err(error) = self.child.start_kill() {
                error!("Failed to kill raw app-server process on drop: {error}");
            }
        }
    }
}

fn single_line(line: &str) -> Result<&str> {
    let line = line.trim_end_matches(['\r', '\n']);
    if line.contains(['\r', '\n']) {
        return Err(Error::Protocol(
            "raw app-server frame contains an embedded line break".to_string(),
        ));
    }
    Ok(line)
}

fn remove_line_ending(line: &mut String) {
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_one_frame_with_a_line_ending() {
        assert_eq!(single_line("{\"id\":1}\r\n").unwrap(), "{\"id\":1}");
    }

    #[test]
    fn rejects_multiple_frames() {
        assert!(single_line("{\"id\":1}\n{\"id\":2}").is_err());
    }
}
