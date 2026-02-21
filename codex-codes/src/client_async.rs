//! Asynchronous client for Codex CLI communication.
//!
//! Spawns `codex exec --json -`, writes the prompt to stdin, closes stdin,
//! then reads JSONL events from stdout until the turn completes.

use crate::cli::CodexCliBuilder;
use crate::error::{Error, Result};
use crate::io::events::ThreadEvent;
use log::{debug, error, warn};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr};

/// Buffer size for reading stdout (10MB).
const STDOUT_BUFFER_SIZE: usize = 10 * 1024 * 1024;

/// Asynchronous client for one-shot Codex queries.
///
/// Each query spawns a fresh `codex exec --json -` process, writes the prompt
/// to stdin, closes stdin, then reads JSONL [`ThreadEvent`]s from stdout.
pub struct AsyncClient {
    child: Child,
    stdout: BufReader<tokio::process::ChildStdout>,
    stderr: Option<BufReader<ChildStderr>>,
    finished: bool,
}

impl AsyncClient {
    /// Spawn a client from a builder and prompt.
    pub async fn from_builder(builder: CodexCliBuilder, prompt: &str) -> Result<Self> {
        crate::version::check_codex_version_async().await?;

        let mut child = builder.spawn().await?;

        // Write prompt to stdin and close it
        {
            let mut stdin = child
                .stdin
                .take()
                .ok_or_else(|| Error::Protocol("Failed to get stdin".to_string()))?;
            stdin
                .write_all(prompt.as_bytes())
                .await
                .map_err(Error::Io)?;
            stdin.flush().await.map_err(Error::Io)?;
            // stdin is dropped here, closing the pipe
        }

        let stdout = BufReader::with_capacity(
            STDOUT_BUFFER_SIZE,
            child
                .stdout
                .take()
                .ok_or_else(|| Error::Protocol("Failed to get stdout".to_string()))?,
        );

        let stderr = child.stderr.take().map(BufReader::new);

        Ok(Self {
            child,
            stdout,
            stderr,
            finished: false,
        })
    }

    /// One-shot query with default settings.
    pub async fn exec(prompt: &str) -> Result<Self> {
        Self::from_builder(CodexCliBuilder::new().full_auto(true), prompt).await
    }

    /// One-shot query with a custom builder.
    pub async fn exec_with(builder: CodexCliBuilder, prompt: &str) -> Result<Self> {
        Self::from_builder(builder, prompt).await
    }

    /// Read the next event from the stream.
    pub async fn next_event(&mut self) -> Result<Option<ThreadEvent>> {
        if self.finished {
            return Ok(None);
        }

        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = self.stdout.read_line(&mut line).await.map_err(Error::Io)?;

            if bytes_read == 0 {
                debug!("[CLIENT] Stream closed (EOF)");
                self.finished = true;
                return Ok(None);
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            debug!("[CLIENT] Received: {}", trimmed);

            match serde_json::from_str::<ThreadEvent>(trimmed) {
                Ok(event) => {
                    if matches!(event, ThreadEvent::TurnCompleted(_)) {
                        self.finished = true;
                    }
                    return Ok(Some(event));
                }
                Err(e) => {
                    warn!(
                        "[CLIENT] Failed to deserialize event. \
                         Please report this at https://github.com/meawoppl/rust-code-agent-sdks/issues"
                    );
                    warn!("[CLIENT] Parse error: {}", e);
                    warn!("[CLIENT] Raw: {}", trimmed);
                    return Err(Error::Deserialization(format!("{} (raw: {})", e, trimmed)));
                }
            }
        }
    }

    /// Collect all remaining events into a vector.
    pub async fn collect_all(&mut self) -> Result<Vec<ThreadEvent>> {
        let mut events = Vec::new();
        while let Some(event) = self.next_event().await? {
            events.push(event);
        }
        Ok(events)
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

    /// Check if the stream has finished.
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Shut down the child process.
    pub async fn shutdown(mut self) -> Result<()> {
        debug!("[CLIENT] Shutting down");
        self.child.kill().await.map_err(Error::Io)?;
        Ok(())
    }
}

impl Drop for AsyncClient {
    fn drop(&mut self) {
        if self.is_alive() {
            if let Err(e) = self.child.start_kill() {
                error!("Failed to kill Codex process on drop: {}", e);
            }
        }
    }
}

/// Async stream of [`ThreadEvent`]s from an [`AsyncClient`].
pub struct EventStream<'a> {
    client: &'a mut AsyncClient,
}

impl EventStream<'_> {
    /// Get the next event.
    pub async fn next(&mut self) -> Option<Result<ThreadEvent>> {
        match self.client.next_event().await {
            Ok(Some(event)) => Some(Ok(event)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }

    /// Collect all remaining events.
    pub async fn collect(mut self) -> Result<Vec<ThreadEvent>> {
        let mut events = Vec::new();
        while let Some(result) = self.next().await {
            events.push(result?);
        }
        Ok(events)
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
