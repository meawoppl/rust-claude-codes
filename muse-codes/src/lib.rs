//! # muse-codes
//!
//! Typed Rust SDK for [Meta's Muse Code](https://dev.meta.ai/docs) terminal
//! coding agent. Models the machine-readable JSONL event stream of headless
//! runs (`muse exec --json`): an event-sourced journal of envelope records
//! ([`MuseRecord`]) whose payloads cover command intake, run lifecycle,
//! task lifecycle, and streamed output.
//!
//! Types are derived from **captured real CLI output** (committed under
//! `test_cases/`), captured via the credential-free `--provider echo` mode.
//! Payload types not yet observed deserialize as [`MusePayload::Unknown`]
//! rather than failing, and the envelope keeps the raw payload so
//! round-trips are byte-faithful.
//!
//! ## Features
//!
//! - `types` — serde models only; `wasm32-unknown-unknown` compatible.
//! - `async-client` (default) — Tokio client spawning `muse exec --json`.
//!
//! ## Quick start
//!
//! ```ignore
//! use muse_codes::{ExecRun, MuseExecBuilder, MusePayload, Provider};
//!
//! let run = ExecRun::spawn(
//!     &MuseExecBuilder::new("summarize this repo").provider(Provider::Meta),
//! ).await?;
//!
//! let terminal = run.wait_terminal(|record| {
//!     if let Ok(MusePayload::RunOutputDelta(d)) = record.typed_payload() {
//!         print!("{}", d.text);
//!     }
//! }).await?;
//! println!("\nterminal: {}", terminal.terminal);
//! ```

// Core modules always available
pub mod error;
pub mod io;
pub mod version;

// Client modules
#[cfg(feature = "async-client")]
pub mod auth;
#[cfg(feature = "async-client")]
pub mod cli;
#[cfg(feature = "async-client")]
pub mod client_async;

pub use error::{Error, Result};
pub use io::{
    CommandAccepted, Durability, ModelConfigured, MusePayload, MuseRecord, RecordType,
    RunOutputDelta, RunStarted, RunTerminal, SessionRunLinked, StreamKind, StreamRef,
    TaskLifecycle, TaskLifecycleEvent, TaskStreamLinked, ToolResult, TurnInputUser,
};

#[cfg(feature = "async-client")]
pub use cli::{MuseExecBuilder, Provider};
#[cfg(feature = "async-client")]
pub use client_async::ExecRun;
