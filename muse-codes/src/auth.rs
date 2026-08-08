//! Login support tooling for Muse Code (feature `async-client`).
//!
//! Muse's auth surface is automation-friendly — no TUI to drive:
//!
//! - [`auth_set`] wraps `muse auth set --api-key-stdin` (the key travels
//!   over stdin; Muse refuses to take secrets as arguments).
//! - [`DeviceLoginFlow`] wraps `muse login`, a plain-stdout OAuth
//!   device-code flow: the CLI prints a verification URL and a short code,
//!   then polls until the user approves in a browser.
//! - [`logout`] wraps `muse logout` (removes the saved credential;
//!   `META_API_KEY` in the environment is never touched).
//! - [`credentials_present`] reports whether a run could authenticate
//!   right now (env key or saved credential file).
//!
//! Credential resolution order (per the CLI): `META_API_KEY` env always
//! wins, then the saved credential at `~/.config/muse/auth.json`.

use crate::error::{Error, Result};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdout};

/// Environment variable that overrides any saved credential.
pub const META_API_KEY_VAR: &str = "META_API_KEY";

/// Path of the saved credential file (`~/.config/muse/auth.json`),
/// honoring `XDG_CONFIG_HOME`. `None` when no home directory resolves.
pub fn credentials_path() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg).join("muse/auth.json"));
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".config/muse/auth.json"))
}

/// True when a headless run could authenticate right now: `META_API_KEY`
/// is set (non-empty) or the saved credential file carries at least one
/// provider. (`muse logout` empties the providers map but keeps the file,
/// so bare existence is not enough.)
pub fn credentials_present() -> bool {
    if std::env::var(META_API_KEY_VAR).map(|v| !v.trim().is_empty()) == Ok(true) {
        return true;
    }
    let Some(path) = credentials_path() else {
        return false;
    };
    match std::fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str::<AuthFile>(&raw)
            .map(|f| !f.providers.is_empty())
            // Unparseable file: assume it authenticates (newer schema).
            .unwrap_or(true),
        Err(_) => false,
    }
}

/// Shape of `~/.config/muse/auth.json` (observed schema_version 1).
///
/// `muse logout` rewrites this with an empty `providers` map rather than
/// deleting the file.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AuthFile {
    pub schema_version: u32,
    pub providers: std::collections::BTreeMap<String, ProviderCredential>,
    #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// One saved provider credential.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProviderCredential {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

fn resolve(binary: &str) -> Result<PathBuf> {
    which::which(binary).map_err(|_| Error::BinaryNotFound {
        name: binary.to_string(),
    })
}

/// Save a provider API key: `muse auth set --provider <p> --api-key-stdin`.
///
/// The key is written to the child's stdin and never appears on a command
/// line. `provider` defaults to `"meta"` when `None`.
pub async fn auth_set(api_key: &str, provider: Option<&str>) -> Result<()> {
    auth_set_with_binary("muse", api_key, provider).await
}

/// [`auth_set`] against a specific CLI binary.
pub async fn auth_set_with_binary(
    binary: &str,
    api_key: &str,
    provider: Option<&str>,
) -> Result<()> {
    let mut cmd = tokio::process::Command::new(resolve(binary)?);
    cmd.args([
        "auth",
        "set",
        "--provider",
        provider.unwrap_or("meta"),
        "--api-key-stdin",
    ])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .kill_on_drop(true);
    let mut child = cmd.spawn()?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| Error::Protocol("failed to get stdin".to_string()))?;
    stdin.write_all(api_key.trim().as_bytes()).await?;
    stdin.write_all(b"\n").await?;
    drop(stdin); // EOF tells the CLI the key is complete.
    let out = child.wait_with_output().await?;
    if out.status.success() {
        Ok(())
    } else {
        Err(Error::Protocol(format!(
            "muse auth set failed (exit {:?}): {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim()
        )))
    }
}

/// Remove the saved credential: `muse logout`.
pub async fn logout() -> Result<()> {
    logout_with_binary("muse").await
}

/// [`logout`] against a specific CLI binary.
pub async fn logout_with_binary(binary: &str) -> Result<()> {
    let out = tokio::process::Command::new(resolve(binary)?)
        .arg("logout")
        .stdin(Stdio::null())
        .output()
        .await?;
    if out.status.success() {
        Ok(())
    } else {
        Err(Error::Protocol(format!(
            "muse logout failed (exit {:?}): {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim()
        )))
    }
}

/// The verification details a [`DeviceLoginFlow`] presents to the user.
/// Serde-serializable for relay to remote UIs.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DeviceCode {
    /// URL the user opens to approve the login.
    pub verification_url: String,
    /// Short confirmation code the user checks against the browser page.
    pub code: String,
}

/// An in-flight `muse login` OAuth device-code flow.
///
/// Plain stdout, no pseudo-terminal: the CLI prints the verification URL
/// and code, then blocks polling for browser approval. Dropping the flow
/// cancels the login (the child is killed).
pub struct DeviceLoginFlow {
    child: Child,
    lines: Lines<BufReader<ChildStdout>>,
}

impl DeviceLoginFlow {
    /// Spawn `muse login` from `PATH`.
    pub async fn start() -> Result<Self> {
        Self::start_with_binary("muse").await
    }

    /// [`start`](Self::start) with a specific CLI binary.
    pub async fn start_with_binary(binary: &str) -> Result<Self> {
        let mut cmd = tokio::process::Command::new(resolve(binary)?);
        cmd.arg("login")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let mut child = cmd.spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Protocol("failed to get stdout".to_string()))?;
        Ok(Self {
            child,
            lines: BufReader::new(stdout).lines(),
        })
    }

    /// Read output until the verification URL and code are both seen.
    ///
    /// Show them to your user; the flow keeps polling in the background
    /// until [`wait_approved`](Self::wait_approved) resolves.
    pub async fn device_code(&mut self, timeout: Duration) -> Result<DeviceCode> {
        let read = async {
            let mut url: Option<String> = None;
            let mut code: Option<String> = None;
            while let Some(line) = self.lines.next_line().await? {
                if let Some(u) = extract_url(&line) {
                    url = Some(u);
                }
                if let Some(c) = extract_code_from_url_or_line(&line) {
                    code = Some(c);
                }
                if let (Some(u), Some(c)) = (&url, &code) {
                    return Ok(DeviceCode {
                        verification_url: u.clone(),
                        code: c.clone(),
                    });
                }
            }
            Err(Error::Protocol(
                "muse login ended before printing a device code".to_string(),
            ))
        };
        tokio::time::timeout(timeout, read)
            .await
            .map_err(|_| Error::Protocol("timed out waiting for device code".to_string()))?
    }

    /// Wait for the user to approve in the browser: resolves when the CLI
    /// exits successfully (credential saved) or errors on failure/timeout.
    pub async fn wait_approved(mut self, timeout: Duration) -> Result<()> {
        let status = tokio::time::timeout(timeout, self.child.wait())
            .await
            .map_err(|_| Error::Protocol("timed out waiting for login approval".to_string()))??;
        if status.success() {
            Ok(())
        } else {
            Err(Error::Protocol(format!(
                "muse login exited with {:?} before approval",
                status.code()
            )))
        }
    }

    /// Cancel the login.
    pub async fn cancel(mut self) -> Result<()> {
        self.child.kill().await?;
        Ok(())
    }
}

/// First `https://` run in a line (device URLs carry no trailing prose on
/// the observed wire, but trim conservatively anyway).
fn extract_url(line: &str) -> Option<String> {
    let start = line.find("https://")?;
    let url: String = line[start..]
        .chars()
        .take_while(|c| !c.is_whitespace())
        .collect();
    Some(url)
}

/// The device code: from the URL's `code=` parameter, or a bare
/// `XXXX-XXXX`-shaped token on its own line (the CLI prints both forms).
fn extract_code_from_url_or_line(line: &str) -> Option<String> {
    if let Some(pos) = line.find("code=") {
        let code: String = line[pos + 5..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        if !code.is_empty() {
            return Some(code);
        }
    }
    let t = line.trim();
    let is_code_shaped = t.len() >= 7
        && t.len() <= 12
        && t.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
        && t.contains('-');
    is_code_shaped.then(|| t.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verbatim `muse login` output captured from Muse Code 0.1.0.
    const CAPTURE: &str = "Open this page to sign in:\n  \
        https://auth.meta.com/oauth/device/?code=TBSS-QJWM\n\
        confirm this code matches:\n  TBSS-QJWM\n\nWaiting for approval…\n";

    #[test]
    fn device_code_extracted_from_captured_output() {
        let mut url = None;
        let mut code = None;
        for line in CAPTURE.lines() {
            if let Some(u) = extract_url(line) {
                url = Some(u);
            }
            if let Some(c) = extract_code_from_url_or_line(line) {
                code = Some(c);
            }
        }
        assert_eq!(
            url.as_deref(),
            Some("https://auth.meta.com/oauth/device/?code=TBSS-QJWM")
        );
        assert_eq!(code.as_deref(), Some("TBSS-QJWM"));
    }

    #[test]
    fn bare_code_line_matches_and_prose_does_not() {
        assert_eq!(
            extract_code_from_url_or_line("  TBSS-QJWM"),
            Some("TBSS-QJWM".to_string())
        );
        assert_eq!(extract_code_from_url_or_line("Waiting for approval…"), None);
        assert_eq!(
            extract_code_from_url_or_line("Open this page to sign in:"),
            None
        );
    }
}
