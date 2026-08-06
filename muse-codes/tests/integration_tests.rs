//! Live integration tests against an installed `muse` binary.
//!
//! Gated behind the `integration-tests` feature. Uses the credential-free
//! echo provider, so these run anywhere Muse Code is installed — no
//! login or META_API_KEY required.

#![cfg(feature = "integration-tests")]

use muse_codes::{ExecRun, MuseExecBuilder, MusePayload, Provider};

/// Full headless run through the typed client: spawn → stream records →
/// terminal. Asserts the prompt echoes through the event stream and the
/// run closes with terminal="completed".
#[tokio::test]
async fn echo_run_streams_to_terminal() {
    let run = ExecRun::spawn(
        &MuseExecBuilder::new("muse-codes integration probe")
            .provider(Provider::Echo)
            .working_directory(std::env::temp_dir()),
    )
    .await
    .expect("muse installed and spawnable");

    let mut saw_prompt = false;
    let mut saw_delta = false;
    let mut records = 0usize;
    let terminal = run
        .wait_terminal(|record| {
            records += 1;
            match record.typed_payload().expect("every payload types") {
                MusePayload::TurnInputUser(t) => {
                    saw_prompt = t.prompt.contains("integration probe");
                }
                MusePayload::RunOutputDelta(d) => {
                    saw_delta = saw_delta || !d.text.is_empty();
                }
                _ => {}
            }
        })
        .await
        .expect("run reaches terminal");

    assert_eq!(terminal.terminal, "completed");
    assert!(saw_prompt, "turn.input.user should carry the prompt");
    assert!(saw_delta, "run.output.delta should stream text");
    assert!(
        records >= 20,
        "expected a full journal, saw {records} records"
    );
}

/// Device login flow: real `muse login` prints a verification URL + code
/// on plain stdout. Start, extract, cancel — no approval needed.
#[tokio::test]
async fn device_login_yields_url_and_code_live() {
    use muse_codes::auth::DeviceLoginFlow;
    use std::time::Duration;

    let mut flow = DeviceLoginFlow::start().await.expect("muse spawns");
    let dc = flow
        .device_code(Duration::from_secs(20))
        .await
        .expect("device code appears");
    assert!(dc.verification_url.starts_with("https://"));
    assert!(dc.code.contains('-'), "code shape: {}", dc.code);
    assert!(
        dc.verification_url.contains(&dc.code),
        "URL should embed the code"
    );
    flow.cancel().await.expect("cancel");
}

/// auth_set/logout round-trip in an isolated HOME so real credentials are
/// never touched: save a dummy key, see the file appear, log out, gone.
#[tokio::test]
async fn auth_set_and_logout_roundtrip_in_sandbox_home() {
    let sandbox = std::env::temp_dir().join(format!("muse-auth-sandbox-{}", std::process::id()));
    std::fs::create_dir_all(&sandbox).unwrap();
    // Child processes inherit our env; override HOME only for them.
    // muse resolves ~/.config/muse relative to HOME.
    let orig_home = std::env::var_os("HOME");
    // SAFETY-free approach: spawn with explicit env instead of mutating ours.
    let muse = which::which("muse").expect("muse on PATH");
    let child = tokio::process::Command::new(&muse)
        .args(["auth", "set", "--provider", "meta", "--api-key-stdin"])
        .env("HOME", &sandbox)
        .env_remove("XDG_CONFIG_HOME")
        .stdin(std::process::Stdio::piped())
        .output_stdin(b"meta-dummy-key-not-real\n")
        .await;
    drop(orig_home);
    let out = child.expect("auth set runs");
    assert!(
        out.status.success(),
        "auth set failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let auth_file = sandbox.join(".config/muse/auth.json");
    assert!(auth_file.exists(), "credential file should appear");
    let saved: muse_codes::auth::AuthFile =
        serde_json::from_str(&std::fs::read_to_string(&auth_file).unwrap()).unwrap();
    assert_eq!(
        saved.providers["meta"].api_key.as_deref(),
        Some("meta-dummy-key-not-real")
    );

    let out = tokio::process::Command::new(&muse)
        .arg("logout")
        .env("HOME", &sandbox)
        .env_remove("XDG_CONFIG_HOME")
        .output()
        .await
        .expect("logout runs");
    assert!(out.status.success());
    // logout empties the providers map but keeps the file.
    let post: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&auth_file).unwrap()).unwrap();
    assert_eq!(post["providers"], serde_json::json!({}));
    std::fs::remove_dir_all(&sandbox).ok();
}

/// Helper: run a command feeding stdin then collecting output.
trait StdinExt {
    async fn output_stdin(&mut self, input: &[u8]) -> std::io::Result<std::process::Output>;
}

impl StdinExt for tokio::process::Command {
    async fn output_stdin(&mut self, input: &[u8]) -> std::io::Result<std::process::Output> {
        use tokio::io::AsyncWriteExt;
        let mut child = self
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input).await?;
        }
        child.wait_with_output().await
    }
}

/// Multi-turn continuity: two turns under one caller-supplied session id.
/// The id is adopted verbatim as `stream.id`, and — the trap this test
/// exists to pin — `sequence` RESTARTS rather than continuing, so it must
/// never be used as a cross-turn key.
#[tokio::test]
async fn session_id_continuity_and_sequence_reuse() {
    use muse_codes::{ExecRun, MuseExecBuilder, Provider};
    use std::time::Duration;

    let session = uuid_v4_like();
    let mut streams = Vec::new();
    let mut first_seqs = Vec::new();
    let mut ids = Vec::new();

    for turn in ["first turn", "second turn"] {
        let run = ExecRun::spawn(
            &MuseExecBuilder::new(turn)
                .provider(Provider::Echo)
                .session_id(&session)
                .working_directory(std::env::temp_dir()),
        )
        .await
        .expect("spawn");
        let mut seqs = Vec::new();
        let mut turn_ids = Vec::new();
        let terminal = run
            .wait_terminal(|r| {
                seqs.push(r.sequence);
                turn_ids.push(r.id.clone());
                streams.push(r.stream.id.clone());
            })
            .await
            .expect("terminal");
        assert_eq!(terminal.terminal, "completed");
        first_seqs.push(*seqs.first().expect("records"));
        ids.push(turn_ids);
    }

    assert!(
        streams.iter().all(|s| *s == session),
        "the caller-supplied session id must be adopted verbatim as stream.id"
    );
    // Both turns start near sequence 1 rather than the second continuing
    // from the first — the collision this test pins.
    assert!(
        first_seqs[1] <= 3,
        "sequence restarts per turn (got {first_seqs:?}); never key across turns on it"
    );
    // Within one session, ids keep incrementing — no repeats across turns.
    let within_session_overlap = ids[0].iter().filter(|i| ids[1].contains(i)).count();
    assert_eq!(
        within_session_overlap, 0,
        "record ids should not repeat across turns of the SAME session"
    );
    let _ = Duration::from_secs(0);
}

/// The identity trap, pinned: record ids are UUID-*shaped counters* that
/// restart per session, so two DIFFERENT sessions emit the same ids. Keying
/// on `id` alone would collide across every session; only the composite
/// `(stream.id, id)` is safe.
#[tokio::test]
async fn record_ids_repeat_across_sessions() {
    use muse_codes::{ExecRun, MuseExecBuilder, Provider};

    let mut runs: Vec<Vec<String>> = Vec::new();
    for _ in 0..2 {
        let run = ExecRun::spawn(
            &MuseExecBuilder::new("same prompt, different session")
                .provider(Provider::Echo)
                .session_id(uuid_v4_like())
                .working_directory(std::env::temp_dir()),
        )
        .await
        .expect("spawn");
        let mut ids = Vec::new();
        run.wait_terminal(|r| ids.push(r.id.clone()))
            .await
            .expect("terminal");
        runs.push(ids);
    }
    let overlap = runs[0].iter().filter(|i| runs[1].contains(i)).count();
    assert!(
        overlap > 0,
        "record ids are expected to REPEAT across sessions (UUID-shaped counters). \
         If this ever fails they may have become globally unique — re-check before \
         relaxing any (stream_id, id) composite key."
    );
    assert!(
        runs[0][0].starts_with("018f0000-"),
        "the counter shape is load-bearing context for the composite-key rule: {}",
        runs[0][0]
    );
}

/// Interrupt-as-kill: SIGKILL a run before it emits anything, then run the
/// same session id again. The store must survive. Kept at a pre-first-line
/// window on purpose — a later kill would pass vacuously because an echo
/// run completes in ~250ms.
#[tokio::test]
async fn kill_midflight_then_resume_same_session() {
    use muse_codes::{ExecRun, MuseExecBuilder, Provider};
    use std::time::Duration;

    let session = uuid_v4_like();
    let mut victim = ExecRun::spawn(
        &MuseExecBuilder::new("this run gets killed")
            .provider(Provider::Echo)
            .session_id(&session)
            .working_directory(std::env::temp_dir()),
    )
    .await
    .expect("spawn victim");
    tokio::time::sleep(Duration::from_millis(60)).await;
    victim.kill().await.expect("kill");
    drop(victim);

    let run = ExecRun::spawn(
        &MuseExecBuilder::new("this run must still work")
            .provider(Provider::Echo)
            .session_id(&session)
            .working_directory(std::env::temp_dir()),
    )
    .await
    .expect("spawn after kill");
    let terminal = run
        .wait_terminal(|_| {})
        .await
        .expect("session store survives a mid-flight kill");
    assert_eq!(terminal.terminal, "completed");
}

/// Cheap v4-shaped id without pulling in the uuid crate for tests.
fn uuid_v4_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!(
        "{:08x}-{:04x}-4{:03x}-8{:03x}-{:012x}",
        (n & 0xffff_ffff) as u32,
        ((n >> 32) & 0xffff) as u16,
        ((n >> 48) & 0xfff) as u16,
        ((n >> 60) & 0xfff) as u16,
        (n & 0xffff_ffff_ffff) as u64
    )
}
