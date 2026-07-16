#!/usr/bin/env python3
"""
Extract the Claude Code CLI's SDK stream-json output schemas from the
compiled CLI binary, for diffing against the claude-codes crate.

The CLI ships as a Bun-compiled ELF with the bundled JavaScript embedded as
plain bytes, so the zod schema definitions are recoverable with byte-level
regex work — no unpacking needed. The wire schemas are lazy zod definitions
of the form `NAME=Se(()=>E.object({type:E.literal("..."),...}))`, and the
SDK output union is an `E.union([NAME(), ...])` over ~40 of them.

Minified names change every release; the *structure* does not. This script
anchors on the `rate_limit_event` literal (a stable, unique member), finds
the union that references it, then extracts every member schema plus
transitive references.

Usage:
  python3 scripts/extract_claude_sdk_schemas.py [BINARY] [-o OUT.txt]

BINARY defaults to the resolved `claude` on PATH (follow the symlink to
~/.local/share/claude/versions/<version>). Output is one block per schema,
labeled with its resolved type/subtype, written to stdout or -o.

The extraction logic is also importable — `extract_schemas(data)` powers
`scripts/check_claude_schema_drift.py`.

Exits 0 on success, 1 if the anchor or union cannot be located (usually
means the bundle layout changed — see claude-codes/RESNAPSHOTTING.md).
"""

from __future__ import annotations

import argparse
import re
import shutil
import sys
from pathlib import Path

# A schema definition starts at `NAME=Se(` and ends where the next one begins.
DEF_START = re.compile(rb"[^\w$]([A-Za-z_$][\w$]{1,6})=Se\(")
BOUNDARY = re.compile(rb"[,;({\[]\s*[A-Za-z_$][\w$]{1,6}=Se\(")
# Calls to other lazy schemas inside a definition body: `ref()`.
REF_CALL = re.compile(r"\b([A-Za-z_$][\w$]{1,6})\(\)")
# The anchor schema: the one whose body opens with the rate_limit_event literal.
ANCHOR = re.compile(
    rb'([A-Za-z_$][\w$]{1,6})=Se\(\(\)=>E\.object\(\{type:E\.literal\("rate_limit_event"\)'
)
UNION = re.compile(rb"[A-Za-z_$][\w$]{1,6}=Se\(\(\)=>E\.union\(\[[^\]]*\]\)\)")


class SchemaExtractionError(RuntimeError):
    """The bundle layout changed and an anchor/union could not be located."""


def resolve_binary(arg: str | None) -> Path:
    if arg:
        return Path(arg)
    on_path = shutil.which("claude")
    if not on_path:
        sys.exit("error: no `claude` on PATH; pass the binary path explicitly")
    return Path(on_path).resolve()


def index_definitions(data: bytes) -> dict[str, list[tuple[int, bytes]]]:
    """One pass over the binary → {name: [(offset, body-bytes), ...]}.

    Minified names collide across bundle modules, so each name maps to every
    candidate definition; callers pick the one nearest the union with
    [`pick_definition`]. Indexing once turns per-name full-binary scans into
    O(1) lookups — the difference between minutes and seconds on a ~250 MB ELF.
    """
    idx: dict[str, list[tuple[int, bytes]]] = {}
    for m in DEF_START.finditer(data):
        name = m.group(1).decode()
        start = m.start() + 1
        nxt = BOUNDARY.search(data, m.end())
        end = nxt.start() + 1 if nxt else m.end() + 20_000
        idx.setdefault(name, []).append((start, data[start:end]))
    return idx


def pick_definition(cands: list[tuple[int, bytes]], anchor: int) -> bytes | None:
    if not cands:
        return None
    return min(cands, key=lambda c: abs(c[0] - anchor))[1]


def label_of(body: str) -> str:
    t = re.search(r'type:E\.literal\("([a-z_]+)"\)', body)
    s = re.search(r'subtype:E\.literal\("([a-z_0-9]+)"\)', body)
    if t and s:
        return f"{t.group(1)}/{s.group(1)}"
    if t:
        return t.group(1)
    return "(nested)"


def extract_schemas(data: bytes) -> tuple[str, list[tuple[str, str, str]]]:
    """Extract the SDK output union and every reachable schema.

    Returns `(union_text, [(minified_name, label, body_text), ...])` in
    breadth-first order from the union members. Raises
    [`SchemaExtractionError`] if the anchor or union can't be found.
    """
    m = ANCHOR.search(data)
    if not m:
        raise SchemaExtractionError(
            "rate_limit_event anchor schema not found — bundle layout changed?"
        )
    anchor_name, anchor_off = m.group(1).decode(), m.start()

    union = None
    for um in UNION.finditer(data):
        if (anchor_name + "()").encode() in um.group(0):
            union = um
            break
    if union is None:
        raise SchemaExtractionError("SDK output union referencing the anchor not found")
    union_text = union.group(0).decode()
    members = REF_CALL.findall(union_text)

    idx = index_definitions(data)
    results: list[tuple[str, str, str]] = []
    seen: set[str] = set()
    queue = list(dict.fromkeys(members))
    while queue:
        name = queue.pop(0)
        if name in seen:
            continue
        seen.add(name)
        body = pick_definition(idx.get(name, []), union.start())
        if body is None:
            results.append((name, "NOT FOUND", ""))
            continue
        text = body.decode("utf-8", "replace")
        results.append((name, label_of(text), text))
        for ref in REF_CALL.findall(text):
            if ref not in seen and ref not in queue:
                queue.append(ref)
    return union_text, results


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("binary", nargs="?", help="path to the claude CLI ELF")
    ap.add_argument("-o", "--out", help="write schemas here instead of stdout")
    args = ap.parse_args()

    binary = resolve_binary(args.binary)
    data = binary.read_bytes()
    print(f"# read {binary} ({len(data):,} bytes)", file=sys.stderr)

    try:
        union_text, results = extract_schemas(data)
    except SchemaExtractionError as e:
        sys.exit(f"error: {e}")

    print(f"# union: {len(REF_CALL.findall(union_text))} members", file=sys.stderr)
    blocks = [f"// SDK output union\n{union_text}"]
    for name, label, text in results:
        blocks.append(f"// {name}: {label}\n{text}" if text else f"// {name}: {label}")

    out = "\n\n".join(blocks) + "\n"
    if args.out:
        Path(args.out).write_text(out)
        print(f"# wrote {len(blocks)} schemas to {args.out}", file=sys.stderr)
    else:
        sys.stdout.write(out)


if __name__ == "__main__":
    main()
