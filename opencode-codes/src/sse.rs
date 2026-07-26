//! Server-Sent Events stream handling for `GET /event`.
//!
//! opencode publishes all server activity — incremental message parts, tool
//! calls, permission requests, session lifecycle — on a single long-lived SSE
//! stream at `GET /event`. This module wraps that stream, decoding each `data:`
//! frame into the generated [`Event`] union and exposing the result as an async
//! [`Stream`] (and an inherent [`EventStream::next`] for consumers that would
//! rather not pull in `futures`/`tokio-stream`).
//!
//! # Endpoints
//!
//! - [`EVENT_PATH`] (`/event`) — events scoped to the server's working
//!   directory. Accepts optional `directory` and `workspace` query parameters;
//!   attach them with [`EventStream::from_request`] if you need them.
//! - [`GLOBAL_EVENT_PATH`] (`/global/event`) — events from *every* directory the
//!   opencode process manages, each wrapped in a
//!   [`GlobalEvent`](crate::protocol_generated::types::GlobalEvent) envelope that
//!   carries `directory`/`workspace` context. That envelope is a different wire
//!   shape than the bare [`Event`] this stream decodes, so global frames arrive
//!   as [`StreamEvent::Unknown`] here; decode them yourself with
//!   `serde_json::from_value::<GlobalEvent>` off the raw [`serde_json::Value`].
//!
//! # Reconnection
//!
//! The underlying [`reqwest_eventsource::EventSource`] reconnects on its own with
//! a capped exponential backoff (configurable via [`RetryConfig`]) and replays
//! the `Last-Event-ID` header so the server can resume. Each successful (re)open
//! surfaces as [`StreamEvent::Connected`]. Transient failures that the backoff
//! will retry are swallowed; the stream yields an `Err` only when the retry
//! budget is exhausted ([`RetryConfig::max_retries`]), after which it ends.
//!
//! Because [`EventStream::connect`] performs no network I/O — it only prepares
//! the request — it returns `Ok` even against an unreachable or permanently-dead
//! server. The *initial* connection failure (connection refused, DNS error) is
//! treated as just another transient error: with the default
//! [`RetryConfig::max_retries`] of `None` it is retried forever and never
//! surfaces. A naive `while let Some(ev) = stream.next().await` loop against a
//! server that never comes up therefore blocks indefinitely, yielding neither an
//! event nor an `Err`. Guard against this by giving [`RetryConfig::max_retries`]
//! a bound (so the stream ends with an `Err` once the budget is spent) and/or by
//! keeping the periodic-poll safety net shown below, which stays live regardless
//! of the stream's connection state.
//!
//! # Reconcile, don't trust
//!
//! SSE is best-effort: frames can be dropped across a reconnect, and the window
//! between your prompt landing and your subscription opening is not covered by
//! the stream. Treat [`GET /session/{sessionID}/message`](crate::http) as the
//! source of truth and the stream as a low-latency hint. The idiomatic shape is
//! to `select!` the event stream against a periodic poll, and to force a poll
//! every time [`StreamEvent::Connected`] arrives (a reconnect means you may have
//! missed frames):
//!
//! ```ignore
//! use opencode_codes::sse::{EventStream, StreamEvent};
//!
//! let mut events = EventStream::connect("http://127.0.0.1:41999")?;
//! let mut poll = tokio::time::interval(std::time::Duration::from_secs(2));
//!
//! loop {
//!     tokio::select! {
//!         item = events.next() => match item {
//!             Some(Ok(StreamEvent::Connected)) => {
//!                 // (Re)connected — the stream may have gaps; reconcile now.
//!                 reconcile(&client, &session_id).await?;
//!             }
//!             Some(Ok(StreamEvent::Event(ev))) => handle(ev),
//!             Some(Ok(StreamEvent::Unknown(raw))) => log_unknown(raw),
//!             Some(Err(e)) => return Err(e),   // retry budget exhausted
//!             None => break,                   // stream closed
//!         },
//!         _ = poll.tick() => {
//!             // Periodic safety net independent of the stream.
//!             reconcile(&client, &session_id).await?;
//!         }
//!     }
//! }
//! ```

use crate::error::{Error, Result};
use crate::protocol_generated::types::Event;
use futures_core::Stream;
use reqwest::{Client, RequestBuilder};
use reqwest_eventsource::retry::ExponentialBackoff;
use reqwest_eventsource::{Event as EsEvent, EventSource, ReadyState};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

/// Path of the directory-scoped event stream (`GET /event`).
pub const EVENT_PATH: &str = "/event";

/// Path of the cross-directory event stream (`GET /global/event`).
pub const GLOBAL_EVENT_PATH: &str = "/global/event";

/// Reconnection backoff for the SSE stream.
///
/// Maps directly onto [`reqwest_eventsource`]'s exponential backoff retry
/// policy: the first reconnect waits [`initial_interval`](Self::initial_interval),
/// each subsequent attempt multiplies by [`factor`](Self::factor), and the delay
/// is clamped to [`max_interval`](Self::max_interval). The backoff resets after
/// any successful reopen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetryConfig {
    /// Delay before the first reconnect attempt.
    pub initial_interval: Duration,
    /// Ceiling applied to every reconnect delay.
    pub max_interval: Duration,
    /// Multiplier applied to the delay after each failed attempt.
    pub factor: f64,
    /// Maximum number of consecutive reconnect attempts before the stream ends.
    ///
    /// `None` (the default) retries indefinitely, which is what a long-lived
    /// subscription usually wants. Note that the *initial* connect counts as a
    /// reconnect here: with `None`, a server that is unreachable from the start
    /// is retried forever, so the stream never yields an `Err` and a bare
    /// `next()` loop hangs. Set a bound if you need an unreachable server to
    /// surface as a terminal `Err` rather than silence.
    pub max_retries: Option<usize>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            initial_interval: Duration::from_millis(500),
            max_interval: Duration::from_secs(30),
            factor: 2.0,
            max_retries: None,
        }
    }
}

impl RetryConfig {
    fn into_policy(self) -> ExponentialBackoff {
        ExponentialBackoff::new(
            self.initial_interval,
            self.factor,
            Some(self.max_interval),
            self.max_retries,
        )
    }
}

/// A single item yielded by an [`EventStream`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum StreamEvent {
    /// The SSE connection opened, or reopened after a reconnect.
    ///
    /// Because a reconnect can straddle dropped frames, treat every `Connected`
    /// as a cue to reconcile against `GET /session/{sessionID}/message`.
    Connected,
    /// A frame that decoded cleanly into the generated event union.
    ///
    /// Boxed because [`Event`] is a large union; keeping the variant small keeps
    /// `StreamEvent` (and `Result<StreamEvent>`) cheap to move.
    Event(Box<Event>),
    /// A frame whose `type` matched no known [`Event`] variant (or whose payload
    /// did not fit the expected shape).
    ///
    /// The raw JSON is preserved so callers can inspect newer server events
    /// without this crate erroring out. Only genuinely non-JSON frames surface
    /// as an `Err` instead.
    Unknown(serde_json::Value),
}

impl StreamEvent {
    /// The decoded [`Event`], if this item is [`StreamEvent::Event`].
    #[must_use]
    pub fn as_event(&self) -> Option<&Event> {
        match self {
            Self::Event(ev) => Some(ev.as_ref()),
            _ => None,
        }
    }

    /// Whether this item marks a fresh (re)connection.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected)
    }
}

/// A decoded, self-reconnecting subscription to an opencode event stream.
///
/// Construct one with [`EventStream::connect`] (or [`EventStream::from_request`]
/// when you need custom headers, query parameters, or authentication) and pull
/// items with [`EventStream::next`] or via its [`Stream`] implementation.
pub struct EventStream {
    inner: EventSource,
}

impl EventStream {
    /// Subscribe to `GET {base_url}/event` with the default [`RetryConfig`],
    /// against a **no-auth** server.
    ///
    /// No credentials are attached and the environment is never consulted. For
    /// an authenticated server, prefer
    /// [`OpencodeClient::event_stream`](crate::client_async::OpencodeClient::event_stream),
    /// which reuses the client's configured auth, or build the request yourself
    /// and use [`EventStream::from_request`].
    ///
    /// This prepares the request but performs no network I/O, so it returns `Ok`
    /// even if the server is down. With the default [`RetryConfig`]
    /// ([`max_retries`](RetryConfig::max_retries) = `None`) an unreachable server
    /// is retried forever and never surfaces an `Err`; see the [module
    /// docs](self#reconnection) for how to avoid a silent hang.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if the request cannot be prepared for streaming.
    pub fn connect(base_url: &str) -> Result<Self> {
        Self::connect_with(
            &Client::builder().build()?,
            base_url,
            RetryConfig::default(),
        )
    }

    /// Subscribe to `GET {base_url}/event` on a caller-supplied [`Client`] with
    /// an explicit [`RetryConfig`], reusing the client's connection pool.
    ///
    /// Like [`EventStream::connect`], this attaches no auth and never reads the
    /// environment. Consistent with the REST transport, `OPENCODE_SERVER_PASSWORD`
    /// is only consulted through an explicit opt-in
    /// ([`BasicAuth::from_env`](crate::http::BasicAuth::from_env) /
    /// [`OpencodeClientBuilder::auth_from_env`](crate::client_async::OpencodeClientBuilder::auth_from_env)),
    /// not on this connect path.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if the request cannot be prepared for streaming.
    pub fn connect_with(client: &Client, base_url: &str, retry: RetryConfig) -> Result<Self> {
        let url = format!("{}{EVENT_PATH}", base_url.trim_end_matches('/'));
        Self::from_request(client.get(url), retry)
    }

    /// Wrap a fully-prepared [`RequestBuilder`] (with whatever headers, query
    /// parameters, or auth you need) as a decoded event stream.
    ///
    /// The request should target an SSE endpoint — [`EVENT_PATH`] or
    /// [`GLOBAL_EVENT_PATH`]. The `Accept: text/event-stream` header is added for
    /// you.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if the request cannot be cloned for reconnects (only
    /// possible for requests carrying a non-replayable streaming body, which the
    /// `GET` event endpoints never do).
    pub fn from_request(builder: RequestBuilder, retry: RetryConfig) -> Result<Self> {
        let mut source = EventSource::new(builder).map_err(|e| Error::Http {
            status: 0,
            body: format!("cannot prepare SSE request: {e}"),
        })?;
        source.set_retry_policy(Box::new(retry.into_policy()));
        Ok(Self { inner: source })
    }

    /// Await the next decoded item, or `None` once the stream has closed.
    ///
    /// This is the [`Stream`] contract as an inherent async method so callers
    /// need no `StreamExt` import.
    pub async fn next(&mut self) -> Option<Result<StreamEvent>> {
        std::future::poll_fn(|cx| Pin::new(&mut *self).poll_next(cx)).await
    }

    /// The current connection state of the underlying event source.
    #[must_use]
    pub fn ready_state(&self) -> ReadyState {
        self.inner.ready_state()
    }

    /// Stop the stream and its reconnection attempts. The next poll yields
    /// `None`. Dropping the [`EventStream`] does the same.
    pub fn close(&mut self) {
        self.inner.close();
    }
}

fn decode_frame(data: &str) -> Result<StreamEvent> {
    let value: serde_json::Value = serde_json::from_str(data)?;
    match serde_json::from_value::<Event>(value.clone()) {
        Ok(event) => Ok(StreamEvent::Event(Box::new(event))),
        Err(_) => Ok(StreamEvent::Unknown(value)),
    }
}

impl Stream for EventStream {
    type Item = Result<StreamEvent>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            match Pin::new(&mut this.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(EsEvent::Open))) => {
                    return Poll::Ready(Some(Ok(StreamEvent::Connected)));
                }
                Poll::Ready(Some(Ok(EsEvent::Message(message)))) => {
                    return Poll::Ready(Some(decode_frame(&message.data)));
                }
                Poll::Ready(Some(Err(e))) => {
                    if this.inner.ready_state() == ReadyState::Closed {
                        return Poll::Ready(Some(Err(e.into())));
                    }
                    // Transient: the EventSource has scheduled a backoff retry.
                    // Poll again so its delay registers a waker (yielding
                    // Pending) instead of surfacing reconnect churn to callers.
                    continue;
                }
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_config_default_is_capped_and_unbounded() {
        let cfg = RetryConfig::default();
        assert_eq!(cfg.initial_interval, Duration::from_millis(500));
        assert_eq!(cfg.max_interval, Duration::from_secs(30));
        assert_eq!(cfg.max_retries, None);
    }

    #[test]
    fn decode_known_event_frame() {
        let frame = r#"{"type":"session.idle","id":"evt_1","properties":{"sessionID":"ses_123"}}"#;
        let decoded = decode_frame(frame).expect("valid json");
        match decoded {
            StreamEvent::Event(ev) if matches!(*ev, Event::SessionIdle(_)) => {}
            other => panic!("expected SessionIdle event, got {other:?}"),
        }
    }

    #[test]
    fn decode_unknown_type_becomes_unknown_variant() {
        let frame = r#"{"type":"totally.new.event.from.the.future","properties":{"x":1}}"#;
        let decoded = decode_frame(frame).expect("valid json");
        match decoded {
            StreamEvent::Unknown(value) => {
                assert_eq!(value["type"], "totally.new.event.from.the.future");
            }
            other => panic!("expected Unknown, got {other:?}"),
        }
    }

    #[test]
    fn decode_known_type_wrong_shape_becomes_unknown_not_error() {
        // Recognized discriminant, but the payload does not match the variant.
        let frame = r#"{"type":"session.idle","id":"evt_1","properties":{"sessionID":42}}"#;
        let decoded = decode_frame(frame).expect("valid json");
        assert!(matches!(decoded, StreamEvent::Unknown(_)));
    }

    #[test]
    fn decode_non_json_frame_is_error() {
        assert!(decode_frame("not json at all").is_err());
    }

    #[test]
    fn stream_event_accessors() {
        assert!(StreamEvent::Connected.is_connected());
        assert!(StreamEvent::Connected.as_event().is_none());
        let ev =
            decode_frame(r#"{"type":"session.idle","id":"evt_1","properties":{"sessionID":"s"}}"#)
                .unwrap();
        assert!(ev.as_event().is_some());
        assert!(!ev.is_connected());
    }

    // Live tests: require a running opencode server. Run with:
    //   cargo test -p opencode-codes --features integration-tests -- --nocapture
    #[cfg(feature = "integration-tests")]
    mod live {
        use super::*;

        const BASE_URL: &str = "http://127.0.0.1:41999";

        async fn next_item(stream: &mut EventStream) -> Option<Result<StreamEvent>> {
            stream.next().await
        }

        #[tokio::test]
        async fn connects_and_receives_real_frames() {
            let client = Client::new();
            let mut stream =
                EventStream::connect_with(&client, BASE_URL, RetryConfig::default()).unwrap();

            // First item off a healthy server is the connection-open marker.
            let first = tokio::time::timeout(Duration::from_secs(5), next_item(&mut stream))
                .await
                .expect("server did not open the SSE stream in time")
                .expect("stream closed immediately")
                .expect("connection error");
            assert!(first.is_connected(), "expected Connected, got {first:?}");

            // Trigger activity so at least one event flows.
            let created = client
                .post(format!("{BASE_URL}/session"))
                .json(&serde_json::json!({}))
                .send()
                .await
                .expect("create session request failed");
            assert!(
                created.status().is_success(),
                "POST /session failed: {}",
                created.status()
            );

            let mut saw_event = false;
            for _ in 0..50 {
                match tokio::time::timeout(Duration::from_secs(10), next_item(&mut stream)).await {
                    Ok(Some(Ok(StreamEvent::Event(_) | StreamEvent::Unknown(_)))) => {
                        saw_event = true;
                        break;
                    }
                    Ok(Some(Ok(StreamEvent::Connected))) => continue,
                    Ok(Some(Err(e))) => panic!("stream error: {e}"),
                    Ok(None) => panic!("stream closed unexpectedly"),
                    Err(_) => panic!("timed out waiting for an event frame"),
                }
            }
            assert!(saw_event, "no event frames observed after POST /session");
        }

        #[tokio::test]
        async fn event_stream_is_a_futures_stream() {
            fn assert_stream<S: Stream>(_: &S) {}
            let stream = EventStream::connect(BASE_URL).unwrap();
            assert_stream(&stream);
        }
    }
}
