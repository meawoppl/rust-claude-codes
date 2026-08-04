#!/usr/bin/env python3
"""Generate hermes-codes protocol types from the vendored ACP schema.

Reads the committed snapshots (the drift-checked source of truth):

    hermes-codes/tests/schemas/acp_schema.unstable.json
    hermes-codes/tests/schemas/acp_meta.unstable.json

and writes:

    hermes-codes/src/protocol_generated/mod.rs
    hermes-codes/src/protocol_generated/types.rs
    hermes-codes/src/protocol_generated/methods.rs

The unstable schema variants are what hermes-agent actually speaks: its
pinned Python SDK (agent-client-protocol) generates its models from
schema.unstable.json / meta.unstable.json at the tag recorded in
tests/schemas/hermes_acp_provenance.json.

Deterministic: same inputs -> byte-identical outputs. Regenerating must
produce no diff against the committed files (CI enforces this).
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
SCHEMA_DIR = ROOT / "hermes-codes" / "tests" / "schemas"
OUT_DIR = ROOT / "hermes-codes" / "src" / "protocol_generated"

SCHEMA = json.loads((SCHEMA_DIR / "acp_schema.unstable.json").read_text())
META = json.loads((SCHEMA_DIR / "acp_meta.unstable.json").read_text())
DEFS: dict[str, Any] = SCHEMA["$defs"]

# JSON-RPC envelope machinery is hand-written in src/protocol.rs; the schema
# marks these defs x-docs-ignore and we skip them here.
SKIPPED = sorted(n for n, d in DEFS.items() if isinstance(d, dict) and d.get("x-docs-ignore"))

# Method -> (params type, response type) — validated against $defs below so
# an upstream rename fails generation instead of drifting silently.
# `None` response = notification (no reply expected).
AGENT_METHOD_TYPES: dict[str, tuple[str, str | None]] = {
    "initialize": ("InitializeRequest", "InitializeResponse"),
    "authenticate": ("AuthenticateRequest", "AuthenticateResponse"),
    "session/new": ("NewSessionRequest", "NewSessionResponse"),
    "session/load": ("LoadSessionRequest", "LoadSessionResponse"),
    "session/list": ("ListSessionsRequest", "ListSessionsResponse"),
    "session/fork": ("ForkSessionRequest", "ForkSessionResponse"),
    "session/resume": ("ResumeSessionRequest", "ResumeSessionResponse"),
    "session/close": ("CloseSessionRequest", "CloseSessionResponse"),
    "session/prompt": ("PromptRequest", "PromptResponse"),
    "session/set_mode": ("SetSessionModeRequest", "SetSessionModeResponse"),
    "session/set_model": ("SetSessionModelRequest", "SetSessionModelResponse"),
    "session/set_config_option": (
        "SetSessionConfigOptionRequest",
        "SetSessionConfigOptionResponse",
    ),
    "session/cancel": ("CancelNotification", None),
}
CLIENT_METHOD_TYPES: dict[str, tuple[str, str | None]] = {
    "fs/read_text_file": ("ReadTextFileRequest", "ReadTextFileResponse"),
    "fs/write_text_file": ("WriteTextFileRequest", "WriteTextFileResponse"),
    "session/request_permission": (
        "RequestPermissionRequest",
        "RequestPermissionResponse",
    ),
    "session/update": ("SessionNotification", None),
    "terminal/create": ("CreateTerminalRequest", "CreateTerminalResponse"),
    "terminal/output": ("TerminalOutputRequest", "TerminalOutputResponse"),
    "terminal/kill": ("KillTerminalRequest", "KillTerminalResponse"),
    "terminal/release": ("ReleaseTerminalRequest", "ReleaseTerminalResponse"),
    "terminal/wait_for_exit": (
        "WaitForTerminalExitRequest",
        "WaitForTerminalExitResponse",
    ),
}


def validate_method_tables() -> None:
    meta_agent = set(META["agentMethods"].values())
    meta_client = set(META["clientMethods"].values())
    if meta_agent != set(AGENT_METHOD_TYPES):
        raise SystemExit(
            f"agent method drift vs meta.unstable.json:\n"
            f"  meta only: {sorted(meta_agent - set(AGENT_METHOD_TYPES))}\n"
            f"  table only: {sorted(set(AGENT_METHOD_TYPES) - meta_agent)}"
        )
    if meta_client != set(CLIENT_METHOD_TYPES):
        raise SystemExit(
            f"client method drift vs meta.unstable.json:\n"
            f"  meta only: {sorted(meta_client - set(CLIENT_METHOD_TYPES))}\n"
            f"  table only: {sorted(set(CLIENT_METHOD_TYPES) - meta_client)}"
        )
    for table in (AGENT_METHOD_TYPES, CLIENT_METHOD_TYPES):
        for method, (req, resp) in table.items():
            for t in (req, resp):
                if t is not None and t not in DEFS:
                    raise SystemExit(f"{method}: type {t} not in $defs (upstream rename?)")


# ──────────────────────────────────────────────────────────────────────────
# Rust name / doc helpers
# ──────────────────────────────────────────────────────────────────────────

def to_snake(name: str) -> str:
    s = re.sub(r"(.)([A-Z][a-z]+)", r"\1_\2", name)
    s = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", s)
    return s.replace("-", "_").lower()


RUST_KEYWORDS = {"type", "ref", "match", "move", "box", "loop", "in", "fn", "mod", "use"}


def field_ident(wire_name: str) -> str:
    ident = to_snake(wire_name.lstrip("_"))
    if ident in RUST_KEYWORDS:
        ident += "_"
    return ident


def variant_ident(tag: str) -> str:
    parts = re.split(r"[^A-Za-z0-9]", tag)
    ident = "".join(p[:1].upper() + p[1:] for p in parts if p)
    if ident and ident[0].isdigit():
        ident = "_" + ident
    return ident or "Unknown"


def doc_lines(schema: dict[str, Any], indent: str = "") -> list[str]:
    desc = schema.get("description")
    if not desc:
        return []
    out = []
    for line in str(desc).splitlines():
        out.append(f"{indent}/// {line}".rstrip())
    return out


# ──────────────────────────────────────────────────────────────────────────
# Schema classification
# ──────────────────────────────────────────────────────────────────────────

def is_null(s: Any) -> bool:
    return isinstance(s, dict) and s.get("type") == "null"


def single_ref(s: dict[str, Any]) -> str | None:
    """`{"$ref": X}` or `{"allOf": [{"$ref": X}], ...}` -> def name."""
    if "$ref" in s:
        return s["$ref"].rsplit("/", 1)[-1]
    all_of = s.get("allOf")
    if isinstance(all_of, list) and len(all_of) == 1 and "$ref" in all_of[0]:
        return all_of[0]["$ref"].rsplit("/", 1)[-1]
    return None


def tagged_union_tag(d: dict[str, Any]) -> str | None:
    """For a oneOf def, the shared const-discriminator property, if any."""
    variants = d.get("oneOf")
    if not isinstance(variants, list) or not variants:
        return None
    tag: str | None = None
    for v in variants:
        props = v.get("properties") or {}
        consts = [k for k, p in props.items() if isinstance(p, dict) and "const" in p]
        if len(consts) != 1:
            return None
        if tag is None:
            tag = consts[0]
        elif tag != consts[0]:
            return None
    return tag


# ──────────────────────────────────────────────────────────────────────────
# Type mapping for property schemas
# ──────────────────────────────────────────────────────────────────────────

INT_FORMATS = {
    "uint8": "u8",
    "uint16": "u16",
    "uint32": "u32",
    "uint64": "u64",
    "uint": "u64",
    "int8": "i8",
    "int16": "i16",
    "int32": "i32",
    "int64": "i64",
    "int": "i64",
}


def schema_to_rust(s: Any) -> str:
    """Map a property schema to a Rust type. Unknown shapes fall back to
    serde_json::Value so generation never silently drops a field."""
    if not isinstance(s, dict) or not s:
        return "Value"
    ref = single_ref(s)
    if ref:
        return rust_alias_or_name(ref)
    any_of = s.get("anyOf")
    if isinstance(any_of, list):
        non_null = [v for v in any_of if not is_null(v)]
        if len(non_null) == 1 and len(any_of) == 2:
            return f"Option<{schema_to_rust(non_null[0])}>"
        return "Value"
    if "oneOf" in s:
        return "Value"
    ty = s.get("type")
    if isinstance(ty, list):
        non_null = [t for t in ty if t != "null"]
        if len(non_null) == 1 and len(ty) == 2:
            inner = schema_to_rust({**s, "type": non_null[0]})
            return f"Option<{inner}>"
        return "Value"
    if ty == "string":
        return "String"
    if ty == "boolean":
        return "bool"
    if ty == "integer":
        return INT_FORMATS.get(str(s.get("format")), "i64")
    if ty == "number":
        return "f64"
    if ty == "array":
        return f"Vec<{schema_to_rust(s.get('items'))}>"
    if ty == "object":
        ap = s.get("additionalProperties")
        if isinstance(ap, dict) and ap:
            return f"std::collections::BTreeMap<String, {schema_to_rust(ap)}>"
        return "serde_json::Map<String, Value>"
    return "Value"


def rust_alias_or_name(name: str) -> str:
    """Defs that degrade to aliases keep their name via `pub type`."""
    return name


def is_defaultable(rs_type: str) -> bool:
    if rs_type in {"String", "bool", "f64", "Value"} or rs_type in set(INT_FORMATS.values()):
        return True
    return rs_type.startswith(("Option<", "Vec<", "std::collections::BTreeMap<", "serde_json::Map<"))


# ──────────────────────────────────────────────────────────────────────────
# Emitters
# ──────────────────────────────────────────────────────────────────────────

def emit_struct(name: str, d: dict[str, Any]) -> str:
    props: dict[str, Any] = d.get("properties") or {}
    required = set(d.get("required") or [])
    body: list[str] = []
    all_default = True
    for wire in sorted(props):
        ps = props[wire]
        if isinstance(ps, dict) and "const" in ps:
            # Tag properties only appear inside union variants, not here.
            raise SystemExit(f"{name}.{wire}: unexpected const property on struct")
        ident = field_ident(wire)
        rs_type = schema_to_rust(ps)
        is_opt = rs_type.startswith("Option<") or wire not in required
        if is_opt and not rs_type.startswith("Option<"):
            rs_type = f"Option<{rs_type}>"
        attrs = []
        # rename_all = camelCase covers snake->camel; anything it wouldn't
        # reproduce (e.g. `_meta`) gets an explicit rename.
        if to_camel(ident) != wire:
            attrs.append(f'rename = "{wire}"')
        if is_opt:
            attrs.append("default")
            attrs.append('skip_serializing_if = "Option::is_none"')
        else:
            # Required fields stay strict — NO #[serde(default)]. The ACP
            # schema is generated from the reference Rust types, so required
            # is exact; and leniency here would break untagged-union variant
            # selection (a defaulted discriminating field makes the first
            # variant match anything).
            all_default = False
        body.extend(doc_lines(ps if isinstance(ps, dict) else {}, "    "))
        if attrs:
            body.append("    #[serde(" + ", ".join(attrs) + ")]")
        body.append(f"    pub {ident}: {rs_type},")
    out = doc_lines(d)
    derives = "Debug, Clone, PartialEq, Serialize, Deserialize"
    if all_default:
        derives += ", Default"
    out.append(f"#[derive({derives})]")
    out.append('#[serde(rename_all = "camelCase")]')
    out.append(f"pub struct {name} {{")
    if not props:
        out.append(
            '    #[serde(flatten, default, skip_serializing_if = "serde_json::Map::is_empty")]'
        )
        out.append("    pub extra: serde_json::Map<String, Value>,")
    out.extend(body)
    out.append("}")
    return "\n".join(out)


def to_camel(snake: str) -> str:
    parts = snake.split("_")
    return parts[0] + "".join(p[:1].upper() + p[1:] for p in parts[1:])


def emit_tagged_enum(name: str, d: dict[str, Any], tag: str) -> str:
    out = doc_lines(d)
    out.append("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]")
    out.append(f'#[serde(tag = "{tag}")]')
    out.append(f"pub enum {name} {{")
    for v in d["oneOf"]:
        props = v.get("properties") or {}
        tag_value = props[tag]["const"]
        payload = single_ref(v)
        others = {k: p for k, p in props.items() if k != tag}
        out.extend(doc_lines(v, "    "))
        out.append(f'    #[serde(rename = "{tag_value}")]')
        ident = variant_ident(str(tag_value))
        if payload:
            out.append(f"    {ident}({payload}),")
        elif others:
            required = set(v.get("required") or [])
            out.append(f"    {ident} {{")
            for wire in sorted(others):
                ident_f = field_ident(wire)
                rs_type = schema_to_rust(others[wire])
                if rs_type.startswith("Option<") or wire not in required:
                    if not rs_type.startswith("Option<"):
                        rs_type = f"Option<{rs_type}>"
                rename = f'#[serde(rename = "{wire}")] ' if to_camel(ident_f) != wire else ""
                out.append(f"        {rename}{ident_f}: {rs_type},")
            out.append("    },")
        else:
            out.append(f"    {ident},")
    out.append("}")
    return "\n".join(out)


def emit_untagged_enum(name: str, d: dict[str, Any]) -> str | None:
    """anyOf of pure refs -> untagged enum; None when the shape is gnarlier."""
    variants = d.get("anyOf") or []
    refs = [single_ref(v) for v in variants if not is_null(v)]
    if not refs or any(r is None for r in refs):
        return None
    nullable = any(is_null(v) for v in variants)
    if nullable:
        return None
    out = doc_lines(d)
    out.append("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]")
    out.append("#[serde(untagged)]")
    out.append(f"pub enum {name} {{")
    for r in refs:
        out.append(f"    {r}({r}),")
    out.append("}")
    return "\n".join(out)


def emit_string_newtype(name: str, d: dict[str, Any]) -> str:
    out = doc_lines(d)
    out.append(
        "#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]"
    )
    out.append("#[serde(transparent)]")
    out.append(f"pub struct {name}(pub String);")
    out.append("")
    out.append(f"impl std::fmt::Display for {name} {{")
    out.append("    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {")
    out.append("        self.0.fmt(f)")
    out.append("    }")
    out.append("}")
    out.append("")
    out.append(f"impl From<String> for {name} {{")
    out.append(f"    fn from(s: String) -> Self {{ {name}(s) }}")
    out.append("}")
    out.append("")
    out.append(f"impl From<&str> for {name} {{")
    out.append(f"    fn from(s: &str) -> Self {{ {name}(s.to_string()) }}")
    out.append("}")
    return "\n".join(out)


def emit_string_enum(name: str, d: dict[str, Any]) -> str:
    out = doc_lines(d)
    out.append("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]")
    out.append(f"pub enum {name} {{")
    for v in d["enum"]:
        out.append(f'    #[serde(rename = "{v}")]')
        out.append(f"    {variant_ident(str(v))},")
    out.append("}")
    return "\n".join(out)


def is_string_const(v: Any) -> bool:
    return isinstance(v, dict) and v.get("type") == "string" and "const" in v


def is_open_string(v: Any) -> bool:
    return isinstance(v, dict) and v.get("type") == "string" and "const" not in v and "enum" not in v


def emit_const_string_enum(name: str, d: dict[str, Any], variants: list[dict]) -> str:
    out = doc_lines(d)
    out.append("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]")
    out.append(f"pub enum {name} {{")
    for v in variants:
        out.extend(doc_lines(v, "    "))
        out.append(f'    #[serde(rename = "{v["const"]}")]')
        out.append(f"    {variant_ident(str(v['const']))},")
    out.append("}")
    return "\n".join(out)


def emit_open_string_newtype(name: str, d: dict[str, Any], consts: list[dict]) -> str:
    """String-const variants plus an open-string catch-all: a closed enum
    would reject forward-compatible values the schema explicitly allows, so
    emit a newtype with the well-known values as associated consts."""
    out = emit_string_newtype(name, d).splitlines()
    out.append("")
    out.append(f"impl {name} {{")
    for v in consts:
        cname = re.sub(r"[^A-Z0-9]", "_", str(v["const"]).upper())
        for line in doc_lines(v, "    "):
            out.append(line)
        out.append(f'    pub const {cname}: &str = "{v["const"]}";')
    out.append("}")
    return "\n".join(out)


def emit_mixed_tag_union(name: str, d: dict[str, Any]) -> str | None:
    """anyOf where some variants are const-tag wrappers around a payload ref
    and the rest are plain refs (the tag-less wire default) — e.g. McpServer
    (`type: http|sse` + bare stdio) and AuthMethod. Emits an untagged enum:
    tagged variants first, each with a single-variant tag enum pinning the
    const so untagged matching can't cross-select; bare defaults last."""
    variants = d.get("anyOf") or []
    tagged: list[tuple[str, str, str, dict]] = []  # (tag_prop, const, payload, variant schema)
    bare: list[tuple[str, dict]] = []
    for v in variants:
        if not isinstance(v, dict):
            return None
        props = v.get("properties") or {}
        consts = {k: p for k, p in props.items() if isinstance(p, dict) and "const" in p}
        payload = single_ref(v)
        if consts:
            if len(consts) != 1 or payload is None or set(props) != set(consts):
                return None
            (tag_prop, tag_schema) = next(iter(consts.items()))
            tagged.append((tag_prop, str(tag_schema["const"]), payload, v))
        elif payload:
            bare.append((payload, v))
        else:
            return None
    if not tagged or len({t[0] for t in tagged}) != 1:
        return None
    tag_prop = tagged[0][0]
    out = doc_lines(d)
    out.append("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]")
    out.append("#[serde(untagged)]")
    out.append(f"pub enum {name} {{")
    for _, const, payload, v in tagged:
        out.extend(doc_lines(v, "    "))
        out.append(f"    {variant_ident(const)} {{")
        out.append(f'        #[serde(rename = "{tag_prop}")]')
        out.append(f"        kind: {name}{variant_ident(const)}Tag,")
        out.append("        #[serde(flatten)]")
        out.append(f"        inner: {payload},")
        out.append("    },")
    for payload, v in bare:
        out.extend(doc_lines(v, "    "))
        out.append(f"    {variant_ident(payload)}({payload}),")
    out.append("}")
    for _, const, _, _ in tagged:
        out.append("")
        out.append(f'/// Pins the `"{const}"` discriminator of [`{name}::{variant_ident(const)}`].')
        out.append(
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]"
        )
        out.append(f"pub enum {name}{variant_ident(const)}Tag {{")
        out.append("    #[default]")
        out.append(f'    #[serde(rename = "{const}")]')
        out.append(f"    {variant_ident(const)},")
        out.append("}")
    return "\n".join(out)


def emit_titled_untagged_enum(name: str, d: dict[str, Any], variants: list[dict]) -> str | None:
    """anyOf where every variant carries a `title`: untagged enum with
    title-derived variant names and schema-mapped payloads."""
    if not all(isinstance(v, dict) and v.get("title") for v in variants):
        return None
    out = doc_lines(d)
    out.append("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]")
    out.append("#[serde(untagged)]")
    out.append(f"pub enum {name} {{")
    for v in variants:
        ident = variant_ident(str(v["title"]))
        out.extend(doc_lines(v, "    "))
        if is_null(v):
            out.append(f"    {ident},")
        else:
            out.append(f"    {ident}({schema_to_rust(v)}),")
    out.append("}")
    return "\n".join(out)


# The one schema shape that needs custom serde: a struct with a FLATTENED
# untagged value union whose default variant has no tag on the wire.
HANDWRITTEN: dict[str, str] = {
    "SetSessionConfigOptionRequest": '''\
/// Request parameters for setting a session configuration option.
///
/// The value payload is flattened: `type: "boolean"` selects the boolean
/// shape; an absent or unknown `type` with a string payload is a
/// [`SessionConfigValueId`] selection (the wire default).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetSessionConfigOptionRequest {
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Map<String, Value>>,
    /// The ID of the configuration option to set.
    pub config_id: SessionConfigId,
    /// The ID of the session to set the configuration option for.
    pub session_id: SessionId,
    #[serde(flatten)]
    pub value: SessionConfigValue,
}

/// Flattened value payload of [`SetSessionConfigOptionRequest`].
///
/// Variant order matters for untagged deserialization: the tagged boolean
/// shape must be tried before the tag-less value-id default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SessionConfigValue {
    /// A boolean value (`type: "boolean"`).
    Boolean {
        /// Always `"boolean"` on the wire.
        #[serde(rename = "type")]
        kind: String,
        /// The boolean value.
        value: bool,
    },
    /// A value-id string selection — the default when `type` is absent.
    ValueId {
        /// The value ID.
        value: SessionConfigValueId,
    },
}

impl SessionConfigValue {
    /// A well-formed boolean payload (`type: "boolean"`).
    pub fn boolean(value: bool) -> Self {
        SessionConfigValue::Boolean {
            kind: "boolean".to_string(),
            value,
        }
    }
}''',
}


def emit_hybrid_base_union(name: str, d: dict[str, Any], tag: str) -> str:
    """oneOf + sibling `properties`: shared base fields alongside a tagged
    payload (e.g. SessionConfigOption's id/name + select|boolean payload).
    Emits a base struct with a flattened `{Name}Payload` enum."""
    base = {
        "description": d.get("description"),
        "properties": d.get("properties") or {},
        "required": d.get("required") or [],
    }
    struct_src = emit_struct(name, base)
    # The flattened payload enum has no Default, so the base struct can't
    # derive one either.
    struct_src = struct_src.replace(", Default)]", ")]")
    lines = struct_src.splitlines()
    assert lines[-1] == "}"
    lines.insert(len(lines) - 1, "    /// The type-discriminated payload.")
    lines.insert(len(lines) - 1, "    #[serde(flatten)]")
    lines.insert(len(lines) - 1, f"    pub payload: {name}Payload,")
    payload_doc = {
        "description": f"The `{tag}`-discriminated payload of [`{name}`].",
        "oneOf": d["oneOf"],
    }
    return "\n".join(lines) + "\n\n" + emit_tagged_enum(f"{name}Payload", payload_doc, tag)


def emit_def(name: str, d: dict[str, Any]) -> str:
    if name in HANDWRITTEN:
        return HANDWRITTEN[name]
    if "oneOf" in d:
        variants = d["oneOf"]
        if all(is_string_const(v) for v in variants):
            return emit_const_string_enum(name, d, variants)
        tag = tagged_union_tag(d)
        if tag and d.get("properties"):
            return emit_hybrid_base_union(name, d, tag)
        if tag:
            return emit_tagged_enum(name, d, tag)
        u = emit_untagged_enum(name, {"anyOf": variants, "description": d.get("description")})
        if u:
            return u
        return f"// {name}: unmodeled oneOf shape\npub type {name} = Value;"
    if "anyOf" in d:
        variants = d["anyOf"]
        consts = [v for v in variants if is_string_const(v)]
        if consts and all(is_string_const(v) or is_open_string(v) for v in variants):
            if len(consts) == len(variants):
                return emit_const_string_enum(name, d, variants)
            return emit_open_string_newtype(name, d, consts)
        if variants and all(
            isinstance(v, dict) and v.get("type") == "integer" and "const" in v
            for v in variants
        ):
            rs = INT_FORMATS.get(str(variants[0].get("format")), "i64")
            lines = doc_lines(d)
            for v in variants:
                lines.append(f"/// `{v['const']}` — {str(v.get('title', '')).strip()}")
            lines.append(f"pub type {name} = {rs};")
            return "\n".join(lines)
        non_null = [v for v in variants if not is_null(v)]
        if len(non_null) == 1 and len(variants) == 2:
            inner = schema_to_rust(non_null[0])
            lines = doc_lines(d)
            lines.append(f"pub type {name} = Option<{inner}>;")
            return "\n".join(lines)
        m = emit_mixed_tag_union(name, d)
        if m:
            return m
        u = emit_untagged_enum(name, d)
        if u:
            return u
        t = emit_titled_untagged_enum(name, d, variants)
        if t:
            return t
        return f"// {name}: unmodeled anyOf shape\npub type {name} = Value;"
    ty = d.get("type")
    if ty == "object" or "properties" in d:
        return emit_struct(name, d)
    if ty == "string" and "enum" in d:
        return emit_string_enum(name, d)
    if ty == "string":
        return emit_string_newtype(name, d)
    if ty == "integer":
        rs = INT_FORMATS.get(str(d.get("format")), "i64")
        lines = doc_lines(d)
        lines.append(f"pub type {name} = {rs};")
        return "\n".join(lines)
    if list(d.keys()) in (["description"], []):
        lines = doc_lines(d)
        lines.append(f"pub type {name} = Value;")
        return "\n".join(lines)
    return f"// {name}: unmodeled shape {sorted(d.keys())}\npub type {name} = Value;"


# ──────────────────────────────────────────────────────────────────────────
# File assembly
# ──────────────────────────────────────────────────────────────────────────

HEADER = "// AUTO-GENERATED by scripts/codegen_acp.py — DO NOT EDIT BY HAND.\n"


def emit_types() -> str:
    out = [HEADER]
    out.append("// ACP type definitions generated from the vendored")
    out.append("// acp_schema.unstable.json snapshot (see tests/schemas/")
    out.append("// hermes_acp_provenance.json for the upstream pin chain).")
    out.append("")
    out.append("#![allow(clippy::large_enum_variant, clippy::empty_docs)]")
    out.append("")
    out.append("use serde::{Deserialize, Serialize};")
    out.append("use serde_json::Value;")
    out.append("")
    for name in sorted(DEFS):
        if name in SKIPPED:
            continue
        out.append(emit_def(name, DEFS[name]))
        out.append("")
    return "\n".join(out)


def emit_methods() -> str:
    out = [HEADER]
    out.append("//! ACP method names and their params/response type mapping,")
    out.append("//! generated from acp_meta.unstable.json.")
    out.append("")
    out.append(f"/// ACP protocol version this schema snapshot describes.")
    out.append(f"pub const PROTOCOL_VERSION: u16 = {META['version']};")
    out.append("")

    def const_name(method: str) -> str:
        return re.sub(r"[^A-Z0-9]", "_", method.upper())

    for label, table in (("Agent", AGENT_METHOD_TYPES), ("Client", CLIENT_METHOD_TYPES)):
        out.append(f"// {label} methods ({'client -> agent' if label == 'Agent' else 'agent -> client'})")
        for method in sorted(table):
            out.append(f'pub const {const_name(method)}: &str = "{method}";')
        out.append("")
    for label, table in (("AGENT", AGENT_METHOD_TYPES), ("CLIENT", CLIENT_METHOD_TYPES)):
        recv = "the agent" if label == "AGENT" else "the client"
        out.append(f"/// Methods handled by {recv}: (method, params type, response type).")
        out.append("/// A `None` response type marks a notification.")
        out.append(
            f"pub const {label}_METHODS: &[(&str, &str, Option<&str>)] = &["
        )
        for method in sorted(table):
            req, resp = table[method]
            resp_s = f'Some("{resp}")' if resp else "None"
            out.append(f'    ("{method}", "{req}", {resp_s}),')
        out.append("];")
        out.append("")
    return "\n".join(out)


def emit_mod() -> str:
    out = [HEADER]
    out.append("pub mod methods;")
    out.append("pub mod types;")
    out.append("")
    return "\n".join(out)


def main() -> None:
    validate_method_tables()
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    (OUT_DIR / "types.rs").write_text(emit_types())
    (OUT_DIR / "methods.rs").write_text(emit_methods())
    (OUT_DIR / "mod.rs").write_text(emit_mod())
    total = len([n for n in DEFS if n not in SKIPPED])
    print(f"defs emitted:    {total} of {len(DEFS)} ({len(SKIPPED)} envelope defs hand-written)")
    print(f"agent methods:   {len(AGENT_METHOD_TYPES)}")
    print(f"client methods:  {len(CLIENT_METHOD_TYPES)}")
    print(f"wrote {OUT_DIR}/mod.rs / types.rs / methods.rs")


if __name__ == "__main__":
    main()
