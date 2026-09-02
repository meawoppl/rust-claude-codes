//! Convenience enum for the models the Meta provider exposes.
//!
//! Variants are keyed by human-friendly names; [`MuseModel::cli_arg`]
//! returns the exact string to pass to `muse exec --model` (and therefore
//! to `MuseExecBuilder::model`, which accepts the enum via `Into<String>`).
//!
//! The table mirrors Muse Code's on-disk model catalog
//! (`~/.local/share/muse/model-catalog/`, `provider_catalog` source) as of
//! Muse Code 1.0.2. Unknown or future model ids round-trip through
//! [`MuseModel::Custom`]. Context/output limits from the same catalog are
//! exposed via [`MuseModel::context_limit`] / [`MuseModel::output_limit`].

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// A model id accepted by `muse exec --model` (Meta provider).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MuseModel {
    /// muse-spark-1.3 (`muse-spark-1.3`), released 2026-09-02.
    Spark13,
    /// muse-spark-1.3 contributor build (`muse-spark-1.3-contributor`),
    /// released 2026-09-02. The catalog default as of Muse Code 1.0.2.
    Spark13Contributor,
    /// muse-spark-1.2 (`muse-spark-1.2`), released 2026-08-05.
    Spark12,
    /// muse-spark-1.2 contributor build (`muse-spark-1.2-contributor`),
    /// released 2026-08-05.
    Spark12Contributor,
    /// A model id not yet known to this version of the crate. Passed to
    /// the CLI verbatim.
    Custom(String),
}

impl MuseModel {
    /// The string to pass to `muse exec --model` for this model.
    pub fn cli_arg(&self) -> &str {
        match self {
            Self::Spark13 => "muse-spark-1.3",
            Self::Spark13Contributor => "muse-spark-1.3-contributor",
            Self::Spark12 => "muse-spark-1.2",
            Self::Spark12Contributor => "muse-spark-1.2-contributor",
            Self::Custom(s) => s.as_str(),
        }
    }

    /// Alias for [`cli_arg`](Self::cli_arg), matching the crate's
    /// string-enum convention.
    pub fn as_str(&self) -> &str {
        self.cli_arg()
    }

    /// Human-friendly display name (the catalog's `display_label` equals
    /// the id for every current row).
    pub fn display_name(&self) -> &str {
        self.cli_arg()
    }

    /// Context window in tokens, from the provider catalog. `None` for
    /// [`Custom`](Self::Custom) ids the catalog hasn't described.
    pub fn context_limit(&self) -> Option<u64> {
        match self {
            Self::Spark13 | Self::Spark13Contributor | Self::Spark12 | Self::Spark12Contributor => {
                Some(1_007_997)
            }
            Self::Custom(_) => None,
        }
    }

    /// Maximum output tokens, from the provider catalog. `None` for
    /// [`Custom`](Self::Custom) ids the catalog hasn't described.
    pub fn output_limit(&self) -> Option<u64> {
        match self {
            Self::Spark13 | Self::Spark13Contributor | Self::Spark12 | Self::Spark12Contributor => {
                Some(128_000)
            }
            Self::Custom(_) => None,
        }
    }

    /// The catalog-default model as of Muse Code 1.0.2 — what a run
    /// resolves to when `--model` is omitted.
    pub fn catalog_default() -> Self {
        Self::Spark13Contributor
    }

    /// Every model known to this version of the crate, newest first.
    pub fn known() -> &'static [MuseModel] {
        &[
            Self::Spark13,
            Self::Spark13Contributor,
            Self::Spark12,
            Self::Spark12Contributor,
        ]
    }
}

impl fmt::Display for MuseModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.cli_arg())
    }
}

impl From<&str> for MuseModel {
    fn from(s: &str) -> Self {
        match s {
            "muse-spark-1.3" => Self::Spark13,
            "muse-spark-1.3-contributor" => Self::Spark13Contributor,
            "muse-spark-1.2" => Self::Spark12,
            "muse-spark-1.2-contributor" => Self::Spark12Contributor,
            other => Self::Custom(other.to_string()),
        }
    }
}

impl From<MuseModel> for String {
    fn from(model: MuseModel) -> Self {
        model.cli_arg().to_string()
    }
}

impl Serialize for MuseModel {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.cli_arg())
    }
}

impl<'de> Deserialize<'de> for MuseModel {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::MuseModel;

    #[test]
    fn cli_arg_round_trips_for_all_known_models() {
        for model in MuseModel::known() {
            assert_eq!(&MuseModel::from(model.cli_arg()), model);
        }
        assert_eq!(
            MuseModel::from("muse-nova-9"),
            MuseModel::Custom("muse-nova-9".to_string())
        );
    }

    #[test]
    fn catalog_metadata_present_for_known_absent_for_custom() {
        for model in MuseModel::known() {
            assert!(model.context_limit().is_some());
            assert!(model.output_limit().is_some());
        }
        assert_eq!(MuseModel::from("muse-nova-9").context_limit(), None);
    }

    #[test]
    fn default_is_spark_13_contributor() {
        assert_eq!(
            MuseModel::catalog_default().cli_arg(),
            "muse-spark-1.3-contributor"
        );
    }
}
