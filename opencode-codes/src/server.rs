//! Managed launcher for the local `opencode serve` process.
//!
//! [`ManagedServer`] spawns `opencode serve --hostname <host> --port <n>` on a
//! free port (chosen with [`portpicker`] unless overridden), waits until the
//! HTTP surface answers `GET /global/health`, exposes the bound base URL via
//! [`ManagedServer::url`], and tears the process down on drop.
//!
//! # Orphan cleanup
//!
//! `opencode serve` may itself spawn child processes (MCP servers, tool
//! subprocesses). On Unix the child is placed in its own process group
//! (`setpgid(0, 0)`) so that termination signals the entire tree rather than
//! just the top-level `opencode` process. On drop the group receives `SIGTERM`,
//! followed by `SIGKILL` after a short grace period, ensuring nothing is left
//! orphaned even if the parent aborts. [`tokio::process::Command::kill_on_drop`]
//! guarantees the direct child is reaped as a backstop on every platform.
//!
//! # Feature gating
//!
//! This module is compiled only when the `server` feature is enabled.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;

use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Child;
use tokio::process::Command;

use crate::error::Error;
use crate::error::Result;
use crate::http::DEFAULT_USERNAME;

/// Substring emitted on stdout once the server has bound its listener.
///
/// The full line is `opencode server listening on http://<host>:<port>`.
const LISTENING_MARKER: &str = "listening on ";

/// Grace period between `SIGTERM` and `SIGKILL` when terminating the group.
#[cfg(unix)]
const KILL_GRACE: Duration = Duration::from_millis(250);

#[cfg(unix)]
const SIGTERM: i32 = 15;

#[cfg(unix)]
const SIGKILL: i32 = 9;

/// Send `sig` to the entire process group identified by `pgid`.
///
/// Signalling the negated PGID targets every process in the group created via
/// `process_group(0)`. Best-effort: a missing group (already-exited process,
/// `ESRCH`) is the desired state, and there is nothing actionable to do on any
/// other failure during teardown.
#[cfg(unix)]
#[allow(unsafe_code)] // The workspace denies unsafe_code; this FFI signal call is the one vetted exception.
fn signal_process_group(pgid: i32, sig: i32) {
    extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }

    // SAFETY: `kill` takes two plain integers and borrows no memory across the
    // FFI boundary. A negative pid addresses the process group.
    unsafe {
        kill(-pgid, sig);
    }
}

/// Fluent builder for a [`ManagedServer`].
///
/// Obtain one via [`ManagedServer::builder`]. All fields are optional; the
/// defaults launch `opencode` from `PATH`, bind `127.0.0.1` on a
/// portpicker-selected port, and wait up to ten seconds for health.
#[derive(Debug, Clone)]
pub struct ManagedServerBuilder {
    binary: PathBuf,
    hostname: String,
    port: Option<u16>,
    working_dir: Option<PathBuf>,
    password: Option<String>,
    envs: HashMap<String, String>,
    startup_timeout: Duration,
}

impl Default for ManagedServerBuilder {
    fn default() -> Self {
        Self {
            binary: PathBuf::from("opencode"),
            hostname: "127.0.0.1".to_string(),
            port: None,
            working_dir: None,
            password: None,
            envs: HashMap::new(),
            startup_timeout: Duration::from_secs(10),
        }
    }
}

impl ManagedServerBuilder {
    /// Create a builder with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Override the `opencode` executable (path or `PATH`-resolved name).
    ///
    /// Defaults to `opencode`.
    #[must_use]
    pub fn binary(mut self, binary: impl Into<PathBuf>) -> Self {
        self.binary = binary.into();
        self
    }

    /// Set the hostname passed to `--hostname`. Defaults to `127.0.0.1`.
    #[must_use]
    pub fn hostname(mut self, hostname: impl Into<String>) -> Self {
        self.hostname = hostname.into();
        self
    }

    /// Pin the listen port passed to `--port`.
    ///
    /// When unset, a free port is chosen with [`portpicker`]. The port actually
    /// bound is read back from the server's stdout, so pinning is only needed
    /// when a caller must know the port in advance.
    #[must_use]
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Set the child's working directory.
    #[must_use]
    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Set `OPENCODE_SERVER_PASSWORD`, enabling HTTP basic auth on the server.
    ///
    /// The managed health check authenticates as
    /// [`DEFAULT_USERNAME`](crate::http::DEFAULT_USERNAME) (`opencode`) with this
    /// password. Consumers of [`ManagedServer::url`] must send the same
    /// credentials.
    #[must_use]
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Inject an environment variable into the child process.
    ///
    /// Applied on top of the inherited parent environment. Call repeatedly to
    /// set several. `OPENCODE_SERVER_PASSWORD` is managed by
    /// [`password`](Self::password); setting it here directly is also honored.
    #[must_use]
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.envs.insert(key.into(), value.into());
        self
    }

    /// Maximum time to wait for the server to become healthy. Defaults to 10s.
    #[must_use]
    pub fn startup_timeout(mut self, timeout: Duration) -> Self {
        self.startup_timeout = timeout;
        self
    }

    /// Spawn the server and wait until `GET /global/health` succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Server`] if the binary cannot be spawned, if it exits
    /// before reporting a listening address, or if it does not answer a healthy
    /// response within [`startup_timeout`](Self::startup_timeout).
    pub async fn spawn(self) -> Result<ManagedServer> {
        ManagedServer::spawn(self).await
    }
}

/// A running, self-terminating `opencode serve` process.
///
/// Construct one with [`ManagedServer::builder`]. The process is killed and
/// reaped when the value is dropped (see the [module docs](self) for the
/// process-group teardown strategy). Prefer [`ManagedServer::stop`] for an
/// awaited, deterministic shutdown.
#[derive(Debug)]
pub struct ManagedServer {
    child: Option<Child>,
    base_url: String,
    port: u16,
    password: Option<String>,
    /// Process-group id for whole-tree termination on Unix.
    #[cfg(unix)]
    pgid: Option<i32>,
}

impl ManagedServer {
    /// Start a builder with default settings.
    #[must_use]
    pub fn builder() -> ManagedServerBuilder {
        ManagedServerBuilder::new()
    }

    /// Spawn a server from the given builder configuration.
    ///
    /// # Errors
    ///
    /// See [`ManagedServerBuilder::spawn`].
    pub async fn spawn(builder: ManagedServerBuilder) -> Result<Self> {
        let port = match builder.port {
            Some(p) => p,
            None => portpicker::pick_unused_port()
                .ok_or_else(|| Error::Server("no free TCP port available".to_string()))?,
        };

        let mut cmd = Command::new(&builder.binary);
        cmd.arg("serve")
            .arg("--hostname")
            .arg(&builder.hostname)
            .arg("--port")
            .arg(port.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);

        #[cfg(unix)]
        cmd.process_group(0);

        if let Some(dir) = &builder.working_dir {
            cmd.current_dir(dir);
        }

        for (key, value) in &builder.envs {
            cmd.env(key, value);
        }

        if let Some(password) = &builder.password {
            cmd.env("OPENCODE_SERVER_PASSWORD", password);
        }

        crate::process::configure_no_window(cmd.as_std_mut());
        let mut child = cmd.spawn().map_err(|e| {
            Error::Server(format!(
                "failed to spawn `{}`: {e}",
                builder.binary.display()
            ))
        })?;

        let pgid = child.id().map(|id| id as i32);

        let stdout = child.stdout.take().ok_or_else(|| {
            Error::Server("managed server child produced no stdout handle".to_string())
        })?;

        let deadline = Instant::now() + builder.startup_timeout;
        let result = Self::await_listening(
            stdout,
            &builder.hostname,
            port,
            deadline,
            builder.startup_timeout,
        )
        .await;

        let (base_url, reader) = match result {
            Ok(pair) => pair,
            Err(e) => {
                signal_group_and_kill(pgid, &mut child);
                let _ = child.wait().await;
                return Err(e);
            }
        };

        // Drain any further stdout so the child never blocks on a full pipe.
        tokio::spawn(async move {
            let mut reader = reader;
            while let Ok(Some(_)) = reader.next_line().await {}
        });

        let server = Self {
            child: Some(child),
            base_url,
            port,
            password: builder.password,
            #[cfg(unix)]
            pgid,
        };

        // On failure `server` is dropped here, tearing the process down.
        server.await_healthy(deadline).await?;

        Ok(server)
    }

    /// Read stdout until the listening line appears, returning the parsed base
    /// URL and the still-open reader (so callers can keep draining stdout).
    async fn await_listening(
        stdout: tokio::process::ChildStdout,
        hostname: &str,
        port: u16,
        deadline: Instant,
        startup_timeout: Duration,
    ) -> Result<(
        String,
        tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    )> {
        let mut reader = BufReader::new(stdout).lines();

        loop {
            let now = Instant::now();
            if now >= deadline {
                return Err(Error::Server(format!(
                    "opencode serve did not report a listening address within {startup_timeout:?}"
                )));
            }
            let slice = (deadline - now).min(Duration::from_millis(200));

            match tokio::time::timeout(slice, reader.next_line()).await {
                Ok(Ok(Some(line))) => {
                    if let Some(url) = parse_listening_url(&line, hostname, port) {
                        return Ok((url, reader));
                    }
                }
                Ok(Ok(None)) => {
                    return Err(Error::Server(
                        "opencode serve exited before reporting a listening address".to_string(),
                    ));
                }
                Ok(Err(e)) => {
                    return Err(Error::Server(format!(
                        "error reading opencode serve stdout: {e}"
                    )));
                }
                // Per-slice timeout: loop and re-check the overall deadline.
                Err(_) => {}
            }
        }
    }

    /// Poll `GET /global/health` until it answers healthy or the deadline lapses.
    async fn await_healthy(&self, deadline: Instant) -> Result<()> {
        let client = reqwest::Client::new();
        let health_url = format!("{}/global/health", self.base_url);

        loop {
            let request = {
                let req = client.get(&health_url).timeout(Duration::from_millis(500));
                match &self.password {
                    Some(pw) => req.basic_auth(DEFAULT_USERNAME, Some(pw)),
                    None => req,
                }
            };

            if let Ok(resp) = request.send().await {
                if resp.status().is_success() {
                    return Ok(());
                }
            }

            if Instant::now() >= deadline {
                return Err(Error::Server(format!(
                    "opencode serve at {} never became healthy within the startup timeout",
                    self.base_url
                )));
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// The base URL the server bound to, e.g. `http://127.0.0.1:41999` (no
    /// trailing slash). Read back from the server's own stdout, so it reflects
    /// the actually-bound port even when the port was left to the OS.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.base_url
    }

    /// The TCP port the server listens on.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Whether the child process is still running (non-blocking check).
    ///
    /// Returns `false` once the server has been consumed by [`ManagedServer::stop`].
    pub fn is_running(&mut self) -> bool {
        self.child
            .as_mut()
            .is_some_and(|child| matches!(child.try_wait(), Ok(None)))
    }

    /// Terminate the server and await reaping of the child.
    ///
    /// On Unix the whole process group is signalled (`SIGTERM`, then `SIGKILL`
    /// after a grace period) before the direct child is awaited.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Server`] only if awaiting the child fails; an
    /// already-exited process is not an error.
    pub async fn stop(mut self) -> Result<()> {
        #[cfg(unix)]
        if let Some(pgid) = self.pgid.take() {
            signal_process_group(pgid, SIGTERM);
            tokio::time::sleep(KILL_GRACE).await;
            signal_process_group(pgid, SIGKILL);
        }

        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
            child
                .wait()
                .await
                .map_err(|e| Error::Server(format!("awaiting managed server exit failed: {e}")))?;
        }
        Ok(())
    }
}

impl Drop for ManagedServer {
    fn drop(&mut self) {
        let child = self.child.take();

        #[cfg(unix)]
        if let Some(pgid) = self.pgid.take() {
            // The direct child is the group leader, so a group SIGTERM starts
            // its grace period too. Drop cannot await, so the delayed SIGKILL
            // runs on a detached thread that also *owns* the child handle: while
            // it is held across the grace, the leader is not reaped, which both
            // keeps the pgid valid throughout (a zombie stays a group member
            // until reaped, so `-pgid` cannot target a recycled group) and
            // defers the handle's `kill_on_drop` SIGKILL until after the grace —
            // matching the documented SIGTERM-then-SIGKILL teardown.
            signal_process_group(pgid, SIGTERM);
            std::thread::spawn(move || {
                let _child = child;
                std::thread::sleep(KILL_GRACE);
                signal_process_group(pgid, SIGKILL);
            });
            return;
        }

        // Non-unix, or a server already consumed by `stop`: the child handle's
        // `kill_on_drop(true)` is the cross-platform backstop.
        if let Some(mut child) = child {
            let _ = child.start_kill();
        }
    }
}

/// Signal the process group and start killing the direct child (Unix + fallback).
fn signal_group_and_kill(pgid: Option<i32>, child: &mut Child) {
    #[cfg(unix)]
    if let Some(pgid) = pgid {
        signal_process_group(pgid, SIGTERM);
        signal_process_group(pgid, SIGKILL);
    }
    #[cfg(not(unix))]
    let _ = pgid;
    let _ = child.start_kill();
}

/// Extract the base URL from a `listening on http://host:port` stdout line.
///
/// Returns the URL exactly as printed (trailing slash stripped). Falls back to
/// synthesizing `http://<hostname>:<port>` from the requested values if the
/// line matches the marker but no `http` URL can be located.
fn parse_listening_url(line: &str, hostname: &str, port: u16) -> Option<String> {
    let idx = line.find(LISTENING_MARKER)?;
    let rest = line[idx + LISTENING_MARKER.len()..].trim();
    if let Some(start) = rest.find("http") {
        let url = rest[start..].trim().trim_end_matches('/');
        if !url.is_empty() {
            return Some(url.to_string());
        }
    }
    Some(format!("http://{hostname}:{port}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_defaults() {
        let b = ManagedServerBuilder::new();
        assert_eq!(b.binary, PathBuf::from("opencode"));
        assert_eq!(b.hostname, "127.0.0.1");
        assert!(b.port.is_none());
        assert!(b.working_dir.is_none());
        assert!(b.password.is_none());
        assert!(b.envs.is_empty());
        assert_eq!(b.startup_timeout, Duration::from_secs(10));
    }

    #[test]
    fn builder_setters() {
        let b = ManagedServer::builder()
            .binary("/opt/opencode")
            .hostname("0.0.0.0")
            .port(1234)
            .working_dir("/tmp/work")
            .password("hunter2")
            .env("FOO", "bar")
            .env("BAZ", "qux")
            .startup_timeout(Duration::from_secs(3));

        assert_eq!(b.binary, PathBuf::from("/opt/opencode"));
        assert_eq!(b.hostname, "0.0.0.0");
        assert_eq!(b.port, Some(1234));
        assert_eq!(b.working_dir, Some(PathBuf::from("/tmp/work")));
        assert_eq!(b.password.as_deref(), Some("hunter2"));
        assert_eq!(b.envs.get("FOO").map(String::as_str), Some("bar"));
        assert_eq!(b.envs.get("BAZ").map(String::as_str), Some("qux"));
        assert_eq!(b.startup_timeout, Duration::from_secs(3));
    }

    #[test]
    fn parse_listening_url_extracts_printed_url() {
        let line = "opencode server listening on http://127.0.0.1:52439";
        assert_eq!(
            parse_listening_url(line, "127.0.0.1", 4096).as_deref(),
            Some("http://127.0.0.1:52439")
        );
    }

    #[test]
    fn parse_listening_url_strips_trailing_slash() {
        let line = "opencode server listening on http://127.0.0.1:52439/";
        assert_eq!(
            parse_listening_url(line, "127.0.0.1", 4096).as_deref(),
            Some("http://127.0.0.1:52439")
        );
    }

    #[test]
    fn parse_listening_url_falls_back_when_no_url() {
        let line = "opencode server listening on <unknown>";
        assert_eq!(
            parse_listening_url(line, "localhost", 9000).as_deref(),
            Some("http://localhost:9000")
        );
    }

    #[test]
    fn parse_listening_url_rejects_unrelated_lines() {
        assert!(parse_listening_url("Warning: password not set", "127.0.0.1", 4096).is_none());
    }

    /// Live smoke test: spawn the real `opencode` binary through
    /// [`ManagedServer`] and confirm it answers `GET /global/health`.
    ///
    /// Gated behind `integration-tests` (in addition to `server`) since it
    /// requires the `opencode` executable. Override its location with
    /// `OPENCODE_BIN`; otherwise the npm-installed default path is used.
    #[cfg(feature = "integration-tests")]
    #[tokio::test]
    async fn managed_server_reaches_health() {
        let binary = std::env::var("OPENCODE_BIN").unwrap_or_else(|_| {
            let home = std::env::var("HOME").expect("HOME must be set");
            format!("{home}/.local/opencode-npm/node_modules/.bin/opencode")
        });

        let mut server = ManagedServer::builder()
            .binary(binary)
            .startup_timeout(Duration::from_secs(30))
            .spawn()
            .await
            .expect("managed server should start");

        assert!(server.is_running());
        assert!(server.url().starts_with("http://127.0.0.1:"));

        let health = format!("{}/global/health", server.url());
        let body: serde_json::Value = reqwest::Client::new()
            .get(&health)
            .send()
            .await
            .expect("health request should succeed")
            .json()
            .await
            .expect("health body should be JSON");
        assert_eq!(body["healthy"], serde_json::Value::Bool(true));

        server.stop().await.expect("server should stop cleanly");
    }
}
