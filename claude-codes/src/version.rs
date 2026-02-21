//! Version checking utilities for Claude CLI compatibility

use crate::error::Result;
use log::{debug, warn};
use std::process::Command;
use std::sync::Once;

/// The latest Claude CLI version we've tested against
const TESTED_VERSION: &str = "2.1.47";

/// Ensures version warning is only shown once per session
static VERSION_CHECK: Once = Once::new();

/// Check the Claude CLI version and warn if newer than tested
/// This will only issue a warning once per program execution
pub fn check_claude_version() -> Result<()> {
    VERSION_CHECK.call_once(|| {
        if let Err(e) = check_version_impl() {
            debug!("Failed to check Claude CLI version: {}", e);
        }
    });
    Ok(())
}

/// Internal implementation of version checking
fn check_version_impl() -> Result<()> {
    // Run claude --version
    let output = Command::new("claude")
        .arg("--version")
        .output()
        .map_err(crate::error::Error::Io)?;

    if !output.status.success() {
        debug!("Failed to check Claude CLI version - command failed");
        return Ok(());
    }

    let version_str = String::from_utf8_lossy(&output.stdout);
    let version_line = version_str.lines().next().unwrap_or("");

    // Extract version number (format: "1.0.89 (Claude Code)")
    if let Some(version) = version_line.split_whitespace().next() {
        if is_version_newer(version, TESTED_VERSION) {
            warn!(
                "Claude CLI version {} is newer than tested version {}. \
                 Please report compatibility at: https://github.com/meawoppl/rust-claude-codes/pulls",
                version, TESTED_VERSION
            );
        } else {
            debug!(
                "Claude CLI version {} is compatible (tested: {})",
                version, TESTED_VERSION
            );
        }
    } else {
        warn!(
            "Could not parse Claude CLI version from output: '{}'. \
             Please report compatibility at: https://github.com/meawoppl/rust-claude-codes/pulls",
            version_line
        );
    }

    Ok(())
}

/// Compare two version strings (e.g., "1.0.89" vs "1.0.90")
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

    // If all compared parts are equal, longer version is newer
    v_parts.len() > t_parts.len()
}

/// Async version check for tokio-based clients
#[cfg(feature = "async-client")]
pub async fn check_claude_version_async() -> Result<()> {
    use tokio::sync::OnceCell;

    // Use a static OnceCell for async initialization
    static ASYNC_VERSION_CHECK: OnceCell<()> = OnceCell::const_new();

    ASYNC_VERSION_CHECK
        .get_or_init(|| async {
            if let Err(e) = check_version_impl_async().await {
                debug!("Failed to check Claude CLI version: {}", e);
            }
        })
        .await;

    Ok(())
}

/// Internal async implementation of version checking
#[cfg(feature = "async-client")]
async fn check_version_impl_async() -> Result<()> {
    use tokio::process::Command;

    // Run claude --version
    let output = Command::new("claude")
        .arg("--version")
        .output()
        .await
        .map_err(crate::error::Error::Io)?;

    if !output.status.success() {
        debug!("Failed to check Claude CLI version - command failed");
        return Ok(());
    }

    let version_str = String::from_utf8_lossy(&output.stdout);
    let version_line = version_str.lines().next().unwrap_or("");

    // Extract version number (format: "1.0.89 (Claude Code)")
    if let Some(version) = version_line.split_whitespace().next() {
        if is_version_newer(version, TESTED_VERSION) {
            warn!(
                "Claude CLI version {} is newer than tested version {}. \
                 Please report compatibility at: https://github.com/meawoppl/rust-claude-codes/pulls",
                version, TESTED_VERSION
            );
        } else {
            debug!(
                "Claude CLI version {} is compatible (tested: {})",
                version, TESTED_VERSION
            );
        }
    } else {
        warn!(
            "Could not parse Claude CLI version from output: '{}'. \
             Please report compatibility at: https://github.com/meawoppl/rust-claude-codes/pulls",
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
        // Test basic version comparison
        assert!(is_version_newer("1.0.90", "1.0.89"));
        assert!(!is_version_newer("1.0.89", "1.0.90"));
        assert!(!is_version_newer("1.0.89", "1.0.89"));

        // Test with different segment counts
        assert!(is_version_newer("1.1", "1.0.89"));
        assert!(!is_version_newer("1.0", "1.0.89"));
        assert!(is_version_newer("1.0.89.1", "1.0.89"));

        // Test major version differences
        assert!(is_version_newer("2.0.0", "1.99.99"));
        assert!(!is_version_newer("0.9.99", "1.0.0"));
    }
}
