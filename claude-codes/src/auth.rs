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

/// Result of a completed [`LoginFlow`].
#[derive(Debug, Clone)]
pub struct LoginOutcome {
    /// The minted long-lived token (`SetupToken` mode only).
    pub token: Option<String>,
    /// The flow's full terminal output with ANSI escapes stripped — for
    /// surfacing the CLI's own error text when something goes wrong.
    pub transcript: String,
}

/// Shared PTY output buffer: bytes so far + whether the reader hit EOF.
type OutBuf = Arc<(Mutex<(Vec<u8>, bool)>, Condvar)>;

/// An in-flight interactive login, driven over a pseudo-terminal.
pub struct LoginFlow {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    writer: Box<dyn std::io::Write + Send>,
    // Keeps the PTY master (and thus the reader) alive for the flow's life.
    _pair: portable_pty::PtyPair,
    buf: OutBuf,
    mode: LoginMode,
    finished: bool,
}

impl LoginFlow {
    /// Spawn `claude` from `PATH` and start the given login flow.
    pub fn start(mode: LoginMode) -> Result<Self> {
        Self::start_with_binary("claude", mode)
    }

    /// [`start`](Self::start) with a specific CLI binary path.
    pub fn start_with_binary(binary: &str, mode: LoginMode) -> Result<Self> {
        let pty = native_pty_system();
        let pair = pty
            .openpty(PtySize {
                rows: 40,
                // Wide enough that the authorize URL also survives unwrapped
                // in the visible text, not only inside the OSC 8 hyperlink.
                cols: 500,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| Error::Unknown(format!("openpty failed: {e}")))?;

        let mut cmd = CommandBuilder::new(binary);
        cmd.args(mode.args());
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

        Ok(Self {
            child,
            killer,
            writer,
            _pair: pair,
            buf,
            mode,
            finished: false,
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
                return Err(Error::Unknown(format!(
                    "login flow exited before printing an authorize URL; output:\n{transcript}"
                )));
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
    pub fn submit_code(&mut self, code: &str) -> Result<()> {
        use std::io::Write;
        self.writer.write_all(code.trim().as_bytes())?;
        self.writer.write_all(b"\r")?;
        self.writer.flush()?;
        Ok(())
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
        drop(guard);

        let token = extract_token(&transcript);
        if self.mode == LoginMode::SetupToken && token.is_none() {
            return Err(Error::Unknown(format!(
                "setup-token completed without minting a token; output:\n{transcript}"
            )));
        }
        Ok(LoginOutcome { token, transcript })
    }
}

impl Drop for LoginFlow {
    fn drop(&mut self) {
        if !self.finished {
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
}
