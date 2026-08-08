//! Replays captured sessions through [`StepAssembler`].
//!
//! The unit tests in `steps.rs` feed it hand-built updates, which proves the
//! accumulation arithmetic but not that the arithmetic matches how a real
//! harness actually streams. These tests replay whole recorded sessions —
//! frames in the order they arrived — and assert the invariants that consumers
//! rely on.

#![cfg(feature = "async-client")]

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use antigravity_codes::protocol::{OutputEvent, StepUpdateState};
use antigravity_codes::steps::StepAssembler;

/// Every replayable session, ordered within itself by the harness's own
/// sequence number.
///
/// Two sources. `test_cases/events/` holds frames captured verbatim, grouped
/// into sessions by filename prefix. `test_cases/synthetic/subagent-session/`
/// is hand-built: the free-tier quota would not sustain a real delegating turn
/// long enough to record one, so that session is modelled on the shape of one
/// that *was* observed live — main conversation on a 32-hex cascade id, the
/// subagent on its own UUID trajectory, both numbering steps from zero.
fn sessions() -> BTreeMap<String, Vec<OutputEvent>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test_cases");
    let mut out: BTreeMap<String, Vec<(i64, OutputEvent)>> = BTreeMap::new();

    let mut collect = |dir: &Path, name_of: &dyn Fn(&str) -> String| {
        for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let path = entry.expect("readable dir entry").path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            let file = path.file_name().unwrap().to_string_lossy().to_string();
            let raw = std::fs::read_to_string(&path).expect("readable fixture");
            let event: OutputEvent =
                serde_json::from_str(&raw).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            out.entry(name_of(&file))
                .or_default()
                .push((event.seq_num.unwrap_or(0), event));
        }
    };

    collect(&root.join("events"), &|file| {
        format!(
            "captured-{}",
            file.chars().next().expect("fixtures are prefixed")
        )
    });
    collect(&root.join("synthetic/subagent-session"), &|_| {
        "synthetic-subagent".to_string()
    });

    out.into_iter()
        .map(|(session, mut frames)| {
            frames.sort_by_key(|(seq, _)| *seq);
            (
                session,
                frames.into_iter().map(|(_, event)| event).collect(),
            )
        })
        .collect()
}

#[test]
fn every_session_replays_without_losing_text() {
    for (session, frames) in sessions() {
        let mut assembler = StepAssembler::new(None);
        // Longest text seen per step, to prove accumulation never goes backwards.
        let mut high_water: HashMap<String, usize> = HashMap::new();

        for event in frames {
            let Some(update) = event.step_update else {
                continue;
            };
            let step = assembler.ingest(update);
            let seen = high_water.entry(step.id()).or_default();
            assert!(
                step.text.chars().count() >= *seen,
                "session {session}: step {} lost text ({} -> {})",
                step.id(),
                seen,
                step.text.chars().count()
            );
            *seen = step.text.chars().count();
        }
        assert!(
            !high_water.is_empty(),
            "session {session} produced no steps"
        );
    }
}

#[test]
fn a_settled_step_is_final_and_keeps_its_text() {
    let mut settled = 0;
    for (session, frames) in sessions() {
        let mut assembler = StepAssembler::new(None);
        for event in frames {
            let Some(update) = event.step_update else {
                continue;
            };
            let state = update.state.clone();
            let step = assembler.ingest(update);
            if matches!(state, Some(StepUpdateState::Done)) {
                assert!(
                    step.is_final(),
                    "session {session}: STATE_DONE step is not final"
                );
                settled += 1;
            }
        }
    }
    assert!(settled > 0, "the corpus should contain settled steps");
}

/// A delegating agent runs its subagent on a *different* trajectory, so any
/// bookkeeping keyed on `step_index` alone would collide. This replays the
/// session with the most trajectories and asserts they stay separate.
#[test]
fn concurrent_trajectories_stay_separate() {
    let sessions = sessions();
    let (session, frames) = sessions
        .iter()
        .map(|(session, frames)| {
            let trajectories: std::collections::HashSet<_> = frames
                .iter()
                .filter_map(|e| e.step_update.as_ref()?.trajectory_id.clone())
                .collect();
            (session, frames, trajectories.len())
        })
        .max_by_key(|(_, _, count)| *count)
        .map(|(session, frames, _)| (session, frames))
        .expect("the corpus has at least one session");

    let mut assembler = StepAssembler::new(None);
    let mut per_trajectory: HashMap<String, usize> = HashMap::new();
    for event in frames {
        let Some(update) = event.step_update.clone() else {
            continue;
        };
        let step = assembler.ingest(update);
        *per_trajectory
            .entry(step.trajectory_id.clone())
            .or_default() += 1;
    }

    assert!(
        per_trajectory.len() >= 2,
        "session {session} should carry a subagent on its own trajectory, saw {:?}",
        per_trajectory.keys()
    );

    // Exactly one trajectory is the conversation; the rest are subagents. Only
    // the main one ending should end the turn.
    let main = assembler
        .main_trajectory()
        .expect("a main trajectory was adopted");
    let subagents = per_trajectory
        .keys()
        .filter(|id| id.as_str() != main)
        .count();
    assert_eq!(subagents, per_trajectory.len() - 1);
    assert!(assembler.is_main(main));
}

/// Step indices restart per trajectory, so `(trajectory, index)` is the only
/// safe key — this asserts the corpus actually exhibits the collision that
/// keying on the index alone would hit.
#[test]
fn step_indices_collide_across_trajectories() {
    let mut collided = false;
    for (_, frames) in sessions() {
        let mut by_index: HashMap<u32, std::collections::HashSet<String>> = HashMap::new();
        for event in &frames {
            if let Some(update) = event.step_update.as_ref() {
                if let (Some(index), Some(trajectory)) =
                    (update.step_index, update.trajectory_id.clone())
                {
                    by_index.entry(index).or_default().insert(trajectory);
                }
            }
        }
        if by_index.values().any(|t| t.len() > 1) {
            collided = true;
        }
    }
    assert!(
        collided,
        "no session reuses a step index across trajectories, so \
         `concurrent_trajectories_stay_separate` is not actually exercising the \
         collision it claims to — check that the subagent session is present"
    );
}
