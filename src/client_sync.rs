//! Synchronous client for Claude communication

use crate::cli::ClaudeCliBuilder;
use crate::error::{Error, Result};
use crate::io::{
    ClaudeInput, ClaudeOutput, ContentBlock, ControlRequestMessage, ControlResponse,
    ControlResponseMessage, ParseError,
};
use crate::protocol::Protocol;
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout};
use uuid::Uuid;

/// Synchronous client for communicating with Claude
pub struct SyncClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    session_uuid: Option<Uuid>,
    /// Whether tool approval protocol has been initialized
    tool_approval_enabled: bool,
}

/// Buffer size for reading Claude's stdout (10MB).
const STDOUT_BUFFER_SIZE: usize = 10 * 1024 * 1024;

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
            stdout: BufReader::with_capacity(STDOUT_BUFFER_SIZE, stdout),
            session_uuid: None,
            tool_approval_enabled: false,
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
                        warn!("[CLIENT] Failed to deserialize message from Claude CLI. Please report this at https://github.com/meawoppl/rust-claude-codes/issues with the raw message below.");
                        warn!("[CLIENT] Parse error: {}", error_message);
                        warn!("[CLIENT] Raw message: {}", trimmed);
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

    // =========================================================================
    // Tool Approval Protocol
    // =========================================================================

    /// Enable the tool approval protocol by performing the initialization handshake.
    ///
    /// After calling this method, the CLI will send `ControlRequest` messages when
    /// Claude wants to use a tool. You must handle these by calling
    /// `send_control_response()` with an appropriate response.
    ///
    /// **Important**: The client must have been created with
    /// `ClaudeCliBuilder::permission_prompt_tool("stdio")` for this to work.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use claude_codes::{SyncClient, ClaudeCliBuilder, ClaudeOutput, ControlRequestPayload};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let child = ClaudeCliBuilder::new()
    ///     .model("sonnet")
    ///     .permission_prompt_tool("stdio")
    ///     .spawn_sync()?;
    ///
    /// let mut client = SyncClient::new(child)?;
    /// client.enable_tool_approval()?;
    ///
    /// // Now when you receive messages, you may get ControlRequest messages
    /// // that need responses
    /// # Ok(())
    /// # }
    /// ```
    pub fn enable_tool_approval(&mut self) -> Result<()> {
        if self.tool_approval_enabled {
            debug!("[TOOL_APPROVAL] Already enabled, skipping initialization");
            return Ok(());
        }

        let request_id = format!("init-{}", Uuid::new_v4());
        let init_request = ControlRequestMessage::initialize(&request_id);

        debug!("[TOOL_APPROVAL] Sending initialization handshake");
        Protocol::write_sync(&mut self.stdin, &init_request)?;

        // Wait for the initialization response
        loop {
            let mut line = String::new();
            let bytes_read = self.stdout.read_line(&mut line).map_err(Error::Io)?;

            if bytes_read == 0 {
                return Err(Error::ConnectionClosed);
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            debug!("[TOOL_APPROVAL] Received: {}", trimmed);

            // Try to parse as ClaudeOutput
            match ClaudeOutput::parse_json_tolerant(trimmed) {
                Ok(ClaudeOutput::ControlResponse(resp)) => {
                    use crate::io::ControlResponsePayload;
                    match &resp.response {
                        ControlResponsePayload::Success {
                            request_id: rid, ..
                        } if rid == &request_id => {
                            debug!("[TOOL_APPROVAL] Initialization successful");
                            self.tool_approval_enabled = true;
                            return Ok(());
                        }
                        ControlResponsePayload::Error { error, .. } => {
                            return Err(Error::Protocol(format!(
                                "Tool approval initialization failed: {}",
                                error
                            )));
                        }
                        _ => {
                            // Different request_id, keep waiting
                            continue;
                        }
                    }
                }
                Ok(_) => {
                    // Got a different message type (system, etc.), keep waiting
                    continue;
                }
                Err(e) => {
                    return Err(Error::Deserialization(e.to_string()));
                }
            }
        }
    }

    /// Send a control response back to the CLI.
    ///
    /// Use this to respond to `ControlRequest` messages received during tool approval.
    /// The easiest way to create responses is using the helper methods on
    /// `ToolPermissionRequest`:
    ///
    /// # Example
    ///
    /// ```ignore
    /// use claude_codes::{SyncClient, ControlRequestPayload, ControlResponse, ToolPermissionRequest};
    ///
    /// fn handle_permission(
    ///     client: &mut SyncClient,
    ///     perm_req: &ToolPermissionRequest,
    ///     request_id: &str,
    /// ) -> claude_codes::Result<()> {
    ///     let response = if perm_req.tool_name == "Bash" {
    ///         perm_req.deny("Bash commands not allowed", request_id)
    ///     } else {
    ///         perm_req.allow(request_id)
    ///     };
    ///     client.send_control_response(response)
    /// }
    /// ```
    pub fn send_control_response(&mut self, response: ControlResponse) -> Result<()> {
        let message: ControlResponseMessage = response.into();
        debug!(
            "[TOOL_APPROVAL] Sending control response: {:?}",
            serde_json::to_string(&message)
        );
        Protocol::write_sync(&mut self.stdin, &message)?;
        Ok(())
    }

    /// Check if tool approval protocol is enabled
    pub fn is_tool_approval_enabled(&self) -> bool {
        self.tool_approval_enabled
    }
}

// Protocol extension methods for synchronous I/O
impl Protocol {
    /// Write a message to a synchronous writer
    pub fn write_sync<W: Write, T: Serialize>(writer: &mut W, message: &T) -> Result<()> {
        let line = Self::serialize(message)?;
        debug!("[PROTOCOL] Sending: {}", line.trim());
        writer.write_all(line.as_bytes())?;
        writer.flush()?;
        Ok(())
    }

    /// Read a message from a synchronous reader
    pub fn read_sync<R: BufRead, T: for<'de> Deserialize<'de>>(reader: &mut R) -> Result<T> {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            return Err(Error::ConnectionClosed);
        }
        debug!("[PROTOCOL] Received: {}", line.trim());
        Self::deserialize(&line)
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

/// Stream processor for handling continuous message streams
pub struct StreamProcessor<R> {
    reader: BufReader<R>,
}

impl<R: std::io::Read> StreamProcessor<R> {
    /// Create a new stream processor
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
        }
    }

    /// Process the next message from the stream
    pub fn next_message<T: for<'de> Deserialize<'de>>(&mut self) -> Result<T> {
        Protocol::read_sync(&mut self.reader)
    }

    /// Process all messages in the stream
    pub fn process_all<T, F>(&mut self, mut handler: F) -> Result<()>
    where
        T: for<'de> Deserialize<'de>,
        F: FnMut(T) -> Result<()>,
    {
        loop {
            match self.next_message() {
                Ok(message) => handler(message)?,
                Err(Error::ConnectionClosed) => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}
