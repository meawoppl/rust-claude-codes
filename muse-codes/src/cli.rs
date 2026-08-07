//! Builder for spawning headless `muse exec --json` runs.

use crate::error::{Error, Result};
use std::path::PathBuf;
use std::process::Stdio;

/// Provider mode for a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    /// The Meta provider (default; requires credentials — `muse login`,
    /// `META_API_KEY`, or `~/.config/muse/auth.json`).
    Meta,
    /// Credential-free echo provider — exercises the full event stream
    /// without model calls. What this crate's committed captures use.
    Echo,
}

impl Provider {
    fn as_str(self) -> &'static str {
        match self {
            Provider::Meta => "meta",
            Provider::Echo => "echo",
        }
    }
}

/// Session git-worktree mode (`--worktree`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorktreeMode {
    Off,
    /// Create a fresh worktree (base ref via
    /// [`MuseExecBuilder::worktree_base`], default `HEAD`).
    Create,
    /// Use an existing worktree (path via
    /// [`MuseExecBuilder::worktree_existing`]).
    Existing,
}

impl WorktreeMode {
    fn as_str(self) -> &'static str {
        match self {
            WorktreeMode::Off => "off",
            WorktreeMode::Create => "create",
            WorktreeMode::Existing => "existing",
        }
    }
}

/// Builder for one `muse exec --json` invocation.
///
/// Covers the full `muse exec` flag surface (Muse Code 0.1.0), verified
/// flag-by-flag against the real binary. Constraints the CLI enforces are
/// noted on each method (several flags are Meta-provider-only; the echo
/// provider rejects them at startup with a usage error).
#[derive(Debug, Clone)]
pub struct MuseExecBuilder {
    binary: String,
    prompt: String,
    prompt_file: Option<PathBuf>,
    api_key_stdin: bool,
    provider: Option<Provider>,
    preset: Option<String>,
    model: Option<String>,
    session_id: Option<String>,
    reasoning_effort: Option<String>,
    parallel_tool_calls: Option<bool>,
    base_url: Option<String>,
    agents: Option<String>,
    images: Vec<PathBuf>,
    workspace: Option<PathBuf>,
    worktree: Option<WorktreeMode>,
    worktree_base: Option<String>,
    worktree_existing: Option<PathBuf>,
    context_compaction_strategy: Option<String>,
    context_compaction_soft_threshold: Option<f64>,
    context_compaction_hard_threshold: Option<f64>,
    max_model_steps: Option<u64>,
    max_tool_output_bytes: Option<u64>,
    allow_workspace_switch: bool,
    user_input_auto_resolve: bool,
    subagent_worktree_isolation: bool,
    disable_web_tools: bool,
    no_foreign_personal_context: bool,
    no_session_log: bool,
    yolo: bool,
    trust_workspace: bool,
    disable_approval: bool,
    disable_sandbox: bool,
    sandbox_network: Option<String>,
    disable_write: bool,
    disable_shell: bool,
    enable_shell_tool: bool,
    extra_args: Vec<String>,
    working_directory: Option<PathBuf>,
    envs: Vec<(String, String)>,
}

impl Default for MuseExecBuilder {
    /// An empty-prompt builder resolving `muse` from `PATH`; use
    /// [`MuseExecBuilder::new`] (or [`MuseExecBuilder::prompt_file`]) to
    /// supply the prompt.
    fn default() -> Self {
        Self {
            binary: "muse".to_string(),
            prompt: String::new(),
            prompt_file: None,
            api_key_stdin: false,
            provider: None,
            preset: None,
            model: None,
            session_id: None,
            reasoning_effort: None,
            parallel_tool_calls: None,
            base_url: None,
            agents: None,
            images: Vec::new(),
            workspace: None,
            worktree: None,
            worktree_base: None,
            worktree_existing: None,
            context_compaction_strategy: None,
            context_compaction_soft_threshold: None,
            context_compaction_hard_threshold: None,
            max_model_steps: None,
            max_tool_output_bytes: None,
            allow_workspace_switch: false,
            user_input_auto_resolve: false,
            subagent_worktree_isolation: false,
            disable_web_tools: false,
            no_foreign_personal_context: false,
            no_session_log: false,
            yolo: false,
            trust_workspace: false,
            disable_approval: false,
            disable_sandbox: false,
            sandbox_network: None,
            disable_write: false,
            disable_shell: false,
            enable_shell_tool: false,
            extra_args: Vec::new(),
            working_directory: None,
            envs: Vec::new(),
        }
    }
}

impl MuseExecBuilder {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            ..Self::default()
        }
    }

    /// Use a specific binary instead of `muse` from `PATH`.
    pub fn binary(mut self, path: impl Into<String>) -> Self {
        self.binary = path.into();
        self
    }

    pub fn provider(mut self, provider: Provider) -> Self {
        self.provider = Some(provider);
        self
    }

    /// Built-in preset (`native-basic`, `miniswe`).
    pub fn preset(mut self, preset: impl Into<String>) -> Self {
        self.preset = Some(preset.into());
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Run under a caller-supplied session id (`--session-id`), the basis of
    /// multi-turn continuity: each turn is its own process, and passing the
    /// same id makes the CLI continue that session rather than start a new
    /// one. The id is adopted verbatim as the `stream.id` on every emitted
    /// record.
    ///
    /// Supplying your own id is also what makes
    /// [`MuseRecord`](crate::MuseRecord) identity safe to key on: record
    /// `id`s are UUID-shaped counters that repeat across sessions, so the
    /// only unique handle is the composite `(stream.id, id)` — and that is
    /// trustworthy precisely because `stream.id` is yours. (When omitted,
    /// the CLI mints a random v4 of its own.)
    pub fn session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Meta reasoning effort (`none|minimal|low|medium|high|xhigh|ultra`).
    /// Not supported with [`Provider::Echo`].
    pub fn reasoning_effort(mut self, effort: impl Into<String>) -> Self {
        self.reasoning_effort = Some(effort.into());
        self
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Read the prompt from a file (`--prompt-file`) instead of passing it
    /// as an argument; the positional prompt is omitted when set.
    pub fn prompt_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.prompt_file = Some(path.into());
        self
    }

    /// Read the provider API key from stdin (`--api-key-stdin`).
    /// Meta-provider-only (the echo provider rejects it at startup). When
    /// set, the child's stdin is piped instead of null — the caller writes
    /// the key (newline-terminated) and closes it.
    pub fn api_key_stdin(mut self, enabled: bool) -> Self {
        self.api_key_stdin = enabled;
        self
    }

    /// Meta API parallel tool calls: `true` → `--parallel-tool-calls`,
    /// `false` → `--no-parallel-tool-calls`. Meta-provider-only (the echo
    /// provider rejects both at startup).
    pub fn parallel_tool_calls(mut self, enabled: bool) -> Self {
        self.parallel_tool_calls = Some(enabled);
        self
    }

    /// One ephemeral agent-definition overlay as JSON (`--agents`).
    /// Accepted by `exec` even though only the top-level help lists it
    /// (verified against the binary).
    pub fn agents(mut self, json: impl Into<String>) -> Self {
        self.agents = Some(json.into());
        self
    }

    /// Attach a local image file (`--image`, repeatable). Requires an
    /// image-capable provider — the echo provider rejects it at startup.
    pub fn image(mut self, path: impl Into<PathBuf>) -> Self {
        self.images.push(path.into());
        self
    }

    /// Root policy-gated workspace tools at this path (`--workspace`).
    pub fn workspace(mut self, path: impl Into<PathBuf>) -> Self {
        self.workspace = Some(path.into());
        self
    }

    /// Session git worktree mode (`--worktree off|create|existing`).
    pub fn worktree(mut self, mode: WorktreeMode) -> Self {
        self.worktree = Some(mode);
        self
    }

    /// Base ref for [`WorktreeMode::Create`] (`--worktree-base`, default
    /// `HEAD`).
    pub fn worktree_base(mut self, git_ref: impl Into<String>) -> Self {
        self.worktree_base = Some(git_ref.into());
        self
    }

    /// Existing worktree path for [`WorktreeMode::Existing`]
    /// (`--worktree-existing`).
    pub fn worktree_existing(mut self, path: impl Into<PathBuf>) -> Self {
        self.worktree_existing = Some(path.into());
        self
    }

    /// Context compaction strategy id (`--context-compaction-strategy`,
    /// e.g. `summary-preserved-suffix/v1`). Kept as a string: the ids are
    /// versioned and the set will drift.
    pub fn context_compaction_strategy(mut self, id: impl Into<String>) -> Self {
        self.context_compaction_strategy = Some(id.into());
        self
    }

    /// Soft compaction threshold fraction
    /// (`--context-compaction-soft-threshold`).
    pub fn context_compaction_soft_threshold(mut self, fraction: f64) -> Self {
        self.context_compaction_soft_threshold = Some(fraction);
        self
    }

    /// Hard compaction threshold fraction
    /// (`--context-compaction-hard-threshold`).
    pub fn context_compaction_hard_threshold(mut self, fraction: f64) -> Self {
        self.context_compaction_hard_threshold = Some(fraction);
        self
    }

    /// Cap the number of model steps (`--max-model-steps`).
    pub fn max_model_steps(mut self, steps: u64) -> Self {
        self.max_model_steps = Some(steps);
        self
    }

    /// Cap tool output bytes fed back to the model
    /// (`--max-tool-output-bytes`).
    pub fn max_tool_output_bytes(mut self, bytes: u64) -> Self {
        self.max_tool_output_bytes = Some(bytes);
        self
    }

    /// Allow switching the workspace mid-run (`--allow-workspace-switch`).
    /// The CLI requires [`MuseExecBuilder::session_id`] alongside it
    /// (verified: rejected at startup otherwise).
    pub fn allow_workspace_switch(mut self, enabled: bool) -> Self {
        self.allow_workspace_switch = enabled;
        self
    }

    /// Offer `request_user_input` and auto-cancel prompts in headless runs
    /// (`--user-input-auto-resolve`).
    pub fn user_input_auto_resolve(mut self, enabled: bool) -> Self {
        self.user_input_auto_resolve = enabled;
        self
    }

    /// Compatibility flag for subagent worktree isolation
    /// (`--subagent-worktree-isolation`); the capability defaults on.
    pub fn subagent_worktree_isolation(mut self, enabled: bool) -> Self {
        self.subagent_worktree_isolation = enabled;
        self
    }

    /// Disable web tools for this run (`--disable-web-tools`).
    pub fn disable_web_tools(mut self, disabled: bool) -> Self {
        self.disable_web_tools = disabled;
        self
    }

    /// Exclude foreign personal rules and skills
    /// (`--no-foreign-personal-context`).
    pub fn no_foreign_personal_context(mut self, excluded: bool) -> Self {
        self.no_foreign_personal_context = excluded;
        self
    }

    /// Do not persist session event logs to disk (`--no-session-log`).
    /// Conflicts with [`MuseExecBuilder::session_id`]: the CLI rejects the
    /// pair at startup ("a session id needs retained logging" — verified),
    /// which also means multi-turn continuity requires the log.
    pub fn no_session_log(mut self, disabled: bool) -> Self {
        self.no_session_log = disabled;
        self
    }

    /// Disable approval and sandbox and trust this workspace for the run
    /// (`--yolo`).
    pub fn yolo(mut self, enabled: bool) -> Self {
        self.yolo = enabled;
        self
    }

    /// Load this workspace's skills and rules for the run
    /// (`--trust-workspace`).
    pub fn trust_workspace(mut self, trusted: bool) -> Self {
        self.trust_workspace = trusted;
        self
    }

    /// Disable tool approval prompts for the run (`--disable-approval`).
    pub fn disable_approval(mut self, disabled: bool) -> Self {
        self.disable_approval = disabled;
        self
    }

    /// Disable shell filesystem/network sandboxing for the run
    /// (`--disable-sandbox`).
    pub fn disable_sandbox(mut self, disabled: bool) -> Self {
        self.disable_sandbox = disabled;
        self
    }

    /// Sandbox network mode (`--sandbox-network`, default `proxy-only`).
    /// Kept as a string: the help names no closed set of modes.
    pub fn sandbox_network(mut self, mode: impl Into<String>) -> Self {
        self.sandbox_network = Some(mode.into());
        self
    }

    /// Disable non-shell workspace filesystem writes (`--disable-write`).
    pub fn disable_write(mut self, disabled: bool) -> Self {
        self.disable_write = disabled;
        self
    }

    /// Disable workspace shell execution (`--disable-shell`).
    pub fn disable_shell(mut self, disabled: bool) -> Self {
        self.disable_shell = disabled;
        self
    }

    /// Use the legacy shell tool instead of managed bash
    /// (`--enable-shell-tool`).
    pub fn enable_shell_tool(mut self, enabled: bool) -> Self {
        self.enable_shell_tool = enabled;
        self
    }

    /// Raw argument passthrough for flags this builder does not model
    /// (mirrors codex-codes). Appended AFTER every typed flag and BEFORE
    /// the positional prompt, so callers relaying user-supplied tokens
    /// (e.g. a launch dialog's extra-args box) need no flag parser of
    /// their own. Prefer the typed setters when one exists.
    pub fn extra_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.extra_args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn working_directory(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_directory = Some(dir.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.envs.push((key.into(), value.into()));
        self
    }

    /// Resolve the binary and assemble the command with piped stdio.
    pub fn build_command(&self) -> Result<tokio::process::Command> {
        let program = which::which(&self.binary).map_err(|_| Error::BinaryNotFound {
            name: self.binary.clone(),
        })?;
        let mut cmd = tokio::process::Command::new(program);
        cmd.arg("exec").arg("--json");
        if let Some(p) = self.provider {
            cmd.args(["--provider", p.as_str()]);
        }
        if let Some(p) = &self.preset {
            cmd.args(["--preset", p]);
        }
        if let Some(m) = &self.model {
            cmd.args(["--model", m]);
        }
        if let Some(s) = &self.session_id {
            cmd.args(["--session-id", s]);
        }
        if let Some(e) = &self.reasoning_effort {
            cmd.args(["--reasoning-effort", e]);
        }
        if let Some(enabled) = self.parallel_tool_calls {
            cmd.arg(if enabled {
                "--parallel-tool-calls"
            } else {
                "--no-parallel-tool-calls"
            });
        }
        if let Some(u) = &self.base_url {
            cmd.args(["--base-url", u]);
        }
        if let Some(a) = &self.agents {
            cmd.args(["--agents", a]);
        }
        for image in &self.images {
            cmd.arg("--image").arg(image);
        }
        if let Some(w) = &self.workspace {
            cmd.arg("--workspace").arg(w);
        }
        if let Some(mode) = self.worktree {
            cmd.args(["--worktree", mode.as_str()]);
        }
        if let Some(base) = &self.worktree_base {
            cmd.args(["--worktree-base", base]);
        }
        if let Some(path) = &self.worktree_existing {
            cmd.arg("--worktree-existing").arg(path);
        }
        if let Some(s) = &self.context_compaction_strategy {
            cmd.args(["--context-compaction-strategy", s]);
        }
        if let Some(f) = self.context_compaction_soft_threshold {
            cmd.args(["--context-compaction-soft-threshold", &f.to_string()]);
        }
        if let Some(f) = self.context_compaction_hard_threshold {
            cmd.args(["--context-compaction-hard-threshold", &f.to_string()]);
        }
        if let Some(n) = self.max_model_steps {
            cmd.args(["--max-model-steps", &n.to_string()]);
        }
        if let Some(n) = self.max_tool_output_bytes {
            cmd.args(["--max-tool-output-bytes", &n.to_string()]);
        }
        if self.api_key_stdin {
            cmd.arg("--api-key-stdin");
        }
        if self.allow_workspace_switch {
            cmd.arg("--allow-workspace-switch");
        }
        if self.user_input_auto_resolve {
            cmd.arg("--user-input-auto-resolve");
        }
        if self.subagent_worktree_isolation {
            cmd.arg("--subagent-worktree-isolation");
        }
        if self.disable_web_tools {
            cmd.arg("--disable-web-tools");
        }
        if self.no_foreign_personal_context {
            cmd.arg("--no-foreign-personal-context");
        }
        if self.no_session_log {
            cmd.arg("--no-session-log");
        }
        if self.yolo {
            cmd.arg("--yolo");
        }
        if self.trust_workspace {
            cmd.arg("--trust-workspace");
        }
        if self.disable_approval {
            cmd.arg("--disable-approval");
        }
        if self.disable_sandbox {
            cmd.arg("--disable-sandbox");
        }
        if let Some(mode) = &self.sandbox_network {
            cmd.args(["--sandbox-network", mode]);
        }
        if self.disable_write {
            cmd.arg("--disable-write");
        }
        if self.disable_shell {
            cmd.arg("--disable-shell");
        }
        if self.enable_shell_tool {
            cmd.arg("--enable-shell-tool");
        }
        for arg in &self.extra_args {
            cmd.arg(arg);
        }
        // `--prompt-file` replaces the positional prompt.
        if let Some(file) = &self.prompt_file {
            cmd.arg("--prompt-file").arg(file);
        } else {
            cmd.arg(&self.prompt);
        }
        // `--api-key-stdin` needs a writable stdin; the caller writes the
        // key and closes it. Otherwise stdin stays null.
        cmd.stdin(if self.api_key_stdin {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
        if let Some(dir) = &self.working_directory {
            cmd.current_dir(dir);
        }
        for (k, v) in &self.envs {
            cmd.env(k, v);
        }
        Ok(cmd)
    }

    /// Spawn the run.
    pub async fn spawn(&self) -> Result<tokio::process::Child> {
        Ok(self.build_command()?.spawn()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(builder: &MuseExecBuilder) -> Vec<String> {
        let cmd = builder.build_command().expect("muse on PATH");
        cmd.as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    /// Every flag lands on the command line exactly as the CLI spells it —
    /// the full `muse exec` surface, so a missing arm here is a missing
    /// flag.
    #[test]
    fn full_flag_surface_assembles() {
        let b = MuseExecBuilder::new("do the thing")
            .provider(Provider::Meta)
            .preset("native-basic")
            .model("m-1")
            .session_id("s-1")
            .reasoning_effort("high")
            .parallel_tool_calls(true)
            .base_url("http://localhost:1")
            .agents("{}")
            .image("/tmp/a.png")
            .image("/tmp/b.png")
            .workspace("/ws")
            .worktree(WorktreeMode::Create)
            .worktree_base("main")
            .worktree_existing("/wt")
            .context_compaction_strategy("summary-preserved-suffix/v1")
            .context_compaction_soft_threshold(0.7)
            .context_compaction_hard_threshold(0.9)
            .max_model_steps(5)
            .max_tool_output_bytes(1000)
            .api_key_stdin(true)
            .allow_workspace_switch(true)
            .user_input_auto_resolve(true)
            .subagent_worktree_isolation(true)
            .disable_web_tools(true)
            .no_foreign_personal_context(true)
            .no_session_log(true)
            .yolo(true)
            .trust_workspace(true)
            .disable_approval(true)
            .disable_sandbox(true)
            .sandbox_network("proxy-only")
            .disable_write(true)
            .disable_shell(true)
            .enable_shell_tool(true);
        let got = args(&b);
        let want: Vec<&str> = vec![
            "exec",
            "--json",
            "--provider",
            "meta",
            "--preset",
            "native-basic",
            "--model",
            "m-1",
            "--session-id",
            "s-1",
            "--reasoning-effort",
            "high",
            "--parallel-tool-calls",
            "--base-url",
            "http://localhost:1",
            "--agents",
            "{}",
            "--image",
            "/tmp/a.png",
            "--image",
            "/tmp/b.png",
            "--workspace",
            "/ws",
            "--worktree",
            "create",
            "--worktree-base",
            "main",
            "--worktree-existing",
            "/wt",
            "--context-compaction-strategy",
            "summary-preserved-suffix/v1",
            "--context-compaction-soft-threshold",
            "0.7",
            "--context-compaction-hard-threshold",
            "0.9",
            "--max-model-steps",
            "5",
            "--max-tool-output-bytes",
            "1000",
            "--api-key-stdin",
            "--allow-workspace-switch",
            "--user-input-auto-resolve",
            "--subagent-worktree-isolation",
            "--disable-web-tools",
            "--no-foreign-personal-context",
            "--no-session-log",
            "--yolo",
            "--trust-workspace",
            "--disable-approval",
            "--disable-sandbox",
            "--sandbox-network",
            "proxy-only",
            "--disable-write",
            "--disable-shell",
            "--enable-shell-tool",
            "do the thing",
        ];
        assert_eq!(got, want);
    }

    /// `--no-parallel-tool-calls` is the false arm of one knob, not a
    /// separate builder method.
    #[test]
    fn parallel_tool_calls_false_emits_the_no_flag() {
        let got = args(&MuseExecBuilder::new("p").parallel_tool_calls(false));
        assert!(got.contains(&"--no-parallel-tool-calls".to_string()));
        assert!(!got.contains(&"--parallel-tool-calls".to_string()));
    }

    /// `--prompt-file` replaces the positional prompt entirely.
    #[test]
    fn prompt_file_replaces_the_positional_prompt() {
        let got = args(&MuseExecBuilder::new("ignored").prompt_file("/tmp/p.txt"));
        assert_eq!(got.last().map(String::as_str), Some("/tmp/p.txt"));
        assert!(got.contains(&"--prompt-file".to_string()));
        assert!(!got.contains(&"ignored".to_string()));
    }

    /// Raw passthrough lands after typed flags, before the prompt — the
    /// position a launch dialog's freeform tokens must occupy.
    #[test]
    fn extra_args_sit_between_typed_flags_and_the_prompt() {
        let got = args(
            &MuseExecBuilder::new("go")
                .model("m-1")
                .extra_args(["--reasoning-effort", "low"]),
        );
        assert_eq!(
            got,
            [
                "exec",
                "--json",
                "--model",
                "m-1",
                "--reasoning-effort",
                "low",
                "go"
            ]
        );
    }

    /// Nothing optional leaks into a minimal invocation.
    #[test]
    fn minimal_invocation_stays_minimal() {
        let got = args(&MuseExecBuilder::new("hi"));
        assert_eq!(got, ["exec", "--json", "hi"]);
    }
}
