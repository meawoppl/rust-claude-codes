//! Synchronous client for Claude communication

use crate::cli::ClaudeCliBuilder;
use crate::error::{Error, Result};
use crate::io::{ClaudeInput, ClaudeOutput, ContentBlock, ParseError};
use crate::protocol::Protocol;
use std::io::{BufRead, BufReader};
use std::process::{Child, ChildStdin, ChildStdout};
use tracing::debug;
use uuid::Uuid;

/// Synchronous client for communicating with Claude
pub struct SyncClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    session_uuid: Option<Uuid>,
}

impl SyncClient {
    /// Create a new synchronous client from an existing child process
    pub fn new(mut child: Child) -> Result<Self> {
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
            stdin,
            stdout: BufReader::new(stdout),
            session_uuid: None,
        })
    }

    /// Create a new synchronous client with default settings
    pub fn with_defaults() -> Result<Self> {
        // Check Claude version (only warns once per session)
        // NOTE: The claude-codes API is in high flux. If you wish to work around
        // this version check, you can use SyncClient::new() directly with:
        //   let child = ClaudeCliBuilder::new().spawn_sync()?;
        //   SyncClient::new(child)
        crate::version::check_claude_version()?;
        let child = ClaudeCliBuilder::new().spawn_sync().map_err(Error::Io)?;
        Self::new(child)
    }

    /// Resume a previous session by UUID
    /// This creates a new client that resumes an existing session
    pub fn resume_session(session_uuid: Uuid) -> Result<Self> {
        let child = ClaudeCliBuilder::new()
            .resume(Some(session_uuid.to_string()))
            .spawn_sync()
            .map_err(Error::Io)?;

        debug!("Resuming Claude session with UUID: {}", session_uuid);
        let mut client = Self::new(child)?;
        // Pre-populate the session UUID since we're resuming
        client.session_uuid = Some(session_uuid);
        Ok(client)
    }

    /// Resume a previous session with a specific model
    pub fn resume_session_with_model(session_uuid: Uuid, model: &str) -> Result<Self> {
        let child = ClaudeCliBuilder::new()
            .model(model)
            .resume(Some(session_uuid.to_string()))
            .spawn_sync()
            .map_err(Error::Io)?;

        debug!(
            "Resuming Claude session with UUID: {} and model: {}",
            session_uuid, model
        );
        let mut client = Self::new(child)?;
        // Pre-populate the session UUID since we're resuming
        client.session_uuid = Some(session_uuid);
        Ok(client)
    }

    /// Send a query and collect all responses
    pub fn query(&mut self, input: ClaudeInput) -> Result<Vec<ClaudeOutput>> {
        let mut responses = Vec::new();
        for response in self.query_stream(input)? {
            responses.push(response?);
        }
        Ok(responses)
    }

    /// Send a query and return an iterator over responses
    pub fn query_stream(&mut self, input: ClaudeInput) -> Result<ResponseIterator<'_>> {
        // Send the input
        Protocol::write_sync(&mut self.stdin, &input)?;

        Ok(ResponseIterator {
            client: self,
            finished: false,
        })
    }

    /// Read the next response from Claude
    fn read_next(&mut self) -> Result<Option<ClaudeOutput>> {
        let mut line = String::new();
        match self.stdout.read_line(&mut line) {
            Ok(0) => {
                debug!("[CLIENT] Stream closed");
                Ok(None)
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    debug!("[CLIENT] Skipping empty line");
                    return self.read_next();
                }

                debug!("[CLIENT] Received: {}", trimmed);
                match ClaudeOutput::parse_json_tolerant(trimmed) {
                    Ok(output) => {
                        // Capture UUID from first response if not already set
                        if self.session_uuid.is_none() {
                            if let ClaudeOutput::Assistant(ref msg) = output {
                                if let Some(ref uuid_str) = msg.uuid {
                                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                                        debug!("[CLIENT] Captured session UUID: {}", uuid);
                                        self.session_uuid = Some(uuid);
                                    }
                                }
                            } else if let ClaudeOutput::Result(ref msg) = output {
                                if let Some(ref uuid_str) = msg.uuid {
                                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                                        debug!("[CLIENT] Captured session UUID: {}", uuid);
                                        self.session_uuid = Some(uuid);
                                    }
                                }
                            }
                        }

                        // Check if this is a result message
                        if matches!(output, ClaudeOutput::Result(_)) {
                            debug!("[CLIENT] Received result message, stream complete");
                            Ok(Some(output))
                        } else {
                            Ok(Some(output))
                        }
                    }
                    Err(ParseError { error_message, .. }) => {
                        debug!("[CLIENT] Failed to deserialize: {}", error_message);
                        debug!("[CLIENT] Raw JSON that failed: {}", trimmed);
                        Err(Error::Deserialization(format!(
                            "{} (raw: {})",
                            error_message, trimmed
                        )))
                    }
                }
            }
            Err(e) => {
                debug!("[CLIENT] Error reading from stdout: {}", e);
                Err(Error::Io(e))
            }
        }
    }

    /// Shutdown the client and wait for the process to exit
    pub fn shutdown(&mut self) -> Result<()> {
        debug!("[CLIENT] Shutting down client");
        self.child.kill().map_err(Error::Io)?;
        self.child.wait().map_err(Error::Io)?;
        Ok(())
    }

    /// Get the session UUID if available
    /// Returns an error if no response has been received yet
    pub fn session_uuid(&self) -> Result<Uuid> {
        self.session_uuid.ok_or(Error::SessionNotInitialized)
    }

    /// Test if the Claude connection is working by sending a ping message
    /// Returns true if Claude responds with "pong", false otherwise
    pub fn ping(&mut self) -> bool {
        // Send a simple ping request
        let ping_input = ClaudeInput::user_message(
            "ping - respond with just the word 'pong' and nothing else",
            self.session_uuid.unwrap_or_else(Uuid::new_v4),
        );

        // Try to send the ping and get responses
        match self.query(ping_input) {
            Ok(responses) => {
                // Check all responses for "pong"
                for output in responses {
                    if let ClaudeOutput::Assistant(msg) = &output {
                        for content in &msg.message.content {
                            if let ContentBlock::Text(text) = content {
                                if text.text.to_lowercase().contains("pong") {
                                    return true;
                                }
                            }
                        }
                    }
                }
                false
            }
            Err(e) => {
                debug!("Ping failed: {}", e);
                false
            }
        }
    }
}

/// Iterator over responses from Claude
pub struct ResponseIterator<'a> {
    client: &'a mut SyncClient,
    finished: bool,
}

impl Iterator for ResponseIterator<'_> {
    type Item = Result<ClaudeOutput>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        match self.client.read_next() {
            Ok(Some(output)) => {
                // Check if this is a result message
                if matches!(output, ClaudeOutput::Result(_)) {
                    self.finished = true;
                }
                Some(Ok(output))
            }
            Ok(None) => {
                self.finished = true;
                None
            }
            Err(e) => {
                self.finished = true;
                Some(Err(e))
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
