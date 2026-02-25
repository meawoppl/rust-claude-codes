use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::fmt;

// ============================================================================
// Permission Enums
// ============================================================================

/// The type of a permission grant.
///
/// Determines whether the permission adds rules for specific tools
/// or sets a broad mode.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PermissionType {
    /// Add fine-grained rules for specific tools.
    AddRules,
    /// Set a broad permission mode (e.g., accept all edits).
    SetMode,
    /// A type not yet known to this version of the crate.
    Unknown(String),
}

impl PermissionType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::AddRules => "addRules",
            Self::SetMode => "setMode",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for PermissionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for PermissionType {
    fn from(s: &str) -> Self {
        match s {
            "addRules" => Self::AddRules,
            "setMode" => Self::SetMode,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for PermissionType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for PermissionType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// Where a permission applies.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PermissionDestination {
    /// Applies only to the current session.
    Session,
    /// Persists across sessions for the project.
    Project,
    /// A destination not yet known to this version of the crate.
    Unknown(String),
}

impl PermissionDestination {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Session => "session",
            Self::Project => "project",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for PermissionDestination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for PermissionDestination {
    fn from(s: &str) -> Self {
        match s {
            "session" => Self::Session,
            "project" => Self::Project,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for PermissionDestination {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for PermissionDestination {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// The behavior of a permission rule.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PermissionBehavior {
    /// Allow the tool action.
    Allow,
    /// Deny the tool action.
    Deny,
    /// A behavior not yet known to this version of the crate.
    Unknown(String),
}

impl PermissionBehavior {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for PermissionBehavior {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for PermissionBehavior {
    fn from(s: &str) -> Self {
        match s {
            "allow" => Self::Allow,
            "deny" => Self::Deny,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for PermissionBehavior {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for PermissionBehavior {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// Named permission modes that can be set via `setMode`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PermissionModeName {
    /// Accept all file edits without prompting.
    AcceptEdits,
    /// Bypass all permission checks.
    BypassPermissions,
    /// A mode not yet known to this version of the crate.
    Unknown(String),
}

impl PermissionModeName {
    pub fn as_str(&self) -> &str {
        match self {
            Self::AcceptEdits => "acceptEdits",
            Self::BypassPermissions => "bypassPermissions",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for PermissionModeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for PermissionModeName {
    fn from(s: &str) -> Self {
        match s {
            "acceptEdits" => Self::AcceptEdits,
            "bypassPermissions" => Self::BypassPermissions,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for PermissionModeName {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for PermissionModeName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

// ============================================================================
// Control Protocol Types (for bidirectional tool approval)
// ============================================================================

/// Control request from CLI (tool permission requests, hooks, etc.)
///
/// When using `--permission-prompt-tool stdio`, the CLI sends these requests
/// asking for approval before executing tools. The SDK must respond with a
/// [`ControlResponse`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlRequest {
    /// Unique identifier for this request (used to correlate responses)
    pub request_id: String,
    /// The request payload
    pub request: ControlRequestPayload,
}

/// Control request payload variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum ControlRequestPayload {
    /// Tool permission request - Claude wants to use a tool
    CanUseTool(ToolPermissionRequest),
    /// Hook callback request
    HookCallback(HookCallbackRequest),
    /// MCP message request
    McpMessage(McpMessageRequest),
    /// Initialize request (sent by SDK to CLI)
    Initialize(InitializeRequest),
}

/// A permission to grant for "remember this decision" functionality.
///
/// When responding to a tool permission request, you can include permissions
/// that should be granted to avoid repeated prompts for similar actions.
///
/// # Example
///
/// ```
/// use claude_codes::{Permission, PermissionModeName, PermissionDestination};
///
/// // Grant permission for a specific bash command
/// let perm = Permission::allow_tool("Bash", "npm test");
///
/// // Grant permission to set a mode for the session
/// let mode_perm = Permission::set_mode(PermissionModeName::AcceptEdits, PermissionDestination::Session);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Permission {
    /// The type of permission (e.g., addRules, setMode)
    #[serde(rename = "type")]
    pub permission_type: PermissionType,
    /// Where to apply this permission (e.g., session, project)
    pub destination: PermissionDestination,
    /// The permission mode (for setMode type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<PermissionModeName>,
    /// The behavior (for addRules type, e.g., allow, deny)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behavior: Option<PermissionBehavior>,
    /// The rules to add (for addRules type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<PermissionRule>>,
}

/// A rule within a permission grant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PermissionRule {
    /// The name of the tool this rule applies to
    #[serde(rename = "toolName")]
    pub tool_name: String,
    /// The rule content (glob pattern or command pattern)
    #[serde(rename = "ruleContent")]
    pub rule_content: String,
}

impl Permission {
    /// Create a permission to allow a specific tool with a rule pattern.
    ///
    /// # Example
    /// ```
    /// use claude_codes::Permission;
    ///
    /// // Allow "npm test" bash command for this session
    /// let perm = Permission::allow_tool("Bash", "npm test");
    ///
    /// // Allow reading from /tmp directory
    /// let read_perm = Permission::allow_tool("Read", "/tmp/**");
    /// ```
    pub fn allow_tool(tool_name: impl Into<String>, rule_content: impl Into<String>) -> Self {
        Permission {
            permission_type: PermissionType::AddRules,
            destination: PermissionDestination::Session,
            mode: None,
            behavior: Some(PermissionBehavior::Allow),
            rules: Some(vec![PermissionRule {
                tool_name: tool_name.into(),
                rule_content: rule_content.into(),
            }]),
        }
    }

    /// Create a permission to allow a tool with a specific destination.
    ///
    /// # Example
    /// ```
    /// use claude_codes::{Permission, PermissionDestination};
    ///
    /// // Allow for the entire project, not just session
    /// let perm = Permission::allow_tool_with_destination("Bash", "npm test", PermissionDestination::Project);
    /// ```
    pub fn allow_tool_with_destination(
        tool_name: impl Into<String>,
        rule_content: impl Into<String>,
        destination: PermissionDestination,
    ) -> Self {
        Permission {
            permission_type: PermissionType::AddRules,
            destination,
            mode: None,
            behavior: Some(PermissionBehavior::Allow),
            rules: Some(vec![PermissionRule {
                tool_name: tool_name.into(),
                rule_content: rule_content.into(),
            }]),
        }
    }

    /// Create a permission to set a mode (like acceptEdits or bypassPermissions).
    ///
    /// # Example
    /// ```
    /// use claude_codes::{Permission, PermissionModeName, PermissionDestination};
    ///
    /// // Accept all edits for this session
    /// let perm = Permission::set_mode(PermissionModeName::AcceptEdits, PermissionDestination::Session);
    /// ```
    pub fn set_mode(mode: PermissionModeName, destination: PermissionDestination) -> Self {
        Permission {
            permission_type: PermissionType::SetMode,
            destination,
            mode: Some(mode),
            behavior: None,
            rules: None,
        }
    }

    /// Create a permission from a PermissionSuggestion.
    ///
    /// This is useful when you want to grant a permission that Claude suggested.
    ///
    /// # Example
    /// ```
    /// use claude_codes::{Permission, PermissionSuggestion, PermissionType, PermissionDestination, PermissionModeName};
    ///
    /// // Convert a suggestion to a permission for the response
    /// let suggestion = PermissionSuggestion {
    ///     suggestion_type: PermissionType::SetMode,
    ///     destination: PermissionDestination::Session,
    ///     mode: Some(PermissionModeName::AcceptEdits),
    ///     behavior: None,
    ///     rules: None,
    /// };
    /// let perm = Permission::from_suggestion(&suggestion);
    /// ```
    pub fn from_suggestion(suggestion: &PermissionSuggestion) -> Self {
        Permission {
            permission_type: suggestion.suggestion_type.clone(),
            destination: suggestion.destination.clone(),
            mode: suggestion.mode.clone(),
            behavior: suggestion.behavior.clone(),
            rules: suggestion.rules.as_ref().map(|rules| {
                rules
                    .iter()
                    .filter_map(|v| {
                        Some(PermissionRule {
                            tool_name: v.get("toolName")?.as_str()?.to_string(),
                            rule_content: v.get("ruleContent")?.as_str()?.to_string(),
                        })
                    })
                    .collect()
            }),
        }
    }
}

/// A suggested permission for tool approval.
///
/// When Claude requests tool permission, it may include suggestions for
/// permissions that could be granted to avoid repeated prompts for similar
/// actions. The format varies based on the suggestion type:
///
/// - `setMode`: `{"type": "setMode", "mode": "acceptEdits", "destination": "session"}`
/// - `addRules`: `{"type": "addRules", "rules": [...], "behavior": "allow", "destination": "session"}`
///
/// Use the helper methods to access common fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PermissionSuggestion {
    /// The type of suggestion (e.g., setMode, addRules)
    #[serde(rename = "type")]
    pub suggestion_type: PermissionType,
    /// Where to apply this permission (e.g., session, project)
    pub destination: PermissionDestination,
    /// The permission mode (for setMode type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<PermissionModeName>,
    /// The behavior (for addRules type, e.g., allow)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behavior: Option<PermissionBehavior>,
    /// The rules to add (for addRules type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<Value>>,
}

/// Tool permission request details
///
/// This is sent when Claude wants to use a tool. The SDK should evaluate
/// the request and respond with allow/deny using the ergonomic builder methods.
///
/// # Example
///
/// ```
/// use claude_codes::{ToolPermissionRequest, ControlResponse};
/// use serde_json::json;
///
/// fn handle_permission(req: &ToolPermissionRequest, request_id: &str) -> ControlResponse {
///     // Block dangerous bash commands
///     if req.tool_name == "Bash" {
///         if let Some(cmd) = req.input.get("command").and_then(|v| v.as_str()) {
///             if cmd.contains("rm -rf") {
///                 return req.deny("Dangerous command blocked", request_id);
///             }
///         }
///     }
///
///     // Allow everything else
///     req.allow(request_id)
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissionRequest {
    /// Name of the tool Claude wants to use (e.g., "Bash", "Write", "Read")
    pub tool_name: String,
    /// Input parameters for the tool
    pub input: Value,
    /// Suggested permissions that could be granted to avoid repeated prompts
    #[serde(default)]
    pub permission_suggestions: Vec<PermissionSuggestion>,
    /// Path that was blocked (if this is a retry after path-based denial)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_path: Option<String>,
    /// Reason why this tool use requires approval
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision_reason: Option<String>,
    /// The tool use ID for this request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
}

impl ToolPermissionRequest {
    /// Allow the tool to execute with its original input.
    ///
    /// # Example
    /// ```
    /// # use claude_codes::ToolPermissionRequest;
    /// # use serde_json::json;
    /// let req = ToolPermissionRequest {
    ///     tool_name: "Read".to_string(),
    ///     input: json!({"file_path": "/tmp/test.txt"}),
    ///     permission_suggestions: vec![],
    ///     blocked_path: None,
    ///     decision_reason: None,
    ///     tool_use_id: None,
    /// };
    /// let response = req.allow("req-123");
    /// ```
    pub fn allow(&self, request_id: &str) -> ControlResponse {
        ControlResponse::from_result(request_id, PermissionResult::allow(self.input.clone()))
    }

    /// Allow the tool to execute with modified input.
    ///
    /// Use this to sanitize or redirect tool inputs. For example, redirecting
    /// file writes to a safe directory.
    ///
    /// # Example
    /// ```
    /// # use claude_codes::ToolPermissionRequest;
    /// # use serde_json::json;
    /// let req = ToolPermissionRequest {
    ///     tool_name: "Write".to_string(),
    ///     input: json!({"file_path": "/etc/passwd", "content": "test"}),
    ///     permission_suggestions: vec![],
    ///     blocked_path: None,
    ///     decision_reason: None,
    ///     tool_use_id: None,
    /// };
    /// // Redirect to safe location
    /// let safe_input = json!({"file_path": "/tmp/safe/passwd", "content": "test"});
    /// let response = req.allow_with(safe_input, "req-123");
    /// ```
    pub fn allow_with(&self, modified_input: Value, request_id: &str) -> ControlResponse {
        ControlResponse::from_result(request_id, PermissionResult::allow(modified_input))
    }

    /// Allow with updated permissions list (raw JSON Values).
    ///
    /// Prefer using `allow_and_remember` for type safety.
    pub fn allow_with_permissions(
        &self,
        modified_input: Value,
        permissions: Vec<Value>,
        request_id: &str,
    ) -> ControlResponse {
        ControlResponse::from_result(
            request_id,
            PermissionResult::allow_with_permissions(modified_input, permissions),
        )
    }

    /// Allow the tool and grant permissions for "remember this decision".
    ///
    /// This is the ergonomic way to allow a tool while also granting permissions
    /// so similar actions won't require approval in the future.
    ///
    /// # Example
    /// ```
    /// use claude_codes::{ToolPermissionRequest, Permission};
    /// use serde_json::json;
    ///
    /// let req = ToolPermissionRequest {
    ///     tool_name: "Bash".to_string(),
    ///     input: json!({"command": "npm test"}),
    ///     permission_suggestions: vec![],
    ///     blocked_path: None,
    ///     decision_reason: None,
    ///     tool_use_id: None,
    /// };
    ///
    /// // Allow and remember this decision for the session
    /// let response = req.allow_and_remember(
    ///     vec![Permission::allow_tool("Bash", "npm test")],
    ///     "req-123",
    /// );
    /// ```
    pub fn allow_and_remember(
        &self,
        permissions: Vec<Permission>,
        request_id: &str,
    ) -> ControlResponse {
        ControlResponse::from_result(
            request_id,
            PermissionResult::allow_with_typed_permissions(self.input.clone(), permissions),
        )
    }

    /// Allow the tool with modified input and grant permissions.
    ///
    /// Combines input modification with "remember this decision" functionality.
    pub fn allow_with_and_remember(
        &self,
        modified_input: Value,
        permissions: Vec<Permission>,
        request_id: &str,
    ) -> ControlResponse {
        ControlResponse::from_result(
            request_id,
            PermissionResult::allow_with_typed_permissions(modified_input, permissions),
        )
    }

    /// Allow the tool and remember using the first permission suggestion.
    ///
    /// This is a convenience method for the common case of accepting Claude's
    /// first suggested permission (usually the most relevant one).
    ///
    /// Returns `None` if there are no permission suggestions.
    ///
    /// # Example
    /// ```
    /// use claude_codes::ToolPermissionRequest;
    /// use serde_json::json;
    ///
    /// let req = ToolPermissionRequest {
    ///     tool_name: "Bash".to_string(),
    ///     input: json!({"command": "npm test"}),
    ///     permission_suggestions: vec![],  // Would have suggestions in real use
    ///     blocked_path: None,
    ///     decision_reason: None,
    ///     tool_use_id: None,
    /// };
    ///
    /// // Try to allow with first suggestion, or just allow without remembering
    /// let response = req.allow_and_remember_suggestion("req-123")
    ///     .unwrap_or_else(|| req.allow("req-123"));
    /// ```
    pub fn allow_and_remember_suggestion(&self, request_id: &str) -> Option<ControlResponse> {
        self.permission_suggestions.first().map(|suggestion| {
            let perm = Permission::from_suggestion(suggestion);
            self.allow_and_remember(vec![perm], request_id)
        })
    }

    /// Deny the tool execution.
    ///
    /// The message will be shown to Claude, who may try a different approach.
    ///
    /// # Example
    /// ```
    /// # use claude_codes::ToolPermissionRequest;
    /// # use serde_json::json;
    /// let req = ToolPermissionRequest {
    ///     tool_name: "Bash".to_string(),
    ///     input: json!({"command": "sudo rm -rf /"}),
    ///     permission_suggestions: vec![],
    ///     blocked_path: None,
    ///     decision_reason: None,
    ///     tool_use_id: None,
    /// };
    /// let response = req.deny("Dangerous command blocked by policy", "req-123");
    /// ```
    pub fn deny(&self, message: impl Into<String>, request_id: &str) -> ControlResponse {
        ControlResponse::from_result(request_id, PermissionResult::deny(message))
    }

    /// Deny the tool execution and stop the entire session.
    ///
    /// Use this for severe policy violations that should halt all processing.
    pub fn deny_and_stop(&self, message: impl Into<String>, request_id: &str) -> ControlResponse {
        ControlResponse::from_result(request_id, PermissionResult::deny_and_interrupt(message))
    }
}

/// Result of a permission decision
///
/// This type represents the decision made by the permission callback.
/// It can be serialized directly into the control response format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "behavior", rename_all = "snake_case")]
pub enum PermissionResult {
    /// Allow the tool to execute
    Allow {
        /// The (possibly modified) input to pass to the tool
        #[serde(rename = "updatedInput")]
        updated_input: Value,
        /// Optional updated permissions list
        #[serde(rename = "updatedPermissions", skip_serializing_if = "Option::is_none")]
        updated_permissions: Option<Vec<Value>>,
    },
    /// Deny the tool execution
    Deny {
        /// Message explaining why the tool was denied
        message: String,
        /// If true, stop the entire session
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        interrupt: bool,
    },
}

impl PermissionResult {
    /// Create an allow result with the given input
    pub fn allow(input: Value) -> Self {
        PermissionResult::Allow {
            updated_input: input,
            updated_permissions: None,
        }
    }

    /// Create an allow result with raw permissions (as JSON Values).
    ///
    /// Prefer using `allow_with_typed_permissions` for type safety.
    pub fn allow_with_permissions(input: Value, permissions: Vec<Value>) -> Self {
        PermissionResult::Allow {
            updated_input: input,
            updated_permissions: Some(permissions),
        }
    }

    /// Create an allow result with typed permissions.
    ///
    /// This is the preferred way to grant permissions for "remember this decision"
    /// functionality.
    ///
    /// # Example
    /// ```
    /// use claude_codes::{Permission, PermissionResult};
    /// use serde_json::json;
    ///
    /// let result = PermissionResult::allow_with_typed_permissions(
    ///     json!({"command": "npm test"}),
    ///     vec![Permission::allow_tool("Bash", "npm test")],
    /// );
    /// ```
    pub fn allow_with_typed_permissions(input: Value, permissions: Vec<Permission>) -> Self {
        let permission_values: Vec<Value> = permissions
            .into_iter()
            .filter_map(|p| serde_json::to_value(p).ok())
            .collect();
        PermissionResult::Allow {
            updated_input: input,
            updated_permissions: Some(permission_values),
        }
    }

    /// Create a deny result
    pub fn deny(message: impl Into<String>) -> Self {
        PermissionResult::Deny {
            message: message.into(),
            interrupt: false,
        }
    }

    /// Create a deny result that also interrupts the session
    pub fn deny_and_interrupt(message: impl Into<String>) -> Self {
        PermissionResult::Deny {
            message: message.into(),
            interrupt: true,
        }
    }
}

/// Hook callback request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookCallbackRequest {
    pub callback_id: String,
    pub input: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
}

/// MCP message request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpMessageRequest {
    pub server_name: String,
    pub message: Value,
}

/// Initialize request (SDK -> CLI)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<Value>,
}

/// Control response to CLI
///
/// Built using the ergonomic methods on [`ToolPermissionRequest`] or
/// constructed directly for other control request types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlResponse {
    /// The request ID this response corresponds to
    pub response: ControlResponsePayload,
}

impl ControlResponse {
    /// Create a success response from a PermissionResult
    ///
    /// This is the preferred way to construct permission responses.
    pub fn from_result(request_id: &str, result: PermissionResult) -> Self {
        // Serialize the PermissionResult to Value for the response
        let response_value = serde_json::to_value(&result)
            .expect("PermissionResult serialization should never fail");
        ControlResponse {
            response: ControlResponsePayload::Success {
                request_id: request_id.to_string(),
                response: Some(response_value),
            },
        }
    }

    /// Create a success response with the given payload (raw Value)
    pub fn success(request_id: &str, response_data: Value) -> Self {
        ControlResponse {
            response: ControlResponsePayload::Success {
                request_id: request_id.to_string(),
                response: Some(response_data),
            },
        }
    }

    /// Create an empty success response (for acks)
    pub fn success_empty(request_id: &str) -> Self {
        ControlResponse {
            response: ControlResponsePayload::Success {
                request_id: request_id.to_string(),
                response: None,
            },
        }
    }

    /// Create an error response
    pub fn error(request_id: &str, error_message: impl Into<String>) -> Self {
        ControlResponse {
            response: ControlResponsePayload::Error {
                request_id: request_id.to_string(),
                error: error_message.into(),
            },
        }
    }
}

/// Control response payload
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum ControlResponsePayload {
    Success {
        request_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        response: Option<Value>,
    },
    Error {
        request_id: String,
        error: String,
    },
}

/// Wrapper for outgoing control responses (includes type tag)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlResponseMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub response: ControlResponsePayload,
}

impl From<ControlResponse> for ControlResponseMessage {
    fn from(resp: ControlResponse) -> Self {
        ControlResponseMessage {
            message_type: "control_response".to_string(),
            response: resp.response,
        }
    }
}

/// Wrapper for outgoing control requests (includes type tag)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlRequestMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub request_id: String,
    pub request: ControlRequestPayload,
}

impl ControlRequestMessage {
    /// Create an initialization request to send to CLI
    pub fn initialize(request_id: impl Into<String>) -> Self {
        ControlRequestMessage {
            message_type: "control_request".to_string(),
            request_id: request_id.into(),
            request: ControlRequestPayload::Initialize(InitializeRequest { hooks: None }),
        }
    }

    /// Create an initialization request with hooks configuration
    pub fn initialize_with_hooks(request_id: impl Into<String>, hooks: Value) -> Self {
        ControlRequestMessage {
            message_type: "control_request".to_string(),
            request_id: request_id.into(),
            request: ControlRequestPayload::Initialize(InitializeRequest { hooks: Some(hooks) }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::ClaudeOutput;

    #[test]
    fn test_deserialize_control_request_can_use_tool() {
        let json = r#"{
            "type": "control_request",
            "request_id": "perm-abc123",
            "request": {
                "subtype": "can_use_tool",
                "tool_name": "Write",
                "input": {
                    "file_path": "/home/user/hello.py",
                    "content": "print('hello')"
                },
                "permission_suggestions": [],
                "blocked_path": null
            }
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert!(output.is_control_request());

        if let ClaudeOutput::ControlRequest(req) = output {
            assert_eq!(req.request_id, "perm-abc123");
            if let ControlRequestPayload::CanUseTool(perm_req) = req.request {
                assert_eq!(perm_req.tool_name, "Write");
                assert_eq!(
                    perm_req.input.get("file_path").unwrap().as_str().unwrap(),
                    "/home/user/hello.py"
                );
            } else {
                panic!("Expected CanUseTool payload");
            }
        } else {
            panic!("Expected ControlRequest");
        }
    }

    #[test]
    fn test_deserialize_control_request_edit_tool_real() {
        // Real production message from Claude CLI
        let json = r#"{"type":"control_request","request_id":"f3cf357c-17d6-4eca-b498-dd17c7ac43dd","request":{"subtype":"can_use_tool","tool_name":"Edit","input":{"file_path":"/home/meawoppl/repos/cc-proxy/proxy/src/ui.rs","old_string":"/// Print hint to re-authenticate\npub fn print_reauth_hint() {\n    println!(\n        \"  {} Run: {} to re-authenticate\",\n        \"→\".bright_blue(),\n        \"claude-portal logout && claude-portal login\".bright_cyan()\n    );\n}","new_string":"/// Print hint to re-authenticate\npub fn print_reauth_hint() {\n    println!(\n        \"  {} Run: {} to re-authenticate\",\n        \"→\".bright_blue(),\n        \"claude-portal --reauth\".bright_cyan()\n    );\n}","replace_all":false},"permission_suggestions":[{"type":"setMode","mode":"acceptEdits","destination":"session"}],"tool_use_id":"toolu_015BDGtNiqNrRSJSDrWXNckW"}}"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert!(output.is_control_request());
        assert_eq!(output.message_type(), "control_request");

        if let ClaudeOutput::ControlRequest(req) = output {
            assert_eq!(req.request_id, "f3cf357c-17d6-4eca-b498-dd17c7ac43dd");
            if let ControlRequestPayload::CanUseTool(perm_req) = req.request {
                assert_eq!(perm_req.tool_name, "Edit");
                assert_eq!(
                    perm_req.input.get("file_path").unwrap().as_str().unwrap(),
                    "/home/meawoppl/repos/cc-proxy/proxy/src/ui.rs"
                );
                assert!(perm_req.input.get("old_string").is_some());
                assert!(perm_req.input.get("new_string").is_some());
                assert!(!perm_req
                    .input
                    .get("replace_all")
                    .unwrap()
                    .as_bool()
                    .unwrap());
            } else {
                panic!("Expected CanUseTool payload");
            }
        } else {
            panic!("Expected ControlRequest");
        }
    }

    #[test]
    fn test_tool_permission_request_allow() {
        let req = ToolPermissionRequest {
            tool_name: "Read".to_string(),
            input: serde_json::json!({"file_path": "/tmp/test.txt"}),
            permission_suggestions: vec![],
            blocked_path: None,
            decision_reason: None,
            tool_use_id: None,
        };

        let response = req.allow("req-123");
        let message: ControlResponseMessage = response.into();

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"type\":\"control_response\""));
        assert!(json.contains("\"subtype\":\"success\""));
        assert!(json.contains("\"request_id\":\"req-123\""));
        assert!(json.contains("\"behavior\":\"allow\""));
        assert!(json.contains("\"updatedInput\""));
    }

    #[test]
    fn test_tool_permission_request_allow_with_modified_input() {
        let req = ToolPermissionRequest {
            tool_name: "Write".to_string(),
            input: serde_json::json!({"file_path": "/etc/passwd", "content": "test"}),
            permission_suggestions: vec![],
            blocked_path: None,
            decision_reason: None,
            tool_use_id: None,
        };

        let modified_input = serde_json::json!({
            "file_path": "/tmp/safe/passwd",
            "content": "test"
        });
        let response = req.allow_with(modified_input, "req-456");
        let message: ControlResponseMessage = response.into();

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("/tmp/safe/passwd"));
        assert!(!json.contains("/etc/passwd"));
    }

    #[test]
    fn test_tool_permission_request_deny() {
        let req = ToolPermissionRequest {
            tool_name: "Bash".to_string(),
            input: serde_json::json!({"command": "sudo rm -rf /"}),
            permission_suggestions: vec![],
            blocked_path: None,
            decision_reason: None,
            tool_use_id: None,
        };

        let response = req.deny("Dangerous command blocked", "req-789");
        let message: ControlResponseMessage = response.into();

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"behavior\":\"deny\""));
        assert!(json.contains("Dangerous command blocked"));
        assert!(!json.contains("\"interrupt\":true"));
    }

    #[test]
    fn test_tool_permission_request_deny_and_stop() {
        let req = ToolPermissionRequest {
            tool_name: "Bash".to_string(),
            input: serde_json::json!({"command": "rm -rf /"}),
            permission_suggestions: vec![],
            blocked_path: None,
            decision_reason: None,
            tool_use_id: None,
        };

        let response = req.deny_and_stop("Security violation", "req-000");
        let message: ControlResponseMessage = response.into();

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"behavior\":\"deny\""));
        assert!(json.contains("\"interrupt\":true"));
    }

    #[test]
    fn test_permission_result_serialization() {
        // Test allow
        let allow = PermissionResult::allow(serde_json::json!({"test": "value"}));
        let json = serde_json::to_string(&allow).unwrap();
        assert!(json.contains("\"behavior\":\"allow\""));
        assert!(json.contains("\"updatedInput\""));

        // Test deny
        let deny = PermissionResult::deny("Not allowed");
        let json = serde_json::to_string(&deny).unwrap();
        assert!(json.contains("\"behavior\":\"deny\""));
        assert!(json.contains("\"message\":\"Not allowed\""));
        assert!(!json.contains("\"interrupt\""));

        // Test deny with interrupt
        let deny_stop = PermissionResult::deny_and_interrupt("Stop!");
        let json = serde_json::to_string(&deny_stop).unwrap();
        assert!(json.contains("\"interrupt\":true"));
    }

    #[test]
    fn test_control_request_message_initialize() {
        let init = ControlRequestMessage::initialize("init-1");

        let json = serde_json::to_string(&init).unwrap();
        assert!(json.contains("\"type\":\"control_request\""));
        assert!(json.contains("\"request_id\":\"init-1\""));
        assert!(json.contains("\"subtype\":\"initialize\""));
    }

    #[test]
    fn test_control_response_error() {
        let response = ControlResponse::error("req-err", "Something went wrong");
        let message: ControlResponseMessage = response.into();

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"subtype\":\"error\""));
        assert!(json.contains("\"error\":\"Something went wrong\""));
    }

    #[test]
    fn test_roundtrip_control_request() {
        let original_json = r#"{
            "type": "control_request",
            "request_id": "test-123",
            "request": {
                "subtype": "can_use_tool",
                "tool_name": "Bash",
                "input": {"command": "ls -la"},
                "permission_suggestions": []
            }
        }"#;

        let output: ClaudeOutput = serde_json::from_str(original_json).unwrap();

        let reserialized = serde_json::to_string(&output).unwrap();
        assert!(reserialized.contains("control_request"));
        assert!(reserialized.contains("test-123"));
        assert!(reserialized.contains("Bash"));
    }

    #[test]
    fn test_permission_suggestions_parsing() {
        let json = r#"{
            "type": "control_request",
            "request_id": "perm-456",
            "request": {
                "subtype": "can_use_tool",
                "tool_name": "Bash",
                "input": {"command": "npm test"},
                "permission_suggestions": [
                    {"type": "setMode", "mode": "acceptEdits", "destination": "session"},
                    {"type": "setMode", "mode": "bypassPermissions", "destination": "project"}
                ]
            }
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::ControlRequest(req) = output {
            if let ControlRequestPayload::CanUseTool(perm_req) = req.request {
                assert_eq!(perm_req.permission_suggestions.len(), 2);
                assert_eq!(
                    perm_req.permission_suggestions[0].suggestion_type,
                    PermissionType::SetMode
                );
                assert_eq!(
                    perm_req.permission_suggestions[0].mode,
                    Some(PermissionModeName::AcceptEdits)
                );
                assert_eq!(
                    perm_req.permission_suggestions[0].destination,
                    PermissionDestination::Session
                );
                assert_eq!(
                    perm_req.permission_suggestions[1].suggestion_type,
                    PermissionType::SetMode
                );
                assert_eq!(
                    perm_req.permission_suggestions[1].mode,
                    Some(PermissionModeName::BypassPermissions)
                );
                assert_eq!(
                    perm_req.permission_suggestions[1].destination,
                    PermissionDestination::Project
                );
            } else {
                panic!("Expected CanUseTool payload");
            }
        } else {
            panic!("Expected ControlRequest");
        }
    }

    #[test]
    fn test_permission_suggestion_set_mode_roundtrip() {
        let suggestion = PermissionSuggestion {
            suggestion_type: PermissionType::SetMode,
            destination: PermissionDestination::Session,
            mode: Some(PermissionModeName::AcceptEdits),
            behavior: None,
            rules: None,
        };

        let json = serde_json::to_string(&suggestion).unwrap();
        assert!(json.contains("\"type\":\"setMode\""));
        assert!(json.contains("\"mode\":\"acceptEdits\""));
        assert!(json.contains("\"destination\":\"session\""));
        assert!(!json.contains("\"behavior\""));
        assert!(!json.contains("\"rules\""));

        let parsed: PermissionSuggestion = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, suggestion);
    }

    #[test]
    fn test_permission_suggestion_add_rules_roundtrip() {
        let suggestion = PermissionSuggestion {
            suggestion_type: PermissionType::AddRules,
            destination: PermissionDestination::Session,
            mode: None,
            behavior: Some(PermissionBehavior::Allow),
            rules: Some(vec![serde_json::json!({
                "toolName": "Read",
                "ruleContent": "//tmp/**"
            })]),
        };

        let json = serde_json::to_string(&suggestion).unwrap();
        assert!(json.contains("\"type\":\"addRules\""));
        assert!(json.contains("\"behavior\":\"allow\""));
        assert!(json.contains("\"destination\":\"session\""));
        assert!(json.contains("\"rules\""));
        assert!(json.contains("\"toolName\":\"Read\""));
        assert!(!json.contains("\"mode\""));

        let parsed: PermissionSuggestion = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, suggestion);
    }

    #[test]
    fn test_permission_suggestion_add_rules_from_real_json() {
        let json = r#"{"type":"addRules","rules":[{"toolName":"Read","ruleContent":"//tmp/**"}],"behavior":"allow","destination":"session"}"#;

        let parsed: PermissionSuggestion = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.suggestion_type, PermissionType::AddRules);
        assert_eq!(parsed.destination, PermissionDestination::Session);
        assert_eq!(parsed.behavior, Some(PermissionBehavior::Allow));
        assert!(parsed.rules.is_some());
        assert!(parsed.mode.is_none());
    }

    #[test]
    fn test_permission_allow_tool() {
        let perm = Permission::allow_tool("Bash", "npm test");

        assert_eq!(perm.permission_type, PermissionType::AddRules);
        assert_eq!(perm.destination, PermissionDestination::Session);
        assert_eq!(perm.behavior, Some(PermissionBehavior::Allow));
        assert!(perm.mode.is_none());

        let rules = perm.rules.unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].tool_name, "Bash");
        assert_eq!(rules[0].rule_content, "npm test");
    }

    #[test]
    fn test_permission_allow_tool_with_destination() {
        let perm = Permission::allow_tool_with_destination(
            "Read",
            "/tmp/**",
            PermissionDestination::Project,
        );

        assert_eq!(perm.permission_type, PermissionType::AddRules);
        assert_eq!(perm.destination, PermissionDestination::Project);
        assert_eq!(perm.behavior, Some(PermissionBehavior::Allow));

        let rules = perm.rules.unwrap();
        assert_eq!(rules[0].tool_name, "Read");
        assert_eq!(rules[0].rule_content, "/tmp/**");
    }

    #[test]
    fn test_permission_set_mode() {
        let perm = Permission::set_mode(
            PermissionModeName::AcceptEdits,
            PermissionDestination::Session,
        );

        assert_eq!(perm.permission_type, PermissionType::SetMode);
        assert_eq!(perm.destination, PermissionDestination::Session);
        assert_eq!(perm.mode, Some(PermissionModeName::AcceptEdits));
        assert!(perm.behavior.is_none());
        assert!(perm.rules.is_none());
    }

    #[test]
    fn test_permission_serialization() {
        let perm = Permission::allow_tool("Bash", "npm test");
        let json = serde_json::to_string(&perm).unwrap();

        assert!(json.contains("\"type\":\"addRules\""));
        assert!(json.contains("\"destination\":\"session\""));
        assert!(json.contains("\"behavior\":\"allow\""));
        assert!(json.contains("\"toolName\":\"Bash\""));
        assert!(json.contains("\"ruleContent\":\"npm test\""));
    }

    #[test]
    fn test_permission_from_suggestion_set_mode() {
        let suggestion = PermissionSuggestion {
            suggestion_type: PermissionType::SetMode,
            destination: PermissionDestination::Session,
            mode: Some(PermissionModeName::AcceptEdits),
            behavior: None,
            rules: None,
        };

        let perm = Permission::from_suggestion(&suggestion);

        assert_eq!(perm.permission_type, PermissionType::SetMode);
        assert_eq!(perm.destination, PermissionDestination::Session);
        assert_eq!(perm.mode, Some(PermissionModeName::AcceptEdits));
    }

    #[test]
    fn test_permission_from_suggestion_add_rules() {
        let suggestion = PermissionSuggestion {
            suggestion_type: PermissionType::AddRules,
            destination: PermissionDestination::Session,
            mode: None,
            behavior: Some(PermissionBehavior::Allow),
            rules: Some(vec![serde_json::json!({
                "toolName": "Read",
                "ruleContent": "/tmp/**"
            })]),
        };

        let perm = Permission::from_suggestion(&suggestion);

        assert_eq!(perm.permission_type, PermissionType::AddRules);
        assert_eq!(perm.behavior, Some(PermissionBehavior::Allow));

        let rules = perm.rules.unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].tool_name, "Read");
        assert_eq!(rules[0].rule_content, "/tmp/**");
    }

    #[test]
    fn test_permission_result_allow_with_typed_permissions() {
        let result = PermissionResult::allow_with_typed_permissions(
            serde_json::json!({"command": "npm test"}),
            vec![Permission::allow_tool("Bash", "npm test")],
        );

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"behavior\":\"allow\""));
        assert!(json.contains("\"updatedPermissions\""));
        assert!(json.contains("\"toolName\":\"Bash\""));
    }

    #[test]
    fn test_tool_permission_request_allow_and_remember() {
        let req = ToolPermissionRequest {
            tool_name: "Bash".to_string(),
            input: serde_json::json!({"command": "npm test"}),
            permission_suggestions: vec![],
            blocked_path: None,
            decision_reason: None,
            tool_use_id: None,
        };

        let response =
            req.allow_and_remember(vec![Permission::allow_tool("Bash", "npm test")], "req-123");
        let message: ControlResponseMessage = response.into();
        let json = serde_json::to_string(&message).unwrap();

        assert!(json.contains("\"type\":\"control_response\""));
        assert!(json.contains("\"behavior\":\"allow\""));
        assert!(json.contains("\"updatedPermissions\""));
        assert!(json.contains("\"toolName\":\"Bash\""));
    }

    #[test]
    fn test_tool_permission_request_allow_and_remember_suggestion() {
        let req = ToolPermissionRequest {
            tool_name: "Bash".to_string(),
            input: serde_json::json!({"command": "npm test"}),
            permission_suggestions: vec![PermissionSuggestion {
                suggestion_type: PermissionType::SetMode,
                destination: PermissionDestination::Session,
                mode: Some(PermissionModeName::AcceptEdits),
                behavior: None,
                rules: None,
            }],
            blocked_path: None,
            decision_reason: None,
            tool_use_id: None,
        };

        let response = req.allow_and_remember_suggestion("req-123");
        assert!(response.is_some());

        let message: ControlResponseMessage = response.unwrap().into();
        let json = serde_json::to_string(&message).unwrap();

        assert!(json.contains("\"type\":\"setMode\""));
        assert!(json.contains("\"mode\":\"acceptEdits\""));
    }

    #[test]
    fn test_tool_permission_request_allow_and_remember_suggestion_none() {
        let req = ToolPermissionRequest {
            tool_name: "Bash".to_string(),
            input: serde_json::json!({"command": "npm test"}),
            permission_suggestions: vec![], // No suggestions
            blocked_path: None,
            decision_reason: None,
            tool_use_id: None,
        };

        let response = req.allow_and_remember_suggestion("req-123");
        assert!(response.is_none());
    }
}
