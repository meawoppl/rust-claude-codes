//! Asynchronous client for Claude communication

use crate::cli::ClaudeCliBuilder;
use crate::error::{Error, Result};
use crate::io::{
    ClaudeInput, ClaudeOutput, ContentBlock, ControlRequestMessage, ControlResponse,
    ControlResponseMessage,
};
use crate::protocol::Protocol;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufReader as AsyncBufReader};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout};
use uuid::Uuid;

/// Asynchronous client for communicating with Claude
pub struct AsyncClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    stderr: Option<BufReader<ChildStderr>>,
    session_uuid: Option<Uuid>,
    /// Whether tool approval protocol has been initialized
    tool_approval_enabled: bool,
}

/// Buffer size for reading Claude's stdout (10MB).
const STDOUT_BUFFER_SIZE: usize = 10 * 1024 * 1024;

impl AsyncClient {
    /// Create a new async client from a tokio Child process
    pub fn new(mut child: Child) -> Result<Self> {
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::Io(std::io::Error::other("Failed to get stdin handle")))?;

        let stdout = BufReader::with_capacity(
            STDOUT_BUFFER_SIZE,
            child
                .stdout
                .take()
                .ok_or_else(|| Error::Io(std::io::Error::other("Failed to get stdout handle")))?,
        );

        let stderr = child.stderr.take().map(BufReader::new);

        Ok(Self {
            child,
            stdin,
            stdout,
            stderr,
            session_uuid: None,
            tool_approval_enabled: false,
        })
    }

    /// Create a client with default settings (using logic from start_claude)
    pub async fn with_defaults() -> Result<Self> {
        // Check Claude version (only warns once per session)
        // NOTE: The claude-codes API is in high flux. If you wish to work around
        // this version check, you can use AsyncClient::new() directly with:
        //   let child = ClaudeCliBuilder::new().model("sonnet").spawn().await?;
        //   AsyncClient::new(child)
        crate::version::check_claude_version_async().await?;
        Self::with_model("sonnet").await
    }

    /// Create a client with a specific model
    pub async fn with_model(model: &str) -> Result<Self> {
        let child = ClaudeCliBuilder::new().model(model).spawn().await?;

        info!("Started Claude process with model: {}", model);
        Self::new(child)
    }

    /// Create a client from a custom builder
    pub async fn from_builder(builder: ClaudeCliBuilder) -> Result<Self> {
        let child = builder.spawn().await?;
        info!("Started Claude process from custom builder");
        Self::new(child)
    }

    /// Resume a previous session by UUID
    /// This creates a new client that resumes an existing session
    pub async fn resume_session(session_uuid: Uuid) -> Result<Self> {
        let child = ClaudeCliBuilder::new()
            .resume(Some(session_uuid.to_string()))
            .spawn()
            .await?;

        info!("Resuming Claude session with UUID: {}", session_uuid);
        let mut client = Self::new(child)?;
        // Pre-populate the session UUID since we're resuming
        client.session_uuid = Some(session_uuid);
        Ok(client)
    }

    /// Resume a previous session with a specific model
    pub async fn resume_session_with_model(session_uuid: Uuid, model: &str) -> Result<Self> {
        let child = ClaudeCliBuilder::new()
            .model(model)
            .resume(Some(session_uuid.to_string()))
            .spawn()
            .await?;

        info!(
            "Resuming Claude session with UUID: {} and model: {}",
            session_uuid, model
        );
        let mut client = Self::new(child)?;
        // Pre-populate the session UUID since we're resuming
        client.session_uuid = Some(session_uuid);
        Ok(client)
    }

    /// Send a query and collect all responses until Result message
    /// This is the simplified version that collects all responses
    pub async fn query(&mut self, text: &str) -> Result<Vec<ClaudeOutput>> {
        let session_id = Uuid::new_v4();
        self.query_with_session(text, session_id).await
    }

    /// Send a query with a custom session ID and collect all responses
    pub async fn query_with_session(
        &mut self,
        text: &str,
        session_id: Uuid,
    ) -> Result<Vec<ClaudeOutput>> {
        // Send the query
        let input = ClaudeInput::user_message(text, session_id);
        self.send(&input).await?;

        // Collect responses until we get a Result message
        let mut responses = Vec::new();

        loop {
            let output = self.receive().await?;
            let is_result = matches!(&output, ClaudeOutput::Result(_));
            responses.push(output);

            if is_result {
                break;
            }
        }

        Ok(responses)
    }

    /// Send a query and return an async iterator over responses
    /// Returns a stream that yields ClaudeOutput until Result message is received
    pub async fn query_stream(&mut self, text: &str) -> Result<ResponseStream<'_>> {
        let session_id = Uuid::new_v4();
        self.query_stream_with_session(text, session_id).await
    }

    /// Send a query with session ID and return an async iterator over responses
    pub async fn query_stream_with_session(
        &mut self,
        text: &str,
        session_id: Uuid,
    ) -> Result<ResponseStream<'_>> {
        // Send the query first
        let input = ClaudeInput::user_message(text, session_id);
        self.send(&input).await?;

        // Return a stream that will read responses
        Ok(ResponseStream {
            client: self,
            finished: false,
        })
    }

    /// Send a ClaudeInput directly
    pub async fn send(&mut self, input: &ClaudeInput) -> Result<()> {
        let json_line = Protocol::serialize(input)?;
        debug!("[OUTGOING] Sending JSON to Claude: {}", json_line.trim());

        self.stdin
            .write_all(json_line.as_bytes())
            .await
            .map_err(Error::Io)?;

        self.stdin.flush().await.map_err(Error::Io)?;
        Ok(())
    }

    /// Receive a single response from Claude.
    ///
    /// # Important: Polling Frequency
    ///
    /// This method should be polled frequently to prevent the OS pipe buffer from
    /// filling up. Claude can emit very large JSON messages (hundreds of KB), and
    /// if the pipe buffer overflows, data may be truncated.
    ///
    /// In a `tokio::select!` loop with other async operations, ensure `receive()`
    /// is given priority or called frequently. For high-throughput scenarios,
    /// consider spawning a dedicated task to drain stdout into an unbounded channel.
    ///
    /// # Returns
    ///
    /// - `Ok(ClaudeOutput)` - A parsed message from Claude
    /// - `Err(Error::ConnectionClosed)` - Claude process has exited
    /// - `Err(Error::Deserialization)` - Failed to parse the message
    pub async fn receive(&mut self) -> Result<ClaudeOutput> {
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = self.stdout.read_line(&mut line).await.map_err(Error::Io)?;

            if bytes_read == 0 {
                return Err(Error::ConnectionClosed);
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            debug!("[INCOMING] Received JSON from Claude: {}", trimmed);

            // Use the parse_json_tolerant method which handles ANSI escape codes
            match ClaudeOutput::parse_json_tolerant(trimmed) {
                Ok(output) => {
                    debug!("[INCOMING] Parsed output type: {}", output.message_type());

                    // Capture UUID from first response if not already set
                    if self.session_uuid.is_none() {
                        if let ClaudeOutput::Assistant(ref msg) = output {
                            if let Some(ref uuid_str) = msg.uuid {
                                if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                                    debug!("[INCOMING] Captured session UUID: {}", uuid);
                                    self.session_uuid = Some(uuid);
                                }
                            }
                        } else if let ClaudeOutput::Result(ref msg) = output {
                            if let Some(ref uuid_str) = msg.uuid {
                                if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                                    debug!("[INCOMING] Captured session UUID: {}", uuid);
                                    self.session_uuid = Some(uuid);
                                }
                            }
                        }
                    }

                    return Ok(output);
                }
                Err(parse_error) => {
                    warn!("[INCOMING] Failed to deserialize message from Claude CLI. Please report this at https://github.com/meawoppl/rust-claude-codes/issues with the raw message below.");
                    warn!("[INCOMING] Parse error: {}", parse_error);
                    warn!("[INCOMING] Raw message: {}", trimmed);
                    return Err(Error::Deserialization(format!(
                        "{} (raw: {})",
                        parse_error.error_message, trimmed
                    )));
                }
            }
        }
    }

    /// Check if the Claude process is still running
    pub fn is_alive(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }

    /// Gracefully shutdown the client
    pub async fn shutdown(mut self) -> Result<()> {
        info!("Shutting down Claude process...");
        self.child.kill().await.map_err(Error::Io)?;
        Ok(())
    }

    /// Get the process ID
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }

    /// Take the stderr reader (can only be called once)
    pub fn take_stderr(&mut self) -> Option<BufReader<ChildStderr>> {
        self.stderr.take()
    }

    /// Get the session UUID if available
    /// Returns an error if no response has been received yet
    pub fn session_uuid(&self) -> Result<Uuid> {
        self.session_uuid.ok_or(Error::SessionNotInitialized)
    }

    /// Test if the Claude connection is working by sending a ping message
    /// Returns true if Claude responds with "pong", false otherwise
    pub async fn ping(&mut self) -> bool {
        // Send a simple ping request
        let ping_input = ClaudeInput::user_message(
            "ping - respond with just the word 'pong' and nothing else",
            self.session_uuid.unwrap_or_else(Uuid::new_v4),
        );

        // Try to send the ping
        if let Err(e) = self.send(&ping_input).await {
            debug!("Ping failed to send: {}", e);
            return false;
        }

        // Try to receive responses until we get a result or error
        let mut found_pong = false;
        let mut message_count = 0;
        const MAX_MESSAGES: usize = 10;

        loop {
            match self.receive().await {
                Ok(output) => {
                    message_count += 1;

                    // Check if it's an assistant message containing "pong"
                    if let ClaudeOutput::Assistant(msg) = &output {
                        for content in &msg.message.content {
                            if let ContentBlock::Text(text) = content {
                                if text.text.to_lowercase().contains("pong") {
                                    found_pong = true;
                                }
                            }
                        }
                    }

                    // Stop on result message
                    if matches!(output, ClaudeOutput::Result(_)) {
                        break;
                    }

                    // Safety limit
                    if message_count >= MAX_MESSAGES {
                        debug!("Ping exceeded message limit");
                        break;
                    }
                }
                Err(e) => {
                    debug!("Ping failed to receive response: {}", e);
                    break;
                }
            }
        }

        found_pong
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
    /// use claude_codes::{AsyncClient, ClaudeCliBuilder, ClaudeOutput, ControlRequestPayload};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let child = ClaudeCliBuilder::new()
    ///     .model("sonnet")
    ///     .permission_prompt_tool("stdio")
    ///     .spawn()
    ///     .await?;
    ///
    /// let mut client = AsyncClient::new(child)?;
    /// client.enable_tool_approval().await?;
    ///
    /// // Now when you receive messages, you may get ControlRequest messages
    /// // that need responses
    /// # Ok(())
    /// # }
    /// ```
    pub async fn enable_tool_approval(&mut self) -> Result<()> {
        if self.tool_approval_enabled {
            debug!("[TOOL_APPROVAL] Already enabled, skipping initialization");
            return Ok(());
        }

        let request_id = format!("init-{}", Uuid::new_v4());
        let init_request = ControlRequestMessage::initialize(&request_id);

        debug!("[TOOL_APPROVAL] Sending initialization handshake");
        let json_line = Protocol::serialize(&init_request)?;
        self.stdin
            .write_all(json_line.as_bytes())
            .await
            .map_err(Error::Io)?;
        self.stdin.flush().await.map_err(Error::Io)?;

        // Wait for the initialization response
        loop {
            let mut line = String::new();
            let bytes_read = self.stdout.read_line(&mut line).await.map_err(Error::Io)?;

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
    /// ```no_run
    /// use claude_codes::{AsyncClient, ClaudeOutput, ControlRequestPayload};
    ///
    /// # async fn example(client: &mut AsyncClient) -> Result<(), Box<dyn std::error::Error>> {
    /// # let output = client.receive().await?;
    /// if let ClaudeOutput::ControlRequest(req) = output {
    ///     if let ControlRequestPayload::CanUseTool(perm_req) = &req.request {
    ///         // Use the ergonomic helpers on ToolPermissionRequest
    ///         let response = if perm_req.tool_name == "Bash" {
    ///             perm_req.deny("Bash commands not allowed", &req.request_id)
    ///         } else {
    ///             perm_req.allow(&req.request_id)
    ///         };
    ///         client.send_control_response(response).await?;
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_control_response(&mut self, response: ControlResponse) -> Result<()> {
        let message: ControlResponseMessage = response.into();
        let json_line = Protocol::serialize(&message)?;
        debug!(
            "[TOOL_APPROVAL] Sending control response: {}",
            json_line.trim()
        );

        self.stdin
            .write_all(json_line.as_bytes())
            .await
            .map_err(Error::Io)?;
        self.stdin.flush().await.map_err(Error::Io)?;
        Ok(())
    }

    /// Check if tool approval protocol is enabled
    pub fn is_tool_approval_enabled(&self) -> bool {
        self.tool_approval_enabled
    }
}

/// A response stream that yields ClaudeOutput messages
/// Holds a reference to the client to read from
pub struct ResponseStream<'a> {
    client: &'a mut AsyncClient,
    finished: bool,
}

impl ResponseStream<'_> {
    /// Convert to a vector by collecting all responses
    pub async fn collect(mut self) -> Result<Vec<ClaudeOutput>> {
        let mut responses = Vec::new();

        while !self.finished {
            let output = self.client.receive().await?;
            let is_result = matches!(&output, ClaudeOutput::Result(_));
            responses.push(output);

            if is_result {
                self.finished = true;
                break;
            }
        }

        Ok(responses)
    }

    /// Get the next response
    pub async fn next(&mut self) -> Option<Result<ClaudeOutput>> {
        if self.finished {
            return None;
        }

        match self.client.receive().await {
            Ok(output) => {
                if matches!(&output, ClaudeOutput::Result(_)) {
                    self.finished = true;
                }
                Some(Ok(output))
            }
            Err(e) => {
                self.finished = true;
                Some(Err(e))
            }
        }
    }
}

impl Drop for AsyncClient {
    fn drop(&mut self) {
        if self.is_alive() {
            // Try to kill the process
            if let Err(e) = self.child.start_kill() {
                error!("Failed to kill Claude process on drop: {}", e);
            }
        }
    }
}

// Protocol extension methods for asynchronous I/O
impl Protocol {
    /// Write a message to an async writer
    pub async fn write_async<W: AsyncWriteExt + Unpin, T: Serialize>(
        writer: &mut W,
        message: &T,
    ) -> Result<()> {
        let line = Self::serialize(message)?;
        debug!("[PROTOCOL] Sending async: {}", line.trim());
        writer.write_all(line.as_bytes()).await?;
        writer.flush().await?;
        Ok(())
    }

    /// Read a message from an async reader
    pub async fn read_async<R: AsyncBufReadExt + Unpin, T: for<'de> Deserialize<'de>>(
        reader: &mut R,
    ) -> Result<T> {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            return Err(Error::ConnectionClosed);
        }
        debug!("[PROTOCOL] Received async: {}", line.trim());
        Self::deserialize(&line)
    }
}

/// Async stream processor for handling continuous message streams
pub struct AsyncStreamProcessor<R> {
    reader: AsyncBufReader<R>,
}

impl<R: tokio::io::AsyncRead + Unpin> AsyncStreamProcessor<R> {
    /// Create a new async stream processor
    pub fn new(reader: R) -> Self {
        Self {
            reader: AsyncBufReader::new(reader),
        }
    }

    /// Process the next message from the stream
    pub async fn next_message<T: for<'de> Deserialize<'de>>(&mut self) -> Result<T> {
        Protocol::read_async(&mut self.reader).await
    }

    /// Process all messages in the stream
    pub async fn process_all<T, F, Fut>(&mut self, mut handler: F) -> Result<()>
    where
        T: for<'de> Deserialize<'de>,
        F: FnMut(T) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        loop {
            match self.next_message().await {
                Ok(message) => handler(message).await?,
                Err(Error::ConnectionClosed) => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}
