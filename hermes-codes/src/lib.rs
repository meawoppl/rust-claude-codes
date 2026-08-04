//! # hermes-codes
//!
//! Typed Rust SDK for the [NousResearch Hermes agent](https://github.com/NousResearch/hermes-agent),
//! which exposes a machine interface via the
//! [Agent Client Protocol](https://agentclientprotocol.com) (`hermes acp`:
//! newline-delimited JSON-RPC 2.0 over stdio, bidirectional).
//!
//! Built **from the schema, not from another crate**: types are generated
//! by `scripts/codegen_acp.py` from the vendored ACP schema snapshot that
//! matches what hermes-agent actually speaks — hermes pins the ACP Python
//! SDK, which generates from the *unstable* schema variants at a specific
//! upstream tag. That provenance chain
//! (`tests/schemas/hermes_acp_provenance.json`) is drift-checked in CI.
//!
//! ## Features
//!
//! - `types` — serde models only; `wasm32-unknown-unknown` compatible.
//! - `async-client` (default) — Tokio client that spawns and drives
//!   `hermes acp`.
//!
//! ## Quick start
//!
//! ```ignore
//! use hermes_codes::{AsyncClient, NewSessionRequest, PromptRequest};
//!
//! let (mut client, init) = AsyncClient::start().await?;
//! let session = client.session_new(&NewSessionRequest {
//!     cwd: std::env::current_dir()?.to_string_lossy().into_owned(),
//!     ..Default::default()
//! }).await?;
//!
//! let outcome = client.session_prompt_with(
//!     &PromptRequest {
//!         session_id: session.session_id.clone(),
//!         prompt: vec![hermes_codes::ContentBlock::Text(hermes_codes::TextContent {
//!             text: "Hello!".into(),
//!             ..Default::default()
//!         })],
//!         ..Default::default()
//!     },
//!     |msg| {
//!         // Stream session/update notifications; answer permission
//!         // requests by returning Some(result_json).
//!         None
//!     },
//! ).await?;
//! ```

// Core modules always available
pub mod error;
pub mod jsonrpc;
pub mod protocol;
pub mod protocol_generated;
pub mod version;

// Client modules
#[cfg(feature = "async-client")]
pub mod cli;
#[cfg(feature = "async-client")]
pub mod client_async;
#[cfg(feature = "async-client")]
mod stderr_drain;

pub use error::{Error, Result};
pub use jsonrpc::{JsonRpcMessage, RequestId};
pub use protocol::{
    AgentNotification, AgentToClientRequest, HermesMeta, ServerMessage, SessionProvenance,
};
pub use protocol_generated::methods;
pub use protocol_generated::types::*;

#[cfg(feature = "async-client")]
pub use cli::HermesAcpBuilder;
#[cfg(feature = "async-client")]
pub use client_async::AsyncClient;
