//! Version pins this crate is tested against.

/// Muse Code release the committed captures were taken from
/// (`muse --version` reports `Muse Code 0.1.0 (0.1.0-R708.1)`).
pub const TESTED_MUSE_VERSION: &str = "0.1.0";

/// Full build string of the tested binary.
pub const TESTED_MUSE_BUILD: &str = "0.1.0-R708.1";

/// Envelope `schema_version` the models target.
pub const STREAM_SCHEMA_VERSION: u32 = 1;
