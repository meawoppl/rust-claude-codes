//! Cross-harness conformance tier: four capability checks every wrapped
//! agent must pass, each verified with UUID nonces so a model can't fake
//! its way through —
//!
//!   1. `conform_hello` — responds to a prompt (echoes a nonce back)
//!   2. `conform_read`  — reads a file with its file tool (reports the
//!      UUID the harness planted there)
//!   3. `conform_write` — writes a requested UUID to a requested path
//!      (verified on disk, not from the transcript)
//!   4. `conform_bash`  — runs a shell command that writes a nonce to a
//!      file (disk artifact proves execution, not narration)
//!
//! Each agent module supplies a runner: one prompt in, the agent's final
//! answer text out, tools enabled, operating in the given workdir. A
//! runner may return an error starting with `SKIP:` to mark the tier
//! skipped (e.g. opencode with no model provider configured).

use crate::state::{CheckStatus, Reporter};
use std::future::Future;
use std::path::PathBuf;

/// One conformance turn: the prompt to send and the workspace the agent
/// should operate in (all paths in the prompt are absolute and inside it).
pub struct Turn {
    pub prompt: String,
    pub workdir: PathBuf,
}

fn uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Run the four conformance checks through `runner`. Per-check timeout is
/// 300s; a `SKIP:`-prefixed runner error marks that check Skipped.
pub async fn run<F, Fut>(reporter: &Reporter, runner: F)
where
    F: Fn(Turn) -> Fut,
    Fut: Future<Output = Result<String, String>>,
{
    let root = std::env::temp_dir().join(format!("wirecheck-conform-{}", uuid()));
    if let Err(e) = std::fs::create_dir_all(&root) {
        let started = reporter.start("conform_hello", HELLO_WHAT).await;
        reporter
            .finish(
                "conform_hello",
                started,
                CheckStatus::Fail,
                format!("workdir: {e}"),
            )
            .await;
        return;
    }

    // 1. hello — the agent answers and can repeat a token verbatim.
    let nonce = uuid();
    let started = reporter.start("conform_hello", HELLO_WHAT).await;
    let outcome = timed(
        runner(Turn {
            prompt: format!("Reply with exactly this token and nothing else: {nonce}"),
            workdir: root.clone(),
        }),
        300,
    )
    .await;
    finish(
        reporter,
        "conform_hello",
        started,
        outcome.map(|answer| {
            if answer.contains(&nonce) {
                Ok("answer echoed the nonce".to_string())
            } else {
                Err(format!("nonce missing from answer: {}", clip(&answer)))
            }
        }),
    )
    .await;

    // 2. read — plant a UUID, the agent must actually read it.
    let nonce = uuid();
    let secret = root.join("secret.txt");
    let _ = std::fs::write(&secret, format!("{nonce}\n"));
    let started = reporter.start("conform_read", READ_WHAT).await;
    let outcome = timed(
        runner(Turn {
            prompt: format!(
                "A file exists at {} — read it with your file-reading tool and \
                 reply with the UUID it contains.",
                secret.display()
            ),
            workdir: root.clone(),
        }),
        300,
    )
    .await;
    finish(
        reporter,
        "conform_read",
        started,
        outcome.map(|answer| {
            if answer.contains(&nonce) {
                Ok("answer contained the planted UUID".to_string())
            } else {
                Err(format!("planted UUID not in answer: {}", clip(&answer)))
            }
        }),
    )
    .await;

    // 3. write — the agent writes a requested UUID to a requested path;
    //    the harness verifies on DISK, ignoring what the agent claims.
    let nonce = uuid();
    let target = root.join("written.txt");
    let started = reporter.start("conform_write", WRITE_WHAT).await;
    let outcome = timed(
        runner(Turn {
            prompt: format!(
                "Using your file-writing tool, create a file at {} whose \
                 contents is exactly this UUID: {nonce}",
                target.display()
            ),
            workdir: root.clone(),
        }),
        300,
    )
    .await;
    finish(
        reporter,
        "conform_write",
        started,
        outcome.map(|_| match std::fs::read_to_string(&target) {
            Ok(c) if c.contains(&nonce) => Ok("UUID found on disk at the requested path".into()),
            Ok(c) => Err(format!("file exists but UUID missing: {}", clip(&c))),
            Err(e) => Err(format!("file not written: {e}")),
        }),
    )
    .await;

    // 4. bash — a shell command with a disk-visible side effect; the file
    //    appearing proves the command RAN (an answer alone proves nothing).
    let nonce = uuid();
    let target = root.join("bash_out.txt");
    let started = reporter.start("conform_bash", BASH_WHAT).await;
    let outcome = timed(
        runner(Turn {
            prompt: format!(
                "Using your shell tool, run exactly this command: \
                 echo {nonce} > {}",
                target.display()
            ),
            workdir: root.clone(),
        }),
        300,
    )
    .await;
    finish(
        reporter,
        "conform_bash",
        started,
        outcome.map(|_| match std::fs::read_to_string(&target) {
            Ok(c) if c.contains(&nonce) => Ok("command ran — nonce on disk".into()),
            Ok(c) => Err(format!("file exists but nonce missing: {}", clip(&c))),
            Err(e) => Err(format!("command did not run (no file): {e}")),
        }),
    )
    .await;

    let _ = std::fs::remove_dir_all(&root);
}

const HELLO_WHAT: &str = "agent responds and echoes a UUID nonce";
const READ_WHAT: &str = "agent reads a planted file and reports its UUID";
const WRITE_WHAT: &str = "agent writes a requested UUID to a path (verified on disk)";
const BASH_WHAT: &str = "agent runs a shell command with a disk-visible effect";

async fn timed(
    fut: impl Future<Output = Result<String, String>>,
    timeout_s: u64,
) -> Result<String, String> {
    tokio::time::timeout(std::time::Duration::from_secs(timeout_s), fut)
        .await
        .map_err(|_| format!("timed out after {timeout_s}s"))?
}

/// Collapse the two error layers (runner failure vs assertion failure)
/// into one reported status, honoring the `SKIP:` convention.
async fn finish(
    reporter: &Reporter,
    name: &'static str,
    started: std::time::Instant,
    outcome: Result<Result<String, String>, String>,
) {
    let (status, detail) = match outcome {
        Ok(Ok(d)) => (CheckStatus::Pass, d),
        Ok(Err(d)) => (CheckStatus::Fail, d),
        Err(e) if e.starts_with("SKIP:") => (
            CheckStatus::Skipped,
            e.trim_start_matches("SKIP:").trim().to_string(),
        ),
        Err(e) => (CheckStatus::Fail, e),
    };
    reporter.finish(name, started, status, detail).await;
}

fn clip(s: &str) -> String {
    let s = s.trim();
    if s.len() > 120 {
        format!("{}…", &s[..120])
    } else {
        s.to_string()
    }
}
