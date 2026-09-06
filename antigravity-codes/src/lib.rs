//! A typed Rust interface for the [Google Antigravity](https://antigravity.google)
//! agent runtime.
//!
//! <div class="warning">
//!
//! **Maturity warning**: this crate is new and should be considered **highly
//! untested**. The wire types are generated from the protobuf descriptor
//! embedded in the shipped `localharness` binary, and the handshake is locked
//! against captures from a live harness, but real-world mileage is minimal and
//! the API may change between releases while the surface settles. The upstream
//! SDK is itself alpha (`0.1.x`) and reserves extension ranges on its hottest
//! messages, so expect churn. Bug reports and wire captures that break the
//! types are very welcome at
//! <https://github.com/meawoppl/rust-code-agent-sdks/issues>.
//!
//! </div>
//!
//! # What this actually wraps
//!
//! `google-antigravity` on PyPI is a Python client for a compiled Go binary
//! called **`localharness`**, which is where the agent loop, the built-in tools,
//! and the model calls all live. This crate is a client for that same binary —
//! it is a sibling of the Python SDK, not a binding to it, and no Python is
//! involved at runtime.
//!
//! The binary ships **only** inside the platform-specific wheels published to
//! [PyPI](https://pypi.org/project/google-antigravity/). See
//! [`process::find_harness`] for how the crate locates it.
//!
//! # The protocol in one screen
//!
//! Unlike its sibling crates — [`claude-codes`](https://docs.rs/claude-codes) and
//! [`codex-codes`](https://docs.rs/codex-codes), which speak JSON-Lines over
//! stdio, and [`opencode-codes`](https://docs.rs/opencode-codes), which speaks
//! HTTP+SSE — Antigravity uses stdio *only* to bootstrap, then moves to a
//! loopback WebSocket:
//!
//! 1. **Handshake** (binary protobuf, `u32le`-length-prefixed, over stdio).
//!    The client writes an [`protocol::InputConfig`]; the harness replies with
//!    an [`protocol::OutputConfig`] carrying the port it bound and a
//!    single-use API key. See [`handshake`].
//! 2. **Connect** to `ws://127.0.0.1:{port}/` with an `x-goog-api-key` header.
//! 3. **Initialize** by sending an [`protocol::InitializeConversationEvent`];
//!    the harness replies with an
//!    [`protocol::InitializeConversationResponse`] holding the conversation id
//!    and any replayed history.
//! 4. **Converse** — send [`protocol::InputEvent`]s, receive
//!    [`protocol::OutputEvent`]s. Every frame after the handshake is protobuf's
//!    canonical **JSON** mapping, so the wire is text: `camelCase` members,
//!    64-bit integers as strings, `bytes` as base64, enums as value names.
//!
//! Note that a conversation **must** be configured with at least one model. A
//! harness initialised with no [`protocol::ModelConfig`] exits immediately and
//! closes the socket without an error frame.
//!
//! # Choosing a client
//!
//! | Type | What it gives you |
//! |------|-------------------|
//! | [`RawClient`] | The frames, unchanged. You drive the loop and answer the harness's requests yourself. |
//! | [`Client`] | Turn-oriented: [`Client::send`] returns a [`Turn`] that streams assembled [`Step`]s and answers tool calls, hooks, and policy checks from handlers you register. |
//!
//! A worked end-to-end example lives on [`Client`]. At the protocol tier, a
//! frame off the wire decodes like this:
//!
//! ```
//! use antigravity_codes::protocol::{OutputEvent, OutputEventEvent};
//!
//! // Exactly as a live harness emits it — note the stringified `seqNum`.
//! let frame = r#"{
//!   "stepUpdate": {"cascadeId": "abc", "stepIndex": 0, "state": "STATE_DONE", "text": "hi"},
//!   "seqNum": "3",
//!   "timestampMicros": "1786220347646352"
//! }"#;
//!
//! let event: OutputEvent = serde_json::from_str(frame).unwrap();
//! assert_eq!(event.sequence(), Some(3));
//!
//! let Some(OutputEventEvent::StepUpdate(step)) = event.into_event() else {
//!     panic!("expected a step update")
//! };
//! assert_eq!(step.text.as_deref(), Some("hi"));
//! assert!(step.is_terminal());
//! ```
//!
//! # Feature Flags
//!
//! | Feature | Description | WASM-compatible |
//! |---------|-------------|-----------------|
//! | `types` | Wire types and the handshake codec only (serde) | Yes |
//! | `async-client` | Async WebSocket client using tokio | No |
//! | `integration-tests` | Enables tests that need a real harness binary | No |
//!
//! All features are enabled by default. For WASM or type-sharing use cases:
//!
//! ```toml
//! [dependencies]
//! antigravity-codes = { version = "0.1", default-features = false, features = ["types"] }
//! ```
//!
//! # Versioning
//!
//! The crate version tracks the `google-antigravity` release whose harness it
//! was generated from and tested against — see [`TESTED_SDK_VERSION`].

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod error;
pub mod handshake;
pub mod protocol;
mod protocol_generated;
pub mod wire;

pub use error::{Error, Result};

/// The `google-antigravity` release this crate's types were generated from and
/// tested against.
///
/// The harness exposes no version of its own — it takes no arguments and its
/// handshake reply carries only a port and a key — so the wheel version is the
/// only handle there is. Mismatches are not detectable at runtime; unknown enum
/// values and unknown `oneof` arms are absorbed by design instead.
pub const TESTED_SDK_VERSION: &str = "0.1.16";

#[cfg(feature = "async-client")]
mod client;
#[cfg(feature = "async-client")]
mod client_raw;
#[cfg(feature = "async-client")]
pub mod handlers;
#[cfg(feature = "async-client")]
pub mod process;
#[cfg(feature = "async-client")]
pub mod steps;
#[cfg(feature = "async-client")]
mod ws;

#[cfg(feature = "async-client")]
pub use client::{Client, Turn};
#[cfg(feature = "async-client")]
pub use client_raw::RawClient;
#[cfg(feature = "async-client")]
pub use process::{Harness, HarnessOptions, ModelBuilder};
#[cfg(feature = "async-client")]
pub use steps::{Step, StepKind};
