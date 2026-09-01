#!/usr/bin/env python3
"""Integration-test annotation check and description extractor.

Every integration test function (any `#[test]` / `#[tokio::test]` fn in a
crate's `tests/*.rs`) must carry a `///` doc comment saying what the test
pins, in human terms. Those doc comments are the single source of truth:
this script both ENFORCES them in CI (`--check`) and EXTRACTS them
(`--emit-json`) for consumers like wirecheck, which shows the description
next to each test result so humans and models get real feedback instead
of a bare function name.

Usage:
    check_test_annotations.py --check        # CI gate: exit 1 on gaps
    check_test_annotations.py --emit-json    # {crate: {file: {fn: desc}}}

Only `tests/*.rs` is enforced; in-source `#[cfg(test)]` unit tests are
exempt (their names render without descriptions in wirecheck).
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CRATES = ["claude-codes", "codex-codes", "muse-codes", "opencode-codes", "pi-codes"]

TEST_ATTR = re.compile(r"#\[(?:tokio::)?test(?:\([^)]*\))?\]")
FN_NAME = re.compile(r"(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z0-9_]+)")


def scan_file(path: Path) -> tuple[dict[str, str], list[str]]:
    """Return ({fn_name: description}, [fn_names_missing_docs])."""
    lines = path.read_text().splitlines()
    docs: dict[str, str] = {}
    missing: list[str] = []
    i = 0
    while i < len(lines):
        if not TEST_ATTR.search(lines[i]):
            i += 1
            continue
        # Walk back over contiguous attributes and doc lines to collect the
        # /// block that precedes the attribute stack.
        j = i - 1
        doc_lines: list[str] = []
        while j >= 0:
            stripped = lines[j].strip()
            if stripped.startswith("///"):
                doc_lines.insert(0, stripped.lstrip("/").strip())
                j -= 1
            elif stripped.startswith("#[") or stripped == "":
                # Attributes above the test attr (e.g. #[ignore]); blank
                # lines end the doc block search unless docs already found.
                if stripped == "" and doc_lines:
                    break
                j -= 1
            else:
                break
        # Walk forward past any further attributes to the fn itself.
        k = i + 1
        while k < len(lines) and not FN_NAME.search(lines[k]):
            k += 1
        if k >= len(lines):
            break
        name_match = FN_NAME.search(lines[k])
        if name_match:
            name = name_match.group(1)
            description = " ".join(line for line in doc_lines if line).strip()
            if description:
                docs[name] = description
            else:
                missing.append(name)
        i = k + 1
    return docs, missing


def scan() -> tuple[dict, list[str]]:
    emitted: dict[str, dict[str, dict[str, str]]] = {}
    problems: list[str] = []
    for crate in CRATES:
        tests_dir = ROOT / crate / "tests"
        if not tests_dir.is_dir():
            continue
        for path in sorted(tests_dir.glob("*.rs")):
            docs, missing = scan_file(path)
            rel = f"{crate}/tests/{path.name}"
            if docs:
                emitted.setdefault(crate, {})[path.stem] = docs
            problems.extend(f"{rel}: fn {name}" for name in missing)
    return emitted, problems


def main() -> int:
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--emit-json", action="store_true")
    args = parser.parse_args()

    emitted, problems = scan()
    if args.emit_json:
        json.dump(emitted, sys.stdout, indent=1, sort_keys=True)
        return 0

    total = sum(len(fns) for files in emitted.values() for fns in files.values())
    if problems:
        print(f"❌ {len(problems)} integration test(s) missing /// descriptions:")
        for p in problems:
            print(f"  - {p}")
        print()
        print("Add a /// doc comment saying what the test pins — wirecheck and")
        print("humans read it next to the result.")
        return 1
    print(f"✅ all {total} integration tests carry /// descriptions")
    return 0


if __name__ == "__main__":
    sys.exit(main())
