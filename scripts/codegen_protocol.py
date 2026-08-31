#!/usr/bin/env python3
"""
Generate Rust protocol types + samples for codex-codes from the upstream JSON Schema.

The schema bundles in codex-codes/tests/schemas/ are the source of truth for
every wire type. This script walks every notification, client-request, and
server-request enumerated in the envelopes plus every transitively-referenced
definition and writes:

  - codex-codes/src/protocol_generated/types.rs   (Rust struct/enum per definition)
  - codex-codes/src/protocol_generated/samples.rs (one validating JSON sample per method)
  - codex-codes/src/protocol_generated/mod.rs     (module index)
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
SCHEMA_DIR = ROOT / "codex-codes" / "tests" / "schemas"
V2 = json.loads((SCHEMA_DIR / "codex_app_server_protocol.v2.schemas.json").read_text())
FULL = json.loads((SCHEMA_DIR / "codex_app_server_protocol.schemas.json").read_text())

# v2 has all the inlined definitions for notification + client-request params.
# The full bundle is the only place ServerRequest is enumerated. Merge so we
# have everything reachable.
DEFS: dict[str, Any] = dict(V2["definitions"])
for name, schema in FULL["definitions"].items():
    DEFS.setdefault(name, schema)

# ──────────────────────────────────────────────────────────────────────────
# Envelope extraction
# ──────────────────────────────────────────────────────────────────────────


def envelope_methods(bundle_defs: dict[str, Any], envelope_name: str) -> list[tuple[str, str | None]]:
    """Yield (method, params_def_name|None) for every variant in {envelope}.oneOf."""
    env = bundle_defs.get(envelope_name, {})
    out = []
    for variant in env.get("oneOf", []):
        props = variant.get("properties", {})
        method_enum = props.get("method", {}).get("enum") or []
        params_ref = props.get("params", {}).get("$ref", "")
        params_def = params_ref.rsplit("/", 1)[-1] if params_ref else None
        if method_enum:
            out.append((method_enum[0], params_def))
    return out


SERVER_NOTIFS = envelope_methods(V2["definitions"], "ServerNotification")
CLIENT_REQS = envelope_methods(V2["definitions"], "ClientRequest")
SERVER_REQS = envelope_methods(FULL["definitions"], "ServerRequest")


# ──────────────────────────────────────────────────────────────────────────
# Reachable-definition closure
# ──────────────────────────────────────────────────────────────────────────


def collect_refs(node: Any, into: set[str]) -> None:
    if isinstance(node, dict):
        ref = node.get("$ref")
        if isinstance(ref, str) and ref.startswith("#/definitions/"):
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
        if n in out or n not in DEFS:
            continue
        out.add(n)
        nxt: set[str] = set()
        collect_refs(DEFS[n], nxt)
        frontier |= nxt - out
    return out


# Seed: every notification params, every request params, every ServerRequest params.
SEED: set[str] = set()
for method, params_def in SERVER_NOTIFS + CLIENT_REQS + SERVER_REQS:
    if params_def:
        SEED.add(params_def)

# Also pull in the response definitions referenced by ClientRequest variants:
for variant in V2["definitions"]["ClientRequest"]["oneOf"]:
    # ClientRequest variants are objects with `method` enum + `params` $ref.
    # Their responses live alongside them; we'll discover the *Response types
    # by walking the schema for any names ending in Response.
    pass

# Add any *Response definitions known to live in the bundle.
SEED |= {name for name in DEFS if name.endswith("Response")}

REACHABLE = closure(SEED)

print(f"server notifications: {len(SERVER_NOTIFS)}")
print(f"client requests:      {len(CLIENT_REQS)}")
print(f"server requests:      {len(SERVER_REQS)}")
print(f"reachable defs:       {len(REACHABLE)} of {len(DEFS)}")


# ──────────────────────────────────────────────────────────────────────────
# Rust-name mapping & camelCase helpers
# ──────────────────────────────────────────────────────────────────────────


KEYWORDS = {
    "type", "ref", "match", "self", "where", "for", "in", "if", "else", "fn",
    "let", "mut", "const", "static", "pub", "use", "mod", "struct", "enum",
    "impl", "trait", "as", "async", "await", "break", "continue", "loop",
    "move", "return", "true", "false", "unsafe", "yield",
}


def to_snake(name: str) -> str:
    s = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", name)
    s = re.sub(r"([a-z\d])([A-Z])", r"\1_\2", s)
    s = s.replace("-", "_").replace(".", "_").replace("/", "_")
    # Strip any non-ident chars (e.g. `$schema` → `schema`).
    s = re.sub(r"[^a-zA-Z0-9_]", "", s)
    s = s.lower()
    if not s:
        s = "field"
    if s[0].isdigit():
        s = "_" + s
    if s in KEYWORDS:
        s = s + "_"
    return s


def rust_name(schema_name: str) -> str:
    """Best-effort safe Rust type name from a schema definition name."""
    # Strip namespaces / generic-style brackets.
    n = schema_name
    n = n.replace("Option<", "Opt").replace(">", "").replace("<", "_")
    n = re.sub(r"[^A-Za-z0-9_]", "_", n)
    if n and n[0].isdigit():
        n = "_" + n
    return n


# ──────────────────────────────────────────────────────────────────────────
# Rust-type expression for a schema node
# ──────────────────────────────────────────────────────────────────────────


def is_nullable_type(t: Any) -> bool:
    return isinstance(t, list) and "null" in t


def strip_null(t: list[str]) -> str:
    others = [x for x in t if x != "null"]
    return others[0] if len(others) == 1 else "string"


def schema_to_rust(node: Any) -> str:
    """Best-effort: schema node -> Rust type. Returns `Value` for things we can't model."""
    if not isinstance(node, dict):
        return "Value"

    # $ref → use the referenced Rust type.
    ref = node.get("$ref")
    if isinstance(ref, str) and ref.startswith("#/definitions/"):
        return rust_name(ref.rsplit("/", 1)[-1])

    # anyOf with null → Option<T>
    if "anyOf" in node:
        variants = node["anyOf"]
        non_null = [v for v in variants if v.get("type") != "null"]
        has_null = len(non_null) < len(variants)
        if len(non_null) == 1:
            inner = schema_to_rust(non_null[0])
            return f"Option<{inner}>" if has_null else inner
        # Two-or-more non-null branches with no tag — punt to Value.
        return "Option<Value>" if has_null else "Value"

    # oneOf with no discriminator we can extract — punt.
    if "oneOf" in node:
        return "Value"

    t = node.get("type")
    if isinstance(t, list):
        if "null" in t:
            inner = strip_null(t)
            return f"Option<{schema_to_rust({'type': inner, **{k: v for k, v in node.items() if k != 'type'}})}>"
        # Multiple non-null types — Value.
        return "Value"

    if t == "string":
        if "enum" in node:
            # Could be a unit enum elsewhere; here just emit String for inline use.
            return "String"
        return "String"
    if t == "integer":
        return "i64"
    if t == "number":
        return "f64"
    if t == "boolean":
        return "bool"
    if t == "array":
        items = node.get("items", {})
        if isinstance(items, list):
            return "Vec<Value>"
        return f"Vec<{schema_to_rust(items)}>"
    if t == "object":
        # Inline object without a $ref — has additionalProperties or properties.
        ap = node.get("additionalProperties")
        if isinstance(ap, dict):
            return f"std::collections::BTreeMap<String, {schema_to_rust(ap)}>"
        return "Value"
    if t == "null":
        return "Option<Value>"
    # Fallback.
    return "Value"


# ──────────────────────────────────────────────────────────────────────────
# Rust type emission per definition
# ──────────────────────────────────────────────────────────────────────────


def emit_struct(name: str, schema: dict[str, Any]) -> str:
    props = schema.get("properties") or {}
    required = set(schema.get("required") or [])
    body: list[str] = []
    # Derive `Default` when every field is serde-defaultable — i.e. the
    # struct deserializes from `{}` — so all-optional params can be built
    # with `Params::default()` instead of spelling out every `None` (#203).
    all_default = True
    body.append(f"pub struct {rust_name(name)} {{")
    if not props:
        # Empty struct - allow extra fields via a flatten map.
        body.append("    #[serde(flatten, default, skip_serializing_if = \"serde_json::Map::is_empty\")]")
        body.append("    pub extra: serde_json::Map<String, Value>,")
    for field_name in sorted(props):
        f_schema = props[field_name]
        rs_field = to_snake(field_name)
        rs_type = schema_to_rust(f_schema)
        is_opt = rs_type.startswith("Option<") or field_name not in required
        if is_opt and not rs_type.startswith("Option<"):
            rs_type = f"Option<{rs_type}>"
        attrs = []
        if rs_field != field_name:
            attrs.append(f'rename = "{field_name}"')
        if is_opt:
            attrs.append("default")
            attrs.append('skip_serializing_if = "Option::is_none"')
        elif _is_default_able_type(rs_type):
            # Codex marks some fields required in the schema but omits
            # them on the wire (e.g. `installationId` on
            # RemoteControlStatusChangedNotification when remote control
            # isn't active). For required fields whose Rust type already
            # has a `Default` impl, fill in the default rather than
            # failing typed deserialization.
            attrs.append("default")
        else:
            all_default = False
        body.append("    #[serde(" + ", ".join(attrs) + ")]")
        body.append(f"    pub {rs_field}: {rs_type},")
    if props and schema.get("additionalProperties") is True:
        body.append("    #[serde(flatten, default, skip_serializing_if = \"serde_json::Map::is_empty\")]")
        body.append("    pub additional: serde_json::Map<String, Value>,")
    body.append("}")
    rs = []
    # PartialEq (but not Eq) so structs compose into enums that need
    # equality; serde_json::Value implements PartialEq but not Eq, so Eq
    # would propagate-fail on any field carrying raw JSON.
    derives = "Debug, Clone, PartialEq, Serialize, Deserialize"
    if all_default:
        derives += ", Default"
    rs.append(f"#[derive({derives})]")
    # Config RPC values mirror config.toml and therefore use snake_case,
    # unlike the rest of app-server v2's camelCase payloads.
    rename_all = "snake_case" if name in {
        "AnalyticsConfig",
        "Config",
        "SandboxWorkspaceWrite",
    } else "camelCase"
    rs.append(f'#[serde(rename_all = "{rename_all}")]')
    rs.extend(body)
    return "\n".join(rs)


# Types we know have a `Default` impl in scope — primitives and standard
# containers. Generated types do not (since we don't derive Default on
# struct/enum output to avoid cascading constraints on each variant). When a
# schema-required field uses one of these types we can safely `#[serde(default)]`
# it; for other required types we leave it strict and let the typed parse fail
# if codex omits the field, which signals a real schema/wire mismatch.
def _is_default_able_type(rs_type: str) -> bool:
    if rs_type in {"String", "i64", "i32", "u64", "u32", "f64", "bool", "Value"}:
        return True
    return rs_type.startswith(("Vec<", "Option<", "std::collections::BTreeMap<"))


def emit_string_enum(name: str, schema: dict[str, Any]) -> str:
    """Bare-string enum: {"enum": ["a","b","c"], "type": "string"}."""
    variants = schema["enum"]
    rs = []
    rs.append("#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]")
    rs.append("pub enum " + rust_name(name) + " {")
    for v in variants:
        # Variant must be a valid Rust ident; preserve original via rename.
        v_str = str(v)
        variant_ident = re.sub(r"[^A-Za-z0-9_]", "_", v_str)
        if variant_ident and variant_ident[0].isdigit():
            variant_ident = "_" + variant_ident
        variant_ident = variant_ident[:1].upper() + variant_ident[1:] if variant_ident else "Unknown"
        rs.append(f'    #[serde(rename = "{v_str}")]')
        rs.append(f"    {variant_ident},")
    rs.append("}")
    return "\n".join(rs)


def emit_tagged_enum(name: str, schema: dict[str, Any]) -> str:
    """oneOf with each variant having a `type: {enum: ["..."]}` discriminator."""
    rs = []
    rs.append("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]")
    rs.append('#[serde(tag = "type", rename_all = "camelCase")]')
    rs.append(f"pub enum {rust_name(name)} {{")
    seen_idents: set[str] = set()
    for v in schema["oneOf"]:
        v_props = v.get("properties", {})
        tag_enum = v_props.get("type", {}).get("enum")
        if not tag_enum:
            # Variant without a string-tag discriminator — fall through.
            continue
        tag = tag_enum[0]
        variant_ident = re.sub(r"[^A-Za-z0-9_]", "_", tag)
        if variant_ident and variant_ident[0].isdigit():
            variant_ident = "_" + variant_ident
        variant_ident = variant_ident[:1].upper() + variant_ident[1:]
        # Avoid duplicate variant names (rare but possible).
        base = variant_ident
        i = 2
        while variant_ident in seen_idents:
            variant_ident = f"{base}{i}"
            i += 1
        seen_idents.add(variant_ident)

        # Collect non-`type` fields as struct-like variant body.
        other_props = {k: vp for k, vp in v_props.items() if k != "type"}
        required = set(v.get("required", []))
        if not other_props:
            if tag != to_snake(variant_ident):
                rs.append(f'    #[serde(rename = "{tag}")]')
            rs.append(f"    {variant_ident},")
        else:
            if tag != to_snake(variant_ident):
                rs.append(f'    #[serde(rename = "{tag}")]')
            rs.append(f"    {variant_ident} {{")
            for fn in sorted(other_props):
                fs = other_props[fn]
                rs_field = to_snake(fn)
                rs_type = schema_to_rust(fs)
                is_opt = rs_type.startswith("Option<") or fn not in required
                if is_opt and not rs_type.startswith("Option<"):
                    rs_type = f"Option<{rs_type}>"
                attrs = []
                if rs_field != fn:
                    attrs.append(f'rename = "{fn}"')
                if is_opt:
                    attrs.append("default")
                    attrs.append('skip_serializing_if = "Option::is_none"')
                if attrs:
                    rs.append("        #[serde(" + ", ".join(attrs) + ")]")
                rs.append(f"        {rs_field}: {rs_type},")
            rs.append("    },")
    rs.append("}")
    return "\n".join(rs)


def _is_string_enum_variant(v: Any) -> bool:
    """`{enum: [...], type: "string"}` — one or more string values, any count."""
    return (
        isinstance(v, dict)
        and v.get("type") == "string"
        and isinstance(v.get("enum"), list)
        and len(v["enum"]) >= 1
        and all(isinstance(x, str) for x in v["enum"])
    )


def _is_single_key_object_variant(v: Any) -> tuple[str, dict[str, Any]] | None:
    """Return (key, value_schema) if `v` is `{properties: {<one_key>: <schema>}, required: [<one_key>], type: object}`."""
    if not isinstance(v, dict):
        return None
    if v.get("type") != "object":
        return None
    props = v.get("properties") or {}
    required = v.get("required") or []
    if len(props) != 1 or len(required) != 1 or list(props.keys()) != required:
        return None
    key = required[0]
    return key, props[key]


def _common_object_discriminator(variants: list[Any]) -> str | None:
    """If every object variant has a single-element string-enum on the same key, return that key."""
    keys: set[str] = set()
    for v in variants:
        if not isinstance(v, dict) or v.get("type") != "object":
            return None
        props = v.get("properties") or {}
        local: set[str] = set()
        for k, ks in props.items():
            if (
                isinstance(ks, dict)
                and ks.get("type") == "string"
                and isinstance(ks.get("enum"), list)
                and len(ks["enum"]) == 1
            ):
                local.add(k)
        if not local:
            return None
        keys = local if not keys else keys & local
        if not keys:
            return None
    return next(iter(keys)) if len(keys) == 1 else None


def emit_string_newtype(name: str, schema: dict[str, Any]) -> str:
    """`{"type": "string"}` with no enum — transparent newtype over `String`."""
    desc = schema.get("description")
    rs = []
    if desc:
        for line in str(desc).strip().splitlines():
            rs.append(f"/// {line}")
    rs.append("#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]")
    rs.append("#[serde(transparent)]")
    rs.append(f"pub struct {rust_name(name)}(pub String);")
    return "\n".join(rs)


def _variant_ident(tag: str) -> str:
    """Turn a string-enum tag into a valid PascalCase Rust ident."""
    cleaned = re.sub(r"[^A-Za-z0-9_]+", "_", tag)
    if cleaned and cleaned[0].isdigit():
        cleaned = "_" + cleaned
    # PascalCase: split on _ and capitalize each piece.
    parts = [p for p in cleaned.split("_") if p]
    if not parts:
        return "Unknown"
    ident = "".join(p[:1].upper() + p[1:] for p in parts)
    if ident in KEYWORDS:
        ident = ident + "_"
    return ident


def _push_string_variant(rs: list[str], tag: str, seen: set[str]) -> None:
    """Append one Rust unit variant for a string-enum tag, deduping if needed."""
    ident = _variant_ident(tag)
    base = ident
    i = 2
    while ident in seen:
        ident = f"{base}{i}"
        i += 1
    seen.add(ident)
    if tag != ident:
        rs.append(f'    #[serde(rename = "{tag}")]')
    rs.append(f"    {ident},")


def emit_oneof_string_enum(name: str, schema: dict[str, Any]) -> str:
    """`oneOf` where every variant is a string-enum (single- or multi-valued) — Rust unit enum.

    Multi-value variants are expanded into one Rust variant per enum value,
    so `oneOf: [{enum:["auto","concise"]}, {enum:["none"]}]` becomes
    `enum Foo { Auto, Concise, None_ }`.
    """
    rs = []
    rs.append("#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]")
    rs.append(f"pub enum {rust_name(name)} {{")
    seen: set[str] = set()
    for v in schema["oneOf"]:
        for tag in v["enum"]:
            _push_string_variant(rs, str(tag), seen)
    rs.append("}")
    return "\n".join(rs)


def emit_oneof_mixed(name: str, schema: dict[str, Any]) -> str:
    """`oneOf` mixing string-enum variants with single-key object variants — externally-tagged Rust enum.

    Serde's default external tagging emits unit variants as bare strings and
    single-key-struct variants as `{"variantName": {...}}` — exactly the wire
    shape this pattern is meant to round-trip.
    """
    rs = []
    rs.append("#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]")
    rs.append(f"pub enum {rust_name(name)} {{")
    seen: set[str] = set()
    for v in schema["oneOf"]:
        if _is_string_enum_variant(v):
            for tag in v["enum"]:
                _push_string_variant(rs, str(tag), seen)
            continue
        single = _is_single_key_object_variant(v)
        if single is not None:
            key, inner = single
            ident = _variant_ident(key)
            base = ident
            i = 2
            while ident in seen:
                ident = f"{base}{i}"
                i += 1
            seen.add(ident)
            if key != ident:
                rs.append(f'    #[serde(rename = "{key}")]')
            # Inline the single inner field — preserves its sub-fields as a struct-variant body.
            inner_props = inner.get("properties") or {}
            required = set(inner.get("required") or [])
            if inner.get("type") == "object" and inner_props:
                rs.append(f"    {ident} {{")
                for fn in sorted(inner_props):
                    fs = inner_props[fn]
                    rs_field = to_snake(fn)
                    rs_type = schema_to_rust(fs)
                    is_opt = rs_type.startswith("Option<") or fn not in required
                    if is_opt and not rs_type.startswith("Option<"):
                        rs_type = f"Option<{rs_type}>"
                    attrs = []
                    if rs_field != fn:
                        attrs.append(f'rename = "{fn}"')
                    if is_opt:
                        attrs.append("default")
                        attrs.append('skip_serializing_if = "Option::is_none"')
                    if attrs:
                        rs.append("        #[serde(" + ", ".join(attrs) + ")]")
                    rs.append(f"        {rs_field}: {rs_type},")
                rs.append("    },")
            else:
                # Inner isn't a struct shape — tuple variant with the inner type.
                rs.append(f"    {ident}({schema_to_rust(inner)}),")
            continue
        # Unknown variant shape — keep going but record raw.
        rs.append(f"    // codegen: unhandled variant shape {list(v.keys()) if isinstance(v, dict) else v}")
    rs.append("}")
    return "\n".join(rs)


def emit_anyof_untagged(name: str, schema: dict[str, Any]) -> str:
    """`anyOf` of $refs or inline shapes — untagged Rust enum with one variant per branch."""
    variants = schema["anyOf"]
    rs = []
    rs.append("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]")
    rs.append("#[serde(untagged)]")
    rs.append(f"pub enum {rust_name(name)} {{")
    seen: set[str] = set()
    for idx, v in enumerate(variants):
        if not isinstance(v, dict):
            continue
        ident = None
        body_type = None
        ref = v.get("$ref")
        if isinstance(ref, str) and ref.startswith("#/definitions/"):
            ref_name = ref.rsplit("/", 1)[-1]
            ident = _variant_ident(ref_name)
            body_type = rust_name(ref_name)
        else:
            body_type = schema_to_rust(v)
            # Synthesize an ident from a title or fall back to indexed name.
            title = v.get("title")
            if title:
                ident = _variant_ident(str(title))
            else:
                ident = f"Variant{idx}"
        base = ident
        i = 2
        while ident in seen:
            ident = f"{base}{i}"
            i += 1
        seen.add(ident)
        rs.append(f"    {ident}({body_type}),")
    rs.append("}")
    return "\n".join(rs)


def emit_tagged_enum_on_key(name: str, schema: dict[str, Any], tag_key: str) -> str:
    """`oneOf` of object variants discriminated on `tag_key` (e.g. `kind`)."""
    rs = []
    rs.append("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]")
    rs.append(f'#[serde(tag = "{tag_key}", rename_all = "camelCase")]')
    rs.append(f"pub enum {rust_name(name)} {{")
    seen: set[str] = set()
    for v in schema["oneOf"]:
        v_props = v.get("properties", {})
        tag_enum = v_props.get(tag_key, {}).get("enum")
        if not tag_enum:
            continue
        tag = tag_enum[0]
        ident = _variant_ident(str(tag))
        base = ident
        i = 2
        while ident in seen:
            ident = f"{base}{i}"
            i += 1
        seen.add(ident)
        other_props = {k: vp for k, vp in v_props.items() if k != tag_key}
        required = set(v.get("required") or []) - {tag_key}
        if str(tag) != to_snake(ident):
            rs.append(f'    #[serde(rename = "{tag}")]')
        if not other_props:
            rs.append(f"    {ident},")
        else:
            rs.append(f"    {ident} {{")
            for fn in sorted(other_props):
                fs = other_props[fn]
                rs_field = to_snake(fn)
                rs_type = schema_to_rust(fs)
                is_opt = rs_type.startswith("Option<") or fn not in required
                if is_opt and not rs_type.startswith("Option<"):
                    rs_type = f"Option<{rs_type}>"
                attrs = []
                if rs_field != fn:
                    attrs.append(f'rename = "{fn}"')
                if is_opt:
                    attrs.append("default")
                    attrs.append('skip_serializing_if = "Option::is_none"')
                if attrs:
                    rs.append("        #[serde(" + ", ".join(attrs) + ")]")
                rs.append(f"        {rs_field}: {rs_type},")
            rs.append("    },")
    rs.append("}")
    return "\n".join(rs)


def emit_type(name: str, schema: dict[str, Any]) -> str:
    """Pick the right shape for a definition and emit Rust."""
    # 1. Pure string enum: `{enum: [...], type: "string"}`.
    if "enum" in schema and schema.get("type") == "string":
        return emit_string_enum(name, schema)

    # 2. Bare string newtype: `{type: "string"}` with no enum.
    if schema.get("type") == "string" and "enum" not in schema:
        return emit_string_newtype(name, schema)

    # 3. oneOf — try string-only, mixed, then tagged-on-discriminator forms.
    if "oneOf" in schema:
        variants = schema["oneOf"]
        if variants and all(_is_string_enum_variant(v) for v in variants):
            return emit_oneof_string_enum(name, schema)
        # Mixed: every variant is either a bare string-enum or a single-key
        # object wrapper. Externally-tagged Rust enum round-trips this.
        if variants and all(
            _is_string_enum_variant(v) or _is_single_key_object_variant(v) is not None
            for v in variants
        ):
            return emit_oneof_mixed(name, schema)
        # All variants have a `type`-keyed string-enum discriminator.
        if all(
            isinstance(v, dict) and v.get("properties", {}).get("type", {}).get("enum")
            for v in variants
        ):
            return emit_tagged_enum(name, schema)
        # All variants have a single shared discriminator key (often `kind`).
        disc = _common_object_discriminator(variants)
        if disc:
            return emit_tagged_enum_on_key(name, schema, disc)

    # 4. anyOf — untagged Rust enum per branch.
    if "anyOf" in schema:
        return emit_anyof_untagged(name, schema)

    # 5. Object with properties.
    if schema.get("type") == "object" or "properties" in schema:
        return emit_struct(name, schema)

    # 6. Last-resort opaque newtype — should never trigger now; leaving the
    # `// codegen unhandled` marker so a regression is grepable.
    return f"""// codegen unhandled shape for {name}: {sorted(schema.keys())}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct {rust_name(name)}(pub Value);"""


# ──────────────────────────────────────────────────────────────────────────
# Sample generation: walk a schema and produce a minimal valid JSON Value
# ──────────────────────────────────────────────────────────────────────────


def sample_for(schema: Any, depth: int = 0) -> Any:
    if not isinstance(schema, dict) or depth > 10:
        return None

    ref = schema.get("$ref")
    if isinstance(ref, str) and ref.startswith("#/definitions/"):
        target = DEFS.get(ref.rsplit("/", 1)[-1])
        if target is None:
            return {}
        return sample_for(target, depth + 1)

    if "anyOf" in schema:
        for v in schema["anyOf"]:
            if v.get("type") == "null":
                continue
            return sample_for(v, depth + 1)
        return None

    if "oneOf" in schema:
        return sample_for(schema["oneOf"][0], depth + 1)

    if "enum" in schema:
        return schema["enum"][0]

    t = schema.get("type")
    if isinstance(t, list):
        if "null" in t and len(t) == 2:
            other = [x for x in t if x != "null"][0]
            return sample_for({"type": other, **{k: v for k, v in schema.items() if k != "type"}}, depth + 1)
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
    if t == "object":
        props = schema.get("properties") or {}
        required = set(schema.get("required") or [])
        out = {}
        for fn, fs in props.items():
            if fn in required:
                out[fn] = sample_for(fs, depth + 1)
        return out
    return None


# ──────────────────────────────────────────────────────────────────────────
# Emit generated.rs
# ──────────────────────────────────────────────────────────────────────────




def emit_generated_module() -> str:
    """Produce the contents of generated.rs."""
    out: list[str] = []
    out.append("// AUTO-GENERATED by scripts/codegen_protocol.py — DO NOT EDIT BY HAND.")
    out.append("// Run `python3 scripts/codegen_protocol.py` to regenerate.")
    out.append("//")
    out.append("// Every wire type reachable from a method in the upstream codex")
    out.append("// app-server schema bundle is emitted here as a Rust struct or enum.")
    out.append("// Cross-references resolve to other types in this module.")
    out.append("")
    out.append("#![allow(unused_imports, non_camel_case_types, clippy::large_enum_variant, clippy::enum_variant_names, clippy::empty_docs)]")
    out.append("")
    out.append("use serde::{Deserialize, Serialize};")
    out.append("use serde_json::Value;")
    out.append("")
    # Emit definitions sorted by name (deterministic).
    for name in sorted(REACHABLE):
        schema = DEFS.get(name)
        if not schema:
            continue
        try:
            rust = emit_type(name, schema)
        except Exception as e:  # noqa: BLE001
            rust = (
                f"// codegen skipped {name}: {e}\n"
                f"#[derive(Debug, Clone, Serialize, Deserialize, Default)]\n"
                f"#[serde(transparent)]\n"
                f"pub struct {rust_name(name)}(pub Value);"
            )
        out.append(rust)
        out.append("")
    return "\n".join(out)


def emit_samples_module() -> str:
    """Map each method → a JSON sample that should validate against its params schema."""
    out: list[str] = []
    out.append("// AUTO-GENERATED by scripts/codegen_protocol.py — DO NOT EDIT BY HAND.")
    out.append("")
    out.append("use serde_json::{json, Value};")
    out.append("")
    out.append("/// Notification samples keyed by JSON-RPC method.")
    out.append("pub fn server_notification_samples() -> Vec<(&'static str, Value)> {")
    out.append("    vec![")
    for method, params_def in sorted(SERVER_NOTIFS):
        if params_def and params_def in DEFS:
            s = sample_for(DEFS[params_def])
        else:
            s = {}
        out.append(f"        ({json.dumps(method)}, json!({json.dumps(s)})),")
    out.append("    ]")
    out.append("}")
    out.append("")
    out.append("/// Client-request samples keyed by JSON-RPC method.")
    out.append("pub fn client_request_samples() -> Vec<(&'static str, Value)> {")
    out.append("    vec![")
    for method, params_def in sorted(CLIENT_REQS):
        if params_def and params_def in DEFS:
            s = sample_for(DEFS[params_def])
        else:
            s = {}
        out.append(f"        ({json.dumps(method)}, json!({json.dumps(s)})),")
    out.append("    ]")
    out.append("}")
    out.append("")
    out.append("/// Server-request (approval flow) samples keyed by JSON-RPC method.")
    out.append("pub fn server_request_samples() -> Vec<(&'static str, Value)> {")
    out.append("    vec![")
    for method, params_def in sorted(SERVER_REQS):
        if params_def and params_def in DEFS:
            s = sample_for(DEFS[params_def])
        else:
            s = {}
        out.append(f"        ({json.dumps(method)}, json!({json.dumps(s)})),")
    out.append("    ]")
    out.append("}")
    return "\n".join(out)


# ──────────────────────────────────────────────────────────────────────────
# Write files
# ──────────────────────────────────────────────────────────────────────────


OUT_DIR = ROOT / "codex-codes" / "src" / "protocol_generated"
OUT_DIR.mkdir(exist_ok=True)
(OUT_DIR / "mod.rs").write_text(
    "// AUTO-GENERATED by scripts/codegen_protocol.py — DO NOT EDIT BY HAND.\n"
    "pub mod types;\n"
    "pub mod samples;\n"
)
(OUT_DIR / "types.rs").write_text(emit_generated_module())
(OUT_DIR / "samples.rs").write_text(emit_samples_module())

print(f"wrote {OUT_DIR}/mod.rs / types.rs / samples.rs")
