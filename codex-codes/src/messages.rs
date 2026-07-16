//! Typed dispatch for app-server notifications and server-to-client requests.
//!
//! The Codex app-server speaks JSON-RPC where every message carries a
//! `method` discriminant alongside a free-form `params` blob. This module
//! lifts that loose envelope into closed enums — [`Notification`] for
//! server-initiated notifications and [`ServerRequest`] for server-initiated
//! requests (the approval flow). Each variant wraps a typed param struct
//! from [`crate::protocol`].
//!
//! The pattern mirrors the [`ContentBlock`] dispatch in the sibling
//! `claude-codes` crate: hand-written [`Serialize`]/[`Deserialize`] impls
//! inspect the discriminant, route known cases through `serde_json::from_value`
//! into the typed struct, and route unknown methods into an `Unknown`
//! variant — preserving the raw payload for forward compatibility with
//! future codex versions.
//!
//! ## Typing contract
//!
//! - Unknown methods route to [`Notification::Unknown`] / [`ServerRequest::Unknown`]
//!   without error. Encountering one in production typically means the
//!   installed Codex CLI is newer than the bindings.
//! - Known methods whose payload fails to deserialize **do** cause an error.
//!   If you see one, the typed binding in [`crate::protocol`] is out of
//!   sync with the wire format and needs to be updated.

use crate::error::{Error, ParseError};
use crate::jsonrpc::{JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, RequestId};
use crate::protocol::{
    methods, AccountLoginCompletedNotification, AccountRateLimitsUpdatedNotification,
    AccountUpdatedNotification, AgentMessageDeltaNotification, AppListUpdatedNotification,
    CommandExecOutputDeltaNotification, CommandExecutionOutputDeltaNotification,
    CommandExecutionRequestApprovalParams, ConfigWarningNotification, ContextCompactedNotification,
    DeprecationNoticeNotification, EnvironmentConnectionNotification, ErrorNotification,
    ExternalAgentConfigImportCompletedNotification, ExternalAgentConfigImportProgressNotification,
    FileChangeOutputDeltaNotification, FileChangePatchUpdatedNotification,
    FileChangeRequestApprovalParams, FsChangedNotification,
    FuzzyFileSearchSessionCompletedNotification, FuzzyFileSearchSessionUpdatedNotification,
    GuardianWarningNotification, HookCompletedNotification, HookStartedNotification,
    ItemCompletedNotification, ItemGuardianApprovalReviewCompletedNotification,
    ItemGuardianApprovalReviewStartedNotification, ItemStartedNotification,
    McpServerOauthLoginCompletedNotification, McpServerStatusUpdatedNotification,
    McpToolCallProgressNotification, ModelReroutedNotification,
    ModelSafetyBufferingUpdatedNotification, ModelVerificationNotification, PlanDeltaNotification,
    ProcessExitedNotification, ProcessOutputDeltaNotification,
    ReasoningSummaryPartAddedNotification, ReasoningSummaryTextDeltaNotification,
    ReasoningTextDeltaNotification, RemoteControlStatusChangedNotification,
    ServerRequestResolvedNotification, SkillsChangedNotification, TerminalInteractionNotification,
    ThreadArchivedNotification, ThreadClosedNotification, ThreadDeletedNotification,
    ThreadGoalClearedNotification, ThreadGoalUpdatedNotification, ThreadNameUpdatedNotification,
    ThreadRealtimeClosedNotification, ThreadRealtimeErrorNotification,
    ThreadRealtimeItemAddedNotification, ThreadRealtimeOutputAudioDeltaNotification,
    ThreadRealtimeSdpNotification, ThreadRealtimeStartedNotification,
    ThreadRealtimeTranscriptDeltaNotification, ThreadRealtimeTranscriptDoneNotification,
    ThreadSettingsUpdatedNotification, ThreadStartedNotification, ThreadStatusChangedNotification,
    ThreadTokenUsageUpdatedNotification, ThreadUnarchivedNotification, TurnCompletedNotification,
    TurnDiffUpdatedNotification, TurnModerationMetadataNotification, TurnPlanUpdatedNotification,
    TurnStartedNotification, WarningNotification, WindowsSandboxSetupCompletedNotification,
    WindowsWorldWritableWarningNotification,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

/// A server-to-client notification.
///
/// Each variant maps to a single `method` string on the wire. The `Unknown`
/// variant captures methods this crate version doesn't model yet, preserving
/// the raw payload for inspection.
#[derive(Debug, Clone)]
pub enum Notification {
    /// `thread/started`
    ThreadStarted(ThreadStartedNotification),
    /// `thread/status/changed`
    ThreadStatusChanged(ThreadStatusChangedNotification),
    /// `thread/tokenUsage/updated`
    ThreadTokenUsageUpdated(ThreadTokenUsageUpdatedNotification),
    /// `turn/started`
    TurnStarted(TurnStartedNotification),
    /// `turn/completed`
    TurnCompleted(TurnCompletedNotification),
    /// `item/started`
    ItemStarted(ItemStartedNotification),
    /// `item/completed`
    ItemCompleted(ItemCompletedNotification),
    /// `item/agentMessage/delta`
    AgentMessageDelta(AgentMessageDeltaNotification),
    /// `item/commandExecution/outputDelta`
    CmdOutputDelta(CommandExecutionOutputDeltaNotification),
    /// `item/fileChange/outputDelta`
    FileChangeOutputDelta(FileChangeOutputDeltaNotification),
    /// `item/reasoning/summaryTextDelta`
    ReasoningDelta(ReasoningSummaryTextDeltaNotification),
    /// `error`
    Error(ErrorNotification),
    /// `account/rateLimits/updated`
    AccountRateLimitsUpdated(AccountRateLimitsUpdatedNotification),
    /// `mcpServer/startupStatus/updated`
    McpServerStartupStatusUpdated(McpServerStatusUpdatedNotification),
    /// `remoteControl/status/changed`
    RemoteControlStatusChanged(RemoteControlStatusChangedNotification),
    /// `mcpServer/oauthLogin/completed`
    McpServerOauthLoginCompleted(McpServerOauthLoginCompletedNotification),
    /// `item/fileChange/patchUpdated`
    FileChangePatchUpdated(FileChangePatchUpdatedNotification),
    /// `item/plan/delta` (EXPERIMENTAL)
    PlanDelta(PlanDeltaNotification),
    /// `turn/plan/updated`
    TurnPlanUpdated(TurnPlanUpdatedNotification),
    /// `turn/diff/updated`
    TurnDiffUpdated(TurnDiffUpdatedNotification),
    /// `item/reasoning/summaryPartAdded`
    ReasoningSummaryPartAdded(ReasoningSummaryPartAddedNotification),
    /// `item/reasoning/textDelta`
    ReasoningTextDelta(ReasoningTextDeltaNotification),
    /// `account/login/completed`
    AccountLoginCompleted(AccountLoginCompletedNotification),
    /// `deprecationNotice`
    DeprecationNotice(DeprecationNoticeNotification),
    /// `guardianWarning`
    GuardianWarning(GuardianWarningNotification),
    /// `warning`
    Warning(WarningNotification),
    /// `thread/archived`
    ThreadArchived(ThreadArchivedNotification),
    /// `thread/closed`
    ThreadClosed(ThreadClosedNotification),
    /// `thread/deleted`
    ThreadDeleted(ThreadDeletedNotification),
    /// `thread/unarchived`
    ThreadUnarchived(ThreadUnarchivedNotification),
    /// `thread/goal/cleared`
    ThreadGoalCleared(ThreadGoalClearedNotification),
    /// `thread/name/updated`
    ThreadNameUpdated(ThreadNameUpdatedNotification),
    /// `skills/changed`
    SkillsChanged(SkillsChangedNotification),
    /// `fs/changed`
    FsChanged(FsChangedNotification),
    /// `configWarning`
    ConfigWarning(ConfigWarningNotification),
    /// `account/updated`
    AccountUpdated(AccountUpdatedNotification),
    /// `app/list/updated`
    AppListUpdated(AppListUpdatedNotification),
    /// `command/exec/outputDelta`
    CommandExecOutputDelta(CommandExecOutputDeltaNotification),
    /// `externalAgentConfig/import/completed`
    ExternalAgentConfigImportCompleted(ExternalAgentConfigImportCompletedNotification),
    /// `fuzzyFileSearch/sessionCompleted`
    FuzzyFileSearchSessionCompleted(FuzzyFileSearchSessionCompletedNotification),
    /// `fuzzyFileSearch/sessionUpdated`
    FuzzyFileSearchSessionUpdated(FuzzyFileSearchSessionUpdatedNotification),
    /// `hook/completed`
    HookCompleted(HookCompletedNotification),
    /// `hook/started`
    HookStarted(HookStartedNotification),
    /// `item/autoApprovalReview/completed`
    ItemGuardianApprovalReviewCompleted(ItemGuardianApprovalReviewCompletedNotification),
    /// `item/autoApprovalReview/started`
    ItemGuardianApprovalReviewStarted(ItemGuardianApprovalReviewStartedNotification),
    /// `item/commandExecution/terminalInteraction`
    TerminalInteraction(TerminalInteractionNotification),
    /// `item/mcpToolCall/progress`
    McpToolCallProgress(McpToolCallProgressNotification),
    /// `model/rerouted`
    ModelRerouted(ModelReroutedNotification),
    /// `model/verification`
    ModelVerification(ModelVerificationNotification),
    /// `process/exited`
    ProcessExited(ProcessExitedNotification),
    /// `process/outputDelta`
    ProcessOutputDelta(ProcessOutputDeltaNotification),
    /// `serverRequest/resolved`
    ServerRequestResolved(ServerRequestResolvedNotification),
    /// `thread/compacted`
    ContextCompacted(ContextCompactedNotification),
    /// `thread/goal/updated`
    ThreadGoalUpdated(ThreadGoalUpdatedNotification),
    /// `thread/realtime/closed`
    ThreadRealtimeClosed(ThreadRealtimeClosedNotification),
    /// `thread/realtime/error`
    ThreadRealtimeError(ThreadRealtimeErrorNotification),
    /// `thread/realtime/itemAdded`
    ThreadRealtimeItemAdded(ThreadRealtimeItemAddedNotification),
    /// `thread/realtime/outputAudio/delta`
    ThreadRealtimeOutputAudioDelta(ThreadRealtimeOutputAudioDeltaNotification),
    /// `thread/realtime/sdp`
    ThreadRealtimeSdp(ThreadRealtimeSdpNotification),
    /// `thread/realtime/started`
    ThreadRealtimeStarted(ThreadRealtimeStartedNotification),
    /// `thread/realtime/transcript/delta`
    ThreadRealtimeTranscriptDelta(ThreadRealtimeTranscriptDeltaNotification),
    /// `thread/realtime/transcript/done`
    ThreadRealtimeTranscriptDone(ThreadRealtimeTranscriptDoneNotification),
    /// `windows/worldWritableWarning`
    WindowsWorldWritableWarning(WindowsWorldWritableWarningNotification),
    /// `windowsSandbox/setupCompleted`
    WindowsSandboxSetupCompleted(WindowsSandboxSetupCompletedNotification),
    /// `thread/settings/updated`
    ThreadSettingsUpdated(ThreadSettingsUpdatedNotification),
    /// `turn/moderationMetadata`
    TurnModerationMetadata(TurnModerationMetadataNotification),
    /// `externalAgentConfig/import/progress`
    ExternalAgentConfigImportProgress(ExternalAgentConfigImportProgressNotification),
    /// `model/safetyBuffering/updated`
    ModelSafetyBufferingUpdated(ModelSafetyBufferingUpdatedNotification),
    /// `thread/environment/connected`
    ThreadEnvironmentConnected(EnvironmentConnectionNotification),
    /// `thread/environment/disconnected`
    ThreadEnvironmentDisconnected(EnvironmentConnectionNotification),
    /// A method this crate version does not yet model. The raw params are
    /// preserved for caller inspection. Encountering this typically means
    /// the installed codex CLI is newer than the bindings.
    Unknown {
        method: String,
        params: Option<Value>,
    },
}

impl Notification {
    /// Return the wire `method` string for this notification.
    pub fn method(&self) -> &str {
        match self {
            Self::ThreadStarted(_) => methods::THREAD_STARTED,
            Self::ThreadStatusChanged(_) => methods::THREAD_STATUS_CHANGED,
            Self::ThreadTokenUsageUpdated(_) => methods::THREAD_TOKEN_USAGE_UPDATED,
            Self::TurnStarted(_) => methods::TURN_STARTED,
            Self::TurnCompleted(_) => methods::TURN_COMPLETED,
            Self::ItemStarted(_) => methods::ITEM_STARTED,
            Self::ItemCompleted(_) => methods::ITEM_COMPLETED,
            Self::AgentMessageDelta(_) => methods::AGENT_MESSAGE_DELTA,
            Self::CmdOutputDelta(_) => methods::CMD_OUTPUT_DELTA,
            Self::FileChangeOutputDelta(_) => methods::FILE_CHANGE_OUTPUT_DELTA,
            Self::ReasoningDelta(_) => methods::REASONING_DELTA,
            Self::Error(_) => methods::ERROR,
            Self::AccountRateLimitsUpdated(_) => methods::ACCOUNT_RATE_LIMITS_UPDATED,
            Self::McpServerStartupStatusUpdated(_) => methods::MCP_SERVER_STARTUP_STATUS_UPDATED,
            Self::RemoteControlStatusChanged(_) => methods::REMOTE_CONTROL_STATUS_CHANGED,
            Self::McpServerOauthLoginCompleted(_) => methods::MCP_SERVER_OAUTH_LOGIN_COMPLETED,
            Self::FileChangePatchUpdated(_) => methods::FILE_CHANGE_PATCH_UPDATED,
            Self::PlanDelta(_) => methods::PLAN_DELTA,
            Self::TurnPlanUpdated(_) => methods::TURN_PLAN_UPDATED,
            Self::TurnDiffUpdated(_) => methods::TURN_DIFF_UPDATED,
            Self::ReasoningSummaryPartAdded(_) => methods::REASONING_SUMMARY_PART_ADDED,
            Self::ReasoningTextDelta(_) => methods::REASONING_TEXT_DELTA,
            Self::AccountLoginCompleted(_) => methods::ACCOUNT_LOGIN_COMPLETED,
            Self::DeprecationNotice(_) => methods::DEPRECATION_NOTICE,
            Self::GuardianWarning(_) => methods::GUARDIAN_WARNING,
            Self::Warning(_) => methods::WARNING,
            Self::ThreadArchived(_) => methods::THREAD_ARCHIVED,
            Self::ThreadClosed(_) => methods::THREAD_CLOSED,
            Self::ThreadDeleted(_) => methods::THREAD_DELETED,
            Self::ThreadUnarchived(_) => methods::THREAD_UNARCHIVED,
            Self::ThreadGoalCleared(_) => methods::THREAD_GOAL_CLEARED,
            Self::ThreadNameUpdated(_) => methods::THREAD_NAME_UPDATED,
            Self::SkillsChanged(_) => methods::SKILLS_CHANGED,
            Self::FsChanged(_) => methods::FS_CHANGED,
            Self::ConfigWarning(_) => methods::CONFIG_WARNING,
            Self::AccountUpdated(_) => methods::ACCOUNT_UPDATED,
            Self::AppListUpdated(_) => methods::APP_LIST_UPDATED,
            Self::CommandExecOutputDelta(_) => methods::COMMAND_EXEC_OUTPUT_DELTA,
            Self::ExternalAgentConfigImportCompleted(_) => {
                methods::EXTERNAL_AGENT_CONFIG_IMPORT_COMPLETED
            }
            Self::FuzzyFileSearchSessionCompleted(_) => {
                methods::FUZZY_FILE_SEARCH_SESSION_COMPLETED
            }
            Self::FuzzyFileSearchSessionUpdated(_) => methods::FUZZY_FILE_SEARCH_SESSION_UPDATED,
            Self::HookCompleted(_) => methods::HOOK_COMPLETED,
            Self::HookStarted(_) => methods::HOOK_STARTED,
            Self::ItemGuardianApprovalReviewCompleted(_) => {
                methods::ITEM_AUTO_APPROVAL_REVIEW_COMPLETED
            }
            Self::ItemGuardianApprovalReviewStarted(_) => {
                methods::ITEM_AUTO_APPROVAL_REVIEW_STARTED
            }
            Self::TerminalInteraction(_) => methods::ITEM_COMMAND_EXEC_TERMINAL_INTERACTION,
            Self::McpToolCallProgress(_) => methods::ITEM_MCP_TOOL_CALL_PROGRESS,
            Self::ModelRerouted(_) => methods::MODEL_REROUTED,
            Self::ModelVerification(_) => methods::MODEL_VERIFICATION,
            Self::ProcessExited(_) => methods::PROCESS_EXITED,
            Self::ProcessOutputDelta(_) => methods::PROCESS_OUTPUT_DELTA,
            Self::ServerRequestResolved(_) => methods::SERVER_REQUEST_RESOLVED,
            Self::ContextCompacted(_) => methods::THREAD_COMPACTED,
            Self::ThreadGoalUpdated(_) => methods::THREAD_GOAL_UPDATED,
            Self::ThreadRealtimeClosed(_) => methods::THREAD_REALTIME_CLOSED,
            Self::ThreadRealtimeError(_) => methods::THREAD_REALTIME_ERROR,
            Self::ThreadRealtimeItemAdded(_) => methods::THREAD_REALTIME_ITEM_ADDED,
            Self::ThreadRealtimeOutputAudioDelta(_) => methods::THREAD_REALTIME_OUTPUT_AUDIO_DELTA,
            Self::ThreadRealtimeSdp(_) => methods::THREAD_REALTIME_SDP,
            Self::ThreadRealtimeStarted(_) => methods::THREAD_REALTIME_STARTED,
            Self::ThreadRealtimeTranscriptDelta(_) => methods::THREAD_REALTIME_TRANSCRIPT_DELTA,
            Self::ThreadRealtimeTranscriptDone(_) => methods::THREAD_REALTIME_TRANSCRIPT_DONE,
            Self::WindowsWorldWritableWarning(_) => methods::WINDOWS_WORLD_WRITABLE_WARNING,
            Self::WindowsSandboxSetupCompleted(_) => methods::WINDOWS_SANDBOX_SETUP_COMPLETED,
            Self::ThreadSettingsUpdated(_) => methods::THREAD_SETTINGS_UPDATED,
            Self::TurnModerationMetadata(_) => methods::TURN_MODERATION_METADATA,
            Self::ExternalAgentConfigImportProgress(_) => {
                methods::EXTERNAL_AGENT_CONFIG_IMPORT_PROGRESS
            }
            Self::ModelSafetyBufferingUpdated(_) => methods::MODEL_SAFETY_BUFFERING_UPDATED,
            Self::ThreadEnvironmentConnected(_) => methods::THREAD_ENVIRONMENT_CONNECTED,
            Self::ThreadEnvironmentDisconnected(_) => methods::THREAD_ENVIRONMENT_DISCONNECTED,
            Self::Unknown { method, .. } => method,
        }
    }

    /// `true` if this notification's method isn't modeled by the crate.
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown { .. })
    }

    /// Return the turn id this notification is scoped to, if it carries one.
    ///
    /// Reads the typed `turnId` field (or `turn.id` for the turn-lifecycle
    /// notifications) directly, so callers don't have to round-trip through
    /// [`into_envelope`](Self::into_envelope) and poke `serde_json::Value`
    /// fields. Returns `None` for notifications that aren't turn-scoped, and
    /// treats an empty id the server omitted as absent.
    ///
    /// The turn id from `turn/started` is what
    /// [`turn/interrupt`](crate::AsyncClient::turn_interrupt) needs to cancel
    /// the active turn.
    pub fn turn_id(&self) -> Option<&str> {
        let id = match self {
            Self::TurnStarted(n) => n.turn.id.as_str(),
            Self::TurnCompleted(n) => n.turn.id.as_str(),
            Self::AgentMessageDelta(n) => n.turn_id.as_str(),
            Self::CmdOutputDelta(n) => n.turn_id.as_str(),
            Self::FileChangeOutputDelta(n) => n.turn_id.as_str(),
            Self::ReasoningDelta(n) => n.turn_id.as_str(),
            Self::Error(n) => n.turn_id.as_str(),
            Self::FileChangePatchUpdated(n) => n.turn_id.as_str(),
            Self::PlanDelta(n) => n.turn_id.as_str(),
            Self::TurnPlanUpdated(n) => n.turn_id.as_str(),
            Self::TurnDiffUpdated(n) => n.turn_id.as_str(),
            Self::TurnModerationMetadata(n) => n.turn_id.as_str(),
            Self::ReasoningSummaryPartAdded(n) => n.turn_id.as_str(),
            Self::ReasoningTextDelta(n) => n.turn_id.as_str(),
            Self::ItemStarted(n) => n.turn_id.as_str(),
            Self::ItemCompleted(n) => n.turn_id.as_str(),
            Self::ContextCompacted(n) => n.turn_id.as_str(),
            Self::McpToolCallProgress(n) => n.turn_id.as_str(),
            Self::ModelRerouted(n) => n.turn_id.as_str(),
            Self::ModelVerification(n) => n.turn_id.as_str(),
            Self::ModelSafetyBufferingUpdated(n) => n.turn_id.as_str(),
            Self::TerminalInteraction(n) => n.turn_id.as_str(),
            Self::ThreadTokenUsageUpdated(n) => n.turn_id.as_str(),
            Self::ItemGuardianApprovalReviewStarted(n) => n.turn_id.as_str(),
            Self::ItemGuardianApprovalReviewCompleted(n) => n.turn_id.as_str(),
            _ => return None,
        };
        (!id.is_empty()).then_some(id)
    }

    /// Return the typed [`ThreadItem`](crate::protocol::ThreadItem) this
    /// notification carries, for `item/started` and `item/completed`.
    ///
    /// Lets downstream event adapters read the item's typed fields instead of
    /// reserializing it back into `serde_json::Value`.
    pub fn thread_item(&self) -> Option<&crate::protocol::ThreadItem> {
        match self {
            Self::ItemStarted(n) => Some(&n.item),
            Self::ItemCompleted(n) => Some(&n.item),
            _ => None,
        }
    }

    /// Construct a [`Notification`] from a `method` + `params` envelope.
    ///
    /// Returns an error if `method` is recognized but `params` doesn't
    /// deserialize into the typed struct. Unknown methods route to
    /// [`Notification::Unknown`] without error.
    pub fn from_envelope(method: &str, params: Option<Value>) -> Result<Self, serde_json::Error> {
        let params_value = params.clone().unwrap_or(Value::Null);
        match method {
            methods::THREAD_STARTED => {
                serde_json::from_value(params_value).map(Self::ThreadStarted)
            }
            methods::THREAD_STATUS_CHANGED => {
                serde_json::from_value(params_value).map(Self::ThreadStatusChanged)
            }
            methods::THREAD_TOKEN_USAGE_UPDATED => {
                serde_json::from_value(params_value).map(Self::ThreadTokenUsageUpdated)
            }
            methods::TURN_STARTED => serde_json::from_value(params_value).map(Self::TurnStarted),
            methods::TURN_COMPLETED => {
                serde_json::from_value(params_value).map(Self::TurnCompleted)
            }
            methods::ITEM_STARTED => serde_json::from_value(params_value).map(Self::ItemStarted),
            methods::ITEM_COMPLETED => {
                serde_json::from_value(params_value).map(Self::ItemCompleted)
            }
            methods::AGENT_MESSAGE_DELTA => {
                serde_json::from_value(params_value).map(Self::AgentMessageDelta)
            }
            methods::CMD_OUTPUT_DELTA => {
                serde_json::from_value(params_value).map(Self::CmdOutputDelta)
            }
            methods::FILE_CHANGE_OUTPUT_DELTA => {
                serde_json::from_value(params_value).map(Self::FileChangeOutputDelta)
            }
            methods::REASONING_DELTA => {
                serde_json::from_value(params_value).map(Self::ReasoningDelta)
            }
            methods::ERROR => serde_json::from_value(params_value).map(Self::Error),
            methods::ACCOUNT_RATE_LIMITS_UPDATED => {
                serde_json::from_value(params_value).map(Self::AccountRateLimitsUpdated)
            }
            methods::MCP_SERVER_STARTUP_STATUS_UPDATED => {
                serde_json::from_value(params_value).map(Self::McpServerStartupStatusUpdated)
            }
            methods::REMOTE_CONTROL_STATUS_CHANGED => {
                serde_json::from_value(params_value).map(Self::RemoteControlStatusChanged)
            }
            methods::MCP_SERVER_OAUTH_LOGIN_COMPLETED => {
                serde_json::from_value(params_value).map(Self::McpServerOauthLoginCompleted)
            }
            methods::FILE_CHANGE_PATCH_UPDATED => {
                serde_json::from_value(params_value).map(Self::FileChangePatchUpdated)
            }
            methods::PLAN_DELTA => serde_json::from_value(params_value).map(Self::PlanDelta),
            methods::TURN_PLAN_UPDATED => {
                serde_json::from_value(params_value).map(Self::TurnPlanUpdated)
            }
            methods::TURN_DIFF_UPDATED => {
                serde_json::from_value(params_value).map(Self::TurnDiffUpdated)
            }
            methods::REASONING_SUMMARY_PART_ADDED => {
                serde_json::from_value(params_value).map(Self::ReasoningSummaryPartAdded)
            }
            methods::REASONING_TEXT_DELTA => {
                serde_json::from_value(params_value).map(Self::ReasoningTextDelta)
            }
            methods::ACCOUNT_LOGIN_COMPLETED => {
                serde_json::from_value(params_value).map(Self::AccountLoginCompleted)
            }
            methods::DEPRECATION_NOTICE => {
                serde_json::from_value(params_value).map(Self::DeprecationNotice)
            }
            methods::GUARDIAN_WARNING => {
                serde_json::from_value(params_value).map(Self::GuardianWarning)
            }
            methods::WARNING => serde_json::from_value(params_value).map(Self::Warning),
            methods::THREAD_ARCHIVED => {
                serde_json::from_value(params_value).map(Self::ThreadArchived)
            }
            methods::THREAD_CLOSED => serde_json::from_value(params_value).map(Self::ThreadClosed),
            methods::THREAD_DELETED => {
                serde_json::from_value(params_value).map(Self::ThreadDeleted)
            }
            methods::THREAD_UNARCHIVED => {
                serde_json::from_value(params_value).map(Self::ThreadUnarchived)
            }
            methods::THREAD_GOAL_CLEARED => {
                serde_json::from_value(params_value).map(Self::ThreadGoalCleared)
            }
            methods::THREAD_NAME_UPDATED => {
                serde_json::from_value(params_value).map(Self::ThreadNameUpdated)
            }
            methods::SKILLS_CHANGED => {
                serde_json::from_value(params_value).map(Self::SkillsChanged)
            }
            methods::FS_CHANGED => serde_json::from_value(params_value).map(Self::FsChanged),
            methods::CONFIG_WARNING => {
                serde_json::from_value(params_value).map(Self::ConfigWarning)
            }
            methods::ACCOUNT_UPDATED => {
                serde_json::from_value(params_value).map(Self::AccountUpdated)
            }
            methods::APP_LIST_UPDATED => {
                serde_json::from_value(params_value).map(Self::AppListUpdated)
            }
            methods::COMMAND_EXEC_OUTPUT_DELTA => {
                serde_json::from_value(params_value).map(Self::CommandExecOutputDelta)
            }
            methods::EXTERNAL_AGENT_CONFIG_IMPORT_COMPLETED => {
                serde_json::from_value(params_value).map(Self::ExternalAgentConfigImportCompleted)
            }
            methods::FUZZY_FILE_SEARCH_SESSION_COMPLETED => {
                serde_json::from_value(params_value).map(Self::FuzzyFileSearchSessionCompleted)
            }
            methods::FUZZY_FILE_SEARCH_SESSION_UPDATED => {
                serde_json::from_value(params_value).map(Self::FuzzyFileSearchSessionUpdated)
            }
            methods::HOOK_COMPLETED => {
                serde_json::from_value(params_value).map(Self::HookCompleted)
            }
            methods::HOOK_STARTED => serde_json::from_value(params_value).map(Self::HookStarted),
            methods::ITEM_AUTO_APPROVAL_REVIEW_COMPLETED => {
                serde_json::from_value(params_value).map(Self::ItemGuardianApprovalReviewCompleted)
            }
            methods::ITEM_AUTO_APPROVAL_REVIEW_STARTED => {
                serde_json::from_value(params_value).map(Self::ItemGuardianApprovalReviewStarted)
            }
            methods::ITEM_COMMAND_EXEC_TERMINAL_INTERACTION => {
                serde_json::from_value(params_value).map(Self::TerminalInteraction)
            }
            methods::ITEM_MCP_TOOL_CALL_PROGRESS => {
                serde_json::from_value(params_value).map(Self::McpToolCallProgress)
            }
            methods::MODEL_REROUTED => {
                serde_json::from_value(params_value).map(Self::ModelRerouted)
            }
            methods::MODEL_VERIFICATION => {
                serde_json::from_value(params_value).map(Self::ModelVerification)
            }
            methods::PROCESS_EXITED => {
                serde_json::from_value(params_value).map(Self::ProcessExited)
            }
            methods::PROCESS_OUTPUT_DELTA => {
                serde_json::from_value(params_value).map(Self::ProcessOutputDelta)
            }
            methods::SERVER_REQUEST_RESOLVED => {
                serde_json::from_value(params_value).map(Self::ServerRequestResolved)
            }
            methods::THREAD_COMPACTED => {
                serde_json::from_value(params_value).map(Self::ContextCompacted)
            }
            methods::THREAD_GOAL_UPDATED => {
                serde_json::from_value(params_value).map(Self::ThreadGoalUpdated)
            }
            methods::THREAD_REALTIME_CLOSED => {
                serde_json::from_value(params_value).map(Self::ThreadRealtimeClosed)
            }
            methods::THREAD_REALTIME_ERROR => {
                serde_json::from_value(params_value).map(Self::ThreadRealtimeError)
            }
            methods::THREAD_REALTIME_ITEM_ADDED => {
                serde_json::from_value(params_value).map(Self::ThreadRealtimeItemAdded)
            }
            methods::THREAD_REALTIME_OUTPUT_AUDIO_DELTA => {
                serde_json::from_value(params_value).map(Self::ThreadRealtimeOutputAudioDelta)
            }
            methods::THREAD_REALTIME_SDP => {
                serde_json::from_value(params_value).map(Self::ThreadRealtimeSdp)
            }
            methods::THREAD_REALTIME_STARTED => {
                serde_json::from_value(params_value).map(Self::ThreadRealtimeStarted)
            }
            methods::THREAD_REALTIME_TRANSCRIPT_DELTA => {
                serde_json::from_value(params_value).map(Self::ThreadRealtimeTranscriptDelta)
            }
            methods::THREAD_REALTIME_TRANSCRIPT_DONE => {
                serde_json::from_value(params_value).map(Self::ThreadRealtimeTranscriptDone)
            }
            methods::WINDOWS_WORLD_WRITABLE_WARNING => {
                serde_json::from_value(params_value).map(Self::WindowsWorldWritableWarning)
            }
            methods::WINDOWS_SANDBOX_SETUP_COMPLETED => {
                serde_json::from_value(params_value).map(Self::WindowsSandboxSetupCompleted)
            }
            methods::THREAD_SETTINGS_UPDATED => {
                serde_json::from_value(params_value).map(Self::ThreadSettingsUpdated)
            }
            methods::TURN_MODERATION_METADATA => {
                serde_json::from_value(params_value).map(Self::TurnModerationMetadata)
            }
            methods::EXTERNAL_AGENT_CONFIG_IMPORT_PROGRESS => {
                serde_json::from_value(params_value).map(Self::ExternalAgentConfigImportProgress)
            }
            methods::MODEL_SAFETY_BUFFERING_UPDATED => {
                serde_json::from_value(params_value).map(Self::ModelSafetyBufferingUpdated)
            }
            methods::THREAD_ENVIRONMENT_CONNECTED => {
                serde_json::from_value(params_value).map(Self::ThreadEnvironmentConnected)
            }
            methods::THREAD_ENVIRONMENT_DISCONNECTED => {
                serde_json::from_value(params_value).map(Self::ThreadEnvironmentDisconnected)
            }
            _ => Ok(Self::Unknown {
                method: method.to_string(),
                params,
            }),
        }
    }

    /// Decompose this notification back into a `(method, params)` pair.
    pub fn into_envelope(self) -> Result<(String, Option<Value>), serde_json::Error> {
        fn pack<T: Serialize>(
            method: &str,
            v: &T,
        ) -> Result<(String, Option<Value>), serde_json::Error> {
            Ok((method.to_string(), Some(serde_json::to_value(v)?)))
        }
        match &self {
            Self::ThreadStarted(v) => pack(methods::THREAD_STARTED, v),
            Self::ThreadStatusChanged(v) => pack(methods::THREAD_STATUS_CHANGED, v),
            Self::ThreadTokenUsageUpdated(v) => pack(methods::THREAD_TOKEN_USAGE_UPDATED, v),
            Self::TurnStarted(v) => pack(methods::TURN_STARTED, v),
            Self::TurnCompleted(v) => pack(methods::TURN_COMPLETED, v),
            Self::ItemStarted(v) => pack(methods::ITEM_STARTED, v),
            Self::ItemCompleted(v) => pack(methods::ITEM_COMPLETED, v),
            Self::AgentMessageDelta(v) => pack(methods::AGENT_MESSAGE_DELTA, v),
            Self::CmdOutputDelta(v) => pack(methods::CMD_OUTPUT_DELTA, v),
            Self::FileChangeOutputDelta(v) => pack(methods::FILE_CHANGE_OUTPUT_DELTA, v),
            Self::ReasoningDelta(v) => pack(methods::REASONING_DELTA, v),
            Self::Error(v) => pack(methods::ERROR, v),
            Self::AccountRateLimitsUpdated(v) => pack(methods::ACCOUNT_RATE_LIMITS_UPDATED, v),
            Self::McpServerStartupStatusUpdated(v) => {
                pack(methods::MCP_SERVER_STARTUP_STATUS_UPDATED, v)
            }
            Self::RemoteControlStatusChanged(v) => pack(methods::REMOTE_CONTROL_STATUS_CHANGED, v),
            Self::McpServerOauthLoginCompleted(v) => {
                pack(methods::MCP_SERVER_OAUTH_LOGIN_COMPLETED, v)
            }
            Self::FileChangePatchUpdated(v) => pack(methods::FILE_CHANGE_PATCH_UPDATED, v),
            Self::PlanDelta(v) => pack(methods::PLAN_DELTA, v),
            Self::TurnPlanUpdated(v) => pack(methods::TURN_PLAN_UPDATED, v),
            Self::TurnDiffUpdated(v) => pack(methods::TURN_DIFF_UPDATED, v),
            Self::ReasoningSummaryPartAdded(v) => pack(methods::REASONING_SUMMARY_PART_ADDED, v),
            Self::ReasoningTextDelta(v) => pack(methods::REASONING_TEXT_DELTA, v),
            Self::AccountLoginCompleted(v) => pack(methods::ACCOUNT_LOGIN_COMPLETED, v),
            Self::DeprecationNotice(v) => pack(methods::DEPRECATION_NOTICE, v),
            Self::GuardianWarning(v) => pack(methods::GUARDIAN_WARNING, v),
            Self::Warning(v) => pack(methods::WARNING, v),
            Self::ThreadArchived(v) => pack(methods::THREAD_ARCHIVED, v),
            Self::ThreadClosed(v) => pack(methods::THREAD_CLOSED, v),
            Self::ThreadDeleted(v) => pack(methods::THREAD_DELETED, v),
            Self::ThreadUnarchived(v) => pack(methods::THREAD_UNARCHIVED, v),
            Self::ThreadGoalCleared(v) => pack(methods::THREAD_GOAL_CLEARED, v),
            Self::ThreadNameUpdated(v) => pack(methods::THREAD_NAME_UPDATED, v),
            Self::SkillsChanged(v) => pack(methods::SKILLS_CHANGED, v),
            Self::FsChanged(v) => pack(methods::FS_CHANGED, v),
            Self::ConfigWarning(v) => pack(methods::CONFIG_WARNING, v),
            Self::AccountUpdated(v) => pack(methods::ACCOUNT_UPDATED, v),
            Self::AppListUpdated(v) => pack(methods::APP_LIST_UPDATED, v),
            Self::CommandExecOutputDelta(v) => pack(methods::COMMAND_EXEC_OUTPUT_DELTA, v),
            Self::ExternalAgentConfigImportCompleted(v) => {
                pack(methods::EXTERNAL_AGENT_CONFIG_IMPORT_COMPLETED, v)
            }
            Self::FuzzyFileSearchSessionCompleted(v) => {
                pack(methods::FUZZY_FILE_SEARCH_SESSION_COMPLETED, v)
            }
            Self::FuzzyFileSearchSessionUpdated(v) => {
                pack(methods::FUZZY_FILE_SEARCH_SESSION_UPDATED, v)
            }
            Self::HookCompleted(v) => pack(methods::HOOK_COMPLETED, v),
            Self::HookStarted(v) => pack(methods::HOOK_STARTED, v),
            Self::ItemGuardianApprovalReviewCompleted(v) => {
                pack(methods::ITEM_AUTO_APPROVAL_REVIEW_COMPLETED, v)
            }
            Self::ItemGuardianApprovalReviewStarted(v) => {
                pack(methods::ITEM_AUTO_APPROVAL_REVIEW_STARTED, v)
            }
            Self::TerminalInteraction(v) => {
                pack(methods::ITEM_COMMAND_EXEC_TERMINAL_INTERACTION, v)
            }
            Self::McpToolCallProgress(v) => pack(methods::ITEM_MCP_TOOL_CALL_PROGRESS, v),
            Self::ModelRerouted(v) => pack(methods::MODEL_REROUTED, v),
            Self::ModelVerification(v) => pack(methods::MODEL_VERIFICATION, v),
            Self::ProcessExited(v) => pack(methods::PROCESS_EXITED, v),
            Self::ProcessOutputDelta(v) => pack(methods::PROCESS_OUTPUT_DELTA, v),
            Self::ServerRequestResolved(v) => pack(methods::SERVER_REQUEST_RESOLVED, v),
            Self::ContextCompacted(v) => pack(methods::THREAD_COMPACTED, v),
            Self::ThreadGoalUpdated(v) => pack(methods::THREAD_GOAL_UPDATED, v),
            Self::ThreadRealtimeClosed(v) => pack(methods::THREAD_REALTIME_CLOSED, v),
            Self::ThreadRealtimeError(v) => pack(methods::THREAD_REALTIME_ERROR, v),
            Self::ThreadRealtimeItemAdded(v) => pack(methods::THREAD_REALTIME_ITEM_ADDED, v),
            Self::ThreadRealtimeOutputAudioDelta(v) => {
                pack(methods::THREAD_REALTIME_OUTPUT_AUDIO_DELTA, v)
            }
            Self::ThreadRealtimeSdp(v) => pack(methods::THREAD_REALTIME_SDP, v),
            Self::ThreadRealtimeStarted(v) => pack(methods::THREAD_REALTIME_STARTED, v),
            Self::ThreadRealtimeTranscriptDelta(v) => {
                pack(methods::THREAD_REALTIME_TRANSCRIPT_DELTA, v)
            }
            Self::ThreadRealtimeTranscriptDone(v) => {
                pack(methods::THREAD_REALTIME_TRANSCRIPT_DONE, v)
            }
            Self::WindowsWorldWritableWarning(v) => {
                pack(methods::WINDOWS_WORLD_WRITABLE_WARNING, v)
            }
            Self::WindowsSandboxSetupCompleted(v) => {
                pack(methods::WINDOWS_SANDBOX_SETUP_COMPLETED, v)
            }
            Self::ThreadSettingsUpdated(v) => pack(methods::THREAD_SETTINGS_UPDATED, v),
            Self::TurnModerationMetadata(v) => pack(methods::TURN_MODERATION_METADATA, v),
            Self::ExternalAgentConfigImportProgress(v) => {
                pack(methods::EXTERNAL_AGENT_CONFIG_IMPORT_PROGRESS, v)
            }
            Self::ModelSafetyBufferingUpdated(v) => {
                pack(methods::MODEL_SAFETY_BUFFERING_UPDATED, v)
            }
            Self::ThreadEnvironmentConnected(v) => pack(methods::THREAD_ENVIRONMENT_CONNECTED, v),
            Self::ThreadEnvironmentDisconnected(v) => {
                pack(methods::THREAD_ENVIRONMENT_DISCONNECTED, v)
            }
            Self::Unknown { method, params } => Ok((method.clone(), params.clone())),
        }
    }
}

impl Serialize for Notification {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (method, params) = self
            .clone()
            .into_envelope()
            .map_err(serde::ser::Error::custom)?;
        let mut env = serde_json::Map::new();
        env.insert("method".to_string(), Value::String(method));
        if let Some(p) = params {
            env.insert("params".to_string(), p);
        }
        Value::Object(env).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Notification {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        let method = value
            .get("method")
            .and_then(|v| v.as_str())
            .ok_or_else(|| serde::de::Error::missing_field("method"))?
            .to_string();
        let params = value.get("params").cloned();
        Self::from_envelope(&method, params).map_err(serde::de::Error::custom)
    }
}

/// A server-to-client request that requires a response (approval flow).
///
/// The wire envelope carries an `id` for response correlation; that `id` is
/// held alongside this enum in [`ServerMessage::Request`] rather than embedded
/// inside the variant, since responding doesn't depend on which approval-type
/// was requested.
#[derive(Debug, Clone)]
pub enum ServerRequest {
    /// `item/commandExecution/requestApproval`
    CmdExecApproval(CommandExecutionRequestApprovalParams),
    /// `item/fileChange/requestApproval`
    FileChangeApproval(FileChangeRequestApprovalParams),
    /// `item/tool/requestUserInput`
    ToolRequestUserInput(crate::protocol::ToolRequestUserInputParams),
    /// `mcpServer/elicitation/request`
    McpServerElicitationRequest(crate::protocol::McpServerElicitationRequestParams),
    /// `item/permissions/requestApproval`
    PermissionsRequestApproval(crate::protocol::PermissionsRequestApprovalParams),
    /// `item/tool/call`
    ItemToolCall(crate::protocol::DynamicToolCallParams),
    /// `account/chatgptAuthTokens/refresh`
    ChatgptAuthTokensRefresh(crate::protocol::ChatgptAuthTokensRefreshParams),
    /// `attestation/generate`
    AttestationGenerate(crate::protocol::AttestationGenerateParams),
    /// `applyPatchApproval`
    ApplyPatchApproval(crate::protocol::ApplyPatchApprovalParams),
    /// `execCommandApproval`
    ExecCommandApproval(crate::protocol::ExecCommandApprovalParams),
    /// A request method this crate version does not yet model.
    Unknown {
        method: String,
        params: Option<Value>,
    },
}

impl ServerRequest {
    /// Return the wire `method` string for this request.
    pub fn method(&self) -> &str {
        match self {
            Self::CmdExecApproval(_) => methods::CMD_EXEC_APPROVAL,
            Self::FileChangeApproval(_) => methods::FILE_CHANGE_APPROVAL,
            Self::ToolRequestUserInput(_) => methods::TOOL_REQUEST_USER_INPUT,
            Self::McpServerElicitationRequest(_) => methods::MCP_SERVER_ELICITATION_REQUEST,
            Self::PermissionsRequestApproval(_) => methods::PERMISSIONS_REQUEST_APPROVAL,
            Self::ItemToolCall(_) => methods::ITEM_TOOL_CALL,
            Self::ChatgptAuthTokensRefresh(_) => methods::CHATGPT_AUTH_TOKENS_REFRESH,
            Self::AttestationGenerate(_) => methods::ATTESTATION_GENERATE,
            Self::ApplyPatchApproval(_) => methods::APPLY_PATCH_APPROVAL,
            Self::ExecCommandApproval(_) => methods::EXEC_COMMAND_APPROVAL,
            Self::Unknown { method, .. } => method,
        }
    }

    /// `true` if this request's method isn't modeled by the crate.
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown { .. })
    }

    /// Construct a [`ServerRequest`] from a `method` + `params` envelope.
    pub fn from_envelope(method: &str, params: Option<Value>) -> Result<Self, serde_json::Error> {
        let params_value = params.clone().unwrap_or(Value::Null);
        match method {
            methods::CMD_EXEC_APPROVAL => {
                serde_json::from_value(params_value).map(Self::CmdExecApproval)
            }
            methods::FILE_CHANGE_APPROVAL => {
                serde_json::from_value(params_value).map(Self::FileChangeApproval)
            }
            methods::TOOL_REQUEST_USER_INPUT => {
                serde_json::from_value(params_value).map(Self::ToolRequestUserInput)
            }
            methods::MCP_SERVER_ELICITATION_REQUEST => {
                serde_json::from_value(params_value).map(Self::McpServerElicitationRequest)
            }
            methods::PERMISSIONS_REQUEST_APPROVAL => {
                serde_json::from_value(params_value).map(Self::PermissionsRequestApproval)
            }
            methods::ITEM_TOOL_CALL => serde_json::from_value(params_value).map(Self::ItemToolCall),
            methods::CHATGPT_AUTH_TOKENS_REFRESH => {
                serde_json::from_value(params_value).map(Self::ChatgptAuthTokensRefresh)
            }
            methods::ATTESTATION_GENERATE => {
                serde_json::from_value(params_value).map(Self::AttestationGenerate)
            }
            methods::APPLY_PATCH_APPROVAL => {
                serde_json::from_value(params_value).map(Self::ApplyPatchApproval)
            }
            methods::EXEC_COMMAND_APPROVAL => {
                serde_json::from_value(params_value).map(Self::ExecCommandApproval)
            }
            _ => Ok(Self::Unknown {
                method: method.to_string(),
                params,
            }),
        }
    }
}

/// A message coming from the app-server.
///
/// Replaces the previous loose `{ method, params }` shape with typed enums.
/// Match on the outer variant first to distinguish notifications (no response)
/// from requests (need [`crate::AsyncClient::respond`] /
/// [`crate::SyncClient::respond`]).
#[derive(Debug, Clone)]
pub enum ServerMessage {
    /// A notification — no response required.
    Notification(Notification),
    /// A request — call `respond(id, ...)` on the client with the matching id.
    Request {
        id: RequestId,
        request: ServerRequest,
    },
}

impl ServerMessage {
    /// `true` if this message is an unmodeled method (notification or request).
    pub fn is_unknown(&self) -> bool {
        match self {
            Self::Notification(n) => n.is_unknown(),
            Self::Request { request, .. } => request.is_unknown(),
        }
    }

    /// Parse a raw app-server frame (one JSON-RPC line) into a [`ServerMessage`].
    ///
    /// This is the same parse path the [`AsyncClient`](crate::AsyncClient) and
    /// [`SyncClient`](crate::SyncClient) run on incoming lines, exposed for
    /// replay, recovery, and test fixtures that need to decode a captured frame
    /// without a live client.
    ///
    /// A frame is a [`ServerMessage`] only if it is a server-initiated
    /// notification or request. A JSON-RPC *response* / *error* (a reply to a
    /// client request) is not a server message and returns
    /// [`Error::Protocol`](crate::Error::Protocol). Unknown methods decode to
    /// the `Unknown` variants rather than erroring; a modeled method whose
    /// `params` don't fit returns [`Error::Deserialization`](crate::Error::Deserialization)
    /// carrying the raw frame.
    pub fn from_json_str(s: &str) -> Result<Self, Error> {
        let msg: JsonRpcMessage = serde_json::from_str(s)
            .map_err(|e| Error::Deserialization(ParseError::from_line(s, e)))?;
        Self::from_jsonrpc(msg)
    }

    /// Parse a [`serde_json::Value`] frame into a [`ServerMessage`].
    ///
    /// See [`from_json_str`](Self::from_json_str) for the parsing contract.
    pub fn from_value(value: Value) -> Result<Self, Error> {
        let msg: JsonRpcMessage = serde_json::from_value(value.clone())
            .map_err(|e| Error::Deserialization(ParseError::from_line(value.to_string(), e)))?;
        Self::from_jsonrpc(msg)
    }

    fn from_jsonrpc(msg: JsonRpcMessage) -> Result<Self, Error> {
        match msg {
            JsonRpcMessage::Notification(JsonRpcNotification { method, params }) => {
                Notification::from_envelope(&method, params.clone())
                    .map(ServerMessage::Notification)
                    .map_err(|e| Error::Deserialization(ParseError::from_envelope(method, params, e)))
            }
            JsonRpcMessage::Request(JsonRpcRequest { id, method, params }) => {
                ServerRequest::from_envelope(&method, params.clone())
                    .map(|request| ServerMessage::Request { id, request })
                    .map_err(|e| Error::Deserialization(ParseError::from_envelope(method, params, e)))
            }
            JsonRpcMessage::Response(resp) => Err(Error::Protocol(format!(
                "frame is a JSON-RPC response (id={}), not a server-initiated message",
                resp.id
            ))),
            JsonRpcMessage::Error(err) => Err(Error::Protocol(format!(
                "frame is a JSON-RPC error response (id={}, code={}), not a server-initiated message",
                err.id, err.error.code
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_unknown_method_routes_to_unknown_variant() {
        let n = Notification::from_envelope("foo/bar", Some(serde_json::json!({"x": 1})))
            .expect("unknown methods do not error");
        match n {
            Notification::Unknown { method, params } => {
                assert_eq!(method, "foo/bar");
                assert_eq!(params, Some(serde_json::json!({"x": 1})));
            }
            other => panic!("expected Unknown, got {:?}", other),
        }
    }

    #[test]
    fn test_notification_known_method_with_bad_params_errors() {
        // thread/started expects a `thread` field — wrong shape should error.
        let err = Notification::from_envelope("thread/started", Some(serde_json::json!({})));
        assert!(err.is_err());
    }

    #[test]
    fn test_notification_round_trip_envelope() {
        let wire = serde_json::json!({
            "method": "item/agentMessage/delta",
            "params": {"threadId": "t1", "turnId": "u1", "itemId": "i1", "delta": "hi"},
        });
        let n: Notification = serde_json::from_value(wire.clone()).unwrap();
        assert!(matches!(n, Notification::AgentMessageDelta(_)));
        let back = serde_json::to_value(&n).unwrap();
        assert_eq!(back, wire);
    }

    #[test]
    fn test_turn_id_from_turn_started() {
        // turn/started carries the id under `turn.id`.
        let wire = serde_json::json!({
            "method": "turn/started",
            "params": {"threadId": "t1", "turn": {"id": "turn_42", "status": "inProgress"}},
        });
        let n: Notification = serde_json::from_value(wire).unwrap();
        assert!(matches!(n, Notification::TurnStarted(_)));
        assert_eq!(n.turn_id(), Some("turn_42"));
    }

    #[test]
    fn test_turn_id_from_delta_field() {
        // Streaming notifications carry a flat `turnId`.
        let wire = serde_json::json!({
            "method": "item/agentMessage/delta",
            "params": {"threadId": "t1", "turnId": "turn_7", "itemId": "i1", "delta": "hi"},
        });
        let n: Notification = serde_json::from_value(wire).unwrap();
        assert_eq!(n.turn_id(), Some("turn_7"));
    }

    #[test]
    fn test_turn_id_none_for_non_turn_notification() {
        let n = Notification::from_envelope(
            "thread/deleted",
            Some(serde_json::json!({"threadId": "t1"})),
        )
        .unwrap();
        assert_eq!(n.turn_id(), None);
    }

    #[test]
    fn test_thread_item_accessor() {
        let wire = serde_json::json!({
            "method": "item/started",
            "params": {
                "threadId": "t1",
                "turnId": "turn_1",
                "startedAtMs": 0,
                "item": {"type": "agentMessage", "id": "i1", "text": "hi"},
            },
        });
        let n: Notification = serde_json::from_value(wire).unwrap();
        assert!(n.thread_item().is_some());
        // A non-item notification has no thread item.
        let other = Notification::from_envelope(
            "thread/deleted",
            Some(serde_json::json!({"threadId": "t1"})),
        )
        .unwrap();
        assert!(other.thread_item().is_none());
    }

    #[test]
    fn test_server_message_from_json_str_notification() {
        let line = r#"{"method":"turn/started","params":{"threadId":"t1","turn":{"id":"turn_9","status":"inProgress"}}}"#;
        let msg = ServerMessage::from_json_str(line).expect("parses a notification frame");
        match msg {
            ServerMessage::Notification(n) => assert_eq!(n.turn_id(), Some("turn_9")),
            other => panic!("expected Notification, got {other:?}"),
        }
    }

    #[test]
    fn test_server_message_from_value_request() {
        let frame = serde_json::json!({
            "id": 7,
            "method": "execCommandApproval",
            "params": {
                "callId": "c1",
                "command": ["ls"],
                "conversationId": "th_1",
                "cwd": "/tmp",
                "parsedCmd": [],
            },
        });
        let msg = ServerMessage::from_value(frame).expect("parses a request frame");
        match msg {
            ServerMessage::Request { id, request } => {
                assert_eq!(id, RequestId::Integer(7));
                assert!(matches!(request, ServerRequest::ExecCommandApproval(_)));
            }
            other => panic!("expected Request, got {other:?}"),
        }
    }

    #[test]
    fn test_server_message_from_json_str_rejects_response() {
        // A response to a client request is not a server-initiated message.
        let line = r#"{"id":1,"result":{"threadId":"th_abc"}}"#;
        let err = ServerMessage::from_json_str(line).unwrap_err();
        assert!(matches!(err, Error::Protocol(_)), "got: {err:?}");
    }

    #[test]
    fn test_server_message_from_json_str_unknown_method_ok() {
        let line = r#"{"method":"some/future/notification","params":{"x":1}}"#;
        let msg = ServerMessage::from_json_str(line).unwrap();
        assert!(msg.is_unknown());
    }
}
