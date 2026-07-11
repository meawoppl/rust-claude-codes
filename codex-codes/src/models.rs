//! Convenience enum for the models the Codex CLI exposes.
//!
//! Variants are keyed by human-friendly model names; [`CodexModel::cli_arg`]
//! returns the model slug to pass to `codex -m` / `--model`, to
//! `ThreadStartParams.model`, or to an `AppServerBuilder::config_override`
//! of `model`.
//!
//! The catalog was taken from `openai/codex@main`'s bundled
//! `models-manager/models.json` (2026-07-11). The server catalog evolves
//! faster than this crate; unknown slugs round-trip through
//! [`CodexModel::Custom`].

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// A model slug accepted by the Codex CLI and app-server.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CodexModel {
    /// GPT-5.6-Sol (`gpt-5.6-sol`).
    Gpt56Sol,
    /// GPT-5.6-Terra (`gpt-5.6-terra`).
    Gpt56Terra,
    /// GPT-5.6-Luna (`gpt-5.6-luna`).
    Gpt56Luna,
    /// GPT-5.5 (`gpt-5.5`).
    Gpt55,
    /// GPT-5.4 (`gpt-5.4`).
    Gpt54,
    /// GPT-5.4-Mini (`gpt-5.4-mini`).
    Gpt54Mini,
    /// GPT-5.2 (`gpt-5.2`).
    Gpt52,
    /// Codex Auto Review (`codex-auto-review`) — hidden from the picker but
    /// accepted by the API.
    CodexAutoReview,
    /// A model slug not yet known to this version of the crate. Passed
    /// through verbatim.
    Custom(String),
}

impl CodexModel {
    /// The slug to pass to `codex -m` / `ThreadStartParams.model`.
    pub fn cli_arg(&self) -> &str {
        match self {
            Self::Gpt56Sol => "gpt-5.6-sol",
            Self::Gpt56Terra => "gpt-5.6-terra",
            Self::Gpt56Luna => "gpt-5.6-luna",
            Self::Gpt55 => "gpt-5.5",
            Self::Gpt54 => "gpt-5.4",
            Self::Gpt54Mini => "gpt-5.4-mini",
            Self::Gpt52 => "gpt-5.2",
            Self::CodexAutoReview => "codex-auto-review",
            Self::Custom(s) => s.as_str(),
        }
    }

    /// Alias for [`cli_arg`](Self::cli_arg), matching the crate's string-enum
    /// convention.
    pub fn as_str(&self) -> &str {
        self.cli_arg()
    }

    /// Human-friendly display name, matching the catalog's `display_name`.
    pub fn display_name(&self) -> &str {
        match self {
            Self::Gpt56Sol => "GPT-5.6-Sol",
            Self::Gpt56Terra => "GPT-5.6-Terra",
            Self::Gpt56Luna => "GPT-5.6-Luna",
            Self::Gpt55 => "GPT-5.5",
            Self::Gpt54 => "GPT-5.4",
            Self::Gpt54Mini => "GPT-5.4-Mini",
            Self::Gpt52 => "GPT-5.2",
            Self::CodexAutoReview => "Codex Auto Review",
            Self::Custom(s) => s.as_str(),
        }
    }

    /// Every model known to this version of the crate.
    pub fn known() -> &'static [CodexModel] {
        &[
            Self::Gpt56Sol,
            Self::Gpt56Terra,
            Self::Gpt56Luna,
            Self::Gpt55,
            Self::Gpt54,
            Self::Gpt54Mini,
            Self::Gpt52,
            Self::CodexAutoReview,
        ]
    }
}

impl fmt::Display for CodexModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.cli_arg())
    }
}

impl From<&str> for CodexModel {
    fn from(s: &str) -> Self {
        match s {
            "gpt-5.6-sol" => Self::Gpt56Sol,
            "gpt-5.6-terra" => Self::Gpt56Terra,
            "gpt-5.6-luna" => Self::Gpt56Luna,
            "gpt-5.5" => Self::Gpt55,
            "gpt-5.4" => Self::Gpt54,
            "gpt-5.4-mini" => Self::Gpt54Mini,
            "gpt-5.2" => Self::Gpt52,
            "codex-auto-review" => Self::CodexAutoReview,
            other => Self::Custom(other.to_string()),
        }
    }
}

impl From<CodexModel> for String {
    fn from(model: CodexModel) -> Self {
        model.cli_arg().to_string()
    }
}

impl Serialize for CodexModel {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.cli_arg())
    }
}

impl<'de> Deserialize<'de> for CodexModel {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::CodexModel;

    #[test]
    fn test_cli_arg_round_trip() {
        for model in CodexModel::known() {
            assert_eq!(&CodexModel::from(model.cli_arg()), model);
        }
        assert_eq!(
            CodexModel::from("gpt-9-experimental"),
            CodexModel::Custom("gpt-9-experimental".to_string())
        );
    }

    #[test]
    fn test_into_string_matches_cli_arg() {
        let s: String = CodexModel::Gpt56Sol.into();
        assert_eq!(s, "gpt-5.6-sol");
    }

    #[test]
    fn test_converts_into_thread_start_model_field() {
        // ThreadStartParams.model is Option<String>; the enum feeds it via Into.
        let model: Option<String> = Some(CodexModel::Gpt55.into());
        assert_eq!(model.as_deref(), Some("gpt-5.5"));
    }

    #[test]
    fn test_serde_round_trip() {
        let json = serde_json::to_string(&CodexModel::Gpt54Mini).unwrap();
        assert_eq!(json, "\"gpt-5.4-mini\"");
        let back: CodexModel = serde_json::from_str(&json).unwrap();
        assert_eq!(back, CodexModel::Gpt54Mini);
    }
}
