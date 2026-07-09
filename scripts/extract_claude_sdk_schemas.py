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

Exits 0 on success, 1 if the anchor or union cannot be located (usually
means the bundle layout changed — see claude-codes/RESNAPSHOTTING.md).
"""

from __future__ import annotations

import argparse
import re
import shutil
import sys
from pathlib import Path

# A schema definition ends where the next `NAME=Se(` begins.
BOUNDARY = re.compile(rb"[,;({\[]\s*[A-Za-z_$][\w$]{1,6}=Se\(")
# Calls to other lazy schemas inside a definition body: `ref()`.
REF_CALL = re.compile(r"\b([A-Za-z_$][\w$]{1,6})\(\)")


def resolve_binary(arg: str | None) -> Path:
    if arg:
        return Path(arg)
    on_path = shutil.which("claude")
    if not on_path:
        sys.exit("error: no `claude` on PATH; pass the binary path explicitly")
    return Path(on_path).resolve()


def find_definitions(data: bytes, name: str) -> list[tuple[int, bytes]]:
    """Every `NAME=Se(...)` definition as (offset, body-bytes).

    Minified names collide across bundle modules, so callers must pick the
    candidate nearest the union (see `pick_definition`).
    """
    pat = re.compile(rb"[^\w$]" + re.escape(name.encode()) + rb"=Se\(")
    out = []
    for m in pat.finditer(data):
        nxt = BOUNDARY.search(data, m.end())
        end = nxt.start() + 1 if nxt else m.end() + 20_000
        out.append((m.start() + 1, data[m.start() + 1 : end]))
    return out


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


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("binary", nargs="?", help="path to the claude CLI ELF")
    ap.add_argument("-o", "--out", help="write schemas here instead of stdout")
    args = ap.parse_args()

    binary = resolve_binary(args.binary)
    data = binary.read_bytes()
    print(f"# read {binary} ({len(data):,} bytes)", file=sys.stderr)

    # 1. Anchor: the schema whose body contains the rate_limit_event literal.
    m = re.search(
        rb'([A-Za-z_$][\w$]{1,6})=Se\(\(\)=>E\.object\(\{type:E\.literal\("rate_limit_event"\)',
        data,
    )
    if not m:
        sys.exit("error: rate_limit_event anchor schema not found — bundle layout changed?")
    anchor_name, anchor_off = m.group(1).decode(), m.start()
    print(f"# anchor: {anchor_name} at {anchor_off}", file=sys.stderr)

    # 2. The union that calls the anchor: `=Se(()=>E.union([...anchor()...]))`.
    union_pat = re.compile(rb"[A-Za-z_$][\w$]{1,6}=Se\(\(\)=>E\.union\(\[[^\]]*\]\)\)")
    union = None
    for um in union_pat.finditer(data):
        if (anchor_name + "()").encode() in um.group(0):
            union = um
            break
    if union is None:
        sys.exit("error: SDK output union referencing the anchor not found")
    members = re.findall(r"([A-Za-z_$][\w$]{1,6})\(\)", union.group(0).decode())
    print(f"# union at {union.start()}: {len(members)} members", file=sys.stderr)

    # 3. Extract members + transitive refs, nearest-to-union on name collisions.
    blocks: list[str] = [f"// SDK output union ({len(members)} members)\n{union.group(0).decode()}"]
    seen: set[str] = set()
    queue = list(dict.fromkeys(members))
    while queue:
        name = queue.pop(0)
        if name in seen:
            continue
        seen.add(name)
        body = pick_definition(find_definitions(data, name), union.start())
        if body is None:
            blocks.append(f"// {name}: NOT FOUND")
            continue
        text = body.decode("utf-8", "replace")
        blocks.append(f"// {name}: {label_of(text)}\n{text}")
        for ref in REF_CALL.findall(text):
            if ref not in seen and ref not in queue:
                queue.append(ref)

    out = "\n\n".join(blocks) + "\n"
    if args.out:
        Path(args.out).write_text(out)
        print(f"# wrote {len(blocks)} schemas to {args.out}", file=sys.stderr)
    else:
        sys.stdout.write(out)


if __name__ == "__main__":
    main()
