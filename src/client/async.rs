//! Asynchronous client for Claude communication

use crate::cli::ClaudeCliBuilder;
use crate::error::{Error, Result};
use crate::io::{ClaudeInput, ClaudeOutput};
use crate::protocol::Protocol;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout};
use tracing::{debug, error, info};
use uuid::Uuid;

/// Asynchronous client for communicating with Claude
pub struct AsyncClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    stderr: Option<BufReader<ChildStderr>>,
    session_uuid: Option<Uuid>,
}

impl AsyncClient {
    /// Create a new async client from a tokio Child process
    pub fn new(mut child: Child) -> Result<Self> {
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::Io(std::io::Error::other("Failed to get stdin handle")))?;

        let stdout = BufReader::new(
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
        self.query_with_session(text, "default").await
    }

    /// Send a query with a custom session ID and collect all responses
    pub async fn query_with_session(
        &mut self,
        text: &str,
        session_id: &str,
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
        self.query_stream_with_session(text, "default").await
    }

    /// Send a query with session ID and return an async iterator over responses
    pub async fn query_stream_with_session(
        &mut self,
        text: &str,
        session_id: &str,
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

    /// Try to receive a single response
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

            // Use the parse_json method which returns ParseError
            match ClaudeOutput::parse_json(trimmed) {
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
                    error!("[INCOMING] Failed to deserialize: {}", parse_error);
                    // Convert ParseError to our Error type
                    return Err(Error::Deserialization(parse_error.error_message));
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
