#!/usr/bin/env python3
"""
Diff our snapshotted Antigravity localharness descriptors against the ones in
the latest `google-antigravity` wheel on PyPI.

Writes a Markdown report to stdout and exits:
  0 — no drift
  1 — drift detected; the report says what changed
  2 — failed to fetch or parse the upstream wheel (transient; CI treats this
      separately so a network blip does not open an issue)

## Why the wheel, and not the .proto files in the SDK repo

Two reasons. The repo's `.proto` files run *ahead* of what ships — 0.1.10's
binary has no `HarnessConfig.agent_mode` even though the checked-in proto
declares it — so they describe a contract no released harness speaks. And they
are written in protobuf edition 2024, which `protoc` below v31 cannot parse.
The descriptor embedded in the wheel's `localharness_pb2.py` is both parseable
(see `codegen_antigravity.py`'s reader) and authoritative.

## The wheel is 37 MB; this script downloads ~30 KB of it

PyPI serves HTTP range requests, and a wheel is a zip. So: read the end of the
file to find the central directory, locate the two `*_pb2.py` members, and
fetch only those. Falls back to a full download if the server refuses ranges.

## Usage

    python3 scripts/check_antigravity_schema_drift.py
    python3 scripts/check_antigravity_schema_drift.py --version 0.1.11
"""

from __future__ import annotations

import argparse
import json
import pathlib
import struct
import sys
import urllib.error
import urllib.request
import zlib

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

from codegen_antigravity import (  # noqa: E402
    PB2_FILES,
    SCHEMAS,
    File,
    Registry,
    extract_descriptor,
)

PYPI_JSON = "https://pypi.org/pypi/google-antigravity/json"
# Any platform wheel carries the same descriptors; the binary differs, the
# generated protobuf modules do not.
WHEEL_PREFERENCE = ("manylinux_2_17_x86_64", "manylinux_2_17_aarch64", "macosx", "win_amd64")

EOCD_SIGNATURE = b"PK\x05\x06"
TAIL_BYTES = 65536


def fetch(url: str, start: int | None = None, end: int | None = None) -> bytes:
    request = urllib.request.Request(url)
    if start is not None:
        request.add_header("Range", f"bytes={start}-{'' if end is None else end}")
    with urllib.request.urlopen(request, timeout=120) as response:
        return response.read()


def wheel_url(version: str | None) -> tuple[str, str]:
    """Returns `(version, url)` for the best-matching wheel."""
    try:
        with urllib.request.urlopen(PYPI_JSON, timeout=60) as response:
            index = json.load(response)
    except (urllib.error.URLError, TimeoutError, json.JSONDecodeError) as e:
        raise SystemExit(f"could not reach PyPI: {e}") from e

    version = version or index["info"]["version"]
    files = index["releases"].get(version) if version != index["info"]["version"] else index["urls"]
    if not files:
        raise SystemExit(f"no files published for google-antigravity {version}")

    for want in WHEEL_PREFERENCE:
        for f in files:
            if f["filename"].endswith(".whl") and want in f["filename"]:
                return version, f["url"]
    for f in files:
        if f["filename"].endswith(".whl"):
            return version, f["url"]
    raise SystemExit(f"google-antigravity {version} publishes no wheels")


def content_length(url: str) -> int:
    request = urllib.request.Request(url, method="HEAD")
    with urllib.request.urlopen(request, timeout=60) as response:
        return int(response.headers["Content-Length"])


def zip_members(url: str) -> dict[str, tuple[int, int, int]]:
    """Maps member name -> (local header offset, compressed size, method).

    Parsed out of the central directory, which lives at the end of the archive.
    """
    # An open-ended suffix range (`bytes=-N`) is rejected by PyPI's CDN with a
    # 501, so the size is fetched first and the range given as absolute bounds.
    size = content_length(url)
    tail = fetch(url, start=max(0, size - TAIL_BYTES), end=size - 1)
    eocd = tail.rfind(EOCD_SIGNATURE)
    if eocd < 0:
        raise ValueError("no end-of-central-directory record in the wheel tail")
    cd_size, cd_offset = struct.unpack("<II", tail[eocd + 12 : eocd + 20])

    directory = fetch(url, start=cd_offset, end=cd_offset + cd_size - 1)
    members: dict[str, tuple[int, int, int]] = {}
    pos = 0
    while pos + 46 <= len(directory) and directory[pos : pos + 4] == b"PK\x01\x02":
        method, = struct.unpack("<H", directory[pos + 10 : pos + 12])
        compressed, = struct.unpack("<I", directory[pos + 20 : pos + 24])
        name_len, extra_len, comment_len = struct.unpack("<HHH", directory[pos + 28 : pos + 34])
        offset, = struct.unpack("<I", directory[pos + 42 : pos + 46])
        name = directory[pos + 46 : pos + 46 + name_len].decode("utf-8", "replace")
        members[name] = (offset, compressed, method)
        pos += 46 + name_len + extra_len + comment_len
    return members


def read_member(url: str, name: str, entry: tuple[int, int, int]) -> bytes:
    offset, compressed, method = entry
    header = fetch(url, start=offset, end=offset + 29)
    if header[:4] != b"PK\x03\x04":
        raise ValueError(f"{name}: bad local file header")
    name_len, extra_len = struct.unpack("<HH", header[26:30])
    start = offset + 30 + name_len + extra_len
    body = fetch(url, start=start, end=start + compressed - 1)
    if method == 0:
        return body
    return zlib.decompress(body, -zlib.MAX_WBITS)


def upstream_descriptors(url: str) -> dict[str, bytes]:
    """Pulls the two pb2 modules out of the wheel, by range if possible."""
    try:
        members = zip_members(url)
        out = {}
        for stem, member in PB2_FILES:
            if member not in members:
                raise ValueError(f"{member} is missing from the wheel")
            out[stem] = extract_descriptor(read_member(url, member, members[member]))
        return out
    except (urllib.error.URLError, ValueError, struct.error, zlib.error) as e:
        print(f"<!-- ranged read failed ({e}); falling back to a full download -->")

    import io
    import zipfile

    with zipfile.ZipFile(io.BytesIO(fetch(url))) as zf:
        return {stem: extract_descriptor(zf.read(member)) for stem, member in PB2_FILES}


def fingerprint(files: list[File]) -> dict:
    """Reduces descriptors to the parts a wire-compatible change would move."""
    registry = Registry(files)
    messages = {}
    for name, message in registry.messages.items():
        if name.startswith(".google.protobuf"):
            continue
        messages[name] = {
            f.name: {
                "number": f.number,
                "type": f.type,
                "type_name": f.type_name,
                "repeated": f.repeated,
                "json_name": f.json_name,
                "oneof": f.oneof_index,
            }
            for f in message.fields
        }
    enums = {
        name: {value: number for value, number in enum.values}
        for name, enum in registry.enums.items()
        if not name.startswith(".google.protobuf")
    }
    return {"messages": messages, "enums": enums}


def diff_section(title: str, ours: dict, theirs: dict) -> list[str]:
    lines: list[str] = []
    added = sorted(set(theirs) - set(ours))
    removed = sorted(set(ours) - set(theirs))
    changed = sorted(k for k in set(ours) & set(theirs) if ours[k] != theirs[k])

    if added:
        lines.append(f"### {title} added upstream\n")
        lines += [f"- `{name}`" for name in added] + [""]
    if removed:
        lines.append(f"### {title} removed upstream\n")
        lines += [f"- `{name}`" for name in removed] + [""]
    for name in changed:
        lines.append(f"### `{name}` changed\n")
        mine, other = ours[name], theirs[name]
        for key in sorted(set(mine) | set(other)):
            if mine.get(key) != other.get(key):
                lines.append(f"- `{key}`: ours `{mine.get(key)}` -> theirs `{other.get(key)}`")
        lines.append("")
    return lines


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--version", help="check against a specific release instead of the latest")
    args = ap.parse_args()

    try:
        version, url = wheel_url(args.version)
        upstream = upstream_descriptors(url)
    except SystemExit:
        raise
    except Exception as e:  # noqa: BLE001 — any fetch failure is a soft skip
        print(f"# Antigravity schema drift: fetch failed\n\n{type(e).__name__}: {e}")
        sys.exit(2)

    ours = fingerprint([File((SCHEMAS / f"{stem}.descriptor.bin").read_bytes()) for stem, _ in PB2_FILES])
    theirs = fingerprint([File(upstream[stem]) for stem, _ in PB2_FILES])

    lines = [f"# Antigravity schema drift vs google-antigravity {version}\n"]
    body = diff_section("Messages", ours["messages"], theirs["messages"])
    body += diff_section("Enums", ours["enums"], theirs["enums"])

    if not body:
        lines.append(f"No drift. The snapshot matches {version}.")
        print("\n".join(lines))
        sys.exit(0)

    lines += body
    lines.append("## How to accept this\n")
    lines.append("```sh")
    lines.append("pip download google-antigravity --no-deps -d /tmp/ag")
    lines.append("python3 scripts/codegen_antigravity.py --wheel /tmp/ag/*.whl")
    lines.append("cargo test -p antigravity-codes --all-features")
    lines.append("```")
    lines.append("")
    lines.append(
        "Then bump `antigravity-codes` to the new SDK version and update "
        "`TESTED_SDK_VERSION` in `src/lib.rs`."
    )
    print("\n".join(lines))
    sys.exit(1)


if __name__ == "__main__":
    main()
