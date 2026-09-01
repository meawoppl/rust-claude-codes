//! # pi-codes
//!
//! Typed Rust SDK for the [pi coding agent](https://github.com/earendil-works/pi)
//! (`@earendil-works/pi-coding-agent`): serde models of the
//! `pi --mode json` JSONL event stream and the `pi --mode rpc`
//! stdin/stdout command protocol, plus an async (Tokio) RPC client.
//!
//! ```no_run
//! # #[cfg(feature = "async-client")]
//! # async fn demo() -> pi_codes::Result<()> {
//! use pi_codes::{PiRpcClient, RpcCommand};
//!
//! let mut client = PiRpcClient::start().await?;
//! let state = client.request_ok(RpcCommand::GetState { id: None }).await?;
//! println!("session: {:?}", state.data);
//! client.shutdown().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Wire contract
//!
//! - RPC framing is strict JSONL: records split on `\n` only (`\r`
//!   tolerated); U+2028/U+2029 are legal inside strings and must not
//!   split records.
//! - Unknown event types decode to [`io::PiEvent::Unknown`] with the
//!   payload preserved — ignore them rather than treating them as
//!   errors, so CLI upgrades degrade soft.
//! - Unmodeled commands can be sent via [`rpc::RpcCommand::Raw`].
//!
//! **Alpha**: unlike the sibling crates, the crate version does not yet
//! name the tested pi release — it stays 0.0.x while the API settles.
//! The tested release is still machine-readable via
//! [`version::tested_cli_version`].

pub mod cli;
pub mod error;
#[cfg(feature = "types")]
pub mod io;
#[cfg(feature = "types")]
pub mod rpc;
pub mod version;

#[cfg(feature = "async-client")]
pub mod client_async;

pub use cli::{Mode, PiCliBuilder};
pub use error::{Error, Result};
#[cfg(feature = "types")]
pub use io::{ContentBlock, Model, PiEvent, PiMessage, Usage};
#[cfg(feature = "types")]
pub use rpc::{AgentState, BashResult, RpcCommand, RpcResponse, StreamingBehavior};

#[cfg(feature = "async-client")]
pub use client_async::PiRpcClient;
