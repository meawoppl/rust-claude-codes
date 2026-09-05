#!/usr/bin/env python3
"""
Extract the Claude Code CLI's SDK stream-json output schemas from the
compiled CLI binary, for diffing against the claude-codes crate.

The CLI ships as a Bun-compiled ELF with the bundled JavaScript embedded as
plain bytes, so the zod schema definitions are recoverable with byte-level
regex work — no unpacking needed. The wire schemas are lazy zod definitions
of the form `NAME=<lazy>(()=><zod>.object({type:<zod>.literal("..."),...}))`,
and the SDK output union is a `<zod>.union([NAME(), ...])` over ~40 of them.

Minified names change every release — including the zod-namespace alias
(`E` on CLI 2.1.205) and the lazy-schema wrapper alias (`Se` on 2.1.205) —
but the *structure* does not. This script therefore discovers the aliases
instead of assuming them: it anchors on the stable `"rate_limit_event"`
literal, reads the lazy/zod alias tokens from the bytes around it, and
rebuilds every pattern from the discovered pair. If several alias pairs
match (multiple bundled zod copies), each is tried until one yields the
union.

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
from typing import NamedTuple

# A minified schema name (2-7 chars, like the pre-discovery extractor assumed).
_IDENT = rb"[A-Za-z_$][\w$]{1,6}"
# A minified alias can be a single character (`E` is the zod alias on 2.1.205).
_ALIAS = rb"[A-Za-z_$][\w$]{0,6}"

# Discovers the alias pair from the anchor schema (zod-namespace bundle
# style, CLI <= 2.1.23x). Captures:
#   1. the anchor schema's minified name
#   2. the lazy-schema wrapper alias (`Se` on CLI 2.1.205)
#   3. the zod-namespace alias (`E` on CLI 2.1.205)
ALIAS_DISCOVERY = re.compile(
    rb"(" + _IDENT + rb")=(" + _ALIAS + rb")\(\(\)=>"
    rb"(" + _ALIAS + rb')\.object\(\{type:\3\.literal\("rate_limit_event"\)'
)

# Same discovery for the free-function bundle style (CLI 2.1.239+): the zod
# combinators are destructured into standalone minified functions, so the
# anchor reads `P3b=ve(()=>_e({type:Tt("rate_limit_event"),...}))` instead of
# `NAME=Se(()=>E.object({type:E.literal("rate_limit_event"),...}))`. Captures:
#   1. the anchor schema's minified name
#   2. the lazy-schema wrapper alias (`ve` on 2.1.239)
#   3. the object-combinator function (`_e` on 2.1.239)
#   4. the literal-combinator function (`Tt` on 2.1.239)
FREEFN_DISCOVERY = re.compile(
    rb"(" + _IDENT + rb")=(" + _ALIAS + rb")\(\(\)=>"
    rb"(" + _ALIAS + rb')\(\{type:(' + _ALIAS + rb')\("rate_limit_event"\)'
)

# Calls to other lazy schemas inside a definition body: `ref()`. A plain \b
# does not work here: minified names can start with `$` (2.1.218 does this),
# and there is no word boundary between a delimiter and `$` — it would match
# the truncated `eE` out of `$eE()`. Guard with a lookbehind instead; `.` is
# excluded too so zod method calls (`.string()`, `.optional()`) aren't taken
# for schema refs.
REF_CALL = re.compile(r"(?<![\w$.])([A-Za-z_$][\w$]{1,6})\(\)")


class WirePatterns(NamedTuple):
    """Byte-regexes rebuilt from a discovered alias set.

    `object_open` is the exact text that opens an object schema body
    (`E.object({` in the zod-namespace style, `_e({` in the free-function
    style) and `literal` the callable producing a literal (`E.literal` /
    `Tt`) — consumers use them to parse bodies without caring which bundle
    style the binary uses.
    """

    lazy: str
    object_open: str
    literal: str
    def_start: re.Pattern[bytes]
    boundary: re.Pattern[bytes]
    union: re.Pattern[bytes]


def _shared_patterns(lazy: bytes) -> tuple[re.Pattern[bytes], re.Pattern[bytes]]:
    lz = re.escape(lazy)
    return (
        # A schema definition starts at `NAME=<lazy>(` and ends where the next begins.
        re.compile(rb"[^\w$](" + _IDENT + rb")=" + lz + rb"\("),
        re.compile(rb"[,;({\[]\s*" + _IDENT + rb"=" + lz + rb"\("),
    )


def build_patterns(lazy: bytes, zod: bytes) -> WirePatterns:
    lz, zd = re.escape(lazy), re.escape(zod)
    def_start, boundary = _shared_patterns(lazy)
    return WirePatterns(
        lazy=lazy.decode(),
        object_open=zod.decode() + ".object({",
        literal=zod.decode() + ".literal",
        def_start=def_start,
        boundary=boundary,
        union=re.compile(
            _IDENT + rb"=" + lz + rb"\(\(\)=>" + zd + rb"\.union\(\[[^\]]*\]\)"
        ),
    )


def build_freefn_patterns(lazy: bytes, objfn: bytes, litfn: bytes) -> WirePatterns:
    lz = re.escape(lazy)
    def_start, boundary = _shared_patterns(lazy)
    return WirePatterns(
        lazy=lazy.decode(),
        object_open=objfn.decode() + "({",
        literal=litfn.decode(),
        def_start=def_start,
        boundary=boundary,
        # The union combinator's minified name isn't recoverable from the
        # anchor, so accept any `fn([...])` — the caller keeps only the match
        # that references the anchor schema.
        union=re.compile(
            _IDENT + rb"=" + lz + rb"\(\(\)=>" + _ALIAS + rb"\(\[[^\]]*\]\)"
        ),
    )


class Extraction(NamedTuple):
    """Everything `extract_schemas` recovers from the binary."""

    union_text: str
    # (minified_name, label, body_text) in BFS order from the union members.
    results: list[tuple[str, str, str]]
    # The discovered object-schema opener (`E.object({` / `_e({`) — needed by
    # consumers that parse the bodies (e.g. the drift check's top-level-key
    # scan).
    object_open: str
    # Referenced names with no definition under the schema wrapper — module
    # initializer thunks and helper functions whose call sites happen to
    # match the `ref()` shape, not schemas. Reported for transparency; they
    # carry no wire fields.
    unresolved: list[str]


class SchemaExtractionError(RuntimeError):
    """The bundle layout changed and an anchor/union could not be located."""


def resolve_binary(arg: str | None) -> Path:
    if arg:
        return Path(arg)
    on_path = shutil.which("claude")
    if not on_path:
        sys.exit("error: no `claude` on PATH; pass the binary path explicitly")
    return Path(on_path).resolve()


def discover_aliases(data: bytes) -> list[tuple[re.Match[bytes], WirePatterns]]:
    """All `(anchor_match, patterns)` candidates in the binary, both styles.

    Anchored on the stable `"rate_limit_event"` literal; the aliases are read
    from the surrounding bytes rather than assumed. Multiple candidates can
    appear when the bundle carries more than one zod copy — callers try each.
    """
    out: list[tuple[re.Match[bytes], WirePatterns]] = []
    for m in ALIAS_DISCOVERY.finditer(data):
        out.append((m, build_patterns(m.group(2), m.group(3))))
    for m in FREEFN_DISCOVERY.finditer(data):
        out.append((m, build_freefn_patterns(m.group(2), m.group(3), m.group(4))))
    return out


# Bun separates the bundled JS chunks with a `// @bun` banner. Minified names
# are only unique within one chunk, so schema references are resolved inside
# the chunk that holds the SDK output union.
CHUNK_MARKER = b"// @bun"


def chunk_bounds(data: bytes, offset: int) -> tuple[int, int]:
    """`(start, end)` of the bundle chunk containing `offset`."""
    lo = data.rfind(CHUNK_MARKER, 0, offset)
    hi = data.find(CHUNK_MARKER, offset)
    return (0 if lo < 0 else lo, len(data) if hi < 0 else hi)


def index_definitions(
    data: bytes, pats: WirePatterns, lo: int = 0, hi: int | None = None
) -> dict[str, list[tuple[int, bytes]]]:
    """One pass over `data[lo:hi]` → {name: [(offset, body-bytes), ...]}.

    Minified names collide across bundle modules, so each name maps to every
    candidate definition; callers pick the one nearest the union with
    [`pick_definition`]. Indexing once turns per-name full-binary scans into
    O(1) lookups — the difference between minutes and seconds on a ~250 MB ELF.

    `lo`/`hi` bound the scan to one bundle chunk (see [`chunk_bounds`]). A
    name that is a plain helper in the union's chunk (`se()` is `z.unknown()`
    on CLI 2.1.261) can be a lazy schema in another chunk; resolving it there
    crawled that chunk's whole module graph.
    """
    if hi is None:
        hi = len(data)
    idx: dict[str, list[tuple[int, bytes]]] = {}
    for m in pats.def_start.finditer(data, lo, hi):
        name = m.group(1).decode()
        start = m.start() + 1
        nxt = pats.boundary.search(data, m.end(), hi)
        end = nxt.start() + 1 if nxt else min(m.end() + 20_000, hi)
        idx.setdefault(name, []).append((start, data[start:end]))
    return idx


def pick_definition(cands: list[tuple[int, bytes]], anchor: int) -> bytes | None:
    if not cands:
        return None
    return min(cands, key=lambda c: abs(c[0] - anchor))[1]




def label_of(body: str, literal: str) -> str:
    lit = re.escape(literal)
    t = re.search(r"type:" + lit + r'\("([a-z_]+)"\)', body)
    s = re.search(r"subtype:" + lit + r'\("([a-z_0-9]+)"\)', body)
    if t and s:
        return f"{t.group(1)}/{s.group(1)}"
    if t:
        return t.group(1)
    return "(nested)"


def _extract_with(data: bytes, anchor: re.Match[bytes], pats: WirePatterns) -> Extraction:
    anchor_name = anchor.group(1).decode()

    union = None
    for um in pats.union.finditer(data):
        if (anchor_name + "()").encode() in um.group(0):
            union = um
            break
    if union is None:
        raise SchemaExtractionError("SDK output union referencing the anchor not found")
    union_text = union.group(0).decode()
    members = REF_CALL.findall(union_text)

    idx = index_definitions(data, pats, *chunk_bounds(data, union.start()))
    results: list[tuple[str, str, str]] = []
    unresolved: list[str] = []
    seen: set[str] = set()
    queue = list(dict.fromkeys(members))
    while queue:
        name = queue.pop(0)
        if name in seen:
            continue
        seen.add(name)
        body = pick_definition(idx.get(name, []), union.start())
        if body is None:
            # Not defined under the schema wrapper anywhere in this chunk —
            # a module-initializer thunk (`NAME=S(()=>{...})`) or helper
            # function whose call site happens to match the `ref()` shape,
            # not a schema. Verified empirically: every such name in CLI
            # 2.1.205/2.1.218 is a module init or helper, and following them
            # crawls the bundler's module graph instead of the schema graph.
            unresolved.append(name)
            continue
        text = body.decode("utf-8", "replace")
        results.append((name, label_of(text, pats.literal), text))
        for ref in REF_CALL.findall(text):
            if ref not in seen and ref not in queue:
                queue.append(ref)
    return Extraction(union_text, results, pats.object_open, unresolved)


def extract_schemas(data: bytes) -> Extraction:
    """Extract the SDK output union and every reachable schema.

    Discovers the minified lazy/zod aliases from the `rate_limit_event`
    anchor, then walks the union. Raises [`SchemaExtractionError`] if no
    anchor is found or no candidate alias pair yields the union.
    """
    candidates = discover_aliases(data)
    if not candidates:
        raise SchemaExtractionError(
            "rate_limit_event anchor schema not found — bundle layout changed?"
        )

    failures = []
    for anchor, pats in candidates:
        try:
            return _extract_with(data, anchor, pats)
        except SchemaExtractionError as e:
            failures.append(f"aliases lazy={pats.lazy} object={pats.object_open}: {e}")
    raise SchemaExtractionError(
        "no candidate alias pair yielded the SDK output union: " + "; ".join(failures)
    )


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("binary", nargs="?", help="path to the claude CLI ELF")
    ap.add_argument("-o", "--out", help="write schemas here instead of stdout")
    args = ap.parse_args()

    binary = resolve_binary(args.binary)
    data = binary.read_bytes()
    print(f"# read {binary} ({len(data):,} bytes)", file=sys.stderr)

    try:
        extraction = extract_schemas(data)
    except SchemaExtractionError as e:
        sys.exit(f"error: {e}")

    union_text, results = extraction.union_text, extraction.results
    print(
        f"# union: {len(REF_CALL.findall(union_text))} members"
        f" (object schema opener: {extraction.object_open!r})",
        file=sys.stderr,
    )
    if extraction.unresolved:
        print(
            "# non-schema refs skipped (no lazy definition in the union chunk): "
            + ", ".join(extraction.unresolved),
            file=sys.stderr,
        )
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
