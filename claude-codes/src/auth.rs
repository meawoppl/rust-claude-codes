//! Login support tooling for the Claude CLI (`--features auth`).
//!
//! The CLI's login flows (`claude auth login`, `claude setup-token`) are
//! interactive Ink TUIs: they render nothing on a pipe and wait forever, so
//! plain `std::process::Command` cannot drive them. This module runs them
//! under a **pseudo-terminal** and turns the interaction into three plain
//! function calls, matching the flow's human shape — "visit this URL to log
//! in, then paste the code back":
//!
//! ```no_run
//! use claude_codes::auth::{auth_status, LoginFlow, LoginMode};
//! use std::time::Duration;
//!
//! # fn main() -> claude_codes::Result<()> {
//! // 1. Start the flow and get the URL to show the user.
//! let mut flow = LoginFlow::start(LoginMode::SetupToken)?;
//! let url = flow.auth_url(Duration::from_secs(30))?;
//! println!("Visit to sign in: {url}");
//!
//! // 2. The user authorizes in a browser and brings back a code.
//! let code = read_code_from_user();
//! flow.submit_code(&code)?;
//!
//! // 3. Reap the outcome. SetupToken yields a long-lived `sk-ant-oat01-…`
//! //    token; the login modes persist credentials for later CLI runs.
//! let outcome = flow.finish(Duration::from_secs(60))?;
//! if let Some(token) = outcome.token {
//!     // Hand to ClaudeCliBuilder::oauth_token(...) or store it.
//! }
//! assert!(auth_status()?.logged_in);
//! # Ok(()) }
//! # fn read_code_from_user() -> String { unimplemented!() }
//! ```
//!
//! The API is blocking (PTY I/O is thread-driven); async callers should wrap
//! calls in `tokio::task::spawn_blocking`. Dropping a [`LoginFlow`] before
//! [`finish`](LoginFlow::finish) cancels the login and kills the child.

use crate::error::{Error, Result};
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, PtySize};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// Authentication status as reported by `claude auth status --json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    /// Whether any credential (OAuth login, API key, or token) is active.
    pub logged_in: bool,
    /// Credential kind, e.g. `"claude.ai"` or `"console"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_method: Option<String>,
    /// Backing provider, e.g. `"firstParty"`, `"bedrock"`, `"vertex"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub org_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub org_name: Option<String>,
    /// e.g. `"pro"`, `"max"`; absent for API-key auth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscription_type: Option<String>,
    /// Forward-compatible catch-all for fields newer CLIs may add.
    #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Query `claude auth status --json` using the `claude` binary on `PATH`.
pub fn auth_status() -> Result<AuthStatus> {
    auth_status_with_binary("claude")
}

/// [`auth_status`] against a specific CLI binary.
pub fn auth_status_with_binary(binary: &str) -> Result<AuthStatus> {
    let out = std::process::Command::new(binary)
        .args(["auth", "status", "--json"])
        .output()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => Error::BinaryNotFound {
                name: binary.to_string(),
            },
            _ => Error::Io(e),
        })?;
    // The CLI exits non-zero when logged out but still prints valid JSON,
    // so parse stdout regardless of status.
    serde_json::from_slice(&out.stdout).map_err(Error::Json)
}

/// Which login flow to drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginMode {
    /// `claude setup-token` — mints a long-lived `sk-ant-oat01-…` token
    /// (requires a Claude subscription). The token is returned in
    /// [`LoginOutcome::token`] and is NOT persisted by the CLI; pass it to
    /// [`ClaudeCliBuilder::oauth_token`](crate::cli::ClaudeCliBuilder::oauth_token).
    SetupToken,
    /// `claude auth login --claudeai` — subscription login, persisted by the
    /// CLI for subsequent runs.
    ClaudeAi,
    /// `claude auth login --console` — Anthropic Console (API billing) login.
    Console,
}

impl LoginMode {
    fn args(self) -> &'static [&'static str] {
        match self {
            LoginMode::SetupToken => &["setup-token"],
            LoginMode::ClaudeAi => &["auth", "login", "--claudeai"],
            LoginMode::Console => &["auth", "login", "--console"],
        }
    }
}

/// Which PTY channel a minted token was recovered from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenSource {
    /// Rendered in the terminal's visible text.
    Screen,
    /// Carried in an OSC 52 clipboard-copy escape's base64 payload — exact
    /// bytes, immune to display wrapping, column positioning, and masking.
    Osc52,
}

/// What the OSC 52 clipboard channel showed by the time the flow settled —
/// success-path telemetry so a `token: None` outcome names the reason
/// instead of leaving "no affordance" and "affordance fired with a non-token
/// payload" indistinguishable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Osc52Status {
    /// No OSC 52 sequence ever appeared (the screen may offer no copy
    /// affordance, or the nudge didn't trigger it).
    Absent,
    /// A sequence started but never terminated before the flow settled.
    Unterminated,
    /// Terminated sequence(s) whose payload wasn't valid base64.
    Undecodable,
    /// Valid payload(s) seen, none containing a token (e.g. the URL copy).
    PresentNoToken,
    /// The token was recovered from this channel
    /// ([`LoginOutcome::token_source`] is [`TokenSource::Osc52`]).
    TokenRecovered,
}

/// Result of a completed [`LoginFlow`].
#[derive(Debug, Clone)]
pub struct LoginOutcome {
    /// The minted long-lived token (`SetupToken` mode only). Sourced from the
    /// CLI's screen output or its OSC 52 clipboard copy; `None` when the CLI
    /// never exposed the token over the PTY (it masks secret material), even
    /// on success — check [`credentials_updated`](Self::credentials_updated).
    pub token: Option<String>,
    /// Channel the token came from; `None` when `token` is `None`.
    pub token_source: Option<TokenSource>,
    /// True when the CLI's credentials store (`.credentials.json` under
    /// `CLAUDE_CONFIG_DIR` or `~/.claude`) was created or updated after the
    /// code submission — the authoritative success signal, independent of
    /// anything rendered on screen.
    pub credentials_updated: bool,
    /// State of the OSC 52 clipboard channel when the flow settled.
    pub osc52: Osc52Status,
    /// True when the flow pressed the TUI's `c` copy affordance after
    /// success was confirmed — so any unexpected TUI consequence is
    /// attributable rather than mysterious.
    pub copy_nudge_sent: bool,
    /// The flow's full terminal output with ANSI escapes stripped — for
    /// surfacing the CLI's own error text when something goes wrong.
    pub transcript: String,
}

impl From<&Osc52Scan> for Osc52Status {
    fn from(scan: &Osc52Scan) -> Self {
        match scan {
            Osc52Scan::Absent => Osc52Status::Absent,
            Osc52Scan::Unterminated => Osc52Status::Unterminated,
            Osc52Scan::Undecodable => Osc52Status::Undecodable,
            Osc52Scan::NoTokenInPayload => Osc52Status::PresentNoToken,
            Osc52Scan::Token(_) => Osc52Status::TokenRecovered,
        }
    }
}

/// Shared PTY output buffer: bytes so far + whether the reader hit EOF.
type OutBuf = Arc<(Mutex<(Vec<u8>, bool)>, Condvar)>;

/// An in-flight interactive login, driven over a pseudo-terminal.
pub struct LoginFlow {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    writer: Box<dyn std::io::Write + Send>,
    // The master must outlive the flow; the SLAVE is deliberately dropped
    // right after spawn. Holding the slave open means the master never sees
    // EOF when the child dies — the reader blocks forever and a 5-second
    // child death presents as a full-timeout mystery (production attempt
    // four: 85 seconds of polling a corpse).
    _master: Box<dyn portable_pty::MasterPty + Send>,
    buf: OutBuf,
    mode: LoginMode,
    finished: bool,
    /// Credentials store path + its state when the flow started, so
    /// create-or-update after submission is detectable as a success signal.
    creds: CredsWatch,
}

/// Snapshot-based watcher for the CLI's credentials file.
#[derive(Debug, Clone)]
struct CredsWatch {
    path: Option<std::path::PathBuf>,
    baseline: Option<std::time::SystemTime>,
}

impl CredsWatch {
    fn snapshot() -> Self {
        let path = credentials_path();
        let baseline = path.as_ref().and_then(|p| mtime(p));
        Self { path, baseline }
    }

    /// True when the credentials file now exists with an mtime newer than
    /// (or absent from) the baseline taken at flow start.
    fn updated(&self) -> bool {
        let Some(path) = &self.path else { return false };
        match (mtime(path), self.baseline) {
            (Some(now), Some(then)) => now > then,
            (Some(_), None) => true,
            (None, _) => false,
        }
    }
}

/// `$CLAUDE_CONFIG_DIR/.credentials.json`, else `~/.claude/.credentials.json`.
/// (On macOS the CLI may use the keychain instead; absence of the file there
/// just means this signal stays quiet — the PTY-based signals still work.)
fn credentials_path() -> Option<std::path::PathBuf> {
    if let Some(dir) = std::env::var_os("CLAUDE_CONFIG_DIR") {
        return Some(std::path::PathBuf::from(dir).join(".credentials.json"));
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(std::path::PathBuf::from(home).join(".claude/.credentials.json"))
}

fn mtime(path: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

impl LoginFlow {
    /// Spawn `claude` from `PATH` and start the given login flow.
    pub fn start(mode: LoginMode) -> Result<Self> {
        Self::start_with_binary("claude", mode)
    }

    /// [`start`](Self::start) with a specific CLI binary path.
    pub fn start_with_binary(binary: &str, mode: LoginMode) -> Result<Self> {
        // Baseline the credentials store before the CLI can touch it.
        let creds = CredsWatch::snapshot();
        let pty = native_pty_system();
        let pair = pty
            .openpty(PtySize {
                rows: 40,
                // Very wide on purpose: display-wrapping is what breaks text
                // extraction (URLs, minted tokens), so make it impossible at
                // the source rather than reassembling wrapped output later.
                cols: 1000,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| Error::Unknown(format!("openpty failed: {e}")))?;

        let mut cmd = CommandBuilder::new(binary);
        cmd.args(mode.args());
        // The crate IS the terminal on the other side of this PTY (it parses
        // OSC 8/52, strips ANSI, and speaks bracketed paste), so advertise a
        // deterministic capability surface instead of inheriting whatever
        // TERM the host process happens to have (server processes often have
        // none). Measured on CLI 2.1.220: submission works under TERM=dumb
        // and TERM-unset too — this is eliminating an environment axis, not
        // fixing a reproduced failure.
        cmd.env("TERM", "xterm-256color");
        // Same move for nested-session and credential detection: the session
        // spawn path (cli.rs) deliberately scrubs CLAUDECODE; a login flow
        // additionally must not inherit the host's Anthropic credentials —
        // its entire purpose is to mint fresh ones. Measured on 2.1.220 that
        // none of these break submission when present, but a login child has
        // no legitimate use for any of them, so remove the axis outright.
        for var in [
            "CLAUDECODE",
            "CLAUDE_CODE_ENTRYPOINT",
            "CLAUDE_CODE_CHILD_SESSION",
            "CLAUDE_CODE_SESSION_ID",
            "CLAUDE_CODE_OAUTH_TOKEN",
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_AUTH_TOKEN",
        ] {
            cmd.env_remove(var);
        }
        if let Ok(cwd) = std::env::current_dir() {
            cmd.cwd(cwd);
        }
        let child = pair.slave.spawn_command(cmd).map_err(|e| {
            let msg = e.to_string();
            if msg.contains("No such file") || msg.contains("not found") {
                Error::BinaryNotFound {
                    name: binary.to_string(),
                }
            } else {
                Error::Unknown(format!("spawn {binary} failed: {msg}"))
            }
        })?;
        let killer = child.clone_killer();

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| Error::Unknown(format!("pty writer: {e}")))?;
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| Error::Unknown(format!("pty reader: {e}")))?;

        let buf: OutBuf = Arc::new((Mutex::new((Vec::new(), false)), Condvar::new()));
        let buf_writer = Arc::clone(&buf);
        std::thread::spawn(move || {
            let mut chunk = [0u8; 4096];
            loop {
                match reader.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let (lock, cv) = &*buf_writer;
                        lock.lock().unwrap().0.extend_from_slice(&chunk[..n]);
                        cv.notify_all();
                    }
                }
            }
            let (lock, cv) = &*buf_writer;
            lock.lock().unwrap().1 = true;
            cv.notify_all();
        });

        // Drop the slave NOW: the child owns its own copies of the PTY fds,
        // and holding ours would suppress master-side EOF on child death.
        let portable_pty::PtyPair { master, slave } = pair;
        drop(slave);

        Ok(Self {
            child,
            killer,
            writer,
            _master: master,
            buf,
            mode,
            finished: false,
            creds,
        })
    }

    /// Block until the CLI prints the OAuth authorize URL, and return it.
    ///
    /// The URL is lifted from the OSC 8 hyperlink the CLI emits (falling back
    /// to a plain-text scan), so it is exact even when the terminal rendering
    /// wraps it. Show it to your user; they sign in there and receive a code
    /// to bring back to [`submit_code`](Self::submit_code).
    pub fn auth_url(&mut self, timeout: Duration) -> Result<String> {
        let deadline = Instant::now() + timeout;
        let (lock, cv) = &*self.buf;
        let mut guard = lock.lock().unwrap();
        loop {
            if let Some(url) = extract_auth_url(&guard.0) {
                return Ok(url);
            }
            if guard.1 {
                let transcript = strip_ansi(&guard.0);
                self.finished = true;
                let code = reap_exit_code(&mut self.child);
                return Err(Error::LoginChildExited {
                    code,
                    transcript: format!(
                        "[child=exited({code:?}) before printing an authorize URL]\n{transcript}"
                    ),
                });
            }
            let now = Instant::now();
            if now >= deadline {
                return Err(Error::Timeout);
            }
            let (g, _) = cv.wait_timeout(guard, deadline - now).unwrap();
            guard = g;
        }
    }

    /// Paste the authorization code back into the flow.
    ///
    /// The code is written wrapped in **bracketed-paste framing**
    /// (`ESC[200~ … ESC[201~`), then — after a short beat — a single
    /// carriage return (`0x0D`, the Enter keycode; LF does not submit) as
    /// its own write.
    ///
    /// Both parts are load-bearing. The TUI classifies any single write of
    /// **≥ 64 bytes** as a paste and absorbs a trailing CR into the paste
    /// payload instead of treating it as Enter — measured live on CLI
    /// 2.1.220: a 62-char code + CR (63 bytes) submits, a 63-char code + CR
    /// (64 bytes) sits silently at the prompt forever. Every real
    /// authorization code (~90+ chars) is over the threshold, so an unframed
    /// single-chunk write can never submit in production. The CLI enables
    /// bracketed paste (`ESC[?2004h`), so explicit framing is the paste path
    /// it is actually expecting; the separated, delayed CR then lands
    /// outside the paste boundary as a genuine keypress.
    ///
    /// A code that is empty after trimming is rejected here: pressing Enter
    /// on an empty field produces no detectable outcome, only a silent hang.
    pub fn submit_code(&mut self, code: &str) -> Result<()> {
        use std::io::Write;
        self.writer.write_all(&prepare_code_paste(code)?)?;
        self.writer.flush()?;
        // REDUNDANT ON PURPOSE — do not "optimise away". The framing alone
        // is sufficient (verified in-container: framed burst + CR in the
        // SAME write submits at 92 bytes), and the delayed lone CR is
        // sufficient alone too. Keeping both means a future change must
        // break two independent mechanisms to reintroduce the 64-byte
        // swallow, and either can quietly save us if the CLI's paste
        // handling shifts.
        std::thread::sleep(SUBMIT_ENTER_DELAY);
        self.writer.write_all(b"\r")?;
        self.writer.flush()?;
        Ok(())
    }

    /// Submit the authorization code and wait for a definitive outcome by
    /// watching the CLI's output — not its exit. The CLI does not exit on a
    /// bad code (it prints an OAuth error and waits for a retry), and its
    /// prompts render with cursor-column escapes instead of spaces, so text
    /// gating on human-readable phrases hangs forever.
    ///
    /// Success is detected via a three-source ladder, because the TUI cannot
    /// be trusted to *display* the secret (it masks input, positions words
    /// with cursor-column escapes, and may soft-wrap):
    ///
    /// 1. **Screen** — the token rendered in visible text.
    /// 2. **OSC 52** — the token inside a clipboard-copy escape's base64
    ///    payload, read from RAW bytes (exact, unmaskable).
    /// 3. **Credentials file** — `.credentials.json` created/updated after
    ///    submission: authoritative success even when no token is observable
    ///    on the PTY (a short grace period allows a late token to surface).
    ///
    /// Outcomes:
    /// - Token found → `Ok`, with [`LoginOutcome::token_source`] naming the
    ///   channel; the child is left for the caller to drop (which kills it).
    /// - Credentials updated but no token observable → `Ok` with
    ///   `token: None`, `credentials_updated: true`.
    /// - Rejection printed after this submission (`OAuth error`, or the
    ///   stable `Press Enter to retry` prompt under any wording) →
    ///   [`Error::CodeRejected`]. The flow stays **alive** — the same session
    ///   (and its PKCE verifier) accepts a corrected code via another
    ///   `submit_code_and_wait` call.
    /// - Child exit → outcome from the transcript, as [`finish`](Self::finish).
    /// - Timeout → [`Error::LoginTimeout`] carrying a per-channel status line
    ///   and the post-submission screen content, so a silent failure names
    ///   the blind channel instead of guessing.
    pub fn submit_code_and_wait(&mut self, code: &str, timeout: Duration) -> Result<LoginOutcome> {
        let offset = {
            let (lock, _) = &*self.buf;
            let len = lock.lock().unwrap().0.len();
            len
        };
        // Pre-submit liveness check: timestamps a death BEFORE the paste
        // against one at/after it — the discriminator between "the frame
        // killed it" and "it was already gone when we wrote".
        if let Ok(Some(status)) = self.child.try_wait() {
            self.finished = true;
            let transcript = {
                let (lock, _) = &*self.buf;
                strip_ansi(&lock.lock().unwrap().0)
            };
            return Err(Error::LoginChildExited {
                code: Some(status.exit_code()),
                transcript: format!(
                    "[child=exited({}) BEFORE code submission — nothing was written]\n{transcript}",
                    status.exit_code()
                ),
            });
        }
        self.submit_code(code)?;

        let deadline = Instant::now() + timeout;
        // First moment the credentials file was seen created/updated; token
        // extraction gets this long afterwards to surface before we accept
        // success-without-token.
        let mut creds_seen_at: Option<Instant> = None;
        let mut nudged = false;
        // (exit_code, first seen) — set the moment try_wait reports death.
        let mut child_exit: Option<(u32, Instant)> = None;
        const CREDS_TOKEN_GRACE: Duration = Duration::from_secs(3);

        let (lock, cv) = &*self.buf;
        let mut guard = lock.lock().unwrap();
        loop {
            let stripped = strip_ansi(&guard.0);
            let osc52 = extract_osc52_token(&guard.0);
            let creds_updated = creds_seen_at.is_some() || self.creds.updated();

            if let Some(token) = extract_token(&stripped) {
                if token_wrap_suspect(&stripped) {
                    // A silently truncated token would fail far from here at
                    // first use; fail loudly instead — unless the exact bytes
                    // are recoverable from the clipboard channel.
                    if let Osc52Scan::Token(token) = osc52 {
                        return Ok(LoginOutcome {
                            token: Some(token),
                            token_source: Some(TokenSource::Osc52),
                            credentials_updated: creds_updated,
                            osc52: Osc52Status::TokenRecovered,
                            copy_nudge_sent: nudged,
                            transcript: stripped,
                        });
                    }
                    self.finished = true;
                    let _ = self.killer.kill();
                    return Err(Error::Protocol(
                        "minted token appears display-wrapped in PTY output; cannot extract reliably"
                            .to_string(),
                    ));
                }
                return Ok(LoginOutcome {
                    token: Some(token),
                    token_source: Some(TokenSource::Screen),
                    credentials_updated: creds_updated,
                    osc52: Osc52Status::from(&osc52),
                    copy_nudge_sent: nudged,
                    transcript: stripped,
                });
            }

            if let Osc52Scan::Token(token) = &osc52 {
                return Ok(LoginOutcome {
                    token: Some(token.clone()),
                    token_source: Some(TokenSource::Osc52),
                    credentials_updated: creds_updated,
                    osc52: Osc52Status::TokenRecovered,
                    copy_nudge_sent: nudged,
                    transcript: stripped,
                });
            }

            // Only output produced after THIS submission counts as its error;
            // a previous attempt's error text must not fail a retry.
            let tail = strip_ansi(&guard.0[offset.min(guard.0.len())..]);
            if let Some(message) = detect_oauth_error(&tail) {
                return Err(Error::CodeRejected { message });
            }

            // Credentials write = accepted, even with nothing on screen.
            if creds_updated {
                let first_detection = creds_seen_at.is_none();
                let seen = *creds_seen_at.get_or_insert_with(Instant::now);
                if first_detection {
                    // Success is confirmed, so nudge the TUI's copy
                    // affordance ("c to copy") once: if the success screen
                    // supports it, the CLI emits the token over OSC 52 —
                    // exact bytes — before the grace window closes. Best
                    // effort: on screens without the affordance this is a
                    // stray keypress in an already-succeeded flow. Guarded:
                    // reaching this branch means the post-submission tail
                    // carried no rejection anchor this iteration (that check
                    // returns above), so this is not a retry prompt.
                    use std::io::Write;
                    nudged = self
                        .writer
                        .write_all(b"c")
                        .and_then(|()| self.writer.flush())
                        .is_ok();
                }
                if seen.elapsed() >= CREDS_TOKEN_GRACE {
                    return Ok(LoginOutcome {
                        token: None,
                        token_source: None,
                        credentials_updated: true,
                        osc52: Osc52Status::from(&osc52),
                        copy_nudge_sent: nudged,
                        transcript: stripped,
                    });
                }
            }

            // Child death is a first-class outcome, detected two ways:
            // reader EOF (the slave is dropped at spawn, so the master EOFs
            // the moment the child's side closes) and a direct try_wait
            // (catches a dead parent whose PTY is still held open by an
            // orphaned grandchild). Production attempt four: the child died
            // ~5s after submission and the flow spent 85s polling a corpse,
            // reporting benign absence on every channel.
            if child_exit.is_none() {
                if let Ok(Some(status)) = self.child.try_wait() {
                    child_exit = Some((status.exit_code(), Instant::now()));
                }
            }
            let exit_drained = child_exit
                .map(|(_, at)| at.elapsed() > Duration::from_secs(1))
                .unwrap_or(false);
            if guard.1 || exit_drained {
                let transcript = strip_ansi(&guard.0);
                self.finished = true;
                // The buffer is complete here and the token / OSC 52 /
                // rejection checks above already ran on it this iteration,
                // so the only success still possible is a credentials write
                // racing the exit.
                if creds_updated || self.creds.updated() {
                    return Ok(LoginOutcome {
                        token: None,
                        token_source: None,
                        credentials_updated: true,
                        osc52: Osc52Status::from(&osc52),
                        copy_nudge_sent: nudged,
                        transcript,
                    });
                }
                let code = child_exit
                    .map(|(c, _)| c)
                    .or_else(|| reap_exit_code(&mut self.child));
                let tail = strip_ansi(&guard.0[offset.min(guard.0.len())..]);
                return Err(Error::LoginChildExited {
                    code,
                    transcript: format!(
                        "[channels: screen=no-token osc52={:?} credentials=unchanged copy-nudge={} submit-path={SUBMIT_PATH} child=exited({code:?})]\n{tail}",
                        Osc52Status::from(&osc52),
                        if nudged { "sent" } else { "not-sent" },
                    ),
                });
            }

            let now = Instant::now();
            if now >= deadline {
                // Carry what was actually observable on every channel — a
                // bare timeout renders absence of evidence as evidence of
                // absence, and cannot distinguish "code never landed" from
                // "outcome appeared where nothing was looking".
                let tail = strip_ansi(&guard.0[offset.min(guard.0.len())..]);
                let mut start = tail.len().saturating_sub(2000);
                while !tail.is_char_boundary(start) {
                    start += 1;
                }
                return Err(Error::LoginTimeout {
                    transcript: format!(
                        "[channels: screen=no-token osc52={:?} credentials={} copy-nudge={} submit-path={SUBMIT_PATH} child=alive]\n{}",
                        Osc52Status::from(&osc52),
                        if creds_updated {
                            "updated"
                        } else {
                            "unchanged"
                        },
                        if nudged { "sent" } else { "not-sent" },
                        &tail[start..]
                    ),
                });
            }
            let wait = (deadline - now).min(Duration::from_millis(200));
            let (g, _) = cv.wait_timeout(guard, wait).unwrap();
            guard = g;
        }
    }

    /// Wait for the flow to complete and collect the outcome.
    ///
    /// For [`LoginMode::SetupToken`] the minted token is extracted from the
    /// output; a missing token is an error carrying the CLI's transcript.
    /// For the login modes, verify success with [`auth_status`] — the CLI
    /// persists credentials itself and its exit text varies across versions.
    pub fn finish(mut self, timeout: Duration) -> Result<LoginOutcome> {
        let deadline = Instant::now() + timeout;
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => {
                    if Instant::now() >= deadline {
                        let _ = self.killer.kill();
                        self.finished = true;
                        return Err(Error::Timeout);
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => {
                    self.finished = true;
                    return Err(Error::Unknown(format!("wait on login child: {e}")));
                }
            }
        }
        self.finished = true;

        // Give the reader thread a beat to drain the last PTY bytes.
        let (lock, cv) = &*self.buf;
        let mut guard = lock.lock().unwrap();
        let drain_deadline = Instant::now() + Duration::from_secs(2);
        while !guard.1 && Instant::now() < drain_deadline {
            let (g, _) = cv.wait_timeout(guard, Duration::from_millis(100)).unwrap();
            guard = g;
        }
        let transcript = strip_ansi(&guard.0);
        let osc52 = extract_osc52_token(&guard.0);
        drop(guard);

        let credentials_updated = self.creds.updated();
        let osc52_status = Osc52Status::from(&osc52);
        let (token, token_source) = match (extract_token(&transcript), osc52) {
            (Some(t), _) => (Some(t), Some(TokenSource::Screen)),
            (None, Osc52Scan::Token(t)) => (Some(t), Some(TokenSource::Osc52)),
            (None, _) => (None, None),
        };
        if self.mode == LoginMode::SetupToken && token.is_none() && !credentials_updated {
            return Err(Error::Unknown(format!(
                "setup-token completed without minting a token; output:\n{transcript}"
            )));
        }
        Ok(LoginOutcome {
            token,
            token_source,
            credentials_updated,
            osc52: osc52_status,
            copy_nudge_sent: false,
            transcript,
        })
    }
}

impl Drop for LoginFlow {
    fn drop(&mut self) {
        if !self.finished {
            // Make a self-inflicted kill visible: if a host application
            // drops the flow mid-submission (moved-out state not put back,
            // early return, TTL reaper), the child's death must be
            // attributable to THIS line rather than presenting as a
            // mysterious CLI crash. portable-pty's kill sends SIGTERM, so
            // the child reports exit code 143 — same as an external
            // SIGTERM; this log line is the disambiguator.
            #[cfg(feature = "log")]
            log::warn!("LoginFlow dropped while unfinished — killing login child (SIGTERM)");
            let _ = self.killer.kill();
        }
    }
}

/// Find the OAuth authorize URL in raw PTY output.
///
/// Preferred source is the OSC 8 hyperlink (`ESC ] 8 ; params ; URI ST`),
/// whose URI is never display-wrapped; the fallback scans ANSI-stripped text
/// for a bare `https://…oauth/authorize…` run.
fn extract_auth_url(raw: &[u8]) -> Option<String> {
    let hay = String::from_utf8_lossy(raw);
    let mut rest: &str = &hay;
    while let Some(start) = rest.find("\x1b]8;") {
        let after = &rest[start + 4..];
        if let Some(sep) = after.find(';') {
            let uri = &after[sep + 1..];
            let end = uri.find(['\x1b', '\x07']).unwrap_or(uri.len());
            let uri = &uri[..end];
            if uri.contains("oauth/authorize") {
                return Some(uri.to_string());
            }
            rest = &after[sep + 1..];
        } else {
            break;
        }
    }
    let text = strip_ansi(raw);
    let start = text.find("https://")?;
    let url: String = text[start..]
        .chars()
        .take_while(|c| !c.is_whitespace() && *c != '"' && *c != ')')
        .collect();
    url.contains("oauth/authorize").then_some(url)
}

/// Whitespace-insensitive, case-insensitive form for matching the CLI's TUI
/// text: its renderer positions words with cursor-column escapes instead of
/// spaces, so ANSI-stripped output runs words together
/// (`Pastecodehereifprompted>`). Presence checks must collapse whitespace on
/// both sides; boundary extraction (tokens, URLs) must NOT use this form,
/// because adjacent text would glue onto the match.
fn collapsed_lower(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Outcome of scanning raw PTY bytes for an OSC 52 clipboard-copy token —
/// granular so failures name the blind channel instead of silently falling
/// through to the next source.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Osc52Scan {
    /// No OSC 52 sequence in the stream (the CLI may simply not auto-copy).
    Absent,
    /// A sequence has started but its terminator hasn't arrived yet —
    /// decoding now would truncate the secret; wait for more bytes.
    Unterminated,
    /// Terminated sequence whose payload isn't valid base64.
    Undecodable,
    /// Valid payload(s), none containing a token (e.g. the URL copy).
    NoTokenInPayload,
    Token(String),
}

/// Scan raw PTY bytes for OSC 52 sequences (`ESC ] 52 ; c ; BASE64`,
/// terminated by BEL or ST) and extract a minted token from their payloads.
///
/// Runs on RAW bytes deliberately: ANSI stripping discards OSC content,
/// which is exactly where the clipboard payload lives. Only terminated
/// sequences are decoded — a payload still straddling read boundaries
/// reports [`Osc52Scan::Unterminated`] rather than decoding a truncated
/// prefix into a corrupt secret.
fn extract_osc52_token(raw: &[u8]) -> Osc52Scan {
    use base64::Engine as _;
    let hay = String::from_utf8_lossy(raw);
    let mut best = Osc52Scan::Absent;
    let mut rest: &str = &hay;
    while let Some(start) = rest.find("\x1b]52;") {
        let body = &rest[start + 5..];
        // Payload begins after the selection-parameter field (e.g. `c;`).
        let Some(sep) = body.find(';') else {
            return Osc52Scan::Unterminated;
        };
        let payload_and_more = &body[sep + 1..];
        let Some(end) = payload_and_more.find(['\x07', '\x1b']) else {
            return Osc52Scan::Unterminated;
        };
        let payload = payload_and_more[..end].trim();
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(payload)
            .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(payload));
        match decoded {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                if let Some(token) = extract_token(&text) {
                    return Osc52Scan::Token(token);
                }
                best = Osc52Scan::NoTokenInPayload;
            }
            Err(_) => {
                if best == Osc52Scan::Absent {
                    best = Osc52Scan::Undecodable;
                }
            }
        }
        rest = &payload_and_more[end..];
    }
    best
}

/// Identifies the code-submission write path compiled into this build.
/// Recorded in the [`Error::LoginTimeout`] channel line (and available to
/// downstream logs), so "which write path is actually deployed" is readable
/// from a single log line instead of requiring binary forensics — release
/// builds inline the frame bytes into immediates, so byte-grepping a binary
/// for `ESC[200~` proves nothing in either direction.
pub const SUBMIT_PATH: &str =
    "bracketed-paste+lone-cr-150ms+term-forced+env-scrubbed+exit-aware/v5";

/// Pause between the paste frame and the Enter keypress in
/// [`LoginFlow::submit_code`].
const SUBMIT_ENTER_DELAY: Duration = Duration::from_millis(150);

/// Best-effort exit-code reap after EOF: the child's death is imminent or
/// already happened, but `wait()` could block forever if a grandchild holds
/// the PTY while the parent lingers — poll briefly instead.
fn reap_exit_code(child: &mut Box<dyn portable_pty::Child + Send + Sync>) -> Option<u32> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status.exit_code()),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(50));
            }
            _ => return None,
        }
    }
}

/// The paste frame a code submission writes to the PTY: the trimmed code
/// wrapped in bracketed-paste markers (`ESC[200~ … ESC[201~`), NO trailing
/// CR — Enter is sent separately after [`SUBMIT_ENTER_DELAY`]. See
/// [`LoginFlow::submit_code`] for why: single writes of ≥ 64 bytes are
/// classified as pastes and a trailing CR inside the burst is swallowed, so
/// the frame declares the paste explicitly and the keypress travels alone.
/// An empty-after-trim code is refused: pressing Enter on an empty field
/// produces no detectable outcome, only a silent hang.
fn prepare_code_paste(code: &str) -> Result<Vec<u8>> {
    let code = code.trim();
    if code.is_empty() {
        return Err(Error::Protocol(
            "authorization code is empty after trimming; refusing to submit".to_string(),
        ));
    }
    let mut buf = Vec::with_capacity(code.len() + 12);
    buf.extend_from_slice(b"\x1b[200~");
    buf.extend_from_slice(code.as_bytes());
    buf.extend_from_slice(b"\x1b[201~");
    Ok(buf)
}

/// Detect a rejected submission and return the CLI's message text
/// (single-line, truncated). The CLI does not exit on this — it waits for a
/// retry — so callers must treat detection as the failure signal.
///
/// Primary anchor is `OAuth error`; fallback is the retry prompt
/// (`Press Enter to retry`), the retry loop's one structural constant. The
/// error wording that precedes it is unstable — the TUI positions words with
/// cursor-column escapes, so observed stripped output runs words together
/// and even drops characters (`Requstfailed withstatus code 400` was
/// captured live) — but every rejection parks on that prompt.
fn detect_oauth_error(stripped: &str) -> Option<String> {
    let collapsed = collapsed_lower(stripped);
    let anchor = if collapsed.contains("oautherror") {
        "oauth"
    } else if collapsed.contains("pressentertoretry") {
        // Report the whole post-submission tail so the message includes the
        // error text preceding the prompt, whatever its wording.
        ""
    } else {
        return None;
    };
    let start = if anchor.is_empty() {
        0
    } else {
        stripped.to_lowercase().find(anchor).unwrap_or(0)
    };
    let message: String = stripped[start..]
        .chars()
        .map(|c| if c == '\n' { ' ' } else { c })
        .take(240)
        .collect();
    Some(message.trim().to_string())
}

/// True when a found token ends exactly at a line break and token-charset
/// characters continue on the next line — the signature of display-wrapping,
/// which would silently truncate the extracted token.
fn token_wrap_suspect(text: &str) -> bool {
    let Some(start) = text.find("sk-ant-oat01-") else {
        return false;
    };
    let rest = &text[start..];
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .unwrap_or(rest.len());
    let after = &rest[end..];
    let mut chars = after.chars();
    matches!(
        (chars.next(), chars.next()),
        (Some('\n'), Some(c)) if c.is_ascii_alphanumeric() || c == '-' || c == '_'
    )
}

/// Pull a long-lived OAuth token (`sk-ant-oat01-…`) out of flow output.
fn extract_token(text: &str) -> Option<String> {
    let start = text.find("sk-ant-oat01-")?;
    let token: String = text[start..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    Some(token)
}

/// Remove ANSI escape sequences (CSI, OSC, and single-char escapes) so
/// transcripts are readable and scannable.
fn strip_ansi(raw: &[u8]) -> String {
    let s = String::from_utf8_lossy(raw);
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            if c != '\r' {
                out.push(c);
            }
            continue;
        }
        match chars.next() {
            // CSI: ESC [ … final byte in @–~
            Some('[') => {
                for f in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&f) {
                        break;
                    }
                }
            }
            // OSC: ESC ] … terminated by BEL or ST (ESC \)
            Some(']') => {
                let mut prev = '\0';
                for f in chars.by_ref() {
                    if f == '\x07' || (prev == '\x1b' && f == '\\') {
                        break;
                    }
                    prev = f;
                }
            }
            // Two-char escapes (ESC ( B etc.): drop one following char.
            Some('(') | Some(')') => {
                chars.next();
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Abbreviated real `claude setup-token` PTY capture: spinner CSI noise,
    /// then the URL inside an OSC 8 hyperlink whose display text is wrapped.
    const CAPTURE: &str = "\x1b[38;5;153m\u{2733}\x1b[39m Opening browser to sign in\u{2026}\
        \x1b]8;id=wq6tp2;https://claude.com/cai/oauth/authorize?code=true&client_id=9d1c250a&state=o8LKRdriZ\x1b\\\
        https://claude.com/cai/oauth/aut\nhorize?code=true&client_id=9d1c\x1b]8;;\x1b\\";

    #[test]
    fn url_lifted_from_osc8_hyperlink_unwrapped() {
        let url = extract_auth_url(CAPTURE.as_bytes()).expect("url found");
        assert_eq!(
            url,
            "https://claude.com/cai/oauth/authorize?code=true&client_id=9d1c250a&state=o8LKRdriZ"
        );
    }

    #[test]
    fn url_fallback_from_plain_text() {
        let plain = b"Browser didn't open? Use the url below to sign in:\n\
            https://console.anthropic.com/oauth/authorize?code=true&state=abc\n";
        let url = extract_auth_url(plain).expect("url found");
        assert_eq!(
            url,
            "https://console.anthropic.com/oauth/authorize?code=true&state=abc"
        );
    }

    #[test]
    fn no_url_in_spinner_noise() {
        assert_eq!(
            extract_auth_url(b"\x1b[2K\x1b[1G\xe2\x9c\xa2 waiting"),
            None
        );
    }

    #[test]
    fn token_extracted_from_transcript() {
        let t = "Success! Your token:\n  sk-ant-oat01-Ab3_x-Y9\nKeep it secret.";
        assert_eq!(extract_token(t).as_deref(), Some("sk-ant-oat01-Ab3_x-Y9"));
    }

    #[test]
    fn ansi_stripping_keeps_text() {
        let raw = b"\x1b[1mBold\x1b[0m and \x1b]8;;https://x\x1b\\link\x1b]8;;\x1b\\ text";
        assert_eq!(strip_ansi(raw), "Bold and link text");
    }

    #[test]
    fn auth_status_parses_cli_shape() {
        let json = r#"{
            "loggedIn": true, "authMethod": "claude.ai", "apiProvider": "firstParty",
            "email": "m@x.io", "orgId": "9185", "orgName": "org", "subscriptionType": "max"
        }"#;
        let s: AuthStatus = serde_json::from_str(json).unwrap();
        assert!(s.logged_in);
        assert_eq!(s.auth_method.as_deref(), Some("claude.ai"));
        assert_eq!(s.subscription_type.as_deref(), Some("max"));
        assert!(s.extra.is_empty());
    }

    #[test]
    fn auth_status_logged_out_minimal() {
        let s: AuthStatus = serde_json::from_str(r#"{"loggedIn": false}"#).unwrap();
        assert!(!s.logged_in);
        assert_eq!(s.auth_method, None);
    }

    /// Infra-captured literal bytes: the CLI positions words with
    /// cursor-column escapes (CSI n G) instead of spaces.
    #[test]
    fn column_escape_prompt_strips_to_run_together_text() {
        let raw = b"Paste\x1b[8Gcode\x1b[13Ghere\x1b[18Gif\x1b[21Gprompted\x1b[30G>";
        let stripped = strip_ansi(raw);
        assert_eq!(stripped, "Pastecodehereifprompted>");
        // Word-boundary matching cannot work; collapsed matching does.
        assert!(!stripped.contains("Paste code here"));
        assert!(collapsed_lower(&stripped).contains("pastecodehere"));
    }

    #[test]
    fn oauth_error_detected_despite_run_together_rendering() {
        let stripped =
            "OAuth error: Invalidcode. Please makesure the fullcde wascopied\nPress Enter to retry.";
        let msg = detect_oauth_error(stripped).expect("detected");
        assert!(msg.starts_with("OAuth error"));
        assert!(msg.contains("Press Enter to retry"));
        assert!(detect_oauth_error("Welcometo Claude Codev2.1.220").is_none());
    }

    #[test]
    fn wrapped_token_is_flagged_not_truncated() {
        assert!(token_wrap_suspect("token: sk-ant-oat01-abc\ndef more"));
        // Ends at newline but next line is prose punctuation-first: not a wrap
        assert!(!token_wrap_suspect("token: sk-ant-oat01-abcdef\n(copied)"));
        // No newline at boundary: clean extraction
        assert!(!token_wrap_suspect("token: sk-ant-oat01-abcdef done"));
        assert_eq!(
            extract_token("token: sk-ant-oat01-abcdef done").as_deref(),
            Some("sk-ant-oat01-abcdef")
        );
    }

    /// Live-captured failure wording with characters dropped by the
    /// renderer — no "OAuth error" literal survives, but the retry prompt
    /// does. The fallback anchor must fire.
    #[test]
    fn rejection_detected_via_retry_prompt_when_wording_mangled() {
        let stripped = "Requstfailed withstatus code 400PressEntertoretry.";
        let msg = detect_oauth_error(stripped).expect("retry prompt anchors detection");
        assert!(msg.contains("400"));
        // Prompt text alone in a NON-error screen must not false-positive:
        // the paste prompt says "if prompted", not "to retry".
        assert!(detect_oauth_error("Pastecodehereifprompted>").is_none());
    }

    #[test]
    fn osc52_token_decoded_from_raw_bel_and_st_terminated() {
        use base64::Engine as _;
        let b64 = base64::engine::general_purpose::STANDARD.encode("sk-ant-oat01-XyZ_9-ab");
        // BEL-terminated (observed live on the CLI's OSC 8 links).
        let bel = format!("noise\x1b]52;c;{b64}\x07more");
        assert_eq!(
            extract_osc52_token(bel.as_bytes()),
            Osc52Scan::Token("sk-ant-oat01-XyZ_9-ab".into())
        );
        // ST-terminated.
        let st = format!("\x1b]52;c;{b64}\x1b\\");
        assert_eq!(
            extract_osc52_token(st.as_bytes()),
            Osc52Scan::Token("sk-ant-oat01-XyZ_9-ab".into())
        );
    }

    #[test]
    fn osc52_straddling_read_boundary_is_not_decoded_truncated() {
        use base64::Engine as _;
        let b64 =
            base64::engine::general_purpose::STANDARD.encode("sk-ant-oat01-full-secret-value");
        let full = format!("\x1b]52;c;{b64}\x07");
        // Cut mid-payload: must report Unterminated, never a partial token.
        let partial = &full.as_bytes()[..full.len() - 8];
        assert_eq!(extract_osc52_token(partial), Osc52Scan::Unterminated);
        // ANSI stripping discards the payload entirely — raw-bytes scanning
        // is load-bearing, not an optimization.
        assert!(!strip_ansi(full.as_bytes()).contains("sk-ant"));
    }

    #[test]
    fn osc52_url_copy_is_no_token_and_garbage_is_undecodable() {
        use base64::Engine as _;
        let url_b64 = base64::engine::general_purpose::STANDARD
            .encode("https://claude.com/cai/oauth/authorize?code=true");
        let stream = format!("\x1b]52;c;{url_b64}\x07");
        assert_eq!(
            extract_osc52_token(stream.as_bytes()),
            Osc52Scan::NoTokenInPayload
        );
        assert_eq!(
            extract_osc52_token(b"\x1b]52;c;!!!not-base64!!!\x07"),
            Osc52Scan::Undecodable
        );
        assert_eq!(extract_osc52_token(b"plain output"), Osc52Scan::Absent);
    }

    #[test]
    fn credentials_watch_detects_create_and_update() {
        let dir = std::env::temp_dir().join(format!("creds-watch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(".credentials.json");
        let _ = std::fs::remove_file(&file);

        // Baseline: absent → creation counts as update.
        let watch = CredsWatch {
            path: Some(file.clone()),
            baseline: None,
        };
        assert!(!watch.updated());
        std::fs::write(&file, "{}").unwrap();
        assert!(watch.updated());

        // Baseline: present → only a NEWER mtime counts.
        let watch = CredsWatch {
            path: Some(file.clone()),
            baseline: mtime(&file),
        };
        assert!(!watch.updated());
        let newer = std::time::SystemTime::now() + Duration::from_secs(5);
        let f = std::fs::File::options().write(true).open(&file).unwrap();
        f.set_modified(newer).unwrap();
        drop(f);
        assert!(watch.updated());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn code_paste_is_bracketed_trimmed_and_cr_free() {
        // Fixtures at PRODUCTION lengths. The 64-byte paste-classification
        // bug was invisible to every sub-64-byte fixture; codes under 64
        // bytes are a different input class from real ones (~92 observed,
        // 108 = credentials-token length).
        for len in [64usize, 92, 108, 120] {
            let code: String = "x".repeat(len - 6) + "#state";
            let frame = prepare_code_paste(&format!("{code}\n")).unwrap();
            let mut expected = b"\x1b[200~".to_vec();
            expected.extend_from_slice(code.as_bytes());
            expected.extend_from_slice(b"\x1b[201~");
            assert_eq!(frame, expected, "len {len}");
            // The frame itself must never carry the Enter keypress: a CR
            // inside a ≥64-byte burst is swallowed as paste payload.
            assert!(!frame.contains(&b'\r'), "len {len}: CR must travel alone");
        }
        assert_eq!(
            prepare_code_paste(" abc \r\n").unwrap(),
            b"\x1b[200~abc\x1b[201~"
        );
        for empty in ["", "  ", "\n", "\r\n", "\t"] {
            assert!(
                prepare_code_paste(empty).is_err(),
                "empty guard must fire for {empty:?}"
            );
        }
    }
}
