//! On-disk transcript locations for Claude Code sessions.
//!
//! The CLI persists every session's journal at
//! `~/.claude/projects/<encoded-cwd>/<session-id>.jsonl`. The encoding rule
//! is unpublished CLI behavior, measured against real transcript stores:
//! every `/` and `.` in the working directory's path becomes `-`; all other
//! characters pass through. Observed pairs:
//!
//! - `/home/u/repos/inboxnegative.com` → `-home-u-repos-inboxnegative-com`
//! - `/home/u/repos/x/.worktrees/y` → `-home-u-repos-x--worktrees-y`
//!
//! **The encoding is lossy** (not injective): distinct working directories
//! can collide — `a/b.c` and `a/b/c` and `a-b-c` all encode identically.
//! Treat encoded names as a lookup key for paths you already know, never as
//! something to decode.
//!
//! Exported so consumers stop growing private copies of the rule: a
//! consumer that re-implements it can silently diverge the day the CLI
//! changes, and two implementations of one CLI behavior is one too many.

use std::path::{Path, PathBuf};

/// Encode a working directory the way the CLI names its per-project
/// transcript folder: `/` and `.` become `-`, everything else unchanged.
pub fn encode_project_dir(working_directory: &Path) -> String {
    working_directory
        .to_string_lossy()
        .chars()
        .map(|c| if c == '/' || c == '.' { '-' } else { c })
        .collect()
}

/// Resolve `<home>/.claude/projects/<encoded-cwd>/<session-id>.jsonl` —
/// the transcript file the CLI writes for `session_id` runs in
/// `working_directory`. Takes `home` explicitly (pass a tempdir in tests;
/// never let a test path resolve into a real transcript store).
pub fn transcript_path(
    home: &Path,
    working_directory: &Path,
    session_id: impl AsRef<str>,
) -> PathBuf {
    home.join(".claude")
        .join("projects")
        .join(encode_project_dir(working_directory))
        .join(format!("{}.jsonl", session_id.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pairs observed verbatim in a real ~/.claude/projects store.
    #[test]
    fn encoding_matches_observed_cli_behavior() {
        for (dir, expect) in [
            (
                "/home/u/repos/rust-code-agent-sdks",
                "-home-u-repos-rust-code-agent-sdks",
            ),
            (
                "/home/u/repos/inboxnegative.com",
                "-home-u-repos-inboxnegative-com",
            ),
            (
                "/home/u/repos/x/.worktrees/y",
                "-home-u-repos-x--worktrees-y",
            ),
            ("/tmp/reset-probe", "-tmp-reset-probe"),
        ] {
            assert_eq!(encode_project_dir(Path::new(dir)), expect);
        }
    }

    #[test]
    fn transcript_path_assembles_under_the_given_home() {
        let p = transcript_path(Path::new("/fake/home"), Path::new("/work/dir.x"), "abc-123");
        assert_eq!(
            p,
            Path::new("/fake/home/.claude/projects/-work-dir-x/abc-123.jsonl")
        );
    }

    /// The rule is lossy — document-by-test so nobody builds a decoder.
    #[test]
    fn encoding_is_lossy_by_design() {
        assert_eq!(
            encode_project_dir(Path::new("/a/b.c")),
            encode_project_dir(Path::new("/a/b/c")),
        );
    }
}
