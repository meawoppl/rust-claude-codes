//! Low-level REST transport for the opencode server.
//!
//! This module owns three concerns shared by every hand-wrapped endpoint:
//!
//! - **Base-URL handling** — a normalized origin (trailing slashes stripped)
//!   against which typed path builders produce absolute request URLs.
//! - **HTTP Basic auth** — opencode requires Basic auth only when the server was
//!   started with a password (username defaults to `"opencode"`). Credentials are
//!   supplied explicitly; the `OPENCODE_SERVER_PASSWORD` environment variable is
//!   read only through [`BasicAuth::from_env`], never silently on a request path.
//! - **A typed request helper** — [`HttpTransport::request_json`] /
//!   [`HttpTransport::request_unit`] send a JSON body (if any), apply auth and
//!   the optional per-request timeout, and map any non-2xx status to
//!   [`Error::Http`] carrying the server's response body.
//!
//! Path parameters (`{sessionID}`, `{permissionID}`) are percent-encoded against
//! the RFC 3986 unreserved set before being placed into the path.

use std::time::Duration;

use reqwest::{Client, Method, RequestBuilder};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{Error, Result};

/// Default HTTP Basic auth username opencode expects when a server password is
/// configured.
pub const DEFAULT_USERNAME: &str = "opencode";

/// Environment variable opencode reads for its server password. Used only by
/// [`BasicAuth::from_env`].
pub const SERVER_PASSWORD_ENV: &str = "OPENCODE_SERVER_PASSWORD";

/// HTTP Basic credentials for the opencode server.
///
/// opencode uses Basic auth only when it was launched with a password; against a
/// no-auth server the transport is constructed without any [`BasicAuth`].
#[derive(Clone, Debug)]
pub struct BasicAuth {
    /// Basic auth username. Defaults to [`DEFAULT_USERNAME`] in
    /// [`BasicAuth::from_env`].
    pub username: String,
    /// Basic auth password (the value opencode was started with).
    pub password: String,
}

impl BasicAuth {
    /// Construct credentials from an explicit username and password.
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }

    /// Build credentials from `OPENCODE_SERVER_PASSWORD`, using [`DEFAULT_USERNAME`]
    /// as the username.
    ///
    /// Returns `None` when the variable is unset or empty. This is the *only*
    /// place the crate consults the environment for auth; request paths never do.
    pub fn from_env() -> Option<Self> {
        std::env::var(SERVER_PASSWORD_ENV)
            .ok()
            .filter(|p| !p.is_empty())
            .map(|password| Self {
                username: DEFAULT_USERNAME.to_string(),
                password,
            })
    }
}

/// Directory / workspace scoping for the session endpoints.
///
/// `opencode serve` can manage several project directories at once, and every
/// session endpoint (create, prompt, message, abort, permissions) plus the
/// `GET /event` stream accepts optional `directory` and `workspace` query
/// parameters that pin the call to one of them. A [`Scope`] carries those
/// values; an empty scope (the default) leaves the server on its own working
/// directory.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scope {
    /// The `directory` query parameter — an absolute project directory path.
    pub directory: Option<String>,
    /// The `workspace` query parameter — a workspace identifier.
    pub workspace: Option<String>,
}

impl Scope {
    /// Whether neither `directory` nor `workspace` is set (no scoping applied).
    pub fn is_empty(&self) -> bool {
        self.directory.is_none() && self.workspace.is_none()
    }
}

/// Append `key=value` to `url`, using `*sep` (`?` or `&`) and advancing it.
fn push_query(url: &mut String, sep: &mut char, key: &str, value: &str) {
    url.push(*sep);
    url.push_str(key);
    url.push('=');
    url.push_str(&encode_segment(value));
    *sep = '&';
}

/// Percent-encode a single path segment against the RFC 3986 unreserved set
/// (`ALPHA / DIGIT / "-" / "." / "_" / "~"`); every other byte becomes `%XX`.
fn encode_segment(segment: &str) -> String {
    const UPPER_HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(segment.len());
    for &byte in segment.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char);
            }
            _ => {
                out.push('%');
                out.push(UPPER_HEX[(byte >> 4) as usize] as char);
                out.push(UPPER_HEX[(byte & 0xf) as usize] as char);
            }
        }
    }
    out
}

/// Low-level JSON-over-HTTP transport bound to one opencode base URL.
///
/// Cloning is cheap: the inner [`reqwest::Client`] is reference-counted.
#[derive(Clone, Debug)]
pub struct HttpTransport {
    client: Client,
    base_url: String,
    timeout: Option<Duration>,
    auth: Option<BasicAuth>,
    scope: Scope,
}

impl HttpTransport {
    /// Bind a transport to a base URL.
    ///
    /// Trailing slashes on `base_url` are stripped so path builders can append
    /// `/`-prefixed segments without producing `//`. The URL itself is validated
    /// lazily by reqwest at send time; a malformed base surfaces as
    /// [`Error::Transport`].
    pub fn new(
        client: Client,
        base_url: impl Into<String>,
        timeout: Option<Duration>,
        auth: Option<BasicAuth>,
        scope: Scope,
    ) -> Self {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        Self {
            client,
            base_url,
            timeout,
            auth,
            scope,
        }
    }

    /// The normalized base URL (no trailing slash).
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Credentials attached to every request, if any.
    pub fn auth(&self) -> Option<&BasicAuth> {
        self.auth.as_ref()
    }

    /// The `directory` / `workspace` scoping applied to every session URL and
    /// the `GET /event` stream this transport builds.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    /// Append the configured [`Scope`] query parameters to `url`, given the
    /// current separator (`?` when `url` carries no query yet, else `&`).
    fn append_scope(&self, url: &mut String, sep: &mut char) {
        if let Some(directory) = &self.scope.directory {
            push_query(url, sep, "directory", directory);
        }
        if let Some(workspace) = &self.scope.workspace {
            push_query(url, sep, "workspace", workspace);
        }
    }

    /// Join an arbitrary path (leading slash optional) onto the base URL. Backing
    /// helper for the client's raw escape hatch; the path is used verbatim (not
    /// segment-encoded).
    pub fn join(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    /// URL for `POST /session` (session creation).
    pub fn session_create_url(&self) -> String {
        let mut url = format!("{}/session", self.base_url);
        let mut sep = '?';
        self.append_scope(&mut url, &mut sep);
        url
    }

    /// URL for `POST /session/{sessionID}/prompt_async`.
    pub fn prompt_async_url(&self, session_id: &str) -> String {
        let mut url = format!(
            "{}/session/{}/prompt_async",
            self.base_url,
            encode_segment(session_id)
        );
        let mut sep = '?';
        self.append_scope(&mut url, &mut sep);
        url
    }

    /// URL for `GET /session/{sessionID}/message`, optionally paginated with the
    /// `limit` and `before` query parameters used for reconciliation polling.
    pub fn messages_url(
        &self,
        session_id: &str,
        limit: Option<u64>,
        before: Option<&str>,
    ) -> String {
        let mut url = format!(
            "{}/session/{}/message",
            self.base_url,
            encode_segment(session_id)
        );
        let mut sep = '?';
        if let Some(limit) = limit {
            push_query(&mut url, &mut sep, "limit", &limit.to_string());
        }
        if let Some(before) = before {
            push_query(&mut url, &mut sep, "before", before);
        }
        self.append_scope(&mut url, &mut sep);
        url
    }

    /// URL for `POST /session/{sessionID}/abort`.
    pub fn abort_url(&self, session_id: &str) -> String {
        let mut url = format!(
            "{}/session/{}/abort",
            self.base_url,
            encode_segment(session_id)
        );
        let mut sep = '?';
        self.append_scope(&mut url, &mut sep);
        url
    }

    /// URL for `POST /session/{sessionID}/fork`.
    pub fn fork_url(&self, session_id: &str) -> String {
        let mut url = format!(
            "{}/session/{}/fork",
            self.base_url,
            encode_segment(session_id)
        );
        let mut sep = '?';
        self.append_scope(&mut url, &mut sep);
        url
    }

    /// URL for `POST /session/{sessionID}/permissions/{permissionID}`.
    pub fn permission_url(&self, session_id: &str, permission_id: &str) -> String {
        let mut url = format!(
            "{}/session/{}/permissions/{}",
            self.base_url,
            encode_segment(session_id),
            encode_segment(permission_id)
        );
        let mut sep = '?';
        self.append_scope(&mut url, &mut sep);
        url
    }

    /// URL for the `GET /event` SSE stream, with any configured [`Scope`]
    /// applied as `directory` / `workspace` query parameters.
    pub fn event_url(&self) -> String {
        let mut url = format!("{}/event", self.base_url);
        let mut sep = '?';
        self.append_scope(&mut url, &mut sep);
        url
    }

    /// A [`RequestBuilder`] for the `GET /event` SSE stream on this transport's
    /// [`Client`], with the configured base URL, [`Scope`], and auth applied.
    ///
    /// Hand it to [`crate::sse::EventStream::from_request`] to open an
    /// authenticated event stream that reuses this transport's connection pool.
    /// No per-request timeout is applied, since the stream is long-lived.
    pub fn event_request(&self) -> RequestBuilder {
        let mut builder = self.client.get(self.event_url());
        if let Some(auth) = &self.auth {
            builder = builder.basic_auth(&auth.username, Some(&auth.password));
        }
        builder
    }

    /// Send a request and return the raw response body on 2xx, mapping any other
    /// status to [`Error::Http`].
    async fn send(&self, method: Method, url: &str, body: Option<Value>) -> Result<String> {
        let mut builder = self.client.request(method, url);
        if let Some(timeout) = self.timeout {
            builder = builder.timeout(timeout);
        }
        if let Some(auth) = &self.auth {
            builder = builder.basic_auth(&auth.username, Some(&auth.password));
        }
        if let Some(body) = body {
            builder = builder.json(&body);
        }
        let response = builder.send().await?;
        let status = response.status();
        let text = response.text().await?;
        if status.is_success() {
            Ok(text)
        } else {
            Err(Error::Http {
                status: status.as_u16(),
                body: text,
            })
        }
    }

    /// Send a request and deserialize the JSON response body into `R`.
    pub async fn request_json<R: DeserializeOwned>(
        &self,
        method: Method,
        url: &str,
        body: Option<Value>,
    ) -> Result<R> {
        let text = self.send(method, url, body).await?;
        Ok(serde_json::from_str(&text)?)
    }

    /// Send a request that yields no response body (e.g. an HTTP 204), discarding
    /// the body on success.
    pub async fn request_unit(&self, method: Method, url: &str, body: Option<Value>) -> Result<()> {
        self.send(method, url, body).await?;
        Ok(())
    }
}
