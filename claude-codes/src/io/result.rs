use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::fmt;

/// Result message for completed queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultMessage {
    pub subtype: ResultSubtype,
    pub is_error: bool,
    pub duration_ms: u64,
    pub duration_api_ms: u64,

    /// Time to first token, in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttft_ms: Option<u64>,

    /// Time to first streamed token, in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttft_stream_ms: Option<u64>,

    /// Time from session start until the first request was issued, in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_to_request_ms: Option<u64>,

    /// Time from spawning a worker/spare until the first request was issued, in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_to_request_from_spawn_ms: Option<u64>,

    /// Whether a warm spare process was claimed for this request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warm_spare_claimed: Option<bool>,

    /// Epoch-ish timestamp origin used by CLI timing instrumentation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_origin_ms: Option<u64>,

    /// Wall-clock epoch milliseconds when the API request was sent
    /// (fractional; from CLI timing instrumentation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_sent_wall_ms: Option<f64>,

    /// Wire uuid of the user message this result answers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_message_uuid: Option<String>,

    /// Client uuids of every user message whose prompt this turn consumed, in
    /// consumption order — all members of a prompt batch the host merged into
    /// this one turn, plus any queued user message folded into the running
    /// turn between tool rounds. Always contains `user_message_uuid`; at most
    /// 64 entries. Absent on delivery-failure/zeroed results and from CLIs
    /// before 2.1.259 (fall back to `user_message_uuid`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub user_message_uuids: Vec<String>,

    /// User-initiated sends still waiting in the command queue when this
    /// result was produced. Greater than 0 means at least one more user turn
    /// follows without further input, barring cancellation. Absent on fatal
    /// startup results, on surfaces without a command queue, and from CLIs
    /// before 2.1.259.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queued_turn_count: Option<u64>,

    pub num_turns: i32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,

    #[serde(alias = "sessionId")]
    pub session_id: String,
    pub total_cost_usd: f64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageInfo>,

    /// Tools that were blocked due to permission denials during the session
    #[serde(default)]
    pub permission_denials: Vec<PermissionDenial>,

    /// Error messages when `is_error` is true.
    ///
    /// Contains human-readable error strings (e.g., "No conversation found with session ID: ...").
    /// This allows typed access to error conditions without needing to serialize to JSON and search.
    #[serde(default)]
    pub errors: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,

    /// HTTP status code when the result is an API error (e.g., 429, 500, 529)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_error_status: Option<u16>,

    /// Why generation stopped (e.g., end_turn, max_tokens)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,

    /// Why the session ended (e.g., "completed")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_reason: Option<String>,

    /// Fast mode toggle state (e.g., "off")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fast_mode_state: Option<String>,

    /// Why fast mode can't serve right now. Absent when nothing blocks it.
    /// A paused-after-rate-limit run is not reported here; it rides
    /// `fast_mode_state` as `"cooldown"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fast_mode_disabled_reason: Option<FastModeDisabledReason>,

    /// Per-model cost breakdown, keyed by model name (e.g. `"claude-opus-4-8"`).
    #[serde(skip_serializing_if = "Option::is_none", rename = "modelUsage")]
    pub model_usage: Option<std::collections::BTreeMap<String, ModelUsageEntry>>,

    /// Subagents started through the Agent tool in this session, as running
    /// totals. Cumulative like `modelUsage`: read the latest result rather
    /// than summing across results. Absent from CLIs before 2.1.239.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_stats: Option<SubagentStats>,

    /// Structured-output payload returned by the model, when enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_output: Option<Value>,

    /// Deferred tool-use termination payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deferred_tool_use: Option<DeferredToolUse>,

    /// Provenance of the message/run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<super::message_types::MessageOrigin>,
}

/// Usage and cost for a single model within a session, as found in
/// [`ResultMessage::model_usage`].
///
/// The `extra` field captures any keys the CLI adds that aren't modeled here,
/// so new wire fields deserialize without error.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsageEntry {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default, rename = "costUSD")]
    pub cost_usd: f64,
    #[serde(default)]
    pub web_search_requests: u32,
    #[serde(default)]
    pub context_window: u64,
    #[serde(default)]
    pub max_output_tokens: u64,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Tool use deferred by a terminal result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeferredToolUse {
    pub id: String,
    pub name: String,
    pub input: Value,
}

/// Running totals for subagents started through the Agent tool, carried on
/// `result` frames (CLI 2.1.239+) as [`ResultMessage::subagent_stats`].
///
/// Cumulative for the session: a resumed session starts fresh, and a
/// mid-session `/clear` zeroes it — though a background subagent that outlives
/// the `/clear` still records its outcome, so `completed`, `failed`, and
/// `killed` can then exceed `spawned`. Forked skills, workflows, teammates,
/// and other internal agents are not counted.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SubagentStats {
    /// Subagents actually started; a refused or failed launch is not counted.
    pub spawned: u64,
    /// Spawns by the `run_in_background` value the model passed; all count as
    /// `unset` while the parameter is not offered.
    pub requested: SubagentSpawnRequests,
    /// Spawns that started in the background after defaults and session
    /// settings; the rest blocked the spawning tool call.
    pub started_in_background: u64,
    /// Spawns by agent type.
    #[serde(default)]
    pub by_type: std::collections::BTreeMap<String, u64>,
    /// Deepest spawn: 1 = started by the main thread, 2 = by a depth-1
    /// subagent.
    pub max_depth: u64,
    /// Spawns made from inside another subagent (depth > 1).
    pub spawned_by_subagents: u64,
    pub completed: u64,
    pub failed: u64,
    /// Subagents stopped before finishing, by who stopped them.
    pub killed: SubagentKillCounts,
    /// Agent tool calls turned down because a limit was reached (other
    /// denials, such as an unknown agent type, are not counted).
    pub refused: SubagentRefusalCounts,
}

/// Spawn counts by the `run_in_background` value the model passed, carried in
/// [`SubagentStats::requested`].
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SubagentSpawnRequests {
    pub background: u64,
    pub foreground: u64,
    pub unset: u64,
}

/// Subagents stopped before finishing, carried in [`SubagentStats::killed`].
/// `parent` = by another agent through TaskStop; `system` = by Claude Code
/// itself; `user` = any other stop.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SubagentKillCounts {
    pub parent: u64,
    pub user: u64,
    pub system: u64,
}

/// Agent tool calls refused at a limit, carried in [`SubagentStats::refused`].
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SubagentRefusalCounts {
    pub depth_limit: u64,
    pub concurrency_limit: u64,
    pub budget: u64,
}

/// A record of a tool permission that was denied during the session.
///
/// This is included in `ResultMessage.permission_denials` to provide a summary
/// of all permission denials that occurred.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PermissionDenial {
    /// The name of the tool that was blocked (e.g., "Bash", "Write")
    pub tool_name: String,

    /// The input that was passed to the tool
    pub tool_input: Value,

    /// The unique identifier for this tool use request
    pub tool_use_id: String,
}

/// Why fast mode can't serve right now, carried on `result` frames and
/// `system/init` (CLI 2.1.219+). Absent when nothing blocks fast mode.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FastModeDisabledReason {
    /// Free-tier account.
    Free,
    /// Disabled by user preference.
    Preference,
    /// Extra-usage purchases are disabled for the account.
    ExtraUsageDisabled,
    /// A network error prevented fast-mode eligibility from resolving.
    NetworkError,
    /// The CLI could not determine the reason.
    UnknownReason,
    /// Not a first-party API session (e.g. Bedrock/Vertex).
    NotFirstParty,
    /// Disabled via environment variable.
    DisabledByEnv,
    /// The active model does not support fast mode.
    ModelNotAllowed,
    /// SDK sessions must opt in to fast mode.
    SdkOptInRequired,
    /// Eligibility is still being determined.
    Pending,
    /// A reason not yet known to this version of the crate.
    Unknown(String),
}

impl FastModeDisabledReason {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Free => "free",
            Self::Preference => "preference",
            Self::ExtraUsageDisabled => "extra_usage_disabled",
            Self::NetworkError => "network_error",
            Self::UnknownReason => "unknown",
            Self::NotFirstParty => "not_first_party",
            Self::DisabledByEnv => "disabled_by_env",
            Self::ModelNotAllowed => "model_not_allowed",
            Self::SdkOptInRequired => "sdk_opt_in_required",
            Self::Pending => "pending",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for FastModeDisabledReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for FastModeDisabledReason {
    fn from(s: &str) -> Self {
        match s {
            "free" => Self::Free,
            "preference" => Self::Preference,
            "extra_usage_disabled" => Self::ExtraUsageDisabled,
            "network_error" => Self::NetworkError,
            "unknown" => Self::UnknownReason,
            "not_first_party" => Self::NotFirstParty,
            "disabled_by_env" => Self::DisabledByEnv,
            "model_not_allowed" => Self::ModelNotAllowed,
            "sdk_opt_in_required" => Self::SdkOptInRequired,
            "pending" => Self::Pending,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for FastModeDisabledReason {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for FastModeDisabledReason {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// Result subtypes
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResultSubtype {
    Success,
    ErrorMaxTurns,
    ErrorDuringExecution,
    ErrorMaxBudgetUsd,
    ErrorMaxStructuredOutputRetries,
    Unknown(String),
}

impl ResultSubtype {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Success => "success",
            Self::ErrorMaxTurns => "error_max_turns",
            Self::ErrorDuringExecution => "error_during_execution",
            Self::ErrorMaxBudgetUsd => "error_max_budget_usd",
            Self::ErrorMaxStructuredOutputRetries => "error_max_structured_output_retries",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for ResultSubtype {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for ResultSubtype {
    fn from(s: &str) -> Self {
        match s {
            "success" => Self::Success,
            "error_max_turns" => Self::ErrorMaxTurns,
            "error_during_execution" => Self::ErrorDuringExecution,
            "error_max_budget_usd" => Self::ErrorMaxBudgetUsd,
            "error_max_structured_output_retries" => Self::ErrorMaxStructuredOutputRetries,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for ResultSubtype {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ResultSubtype {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// Usage information for the request.
///
/// **These counters are accumulated roll-ups, not snapshots.** The CLI sums
/// usage field-by-field across every API call ("iteration") in the turn's
/// tool-use loop, so on a turn with N iterations each counter is the sum of
/// all N. This is the right number for **cost**, and the wrong number for
/// **context occupancy**: `cache_read_input_tokens` in particular re-counts
/// the cached context on every iteration and can exceed the model's context
/// window several times over on tool-heavy turns. To estimate context
/// occupancy, use the *last* entry of [`iterations`](Self::iterations)
/// instead (the CLI itself does exactly this).
///
/// Note: the `result` frame's usage covers the **main agent only** — the
/// subagent (`Task` / sidechain) token rollup the CLI renders as
/// `<subagent_tokens>` / `<agent_count>` is not carried here or anywhere
/// else on the wire. Accumulate it from `Task` tool results with
/// [`SubagentUsageRollup`](crate::SubagentUsageRollup).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfo {
    /// Sum of fresh (uncached) input tokens across all iterations of the turn.
    #[serde(default)]
    pub input_tokens: u32,
    /// Sum of cache-write input tokens across all iterations of the turn.
    #[serde(default)]
    pub cache_creation_input_tokens: u32,
    /// Sum of cache-read input tokens across all iterations of the turn.
    /// Re-counts the cached context every iteration — **not** a measure of
    /// context occupancy; see the type-level docs.
    #[serde(default)]
    pub cache_read_input_tokens: u32,
    /// Sum of output tokens across all iterations of the turn.
    #[serde(default)]
    pub output_tokens: u32,
    #[serde(default)]
    pub server_tool_use: ServerToolUse,
    #[serde(default)]
    pub service_tier: String,

    /// Cache creation breakdown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation: Option<super::message_types::CacheCreationDetails>,

    /// Inference geography (e.g., "not_available")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inference_geo: Option<String>,

    /// Per-iteration usage breakdown **within** this turn — one entry per
    /// API call in the tool-use loop, in order. The last entry reflects the
    /// final API call and is what the CLI reads for context-size estimates.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub iterations: Vec<UsageIteration>,

    /// Speed tier (e.g., "standard")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<String>,

    /// Output-token breakdown (CLI 2.1.232+): currently the thinking-token
    /// share of `output_tokens`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens_details: Option<OutputTokensDetails>,
}

/// Breakdown of a turn's output tokens, carried in
/// [`UsageInfo::output_tokens_details`].
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutputTokensDetails {
    /// Output tokens spent on extended thinking.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_tokens: Option<u64>,
}

/// Usage for a single API call ("iteration") within a turn's tool-use loop.
///
/// Carried in [`UsageInfo::iterations`]. The cache fields are optional on
/// the wire: some frames carry only `input_tokens` + `output_tokens`
/// (observed with `type: "turn"`), while others carry the full cache
/// breakdown (observed with `type: "message"` in captured subagent
/// sessions). The CLI computes its context estimate from
/// `input_tokens + output_tokens` of the **last** iteration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageIteration {
    #[serde(default)]
    pub input_tokens: u32,
    #[serde(default)]
    pub output_tokens: u32,
    /// Cache-read input tokens for this iteration, when carried.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
    /// Cache-write input tokens for this iteration, when carried.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,
    /// Cache-write breakdown by TTL, when carried.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_creation: Option<super::message_types::CacheCreationDetails>,
    /// Iteration kind; `"turn"` and `"message"` observed on the wire.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// Server tool usage information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerToolUse {
    #[serde(default)]
    pub web_search_requests: u32,
    /// Number of web fetch requests made
    #[serde(default)]
    pub web_fetch_requests: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::ClaudeOutput;

    #[test]
    fn test_deserialize_result_message() {
        let json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 1,
            "result": "Done",
            "session_id": "123",
            "total_cost_usd": 0.01,
            "permission_denials": []
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert!(!output.is_error());
    }

    #[test]
    fn test_result_subtype_new_and_unknown_values_do_not_fail() {
        let json = r#"{
            "type": "result",
            "subtype": "error_max_budget_usd",
            "is_error": true,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 1,
            "session_id": "123",
            "total_cost_usd": 0.01
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let ClaudeOutput::Result(result) = output else {
            panic!("Expected Result");
        };
        assert_eq!(result.subtype, ResultSubtype::ErrorMaxBudgetUsd);

        let json = r#"{
            "type": "result",
            "subtype": "future_result_subtype",
            "is_error": true,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 1,
            "session_id": "123",
            "total_cost_usd": 0.01
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let ClaudeOutput::Result(result) = output else {
            panic!("Expected Result");
        };
        assert_eq!(
            result.subtype,
            ResultSubtype::Unknown("future_result_subtype".to_string())
        );
    }

    #[test]
    fn test_deserialize_result_with_permission_denials() {
        let json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 2,
            "result": "Done",
            "session_id": "123",
            "total_cost_usd": 0.01,
            "permission_denials": [
                {
                    "tool_name": "Bash",
                    "tool_input": {"command": "rm -rf /", "description": "Delete everything"},
                    "tool_use_id": "toolu_123"
                }
            ]
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::Result(result) = output {
            assert_eq!(result.permission_denials.len(), 1);
            assert_eq!(result.permission_denials[0].tool_name, "Bash");
            assert_eq!(result.permission_denials[0].tool_use_id, "toolu_123");
            assert_eq!(
                result.permission_denials[0]
                    .tool_input
                    .get("command")
                    .unwrap(),
                "rm -rf /"
            );
        } else {
            panic!("Expected Result");
        }
    }

    #[test]
    fn test_permission_denial_roundtrip() {
        let denial = PermissionDenial {
            tool_name: "Write".to_string(),
            tool_input: serde_json::json!({"file_path": "/etc/passwd", "content": "bad"}),
            tool_use_id: "toolu_456".to_string(),
        };

        let json = serde_json::to_string(&denial).unwrap();
        assert!(json.contains("\"tool_name\":\"Write\""));
        assert!(json.contains("\"tool_use_id\":\"toolu_456\""));
        assert!(json.contains("/etc/passwd"));

        let parsed: PermissionDenial = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, denial);
    }

    #[test]
    fn test_deserialize_result_message_with_errors() {
        let json = r#"{
            "type": "result",
            "subtype": "error_during_execution",
            "duration_ms": 0,
            "duration_api_ms": 0,
            "is_error": true,
            "num_turns": 0,
            "session_id": "27934753-425a-4182-892c-6b1c15050c3f",
            "total_cost_usd": 0,
            "errors": ["No conversation found with session ID: d56965c9-c855-4042-a8f5-f12bbb14d6f6"],
            "permission_denials": []
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert!(output.is_error());

        if let ClaudeOutput::Result(res) = output {
            assert!(res.is_error);
            assert_eq!(res.errors.len(), 1);
            assert!(res.errors[0].contains("No conversation found"));
        } else {
            panic!("Expected Result message");
        }
    }

    #[test]
    fn test_deserialize_result_message_errors_defaults_empty() {
        let json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 1,
            "session_id": "123",
            "total_cost_usd": 0.01
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::Result(res) = output {
            assert!(res.errors.is_empty());
        } else {
            panic!("Expected Result message");
        }
    }

    #[test]
    fn test_result_message_errors_roundtrip() {
        let json = r#"{
            "type": "result",
            "subtype": "error_during_execution",
            "is_error": true,
            "duration_ms": 0,
            "duration_api_ms": 0,
            "num_turns": 0,
            "session_id": "test-session",
            "total_cost_usd": 0.0,
            "errors": ["Error 1", "Error 2"]
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let reserialized = serde_json::to_string(&output).unwrap();

        assert!(reserialized.contains("Error 1"));
        assert!(reserialized.contains("Error 2"));
    }

    #[test]
    fn test_result_with_new_fields() {
        let json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 5000,
            "duration_api_ms": 4500,
            "num_turns": 1,
            "result": "Done",
            "session_id": "abc",
            "total_cost_usd": 0.06,
            "api_error_status": null,
            "stop_reason": "end_turn",
            "terminal_reason": "completed",
            "fast_mode_state": "off",
            "modelUsage": {
                "claude-opus-4-7[1m]": {
                    "inputTokens": 3817,
                    "outputTokens": 14,
                    "costUSD": 0.06
                }
            },
            "usage": {
                "input_tokens": 3817,
                "output_tokens": 14,
                "cache_creation_input_tokens": 3540,
                "cache_read_input_tokens": 0,
                "server_tool_use": {
                    "web_search_requests": 0,
                    "web_fetch_requests": 2
                },
                "service_tier": "standard",
                "inference_geo": "not_available",
                "speed": "standard",
                "iterations": [
                    {"input_tokens": 3817, "output_tokens": 14, "type": "turn"}
                ]
            }
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::Result(res) = output {
            assert_eq!(res.stop_reason.as_deref(), Some("end_turn"));
            assert_eq!(res.terminal_reason.as_deref(), Some("completed"));
            assert_eq!(res.fast_mode_state.as_deref(), Some("off"));
            let model_usage = res.model_usage.as_ref().unwrap();
            let entry = model_usage
                .get("claude-opus-4-7[1m]")
                .expect("per-model entry present");
            assert_eq!(entry.input_tokens, 3817);
            assert_eq!(entry.output_tokens, 14);
            assert_eq!(entry.cost_usd, 0.06);
            assert!(res.api_error_status.is_none());

            let usage = res.usage.unwrap();
            assert_eq!(usage.server_tool_use.web_fetch_requests, 2);
            assert_eq!(usage.inference_geo.as_deref(), Some("not_available"));
            assert_eq!(usage.speed.as_deref(), Some("standard"));
            assert_eq!(usage.iterations.len(), 1);
            assert_eq!(usage.iterations[0].input_tokens, 3817);
            assert_eq!(usage.iterations[0].output_tokens, 14);
            assert_eq!(usage.iterations[0].kind.as_deref(), Some("turn"));
        } else {
            panic!("Expected Result");
        }
    }

    #[test]
    fn test_result_backwards_compatible_without_new_fields() {
        // Verify old-format messages still parse fine
        let json = r#"{
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 100,
            "duration_api_ms": 200,
            "num_turns": 1,
            "session_id": "abc",
            "total_cost_usd": 0.01
        }"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        if let ClaudeOutput::Result(res) = output {
            assert!(res.api_error_status.is_none());
            assert!(res.stop_reason.is_none());
            assert!(res.terminal_reason.is_none());
            assert!(res.fast_mode_state.is_none());
            assert!(res.model_usage.is_none());
        } else {
            panic!("Expected Result");
        }
    }

    #[test]
    fn test_result_fast_mode_disabled_reason() {
        let json = r#"{
            "type":"result","subtype":"success","is_error":false,
            "duration_ms":100,"duration_api_ms":80,"num_turns":1,
            "session_id":"s1","total_cost_usd":0.01,
            "fast_mode_state":"off",
            "fast_mode_disabled_reason":"sdk_opt_in_required"
        }"#;
        let output: crate::ClaudeOutput = serde_json::from_str(json).unwrap();
        let crate::ClaudeOutput::Result(res) = &output else {
            panic!("expected Result");
        };
        assert_eq!(
            res.fast_mode_disabled_reason,
            Some(FastModeDisabledReason::SdkOptInRequired)
        );
        assert!(serde_json::to_string(&output)
            .unwrap()
            .contains("\"fast_mode_disabled_reason\":\"sdk_opt_in_required\""));

        // Unknown reasons survive decode and round-trip verbatim; the wire
        // literal "unknown" maps to the typed UnknownReason, not the fallback.
        assert_eq!(
            FastModeDisabledReason::from("unknown"),
            FastModeDisabledReason::UnknownReason
        );
        let novel = FastModeDisabledReason::from("solar_flare");
        assert_eq!(novel, FastModeDisabledReason::Unknown("solar_flare".into()));
        assert_eq!(novel.as_str(), "solar_flare");
    }

    #[test]
    fn test_result_timing_and_user_message_uuid_fields() {
        let json = r#"{
            "type":"result","subtype":"success","is_error":false,
            "duration_ms":100,"duration_api_ms":80,"num_turns":1,
            "session_id":"s1","total_cost_usd":0.01,
            "request_sent_wall_ms":1753212345678.25,
            "user_message_uuid":"um-1"
        }"#;
        let output: crate::ClaudeOutput = serde_json::from_str(json).unwrap();
        let crate::ClaudeOutput::Result(res) = &output else {
            panic!("expected Result");
        };
        assert_eq!(res.request_sent_wall_ms, Some(1753212345678.25));
        assert_eq!(res.user_message_uuid.as_deref(), Some("um-1"));

        let reserialized = serde_json::to_string(&output).unwrap();
        assert!(reserialized.contains("\"user_message_uuid\":\"um-1\""));
    }

    #[test]
    fn test_result_queued_turn_count_and_user_message_uuids() {
        // CLI 2.1.259 additive fields on the result frame.
        let json = r#"{
            "type":"result","subtype":"success","is_error":false,
            "duration_ms":100,"duration_api_ms":80,"num_turns":1,
            "session_id":"s1","total_cost_usd":0.01,
            "user_message_uuid":"um-2",
            "user_message_uuids":["um-1","um-2"],
            "queued_turn_count":3
        }"#;
        let output: crate::ClaudeOutput = serde_json::from_str(json).unwrap();
        let crate::ClaudeOutput::Result(res) = &output else {
            panic!("expected Result");
        };
        assert_eq!(res.user_message_uuids, vec!["um-1", "um-2"]);
        assert_eq!(res.queued_turn_count, Some(3));

        let reserialized = serde_json::to_string(&output).unwrap();
        assert!(reserialized.contains("\"user_message_uuids\":[\"um-1\",\"um-2\"]"));
        assert!(reserialized.contains("\"queued_turn_count\":3"));

        // Both fields are absent from older producers and must default cleanly.
        let old = r#"{
            "type":"result","subtype":"success","is_error":false,
            "duration_ms":100,"duration_api_ms":80,"num_turns":1,
            "session_id":"s1","total_cost_usd":0.01
        }"#;
        let output: crate::ClaudeOutput = serde_json::from_str(old).unwrap();
        let crate::ClaudeOutput::Result(res) = &output else {
            panic!("expected Result");
        };
        assert!(res.user_message_uuids.is_empty());
        assert_eq!(res.queued_turn_count, None);
        let reserialized = serde_json::to_string(&output).unwrap();
        assert!(!reserialized.contains("user_message_uuids"));
        assert!(!reserialized.contains("queued_turn_count"));
    }
}
