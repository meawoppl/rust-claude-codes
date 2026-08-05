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
