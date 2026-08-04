//! Version pins this crate is tested against.
//!
//! The full provenance chain lives in
//! `tests/schemas/hermes_acp_provenance.json` and is drift-checked by CI
//! against upstream (see `.github/workflows/hermes-schema-drift.yml`).

/// hermes-agent version the schema snapshots were taken from.
pub const TESTED_HERMES_VERSION: &str = "0.20.0";

/// The Python SDK hermes pins (`agent-client-protocol` on PyPI) — the layer
/// that turns the ACP schema into what actually crosses the wire.
pub const HERMES_PYTHON_SDK_PIN: &str = "0.9.0";

/// The zed-industries/agent-client-protocol tag the Python SDK generates
/// from — the source of `tests/schemas/acp_schema.unstable.json`.
pub const ACP_SCHEMA_REF: &str = "v0.11.2";

/// ACP protocol version negotiated at `initialize`.
pub const ACP_PROTOCOL_VERSION: u16 = crate::protocol_generated::methods::PROTOCOL_VERSION;
