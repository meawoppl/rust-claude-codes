//! High-level async client for the opencode server.
//!
//! [`OpencodeClient`] wraps the low-level [`crate::http::HttpTransport`] with
//! typed methods for the six hand-wrapped endpoints. Construct one with
//! [`OpencodeClient::builder`]:
//!
//! ```rust,ignore
//! use opencode_codes::client_async::OpencodeClient;
//! use opencode_codes::protocol_generated::types::SessionCreateParams;
//!
//! # async fn demo() -> opencode_codes::Result<()> {
//! let client = OpencodeClient::builder()
//!     .base_url("http://127.0.0.1:4096")
//!     .build()?;
//! let session = client.create_session(&SessionCreateParams {
//!     title: Some("demo".into()),
//!     ..Default::default()
//! }).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Reconciliation
//!
//! The `GET /event` SSE stream (see [`crate::sse`]) is best-effort and must not
//! be trusted as the sole source of truth. After observing activity on the
//! stream, poll [`OpencodeClient::list_messages`] to reconcile against the
//! server's authoritative message state.

use std::time::Duration;

use reqwest::{Client, Method};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::Result;
use crate::http::{BasicAuth, HttpTransport, Scope};
use crate::protocol_generated::types::{
    MessageWithParts, PermissionReplyParams, PromptAsyncParams, Session, SessionCreateParams,
};
use crate::sse::{EventStream, RetryConfig};

/// Base URL of a default local `opencode serve` instance.
pub const DEFAULT_BASE_URL: &str = "http://127.0.0.1:4096";

/// Async client for the opencode HTTP/SSE server.
///
/// Cloning is cheap; the underlying [`reqwest::Client`] is shared.
#[derive(Clone, Debug)]
pub struct OpencodeClient {
    transport: HttpTransport,
}

impl OpencodeClient {
    /// Start building a client. Equivalent to [`OpencodeClientBuilder::new`].
    pub fn builder() -> OpencodeClientBuilder {
        OpencodeClientBuilder::new()
    }

    /// The underlying transport, exposing base-URL, auth, and the `GET /event`
    /// URL for an SSE subscriber.
    pub fn transport(&self) -> &HttpTransport {
        &self.transport
    }

    /// Open the `GET /event` SSE stream, reusing this client's base URL, auth,
    /// directory/workspace scope, and `reqwest` connection pool.
    ///
    /// This is the ergonomic counterpart to [`OpencodeClient::list_messages`]:
    /// the same client drives both the low-latency event stream and the
    /// authoritative reconciliation poll, so credentials configured with
    /// [`OpencodeClientBuilder::auth`] flow to the stream without a detour
    /// through the `OPENCODE_SERVER_PASSWORD` environment variable.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error`] if the request cannot be prepared for streaming.
    pub fn event_stream(&self, retry: RetryConfig) -> Result<EventStream> {
        EventStream::from_request(self.transport.event_request(), retry)
    }

    /// Create a new session — `POST /session`.
    ///
    /// The response is the freshly created [`Session`]; its `id` (a `ses…`
    /// string) is used to address every subsequent per-session call.
    pub async fn create_session(&self, params: &SessionCreateParams) -> Result<Session> {
        let body = serde_json::to_value(params)?;
        self.transport
            .request_json(
                Method::POST,
                &self.transport.session_create_url(),
                Some(body),
            )
            .await
    }

    /// Submit a prompt — `POST /session/{sessionID}/prompt_async`.
    ///
    /// Returns as soon as the server accepts the work (HTTP 204); the agent's
    /// output is observed on the `GET /event` SSE stream and reconciled via
    /// [`OpencodeClient::list_messages`].
    pub async fn prompt_async(&self, session_id: &str, params: &PromptAsyncParams) -> Result<()> {
        let body = serde_json::to_value(params)?;
        self.transport
            .request_unit(
                Method::POST,
                &self.transport.prompt_async_url(session_id),
                Some(body),
            )
            .await
    }

    /// List a session's messages — `GET /session/{sessionID}/message`.
    ///
    /// Returns every message with its parts. This is the authoritative
    /// reconciliation path for the best-effort SSE stream. For pagination use
    /// [`OpencodeClient::list_messages_page`].
    pub async fn list_messages(&self, session_id: &str) -> Result<Vec<MessageWithParts>> {
        self.list_messages_page(session_id, None, None).await
    }

    /// Paginated variant of [`OpencodeClient::list_messages`].
    ///
    /// `limit` caps the number of messages returned; `before` is a message id
    /// cursor (results strictly older than it) for walking history backwards.
    pub async fn list_messages_page(
        &self,
        session_id: &str,
        limit: Option<u64>,
        before: Option<&str>,
    ) -> Result<Vec<MessageWithParts>> {
        self.transport
            .request_json(
                Method::GET,
                &self.transport.messages_url(session_id, limit, before),
                None,
            )
            .await
    }

    /// Abort in-flight work — `POST /session/{sessionID}/abort`.
    ///
    /// Returns `true` when the session had work that was aborted.
    pub async fn abort(&self, session_id: &str) -> Result<bool> {
        self.transport
            .request_json(Method::POST, &self.transport.abort_url(session_id), None)
            .await
    }

    /// Reply to a permission request —
    /// `POST /session/{sessionID}/permissions/{permissionID}`.
    ///
    /// # Deprecation
    ///
    /// In the 1.18.5 spec this route (operation `permission.respond`) is marked
    /// **deprecated** in favor of the newer reply endpoints
    /// `POST /permission/{requestID}/reply` (operation `permission.reply`) and
    /// `POST /api/session/{sessionID}/permission/{requestID}/reply` (operation
    /// `v2.session.permission.reply`), neither of which this crate wraps yet.
    /// Reach either through the raw [`OpencodeClient::request`] escape hatch using
    /// those exact paths — note the session-scoped one requires the `/api/`
    /// prefix. This deprecated route remains the reply channel for the
    /// `permission.asked` event and works on 1.18.5; a future opencode release
    /// may remove it.
    ///
    /// # Correlation contract
    ///
    /// Permission handling is deliberately split across two channels and
    /// correlating them is the **consumer's** responsibility:
    ///
    /// 1. A permission *request* arrives on the `GET /event` SSE stream as an
    ///    [`Event::PermissionAsked`](crate::protocol_generated::types::Event::PermissionAsked)
    ///    event (wire type `permission.asked`), whose `properties` carry a `ses…`
    ///    session id and a `per…` permission id. This is the *only* ask event
    ///    that pairs with this call: the coexisting
    ///    [`Event::PermissionV2Asked`](crate::protocol_generated::types::Event::PermissionV2Asked)
    ///    (`permission.v2.asked`) belongs to the unwrapped v2 reply endpoints, so
    ///    do **not** feed its id here.
    /// 2. The *reply* is this separate REST call, addressed by exactly those two
    ///    ids. There is no server-side callback and no implicit pairing: the
    ///    caller must remember which pending `(session_id, permission_id)` a reply
    ///    answers.
    ///
    /// [`PermissionReplyParams::response`] is one of
    /// [`PermissionReplyResponse::Once`](crate::protocol_generated::types::PermissionReplyResponse::Once),
    /// [`Always`](crate::protocol_generated::types::PermissionReplyResponse::Always),
    /// or [`Reject`](crate::protocol_generated::types::PermissionReplyResponse::Reject).
    /// Returns `true` when the reply was accepted; a stale or unknown permission
    /// id yields [`crate::Error::Http`] with status 404.
    pub async fn respond_permission(
        &self,
        session_id: &str,
        permission_id: &str,
        reply: &PermissionReplyParams,
    ) -> Result<bool> {
        let body = serde_json::to_value(reply)?;
        self.transport
            .request_json(
                Method::POST,
                &self.transport.permission_url(session_id, permission_id),
                Some(body),
            )
            .await
    }

    /// Raw escape hatch for endpoints this crate does not hand-wrap.
    ///
    /// `path` is joined onto the configured base URL (leading slash optional) and
    /// used verbatim; `body`, when present, is sent as a JSON request body. The
    /// 2xx response is deserialized into `T`; non-2xx becomes
    /// [`crate::Error::Http`].
    pub async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<T> {
        self.transport
            .request_json(method, &self.transport.join(path), body)
            .await
    }

    /// Raw escape hatch for endpoints that answer with an empty body.
    ///
    /// Identical to [`OpencodeClient::request`] but discards the response body
    /// instead of deserializing it. Many opencode `POST` endpoints reply `204 No
    /// Content` (e.g. `prompt_async` and several unwrapped routes); calling those
    /// through [`OpencodeClient::request`] would fail deserializing the empty
    /// body, so use this variant for them.
    pub async fn request_unit(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<()> {
        self.transport
            .request_unit(method, &self.transport.join(path), body)
            .await
    }
}

/// Builder for [`OpencodeClient`].
#[derive(Clone, Debug)]
pub struct OpencodeClientBuilder {
    base_url: String,
    auth: Option<BasicAuth>,
    timeout: Option<Duration>,
    client: Option<Client>,
    scope: Scope,
}

impl Default for OpencodeClientBuilder {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            auth: None,
            timeout: None,
            client: None,
            scope: Scope::default(),
        }
    }
}

impl OpencodeClientBuilder {
    /// A builder defaulting to [`DEFAULT_BASE_URL`] with no auth or timeout.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the opencode server base URL (e.g. `http://127.0.0.1:4096`).
    #[must_use]
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Attach explicit HTTP Basic credentials.
    #[must_use]
    pub fn auth(mut self, auth: BasicAuth) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Attach HTTP Basic credentials derived from `OPENCODE_SERVER_PASSWORD`
    /// (username `"opencode"`). A no-op when the variable is unset or empty.
    ///
    /// This is the explicit opt-in for reading the environment; the request path
    /// never consults it implicitly.
    #[must_use]
    pub fn auth_from_env(mut self) -> Self {
        if let Some(auth) = BasicAuth::from_env() {
            self.auth = Some(auth);
        }
        self
    }

    /// Apply a per-request timeout to every call the client makes.
    ///
    /// Applied per request, so it also constrains an injected
    /// [`reqwest::Client`] that was built without a timeout.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Inject a pre-configured [`reqwest::Client`] (connection pools, proxies,
    /// custom TLS). When omitted, a default client is built.
    #[must_use]
    pub fn reqwest_client(mut self, client: Client) -> Self {
        self.client = Some(client);
        self
    }

    /// Scope every session endpoint (and the `GET /event` stream) to a project
    /// `directory`.
    ///
    /// `opencode serve` can manage several directories at once; without this the
    /// server uses its own working directory. Set it to target a specific
    /// project on a multi-directory server. Build one client per directory (the
    /// clone is cheap) to drive several concurrently.
    #[must_use]
    pub fn directory(mut self, directory: impl Into<String>) -> Self {
        self.scope.directory = Some(directory.into());
        self
    }

    /// Scope every session endpoint (and the `GET /event` stream) to a
    /// `workspace` identifier. See [`directory`](Self::directory).
    #[must_use]
    pub fn workspace(mut self, workspace: impl Into<String>) -> Self {
        self.scope.workspace = Some(workspace.into());
        self
    }

    /// Set the full directory/workspace [`Scope`] at once.
    #[must_use]
    pub fn scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    /// Build the [`OpencodeClient`].
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Transport`] if a default [`reqwest::Client`] must
    /// be constructed and its builder fails.
    pub fn build(self) -> Result<OpencodeClient> {
        let client = match self.client {
            Some(client) => client,
            None => Client::builder().build()?,
        };
        let transport =
            HttpTransport::new(client, self.base_url, self.timeout, self.auth, self.scope);
        Ok(OpencodeClient { transport })
    }
}

#[cfg(all(test, feature = "integration-tests"))]
mod tests {
    use super::*;
    use crate::protocol_generated::types::{PromptAsyncParamsPartsItem, TextPartInput};

    fn client() -> OpencodeClient {
        let base_url = std::env::var("OPENCODE_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:41999".to_string());
        OpencodeClient::builder()
            .base_url(base_url)
            .auth_from_env()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("client builds")
    }

    #[tokio::test]
    async fn create_list_abort_roundtrip() {
        let client = client();
        let session = client
            .create_session(&SessionCreateParams {
                title: Some("opencode-codes integration probe".into()),
                agent: None,
                metadata: None,
                model: None,
                parent_id: None,
                permission: None,
                workspace_id: None,
            })
            .await
            .expect("create session");
        assert!(session.id.starts_with("ses"));

        let messages = client
            .list_messages(&session.id)
            .await
            .expect("list messages");
        assert!(messages.is_empty());

        let aborted = client.abort(&session.id).await.expect("abort");
        // No work was running, but the endpoint still answers with a boolean.
        let _ = aborted;
    }

    #[tokio::test]
    async fn respond_to_unknown_permission_is_404() {
        let client = client();
        let session = client
            .create_session(&SessionCreateParams {
                title: Some("opencode-codes permission probe".into()),
                agent: None,
                metadata: None,
                model: None,
                parent_id: None,
                permission: None,
                workspace_id: None,
            })
            .await
            .expect("create session");

        let err = client
            .respond_permission(
                &session.id,
                "per_does_not_exist",
                &PermissionReplyParams {
                    response: "reject".into(),
                },
            )
            .await
            .expect_err("stale permission id must fail");
        match err {
            crate::Error::Http { status, .. } => assert_eq!(status, 404),
            other => panic!("expected HTTP 404, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn raw_request_escape_hatch() {
        let client = client();
        let config: Value = client
            .request(Method::GET, "/config", None)
            .await
            .expect("raw GET /config");
        assert!(config.is_object());
    }

    #[test]
    fn prompt_parts_serialize_shape() {
        let params = PromptAsyncParams {
            agent: None,
            format: None,
            message_id: None,
            model: None,
            no_reply: None,
            parts: vec![PromptAsyncParamsPartsItem::Text(TextPartInput {
                id: None,
                ignored: None,
                metadata: None,
                synthetic: None,
                text: "hello".into(),
                time: None,
                type_: String::new(),
            })],
            system: None,
            tools: None,
            variant: None,
        };
        let value = serde_json::to_value(&params).expect("serialize");
        assert_eq!(value["parts"][0]["type"], "text");
        assert_eq!(value["parts"][0]["text"], "hello");
    }
}
