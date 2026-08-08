//! Answering the requests the harness sends *back* to the client.
//!
//! A turn does not just stream output. Depending on how the session was
//! configured, the harness will stop and wait for the client on four
//! occasions, and the turn stays blocked until each is answered:
//!
//! | Request | Raised when | Answered with |
//! |---|---|---|
//! | [`ToolCall`] | the model calls a tool declared via [`HarnessOptions::tool`](crate::HarnessOptions::tool) | [`ToolResponse`] |
//! | [`CallHookRequest`] | a lifecycle hook registered via [`HarnessOptions::hook`](crate::HarnessOptions::hook) fires | [`CallHookResponse`] |
//! | [`PolicyDecisionRequest`] | a dynamic [`PolicyRule`](crate::protocol::PolicyRule) needs adjudicating | [`PolicyDecisionResponse`] |
//! | [`UserQuestionsRequest`] | the agent asks the user something | [`UserQuestionsResponse`] |
//!
//! None of these arrive unless the corresponding feature was configured, so
//! the empty [`Handlers`] is a perfectly good default for a plain chat
//! session. When one *does* arrive with no handler registered, the defaults
//! below keep the turn moving rather than deadlocking it.

use std::collections::HashMap;

use futures_util::future::BoxFuture;

use crate::protocol::{
    CallHookRequest, CallHookResponse, EmptyResult, PolicyDecisionRequest, PolicyDecisionResponse,
    PolicyEvaluationOutcome, StepUpdate, ToolCall, ToolResponse, UserQuestionsRequest,
    UserQuestionsResponse,
};

type ToolFn = Box<dyn Fn(ToolCall) -> BoxFuture<'static, ToolResponse> + Send + Sync>;
type HookFn = Box<dyn Fn(CallHookRequest) -> BoxFuture<'static, CallHookResponse> + Send + Sync>;
type PolicyFn =
    Box<dyn Fn(PolicyDecisionRequest) -> BoxFuture<'static, PolicyDecisionResponse> + Send + Sync>;
type QuestionFn =
    Box<dyn Fn(UserQuestionsRequest) -> BoxFuture<'static, UserQuestionsResponse> + Send + Sync>;
type ConfirmFn = Box<dyn Fn(StepUpdate) -> BoxFuture<'static, bool> + Send + Sync>;

/// The callbacks a [`Client`](crate::Client) uses to answer the harness.
#[derive(Default)]
pub struct Handlers {
    tools: HashMap<String, ToolFn>,
    hook: Option<HookFn>,
    policy: Option<PolicyFn>,
    questions: Option<QuestionFn>,
    confirm: Option<ConfirmFn>,
}

impl std::fmt::Debug for Handlers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handlers")
            .field("tools", &self.tools.keys().collect::<Vec<_>>())
            .field("hook", &self.hook.is_some())
            .field("policy", &self.policy.is_some())
            .field("questions", &self.questions.is_some())
            .field("confirm", &self.confirm.is_some())
            .finish()
    }
}

impl Handlers {
    /// No handlers. Fine for a session with no client-side tools or hooks.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the implementation of a client-side tool.
    ///
    /// `name` must match the [`Tool`](crate::protocol::Tool) declared on
    /// [`HarnessOptions::tool`](crate::HarnessOptions::tool); the harness
    /// dispatches by name.
    ///
    /// ```
    /// use antigravity_codes::handlers::Handlers;
    /// use antigravity_codes::protocol::ToolResponse;
    ///
    /// let handlers = Handlers::new().tool("clock", |call| async move {
    ///     ToolResponse::ok(call.id.unwrap_or_default(), r#"{"now":"2026-08-08T00:00:00Z"}"#)
    /// });
    /// assert!(handlers.handles_tool("clock"));
    /// ```
    pub fn tool<F, Fut>(mut self, name: impl Into<String>, f: F) -> Self
    where
        F: Fn(ToolCall) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ToolResponse> + Send + 'static,
    {
        self.tools
            .insert(name.into(), Box::new(move |call| Box::pin(f(call))));
        self
    }

    /// Handles every lifecycle hook the session subscribed to.
    pub fn on_hook<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(CallHookRequest) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = CallHookResponse> + Send + 'static,
    {
        self.hook = Some(Box::new(move |request| Box::pin(f(request))));
        self
    }

    /// Adjudicates dynamic policy rules.
    pub fn on_policy<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(PolicyDecisionRequest) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = PolicyDecisionResponse> + Send + 'static,
    {
        self.policy = Some(Box::new(move |request| Box::pin(f(request))));
        self
    }

    /// Answers questions the agent puts to the user.
    pub fn on_questions<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(UserQuestionsRequest) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = UserQuestionsResponse> + Send + 'static,
    {
        self.questions = Some(Box::new(move |request| Box::pin(f(request))));
        self
    }

    /// Approves or refuses a tool the harness wants confirmed before running.
    ///
    /// The handler receives the whole [`StepUpdate`], because the action being
    /// confirmed — the command line, the file path, the diff — is on the step,
    /// not on the (empty) request itself.
    pub fn on_tool_confirmation<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(StepUpdate) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = bool> + Send + 'static,
    {
        self.confirm = Some(Box::new(move |step| Box::pin(f(step))));
        self
    }

    /// Whether a tool of this name has an implementation registered.
    pub fn handles_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    pub(crate) async fn call_tool(&self, call: ToolCall) -> ToolResponse {
        let id = call.id.clone().unwrap_or_default();
        let name = call.name.clone().unwrap_or_default();
        match self.tools.get(&name) {
            Some(f) => f(call).await,
            // Reported to the model as a tool failure rather than raised to the
            // caller: the harness only asks for tools the session declared, so
            // this is a client-side wiring bug, and failing the one call lets
            // the agent recover or explain itself instead of wedging the turn.
            None => ToolResponse::error(id, format!("no handler registered for tool `{name}`")),
        }
    }

    pub(crate) async fn call_hook(&self, request: CallHookRequest) -> CallHookResponse {
        let request_id = request.request_id.clone();
        match &self.hook {
            Some(f) => f(request).await,
            None => CallHookResponse {
                request_id,
                empty_result: Some(EmptyResult {}),
                ..Default::default()
            },
        }
    }

    pub(crate) async fn call_policy(
        &self,
        request: PolicyDecisionRequest,
    ) -> PolicyDecisionResponse {
        let request_id = request.request_id.clone();
        match &self.policy {
            Some(f) => f(request).await,
            // `NO_MATCH` defers to whatever static rules the harness has, which
            // is the safe reading of "the client expressed no opinion".
            None => PolicyDecisionResponse {
                request_id,
                outcome: Some(PolicyEvaluationOutcome::NoMatch),
                ..Default::default()
            },
        }
    }

    pub(crate) async fn call_questions(
        &self,
        request: UserQuestionsRequest,
    ) -> UserQuestionsResponse {
        match &self.questions {
            Some(f) => f(request).await,
            // Cancelling is the only answer that cannot be wrong: it tells the
            // agent nobody is there, and unblocks the turn.
            None => UserQuestionsResponse {
                cancelled: Some(true),
                ..Default::default()
            },
        }
    }

    pub(crate) async fn call_confirm(&self, step: StepUpdate) -> bool {
        match &self.confirm {
            Some(f) => f(step).await,
            // Refusing by default. The harness only asks when the session was
            // configured to require confirmation, so silently approving would
            // undo the very control the caller asked for.
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn an_unregistered_tool_fails_that_call_only() {
        let handlers = Handlers::new();
        let response = handlers
            .call_tool(ToolCall {
                id: Some("call-1".into()),
                name: Some("missing".into()),
                ..Default::default()
            })
            .await;
        assert_eq!(response.id.as_deref(), Some("call-1"));
        assert!(response
            .error_message
            .unwrap()
            .contains("no handler registered"));
    }

    #[tokio::test]
    async fn a_registered_tool_is_dispatched_by_name() {
        let handlers = Handlers::new().tool("echo", |call| async move {
            ToolResponse::ok(call.id.unwrap_or_default(), "42")
        });
        let response = handlers
            .call_tool(ToolCall {
                id: Some("c".into()),
                name: Some("echo".into()),
                ..Default::default()
            })
            .await;
        assert_eq!(response.response_json.as_deref(), Some("42"));
    }

    #[tokio::test]
    async fn defaults_keep_the_turn_moving() {
        let handlers = Handlers::new();
        let hook = handlers
            .call_hook(CallHookRequest {
                request_id: Some("r".into()),
                ..Default::default()
            })
            .await;
        assert_eq!(hook.request_id.as_deref(), Some("r"));
        assert!(hook.empty_result.is_some());

        let policy = handlers
            .call_policy(PolicyDecisionRequest {
                request_id: Some("p".into()),
                ..Default::default()
            })
            .await;
        assert_eq!(policy.outcome, Some(PolicyEvaluationOutcome::NoMatch));

        let questions = handlers
            .call_questions(UserQuestionsRequest::default())
            .await;
        assert_eq!(questions.cancelled, Some(true));
    }
}
