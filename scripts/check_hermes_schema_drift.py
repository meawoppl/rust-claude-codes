#!/usr/bin/env python3
"""Drift check for hermes-codes' ACP schema snapshots.

hermes-codes tracks a three-link provenance chain (recorded in
hermes-codes/tests/schemas/hermes_acp_provenance.json):

    hermes-agent@main  --pins-->  agent-client-protocol (PyPI)
                       --generates from-->  zed ACP schema @ tag

This script re-derives every link from upstream and diffs each against the
committed snapshots:

  1. hermes-agent@main's pyproject pin of `agent-client-protocol`
  2. that PyPI version's embedded schema ref + method tables + protocol
     version (parsed from its generated meta.py)
  3. the zed schema files (schema.unstable.json / meta.unstable.json) at
     that ref, byte-diffed structurally against our snapshots

Exit codes: 0 = no drift, 1 = drift detected (report on stdout, markdown),
2 = fetch failure (network/upstream rename — retry later, don't file).
"""

from __future__ import annotations

import io
import json
import re
import sys
import tarfile
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCHEMA_DIR = ROOT / "hermes-codes" / "tests" / "schemas"

HERMES_PYPROJECT_URL = (
    "https://raw.githubusercontent.com/NousResearch/hermes-agent/main/pyproject.toml"
)
PYPI_JSON_URL = "https://pypi.org/pypi/agent-client-protocol/{version}/json"
ZED_SCHEMA_URL = (
    "https://raw.githubusercontent.com/zed-industries/agent-client-protocol/"
    "{ref}/schema/{file}"
)


def fetch(url: str) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": "hermes-codes-drift-check"})
    with urllib.request.urlopen(req, timeout=60) as resp:  # noqa: S310
        return resp.read()


def main() -> int:
    provenance = json.loads((SCHEMA_DIR / "hermes_acp_provenance.json").read_text())
    committed_schema = json.loads((SCHEMA_DIR / "acp_schema.unstable.json").read_text())
    committed_meta = json.loads((SCHEMA_DIR / "acp_meta.unstable.json").read_text())

    report: list[str] = ["# Hermes ACP schema drift report", ""]
    drift = False

    # ── Link 1: hermes-agent@main's SDK pin ────────────────────────────
    try:
        pyproject = fetch(HERMES_PYPROJECT_URL).decode()
    except Exception as e:  # noqa: BLE001
        print(f"fetch failure: hermes-agent pyproject.toml: {e}")
        return 2
    m = re.search(r'"agent-client-protocol==([^"]+)"', pyproject)
    if not m:
        print("fetch failure: could not locate agent-client-protocol pin in pyproject.toml")
        return 2
    upstream_pin = m.group(1)
    committed_pin = provenance["hermes_agent"]["python_sdk_pin"].split("==")[-1]
    vm = re.search(r'^version\s*=\s*"([^"]+)"', pyproject, re.M)
    hermes_version = vm.group(1) if vm else "unknown"
    report.append(
        f"- hermes-agent@main is `{hermes_version}` "
        f"(snapshots taken at `{provenance['hermes_agent']['version']}`)"
    )
    if upstream_pin != committed_pin:
        drift = True
        report.append(
            f"- ⚠️ **SDK pin drift**: hermes-agent@main pins "
            f"`agent-client-protocol=={upstream_pin}`; snapshots track `{committed_pin}`"
        )
    else:
        report.append(f"- ✅ SDK pin unchanged: `agent-client-protocol=={upstream_pin}`")

    # ── Link 2: that SDK version's schema ref + method tables ─────────
    try:
        pypi = json.loads(fetch(PYPI_JSON_URL.format(version=upstream_pin)))
        sdist_url = next(u["url"] for u in pypi["urls"] if u["packagetype"] == "sdist")
        sdist = fetch(sdist_url)
        meta_py = None
        with tarfile.open(fileobj=io.BytesIO(sdist), mode="r:gz") as tf:
            for member in tf.getmembers():
                if member.name.endswith("src/acp/meta.py"):
                    meta_py = tf.extractfile(member).read().decode()
                    break
        if meta_py is None:
            raise RuntimeError("src/acp/meta.py not in sdist")
    except Exception as e:  # noqa: BLE001
        print(f"fetch failure: python-sdk {upstream_pin} sdist: {e}")
        return 2

    ref_m = re.search(r"Schema ref:\s*refs/tags/(\S+)", meta_py)
    if not ref_m:
        print("fetch failure: schema ref not found in python-sdk meta.py")
        return 2
    upstream_ref = ref_m.group(1)
    committed_ref = provenance["schema_source"]["tag"]
    if upstream_ref != committed_ref:
        drift = True
        report.append(
            f"- ⚠️ **Schema ref drift**: python-sdk {upstream_pin} generates from "
            f"`{upstream_ref}`; snapshots taken at `{committed_ref}`"
        )
    else:
        report.append(f"- ✅ Schema ref unchanged: `{upstream_ref}`")

    pv_m = re.search(r"^PROTOCOL_VERSION\s*=\s*(\d+)", meta_py, re.M)
    if pv_m and int(pv_m.group(1)) != committed_meta["version"]:
        drift = True
        report.append(
            f"- ⚠️ **Protocol version drift**: python-sdk says {pv_m.group(1)}, "
            f"snapshot says {committed_meta['version']}"
        )

    # ── Link 3: the zed schema files at that ref ───────────────────────
    try:
        upstream_schema = json.loads(
            fetch(ZED_SCHEMA_URL.format(ref=upstream_ref, file="schema.unstable.json"))
        )
        upstream_meta = json.loads(
            fetch(ZED_SCHEMA_URL.format(ref=upstream_ref, file="meta.unstable.json"))
        )
    except Exception as e:  # noqa: BLE001
        print(f"fetch failure: zed schema files @ {upstream_ref}: {e}")
        return 2

    if upstream_meta != committed_meta:
        drift = True
        report.append("- ⚠️ **meta.unstable.json drift**")
        for side in ("agentMethods", "clientMethods"):
            ours = set(committed_meta.get(side, {}).values())
            theirs = set(upstream_meta.get(side, {}).values())
            for added in sorted(theirs - ours):
                report.append(f"  - `{side}` added upstream: `{added}`")
            for removed in sorted(ours - theirs):
                report.append(f"  - `{side}` removed upstream: `{removed}`")

    ours_defs = committed_schema["$defs"]
    theirs_defs = upstream_schema["$defs"]
    added = sorted(set(theirs_defs) - set(ours_defs))
    removed = sorted(set(ours_defs) - set(theirs_defs))
    changed = sorted(
        k for k in set(ours_defs) & set(theirs_defs) if ours_defs[k] != theirs_defs[k]
    )
    if added or removed or changed:
        drift = True
        report.append("- ⚠️ **schema.unstable.json drift**")
        if added:
            report.append(f"  - definitions added upstream ({len(added)}): "
                          + ", ".join(f"`{a}`" for a in added))
        if removed:
            report.append(f"  - definitions removed upstream ({len(removed)}): "
                          + ", ".join(f"`{r}`" for r in removed))
        if changed:
            report.append(f"  - definitions changed ({len(changed)}): "
                          + ", ".join(f"`{c}`" for c in changed))
    if not (added or removed or changed) and upstream_meta == committed_meta:
        report.append("- ✅ schema.unstable.json and meta.unstable.json match upstream")

    report.append("")
    report.append(
        "To fix: update `hermes-codes/tests/schemas/` snapshots + provenance, run "
        "`python3 scripts/codegen_acp.py`, review the generated diff, bump the "
        "crate version, and update the CHANGELOG."
    )
    print("\n".join(report))
    return 1 if drift else 0


if __name__ == "__main__":
    sys.exit(main())
