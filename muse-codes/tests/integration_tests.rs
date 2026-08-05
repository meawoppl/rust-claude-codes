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
