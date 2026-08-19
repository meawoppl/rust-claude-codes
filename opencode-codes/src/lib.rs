//! A typed Rust interface for the [opencode](https://opencode.ai) agent server.
//!
//! <div class="warning">
//!
//! **Maturity warning**: this crate is new and should be considered **highly
//! untested** — it is under active development. The types are generated from
//! opencode's published OpenAPI spec and the suite passes against a live
//! server, but real-world mileage is minimal and the API may change between
//! releases while the surface settles. Bug reports and wire captures that
//! break the types are very welcome at
//! <https://github.com/meawoppl/rust-code-agent-sdks/issues>.
//!
//! </div>
//!
//! Unlike its sibling crates ([`claude-codes`](https://docs.rs/claude-codes) and
//! [`codex-codes`](https://docs.rs/codex-codes)), which wrap CLIs that speak a
//! JSON-Lines protocol over stdio, opencode runs a **local HTTP server** with a
//! **Server-Sent Events** (SSE) stream. This crate models that server's OpenAPI
//! 3.1 wire contract with serde types, and offers an async (Tokio) client plus a
//! managed launcher for the `opencode serve` process.
//!
//! # Architecture
//!
//! opencode exposes a REST + SSE surface:
//!
//! - **REST** — session lifecycle and prompt submission are ordinary JSON
//!   request/response calls (`POST /session`, `POST /session/{sessionID}/prompt_async`,
//!   `GET /session/{sessionID}/message`, `POST /session/{sessionID}/abort`,
//!   `POST /session/{sessionID}/permissions/{permissionID}`).
//! - **SSE** — `GET /event` is a long-lived event stream carrying incremental
//!   message parts, tool activity, and permission requests.
//!
//! The conversation lifecycle is:
//!
//! 1. **Create a session** — `POST /session`.
//! 2. **Subscribe** — open the `GET /event` SSE stream.
//! 3. **Prompt** — `POST /session/{sessionID}/prompt_async` returns immediately;
//!    the agent's work is observed on the SSE stream.
//! 4. **Handle permissions** — a permission request arrives on the stream; the
//!    reply is a *separate* REST call
//!    (`POST /session/{sessionID}/permissions/{permissionID}`). Correlating a
//!    request to its reply is the consumer's responsibility — the pending
//!    permission surface is kept explicit rather than hidden behind a callback.
//! 5. **Reconcile** — SSE is best-effort and must not be trusted alone. Poll
//!    `GET /session/{sessionID}/message` to reconcile the authoritative message
//!    state against what the stream delivered. This crate documents and exposes
//!    that poll path deliberately.
//! 6. **Abort** — `POST /session/{sessionID}/abort` cancels in-flight work.
//!
//! Authentication is HTTP Basic when `OPENCODE_SERVER_PASSWORD` is set; the
//! username defaults to `"opencode"`.
//!
//! # Feature Flags
//!
//! | Feature | Description | WASM-compatible |
//! |---------|-------------|-----------------|
//! | `types` | Core protocol types only (serde) | Yes |
//! | `async-client` | Async HTTP/SSE client using reqwest + tokio | No |
//! | `server` | Managed `opencode serve` launcher (picks a free port) | No |
//! | `integration-tests` | Enables tests that require a live opencode server | No |
//!
//! `default = ["types", "async-client"]`. For WASM or type-sharing use cases:
//!
//! ```toml
//! [dependencies]
//! opencode-codes = { version = "1.18", default-features = false, features = ["types"] }
//! ```
//!
//! # Provenance
//!
//! Module layout, SSE handling, and server-lifecycle lessons were informed by the
//! Apache-2.0 licensed `opencode_rs` reference SDK. No code is copied verbatim;
//! it is credited here as a design reference in keeping with its license.
//!
//! **Tested against:** opencode 1.18.5.

pub mod error;
pub mod protocol_generated;
pub mod version;

pub use error::{Error, Result};

#[cfg(feature = "async-client")]
pub mod http;

#[cfg(feature = "async-client")]
pub mod sse;

#[cfg(feature = "async-client")]
pub use sse::{EventStream, RetryConfig, StreamEvent};

#[cfg(feature = "async-client")]
pub mod client_async;

#[cfg(feature = "async-client")]
pub use client_async::{OpencodeClient, OpencodeClientBuilder};

#[cfg(feature = "server")]
pub mod server;

#[cfg(feature = "server")]
mod process;

#[cfg(feature = "server")]
pub use server::{ManagedServer, ManagedServerBuilder};
