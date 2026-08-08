//! The turn-oriented client.

use std::collections::{HashMap, HashSet};

use crate::error::{Error, Result};
use crate::handlers::Handlers;
use crate::process::HarnessOptions;
use crate::protocol::{
    InitializeConversationResponse, InputEvent, OutputEventEvent, StepUpdate, ToolConfirmation,
    TrajectoryStateUpdate, TrajectoryStateUpdateState, UsageMetadata,
};
use crate::steps::{Step, StepAssembler};
use crate::RawClient;

/// A harness session that drives whole turns.
///
/// [`Client`] wraps [`RawClient`] with the three things every caller ends up
/// writing otherwise: [`Step`] assembly from delta frames, replies to the
/// harness's [tool, hook, policy, and question requests](crate::handlers), and
/// turn-completion detection from trajectory state.
///
/// ```no_run
/// use antigravity_codes::{Client, HarnessOptions, ModelBuilder};
///
/// # async fn run() -> antigravity_codes::Result<()> {
/// let mut client = Client::launch(
///     HarnessOptions::new()
///         .workspace("/tmp/project")
///         .model(ModelBuilder::gemini("gemini-flash-latest", std::env::var("GEMINI_API_KEY").unwrap())),
/// )
/// .await?;
///
/// let mut turn = client.send("What files are here?").await?;
/// while let Some(step) = turn.next_step().await? {
///     if let Some(text) = step.user_facing_text() {
///         println!("{text}");
///     }
/// }
///
/// client.shutdown().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct Client {
    raw: RawClient,
    handlers: Handlers,
    assembler: StepAssembler,
    usage: Option<UsageMetadata>,
    trajectory_usage: HashMap<String, UsageMetadata>,
}

impl Client {
    /// Launches a harness with no client-side handlers registered.
    pub async fn launch(options: HarnessOptions) -> Result<Self> {
        Self::launch_with(options, Handlers::new()).await
    }

    /// Launches a harness with handlers for its callbacks.
    pub async fn launch_with(options: HarnessOptions, handlers: Handlers) -> Result<Self> {
        let raw = RawClient::launch(options).await?;
        let mut assembler = StepAssembler::new(raw.cascade_id().map(str::to_string));

        let initialize = raw.initialize_response().clone();
        // A resumed session replays its transcript in the initialize reply.
        // Folding it through the assembler now means step indices already seen
        // do not look new when the conversation continues.
        for update in initialize.history.iter().cloned() {
            assembler.ingest(update);
        }

        Ok(Self {
            usage: initialize.cumulative_usage.clone(),
            trajectory_usage: initialize
                .trajectory_usage
                .iter()
                .filter_map(|e| Some((e.trajectory_id.clone()?, e.usage.clone()?)))
                .collect(),
            raw,
            handlers,
            assembler,
        })
    }

    /// The conversation id, which the harness calls a "cascade id".
    ///
    /// Pass it to [`HarnessOptions::cascade_id`] to resume this conversation in
    /// a later process.
    pub fn cascade_id(&self) -> Option<&str> {
        self.raw.cascade_id()
    }

    /// The initialize reply, including any replayed history.
    pub fn initialize_response(&self) -> &InitializeConversationResponse {
        self.raw.initialize_response()
    }

    /// Cumulative token usage for the conversation, as last reported.
    pub fn usage(&self) -> Option<&UsageMetadata> {
        self.usage.as_ref()
    }

    /// Per-trajectory token usage, which separates subagent spend from the
    /// main conversation's.
    pub fn trajectory_usage(&self) -> &HashMap<String, UsageMetadata> {
        &self.trajectory_usage
    }

    /// The underlying frame-level client, for anything this layer does not model.
    pub fn raw(&mut self) -> &mut RawClient {
        &mut self.raw
    }

    /// Sends a prompt and returns the turn it started.
    pub async fn send(&mut self, prompt: impl Into<String>) -> Result<Turn<'_>> {
        self.send_event(InputEvent::user(prompt)).await
    }

    /// Sends an arbitrary input frame and treats it as the start of a turn.
    ///
    /// Use this for multimodal input
    /// ([`complex_user_input`](crate::protocol::InputEvent::complex_user_input))
    /// or to fire an
    /// [`automated_trigger`](crate::protocol::InputEvent::automated_trigger).
    pub async fn send_event(&mut self, event: InputEvent) -> Result<Turn<'_>> {
        self.raw.send(&event).await?;
        Ok(Turn {
            client: self,
            finished: false,
            failure: None,
            answered: HashSet::new(),
        })
    }

    /// Asks the harness to abandon the turn in flight.
    ///
    /// The turn does not end here — the harness finishes what it was doing and
    /// reports `STATE_CANCELLED`, which the in-progress [`Turn`] observes.
    pub async fn cancel(&mut self) -> Result<()> {
        self.raw.send(&InputEvent::halt()).await
    }

    /// Ends the session cleanly and stops the process.
    pub async fn shutdown(self) -> Result<()> {
        self.raw.shutdown().await
    }

    fn record_usage(&mut self, update: crate::protocol::UsageUpdate) {
        if let Some(total) = update.total {
            self.usage = Some(total);
        }
        for entry in update.agents {
            if let (Some(id), Some(usage)) = (entry.trajectory_id, entry.usage) {
                self.trajectory_usage.insert(id, usage);
            }
        }
    }
}

/// One turn of the conversation: everything between a prompt and the agent
/// going idle again.
///
/// Steps stream out of [`Turn::next_step`]. While it is being polled, the turn
/// also answers whatever the harness asks of the client — so a tool handler
/// runs *inside* `next_step`, not on a background task. Tool calls are
/// therefore executed one at a time, in arrival order.
#[derive(Debug)]
pub struct Turn<'a> {
    client: &'a mut Client,
    finished: bool,
    failure: Option<String>,
    /// `(trajectory_id, step_index, kind)` triples already answered, so a step
    /// that the harness re-sends is not answered twice.
    answered: HashSet<(String, u32, &'static str)>,
}

impl Turn<'_> {
    /// The next step, or `None` once the agent has gone idle.
    ///
    /// If the turn failed, the error surfaces *after* the last step, so
    /// whatever the agent said before failing is still delivered.
    pub async fn next_step(&mut self) -> Result<Option<Step>> {
        loop {
            if self.finished {
                return match self.failure.take() {
                    Some(message) => Err(Error::Turn { message }),
                    None => Ok(None),
                };
            }

            let Some(event) = self.client.raw.next_event().await? else {
                self.finished = true;
                continue;
            };

            match event.into_event() {
                Some(OutputEventEvent::StepUpdate(update)) => {
                    self.answer_in_band_requests(&update).await?;
                    return Ok(Some(self.client.assembler.ingest(update)));
                }
                Some(OutputEventEvent::ToolCall(call)) => {
                    let response = self.client.handlers.call_tool(call).await;
                    self.client
                        .raw
                        .send(&InputEvent::tool_response(response))
                        .await?;
                }
                Some(OutputEventEvent::CallHookRequest(request)) => {
                    let response = self.client.handlers.call_hook(request).await;
                    self.client
                        .raw
                        .send(&InputEvent::hook_response(response))
                        .await?;
                }
                Some(OutputEventEvent::PolicyDecisionRequest(request)) => {
                    let response = self.client.handlers.call_policy(request).await;
                    self.client
                        .raw
                        .send(&InputEvent::policy_response(response))
                        .await?;
                }
                Some(OutputEventEvent::UsageUpdate(update)) => self.client.record_usage(update),
                Some(OutputEventEvent::TrajectoryStateUpdate(update)) => {
                    self.observe_trajectory(&update)
                }
                Some(OutputEventEvent::SessionEndResponse(_)) => self.finished = true,
                // The initialize reply was consumed at launch; anything else is
                // a frame from a harness newer than this crate, and ignoring it
                // is the documented contract.
                Some(OutputEventEvent::InitializeConversationResponse(_)) | None => {}
            }
        }
    }

    /// Drains the turn and returns everything the agent said to the user.
    pub async fn collect_text(&mut self) -> Result<String> {
        let mut out = String::new();
        let mut seen = HashSet::new();
        while let Some(step) = self.next_step().await? {
            if step.is_final() && seen.insert(step.id()) {
                if let Some(text) = step.user_facing_text() {
                    out.push_str(text);
                }
            }
        }
        Ok(out)
    }

    /// True once the agent has gone idle.
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// A step can carry a request that has to be answered before the harness
    /// will move on: a question for the user, or a tool awaiting confirmation.
    async fn answer_in_band_requests(&mut self, update: &StepUpdate) -> Result<()> {
        let trajectory_id = update.trajectory_id.clone().unwrap_or_default();
        let step_index = update.step_index.unwrap_or_default();

        if let Some(request) = update.questions_request.clone() {
            if self
                .answered
                .insert((trajectory_id.clone(), step_index, "questions"))
            {
                let mut response = self.client.handlers.call_questions(request).await;
                response.trajectory_id = Some(trajectory_id.clone());
                response.step_index = Some(step_index);
                self.client
                    .raw
                    .send(&InputEvent {
                        question_response: Some(response),
                        ..Default::default()
                    })
                    .await?;
            }
        }

        if update.tool_confirmation_request.is_some()
            && self
                .answered
                .insert((trajectory_id.clone(), step_index, "confirm"))
        {
            let accepted = self.client.handlers.call_confirm(update.clone()).await;
            self.client
                .raw
                .send(&InputEvent {
                    tool_confirmation: Some(ToolConfirmation {
                        trajectory_id: Some(trajectory_id),
                        step_index: Some(step_index),
                        accepted: Some(accepted),
                    }),
                    ..Default::default()
                })
                .await?;
        }

        Ok(())
    }

    /// Only the main trajectory ends the turn. Subagents idle and fail on their
    /// own schedule while the conversation carries on around them.
    fn observe_trajectory(&mut self, update: &TrajectoryStateUpdate) {
        let id = update.trajectory_id.as_deref().unwrap_or_default();
        let error = update.error.clone().filter(|e| !e.is_empty());

        if !self.client.assembler.is_main(id) {
            if let Some(error) = error {
                log::info!("subagent trajectory {id} failed: {error}");
            }
            return;
        }

        match update.state {
            Some(TrajectoryStateUpdateState::FullyIdle) => {
                self.finished = true;
                self.failure = error;
            }
            Some(TrajectoryStateUpdateState::Cancelled) => {
                self.finished = true;
                self.failure = Some(error.unwrap_or_else(|| "turn cancelled".into()));
            }
            _ => {}
        }
    }
}
