#!/usr/bin/env python3
"""
Generate Rust protocol types + samples for opencode-codes from the OpenAPI 3.1 snapshot.

The snapshot at opencode-codes/tests/schemas/opencode_openapi.json (pulled live from
`GET /doc` of opencode 1.18.5) is the source of truth for every wire type. This script
walks `components.schemas` plus the request/response bodies and parameters of the six
hand-wrapped endpoints and the `/event` SSE union, synthesizes named types for inline
object / union shapes, and writes:

  - opencode-codes/src/protocol_generated/types.rs   (one Rust item per schema)
  - opencode-codes/src/protocol_generated/samples.rs (JSON samples for the six endpoints)
  - opencode-codes/src/protocol_generated/mod.rs     (module index)

OpenAPI 3.1 handling:
  - $ref                                   -> referenced Rust type.
  - type: ["string","null"] / anyOf null   -> Option<T>.
  - anyOf/oneOf of objects sharing a const  -> internally-tagged serde enum on that key;
    string-const variants                      ref variants keep their tag field
                                               #[serde(skip_serializing)] so the tag is
                                               written exactly once.
  - anyOf/oneOf all `type: string`          -> open string enum with Unknown(String)
                                               (the workspace as_str/From/Display/serde
                                               pattern).
  - anyOf/oneOf otherwise                   -> untagged serde enum (branch order preserved).
  - required[]                              -> non-Option; everything else Option.
  - integer (minimum >= 0)                  -> u64, else i64; number -> f64.
  - object w/ additionalProperties schema   -> BTreeMap<String, V>; bare object -> Map.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
SNAPSHOT = ROOT / "opencode-codes" / "tests" / "schemas" / "opencode_openapi.json"
OUT_DIR = ROOT / "opencode-codes" / "src" / "protocol_generated"
DOC = json.loads(SNAPSHOT.read_text())
SCHEMAS: dict[str, Any] = dict(DOC["components"]["schemas"])

# ──────────────────────────────────────────────────────────────────────────
# Synthetic request/response schemas for the six hand-wrapped endpoints.
# The bodies are inline in the spec; naming them gives client authors concrete
# types and anchors the reachable-set closure.
# ──────────────────────────────────────────────────────────────────────────


def _endpoint(path: str, method: str) -> dict[str, Any]:
    return DOC["paths"][path][method]


def _json_body(op: dict[str, Any]) -> Any | None:
    rb = op.get("requestBody") or {}
    return (rb.get("content", {}).get("application/json", {}) or {}).get("schema")


SYNTHETIC_ENDPOINT_TYPES: dict[str, Any] = {}
_create = _json_body(_endpoint("/session", "post"))
if _create:
    SYNTHETIC_ENDPOINT_TYPES["SessionCreateParams"] = _create
_prompt = _json_body(_endpoint("/session/{sessionID}/prompt_async", "post"))
if _prompt:
    SYNTHETIC_ENDPOINT_TYPES["PromptAsyncParams"] = _prompt
_perm = _json_body(_endpoint("/session/{sessionID}/permissions/{permissionID}", "post"))
if _perm:
    SYNTHETIC_ENDPOINT_TYPES["PermissionReplyParams"] = _perm
# GET /session/{sessionID}/message -> array of {info: Message, parts: [Part]}.
_msg_op = _endpoint("/session/{sessionID}/message", "get")
_msg_item = (
    _msg_op.get("responses", {})
    .get("200", {})
    .get("content", {})
    .get("application/json", {})
    .get("schema", {})
    .get("items")
)
if _msg_item:
    SYNTHETIC_ENDPOINT_TYPES["MessageWithParts"] = _msg_item

for _name, _schema in SYNTHETIC_ENDPOINT_TYPES.items():
    SCHEMAS[_name] = _schema

# ──────────────────────────────────────────────────────────────────────────
# Reachable-set closure (for reporting; every schema is emitted regardless).
# ──────────────────────────────────────────────────────────────────────────


def collect_refs(node: Any, into: set[str]) -> None:
    if isinstance(node, dict):
        ref = node.get("$ref")
        if isinstance(ref, str) and ref.startswith("#/components/schemas/"):
            into.add(ref.rsplit("/", 1)[-1])
        for v in node.values():
            collect_refs(v, into)
    elif isinstance(node, list):
        for v in node:
            collect_refs(v, into)


def closure(seed: set[str]) -> set[str]:
    out: set[str] = set()
    frontier = set(seed)
    while frontier:
        n = frontier.pop()
        if n in out or n not in SCHEMAS:
            continue
        out.add(n)
        nxt: set[str] = set()
        collect_refs(SCHEMAS[n], nxt)
        frontier |= nxt - out
    return out


ENDPOINT_SEED: set[str] = set(SYNTHETIC_ENDPOINT_TYPES)
ENDPOINT_SEED |= {"Session", "Message", "Part", "Event", "PermissionRuleset", "OutputFormat"}
for _path, _method in [
    ("/session", "post"),
    ("/session/{sessionID}/prompt_async", "post"),
    ("/session/{sessionID}/message", "get"),
    ("/session/{sessionID}/abort", "post"),
    ("/session/{sessionID}/permissions/{permissionID}", "post"),
    ("/event", "get"),
]:
    collect_refs(_endpoint(_path, _method), ENDPOINT_SEED)
ENDPOINT_SEED &= set(SCHEMAS)
REACHABLE = closure(ENDPOINT_SEED)

# ──────────────────────────────────────────────────────────────────────────
# Identifier helpers.
# ──────────────────────────────────────────────────────────────────────────

KEYWORDS = {
    "type", "ref", "match", "self", "where", "for", "in", "if", "else", "fn",
    "let", "mut", "const", "static", "pub", "use", "mod", "struct", "enum",
    "impl", "trait", "as", "async", "await", "break", "continue", "loop",
    "move", "return", "true", "false", "unsafe", "yield", "box", "dyn", "crate",
    "super", "extern", "abstract", "final", "override", "macro",
}


def to_snake(name: str) -> str:
    s = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", name)
    s = re.sub(r"([a-z\d])([A-Z])", r"\1_\2", s)
    s = s.replace("-", "_").replace(".", "_").replace("/", "_")
    s = re.sub(r"[^a-zA-Z0-9_]", "", s)
    s = s.lower()
    if not s:
        s = "field"
    if s[0].isdigit():
        s = "_" + s
    if s in KEYWORDS:
        s = s + "_"
    return s


def pascal(name: str) -> str:
    # Split on non-alnum and camelCase humps, then capitalize each part.
    spaced = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1 \2", name)
    spaced = re.sub(r"([a-z\d])([A-Z])", r"\1 \2", spaced)
    parts = [p for p in re.split(r"[^A-Za-z0-9]+", spaced) if p]
    ident = "".join(p[:1].upper() + p[1:] for p in parts)
    if not ident:
        ident = "Unknown"
    if ident[0].isdigit():
        ident = "N" + ident
    if ident in KEYWORDS:
        ident = ident + "_"
    return ident


# Deterministic Rust type name per top-level schema key, deduped.
NAME_MAP: dict[str, str] = {}
_used_names: set[str] = set()
for _key in sorted(SCHEMAS):
    ident = pascal(_key)
    base = ident
    i = 2
    while ident in _used_names:
        ident = f"{base}{i}"
        i += 1
    _used_names.add(ident)
    NAME_MAP[_key] = ident


def ref_target(node: Any) -> str | None:
    if isinstance(node, dict):
        ref = node.get("$ref")
        if isinstance(ref, str) and ref.startswith("#/components/schemas/"):
            return ref.rsplit("/", 1)[-1]
    return None


# ──────────────────────────────────────────────────────────────────────────
# Discovery / synthesis state.
#   WORK:      name -> schema for every item to emit (top-level + synthesized).
#   SYNTH_SIG: canonical-json -> assigned Rust name (structural dedup).
#   TAG_SKIP:  member schema name -> discriminator key whose field the member
#              must NOT serialize (it is written by the internally-tagged enum).
# ──────────────────────────────────────────────────────────────────────────

WORK: dict[str, Any] = {}
SYNTH_SIG: dict[str, str] = {}
TAG_SKIP: dict[str, str] = {}
_QUEUE: list[str] = []


def _register_synth(schema: Any, ctx: str) -> str:
    sig = json.dumps(schema, sort_keys=True)
    if sig in SYNTH_SIG:
        return SYNTH_SIG[sig]
    ident = pascal(ctx)
    base = ident
    i = 2
    while ident in _used_names:
        ident = f"{base}{i}"
        i += 1
    _used_names.add(ident)
    SYNTH_SIG[sig] = ident
    NAME_MAP_SYNTH[ident] = schema
    WORK[ident] = schema
    _QUEUE.append(ident)
    return ident


NAME_MAP_SYNTH: dict[str, Any] = {}


def is_object_variant(v: Any) -> dict[str, Any] | None:
    """Return the object schema (resolving a $ref) if `v` is an object, else None."""
    if not isinstance(v, dict):
        return None
    tgt = ref_target(v)
    if tgt is not None:
        rv = SCHEMAS.get(tgt)
        return rv if isinstance(rv, dict) and (rv.get("type") == "object" or "properties" in rv) else None
    if v.get("type") == "object" or "properties" in v:
        return v
    return None


def const_keys(obj: dict[str, Any]) -> set[str]:
    """Property keys whose schema is a single-value string enum (const discriminator)."""
    out: set[str] = set()
    for k, ks in (obj.get("properties") or {}).items():
        if (
            isinstance(ks, dict)
            and ks.get("type") == "string"
            and isinstance(ks.get("enum"), list)
            and len(ks["enum"]) == 1
        ):
            out.add(k)
    return out


def union_branches(schema: dict[str, Any]) -> list[Any]:
    return schema.get("anyOf") or schema.get("oneOf") or []


def classify_union(schema: dict[str, Any]) -> tuple[str, str | None]:
    """Return (kind, disc_key). kind in {"string","tagged","untagged"}."""
    branches = [b for b in union_branches(schema) if not (isinstance(b, dict) and b.get("type") == "null")]
    if not branches:
        return "untagged", None
    # All string branches -> open string enum.
    if all(isinstance(b, dict) and b.get("type") == "string" for b in branches):
        return "string", None
    # Shared const discriminator across all object branches -> internally tagged.
    shared: set[str] | None = None
    all_objects = True
    for b in branches:
        obj = is_object_variant(b)
        if obj is None:
            all_objects = False
            break
        ck = const_keys(obj)
        shared = ck if shared is None else shared & ck
        if not shared:
            break
    if all_objects and shared:
        # A usable discriminator must have DISTINCT const values across branches.
        def distinct(key: str) -> bool:
            vals = []
            for b in branches:
                obj = is_object_variant(b) or {}
                v = (obj.get("properties", {}).get(key, {}).get("enum") or [None])[0]
                vals.append(v)
            return len(set(vals)) == len(vals) and None not in vals

        usable = [k for k in shared if distinct(k)]
        if usable:
            for pref in ("type", "role", "status", "kind"):
                if pref in usable:
                    return "tagged", pref
            return "tagged", sorted(usable)[0]
    return "untagged", None


# ──────────────────────────────────────────────────────────────────────────
# Discovery: walk a schema, ensure referenced top-level types are emitted and
# synthesize named types for inline objects / unions. Populates WORK + TAG_SKIP.
# ──────────────────────────────────────────────────────────────────────────


def _ensure_toplevel(name: str) -> None:
    if name in SCHEMAS and name not in WORK:
        WORK[name] = SCHEMAS[name]
        _QUEUE.append(name)


def type_expr(node: Any, ctx: str) -> str:
    """Schema node -> Rust type expression, synthesizing named types as needed."""
    if not isinstance(node, dict):
        return "serde_json::Value"

    tgt = ref_target(node)
    if tgt is not None:
        _ensure_toplevel(tgt)
        return NAME_MAP.get(tgt, "serde_json::Value")

    # type as a list, e.g. ["string","null"].
    t = node.get("type")
    if isinstance(t, list):
        non_null = [x for x in t if x != "null"]
        inner_schema = {k: v for k, v in node.items() if k != "type"}
        if len(non_null) == 1:
            inner_schema["type"] = non_null[0]
            inner = type_expr(inner_schema, ctx)
        else:
            inner = "serde_json::Value"
        return f"Option<{inner}>" if "null" in t else inner

    if "anyOf" in node or "oneOf" in node:
        branches = union_branches(node)
        non_null = [b for b in branches if not (isinstance(b, dict) and b.get("type") == "null")]
        has_null = len(non_null) < len(branches)
        if len(non_null) == 1:
            inner = type_expr(non_null[0], ctx)
            return f"Option<{inner}>" if has_null else inner
        # Multi-branch union in field position: synthesize a named enum.
        synth = dict(node)
        if has_null:
            synth = {"anyOf": non_null}
        name = _register_synth(synth, ctx)
        return f"Option<{name}>" if has_null else name

    if t == "string":
        return "String"
    if t == "integer":
        minimum = node.get("minimum")
        return "u64" if isinstance(minimum, (int, float)) and minimum >= 0 else "i64"
    if t == "number":
        return "f64"
    if t == "boolean":
        return "bool"
    if t == "array":
        items = node.get("items")
        if isinstance(items, dict):
            return f"Vec<{type_expr(items, ctx + 'Item')}>"
        return "Vec<serde_json::Value>"
    if t == "object" or "properties" in node:
        props = node.get("properties")
        if props:
            return _register_synth(node, ctx)
        ap = node.get("additionalProperties")
        if isinstance(ap, dict):
            return f"std::collections::BTreeMap<String, {type_expr(ap, ctx + 'Value')}>"
        return "serde_json::Map<String, serde_json::Value>"
    return "serde_json::Value"


def discover(name: str, schema: Any) -> None:
    if not isinstance(schema, dict):
        return
    if "anyOf" in schema or "oneOf" in schema:
        kind, disc = classify_union(schema)
        branches = [b for b in union_branches(schema) if not (isinstance(b, dict) and b.get("type") == "null")]
        if kind == "string":
            return
        if kind == "tagged":
            for b in branches:
                tgt = ref_target(b)
                if tgt is not None:
                    _ensure_toplevel(tgt)
                    TAG_SKIP[tgt] = disc  # member serializes its tag via the enum only.
                    obj = SCHEMAS.get(tgt, {})
                    for fk, fs in (obj.get("properties") or {}).items():
                        if fk != disc:
                            type_expr(fs, NAME_MAP.get(tgt, "X") + pascal(fk))
                else:
                    obj = is_object_variant(b) or {}
                    ctx = pascal(name) + pascal(str((obj.get("properties", {}).get(disc, {}).get("enum") or ["V"])[0]))
                    for fk, fs in (obj.get("properties") or {}).items():
                        if fk != disc:
                            type_expr(fs, ctx + pascal(fk))
            return
        # untagged
        for idx, b in enumerate(branches):
            tgt = ref_target(b)
            if tgt is not None:
                _ensure_toplevel(tgt)
            elif isinstance(b, dict) and b.get("type") == "string":
                continue
            else:
                type_expr(b, pascal(name) + f"Variant{idx}")
        return

    t = schema.get("type")
    if t == "array":
        items = schema.get("items")
        if isinstance(items, dict):
            type_expr(items, NAME_MAP.get(name, pascal(name)) + "Item")
        return
    if t == "object" or "properties" in schema:
        for fk, fs in (schema.get("properties") or {}).items():
            type_expr(fs, NAME_MAP.get(name, pascal(name)) + pascal(fk))
        ap = schema.get("additionalProperties")
        if isinstance(ap, dict):
            type_expr(ap, NAME_MAP.get(name, pascal(name)) + "Value")
        return


# Seed discovery with every top-level schema (maximal coverage) and drain.
for _key in sorted(SCHEMAS):
    _ensure_toplevel(_key)
while _QUEUE:
    _n = _QUEUE.pop()
    discover(_n, WORK[_n])


def schema_of(name: str) -> Any:
    return WORK.get(name)


def rust_name(name: str) -> str:
    if name in NAME_MAP:
        return NAME_MAP[name]
    return name  # synthesized names are already Rust idents.


# ──────────────────────────────────────────────────────────────────────────
# Rendering.
# ──────────────────────────────────────────────────────────────────────────


def _doc_lines(schema: dict[str, Any]) -> list[str]:
    desc = schema.get("description")
    if not desc:
        return []
    return [f"/// {ln}" for ln in str(desc).strip().splitlines()]


def _field_lines(props: dict[str, Any], required: set[str], ctx: str, indent: str,
                 skip: str | None = None, skip_serialize_key: str | None = None,
                 vis: str = "pub ") -> list[str]:
    out: list[str] = []
    for wire in sorted(props):
        if skip is not None and wire == skip:
            continue
        fs = props[wire]
        field = to_snake(wire)
        ty = type_expr(fs, ctx + pascal(wire))
        optional = wire not in required and not ty.startswith("Option<")
        if optional:
            ty = f"Option<{ty}>"
        attrs: list[str] = []
        if field != wire:
            attrs.append(f'rename = "{wire}"')
        if wire == skip_serialize_key:
            attrs.append("default")
            attrs.append("skip_serializing")
        elif wire not in required:
            attrs.append("default")
            attrs.append('skip_serializing_if = "Option::is_none"')
        if attrs:
            out.append(f"{indent}#[serde(" + ", ".join(attrs) + ")]")
        out.append(f"{indent}{vis}{field}: {ty},")
    return out


def render_struct(name: str, schema: dict[str, Any]) -> str:
    rs: list[str] = _doc_lines(schema)
    rs.append("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]")
    ident = rust_name(name)
    rs.append(f"pub struct {ident} {{")
    props = schema.get("properties") or {}
    if not props:
        ap = schema.get("additionalProperties")
        if isinstance(ap, dict):
            rs.append(
                f"    #[serde(flatten)]\n    pub map: std::collections::BTreeMap<String, {type_expr(ap, ident + 'Value')}>,"
            )
        else:
            rs.append('    #[serde(flatten)]')
            rs.append("    pub extra: serde_json::Map<String, serde_json::Value>,")
        rs.append("}")
        return "\n".join(rs)
    required = set(schema.get("required") or [])
    skip_key = TAG_SKIP.get(name)
    rs += _field_lines(props, required, ident, "    ", skip_serialize_key=skip_key)
    rs.append("}")
    return "\n".join(rs)


def render_string_enum_values(name: str, values: list[str], desc_schema: dict[str, Any]) -> str:
    ident = rust_name(name)
    seen: dict[str, str] = {}
    for v in values:
        vi = pascal(v)
        base = vi
        i = 2
        while vi in seen:
            vi = f"{base}{i}"
            i += 1
        seen[vi] = v
    rs: list[str] = _doc_lines(desc_schema)
    rs.append("#[derive(Debug, Clone, PartialEq, Eq, Hash)]")
    rs.append(f"pub enum {ident} {{")
    for vi in seen:
        rs.append(f"    {vi},")
    rs.append("    Unknown(String),")
    rs.append("}")
    rs.append("")
    rs.append(f"impl {ident} {{")
    rs.append("    pub fn as_str(&self) -> &str {")
    rs.append("        match self {")
    for vi, wire in seen.items():
        rs.append(f'            Self::{vi} => "{wire}",')
    rs.append("            Self::Unknown(s) => s.as_str(),")
    rs.append("        }")
    rs.append("    }")
    rs.append("}")
    rs.append("")
    rs.append(f"impl std::fmt::Display for {ident} {{")
    rs.append("    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {")
    rs.append("        f.write_str(self.as_str())")
    rs.append("    }")
    rs.append("}")
    rs.append("")
    rs.append(f"impl From<&str> for {ident} {{")
    rs.append("    fn from(s: &str) -> Self {")
    rs.append("        match s {")
    for vi, wire in seen.items():
        rs.append(f'            "{wire}" => Self::{vi},')
    rs.append("            other => Self::Unknown(other.to_string()),")
    rs.append("        }")
    rs.append("    }")
    rs.append("}")
    rs.append("")
    rs.append(f"impl Serialize for {ident} {{")
    rs.append("    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {")
    rs.append("        serializer.serialize_str(self.as_str())")
    rs.append("    }")
    rs.append("}")
    rs.append("")
    rs.append(f"impl<'de> Deserialize<'de> for {ident} {{")
    rs.append("    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {")
    rs.append("        let s = String::deserialize(deserializer)?;")
    rs.append("        Ok(Self::from(s.as_str()))")
    rs.append("    }")
    rs.append("}")
    return "\n".join(rs)


def render_tagged_enum(name: str, schema: dict[str, Any], disc: str) -> str:
    ident = rust_name(name)
    rs: list[str] = _doc_lines(schema)
    rs.append("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]")
    rs.append(f'#[serde(tag = "{disc}")]')
    rs.append(f"pub enum {ident} {{")
    branches = [b for b in union_branches(schema) if not (isinstance(b, dict) and b.get("type") == "null")]
    seen: set[str] = set()
    for b in branches:
        tgt = ref_target(b)
        obj = SCHEMAS.get(tgt) if tgt is not None else b
        const = (obj.get("properties", {}).get(disc, {}).get("enum") or [None])[0]
        if const is None:
            continue
        vi = pascal(str(const))
        base = vi
        i = 2
        while vi in seen:
            vi = f"{base}{i}"
            i += 1
        seen.add(vi)
        if str(const) != vi:
            rs.append(f'    #[serde(rename = "{const}")]')
        if tgt is not None:
            rs.append(f"    {vi}({rust_name(tgt)}),")
        else:
            other = {k: v for k, v in (obj.get("properties") or {}).items() if k != disc}
            if not other:
                rs.append(f"    {vi},")
            else:
                req = set(obj.get("required") or []) - {disc}
                rs.append(f"    {vi} {{")
                rs += _field_lines(other, req, ident + vi, "        ", vis="")
                rs.append("    },")
    rs.append("}")
    return "\n".join(rs)


def render_untagged_enum(name: str, schema: dict[str, Any]) -> str:
    ident = rust_name(name)
    rs: list[str] = _doc_lines(schema)
    rs.append("/// Untagged union: serde tries each variant in the order below.")
    rs.append("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]")
    rs.append("#[serde(untagged)]")
    rs.append(f"pub enum {ident} {{")
    branches = [b for b in union_branches(schema) if not (isinstance(b, dict) and b.get("type") == "null")]
    seen: set[str] = set()
    for idx, b in enumerate(branches):
        tgt = ref_target(b)
        if tgt is not None:
            vi = pascal(tgt)
            body = rust_name(tgt)
        else:
            body = type_expr(b, ident + f"Variant{idx}")
            title = b.get("title") if isinstance(b, dict) else None
            vi = pascal(str(title)) if title else f"Variant{idx}"
        base = vi
        i = 2
        while vi in seen:
            vi = f"{base}{i}"
            i += 1
        seen.add(vi)
        rs.append(f"    {vi}({body}),")
    rs.append("}")
    return "\n".join(rs)


def render(name: str, schema: Any) -> str:
    if not isinstance(schema, dict):
        return f"pub type {rust_name(name)} = serde_json::Value;"

    if "anyOf" in schema or "oneOf" in schema:
        kind, disc = classify_union(schema)
        if kind == "string":
            values: list[str] = []
            for b in union_branches(schema):
                if isinstance(b, dict):
                    for v in b.get("enum") or []:
                        if v not in values:
                            values.append(v)
            return render_string_enum_values(name, values, schema)
        if kind == "tagged":
            return render_tagged_enum(name, schema, disc)
        return render_untagged_enum(name, schema)

    t = schema.get("type")
    if t == "string" and "enum" in schema:
        return render_string_enum_values(name, list(schema["enum"]), schema)
    if t == "string":
        rs = _doc_lines(schema)
        rs.append("#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]")
        rs.append("#[serde(transparent)]")
        rs.append(f"pub struct {rust_name(name)}(pub String);")
        return "\n".join(rs)
    if t == "array":
        items = schema.get("items")
        inner = type_expr(items, rust_name(name) + "Item") if isinstance(items, dict) else "serde_json::Value"
        doc = _doc_lines(schema)
        return "\n".join(doc + [f"pub type {rust_name(name)} = Vec<{inner}>;"])
    if t in ("integer",):
        minimum = schema.get("minimum")
        inner = "u64" if isinstance(minimum, (int, float)) and minimum >= 0 else "i64"
        return f"pub type {rust_name(name)} = {inner};"
    if t == "number":
        return f"pub type {rust_name(name)} = f64;"
    if t == "boolean":
        return f"pub type {rust_name(name)} = bool;"
    if t == "object" or "properties" in schema:
        return render_struct(name, schema)
    return f"pub type {rust_name(name)} = serde_json::Value;"


# ──────────────────────────────────────────────────────────────────────────
# Emit types.rs
# ──────────────────────────────────────────────────────────────────────────


def emit_types() -> str:
    out: list[str] = []
    out.append("// AUTO-GENERATED by scripts/codegen_opencode.py — DO NOT EDIT BY HAND.")
    out.append("// Source: opencode-codes/tests/schemas/opencode_openapi.json (opencode 1.18.5).")
    out.append("// Run `python3 scripts/codegen_opencode.py` to regenerate.")
    out.append("//")
    out.append("// Every schema in components.schemas is emitted, plus synthesized named types")
    out.append("// for the six hand-wrapped endpoints' inline request/response bodies and for")
    out.append("// inline object / union field shapes. Discriminated unions become internally-")
    out.append("// tagged serde enums; all-string unions become open enums with Unknown(String);")
    out.append("// remaining unions are #[serde(untagged)] with branch order preserved. Inline")
    out.append("// string-enum fields are represented as String.")
    out.append("")
    out.append(
        "#![allow(clippy::large_enum_variant, clippy::enum_variant_names, "
        "clippy::doc_markdown, clippy::doc_lazy_continuation)]"
    )
    out.append("")
    out.append("use serde::{Deserialize, Deserializer, Serialize, Serializer};")
    out.append("")
    for name in sorted(WORK):
        try:
            out.append(render(name, WORK[name]))
        except Exception as e:  # noqa: BLE001
            out.append(f"// codegen fallback for {name}: {e}")
            out.append(f"pub type {rust_name(name)} = serde_json::Value;")
        out.append("")
    return "\n".join(out)


# ──────────────────────────────────────────────────────────────────────────
# Emit samples.rs — minimal valid JSON per endpoint primary type.
# ──────────────────────────────────────────────────────────────────────────


def sample_for(schema: Any, depth: int = 0) -> Any:
    if not isinstance(schema, dict) or depth > 12:
        return None
    tgt = ref_target(schema)
    if tgt is not None:
        return sample_for(SCHEMAS.get(tgt, {}), depth + 1)
    for key in ("anyOf", "oneOf"):
        if key in schema:
            for b in schema[key]:
                if isinstance(b, dict) and b.get("type") == "null":
                    continue
                return sample_for(b, depth + 1)
            return None
    if "enum" in schema:
        return schema["enum"][0]
    t = schema.get("type")
    if isinstance(t, list):
        non_null = [x for x in t if x != "null"]
        if non_null:
            return sample_for({**{k: v for k, v in schema.items() if k != "type"}, "type": non_null[0]}, depth + 1)
        return None
    if t == "string":
        return "x"
    if t == "integer":
        return 0
    if t == "number":
        return 0.0
    if t == "boolean":
        return False
    if t == "array":
        return []
    if t == "object" or "properties" in schema:
        props = schema.get("properties") or {}
        required = set(schema.get("required") or [])
        return {k: sample_for(props[k], depth + 1) for k in props if k in required}
    return None


ENDPOINT_SAMPLES = [
    ("SessionCreateParams", "SessionCreateParams"),
    ("PromptAsyncParams", "PromptAsyncParams"),
    ("PermissionReplyParams", "PermissionReplyParams"),
    ("Session", "Session"),
    ("MessageWithParts", "MessageWithParts"),
    ("Event", "Event"),
]


def emit_samples() -> str:
    out: list[str] = []
    out.append("// AUTO-GENERATED by scripts/codegen_opencode.py — DO NOT EDIT BY HAND.")
    out.append("")
    out.append("use serde_json::{json, Value};")
    out.append("")
    out.append("/// Minimal valid JSON samples for the primary types of the six hand-wrapped")
    out.append("/// endpoints, keyed by generated Rust type name. For round-trip tests.")
    out.append("pub fn endpoint_samples() -> Vec<(&'static str, Value)> {")
    out.append("    vec![")
    for label, sname in ENDPOINT_SAMPLES:
        s = sample_for(SCHEMAS.get(sname, {}))
        out.append(f"        ({json.dumps(rust_name(label))}, json!({json.dumps(s)})),")
    out.append("    ]")
    out.append("}")
    return "\n".join(out)


# ──────────────────────────────────────────────────────────────────────────
# Write files.
# ──────────────────────────────────────────────────────────────────────────

OUT_DIR.mkdir(exist_ok=True)
(OUT_DIR / "mod.rs").write_text(
    "//! Generated serde models of the opencode OpenAPI 3.1 wire contract.\n"
    "//!\n"
    "//! AUTO-GENERATED by `scripts/codegen_opencode.py` from\n"
    "//! `tests/schemas/opencode_openapi.json` (opencode 1.18.5). Do not edit by hand.\n"
    "\n"
    "pub mod samples;\n"
    "pub mod types;\n"
)
(OUT_DIR / "types.rs").write_text(emit_types())
(OUT_DIR / "samples.rs").write_text(emit_samples())

print(f"components.schemas:   {len(DOC['components']['schemas'])}")
print(f"synthetic endpoints:  {len(SYNTHETIC_ENDPOINT_TYPES)}")
print(f"reachable (endpoints):{len(REACHABLE)}")
print(f"emitted items:        {len(WORK)} (top-level + synthesized {len(SYNTH_SIG)})")
print(f"tag-skip members:     {len(TAG_SKIP)}")
print(f"wrote {OUT_DIR}/mod.rs, types.rs, samples.rs")
