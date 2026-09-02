#!/usr/bin/env python3
"""
Detect drift between the Claude Code CLI's stream-json wire schemas and the
snapshot committed at `claude-codes/tests/schemas/claude_stream_json_snapshot.txt`.

Unlike the Codex app-server (which publishes JSON Schemas we fetch directly),
Claude Code ships no schema — the wire types live as minified zod definitions
inside the Bun-compiled CLI ELF. `scripts/extract_claude_sdk_schemas.py` pulls
them out; this script reduces them to a **minification-invariant fingerprint**
and diffs it against the committed snapshot.

## What is (and isn't) compared

The minified schema *variable* names churn every CLI release, so we key on the
stable parts only:

  - the set of wire **labels** — a message's `type` literal, or `type/subtype`
    for `system` frames, and
  - each labeled schema's **top-level field keys** (the object keys are real
    wire names; nested objects and `.describe()` prose are ignored).

This catches the highest-severity drift — a new/removed message `type`
(which fails `ClaudeOutput`'s tagged-enum decode outright) and added/removed
top-level fields — with no false positives from minification. It intentionally
does **not** diff deep field types or required/optional flips; for those, run
the full extractor and follow `claude-codes/RESNAPSHOTTING.md`.

## Usage

    python3 scripts/check_claude_schema_drift.py [--binary PATH]   # check
    python3 scripts/check_claude_schema_drift.py --update [--binary PATH]  # re-snapshot

## Exit codes

  0 — no drift (or --update wrote the snapshot)
  1 — drift detected; the Markdown report on stdout says what changed
  2 — could not extract (no `claude` binary, or the bundle layout changed so
      the anchor/union moved — treated as a soft skip by CI, like a fetch blip)
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from extract_claude_sdk_schemas import (  # noqa: E402
    REF_CALL,
    SchemaExtractionError,
    extract_schemas,
    resolve_binary,
)

ROOT = Path(__file__).resolve().parent.parent
SNAPSHOT = ROOT / "claude-codes" / "tests" / "schemas" / "claude_stream_json_snapshot.txt"

_KEY = re.compile(r"[A-Za-z_$][\w$]*")


def top_level_keys(body: str, opener: str) -> list[str]:
    """The direct object keys of a schema's outermost object combinator.

    `opener` is the exact object-schema opening text the extractor discovered
    from the binary (`E.object({` on CLI 2.1.205, `_e({` on 2.1.239) — it
    changes between releases.
    String-literal aware (so colons/braces inside `.describe("...")` prose are
    skipped) and bracket-depth aware (so nested object combinator keys aren't
    counted). Backtick template literals are treated as plain strings — the
    bundle's describe() prose uses them (interpolation-free) from CLI 2.1.258.
    Only bare-identifier keys are recognized — all Claude wire schemas use
    them.
    """
    start = body.find(opener)
    if start < 0:
        return []
    i = start + len(opener)
    depth = 0
    n = len(body)
    keys: list[str] = []
    while i < n:
        c = body[i]
        if c in "\"'`":
            quote = c
            i += 1
            while i < n:
                if body[i] == "\\":
                    i += 2
                    continue
                if body[i] == quote:
                    i += 1
                    break
                i += 1
            continue
        if c in "{[(":
            depth += 1
            i += 1
            continue
        if c in "}])":
            if depth == 0:
                break  # closed the outermost object
            depth -= 1
            i += 1
            continue
        if depth == 0 and (c.isalpha() or c in "_$"):
            m = _KEY.match(body, i)
            j = m.end()
            while j < n and body[j] in " \t\n":
                j += 1
            if j < n and body[j] == ":":
                keys.append(m.group(0))
                i = j + 1
                continue
            i = m.end()
            continue
        i += 1
    return keys


def build_fingerprint(data: bytes) -> tuple[int, dict[str, list[str]]]:
    """`(union_member_count, {label: sorted top-level keys})` for labeled schemas."""
    extraction = extract_schemas(data)
    members = len(REF_CALL.findall(extraction.union_text))
    labeled: dict[str, list[str]] = {}
    for _name, label, body in extraction.results:
        if label in ("(nested)", "NOT FOUND"):
            continue
        keys = sorted(set(top_level_keys(body, extraction.object_open)))
        # First definition wins on the rare label collision; keep it deterministic.
        labeled.setdefault(label, keys)
    return members, labeled


def render_snapshot(members: int, labeled: dict[str, list[str]]) -> str:
    lines = [
        "# Claude Code stream-json wire schema snapshot.",
        "# Regenerate: python3 scripts/check_claude_schema_drift.py --update",
        "# One line per wire label: `<type[/subtype]>: comma-separated top-level field keys`.",
        "# Minified schema names and nested/describe() detail are intentionally omitted;",
        "# see the header of check_claude_schema_drift.py for the comparison contract.",
        f"# union_members: {members}",
        "",
    ]
    for label in sorted(labeled):
        lines.append(f"{label}: {', '.join(labeled[label])}")
    return "\n".join(lines) + "\n"


def parse_snapshot(text: str) -> tuple[int | None, dict[str, list[str]]]:
    members: int | None = None
    labeled: dict[str, list[str]] = {}
    for line in text.splitlines():
        line = line.rstrip()
        if not line:
            continue
        if line.startswith("#"):
            m = re.match(r"#\s*union_members:\s*(\d+)", line)
            if m:
                members = int(m.group(1))
            continue
        label, _, keys = line.partition(":")
        labeled[label.strip()] = [k.strip() for k in keys.split(",") if k.strip()]
    return members, labeled


def diff_report(
    old_members: int | None,
    old: dict[str, list[str]],
    new_members: int,
    new: dict[str, list[str]],
) -> tuple[bool, str]:
    """`(drift, markdown_report)`."""
    lines = ["# Claude Code stream-json schema drift report", ""]
    lines.append("Comparing the installed `claude` CLI against")
    lines.append("`claude-codes/tests/schemas/claude_stream_json_snapshot.txt`.")
    lines.append("")

    drift = False
    added = sorted(set(new) - set(old))
    removed = sorted(set(old) - set(new))
    common = sorted(set(old) & set(new))

    if old_members is not None and old_members != new_members:
        drift = True
        lines.append(
            f"- ⚠️ **SDK output union member count changed**: {old_members} → {new_members}"
        )
        lines.append("")

    if added:
        drift = True
        lines.append(f"### New wire labels ({len(added)})")
        lines.append("")
        for label in added:
            keys = ", ".join(new[label]) or "(no top-level keys)"
            lines.append(f"- `{label}` — fields: {keys}")
        lines.append("")

    if removed:
        drift = True
        lines.append(f"### Removed wire labels ({len(removed)})")
        lines.append("")
        for label in removed:
            lines.append(f"- `{label}`")
        lines.append("")

    field_changes = []
    for label in common:
        a, b = set(old[label]), set(new[label])
        if a != b:
            field_changes.append((label, sorted(b - a), sorted(a - b)))
    if field_changes:
        drift = True
        lines.append(f"### Changed top-level fields ({len(field_changes)})")
        lines.append("")
        for label, gained, lost in field_changes:
            parts = []
            if gained:
                parts.append("added " + ", ".join(f"`{k}`" for k in gained))
            if lost:
                parts.append("removed " + ", ".join(f"`{k}`" for k in lost))
            lines.append(f"- `{label}` — {'; '.join(parts)}")
        lines.append("")

    if not drift:
        lines.append("- ✅ No drift: wire labels and top-level fields match the snapshot.")
        lines.append("")

    lines.append("---")
    lines.append("")
    lines.append("Regenerate the snapshot + investigate deeper changes with:")
    lines.append("")
    lines.append("```bash")
    lines.append("python3 scripts/extract_claude_sdk_schemas.py -o /tmp/claude_schemas.txt")
    lines.append("python3 scripts/check_claude_schema_drift.py --update  # accept new snapshot")
    lines.append("# then follow claude-codes/RESNAPSHOTTING.md for the crate-side diff")
    lines.append("```")
    return drift, "\n".join(lines) + "\n"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--binary", help="path to the claude CLI ELF (default: `claude` on PATH)")
    ap.add_argument("--update", action="store_true", help="(re)write the committed snapshot")
    args = ap.parse_args()

    try:
        binary = resolve_binary(args.binary)
        data = binary.read_bytes()
        new_members, new = build_fingerprint(data)
    except SchemaExtractionError as e:
        print(f"# Claude schema drift check skipped\n\nCould not extract schemas: {e}")
        return 2
    except (FileNotFoundError, SystemExit) as e:
        print(f"# Claude schema drift check skipped\n\nCould not read the CLI binary: {e}")
        return 2

    if args.update:
        SNAPSHOT.parent.mkdir(parents=True, exist_ok=True)
        SNAPSHOT.write_text(render_snapshot(new_members, new))
        print(f"# wrote {len(new)} labels ({new_members} union members) to {SNAPSHOT}")
        return 0

    if not SNAPSHOT.exists():
        print(f"error: snapshot missing at {SNAPSHOT}; run with --update to create it")
        return 2

    old_members, old = parse_snapshot(SNAPSHOT.read_text())
    drift, report = diff_report(old_members, old, new_members, new)
    print(report)
    return 1 if drift else 0


if __name__ == "__main__":
    sys.exit(main())
