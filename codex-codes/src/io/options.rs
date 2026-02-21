use serde::{Deserialize, Serialize};

/// Approval mode for tool execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovalMode {
    Never,
    OnRequest,
    OnFailure,
    Untrusted,
}

/// Sandbox mode controlling file system access.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SandboxMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

impl SandboxMode {
    /// Get the CLI flag value for this sandbox mode.
    pub fn as_cli_str(&self) -> &'static str {
        match self {
            SandboxMode::ReadOnly => "read-only",
            SandboxMode::WorkspaceWrite => "workspace-write",
            SandboxMode::DangerFullAccess => "danger-full-access",
        }
    }
}

/// Model reasoning effort level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelReasoningEffort {
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
}

/// Web search mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebSearchMode {
    Disabled,
    Cached,
    Live,
}

/// Per-thread options controlling model, sandbox, and behavior.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThreadOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox_mode: Option<SandboxMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_git_repo_check: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_reasoning_effort: Option<ModelReasoningEffort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_access_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search_mode: Option<WebSearchMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_policy: Option<ApprovalMode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional_directories: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_options_default() {
        let opts = ThreadOptions::default();
        assert!(opts.model.is_none());
        assert!(opts.sandbox_mode.is_none());
        assert!(opts.additional_directories.is_empty());
    }

    #[test]
    fn test_approval_mode_serde() {
        let json = r#""on-request""#;
        let mode: ApprovalMode = serde_json::from_str(json).unwrap();
        assert_eq!(mode, ApprovalMode::OnRequest);
    }

    #[test]
    fn test_sandbox_mode_serde() {
        let json = r#""workspace-write""#;
        let mode: SandboxMode = serde_json::from_str(json).unwrap();
        assert_eq!(mode, SandboxMode::WorkspaceWrite);
    }

    #[test]
    fn test_reasoning_effort_serde() {
        let json = r#""xhigh""#;
        let effort: ModelReasoningEffort = serde_json::from_str(json).unwrap();
        assert_eq!(effort, ModelReasoningEffort::Xhigh);
    }

    #[test]
    fn test_web_search_mode_serde() {
        let json = r#""live""#;
        let mode: WebSearchMode = serde_json::from_str(json).unwrap();
        assert_eq!(mode, WebSearchMode::Live);
    }
}
