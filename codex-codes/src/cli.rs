//! Builder for launching the Codex app-server process.
//!
//! The [`AppServerBuilder`] configures and spawns `codex app-server --listen stdio://`,
//! a long-lived process that speaks JSON-RPC over newline-delimited stdio.

use log::debug;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::Stdio;

/// Builder for launching a Codex app-server process.
///
/// Produces commands of the form: `codex [-c k=v]... app-server --listen stdio:// [extra]...`
///
/// All model, sandbox, and approval configuration that isn't expressible as a
/// CLI flag is done via JSON-RPC requests after connecting. For everything
/// that *is* a CLI flag, see [`config_override`](Self::config_override) and
/// [`extra_args`](Self::extra_args).
#[derive(Debug, Clone)]
pub struct AppServerBuilder {
    command: PathBuf,
    working_directory: Option<PathBuf>,
    /// Environment variables set on the app-server child process.
    environment: Vec<(OsString, OsString)>,
    /// `-c key=value` overrides, in insertion order. Emitted *before* the
    /// `app-server` subcommand because `-c` is a global `codex` flag.
    config_overrides: Vec<(String, String)>,
    /// Raw additional args appended *after* the `--listen stdio://` so they
    /// land as subcommand args to `app-server`.
    extra_args: Vec<String>,
}

impl Default for AppServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AppServerBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self {
            command: PathBuf::from("codex"),
            working_directory: None,
            environment: Vec::new(),
            config_overrides: Vec::new(),
            extra_args: Vec::new(),
        }
    }

    /// Set custom path to the codex binary.
    pub fn command<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.command = path.into();
        self
    }

    /// Set the working directory for the app-server process.
    pub fn working_directory<P: Into<PathBuf>>(mut self, dir: P) -> Self {
        self.working_directory = Some(dir.into());
        self
    }

    /// Set an environment variable on the app-server child process.
    ///
    /// Calling this more than once with the same key follows
    /// [`std::process::Command::env`] semantics: the last value wins.
    ///
    /// # Example
    ///
    /// ```
    /// use codex_codes::AppServerBuilder;
    ///
    /// let builder = AppServerBuilder::new().env("CODEX_HOME", "/tmp/codex-home");
    /// ```
    pub fn env<K, V>(mut self, key: K, value: V) -> Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.environment
            .push((key.as_ref().to_os_string(), value.as_ref().to_os_string()));
        self
    }

    /// Set environment variables on the app-server child process.
    ///
    /// If the iterator contains duplicate keys, or a key was already set by
    /// [`env`](Self::env), the last value wins.
    pub fn envs<I, K, V>(mut self, variables: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.environment.extend(
            variables
                .into_iter()
                .map(|(key, value)| (key.as_ref().to_os_string(), value.as_ref().to_os_string())),
        );
        self
    }

    /// Append a `-c key=value` global config override.
    ///
    /// Repeatable. Each call appends one override; order is preserved on the
    /// command line. The `value` is passed to codex unparsed — codex tries
    /// TOML, then falls back to the raw string. The caller is responsible for
    /// any quoting / escaping the value itself needs (e.g. arrays:
    /// `("sandbox_permissions", r#"["disk-full-read-access"]"#)`).
    ///
    /// `-c` flags are placed *before* the `app-server` subcommand because
    /// they're parsed as global `codex` options, not subcommand args.
    ///
    /// # Example
    ///
    /// ```
    /// use codex_codes::AppServerBuilder;
    ///
    /// let builder = AppServerBuilder::new()
    ///     .config_override("sandbox_mode", "workspace-write")
    ///     .config_override("approval_policy", "on-request");
    /// ```
    pub fn config_override<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.config_overrides.push((key.into(), value.into()));
        self
    }

    /// Append raw arguments to the `app-server` subcommand invocation.
    ///
    /// Inserted *after* the hardcoded `--listen stdio://`, so they land as
    /// subcommand args. Use this for flags the SDK doesn't model yet — e.g.
    /// `--strict-config`, or `--session-source app-server` once that becomes
    /// available on the multitool subcommand.
    ///
    /// For `-c key=value` global overrides use [`config_override`](Self::config_override)
    /// instead; those need to be placed before the subcommand.
    ///
    /// # Example
    ///
    /// ```
    /// use codex_codes::AppServerBuilder;
    ///
    /// let builder = AppServerBuilder::new()
    ///     .extra_args(["--strict-config"]);
    /// ```
    pub fn extra_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.extra_args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Resolve the command path, using `which` for non-absolute paths.
    fn resolve_command(&self) -> crate::error::Result<PathBuf> {
        if self.command.is_absolute() {
            return Ok(self.command.clone());
        }
        which::which(&self.command).map_err(|_| crate::error::Error::BinaryNotFound {
            name: self.command.display().to_string(),
        })
    }

    /// Build the command arguments.
    ///
    /// Layout: `[-c k=v]... app-server --listen stdio:// [extra_args]...`
    fn build_args(&self) -> Vec<String> {
        let mut args =
            Vec::with_capacity(self.config_overrides.len() * 2 + 3 + self.extra_args.len());
        for (k, v) in &self.config_overrides {
            args.push("-c".to_string());
            args.push(format!("{k}={v}"));
        }
        args.push("app-server".to_string());
        args.push("--listen".to_string());
        args.push("stdio://".to_string());
        args.extend(self.extra_args.iter().cloned());
        args
    }

    /// Build a Tokio command without spawning it.
    ///
    /// This is the escape hatch for process configuration not modeled by the
    /// builder. The command is fully configured for app-server stdio; callers
    /// can make further adjustments before spawning it.
    ///
    /// Pair this with [`crate::AsyncClient::new`] to customize the command
    /// while preserving the SDK's stdio setup.
    #[cfg(feature = "async-client")]
    pub fn build_command(self) -> crate::error::Result<tokio::process::Command> {
        self.build_command_sync().map(tokio::process::Command::from)
    }

    /// Build a standard-library command without spawning it.
    ///
    /// This is the synchronous counterpart to [`build_command`](Self::build_command).
    /// Pair it with [`crate::SyncClient::new`] to customize the command while
    /// preserving the SDK's stdio setup.
    pub fn build_command_sync(self) -> crate::error::Result<std::process::Command> {
        let resolved = self.resolve_command()?;
        let args = self.build_args();

        debug!(
            "[CLI] Building app-server command: {} {}",
            resolved.display(),
            args.join(" ")
        );

        let mut command = std::process::Command::new(resolved);
        command
            .args(args)
            .envs(self.environment)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(dir) = self.working_directory {
            command.current_dir(dir);
        }

        Ok(command)
    }

    /// Spawn the app-server process asynchronously.
    #[cfg(feature = "async-client")]
    pub async fn spawn(self) -> crate::error::Result<tokio::process::Child> {
        self.build_command()?
            .spawn()
            .map_err(crate::error::Error::Io)
    }

    /// Spawn the app-server process synchronously.
    pub fn spawn_sync(self) -> crate::error::Result<std::process::Child> {
        self.build_command_sync()?
            .spawn()
            .map_err(crate::error::Error::Io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_args() {
        let builder = AppServerBuilder::new();
        let args = builder.build_args();

        assert_eq!(args, vec!["app-server", "--listen", "stdio://"]);
    }

    #[test]
    fn test_custom_command() {
        let builder = AppServerBuilder::new().command("/usr/local/bin/codex");
        assert_eq!(builder.command, PathBuf::from("/usr/local/bin/codex"));
    }

    #[test]
    fn test_working_directory() {
        let builder = AppServerBuilder::new().working_directory("/tmp/work");
        assert_eq!(builder.working_directory, Some(PathBuf::from("/tmp/work")));
    }

    #[test]
    fn test_build_command_sync_applies_process_configuration() {
        let executable = std::env::current_exe().unwrap();
        let command = AppServerBuilder::new()
            .command(&executable)
            .working_directory("/tmp/work")
            .env("CODEX_HOME", "/tmp/original")
            .envs([("RUST_LOG", "debug"), ("CODEX_HOME", "/tmp/codex-home")])
            .config_override("approval_policy", "never")
            .extra_args(["--strict-config"])
            .build_command_sync()
            .unwrap();

        assert_eq!(command.get_program(), executable);
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            [
                "-c",
                "approval_policy=never",
                "app-server",
                "--listen",
                "stdio://",
                "--strict-config",
            ]
        );
        assert_eq!(
            command.get_current_dir(),
            Some(std::path::Path::new("/tmp/work"))
        );
        assert!(command.get_envs().any(|(key, value)| {
            key == "CODEX_HOME" && value == Some(OsStr::new("/tmp/codex-home"))
        }));
        assert!(command
            .get_envs()
            .any(|(key, value)| key == "RUST_LOG" && value == Some(OsStr::new("debug"))));
    }

    #[test]
    fn test_config_override_single() {
        let args = AppServerBuilder::new()
            .config_override("sandbox_mode", "workspace-write")
            .build_args();
        assert_eq!(
            args,
            vec![
                "-c",
                "sandbox_mode=workspace-write",
                "app-server",
                "--listen",
                "stdio://"
            ]
        );
    }

    #[test]
    fn test_config_override_multiple_preserves_order() {
        let args = AppServerBuilder::new()
            .config_override("sandbox_mode", "workspace-write")
            .config_override("approval_policy", "on-request")
            .build_args();
        // Both `-c` pairs come BEFORE `app-server` since `-c` is a global
        // codex flag.
        assert_eq!(
            args,
            vec![
                "-c",
                "sandbox_mode=workspace-write",
                "-c",
                "approval_policy=on-request",
                "app-server",
                "--listen",
                "stdio://"
            ]
        );
    }

    #[test]
    fn test_extra_args_appended_after_listen() {
        let args = AppServerBuilder::new()
            .extra_args(["--strict-config"])
            .build_args();
        assert_eq!(
            args,
            vec!["app-server", "--listen", "stdio://", "--strict-config"]
        );
    }

    #[test]
    fn test_config_override_and_extra_args_combined() {
        let args = AppServerBuilder::new()
            .config_override("sandbox_mode", "workspace-write")
            .extra_args(["--strict-config", "--something-else"])
            .build_args();
        assert_eq!(
            args,
            vec![
                "-c",
                "sandbox_mode=workspace-write",
                "app-server",
                "--listen",
                "stdio://",
                "--strict-config",
                "--something-else",
            ]
        );
    }

    #[test]
    fn test_config_override_value_with_special_chars_unchanged() {
        // Caller is responsible for quoting the value half; we pass it
        // through unchanged so codex's TOML parser sees it verbatim.
        let args = AppServerBuilder::new()
            .config_override("sandbox_permissions", r#"["disk-full-read-access"]"#)
            .build_args();
        assert_eq!(args[1], r#"sandbox_permissions=["disk-full-read-access"]"#);
    }
}
