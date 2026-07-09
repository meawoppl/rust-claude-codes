#![cfg(feature = "corpus-test")]

use claude_codes::ClaudeOutput;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct ParseFailure {
    path: PathBuf,
    line_number: usize,
    message_type: Option<String>,
    error: String,
    raw: String,
}

#[test]
fn parse_configured_claude_transcript_corpus() {
    let Some(root) = std::env::var_os("CLAUDE_CODES_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("CLAUDE_CODES_CORPUS_DIR is not set; skipping corpus test");
        return;
    };

    let files = jsonl_files(&root).expect("read corpus directory");
    assert!(
        !files.is_empty(),
        "no .jsonl files found under {}",
        root.display()
    );

    let mut total_lines = 0usize;
    let mut invalid_json = 0usize;
    let mut parsed_by_type: BTreeMap<String, usize> = BTreeMap::new();
    let mut failures = Vec::new();

    for path in files {
        scan_file(
            &path,
            &mut total_lines,
            &mut invalid_json,
            &mut parsed_by_type,
            &mut failures,
        )
        .unwrap_or_else(|e| panic!("scan {}: {}", path.display(), e));
    }

    eprintln!("corpus lines: {}", total_lines);
    eprintln!("invalid json lines skipped: {}", invalid_json);
    eprintln!("parsed message types:");
    for (message_type, count) in &parsed_by_type {
        eprintln!("  {}: {}", message_type, count);
    }

    if !failures.is_empty() {
        eprintln!("parse failures: {}", failures.len());
        for failure in failures.iter().take(25) {
            eprintln!(
                "{}:{} type={:?}: {}\n{}",
                failure.path.display(),
                failure.line_number,
                failure.message_type,
                failure.error,
                raw_preview(&failure.raw)
            );
        }
    }

    assert!(
        failures.is_empty(),
        "{} valid JSON corpus line(s) failed ClaudeOutput parsing",
        failures.len()
    );
}

fn jsonl_files(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(path) = stack.pop() {
        let meta = fs::metadata(&path)?;
        if meta.is_dir() {
            for entry in fs::read_dir(&path)? {
                stack.push(entry?.path());
            }
        } else if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

fn scan_file(
    path: &Path,
    total_lines: &mut usize,
    invalid_json: &mut usize,
    parsed_by_type: &mut BTreeMap<String, usize>,
    failures: &mut Vec<ParseFailure>,
) -> std::io::Result<()> {
    let file = File::open(path)?;
    for (idx, line) in BufReader::new(file).lines().enumerate() {
        let line = line?;
        let line_number = idx + 1;
        if line.trim().is_empty() {
            continue;
        }
        *total_lines += 1;

        let raw_value = match serde_json::from_str::<Value>(&line) {
            Ok(value) => value,
            Err(_) => {
                *invalid_json += 1;
                continue;
            }
        };

        let message_type = raw_value
            .get("type")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned);

        match ClaudeOutput::parse_json_tolerant(&line) {
            Ok(output) => {
                *parsed_by_type.entry(output.message_type()).or_default() += 1;
            }
            Err(error) => failures.push(ParseFailure {
                path: path.to_path_buf(),
                line_number,
                message_type,
                error: error.error_message,
                raw: line,
            }),
        }
    }
    Ok(())
}

fn raw_preview(raw: &str) -> String {
    const MAX_CHARS: usize = 500;

    let mut chars = raw.chars();
    let preview: String = chars.by_ref().take(MAX_CHARS).collect();
    let remaining = chars.count();

    if remaining == 0 {
        preview
    } else {
        format!("{preview}... <truncated {remaining} chars>")
    }
}

#[cfg(test)]
mod tests {
    use super::raw_preview;

    #[test]
    fn raw_preview_keeps_short_lines_unchanged() {
        assert_eq!(
            raw_preview(r#"{"type":"unknown"}"#),
            r#"{"type":"unknown"}"#
        );
    }

    #[test]
    fn raw_preview_truncates_long_lines_on_char_boundaries() {
        let raw = format!("{}{}", "a".repeat(500), "b".repeat(3));
        assert_eq!(
            raw_preview(&raw),
            format!("{}... <truncated 3 chars>", "a".repeat(500))
        );
    }
}
