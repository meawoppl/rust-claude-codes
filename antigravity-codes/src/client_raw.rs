//! The unopinionated client: frames in, frames out.

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use crate::error::{Error, Result};
use crate::process::{Harness, HarnessOptions};
use crate::protocol::{
    InitializeConversationEvent, InitializeConversationResponse, InputEvent, OutputEvent,
    OutputEventEvent,
};
use crate::ws;

/// A harness session with no interpretation layered on top.
///
/// [`RawClient`] owns the process and the socket and does exactly three things:
/// the handshake, the initialize exchange, and frame codec. Answering the
/// harness's tool calls, hooks, and policy checks is the caller's job — see
/// [`crate::Client`] for a version that does that for you.
///
/// ```no_run
/// use antigravity_codes::{HarnessOptions, ModelBuilder, RawClient};
/// use antigravity_codes::protocol::{InputEvent, OutputEventEvent};
///
/// # async fn run() -> antigravity_codes::Result<()> {
/// let mut client = RawClient::launch(
///     HarnessOptions::new()
///         .workspace("/tmp/project")
///         .model(ModelBuilder::gemini("gemini-3-pro-preview", "…")),
/// )
/// .await?;
///
/// client.send(&InputEvent::user("hello")).await?;
/// while let Some(event) = client.next_event().await? {
///     if let Some(OutputEventEvent::StepUpdate(step)) = event.into_event() {
///         println!("{:?}", step.text_or_delta());
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct RawClient {
    harness: Harness,
    socket: ws::Socket,
    initialize: InitializeConversationResponse,
    closed: bool,
}

impl RawClient {
    /// Launches a harness, connects, and completes the initialize exchange.
    pub async fn launch(options: HarnessOptions) -> Result<Self> {
        let harness = Harness::launch(&options).await?;
        let socket = ws::connect(harness.port(), harness.api_key()).await?;

        let mut client = Self {
            harness,
            socket,
            initialize: Default::default(),
            closed: false,
        };

        let event = InitializeConversationEvent {
            config: Some(options.config().clone()),
        };
        client.send_json(&event).await?;

        // The harness answers the initialize event before anything else. If it
        // instead drops the socket, the reason is on stderr — most often "no
        // model configured", which it treats as fatal.
        match client.next_event().await? {
            Some(event) => match event.into_event() {
                Some(OutputEventEvent::InitializeConversationResponse(response)) => {
                    client.initialize = response;
                }
                other => {
                    return Err(Error::HandshakeFailed {
                        stderr: format!(
                            "expected an initialize response, got {other:?}; harness stderr: {}",
                            client.harness.stderr_tail()
                        ),
                    })
                }
            },
            None => {
                return Err(Error::HandshakeFailed {
                    stderr: format!(
                        "harness closed the socket during initialize; stderr: {}",
                        client.harness.stderr_tail()
                    ),
                })
            }
        }

        Ok(client)
    }

    /// The harness's reply to initialize: the conversation id, any replayed
    /// history, and cumulative usage for a resumed session.
    pub fn initialize_response(&self) -> &InitializeConversationResponse {
        &self.initialize
    }

    /// The conversation id, which the harness calls a "cascade id".
    pub fn cascade_id(&self) -> Option<&str> {
        self.initialize.cascade_id.as_deref()
    }

    /// The running process, for its stderr tail and port.
    pub fn harness(&self) -> &Harness {
        &self.harness
    }

    /// Sends one frame.
    pub async fn send(&mut self, event: &InputEvent) -> Result<()> {
        self.send_json(event).await
    }

    async fn send_json<T: serde::Serialize>(&mut self, value: &T) -> Result<()> {
        if self.closed {
            return Err(Error::SessionClosed);
        }
        let payload = serde_json::to_string(value).map_err(|e| Error::decode(e, ""))?;
        log::trace!("--> {payload}");
        self.socket.send(Message::Text(payload)).await?;
        Ok(())
    }

    /// Reads the next frame, or `None` once the harness has closed the socket.
    ///
    /// Non-text frames (pings, pongs, the close frame itself) are handled and
    /// skipped rather than surfaced.
    pub async fn next_event(&mut self) -> Result<Option<OutputEvent>> {
        loop {
            let message = match self.socket.next().await {
                Some(Ok(message)) => message,
                Some(Err(e)) => {
                    self.closed = true;
                    // A harness that dies mid-turn drops the TCP connection
                    // without a close frame, which tungstenite reports as a
                    // protocol violation. The useful diagnosis is on stderr.
                    let stderr = self.harness.stderr_tail();
                    if stderr.is_empty() {
                        return Err(Error::from(e));
                    }
                    return Err(Error::HandshakeFailed { stderr });
                }
                None => {
                    self.closed = true;
                    return Ok(None);
                }
            };

            match message {
                Message::Text(text) => {
                    log::trace!("<-- {text}");
                    let event = serde_json::from_str::<OutputEvent>(&text)
                        .map_err(|e| Error::decode(e, text))?;
                    return Ok(Some(event));
                }
                Message::Close(_) => {
                    self.closed = true;
                    return Ok(None);
                }
                Message::Binary(_) | Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {
                    continue
                }
            }
        }
    }

    /// True once the socket has closed in either direction.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// Asks the harness to end the session, waits for it to acknowledge, then
    /// stops the process.
    ///
    /// The acknowledgement matters: the harness flushes conversation state to
    /// its storage directory on the way out, so killing it early loses the
    /// transcript a later `cascade_id` resume would have replayed.
    pub async fn shutdown(mut self) -> Result<()> {
        if !self.closed {
            let _ = self.send(&InputEvent::session_end()).await;
            loop {
                match self.next_event().await {
                    Ok(Some(event)) => {
                        if event.session_end_response == Some(true) {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
            let _ = self.socket.close(None).await;
        }
        self.harness.shutdown().await
    }
}
