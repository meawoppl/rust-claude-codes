//! The loopback WebSocket the harness serves once it has handshaked.

use std::time::Duration;

use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::error::{Error, Result};

/// The socket type the rest of the crate passes around.
pub(crate) type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// The header the harness authenticates the upgrade with.
const API_KEY_HEADER: &str = "x-goog-api-key";

/// How many times to retry before giving up on the port.
const MAX_ATTEMPTS: u32 = 5;

/// Connects to the harness, retrying with exponential backoff.
///
/// The harness prints its port *before* it finishes binding, so the first
/// attempt often loses the race. Both loopback spellings are tried on every
/// pass: some environments resolve `localhost` to an address the harness did
/// not bind, and others have no `localhost` entry at all.
pub(crate) async fn connect(port: u16, api_key: &str) -> Result<Socket> {
    let key = HeaderValue::from_str(api_key).map_err(|_| Error::HandshakeFailed {
        stderr: "harness returned an API key that is not a valid header value".into(),
    })?;

    for attempt in 0..MAX_ATTEMPTS {
        for host in ["127.0.0.1", "localhost"] {
            let url = format!("ws://{host}:{port}/");
            let mut request = match url.as_str().into_client_request() {
                Ok(request) => request,
                Err(e) => return Err(Error::from(e)),
            };
            request.headers_mut().insert(API_KEY_HEADER, key.clone());

            match tokio_tungstenite::connect_async(request).await {
                Ok((socket, _response)) => {
                    log::debug!("connected to harness at {url}");
                    return Ok(socket);
                }
                Err(e) => log::debug!("attempt {attempt} to {url} failed: {e}"),
            }
        }
        tokio::time::sleep(Duration::from_millis(100 * 2u64.pow(attempt))).await;
    }

    Err(Error::WebSocketUnreachable {
        port,
        attempts: MAX_ATTEMPTS,
    })
}
