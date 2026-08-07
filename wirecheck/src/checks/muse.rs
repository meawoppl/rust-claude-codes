//! Muse Code wire checks.
//!
//! Two tiers: the echo tier runs credential-free and pins the envelope
//! contract; the live tier (Meta provider) covers exactly what the nightly
//! echo-only drift fingerprint cannot — the provider-only vocabulary
//! (`tool.result`, `run.model.configured`, live task lifecycle) and the
//! tool-name ↔ `tool.<name>`-task correlation consumers key on.

use crate::state::{CheckStatus, Reporter};
use muse_codes::{ExecRun, MuseExecBuilder, MusePayload, MuseRecord, Provider};
use std::collections::{BTreeMap, BTreeSet};

pub async fn run_suite(reporter: Reporter) {
    let Some(version) = binary_check(&reporter).await else {
        return; // no binary — every other check is meaningless
    };
    let _ = version;

    let echo = echo_capture(&reporter).await;
    if let Some(records) = &echo {
        envelope_invariants(&reporter, records).await;
        session_id_adoption(&reporter, records).await;
    }
    record_id_counter_trap(&reporter).await;

    if muse_codes::auth::credentials_present() {
        live_meta_checks(&reporter).await;
    } else {
        let started = reporter
            .start(
                "meta_live",
                "live-provider vocabulary (typed audit, tool correlation, answer text)",
            )
            .await;
        reporter
            .finish(
                "meta_live",
                started,
                CheckStatus::Skipped,
                "no Meta credentials — log in above to enable the live tier".into(),
            )
            .await;
    }
}

async fn binary_check(reporter: &Reporter) -> Option<String> {
    let started = reporter.start("binary", "muse CLI present on PATH").await;
    let out = tokio::process::Command::new("muse")
        .arg("--version")
        .output()
        .await;
    match out {
        Ok(o) if o.status.success() => {
            let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
            reporter
                .finish("binary", started, CheckStatus::Pass, v.clone())
                .await;
            Some(v)
        }
        other => {
            reporter
                .finish(
                    "binary",
                    started,
                    CheckStatus::Fail,
                    format!("muse --version failed: {other:?}"),
                )
                .await;
            None
        }
    }
}

/// One echo run to its terminal, records collected. Reported as its own
/// check so a hang here is visible rather than blocking silently.
async fn echo_capture(reporter: &Reporter) -> Option<Vec<MuseRecord>> {
    let started = reporter
        .start(
            "echo_stream",
            "echo run reaches run.terminal.* and every line parses",
        )
        .await;
    match capture(
        MuseExecBuilder::new("wirecheck echo probe").provider(Provider::Echo),
        60,
    )
    .await
    {
        Ok(records) => {
            reporter
                .finish(
                    "echo_stream",
                    started,
                    CheckStatus::Pass,
                    format!("{} records to terminal", records.len()),
                )
                .await;
            Some(records)
        }
        Err(e) => {
            reporter
                .finish("echo_stream", started, CheckStatus::Fail, e)
                .await;
            None
        }
    }
}

/// Envelope contract: schema_version stable, sequence strictly increasing
/// within each stream, and every payload lifts into a typed variant.
async fn envelope_invariants(reporter: &Reporter, records: &[MuseRecord]) {
    let started = reporter
        .start(
            "envelope",
            "schema_version, per-stream sequence monotonicity, typed lift",
        )
        .await;
    let mut problems = Vec::new();
    let mut last_seq: BTreeMap<&str, u64> = BTreeMap::new();
    for r in records {
        if r.schema_version != 1 {
            problems.push(format!(
                "schema_version {} on {}",
                r.schema_version, r.payload_type
            ));
        }
        if let Some(prev) = last_seq.get(r.stream.id.as_str()) {
            if r.sequence <= *prev {
                problems.push(format!(
                    "sequence not increasing in stream {}: {} after {}",
                    r.stream.id, r.sequence, prev
                ));
            }
        }
        last_seq.insert(&r.stream.id, r.sequence);
        if let Err(e) = r.typed_payload() {
            problems.push(format!("{} failed typed lift: {e}", r.payload_type));
        }
    }
    finish_list(reporter, "envelope", started, problems, || {
        format!(
            "{} records clean across {} streams",
            records.len(),
            last_seq.len()
        )
    })
    .await;
}

/// A caller-supplied `--session-id` must be adopted verbatim as the session
/// `stream.id` — the property the composite persistence key rests on.
async fn session_id_adoption(reporter: &Reporter, prior: &[MuseRecord]) {
    let started = reporter
        .start(
            "session_identity",
            "--session-id adopted verbatim as stream.id",
        )
        .await;
    let supplied = uuid_v4();
    match capture(
        MuseExecBuilder::new("identity probe")
            .provider(Provider::Echo)
            .session_id(&supplied),
        60,
    )
    .await
    {
        Ok(records) => {
            let session_streams: BTreeSet<&str> = records
                .iter()
                .filter(|r| r.stream.kind == muse_codes::StreamKind::Session)
                .map(|r| r.stream.id.as_str())
                .collect();
            if session_streams.contains(supplied.as_str()) {
                reporter
                    .finish(
                        "session_identity",
                        started,
                        CheckStatus::Pass,
                        format!("stream.id == supplied id ({supplied})"),
                    )
                    .await;
            } else {
                reporter
                    .finish(
                        "session_identity",
                        started,
                        CheckStatus::Fail,
                        format!("supplied {supplied}, session streams: {session_streams:?}"),
                    )
                    .await;
            }
            let _ = prior;
        }
        Err(e) => {
            reporter
                .finish("session_identity", started, CheckStatus::Fail, e)
                .await;
        }
    }
}

/// Record ids are UUID-shaped COUNTERS: two fresh sessions must emit
/// byte-identical id sequences. If this ever fails, Meta fixed the counter
/// and the composite-key documentation should be revisited.
async fn record_id_counter_trap(reporter: &Reporter) {
    let started = reporter
        .start(
            "id_counter_trap",
            "record ids repeat byte-identical across sessions",
        )
        .await;
    let a = capture(MuseExecBuilder::new("ids a").provider(Provider::Echo), 60).await;
    let b = capture(MuseExecBuilder::new("ids b").provider(Provider::Echo), 60).await;
    match (a, b) {
        (Ok(a), Ok(b)) => {
            let ids_a: Vec<&str> = a.iter().map(|r| r.id.as_str()).collect();
            let ids_b: Vec<&str> = b.iter().map(|r| r.id.as_str()).collect();
            let overlap = ids_a.iter().filter(|id| ids_b.contains(id)).count();
            if overlap > 0 {
                reporter
                    .finish(
                        "id_counter_trap",
                        started,
                        CheckStatus::Pass,
                        format!(
                            "{overlap}/{} ids collide across sessions — ids are counters; \
                             composite (stream_id, id) keying remains required",
                            ids_a.len()
                        ),
                    )
                    .await;
            } else {
                reporter
                    .finish(
                        "id_counter_trap",
                        started,
                        CheckStatus::Fail,
                        "no cross-session id collisions: Meta may have fixed the counter — \
                         re-measure before relaxing any keying rules"
                            .into(),
                    )
                    .await;
            }
        }
        (a, b) => {
            reporter
                .finish(
                    "id_counter_trap",
                    started,
                    CheckStatus::Fail,
                    format!("capture failed: a={:?} b={:?}", a.is_ok(), b.is_ok()),
                )
                .await;
        }
    }
}

/// One live Meta tool-use turn evaluated for several properties at once —
/// one model turn, four checks, so the live tier stays cheap.
async fn live_meta_checks(reporter: &Reporter) {
    let started = reporter
        .start("meta_capture", "live Meta tool-use turn reaches terminal")
        .await;
    let dir = std::env::temp_dir().join(format!("wirecheck-muse-{}", uuid_v4()));
    if let Err(e) = std::fs::create_dir_all(&dir) {
        reporter
            .finish("meta_capture", started, CheckStatus::Fail, e.to_string())
            .await;
        return;
    }
    let records = match capture(
        MuseExecBuilder::new(
            "Use your file tools: create a file named probe.txt containing the word hello, \
             then read it back and reply with its contents.",
        )
        .provider(Provider::Meta)
        .working_directory(&dir),
        300,
    )
    .await
    {
        Ok(r) => {
            reporter
                .finish(
                    "meta_capture",
                    started,
                    CheckStatus::Pass,
                    format!("{} records", r.len()),
                )
                .await;
            r
        }
        Err(e) => {
            reporter
                .finish("meta_capture", started, CheckStatus::Fail, e)
                .await;
            return;
        }
    };

    // 1. Strict typed audit — the drift catcher the nightly can't reach.
    let started = reporter
        .start(
            "meta_typed_audit",
            "every live record lifts into a typed payload (no Unknown)",
        )
        .await;
    let mut unknown: BTreeSet<String> = BTreeSet::new();
    let mut failed: Vec<String> = Vec::new();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for r in &records {
        seen.insert(r.payload_type.as_str());
        match r.typed_payload() {
            Ok(MusePayload::Unknown { .. }) => {
                unknown.insert(r.payload_type.clone());
            }
            Ok(_) => {}
            Err(e) => failed.push(format!("{}: {e}", r.payload_type)),
        }
    }
    let mut problems: Vec<String> = unknown
        .iter()
        .map(|t| format!("untyped payload_type on the live wire: {t}"))
        .collect();
    problems.extend(failed);
    finish_list(reporter, "meta_typed_audit", started, problems, || {
        format!("{} payload types, all typed: {seen:?}", seen.len())
    })
    .await;

    // 2. Tool correlation — each tool.result names a tool whose
    //    `tool.<name>` task exists in the same turn.
    let started = reporter
        .start(
            "tool_correlation",
            "tool.result ↔ tool.<name> task kind-match holds live",
        )
        .await;
    let tool_tasks: BTreeSet<String> = records
        .iter()
        .filter_map(|r| {
            let ev = r.payload.get("event")?;
            (ev.get("kind")?.as_str()? == "proposed")
                .then(|| ev.get("task_kind")?.as_str().map(str::to_string))?
        })
        .filter(|k| k.starts_with("tool."))
        .collect();
    let results: Vec<&MuseRecord> = records
        .iter()
        .filter(|r| r.payload_type == "tool.result")
        .collect();
    let mut problems = Vec::new();
    if results.is_empty() {
        problems.push("model used no tools despite the prompt — rerun the suite".to_string());
    }
    for r in &results {
        let name = r
            .payload
            .get("correlation_facts")
            .and_then(|f| f.get("tool_name"))
            .and_then(|t| t.as_str());
        match name {
            Some(n) if tool_tasks.contains(&format!("tool.{n}")) => {}
            Some(n) => problems.push(format!("tool.result '{n}' has no matching tool.{n} task")),
            None => problems.push("tool.result without correlation_facts.tool_name".to_string()),
        }
    }
    finish_list(reporter, "tool_correlation", started, problems, || {
        format!(
            "{} tool results, all matched to {:?}",
            results.len(),
            tool_tasks
        )
    })
    .await;

    // 3. The answer is on the terminal record, non-empty.
    let started = reporter
        .start("answer_text", "run.terminal.* carries the reply text")
        .await;
    let text = records
        .iter()
        .filter(|r| r.payload_type.starts_with("run.terminal."))
        .filter_map(|r| r.payload.get("text").and_then(|t| t.as_str()))
        .next_back()
        .unwrap_or("");
    if text.trim().is_empty() {
        reporter
            .finish(
                "answer_text",
                started,
                CheckStatus::Fail,
                "terminal record has no text".into(),
            )
            .await;
    } else {
        reporter
            .finish(
                "answer_text",
                started,
                CheckStatus::Pass,
                format!("{} chars of reply text", text.len()),
            )
            .await;
    }

    let _ = std::fs::remove_dir_all(&dir);
}

// ── plumbing ─────────────────────────────────────────────────────────

async fn capture(builder: MuseExecBuilder, timeout_s: u64) -> Result<Vec<MuseRecord>, String> {
    let fut = async {
        let mut run = ExecRun::spawn(&builder).await.map_err(|e| e.to_string())?;
        let mut records = Vec::new();
        while let Some(record) = run.next_record().await.map_err(|e| e.to_string())? {
            let terminal = record.payload_type.starts_with("run.terminal.");
            records.push(record);
            if terminal {
                break;
            }
        }
        Ok::<_, String>(records)
    };
    tokio::time::timeout(std::time::Duration::from_secs(timeout_s), fut)
        .await
        .map_err(|_| format!("timed out after {timeout_s}s"))?
}

async fn finish_list(
    reporter: &Reporter,
    name: &'static str,
    started: std::time::Instant,
    problems: Vec<String>,
    pass_detail: impl FnOnce() -> String,
) {
    if problems.is_empty() {
        reporter
            .finish(name, started, CheckStatus::Pass, pass_detail())
            .await;
    } else {
        reporter
            .finish(name, started, CheckStatus::Fail, problems.join("; "))
            .await;
    }
}

/// Random v4-shaped id without a uuid dependency.
fn uuid_v4() -> String {
    let mut bytes = [0u8; 16];
    // getrandom via std: fill from a hasher over time+pid is NOT random
    // enough for crypto but fine for a probe session id.
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64
        ^ (std::process::id() as u64) << 32
        ^ SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
    let mut seed = nanos | 1;
    for b in bytes.iter_mut() {
        // xorshift64*
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        *b = (seed & 0xff) as u8;
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let h: Vec<String> = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}{}{}{}-{}{}-{}{}-{}{}-{}{}{}{}{}{}",
        h[0],
        h[1],
        h[2],
        h[3],
        h[4],
        h[5],
        h[6],
        h[7],
        h[8],
        h[9],
        h[10],
        h[11],
        h[12],
        h[13],
        h[14],
        h[15]
    )
}
