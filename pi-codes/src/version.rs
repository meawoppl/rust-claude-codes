//! Version pin this crate is tested against.

/// pi coding-agent release the live suite last passed against
/// (`pi --version` reports this; npm package
/// `@earendil-works/pi-coding-agent`). The live tier covers the
/// credential-free RPC surface (state, models, bash, session commands)
/// and the full deserialization suite; model-turn coverage requires a
/// configured provider and is gated the same way.
pub const TESTED_PI_VERSION: &str = "0.85.1";

/// The pi release this crate's live suite last passed against — the
/// machine-readable form of the crate's version convention. Kept in
/// lockstep with the README by CI.
pub fn tested_cli_version() -> &'static str {
    TESTED_PI_VERSION
}
