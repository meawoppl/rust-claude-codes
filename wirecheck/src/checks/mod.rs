//! Per-agent check suites. Each suite reports into the shared state via
//! [`crate::state::Reporter`] as it goes, so the page shows progress live.

pub mod cargo_suite;
pub mod claude;
pub mod codex;
pub mod muse;
pub mod opencode;
