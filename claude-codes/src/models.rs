//! Convenience enum for the models the Claude CLI exposes.
//!
//! Variants are keyed by human-friendly model names; [`ClaudeModel::cli_arg`]
//! returns the exact string to pass to `claude --model` (and therefore to
//! `ClaudeCliBuilder::model`, which accepts the enum directly via
//! `Into<String>`).
//!
//! The model table was extracted from the Claude CLI 2.1.219 binary's model
//! registry. Aliases (`sonnet`, `opus`, …) float to the newest model of that
//! family on the CLI side; pinned variants name one concrete model. Unknown
//! or future model strings round-trip through [`ClaudeModel::Custom`].

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// A model selector accepted by `claude --model`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClaudeModel {
    /// Newest Sonnet-family model (floating alias `sonnet`).
    Sonnet,
    /// Newest Opus-family model (floating alias `opus`). Resolves to
    /// `claude-opus-5` first-party as of CLI 2.1.219.
    Opus,
    /// Newest Haiku-family model (floating alias `haiku`).
    Haiku,
    /// Newest Fable-family model (floating alias `fable`).
    Fable,
    /// The most capable model available to the account (alias `best`).
    Best,
    /// Opus for plan mode, Sonnet otherwise (alias `opusplan`).
    OpusPlan,
    /// Newest Sonnet with the 1M-token context beta (alias `sonnet[1m]`).
    Sonnet1m,
    /// Newest Opus with the 1M-token context beta (alias `opus[1m]`).
    Opus1m,
    /// Newest Fable with the 1M-token context beta (alias `fable[1m]`).
    Fable1m,
    /// Fable 5 (`claude-fable-5`).
    Fable5,
    /// Mythos 5 (`claude-mythos-5`).
    Mythos5,
    /// Opus 5 (`claude-opus-5`).
    Opus5,
    /// Opus 4.8 (`claude-opus-4-8`).
    Opus48,
    /// Opus 4.7 (`claude-opus-4-7`).
    Opus47,
    /// Opus 4.6 (`claude-opus-4-6`).
    Opus46,
    /// Opus 4.5 (`claude-opus-4-5`).
    Opus45,
    /// Opus 4.1 (`claude-opus-4-1`).
    Opus41,
    /// Opus 4 (`claude-opus-4-0`).
    Opus40,
    /// Sonnet 5 (`claude-sonnet-5`).
    Sonnet5,
    /// Sonnet 4.6 (`claude-sonnet-4-6`).
    Sonnet46,
    /// Sonnet 4.5 (`claude-sonnet-4-5`).
    Sonnet45,
    /// Sonnet 4 (`claude-sonnet-4-0`).
    Sonnet40,
    /// Sonnet 3.7 (`claude-3-7-sonnet`).
    Sonnet37,
    /// Sonnet 3.5 (`claude-3-5-sonnet`).
    Sonnet35,
    /// Haiku 4.5 (`claude-haiku-4-5`).
    Haiku45,
    /// Haiku 3.5 (`claude-3-5-haiku`).
    Haiku35,
    /// A model string not yet known to this version of the crate. Passed to
    /// the CLI verbatim.
    Custom(String),
}

impl ClaudeModel {
    /// The string to pass to `claude --model` for this model.
    pub fn cli_arg(&self) -> &str {
        match self {
            Self::Sonnet => "sonnet",
            Self::Opus => "opus",
            Self::Haiku => "haiku",
            Self::Fable => "fable",
            Self::Best => "best",
            Self::OpusPlan => "opusplan",
            Self::Sonnet1m => "sonnet[1m]",
            Self::Opus1m => "opus[1m]",
            Self::Fable1m => "fable[1m]",
            Self::Fable5 => "claude-fable-5",
            Self::Mythos5 => "claude-mythos-5",
            Self::Opus5 => "claude-opus-5",
            Self::Opus48 => "claude-opus-4-8",
            Self::Opus47 => "claude-opus-4-7",
            Self::Opus46 => "claude-opus-4-6",
            Self::Opus45 => "claude-opus-4-5",
            Self::Opus41 => "claude-opus-4-1",
            Self::Opus40 => "claude-opus-4-0",
            Self::Sonnet5 => "claude-sonnet-5",
            Self::Sonnet46 => "claude-sonnet-4-6",
            Self::Sonnet45 => "claude-sonnet-4-5",
            Self::Sonnet40 => "claude-sonnet-4-0",
            Self::Sonnet37 => "claude-3-7-sonnet",
            Self::Sonnet35 => "claude-3-5-sonnet",
            Self::Haiku45 => "claude-haiku-4-5",
            Self::Haiku35 => "claude-3-5-haiku",
            Self::Custom(s) => s.as_str(),
        }
    }

    /// Alias for [`cli_arg`](Self::cli_arg), matching the crate's string-enum
    /// convention.
    pub fn as_str(&self) -> &str {
        self.cli_arg()
    }

    /// Human-friendly display name, matching what the CLI's own UI renders.
    pub fn display_name(&self) -> &str {
        match self {
            Self::Sonnet => "Sonnet (latest)",
            Self::Opus => "Opus (latest)",
            Self::Haiku => "Haiku (latest)",
            Self::Fable => "Fable (latest)",
            Self::Best => "Best available",
            Self::OpusPlan => "Opus (plan mode) / Sonnet",
            Self::Sonnet1m => "Sonnet (latest, 1M context)",
            Self::Opus1m => "Opus (latest, 1M context)",
            Self::Fable1m => "Fable (latest, 1M context)",
            Self::Fable5 => "Fable 5",
            Self::Mythos5 => "Mythos 5",
            Self::Opus5 => "Opus 5",
            Self::Opus48 => "Opus 4.8",
            Self::Opus47 => "Opus 4.7",
            Self::Opus46 => "Opus 4.6",
            Self::Opus45 => "Opus 4.5",
            Self::Opus41 => "Opus 4.1",
            Self::Opus40 => "Opus 4",
            Self::Sonnet5 => "Sonnet 5",
            Self::Sonnet46 => "Sonnet 4.6",
            Self::Sonnet45 => "Sonnet 4.5",
            Self::Sonnet40 => "Sonnet 4",
            Self::Sonnet37 => "Sonnet 3.7",
            Self::Sonnet35 => "Sonnet 3.5",
            Self::Haiku45 => "Haiku 4.5",
            Self::Haiku35 => "Haiku 3.5",
            Self::Custom(s) => s.as_str(),
        }
    }

    /// True for the floating aliases (`sonnet`, `opus`, …) that the CLI
    /// resolves to the newest model of the family at session start.
    pub fn is_alias(&self) -> bool {
        matches!(
            self,
            Self::Sonnet
                | Self::Opus
                | Self::Haiku
                | Self::Fable
                | Self::Best
                | Self::OpusPlan
                | Self::Sonnet1m
                | Self::Opus1m
                | Self::Fable1m
        )
    }

    /// Every model known to this version of the crate, aliases first.
    pub fn known() -> &'static [ClaudeModel] {
        &[
            Self::Sonnet,
            Self::Opus,
            Self::Haiku,
            Self::Fable,
            Self::Best,
            Self::OpusPlan,
            Self::Sonnet1m,
            Self::Opus1m,
            Self::Fable1m,
            Self::Fable5,
            Self::Mythos5,
            Self::Opus5,
            Self::Opus48,
            Self::Opus47,
            Self::Opus46,
            Self::Opus45,
            Self::Opus41,
            Self::Opus40,
            Self::Sonnet5,
            Self::Sonnet46,
            Self::Sonnet45,
            Self::Sonnet40,
            Self::Sonnet37,
            Self::Sonnet35,
            Self::Haiku45,
            Self::Haiku35,
        ]
    }
}

impl fmt::Display for ClaudeModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.cli_arg())
    }
}

impl From<&str> for ClaudeModel {
    fn from(s: &str) -> Self {
        match s {
            "sonnet" => Self::Sonnet,
            "opus" => Self::Opus,
            "haiku" => Self::Haiku,
            "fable" => Self::Fable,
            "best" => Self::Best,
            "opusplan" => Self::OpusPlan,
            "sonnet[1m]" => Self::Sonnet1m,
            "opus[1m]" => Self::Opus1m,
            "fable[1m]" => Self::Fable1m,
            "claude-fable-5" => Self::Fable5,
            "claude-mythos-5" => Self::Mythos5,
            "claude-opus-5" => Self::Opus5,
            "claude-opus-4-8" => Self::Opus48,
            "claude-opus-4-7" => Self::Opus47,
            "claude-opus-4-6" => Self::Opus46,
            "claude-opus-4-5" => Self::Opus45,
            "claude-opus-4-1" => Self::Opus41,
            "claude-opus-4-0" => Self::Opus40,
            "claude-sonnet-5" => Self::Sonnet5,
            "claude-sonnet-4-6" => Self::Sonnet46,
            "claude-sonnet-4-5" => Self::Sonnet45,
            "claude-sonnet-4-0" => Self::Sonnet40,
            "claude-3-7-sonnet" => Self::Sonnet37,
            "claude-3-5-sonnet" => Self::Sonnet35,
            "claude-haiku-4-5" => Self::Haiku45,
            "claude-3-5-haiku" => Self::Haiku35,
            other => Self::Custom(other.to_string()),
        }
    }
}

impl From<ClaudeModel> for String {
    fn from(model: ClaudeModel) -> Self {
        model.cli_arg().to_string()
    }
}

impl Serialize for ClaudeModel {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.cli_arg())
    }
}

impl<'de> Deserialize<'de> for ClaudeModel {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::ClaudeModel;

    #[test]
    fn test_cli_arg_round_trip() {
        for model in ClaudeModel::known() {
            assert_eq!(&ClaudeModel::from(model.cli_arg()), model);
        }
        assert_eq!(
            ClaudeModel::from("claude-nova-7"),
            ClaudeModel::Custom("claude-nova-7".to_string())
        );
        assert_eq!(
            ClaudeModel::from("claude-nova-7").cli_arg(),
            "claude-nova-7"
        );
    }

    #[test]
    fn test_into_string_matches_cli_arg() {
        let s: String = ClaudeModel::Sonnet5.into();
        assert_eq!(s, "claude-sonnet-5");
        let s: String = ClaudeModel::Opus1m.into();
        assert_eq!(s, "opus[1m]");
    }

    #[test]
    fn test_display_names() {
        assert_eq!(ClaudeModel::Opus5.display_name(), "Opus 5");
        assert_eq!(ClaudeModel::Opus5.cli_arg(), "claude-opus-5");
        assert_eq!(ClaudeModel::Fable5.display_name(), "Fable 5");
        assert_eq!(ClaudeModel::Opus48.display_name(), "Opus 4.8");
        assert_eq!(ClaudeModel::Sonnet.display_name(), "Sonnet (latest)");
        assert!(ClaudeModel::Sonnet.is_alias());
        assert!(!ClaudeModel::Sonnet5.is_alias());
    }

    #[test]
    fn test_serde_round_trip() {
        let json = serde_json::to_string(&ClaudeModel::Haiku45).unwrap();
        assert_eq!(json, "\"claude-haiku-4-5\"");
        let back: ClaudeModel = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ClaudeModel::Haiku45);
    }
}
