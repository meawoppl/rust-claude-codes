#!/usr/bin/env python3
"""Drift check for muse-codes' JSONL stream models.

Muse Code publishes no schema, so the contract is fingerprinted from the
wire itself: run credential-free `muse exec --json --provider echo`
captures against the installed binary, reduce every record to its
`payload_type -> sorted field set` (envelope fields fingerprinted once),
and diff against the committed snapshot at
`muse-codes/tests/schemas/muse_stream_fingerprint.txt`.

The echo provider exercises the headless vocabulary without model calls or
credentials; payload types that only occur under a live provider are
outside this fingerprint's reach and are handled by the crate's open
`MusePayload::Unknown` fallback until captured.

Usage:
    check_muse_schema_drift.py [--binary PATH] [--update]

Exit codes: 0 = no drift, 1 = drift (markdown report on stdout),
2 = capture failure (binary missing/broken — retry, don't file).
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SNAPSHOT = ROOT / "muse-codes" / "tests" / "schemas" / "muse_stream_fingerprint.txt"

CAPTURE_RUNS = [
    ["--provider", "echo", "print hello"],
    ["--provider", "echo", "--preset", "native-basic", "list files"],
]

ENVELOPE_KEY = "__envelope__"


def capture_fingerprint(binary: str) -> dict[str, list[str]]:
    fingerprint: dict[str, set[str]] = {}
    for extra in CAPTURE_RUNS:
        with tempfile.TemporaryDirectory() as td:
            proc = subprocess.run(  # noqa: S603
                [binary, "exec", "--json", *extra],
                capture_output=True,
                text=True,
                timeout=120,
                cwd=td,
            )
        if proc.returncode != 0 and not proc.stdout.strip():
            raise RuntimeError(
                f"capture failed (rc={proc.returncode}): {proc.stderr.strip()[:300]}"
            )
        for line in proc.stdout.splitlines():
            line = line.strip()
            if not line:
                continue
            r = json.loads(line)
            fingerprint.setdefault(ENVELOPE_KEY, set()).update(r.keys())
            fields = set((r.get("payload") or {}).keys())
            pt = r["payload_type"]
            fingerprint.setdefault(pt, set()).update(fields)
            # task lifecycle events carry a nested tagged event object
            ev = (r.get("payload") or {}).get("event")
            if isinstance(ev, dict):
                key = f"{pt}::event[{ev.get('kind')}]"
                fingerprint.setdefault(key, set()).update(ev.keys())
    return {k: sorted(v) for k, v in sorted(fingerprint.items())}


def render(fp: dict[str, list[str]]) -> str:
    return "\n".join(f"{k}: {', '.join(v)}" for k, v in fp.items()) + "\n"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--binary", default="muse")
    ap.add_argument("--update", action="store_true", help="accept the new fingerprint")
    args = ap.parse_args()

    try:
        fp = capture_fingerprint(args.binary)
    except FileNotFoundError:
        print(f"capture failure: {args.binary} not found")
        return 2
    except Exception as e:  # noqa: BLE001
        print(f"capture failure: {e}")
        return 2

    new = render(fp)
    if args.update:
        SNAPSHOT.write_text(new)
        print(f"# wrote {len(fp)} fingerprint entries to {SNAPSHOT}")
        return 0

    old = SNAPSHOT.read_text() if SNAPSHOT.exists() else ""
    if new == old:
        print("# Muse Code stream fingerprint matches the snapshot")
        return 0

    old_map = {}
    for line in old.splitlines():
        if ": " in line:
            k, v = line.split(": ", 1)
            old_map[k] = v
    print("# Muse Code stream fingerprint drift report")
    print()
    print("Comparing the installed `muse` CLI (echo-provider captures) against")
    print("`muse-codes/tests/schemas/muse_stream_fingerprint.txt`.")
    print()
    for k, v in fp.items():
        joined = ", ".join(v)
        if k not in old_map:
            print(f"- **added**: `{k}` ({joined})")
        elif old_map[k] != joined:
            print(f"- **changed**: `{k}`")
            print(f"  - was: {old_map[k]}")
            print(f"  - now: {joined}")
    for k in old_map:
        if k not in fp:
            print(f"- **removed**: `{k}`")
    print()
    print("Regenerate with `python3 scripts/check_muse_schema_drift.py --update`,")
    print("re-capture `test_cases/`, and update the typed models to match.")
    return 1


if __name__ == "__main__":
    sys.exit(main())
