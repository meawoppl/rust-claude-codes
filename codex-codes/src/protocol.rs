//! App-server protocol types for the Codex CLI.
//!
//! Every wire type is generated from the upstream JSON Schema bundle by
//! `scripts/codegen_protocol.py` and lives in [`crate::protocol_generated::types`].
//! This module re-exports them and adds the JSON-RPC method-name constants
//! the dispatch layer matches against.
//!
//! # Parsing notifications
//!
//! Prefer the typed dispatch in [`crate::messages`] over manual `method` checks:
//!
//! ```
//! use codex_codes::{Notification, ServerMessage};
//!
//! fn handle(msg: ServerMessage) {
//!     if let ServerMessage::Notification(Notification::TurnCompleted(c)) = msg {
//!         println!("Turn on thread {} completed", c.thread_id);
//!     }
//! }
//! ```

pub use crate::protocol_generated::types::*;

/// JSON-RPC method names used by the app-server protocol.
///
/// Use these constants when matching on [`crate::ServerMessage::Notification`] or
/// [`crate::ServerMessage::Request`] method fields to avoid typos.
pub mod methods {
    // Client → server requests
    pub const INITIALIZE: &str = "initialize";
    pub const INITIALIZED: &str = "initialized";
    pub const THREAD_START: &str = "thread/start";
    pub const THREAD_ARCHIVE: &str = "thread/archive";
    pub const THREAD_DELETE: &str = "thread/delete";
    pub const TURN_START: &str = "turn/start";
    pub const TURN_INTERRUPT: &str = "turn/interrupt";
    pub const TURN_STEER: &str = "turn/steer";
    pub const THREAD_RESUME: &str = "thread/resume";
    pub const THREAD_FORK: &str = "thread/fork";
    pub const THREAD_UNSUBSCRIBE: &str = "thread/unsubscribe";
    pub const THREAD_NAME_SET: &str = "thread/name/set";
    pub const THREAD_METADATA_UPDATE: &str = "thread/metadata/update";
    pub const THREAD_UNARCHIVE: &str = "thread/unarchive";
    pub const THREAD_COMPACT_START: &str = "thread/compact/start";
    pub const THREAD_SHELLCOMMAND: &str = "thread/shellCommand";
    pub const THREAD_APPROVEGUARDIANDENIEDACTION: &str = "thread/approveGuardianDeniedAction";
    pub const THREAD_ROLLBACK: &str = "thread/rollback";
    pub const THREAD_LIST: &str = "thread/list";
    pub const THREAD_LOADED_LIST: &str = "thread/loaded/list";
    pub const THREAD_READ: &str = "thread/read";
    pub const THREAD_INJECT_ITEMS: &str = "thread/inject_items";
    pub const SKILLS_LIST: &str = "skills/list";
    pub const HOOKS_LIST: &str = "hooks/list";
    pub const MARKETPLACE_ADD: &str = "marketplace/add";
    pub const MARKETPLACE_REMOVE: &str = "marketplace/remove";
    pub const MARKETPLACE_UPGRADE: &str = "marketplace/upgrade";
    pub const PLUGIN_LIST: &str = "plugin/list";
    pub const PLUGIN_READ: &str = "plugin/read";
    pub const PLUGIN_SKILL_READ: &str = "plugin/skill/read";
    pub const PLUGIN_SHARE_SAVE: &str = "plugin/share/save";
    pub const PLUGIN_SHARE_UPDATETARGETS: &str = "plugin/share/updateTargets";
    pub const PLUGIN_SHARE_LIST: &str = "plugin/share/list";
    pub const PLUGIN_SHARE_CHECKOUT: &str = "plugin/share/checkout";
    pub const PLUGIN_SHARE_DELETE: &str = "plugin/share/delete";
    pub const APP_LIST: &str = "app/list";
    pub const FS_READFILE: &str = "fs/readFile";
    pub const FS_WRITEFILE: &str = "fs/writeFile";
    pub const FS_CREATEDIRECTORY: &str = "fs/createDirectory";
    pub const FS_GETMETADATA: &str = "fs/getMetadata";
    pub const FS_READDIRECTORY: &str = "fs/readDirectory";
    pub const FS_REMOVE: &str = "fs/remove";
    pub const FS_COPY: &str = "fs/copy";
    pub const FS_WATCH: &str = "fs/watch";
    pub const FS_UNWATCH: &str = "fs/unwatch";
    pub const SKILLS_CONFIG_WRITE: &str = "skills/config/write";
    pub const PLUGIN_INSTALL: &str = "plugin/install";
    pub const PLUGIN_UNINSTALL: &str = "plugin/uninstall";
    pub const REVIEW_START: &str = "review/start";
    pub const MODEL_LIST: &str = "model/list";
    pub const MODELPROVIDER_CAPABILITIES_READ: &str = "modelProvider/capabilities/read";
    pub const EXPERIMENTALFEATURE_LIST: &str = "experimentalFeature/list";
    pub const EXPERIMENTALFEATURE_ENABLEMENT_SET: &str = "experimentalFeature/enablement/set";
    pub const MCPSERVER_OAUTH_LOGIN: &str = "mcpServer/oauth/login";
    pub const CONFIG_MCPSERVER_RELOAD: &str = "config/mcpServer/reload";
    pub const MCPSERVERSTATUS_LIST: &str = "mcpServerStatus/list";
    pub const MCPSERVER_RESOURCE_READ: &str = "mcpServer/resource/read";
    pub const MCPSERVER_TOOL_CALL: &str = "mcpServer/tool/call";
    pub const WINDOWSSANDBOX_SETUPSTART: &str = "windowsSandbox/setupStart";
    pub const WINDOWSSANDBOX_READINESS: &str = "windowsSandbox/readiness";
    pub const ACCOUNT_LOGIN_START: &str = "account/login/start";
    pub const ACCOUNT_LOGIN_CANCEL: &str = "account/login/cancel";
    pub const ACCOUNT_LOGOUT: &str = "account/logout";
    pub const ACCOUNT_RATELIMITS_READ: &str = "account/rateLimits/read";
    pub const ACCOUNT_SENDADDCREDITSNUDGEEMAIL: &str = "account/sendAddCreditsNudgeEmail";
    pub const FEEDBACK_UPLOAD: &str = "feedback/upload";
    pub const COMMAND_EXEC: &str = "command/exec";
    pub const COMMAND_EXEC_WRITE: &str = "command/exec/write";
    pub const COMMAND_EXEC_TERMINATE: &str = "command/exec/terminate";
    pub const COMMAND_EXEC_RESIZE: &str = "command/exec/resize";
    pub const CONFIG_READ: &str = "config/read";
    pub const EXTERNALAGENTCONFIG_DETECT: &str = "externalAgentConfig/detect";
    pub const EXTERNALAGENTCONFIG_IMPORT: &str = "externalAgentConfig/import";
    pub const CONFIG_VALUE_WRITE: &str = "config/value/write";
    pub const CONFIG_BATCHWRITE: &str = "config/batchWrite";
    pub const CONFIGREQUIREMENTS_READ: &str = "configRequirements/read";
    pub const ACCOUNT_READ: &str = "account/read";
    pub const FUZZYFILESEARCH: &str = "fuzzyFileSearch";
    pub const ACCOUNT_USAGE_READ: &str = "account/usage/read";
    pub const PERMISSION_PROFILE_LIST: &str = "permissionProfile/list";
    pub const PLUGIN_INSTALLED: &str = "plugin/installed";
    pub const SKILLS_EXTRA_ROOTS_SET: &str = "skills/extraRoots/set";
    pub const THREAD_GOAL_GET: &str = "thread/goal/get";
    pub const THREAD_GOAL_SET: &str = "thread/goal/set";
    pub const THREAD_GOAL_CLEAR: &str = "thread/goal/clear";
    pub const ACCOUNT_RATELIMITRESETCREDIT_CONSUME: &str = "account/rateLimitResetCredit/consume";
    pub const ACCOUNT_WORKSPACEMESSAGES_READ: &str = "account/workspaceMessages/read";
    pub const EXTERNALAGENTCONFIG_IMPORT_READHISTORIES: &str =
        "externalAgentConfig/import/readHistories";

    // Server → client notifications
    pub const THREAD_STARTED: &str = "thread/started";
    pub const THREAD_STATUS_CHANGED: &str = "thread/status/changed";
    pub const THREAD_TOKEN_USAGE_UPDATED: &str = "thread/tokenUsage/updated";
    pub const TURN_STARTED: &str = "turn/started";
    pub const TURN_COMPLETED: &str = "turn/completed";
    pub const ITEM_STARTED: &str = "item/started";
    pub const ITEM_COMPLETED: &str = "item/completed";
    pub const AGENT_MESSAGE_DELTA: &str = "item/agentMessage/delta";
    pub const CMD_OUTPUT_DELTA: &str = "item/commandExecution/outputDelta";
    pub const FILE_CHANGE_OUTPUT_DELTA: &str = "item/fileChange/outputDelta";
    pub const REASONING_DELTA: &str = "item/reasoning/summaryTextDelta";
    pub const ERROR: &str = "error";
    pub const ACCOUNT_RATE_LIMITS_UPDATED: &str = "account/rateLimits/updated";
    pub const MCP_SERVER_STARTUP_STATUS_UPDATED: &str = "mcpServer/startupStatus/updated";
    pub const MCP_SERVER_OAUTH_LOGIN_COMPLETED: &str = "mcpServer/oauthLogin/completed";
    pub const REMOTE_CONTROL_STATUS_CHANGED: &str = "remoteControl/status/changed";
    pub const FILE_CHANGE_PATCH_UPDATED: &str = "item/fileChange/patchUpdated";
    pub const PLAN_DELTA: &str = "item/plan/delta";
    pub const TURN_PLAN_UPDATED: &str = "turn/plan/updated";
    pub const TURN_DIFF_UPDATED: &str = "turn/diff/updated";
    pub const REASONING_SUMMARY_PART_ADDED: &str = "item/reasoning/summaryPartAdded";
    pub const REASONING_TEXT_DELTA: &str = "item/reasoning/textDelta";
    pub const ACCOUNT_LOGIN_COMPLETED: &str = "account/login/completed";
    pub const DEPRECATION_NOTICE: &str = "deprecationNotice";
    pub const GUARDIAN_WARNING: &str = "guardianWarning";
    pub const WARNING: &str = "warning";
    pub const THREAD_ARCHIVED: &str = "thread/archived";
    pub const THREAD_CLOSED: &str = "thread/closed";
    pub const THREAD_DELETED: &str = "thread/deleted";
    pub const THREAD_UNARCHIVED: &str = "thread/unarchived";
    pub const THREAD_GOAL_CLEARED: &str = "thread/goal/cleared";
    pub const THREAD_NAME_UPDATED: &str = "thread/name/updated";
    pub const SKILLS_CHANGED: &str = "skills/changed";
    pub const FS_CHANGED: &str = "fs/changed";
    pub const CONFIG_WARNING: &str = "configWarning";
    pub const ACCOUNT_UPDATED: &str = "account/updated";
    pub const APP_LIST_UPDATED: &str = "app/list/updated";
    pub const COMMAND_EXEC_OUTPUT_DELTA: &str = "command/exec/outputDelta";
    pub const EXTERNAL_AGENT_CONFIG_IMPORT_COMPLETED: &str = "externalAgentConfig/import/completed";
    pub const FUZZY_FILE_SEARCH_SESSION_COMPLETED: &str = "fuzzyFileSearch/sessionCompleted";
    pub const FUZZY_FILE_SEARCH_SESSION_UPDATED: &str = "fuzzyFileSearch/sessionUpdated";
    pub const HOOK_COMPLETED: &str = "hook/completed";
    pub const HOOK_STARTED: &str = "hook/started";
    pub const ITEM_AUTO_APPROVAL_REVIEW_COMPLETED: &str = "item/autoApprovalReview/completed";
    pub const ITEM_AUTO_APPROVAL_REVIEW_STARTED: &str = "item/autoApprovalReview/started";
    pub const ITEM_COMMAND_EXEC_TERMINAL_INTERACTION: &str =
        "item/commandExecution/terminalInteraction";
    pub const ITEM_MCP_TOOL_CALL_PROGRESS: &str = "item/mcpToolCall/progress";
    pub const MODEL_REROUTED: &str = "model/rerouted";
    pub const MODEL_VERIFICATION: &str = "model/verification";
    pub const PROCESS_EXITED: &str = "process/exited";
    pub const PROCESS_OUTPUT_DELTA: &str = "process/outputDelta";
    pub const SERVER_REQUEST_RESOLVED: &str = "serverRequest/resolved";
    pub const THREAD_COMPACTED: &str = "thread/compacted";
    pub const THREAD_GOAL_UPDATED: &str = "thread/goal/updated";
    pub const THREAD_REALTIME_CLOSED: &str = "thread/realtime/closed";
    pub const THREAD_REALTIME_ERROR: &str = "thread/realtime/error";
    pub const THREAD_REALTIME_ITEM_ADDED: &str = "thread/realtime/itemAdded";
    pub const THREAD_REALTIME_OUTPUT_AUDIO_DELTA: &str = "thread/realtime/outputAudio/delta";
    pub const THREAD_REALTIME_SDP: &str = "thread/realtime/sdp";
    pub const THREAD_REALTIME_STARTED: &str = "thread/realtime/started";
    pub const THREAD_REALTIME_TRANSCRIPT_DELTA: &str = "thread/realtime/transcript/delta";
    pub const THREAD_REALTIME_TRANSCRIPT_DONE: &str = "thread/realtime/transcript/done";
    pub const WINDOWS_WORLD_WRITABLE_WARNING: &str = "windows/worldWritableWarning";
    pub const WINDOWS_SANDBOX_SETUP_COMPLETED: &str = "windowsSandbox/setupCompleted";
    pub const THREAD_SETTINGS_UPDATED: &str = "thread/settings/updated";
    pub const TURN_MODERATION_METADATA: &str = "turn/moderationMetadata";
    pub const EXTERNAL_AGENT_CONFIG_IMPORT_PROGRESS: &str = "externalAgentConfig/import/progress";
    pub const MODEL_SAFETY_BUFFERING_UPDATED: &str = "model/safetyBuffering/updated";
    pub const THREAD_ENVIRONMENT_CONNECTED: &str = "thread/environment/connected";
    pub const THREAD_ENVIRONMENT_DISCONNECTED: &str = "thread/environment/disconnected";

    // Server → client requests (approval flow, v2 envelope)
    pub const CMD_EXEC_APPROVAL: &str = "item/commandExecution/requestApproval";
    pub const FILE_CHANGE_APPROVAL: &str = "item/fileChange/requestApproval";
    pub const TOOL_REQUEST_USER_INPUT: &str = "item/tool/requestUserInput";
    pub const MCP_SERVER_ELICITATION_REQUEST: &str = "mcpServer/elicitation/request";
    pub const PERMISSIONS_REQUEST_APPROVAL: &str = "item/permissions/requestApproval";
    pub const ITEM_TOOL_CALL: &str = "item/tool/call";
    pub const CHATGPT_AUTH_TOKENS_REFRESH: &str = "account/chatgptAuthTokens/refresh";
    pub const ATTESTATION_GENERATE: &str = "attestation/generate";
    pub const APPLY_PATCH_APPROVAL: &str = "applyPatchApproval";
    pub const EXEC_COMMAND_APPROVAL: &str = "execCommandApproval";
}

// ──────────────────────────────────────────────────────────────────────────
// Ergonomic constructors over the generated wire types.
//
// The types themselves are code-generated from the upstream schema; these
// hand-written impls live here so they survive regeneration. They cover two
// pain points for downstream consumers:
//
//   * `Default` for the all-optional client request params, so callers don't
//     have to spell out every `None` field or construct via `from_value`.
//   * `accept` / `decline` / `approved` / `denied` constructors for the
//     approval-response payloads, so callers don't hand-roll `serde_json`
//     objects and can't typo the wire `decision` string.
// ──────────────────────────────────────────────────────────────────────────

macro_rules! default_from_empty_object {
    ($($ty:ident),+ $(,)?) => {
        $(
            impl Default for $ty {
                fn default() -> Self {
                    // Every field is `#[serde(default)]`, so an empty object
                    // yields the fully-unset params. Deserializing (rather than
                    // listing fields) keeps this correct as the upstream schema
                    // adds optional fields, and mirrors the construction idiom
                    // shown in the crate docs.
                    serde_json::from_value(serde_json::Value::Object(Default::default()))
                        .expect(concat!(
                            stringify!($ty),
                            ": every field is serde-default, so `{}` must deserialize"
                        ))
                }
            }
        )+
    };
}

default_from_empty_object!(
    ThreadStartParams,
    TurnStartParams,
    ThreadResumeParams,
    ThreadForkParams,
);

impl FileChangeRequestApprovalResponse {
    /// Approve this file-change request (`{"decision":"accept"}`).
    pub fn accept() -> Self {
        Self {
            decision: FileChangeApprovalDecision::Accept,
        }
    }

    /// Approve this and future file changes for the session.
    pub fn accept_for_session() -> Self {
        Self {
            decision: FileChangeApprovalDecision::AcceptForSession,
        }
    }

    /// Decline this file-change request (`{"decision":"decline"}`).
    pub fn decline() -> Self {
        Self {
            decision: FileChangeApprovalDecision::Decline,
        }
    }

    /// Cancel the turn in response to this request.
    pub fn cancel() -> Self {
        Self {
            decision: FileChangeApprovalDecision::Cancel,
        }
    }
}

impl CommandExecutionRequestApprovalResponse {
    /// Approve this command execution (`{"decision":"accept"}`).
    pub fn accept() -> Self {
        Self {
            decision: CommandExecutionApprovalDecision::Accept,
        }
    }

    /// Approve this and future command executions for the session.
    pub fn accept_for_session() -> Self {
        Self {
            decision: CommandExecutionApprovalDecision::AcceptForSession,
        }
    }

    /// Decline this command execution (`{"decision":"decline"}`).
    pub fn decline() -> Self {
        Self {
            decision: CommandExecutionApprovalDecision::Decline,
        }
    }

    /// Cancel the turn in response to this request.
    pub fn cancel() -> Self {
        Self {
            decision: CommandExecutionApprovalDecision::Cancel,
        }
    }
}

impl ExecCommandApprovalResponse {
    /// Approve the exec command (`{"decision":"approved"}`).
    pub fn approved() -> Self {
        Self {
            decision: ReviewDecision::Approved,
        }
    }

    /// Approve the exec command for the rest of the session.
    pub fn approved_for_session() -> Self {
        Self {
            decision: ReviewDecision::ApprovedForSession,
        }
    }

    /// Deny the exec command (`{"decision":"denied"}`).
    pub fn denied() -> Self {
        Self {
            decision: ReviewDecision::Denied,
        }
    }

    /// Abort the turn in response to the request.
    pub fn abort() -> Self {
        Self {
            decision: ReviewDecision::Abort,
        }
    }
}

impl ApplyPatchApprovalResponse {
    /// Approve the patch (`{"decision":"approved"}`).
    pub fn approved() -> Self {
        Self {
            decision: ReviewDecision::Approved,
        }
    }

    /// Approve this and future patches for the session.
    pub fn approved_for_session() -> Self {
        Self {
            decision: ReviewDecision::ApprovedForSession,
        }
    }

    /// Deny the patch (`{"decision":"denied"}`).
    pub fn denied() -> Self {
        Self {
            decision: ReviewDecision::Denied,
        }
    }

    /// Abort the turn in response to the request.
    pub fn abort() -> Self {
        Self {
            decision: ReviewDecision::Abort,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_start_params_default_is_empty_object() {
        let p = ThreadStartParams::default();
        assert_eq!(serde_json::to_value(&p).unwrap(), serde_json::json!({}));
    }

    #[test]
    fn turn_start_params_default_round_trips() {
        let p = TurnStartParams::default();
        let v = serde_json::to_value(&p).unwrap();
        // thread_id + input are required-but-defaulted; everything else omits.
        assert_eq!(v["threadId"], "");
        assert_eq!(v["input"], serde_json::json!([]));
    }

    #[test]
    fn thread_resume_and_fork_params_default() {
        let _ = ThreadResumeParams::default();
        let _ = ThreadForkParams::default();
    }

    #[test]
    fn file_change_approval_response_wire_shape() {
        assert_eq!(
            serde_json::to_value(FileChangeRequestApprovalResponse::accept()).unwrap(),
            serde_json::json!({"decision": "accept"})
        );
        assert_eq!(
            serde_json::to_value(FileChangeRequestApprovalResponse::decline()).unwrap(),
            serde_json::json!({"decision": "decline"})
        );
    }

    #[test]
    fn command_execution_approval_response_wire_shape() {
        assert_eq!(
            serde_json::to_value(CommandExecutionRequestApprovalResponse::accept()).unwrap(),
            serde_json::json!({"decision": "accept"})
        );
        assert_eq!(
            serde_json::to_value(CommandExecutionRequestApprovalResponse::cancel()).unwrap(),
            serde_json::json!({"decision": "cancel"})
        );
    }

    #[test]
    fn exec_and_apply_patch_approval_response_wire_shape() {
        assert_eq!(
            serde_json::to_value(ExecCommandApprovalResponse::approved()).unwrap(),
            serde_json::json!({"decision": "approved"})
        );
        assert_eq!(
            serde_json::to_value(ApplyPatchApprovalResponse::denied()).unwrap(),
            serde_json::json!({"decision": "denied"})
        );
    }
}
