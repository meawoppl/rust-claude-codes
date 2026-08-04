//! Background drain for the ACP adapter's stderr pipe.
//!
//! `hermes acp` routes all Python logging to stderr (stdout is reserved for
//! JSON-RPC). Left unread, the ~64 KB kernel pipe buffer fills and blocks
//! the child; this task drains it line by line into the `log` crate under
//! the `hermes_codes::stderr` target so `RUST_LOG` controls visibility.

use log::{debug, error, warn};
use tokio::io::{AsyncBufReadExt, BufReader};

const TARGET: &str = "hermes_codes::stderr";

fn forward_line(line: &str) {
    // Python logging's default format carries the level name as a token.
    if line.contains("ERROR") || line.contains("CRITICAL") {
        error!(target: TARGET, "{line}");
    } else if line.contains("WARNING") {
        warn!(target: TARGET, "{line}");
    } else {
        debug!(target: TARGET, "{line}");
    }
}

pub(crate) fn spawn_async(stderr: tokio::process::ChildStderr) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            forward_line(&line);
        }
    })
}
