//! Version pins this crate is tested against.

/// Muse Code release the committed captures were taken from
/// (`muse --version` reports `Muse Code 0.1.0 (0.1.0-R708.1)`).
pub const TESTED_MUSE_VERSION: &str = "0.1.0";

/// Full build string of the tested binary.
pub const TESTED_MUSE_BUILD: &str = "0.1.0-R708.1";

/// Envelope `schema_version` the models target.
pub const STREAM_SCHEMA_VERSION: u32 = 1;

/// The tested-against CLI release — the workspace-uniform accessor
/// (`tested_cli_version()` exists in every crate here); alias of
/// [`TESTED_MUSE_VERSION`]. Kept in lockstep with the README by CI.
pub fn tested_cli_version() -> &'static str {
    TESTED_MUSE_VERSION
}

/// The full build string of the tested release; alias of
/// [`TESTED_MUSE_BUILD`].
pub fn tested_cli_build() -> &'static str {
    TESTED_MUSE_BUILD
}
