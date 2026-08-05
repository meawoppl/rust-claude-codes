//! Async client for streaming a headless `muse exec --json` run.
//!
//! One [`ExecRun`] wraps one child process; [`next_record`](ExecRun::next_record)
//! yields typed journal records as they arrive, and
//! [`wait_terminal`](ExecRun::wait_terminal) drives the run to its
//! `run.terminal.*` record.

use crate::cli::MuseExecBuilder;
use crate::error::{Error, Result};
use crate::io::{MusePayload, MuseRecord};
use tokio::io::{AsyncBufReadExt, BufReader, Lines};
use tokio::process::{Child, ChildStderr, ChildStdout};

/// A running `muse exec --json` invocation.
pub struct ExecRun {
    child: Child,
    lines: Lines<BufReader<ChildStdout>>,
    /// stderr is collected in the background for error context.
    stderr_task: tokio::task::JoinHandle<String>,
}

impl ExecRun {
    /// Spawn a run from a builder.
    pub async fn spawn(builder: &MuseExecBuilder) -> Result<Self> {
        let mut child = builder.spawn().await?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Protocol("failed to get stdout".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| Error::Protocol("failed to get stderr".to_string()))?;
        Ok(Self {
            child,
            lines: BufReader::new(stdout).lines(),
            stderr_task: spawn_stderr_collector(stderr),
        })
    }

    /// Next journal record, or `None` at end of stream.
    pub async fn next_record(&mut self) -> Result<Option<MuseRecord>> {
        loop {
            match self.lines.next_line().await? {
                None => return Ok(None),
                Some(line) if line.trim().is_empty() => continue,
                Some(line) => return Ok(Some(serde_json::from_str(&line)?)),
            }
        }
    }

    /// Consume records until the run reaches a terminal state, invoking
    /// `on_record` for each record seen (including the terminal one), and
    /// return the terminal payload.
    ///
    /// If the stream ends without a `run.terminal.*` record, the child's
    /// exit code and collected stderr are folded into the error.
    pub async fn wait_terminal<F>(mut self, mut on_record: F) -> Result<crate::io::RunTerminal>
    where
        F: FnMut(&MuseRecord),
    {
        while let Some(record) = self.next_record().await? {
            on_record(&record);
            if let Ok(MusePayload::RunTerminal(t)) = record.typed_payload() {
                return Ok(t);
            }
        }
        let status = self.child.wait().await?;
        let stderr = self.stderr_task.await.unwrap_or_default();
        Err(Error::Protocol(format!(
            "stream ended without run.terminal.* (exit {:?}); stderr:\n{}",
            status.code(),
            stderr.trim()
        )))
    }

    /// Kill the child process.
    pub async fn kill(&mut self) -> Result<()> {
        self.child.kill().await?;
        Ok(())
    }
}

fn spawn_stderr_collector(stderr: ChildStderr) -> tokio::task::JoinHandle<String> {
    tokio::spawn(async move {
        let mut out = String::new();
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            #[cfg(feature = "async-client")]
            log::debug!(target: "muse_codes::stderr", "{line}");
            out.push_str(&line);
            out.push('\n');
        }
        out
    })
}
