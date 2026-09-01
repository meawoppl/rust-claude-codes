//! Builder for `pi` invocations — assembles argv for `--mode json`
//! one-shots and `--mode rpc` servers without hand-rolled string lists.

use std::ffi::OsString;
use std::path::PathBuf;

/// Output mode (`--mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Interactive/plain text (the CLI default; rarely useful from an SDK).
    Text,
    /// One-shot JSONL event stream on stdout.
    #[default]
    Json,
    /// Headless stdin/stdout command protocol ([`crate::rpc`]).
    Rpc,
}

impl Mode {
    fn as_str(self) -> &'static str {
        match self {
            Mode::Text => "text",
            Mode::Json => "json",
            Mode::Rpc => "rpc",
        }
    }
}

/// Typed argv builder for the `pi` CLI.
///
/// ```
/// use pi_codes::cli::{Mode, PiCliBuilder};
/// let args = PiCliBuilder::new()
///     .mode(Mode::Rpc)
///     .no_session(true)
///     .assembled_args();
/// assert_eq!(args[0], "--mode");
/// ```
#[derive(Debug, Clone, Default)]
pub struct PiCliBuilder {
    mode: Mode,
    provider: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
    system_prompt: Option<String>,
    session: Option<String>,
    session_dir: Option<PathBuf>,
    session_id: Option<String>,
    fork: Option<String>,
    continue_session: bool,
    no_session: bool,
    name: Option<String>,
    no_tools: bool,
    tools: Option<String>,
    exclude_tools: Option<String>,
    thinking: Option<String>,
    print: bool,
    extra_args: Vec<OsString>,
    prompt: Option<String>,
}

impl PiCliBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    pub fn provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// API key for the provider. Prefer environment variables — argv is
    /// visible to other processes; this exists for parity with the CLI.
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// `--session <path|id>` — use a specific session file or partial id.
    pub fn session(mut self, session: impl Into<String>) -> Self {
        self.session = Some(session.into());
        self
    }

    pub fn session_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.session_dir = Some(dir.into());
        self
    }

    /// `--session-id <id>` — exact project session id, created if missing.
    pub fn session_id(mut self, id: impl Into<String>) -> Self {
        self.session_id = Some(id.into());
        self
    }

    /// `--fork <path|id>` — fork a session into a new one.
    pub fn fork(mut self, source: impl Into<String>) -> Self {
        self.fork = Some(source.into());
        self
    }

    /// `--continue` — continue the previous session.
    pub fn continue_session(mut self, enabled: bool) -> Self {
        self.continue_session = enabled;
        self
    }

    /// `--no-session` — ephemeral run, nothing persisted.
    pub fn no_session(mut self, enabled: bool) -> Self {
        self.no_session = enabled;
        self
    }

    /// `--name` — session display name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// `--no-tools` — disable all tools.
    pub fn no_tools(mut self, enabled: bool) -> Self {
        self.no_tools = enabled;
        self
    }

    /// `--tools` — comma-separated tool allowlist.
    pub fn tools(mut self, tools: impl Into<String>) -> Self {
        self.tools = Some(tools.into());
        self
    }

    /// `--exclude-tools` — comma-separated tool denylist.
    pub fn exclude_tools(mut self, tools: impl Into<String>) -> Self {
        self.exclude_tools = Some(tools.into());
        self
    }

    /// `--thinking` — off, minimal, low, medium, high, xhigh, max.
    pub fn thinking(mut self, level: impl Into<String>) -> Self {
        self.thinking = Some(level.into());
        self
    }

    /// `--print` — non-interactive: process the prompt and exit.
    pub fn print(mut self, enabled: bool) -> Self {
        self.print = enabled;
        self
    }

    /// Escape hatch: raw args appended after every typed flag, before
    /// the positional prompt.
    pub fn extra_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.extra_args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Positional prompt (used with `--mode json` one-shots).
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// The full argv (binary not included) — pure and testable.
    pub fn assembled_args(&self) -> Vec<OsString> {
        let mut a: Vec<OsString> = vec!["--mode".into(), self.mode.as_str().into()];
        let mut flag = |name: &str, v: &Option<String>| {
            if let Some(v) = v {
                a.push(name.into());
                a.push(v.into());
            }
        };
        flag("--provider", &self.provider);
        flag("--model", &self.model);
        flag("--api-key", &self.api_key);
        flag("--system-prompt", &self.system_prompt);
        flag("--session", &self.session);
        flag("--session-id", &self.session_id);
        flag("--fork", &self.fork);
        flag("--name", &self.name);
        flag("--tools", &self.tools);
        flag("--exclude-tools", &self.exclude_tools);
        flag("--thinking", &self.thinking);
        if let Some(dir) = &self.session_dir {
            a.push("--session-dir".into());
            a.push(dir.into());
        }
        if self.continue_session {
            a.push("--continue".into());
        }
        if self.no_session {
            a.push("--no-session".into());
        }
        if self.no_tools {
            a.push("--no-tools".into());
        }
        if self.print {
            a.push("--print".into());
        }
        a.extend(self.extra_args.iter().cloned());
        if let Some(p) = &self.prompt {
            a.push(p.into());
        }
        a
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembled_args_order_flags_then_extras_then_prompt() {
        let args = PiCliBuilder::new()
            .mode(Mode::Json)
            .provider("anthropic")
            .no_session(true)
            .extra_args(["--verbose"])
            .prompt("hello")
            .assembled_args();
        let s: Vec<String> = args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(s[0..2], ["--mode", "json"]);
        assert!(s.contains(&"--no-session".to_string()));
        let vi = s.iter().position(|x| x == "--verbose").unwrap();
        assert_eq!(s.last().unwrap(), "hello");
        assert!(vi < s.len() - 1, "extras precede the positional prompt");
    }

    #[test]
    fn rpc_mode_default_has_no_prompt() {
        let args = PiCliBuilder::new().mode(Mode::Rpc).assembled_args();
        assert_eq!(args.len(), 2);
        assert_eq!(args[1], "rpc");
    }
}
