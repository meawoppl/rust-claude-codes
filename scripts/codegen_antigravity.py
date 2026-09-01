#!/usr/bin/env python3
"""
Generate `antigravity-codes/src/protocol_generated/` from the Antigravity
localharness protobuf descriptors.

## Why a descriptor, and not the .proto files

`google-antigravity`'s protos are written in **protobuf edition 2024**. `protoc`
below v31 cannot parse them at all, and `prost-build` panics on
`FileDescriptorProto.syntax == "editions"` (tokio-rs/prost#1031). Meanwhile the
wire format is *protobuf-JSON over a WebSocket*, so nothing in the crate needs a
protobuf runtime — only serde types shaped like the protojson encoding.

The published wheel ships `localharness_pb2.py`, which embeds the fully-resolved
`FileDescriptorProto` as a bytes literal. That descriptor is the source of truth
for what the shipped binary actually speaks (the checked-in `.proto` files in
the SDK repo run *ahead* of it — `HarnessConfig.agent_mode` exists in git but
not in 0.1.10). So: pull the descriptor out of the wheel, snapshot it, and
generate from the snapshot.

This script is stdlib-only. It contains a small protobuf wire-format reader for
just the descriptor.proto subset it needs, so neither codegen nor CI needs
`pip install protobuf`.

## Usage

    # Regenerate from the committed descriptor snapshot:
    python3 scripts/codegen_antigravity.py

    # Re-snapshot from a wheel, then regenerate:
    python3 scripts/codegen_antigravity.py --wheel /path/to/google_antigravity-*.whl
"""

from __future__ import annotations

import argparse
import ast
import pathlib
import re
import subprocess
import sys
import zipfile

REPO = pathlib.Path(__file__).resolve().parent.parent
CRATE = REPO / "antigravity-codes"
SCHEMAS = CRATE / "tests" / "schemas"
GEN = CRATE / "src" / "protocol_generated"

# Files we lift out of the wheel, in dependency order.
PB2_FILES = [
    ("content", "google/antigravity/proto/content_pb2.py"),
    ("localharness", "google/antigravity/proto/localharness_pb2.py"),
]

# Messages the crate actually needs. Everything reachable from these is emitted;
# the rest of content.proto (a large Gemini content model the harness never puts
# on this wire) is skipped.
ROOTS = [
    ".antigravity.localharness.InputConfig",
    ".antigravity.localharness.OutputConfig",
    ".antigravity.localharness.InitializeConversationEvent",
    ".antigravity.localharness.InputEvent",
    ".antigravity.localharness.OutputEvent",
]

# ---------------------------------------------------------------------------
# Minimal protobuf wire reader (descriptor.proto subset)
# ---------------------------------------------------------------------------

WIRE_VARINT, WIRE_64, WIRE_LEN, WIRE_SGROUP, WIRE_EGROUP, WIRE_32 = 0, 1, 2, 3, 4, 5


def read_varint(buf: bytes, i: int) -> tuple[int, int]:
    val = 0
    shift = 0
    while True:
        b = buf[i]
        i += 1
        val |= (b & 0x7F) << shift
        if not b & 0x80:
            return val, i
        shift += 7


def parse_fields(buf: bytes) -> dict[int, list]:
    """Decode a protobuf message into {field_number: [raw values]}."""
    out: dict[int, list] = {}
    i = 0
    n = len(buf)
    while i < n:
        key, i = read_varint(buf, i)
        fnum, wt = key >> 3, key & 7
        if wt == WIRE_VARINT:
            val, i = read_varint(buf, i)
        elif wt == WIRE_LEN:
            ln, i = read_varint(buf, i)
            val = buf[i : i + ln]
            i += ln
        elif wt == WIRE_64:
            val = buf[i : i + 8]
            i += 8
        elif wt == WIRE_32:
            val = buf[i : i + 4]
            i += 4
        else:  # groups — not present in descriptor.proto
            raise ValueError(f"unsupported wire type {wt}")
        out.setdefault(fnum, []).append(val)
    return out


def s(fields: dict[int, list], num: int) -> str | None:
    v = fields.get(num)
    return v[0].decode("utf-8") if v else None


def i32(fields: dict[int, list], num: int) -> int | None:
    v = fields.get(num)
    return v[0] if v else None


def msgs(fields: dict[int, list], num: int) -> list[dict[int, list]]:
    return [parse_fields(b) for b in fields.get(num, [])]


# descriptor.proto FieldDescriptorProto.Type
TYPE_NAMES = {
    1: "double",
    2: "float",
    3: "int64",
    4: "uint64",
    5: "int32",
    6: "fixed64",
    7: "fixed32",
    8: "bool",
    9: "string",
    10: "group",
    11: "message",
    12: "bytes",
    13: "uint32",
    14: "enum",
    15: "sfixed32",
    16: "sfixed64",
    17: "sint32",
    18: "sint64",
}

LABEL_REPEATED = 3

# Well-known types the closure touches. protojson gives `Duration` a string form
# ("1.5s"); the rest are free-form JSON. None of them need generated structs.
WKT_MESSAGES = {
    ".google.protobuf.Duration": "String",
    ".google.protobuf.Timestamp": "String",
    ".google.protobuf.Any": "serde_json::Value",
    ".google.protobuf.Struct": "serde_json::Value",
    ".google.protobuf.Value": "serde_json::Value",
    ".google.protobuf.ListValue": "serde_json::Value",
}

# `google.protobuf.NullValue` has exactly one member; synthesised rather than
# pulled in, so the crate needs no descriptor for descriptor.proto itself.
NULL_VALUE_DESCRIPTOR = b"\n\tNullValue\x12\x0e\n\nNULL_VALUE\x10\x00"


class Field:
    def __init__(self, fd: dict[int, list]):
        self.name = s(fd, 1) or ""
        self.number = i32(fd, 3) or 0
        self.label = i32(fd, 4) or 1
        self.type = TYPE_NAMES[i32(fd, 5) or 9]
        self.type_name = s(fd, 6)
        self.default = s(fd, 7)
        self.oneof_index = i32(fd, 9)
        self.json_name = s(fd, 10) or camel(self.name)

    @property
    def repeated(self) -> bool:
        return self.label == LABEL_REPEATED


class Message:
    def __init__(self, fd: dict[int, list], scope: str):
        self.name = s(fd, 1) or ""
        self.full_name = f"{scope}.{self.name}"
        self.fields = [Field(f) for f in msgs(fd, 2)]
        self.oneofs = [s(o, 1) or "" for o in msgs(fd, 8)]
        opts = msgs(fd, 7)
        self.map_entry = bool(opts and i32(opts[0], 7))
        self.nested = [Message(m, self.full_name) for m in msgs(fd, 3)]
        self.enums = [Enum(e, self.full_name) for e in msgs(fd, 4)]


class Enum:
    def __init__(self, fd: dict[int, list], scope: str):
        self.name = s(fd, 1) or ""
        self.full_name = f"{scope}.{self.name}"
        self.values = [(s(v, 1) or "", i32(v, 2) or 0) for v in msgs(fd, 2)]


class File:
    def __init__(self, blob: bytes):
        fd = parse_fields(blob)
        self.name = s(fd, 1)
        self.package = s(fd, 2) or ""
        scope = f".{self.package}" if self.package else ""
        self.messages = [Message(m, scope) for m in msgs(fd, 4)]
        self.enums = [Enum(e, scope) for e in msgs(fd, 5)]


# ---------------------------------------------------------------------------
# Naming
# ---------------------------------------------------------------------------


def camel(snake: str) -> str:
    head, *rest = snake.split("_")
    return head + "".join(p[:1].upper() + p[1:] for p in rest)


def pascal(name: str) -> str:
    return "".join(p[:1].upper() + p[1:] for p in re.split(r"[_\s]+", name) if p)


RUST_KEYWORDS = {
    "as", "box", "break", "const", "continue", "crate", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
    "move", "mut", "pub", "ref", "return", "self", "static", "struct", "super",
    "trait", "true", "type", "unsafe", "use", "where", "while", "async", "await",
    "dyn", "abstract", "final", "override", "macro", "yield",
}


def field_ident(name: str) -> str:
    return f"r#{name}" if name in RUST_KEYWORDS else name


def rust_type_name(full_name: str) -> str:
    """`.antigravity.localharness.StepUpdate.State` -> `StepUpdateState`."""
    parts = full_name.lstrip(".").split(".")
    # Drop the package components (lowercase by convention); keep type nesting.
    types = [p for p in parts if p[:1].isupper()]
    return "".join(pascal(p) for p in types)


def enum_variant(enum_name: str, value_name: str) -> str:
    """`State`/`STATE_ACTIVE` -> `Active`; unprefixed values pass through."""
    prefix = re.sub(r"(?<!^)(?=[A-Z])", "_", enum_name).upper() + "_"
    stripped = value_name[len(prefix):] if value_name.startswith(prefix) else value_name
    if not stripped:
        stripped = value_name
    return pascal(stripped.lower())


# ---------------------------------------------------------------------------
# Type graph
# ---------------------------------------------------------------------------


class Registry:
    def __init__(self, files: list[File]):
        self.messages: dict[str, Message] = {}
        self.enums: dict[str, Enum] = {}
        for f in files:
            for m in f.messages:
                self._add_message(m)
            for e in f.enums:
                self.enums[e.full_name] = e
        null_value = Enum(parse_fields(NULL_VALUE_DESCRIPTOR), ".google.protobuf")
        self.enums[null_value.full_name] = null_value

    def _add_message(self, m: Message) -> None:
        self.messages[m.full_name] = m
        for n in m.nested:
            self._add_message(n)
        for e in m.enums:
            self.enums[e.full_name] = e

    def map_value(self, type_name: str) -> Field | None:
        """If `type_name` names a synthetic map entry, return its value field."""
        m = self.messages.get(type_name)
        if m and m.map_entry:
            return next(f for f in m.fields if f.name == "value")
        return None

    def reachable(self, roots: list[str]) -> tuple[set[str], set[str]]:
        seen_m: set[str] = set()
        seen_e: set[str] = set()
        stack = list(roots)
        while stack:
            name = stack.pop()
            if name in seen_m or name not in self.messages:
                continue
            seen_m.add(name)
            for f in self.messages[name].fields:
                if f.type == "message" and f.type_name and f.type_name not in WKT_MESSAGES:
                    stack.append(f.type_name)
                elif f.type == "enum" and f.type_name:
                    seen_e.add(f.type_name)
        return seen_m, seen_e

    def reaches(self, start: str, target: str) -> bool:
        """Can `start` reach `target` through singular message fields only?

        Repeated and map fields already own their storage on the heap, so they
        break a cycle; only singular message edges force a `Box`.
        """
        seen: set[str] = set()
        stack = [start]
        while stack:
            cur = stack.pop()
            if cur == target and cur != start or cur in seen:
                if cur == target:
                    return True
                continue
            seen.add(cur)
            m = self.messages.get(cur)
            if not m:
                continue
            for f in m.fields:
                if f.type == "message" and f.type_name and not f.repeated:
                    if f.type_name in WKT_MESSAGES or self.map_value(f.type_name):
                        continue
                    if f.type_name == target:
                        return True
                    stack.append(f.type_name)
        return False


# ---------------------------------------------------------------------------
# Emitters
# ---------------------------------------------------------------------------

SCALARS = {
    "double": "f64",
    "float": "f32",
    "int32": "i32",
    "sint32": "i32",
    "sfixed32": "i32",
    "uint32": "u32",
    "fixed32": "u32",
    "int64": "i64",
    "sint64": "i64",
    "sfixed64": "i64",
    "uint64": "u64",
    "fixed64": "u64",
    "bool": "bool",
    "string": "String",
    "bytes": "Vec<u8>",
}

# protojson encodes 64-bit integers as JSON strings; these need a tolerant codec.
BIG_INTS = {"int64", "sint64", "sfixed64", "uint64", "fixed64"}


def field_rust_type(reg: Registry, f: Field) -> str:
    if f.type == "message":
        if f.type_name in WKT_MESSAGES:
            return WKT_MESSAGES[f.type_name]
        return rust_type_name(f.type_name or "")
    if f.type == "enum":
        return rust_type_name(f.type_name or "")
    return SCALARS[f.type]


def emit_message(reg: Registry, m: Message) -> str:
    lines: list[str] = []
    rname = rust_type_name(m.full_name)
    doc = f"/// `{m.full_name.lstrip('.')}`"
    lines.append(doc)
    lines.append("#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]")
    lines.append('#[serde(rename_all = "camelCase")]')
    lines.append(f"pub struct {rname} {{")

    for f in m.fields:
        lines.extend(emit_field(reg, m, f))

    lines.append("}")
    lines.append("")

    # A protobuf `oneof` is flattened on the wire (protojson emits just the set
    # arm as a normal member), so it stays a set of Option fields here and gets
    # an owned view enum for ergonomic matching.
    for idx, oneof in enumerate(m.oneofs):
        arms = [f for f in m.fields if f.oneof_index == idx]
        if not arms:
            continue
        lines.extend(emit_oneof(reg, m, oneof, arms))

    return "\n".join(lines)


def emit_field(reg: Registry, m: Message, f: Field) -> list[str]:
    ident = field_ident(f.name)
    attrs: list[str] = []
    rename = f.json_name
    if rename != camel(f.name):
        attrs.append(f'rename = "{rename}"')
    if f.name != rename:
        attrs.append(f'alias = "{f.name}"')

    map_value = reg.map_value(f.type_name) if f.type == "message" and f.type_name else None

    if map_value is not None:
        vt = field_rust_type(reg, map_value)
        ty = f"HashMap<String, {vt}>"
        attrs += ["default", 'skip_serializing_if = "HashMap::is_empty"']
    elif f.repeated:
        inner = field_rust_type(reg, f)
        ty = f"Vec<{inner}>"
        attrs += ["default", 'skip_serializing_if = "Vec::is_empty"']
        if f.type in BIG_INTS:
            attrs.append('with = "crate::wire::vec_int"')
        elif f.type == "bytes":
            attrs.append('with = "crate::wire::vec_bytes"')
    else:
        inner = field_rust_type(reg, f)
        if f.type == "message" and reg.reaches(f.type_name or "", m.full_name):
            inner = f"Box<{inner}>"
        ty = f"Option<{inner}>"
        attrs += ["default", 'skip_serializing_if = "Option::is_none"']
        if f.type in BIG_INTS:
            attrs.append('with = "crate::wire::opt_int"')
        elif f.type == "bytes":
            attrs.append('with = "crate::wire::opt_bytes"')

    out = []
    if f.default:
        out.append(f"    /// Proto default: `{f.default}`.")
    out.append("    #[serde(" + ", ".join(attrs) + ")]")
    out.append(f"    pub {ident}: {ty},")
    return out


def emit_oneof(reg: Registry, m: Message, oneof: str, arms: list[Field]) -> list[str]:
    ename = rust_type_name(m.full_name) + pascal(oneof)
    # A nested proto type can flatten to the same Rust name as the oneof view
    # enum (e.g. `VideoContent.Processing` vs the `processing` oneof); suffix
    # the synthetic view enum to keep the real type's name.
    taken = {rust_type_name(n) for n in (*reg.messages, *reg.enums)}
    if ename in taken:
        ename += "Oneof"
    mname = rust_type_name(m.full_name)
    lines = [
        f"/// The `{oneof}` oneof of [`{mname}`], as an owned value.",
        "#[derive(Debug, Clone, PartialEq)]",
        f"pub enum {ename} {{",
    ]
    for f in arms:
        inner = field_rust_type(reg, f)
        if f.type == "message" and reg.reaches(f.type_name or "", m.full_name):
            inner = f"Box<{inner}>"
        lines.append(f"    {pascal(f.name)}({inner}),")
    lines.append("}")
    lines.append("")
    lines.append(f"impl {mname} {{")
    lines.append(f"    /// Takes the set arm of the `{oneof}` oneof, if any.")
    lines.append(f"    pub fn into_{oneof}(self) -> Option<{ename}> {{")
    for f in arms:
        lines.append(f"        if let Some(v) = self.{field_ident(f.name)} {{")
        lines.append(f"            return Some({ename}::{pascal(f.name)}(v));")
        lines.append("        }")
    lines.append("        None")
    lines.append("    }")
    lines.append("")
    lines.append(f"    /// True when any arm of the `{oneof}` oneof is set.")
    lines.append(f"    pub fn has_{oneof}(&self) -> bool {{")
    checks = " || ".join(f"self.{field_ident(f.name)}.is_some()" for f in arms)
    lines.append(f"        {checks}")
    lines.append("    }")
    lines.append("}")
    lines.append("")
    return lines


def emit_enum(e: Enum) -> str:
    rname = rust_type_name(e.full_name)
    lines = [
        f"/// `{e.full_name.lstrip('.')}`",
        "#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]",
        f"pub enum {rname} {{",
    ]
    seen: set[str] = set()
    variants: list[tuple[str, str]] = []
    for value_name, number in e.values:
        v = enum_variant(e.name, value_name)
        if v in seen:
            v = pascal(value_name.lower())
        seen.add(v)
        variants.append((v, value_name))
        if number == 0:
            lines.append("    #[default]")
        lines.append(f"    {v},")
    lines.append("    /// A value this crate does not know about yet.")
    lines.append("    ///")
    lines.append("    /// The harness is versioned independently of this crate, so an")
    lines.append("    /// unrecognised enum value is treated as forward compatibility rather")
    lines.append("    /// than as a decode error.")
    lines.append("    Unknown(String),")
    lines.append("}")
    lines.append("")
    lines.append(f"impl {rname} {{")
    lines.append("    /// The protobuf-JSON spelling of this value.")
    lines.append("    pub fn as_str(&self) -> &str {")
    lines.append("        match self {")
    for v, wire in variants:
        lines.append(f'            Self::{v} => "{wire}",')
    lines.append("            Self::Unknown(s) => s,")
    lines.append("        }")
    lines.append("    }")
    lines.append("}")
    lines.append("")
    lines.append(f"impl std::fmt::Display for {rname} {{")
    lines.append("    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {")
    lines.append("        f.write_str(self.as_str())")
    lines.append("    }")
    lines.append("}")
    lines.append("")
    lines.append(f"impl Serialize for {rname} {{")
    lines.append("    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {")
    lines.append("        s.serialize_str(self.as_str())")
    lines.append("    }")
    lines.append("}")
    lines.append("")
    lines.append(f"impl<'de> Deserialize<'de> for {rname} {{")
    lines.append("    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {")
    lines.append("        Ok(match crate::wire::EnumRepr::deserialize(d)? {")
    for v, wire in variants:
        lines.append(f'            crate::wire::EnumRepr::Name(n) if n == "{wire}" => Self::{v},')
    for v, wire in variants:
        number = dict((n, num) for n, num in e.values)[wire]
        lines.append(f"            crate::wire::EnumRepr::Number({number}) => Self::{v},")
    lines.append("            crate::wire::EnumRepr::Name(n) => Self::Unknown(n),")
    lines.append("            crate::wire::EnumRepr::Number(n) => Self::Unknown(n.to_string()),")
    lines.append("        })")
    lines.append("    }")
    lines.append("}")
    lines.append("")
    return "\n".join(lines)


HEADER = """// AUTO-GENERATED by scripts/codegen_antigravity.py — DO NOT EDIT BY HAND.
// Run `python3 scripts/codegen_antigravity.py` to regenerate.
//
// Source: the `FileDescriptorProto` embedded in `localharness_pb2.py` from the
// `google-antigravity` wheel, snapshotted under `tests/schemas/`.
//
// The harness speaks protobuf-JSON over a WebSocket, so these are plain serde
// types shaped like that encoding: `camelCase` members, 64-bit integers as
// strings, `bytes` as base64, enums as their proto value names, and a protobuf
// `oneof` flattened into sibling `Option` members (with a generated view enum).

#![allow(clippy::large_enum_variant)]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
"""


def generate(reg: Registry, msg_names: set[str], enum_names: set[str]) -> str:
    parts = [HEADER]
    for name in sorted(enum_names, key=rust_type_name):
        parts.append(emit_enum(reg.enums[name]))
    for name in sorted(msg_names, key=rust_type_name):
        m = reg.messages[name]
        if m.map_entry:
            continue
        parts.append(emit_message(reg, m))
    return "\n".join(parts).rstrip() + "\n"


# ---------------------------------------------------------------------------
# Descriptor extraction
# ---------------------------------------------------------------------------

SERIALIZED_RE = re.compile(rb"AddSerializedFile\((b'.*?')\)", re.DOTALL)


def extract_descriptor(pb2_source: bytes) -> bytes:
    match = SERIALIZED_RE.search(pb2_source)
    if not match:
        raise SystemExit("could not find AddSerializedFile(...) in pb2 module")
    return ast.literal_eval(match.group(1).decode("latin-1"))


def snapshot_from_wheel(wheel: pathlib.Path) -> None:
    SCHEMAS.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(wheel) as zf:
        for stem, member in PB2_FILES:
            blob = extract_descriptor(zf.read(member))
            (SCHEMAS / f"{stem}.descriptor.bin").write_bytes(blob)
            print(f"snapshotted {stem}.descriptor.bin ({len(blob)} bytes)")


def rustfmt(source: str) -> str:
    """Runs the emitted source through rustfmt, if it is installed.

    Without this, `cargo fmt --all` and `--check` disagree the moment rustfmt
    reflows anything the generator laid out differently. Missing rustfmt is not
    fatal — the output is still valid Rust, just not canonically formatted.
    """
    try:
        result = subprocess.run(
            ["rustfmt", "--edition", "2021", "--emit", "stdout", "--quiet"],
            input=source,
            capture_output=True,
            text=True,
            check=True,
        )
    except (FileNotFoundError, subprocess.CalledProcessError) as e:
        print(f"warning: rustfmt unavailable ({e}); emitting unformatted", file=sys.stderr)
        return source
    return result.stdout


def load_files() -> list[File]:
    files = []
    for stem, _ in PB2_FILES:
        path = SCHEMAS / f"{stem}.descriptor.bin"
        if not path.exists():
            raise SystemExit(f"missing {path}; re-run with --wheel")
        files.append(File(path.read_bytes()))
    return files


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--wheel", type=pathlib.Path, help="re-snapshot descriptors from this wheel")
    ap.add_argument("--check", action="store_true", help="fail if the emitted file would change")
    args = ap.parse_args()

    if args.wheel:
        snapshot_from_wheel(args.wheel)

    reg = Registry(load_files())
    msg_names, enum_names = reg.reachable(ROOTS)
    rendered = rustfmt(generate(reg, msg_names, enum_names))

    GEN.mkdir(parents=True, exist_ok=True)
    target = GEN / "types.rs"
    if args.check:
        current = target.read_text() if target.exists() else ""
        if current != rendered:
            print("protocol_generated/types.rs is stale — re-run codegen", file=sys.stderr)
            sys.exit(1)
        print("types.rs up to date")
        return

    target.write_text(rendered)
    (GEN / "mod.rs").write_text(
        "// AUTO-GENERATED by scripts/codegen_antigravity.py — DO NOT EDIT BY HAND.\npub mod types;\n"
    )
    print(f"wrote {target} — {len(msg_names)} messages, {len(enum_names)} enums")


if __name__ == "__main__":
    main()
