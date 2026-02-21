//! Version checking utilities for Codex CLI compatibility.

use crate::error::Result;
use log::{debug, warn};
use std::process::Command;
use std::sync::Once;

/// The latest Codex CLI version we've tested against.
const TESTED_VERSION: &str = "0.104.0";

/// Ensures version warning is only shown once per session.
static VERSION_CHECK: Once = Once::new();

/// Check the Codex CLI version and warn if newer than tested.
///
/// This will only issue a warning once per program execution.
pub fn check_codex_version() -> Result<()> {
    VERSION_CHECK.call_once(|| {
        if let Err(e) = check_version_impl() {
            debug!("Failed to check Codex CLI version: {}", e);
        }
    });
    Ok(())
}

fn check_version_impl() -> Result<()> {
    let output = Command::new("codex")
        .arg("--version")
        .output()
        .map_err(crate::error::Error::Io)?;

    if !output.status.success() {
        debug!("Failed to check Codex CLI version - command failed");
        return Ok(());
    }

    let version_str = String::from_utf8_lossy(&output.stdout);
    let version_line = version_str.lines().next().unwrap_or("");

    // Format: "codex-cli X.Y.Z"
    if let Some(version) = version_line.split_whitespace().last() {
        if is_version_newer(version, TESTED_VERSION) {
            warn!(
                "Codex CLI version {} is newer than tested version {}. \
                 Please report compatibility at: https://github.com/meawoppl/rust-code-agent-sdks/issues",
                version, TESTED_VERSION
            );
        } else {
            debug!(
                "Codex CLI version {} is compatible (tested: {})",
                version, TESTED_VERSION
            );
        }
    } else {
        warn!(
            "Could not parse Codex CLI version from output: '{}'. \
             Please report compatibility at: https://github.com/meawoppl/rust-code-agent-sdks/issues",
            version_line
        );
    }

    Ok(())
}

/// Compare two version strings (e.g., "0.104.0" vs "0.103.0").
fn is_version_newer(version: &str, tested: &str) -> bool {
    let v_parts: Vec<u32> = version.split('.').filter_map(|s| s.parse().ok()).collect();
    let t_parts: Vec<u32> = tested.split('.').filter_map(|s| s.parse().ok()).collect();

    use std::cmp::Ordering;

    for i in 0..v_parts.len().min(t_parts.len()) {
        match v_parts[i].cmp(&t_parts[i]) {
            Ordering::Greater => return true,
            Ordering::Less => return false,
            Ordering::Equal => continue,
        }
    }

    v_parts.len() > t_parts.len()
}

/// Async version check for tokio-based clients.
#[cfg(feature = "async-client")]
pub async fn check_codex_version_async() -> Result<()> {
    use tokio::sync::OnceCell;

    static ASYNC_VERSION_CHECK: OnceCell<()> = OnceCell::const_new();

    ASYNC_VERSION_CHECK
        .get_or_init(|| async {
            if let Err(e) = check_version_impl_async().await {
                debug!("Failed to check Codex CLI version: {}", e);
            }
        })
        .await;

    Ok(())
}

#[cfg(feature = "async-client")]
async fn check_version_impl_async() -> Result<()> {
    use tokio::process::Command;

    let output = Command::new("codex")
        .arg("--version")
        .output()
        .await
        .map_err(crate::error::Error::Io)?;

    if !output.status.success() {
        debug!("Failed to check Codex CLI version - command failed");
        return Ok(());
    }

    let version_str = String::from_utf8_lossy(&output.stdout);
    let version_line = version_str.lines().next().unwrap_or("");

    // Format: "codex-cli X.Y.Z"
    if let Some(version) = version_line.split_whitespace().last() {
        if is_version_newer(version, TESTED_VERSION) {
            warn!(
                "Codex CLI version {} is newer than tested version {}. \
                 Please report compatibility at: https://github.com/meawoppl/rust-code-agent-sdks/issues",
                version, TESTED_VERSION
            );
        } else {
            debug!(
                "Codex CLI version {} is compatible (tested: {})",
                version, TESTED_VERSION
            );
        }
    } else {
        warn!(
            "Could not parse Codex CLI version from output: '{}'. \
             Please report compatibility at: https://github.com/meawoppl/rust-code-agent-sdks/issues",
            version_line
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_version_newer("0.105.0", "0.104.0"));
        assert!(!is_version_newer("0.104.0", "0.104.0"));
        assert!(!is_version_newer("0.103.0", "0.104.0"));

        assert!(is_version_newer("1.0.0", "0.104.0"));
        assert!(!is_version_newer("0.0.1", "0.104.0"));
        assert!(is_version_newer("0.104.1", "0.104.0"));
    }
}
