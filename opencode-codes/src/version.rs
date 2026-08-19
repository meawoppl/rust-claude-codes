//! Tested-against pin, machine-readable.

/// The opencode release this crate's live suite last passed against —
/// also asserted at runtime by the integration tests against the server's
/// reported version. Kept in lockstep with the README by CI.
pub fn tested_cli_version() -> &'static str {
    "1.18.18"
}
