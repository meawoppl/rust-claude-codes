# Re-snapshotting claude-codes against a new Claude Code CLI

The Claude Code stream-json protocol evolves with every CLI release. This doc
describes how to extract the CLI's actual wire schemas from the shipped binary
and diff them against this crate, so any agent or contributor can repeat the
process. It was first run against CLI 2.1.205 (PR #180, issues #181–#191).

## Where the code lives

The npm package `@anthropic-ai/claude-code` is only a downloader shim — there
is no `cli.js` in it anymore. The real bundle is a **Bun-compiled ELF** with
the JavaScript embedded as plain bytes:

```bash
claude update                      # get the newest version first
readlink -f "$(which claude)"      # → ~/.local/share/claude/versions/<version>
```

Because the JS is embedded uncompressed, everything below is byte-level
`grep -a` / Python `re` over the ELF. No unpacking required.

## Step 1 — extract the SDK output schemas

```bash
python3 scripts/extract_claude_sdk_schemas.py -o /tmp/claude_sdk_schemas.txt
```

The CLI defines its wire types as lazy zod schemas,
`NAME=Se(()=>E.object({type:E.literal("..."),...}))`, and the SDK output
union is an `E.union([NAME(), ...])` over ~40 of them. Minified names change
every release; the structure does not. The script anchors on the
`rate_limit_event` literal, finds the union that references it, and dumps
every member schema plus transitive references, each labeled with its
resolved `type`/`subtype`.

If the script fails to find the anchor or union, the bundle layout changed;
fall back to manual spelunking (below) and update the script.

## Step 2 — diff against the crate

Map each extracted schema to its crate type and compare field-by-field:

| Wire type | Crate type (in `claude-codes/src/io/`) |
|---|---|
| `assistant` / `user` | `AssistantMessage`, `UserMessage` (`message_types.rs`) |
| `result` (success + error subtypes) | `ResultMessage` (`result.rs`) |
| `system/<subtype>` | `SystemSubtype` + per-subtype structs (`message_types.rs`) |
| `rate_limit_event` | `RateLimitEvent` (`rate_limit.rs`) |
| `control_request` / `control_response` | `control.rs` |
| everything else | `ClaudeOutput` variants (`claude_output.rs`) |

Things to check, in priority order:

1. **New top-level `type` literals.** `ClaudeOutput` is `#[serde(tag = "type")]`
   with no catch-all — an unmodeled type fails typed decode outright.
2. **Closed enums.** Any derive-serde enum without an `Unknown(String)`
   fallback hard-fails the whole frame when the CLI adds a value. Prefer the
   manual `as_str` / `From<&str>` / serde pattern used throughout the crate.
3. **Required → optional flips.** A crate field that is required while the
   wire schema says `.optional()` fails to parse frames that omit it.
4. **New fields.** Dropped on round-trip; the `assert_fully_wrapped` audit
   catches these for `system` frames.

Two caveats that save confusion:

- The `message` payloads of `assistant`/`user`, the `stream_event` `event`,
  and the result `usage` are `E.unknown()` in the CLI's own schema — raw
  Anthropic API passthrough. The crate types those against the API shape,
  not the CLI schema, so "the zod says unknown" is not drift.
- Minified names collide across bundle modules. When extracting by hand,
  prefer the definition nearest the union's byte offset.

## Manual spelunking recipes

Enumerate every wire `type`/`subtype` literal (broader than the SDK union —
includes internal orchestrator frames):

```bash
grep -aoE 'type:E\.literal\("[a-z_]+"\)' <binary> | sort | uniq -c
grep -aoE 'subtype:E\.literal\("[a-z_0-9]+"\)' <binary> | sort | uniq -c
```

Pull context around any anchor string (Python is much faster than grep's
regex context on a ~250 MB binary):

```python
data = open(BINARY, 'rb').read()
i = data.find(b'some_anchor_string')
print(data[i-500:i+1500].decode('utf-8', 'replace'))
```

Definitions end where the next `NAME=Se(` begins — split on that boundary
rather than balanced-paren parsing (regex literals in the minified JS break
paren counting).

Non-protocol provenance worth knowing (for fixtures and header-derived
state): live quota values in `rate_limit_event` come from
`anthropic-ratelimit-unified-*` response headers (including per-window
`-{5h|7d|7d_oi|overage}-utilization` / `-reset` pairs), the `/usage` panel
comes from `GET /api/oauth/usage`, and the CLI fires a 1-max-token probe
request (body `"quota"`, `source: "quota_check"`) at startup purely to
harvest those headers.

## Step 3 — land the changes

- File one issue per drifted type/area with the extracted schema excerpt
  (see #181–#191 for the shape), or fix directly.
- Follow the crate conventions: `skip_serializing_if = "Option::is_none"`,
  `Unknown(String)` fallbacks, round-trip tests against a full-fat JSON
  frame, `CHANGELOG.md` entry under `[Unreleased]`.
- Update the tested CLI version pin when the integration suite passes
  against the new binary.
