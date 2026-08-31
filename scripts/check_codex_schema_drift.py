#!/usr/bin/env python3
"""
Diff our snapshotted Codex app-server schemas against the upstream copies
hosted at github.com/openai/codex@main.

Writes a structured Markdown report to stdout and exits:
  0 — no drift (or schema missing upstream entirely, which we treat as a soft skip)
  1 — drift detected; report on stdout describes what changed
  2 — failed to fetch / parse a schema (transient, treated separately by CI so
      we don't open spurious issues on network blips)

Usage:
  python3 scripts/check_codex_schema_drift.py [--ref main]

The two schemas it checks live in the openai/codex repo at:
  codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.v2.schemas.json
  codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.schemas.json

Our local snapshot lives at:
  codex-codes/tests/schemas/{,v2.}schemas.json
"""

from __future__ import annotations

import argparse
import json
import sys
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOCAL_DIR = ROOT / "codex-codes" / "tests" / "schemas"

UPSTREAM_BASE = (
    "https://raw.githubusercontent.com/openai/codex/{ref}"
    "/codex-rs/app-server-protocol/schema/json"
)

# (local file name, upstream file name) pairs. Same names today, but kept
# explicit so a future rename upstream is easy to handle.
SCHEMAS = [
    ("codex_app_server_protocol.v2.schemas.json", "codex_app_server_protocol.v2.schemas.json"),
    ("codex_app_server_protocol.schemas.json", "codex_app_server_protocol.schemas.json"),
]


def fetch(url: str) -> str:
    req = urllib.request.Request(url, headers={"User-Agent": "codex-codes-drift-check"})
    with urllib.request.urlopen(req, timeout=30) as resp:  # noqa: S310
        return resp.read().decode("utf-8")


def envelope_methods(defs: dict, envelope: str) -> list[tuple[str, str]]:
    """(method, params_def_name) for every variant in `{envelope}.oneOf`."""
    out = []
    for variant in defs.get(envelope, {}).get("oneOf", []):
        props = variant.get("properties", {})
        method_enum = props.get("method", {}).get("enum") or []
        params_ref = props.get("params", {}).get("$ref", "")
        params_def = params_ref.rsplit("/", 1)[-1] if params_ref else "<no params>"
        if method_enum:
            out.append((method_enum[0], params_def))
    return out


def summarize_diff(local: dict, upstream: dict) -> dict:
    """Compute a structured diff: top-level definition adds/removes/changes,
    plus per-envelope method adds/removes."""
    local_defs = local.get("definitions", {}) or {}
    upstream_defs = upstream.get("definitions", {}) or {}
    local_names = set(local_defs)
    upstream_names = set(upstream_defs)
    added = sorted(upstream_names - local_names)
    removed = sorted(local_names - upstream_names)
    shared = sorted(local_names & upstream_names)
    changed = sorted(
        name
        for name in shared
        if local_defs[name] != upstream_defs[name]
    )

    method_changes: dict[str, dict[str, list[str]]] = {}
    for envelope in ("ServerNotification", "ClientRequest", "ServerRequest"):
        local_methods = {m for m, _ in envelope_methods(local_defs, envelope)}
        upstream_methods = {m for m, _ in envelope_methods(upstream_defs, envelope)}
        added_m = sorted(upstream_methods - local_methods)
        removed_m = sorted(local_methods - upstream_methods)
        if added_m or removed_m:
            method_changes[envelope] = {"added": added_m, "removed": removed_m}

    return {
        "definitions_added": added,
        "definitions_removed": removed,
        "definitions_changed": changed,
        "methods_changed_per_envelope": method_changes,
    }


def render_markdown(file_label: str, diff: dict) -> str:
    lines = [f"### `{file_label}`", ""]
    sec = [
        ("Definitions added upstream", diff["definitions_added"]),
        ("Definitions removed upstream", diff["definitions_removed"]),
        ("Definitions whose body changed", diff["definitions_changed"]),
    ]
    for header, items in sec:
        if items:
            lines.append(f"**{header}** ({len(items)}):")
            lines.append("")
            for it in items[:80]:
                lines.append(f"- `{it}`")
            if len(items) > 80:
                lines.append(f"- ... and {len(items) - 80} more")
            lines.append("")
    for envelope, ch in diff["methods_changed_per_envelope"].items():
        lines.append(f"**`{envelope}` methods**:")
        if ch["added"]:
            lines.append("")
            lines.append("- added upstream:")
            for m in ch["added"]:
                lines.append(f"  - `{m}`")
        if ch["removed"]:
            lines.append("")
            lines.append("- removed upstream:")
            for m in ch["removed"]:
                lines.append(f"  - `{m}`")
        lines.append("")
    return "\n".join(lines)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--ref", default="main", help="Upstream ref to compare against (default: main)")
    args = ap.parse_args()

    upstream_base = UPSTREAM_BASE.format(ref=args.ref)
    any_drift = False
    output_chunks = [
        f"# Codex app-server schema drift report",
        "",
        f"Comparing `codex-codes/tests/schemas/*.json` against `openai/codex@{args.ref}`.",
        "",
    ]

    for local_name, upstream_name in SCHEMAS:
        local_path = LOCAL_DIR / local_name
        url = f"{upstream_base}/{upstream_name}"
        try:
            local_text = local_path.read_text()
            local = json.loads(local_text)
        except FileNotFoundError:
            print(f"error: local schema missing at {local_path}", file=sys.stderr)
            return 2

        try:
            upstream_text = fetch(url)
            upstream = json.loads(upstream_text)
        except (urllib.error.URLError, urllib.error.HTTPError, json.JSONDecodeError) as e:
            print(f"error: could not fetch/parse upstream {url}: {e}", file=sys.stderr)
            return 2

        # JSON equality is the contract; formatting and key order don't matter.
        if local == upstream:
            output_chunks.append(f"- ✅ `{local_name}` — matches upstream (JSON-identical)\n")
            continue

        any_drift = True
        diff = summarize_diff(local, upstream)
        output_chunks.append(f"- ⚠️ `{local_name}` — **drift detected**\n")
        output_chunks.append(render_markdown(local_name, diff))
        output_chunks.append(f"\nUpstream source: {url}\n")

    if any_drift:
        output_chunks.append("")
        output_chunks.append("---")
        output_chunks.append("")
        output_chunks.append(
            "Regenerate the snapshot with:"
        )
        output_chunks.append("")
        output_chunks.append("```bash")
        output_chunks.append(
            "curl -sSfL "
            f"{upstream_base}/codex_app_server_protocol.v2.schemas.json "
            "> codex-codes/tests/schemas/codex_app_server_protocol.v2.schemas.json"
        )
        output_chunks.append(
            "curl -sSfL "
            f"{upstream_base}/codex_app_server_protocol.schemas.json "
            "> codex-codes/tests/schemas/codex_app_server_protocol.schemas.json"
        )
        output_chunks.append(
            "python3 scripts/codegen_protocol.py  # regenerate typed structs + samples"
        )
        output_chunks.append("cargo run --example schema_coverage  # confirm 100% coverage")
        output_chunks.append("```")

    print("\n".join(output_chunks))
    return 1 if any_drift else 0


if __name__ == "__main__":
    sys.exit(main())
