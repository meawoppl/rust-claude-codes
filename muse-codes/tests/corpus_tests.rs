//! Corpus tests: every committed capture line must parse, type, and
//! round-trip byte-faithfully.
//!
//! The captures in `test_cases/` are real `muse exec --json` output
//! (credential-free `--provider echo` runs against Muse Code 0.1.0), so
//! these assertions run against the actual wire, not examples.

use muse_codes::{MusePayload, MuseRecord, RecordType, StreamKind};
use serde_json::Value;
use std::path::PathBuf;

fn corpus_files() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_cases");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("test_cases dir")
        .filter_map(|e| {
            let p = e.expect("entry").path();
            (p.extension().and_then(|x| x.to_str()) == Some("jsonl")).then_some(p)
        })
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no corpus files found");
    files
}

/// Envelope + payload round-trip: parse → reserialize → Value-compare.
#[test]
fn every_corpus_line_roundtrips() {
    for file in corpus_files() {
        for (n, line) in std::fs::read_to_string(&file).unwrap().lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let original: Value = serde_json::from_str(line).unwrap();
            let record: MuseRecord = serde_json::from_str(line).unwrap_or_else(|e| {
                panic!("{}:{}: envelope parse failed: {e}", file.display(), n + 1)
            });
            let back = serde_json::to_value(&record).unwrap();
            assert_eq!(
                original,
                back,
                "{}:{}: round-trip drift",
                file.display(),
                n + 1
            );
        }
    }
}

/// Every observed payload type must lift into a KNOWN typed variant —
/// an Unknown here means the corpus outgrew the models.
#[test]
fn every_corpus_payload_is_typed() {
    let mut kinds = std::collections::BTreeSet::new();
    for file in corpus_files() {
        for (n, line) in std::fs::read_to_string(&file).unwrap().lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let record: MuseRecord = serde_json::from_str(line).unwrap();
            let payload = record.typed_payload().unwrap_or_else(|e| {
                panic!(
                    "{}:{}: payload {} failed typed parse: {e}",
                    file.display(),
                    n + 1,
                    record.payload_type
                )
            });
            assert!(
                !matches!(payload, MusePayload::Unknown { .. }),
                "{}:{}: corpus payload type {} is unmodeled",
                file.display(),
                n + 1,
                record.payload_type
            );
            kinds.insert(record.payload_type.clone());
        }
    }
    // The echo corpus exercises the full headless vocabulary observed on
    // Muse Code 0.1.0. Growth here is fine; shrinkage means captures were
    // lost.
    assert!(
        kinds.len() >= 14,
        "corpus vocabulary shrank: {} payload types: {kinds:?}",
        kinds.len()
    );
}

/// Structural invariants of the journal envelope, verified corpus-wide.
#[test]
fn envelope_invariants_hold() {
    for file in corpus_files() {
        let mut last_seq = 0u64;
        for line in std::fs::read_to_string(&file).unwrap().lines() {
            if line.trim().is_empty() {
                continue;
            }
            let r: MuseRecord = serde_json::from_str(line).unwrap();
            assert_eq!(r.schema_version, muse_codes::version::STREAM_SCHEMA_VERSION);
            assert_eq!(
                r.stream.kind,
                StreamKind::Session,
                "exec stream is session-scoped"
            );
            assert!(
                r.sequence > last_seq,
                "sequence must be strictly increasing"
            );
            last_seq = r.sequence;
            // Ephemeral records are status-class in every observed frame.
            if r.durability == muse_codes::Durability::Ephemeral {
                assert_eq!(r.record_type, RecordType::Status);
            }
        }
    }
}

/// Terminal records close every captured run, with terminal="completed".
#[test]
fn corpus_runs_reach_terminal() {
    for file in corpus_files() {
        let content = std::fs::read_to_string(&file).unwrap();
        let last = content
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .unwrap();
        let r: MuseRecord = serde_json::from_str(last).unwrap();
        match r.typed_payload().unwrap() {
            MusePayload::RunTerminal(t) => {
                assert_eq!(t.terminal, "completed", "{}", file.display());
                assert!(t.reason.is_none());
            }
            other => panic!("{}: last record is {other:?}, not terminal", file.display()),
        }
    }
}
