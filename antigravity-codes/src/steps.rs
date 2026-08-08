//! Turning a stream of [`StepUpdate`] frames into coherent steps.
//!
//! The harness reports a step many times as it runs: first as `STATE_ACTIVE`
//! with `text_delta`/`thinking_delta` fragments, then once more as
//! `STATE_DONE` carrying the whole `text`. A step is identified by
//! `(trajectory_id, step_index)` — *not* by index alone, because subagents run
//! on their own trajectories concurrently with the main one.
//!
//! [`StepAssembler`] keeps the running text for each of those keys and hands
//! back a [`Step`] snapshot per update, so a caller can render incrementally
//! and still see a complete step at the end.

use std::collections::HashMap;

use crate::protocol::{StepUpdate, StepUpdateSource, StepUpdateState, StepUpdateTarget};

/// What a step was doing.
///
/// Derived from whichever action member the harness populated. Steps that
/// carry only prose land on [`StepKind::Message`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StepKind {
    /// Prose for the user, or the echo of their own input.
    Message,
    /// A directory listing.
    ListDirectory,
    /// A filename search.
    FindFile,
    /// A content search across a directory.
    SearchDirectory,
    /// A file read.
    ViewFile,
    /// A file creation.
    CreateFile,
    /// A file edit, with the diff attached.
    EditFile,
    /// A shell command, with its exit code and combined output.
    RunCommand,
    /// The harness compacting its own context.
    Compaction,
    /// Delegation to a subagent, which runs on its own trajectory.
    InvokeSubagent,
    /// Image generation.
    GenerateImage,
    /// A web search.
    SearchWeb,
    /// A URL fetch.
    ReadUrlContent,
    /// A tool call routed to an MCP server.
    McpTool,
    /// A tool the *client* is expected to run.
    CustomTool,
    /// The agent's closing summary.
    Finish,
    /// A failure the harness is reporting in-band.
    Error,
    /// The harness is waiting for the user to confirm a tool.
    ToolConfirmationRequest,
    /// The harness is asking the user a question.
    QuestionsRequest,
}

/// A step, merged across every update the harness has sent for it.
#[derive(Debug, Clone)]
pub struct Step {
    /// The trajectory this step belongs to. The main conversation and each
    /// subagent get their own.
    pub trajectory_id: String,
    /// Position within the trajectory.
    pub step_index: u32,
    /// Lifecycle state as of this update.
    pub state: StepUpdateState,
    /// Who produced the step.
    pub source: StepUpdateSource,
    /// Who it is addressed to.
    pub target: StepUpdateTarget,
    /// What the step is doing.
    pub kind: StepKind,
    /// Text accumulated so far, deltas included.
    pub text: String,
    /// Reasoning accumulated so far, deltas included.
    pub thinking: String,
    /// Only the text that arrived in *this* update, for incremental rendering.
    pub text_delta: String,
    /// Failure detail, when [`Self::state`] is [`StepUpdateState::Error`].
    pub error_message: Option<String>,
    /// The frame this snapshot came from, for anything the summary omits.
    pub update: StepUpdate,
}

impl Step {
    /// A stable key for this step: `"{trajectory_id}:{step_index}"`.
    pub fn id(&self) -> String {
        format!("{}:{}", self.trajectory_id, self.step_index)
    }

    /// True once the harness will not update this step again.
    pub fn is_final(&self) -> bool {
        matches!(self.state, StepUpdateState::Done | StepUpdateState::Error)
    }

    /// The accumulated text, if there is any.
    pub fn text(&self) -> Option<&str> {
        Some(self.text.as_str()).filter(|t| !t.is_empty())
    }

    /// Text addressed to the user, as opposed to the model or the environment.
    ///
    /// This is the filter to use when rendering a conversation: it drops the
    /// echo of the user's own input and the agent's tool chatter.
    pub fn user_facing_text(&self) -> Option<&str> {
        match self.target {
            StepUpdateTarget::User => self.text(),
            _ => None,
        }
    }
}

/// Accumulates [`StepUpdate`] frames into [`Step`]s.
#[derive(Debug, Default)]
pub struct StepAssembler {
    buffers: HashMap<(String, u32), Buffer>,
    main_trajectory: Option<String>,
}

#[derive(Debug, Default)]
struct Buffer {
    text: String,
    thinking: String,
}

impl StepAssembler {
    /// A fresh assembler for a conversation.
    ///
    /// `main_trajectory` should be the conversation's cascade id when known;
    /// otherwise the first trajectory seen is adopted as the main one.
    pub fn new(main_trajectory: Option<String>) -> Self {
        Self {
            buffers: HashMap::new(),
            main_trajectory,
        }
    }

    /// The trajectory treated as the main conversation.
    pub fn main_trajectory(&self) -> Option<&str> {
        self.main_trajectory.as_deref()
    }

    /// True when `trajectory_id` is the main conversation rather than a
    /// subagent's.
    pub fn is_main(&self, trajectory_id: &str) -> bool {
        self.main_trajectory.as_deref() == Some(trajectory_id)
    }

    /// Folds one update into the running state and returns the merged step.
    pub fn ingest(&mut self, update: StepUpdate) -> Step {
        let trajectory_id = update.trajectory_id.clone().unwrap_or_default();
        let step_index = update.step_index.unwrap_or_default();
        if self.main_trajectory.is_none() && !trajectory_id.is_empty() {
            self.main_trajectory = Some(trajectory_id.clone());
        }

        let buffer = self
            .buffers
            .entry((trajectory_id.clone(), step_index))
            .or_default();

        let text_delta = update.text_delta.clone().unwrap_or_default();
        if !text_delta.is_empty() {
            buffer.text.push_str(&text_delta);
        }
        // The harness sends the whole text again when the step settles; that
        // copy wins, since it is what the model actually committed to.
        if let Some(text) = update.text.as_deref().filter(|t| !t.is_empty()) {
            buffer.text = text.to_string();
        }

        if let Some(delta) = update.thinking_delta.as_deref().filter(|t| !t.is_empty()) {
            buffer.thinking.push_str(delta);
        }
        if let Some(thinking) = update.thinking.as_deref().filter(|t| !t.is_empty()) {
            buffer.thinking = thinking.to_string();
        }

        let step = Step {
            trajectory_id,
            step_index,
            state: update.state.clone().unwrap_or_default(),
            source: update.source.clone().unwrap_or_default(),
            target: update.target.clone().unwrap_or_default(),
            kind: classify(&update),
            text: buffer.text.clone(),
            thinking: buffer.thinking.clone(),
            text_delta,
            error_message: update.error_message.clone().filter(|m| !m.is_empty()),
            update,
        };

        if step.is_final() {
            self.buffers
                .remove(&(step.trajectory_id.clone(), step.step_index));
        }
        step
    }
}

fn classify(update: &StepUpdate) -> StepKind {
    // Ordered so the more specific requests win over the action that carries
    // them; a step waiting on a confirmation also has its action populated.
    if update.tool_confirmation_request.is_some() {
        StepKind::ToolConfirmationRequest
    } else if update.questions_request.is_some() {
        StepKind::QuestionsRequest
    } else if update.error.is_some() {
        StepKind::Error
    } else if update.finish.is_some() {
        StepKind::Finish
    } else if update.list_directory.is_some() {
        StepKind::ListDirectory
    } else if update.find_file.is_some() {
        StepKind::FindFile
    } else if update.search_directory.is_some() {
        StepKind::SearchDirectory
    } else if update.view_file.is_some() {
        StepKind::ViewFile
    } else if update.create_file.is_some() {
        StepKind::CreateFile
    } else if update.edit_file.is_some() {
        StepKind::EditFile
    } else if update.run_command.is_some() {
        StepKind::RunCommand
    } else if update.compaction.is_some() {
        StepKind::Compaction
    } else if update.invoke_subagent.is_some() {
        StepKind::InvokeSubagent
    } else if update.generate_image.is_some() {
        StepKind::GenerateImage
    } else if update.search_web.is_some() {
        StepKind::SearchWeb
    } else if update.read_url_content.is_some() {
        StepKind::ReadUrlContent
    } else if update.mcp_tool.is_some() {
        StepKind::McpTool
    } else if update.custom_tool.is_some() {
        StepKind::CustomTool
    } else {
        StepKind::Message
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{ActionRunCommand, UserQuestionsRequest};

    fn delta(trajectory: &str, index: u32, text: &str, state: StepUpdateState) -> StepUpdate {
        StepUpdate {
            trajectory_id: Some(trajectory.into()),
            step_index: Some(index),
            state: Some(state),
            text_delta: Some(text.into()),
            ..Default::default()
        }
    }

    #[test]
    fn deltas_accumulate_within_a_step() {
        let mut a = StepAssembler::new(None);
        a.ingest(delta("t1", 0, "Hel", StepUpdateState::Active));
        let step = a.ingest(delta("t1", 0, "lo", StepUpdateState::Active));
        assert_eq!(step.text, "Hello");
        assert_eq!(step.text_delta, "lo");
        assert!(!step.is_final());
    }

    #[test]
    fn the_final_text_replaces_the_accumulated_deltas() {
        let mut a = StepAssembler::new(None);
        a.ingest(delta("t1", 0, "Hel", StepUpdateState::Active));
        let step = a.ingest(StepUpdate {
            trajectory_id: Some("t1".into()),
            step_index: Some(0),
            state: Some(StepUpdateState::Done),
            text: Some("Hello, world".into()),
            ..Default::default()
        });
        assert_eq!(step.text, "Hello, world");
        assert!(step.is_final());
    }

    #[test]
    fn concurrent_trajectories_do_not_bleed_into_each_other() {
        let mut a = StepAssembler::new(Some("main".into()));
        a.ingest(delta("main", 0, "main-", StepUpdateState::Active));
        a.ingest(delta("sub", 0, "sub-", StepUpdateState::Active));
        let main = a.ingest(delta("main", 0, "text", StepUpdateState::Active));
        let sub = a.ingest(delta("sub", 0, "text", StepUpdateState::Active));
        assert_eq!(main.text, "main-text");
        assert_eq!(sub.text, "sub-text");
        assert!(a.is_main("main"));
        assert!(!a.is_main("sub"));
    }

    #[test]
    fn the_first_trajectory_seen_becomes_the_main_one() {
        let mut a = StepAssembler::new(None);
        a.ingest(delta("first", 0, "x", StepUpdateState::Active));
        assert_eq!(a.main_trajectory(), Some("first"));
        a.ingest(delta("second", 0, "y", StepUpdateState::Active));
        assert_eq!(a.main_trajectory(), Some("first"));
    }

    #[test]
    fn a_settled_step_releases_its_buffer() {
        let mut a = StepAssembler::new(None);
        a.ingest(delta("t1", 0, "hi", StepUpdateState::Done));
        assert!(a.buffers.is_empty());
    }

    #[test]
    fn actions_classify_by_their_populated_member() {
        let update = StepUpdate {
            run_command: Some(ActionRunCommand {
                command_line: Some("ls".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(classify(&update), StepKind::RunCommand);
        assert_eq!(classify(&StepUpdate::default()), StepKind::Message);
    }

    #[test]
    fn a_pending_question_outranks_its_action() {
        let update = StepUpdate {
            run_command: Some(ActionRunCommand::default()),
            questions_request: Some(UserQuestionsRequest::default()),
            ..Default::default()
        };
        assert_eq!(classify(&update), StepKind::QuestionsRequest);
    }

    #[test]
    fn only_user_targeted_text_is_user_facing() {
        let mut a = StepAssembler::new(None);
        let to_model = a.ingest(StepUpdate {
            trajectory_id: Some("t".into()),
            target: Some(StepUpdateTarget::Model),
            text: Some("echo of the prompt".into()),
            ..Default::default()
        });
        assert_eq!(to_model.user_facing_text(), None);
        assert_eq!(to_model.text(), Some("echo of the prompt"));
    }
}
