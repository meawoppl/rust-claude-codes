//! Synchronous client for Codex CLI communication.
//!
//! Spawns `codex exec --json -`, writes the prompt to stdin, closes stdin,
//! then reads JSONL events from stdout until the turn completes.

use crate::cli::CodexCliBuilder;
use crate::error::{Error, Result};
use crate::io::events::ThreadEvent;
use log::{debug, warn};
use std::io::{BufRead, BufReader, Write};
use std::process::Child;

/// Buffer size for reading stdout (10MB).
const STDOUT_BUFFER_SIZE: usize = 10 * 1024 * 1024;

/// Synchronous client for one-shot Codex queries.
///
/// Each query spawns a fresh `codex exec --json -` process, writes the prompt
/// to stdin, closes stdin, then reads JSONL [`ThreadEvent`]s from stdout.
pub struct SyncClient {
    child: Child,
    stdout: BufReader<std::process::ChildStdout>,
    finished: bool,
}

impl SyncClient {
    /// Spawn a client from a builder and prompt.
    pub fn from_builder(builder: CodexCliBuilder, prompt: &str) -> Result<Self> {
        crate::version::check_codex_version()?;

        let mut child = builder.spawn_sync().map_err(Error::Io)?;

        // Write prompt to stdin and close it
        {
            let stdin = child
                .stdin
                .take()
                .ok_or_else(|| Error::Protocol("Failed to get stdin".to_string()))?;
            let mut writer = std::io::BufWriter::new(stdin);
            writer.write_all(prompt.as_bytes()).map_err(Error::Io)?;
            writer.flush().map_err(Error::Io)?;
            // stdin is dropped here, closing the pipe
        }

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Protocol("Failed to get stdout".to_string()))?;

        Ok(Self {
            child,
            stdout: BufReader::with_capacity(STDOUT_BUFFER_SIZE, stdout),
            finished: false,
        })
    }

    /// One-shot query with default settings.
    pub fn exec(prompt: &str) -> Result<Self> {
        Self::from_builder(CodexCliBuilder::new().full_auto(true), prompt)
    }

    /// One-shot query with a custom builder.
    pub fn exec_with(builder: CodexCliBuilder, prompt: &str) -> Result<Self> {
        Self::from_builder(builder, prompt)
    }

    /// Read the next event from the stream.
    pub fn next_event(&mut self) -> Result<Option<ThreadEvent>> {
        if self.finished {
            return Ok(None);
        }

        loop {
            let mut line = String::new();
            match self.stdout.read_line(&mut line) {
                Ok(0) => {
                    debug!("[CLIENT] Stream closed (EOF)");
                    self.finished = true;
                    return Ok(None);
                }
                Ok(_) => {
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
                            return Err(Error::Deserialization(format!(
                                "{} (raw: {})",
                                e, trimmed
                            )));
                        }
                    }
                }
                Err(e) => {
                    debug!("[CLIENT] Error reading stdout: {}", e);
                    self.finished = true;
                    return Err(Error::Io(e));
                }
            }
        }
    }

    /// Collect all remaining events into a vector.
    pub fn collect_all(&mut self) -> Result<Vec<ThreadEvent>> {
        let mut events = Vec::new();
        while let Some(event) = self.next_event()? {
            events.push(event);
        }
        Ok(events)
    }

    /// Return an iterator over events.
    pub fn events(&mut self) -> EventIterator<'_> {
        EventIterator { client: self }
    }

    /// Shut down the child process.
    pub fn shutdown(&mut self) -> Result<()> {
        debug!("[CLIENT] Shutting down");
        // The child may have already exited since stdin was closed
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

    /// Check if the stream has finished.
    pub fn is_finished(&self) -> bool {
        self.finished
    }
}

impl Drop for SyncClient {
    fn drop(&mut self) {
        if let Err(e) = self.shutdown() {
            debug!("[CLIENT] Error during shutdown: {}", e);
        }
    }
}

/// Iterator over [`ThreadEvent`]s from a [`SyncClient`].
pub struct EventIterator<'a> {
    client: &'a mut SyncClient,
}

impl Iterator for EventIterator<'_> {
    type Item = Result<ThreadEvent>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.client.next_event() {
            Ok(Some(event)) => Some(Ok(event)),
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
