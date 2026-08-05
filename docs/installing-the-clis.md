# Installing and updating the wrapped CLI tools

Each crate in this workspace wraps a vendor CLI. This page collects the
install and update procedures from the providers' own docs, plus what we've
verified on a live box. Keeping the local CLI current matters here: the
nightly drift workflows compare our committed schemas/fingerprints against
the installed (or freshly-installed) tool, so "update the CLI" is step one
of every re-snapshot cycle.

| Tool | Wrapped by | Install (primary) | Update | Verify |
|------|-----------|-------------------|--------|--------|
| Claude Code | `claude-codes` | `curl -fsSL https://claude.ai/install.sh \| bash` | `claude update` | `claude --version` |
| Codex | `codex-codes` | `npm install -g @openai/codex` | `npm install -g @openai/codex@latest` | `codex --version` |
| opencode | `opencode-codes` | `curl -fsSL https://opencode.ai/install \| bash` | `opencode upgrade` | `opencode --version` |
| Muse Code | `muse-codes` | `curl -fsSL https://dev.meta.ai/install.sh \| bash` | re-run installer | `muse --version` |

---

## Claude Code (Anthropic)

Docs: <https://docs.anthropic.com/en/docs/claude-code>

**Install** — native installer (recommended by Anthropic):

```bash
curl -fsSL https://claude.ai/install.sh | bash
```

Alternates: `npm install -g @anthropic-ai/claude-code`; an existing install
can migrate itself to the native build with `claude install`.

**Update** — self-updating:

```bash
claude update
```

Claude Code also auto-updates in the background by default. Verified here:
`claude update` moved 2.1.220 → 2.1.222 in place.

**Layout**: launcher at `~/.local/bin/claude`; versioned builds under
`~/.local/share/claude/versions/<version>`. Credentials in
`~/.claude/.credentials.json` (Linux) or the keychain (macOS).

**Auth**: `claude auth login` / `claude setup-token` — interactive Ink
TUIs. For programmatic flows use `claude-codes`' `auth` module (PTY-driven;
see the crate docs — do not try to pipe these commands).

## Codex (OpenAI)

Docs: <https://github.com/openai/codex>

**Install** — npm package (also available via Homebrew):

```bash
npm install -g @openai/codex
# or: brew install codex
```

**Update** — npm has no self-updater; reinstall at latest:

```bash
npm install -g @openai/codex@latest
```

**Auth**: `codex login` (ChatGPT OAuth in a browser) or
`codex login --api-key`; status via `codex login status`. Credentials in
`~/.codex/auth.json`. Programmatic login is protocol-native — see
`codex-codes`' `account_login_start` and friends.

**Note**: pre-release `-alpha` versions appear on npm; the drift tooling
and this workspace track **stable** releases plus `openai/codex@main`
schema snapshots.

## opencode (SST)

Docs: <https://opencode.ai/docs>

**Install** — installer script (also npm, Homebrew, Paru):

```bash
curl -fsSL https://opencode.ai/install | bash
# or: npm install -g opencode-ai
```

**Update** — built-in upgrader, which also takes a pinned target:

```bash
opencode upgrade            # latest
opencode upgrade v1.18.10   # specific version
```

If installed via npm, `npm install -g opencode-ai@latest` works too.

**Auth**: `opencode auth login` for provider credentials. The server mode
(`opencode serve`) that `opencode-codes` drives needs no credentials for
session-management endpoints; model calls require a configured provider.

## Muse Code (Meta)

Docs: <https://dev.meta.ai/docs> · beta, macOS + Linux

**Install** — installer script (sha256-checked payload):

```bash
curl -fsSL https://dev.meta.ai/install.sh | bash
```

**Update** — no self-update subcommand as of 0.1.0; re-run the installer:

```bash
curl -fsSL https://dev.meta.ai/install.sh | bash
muse --version   # e.g. "Muse Code 0.1.0 (0.1.0-R708.1)"
```

**Layout**: binary at `~/.local/bin/muse`; sessions/state under
`~/.local/share/muse/`; credentials at `~/.config/muse/auth.json`.

**Auth** — three programmatic-friendly paths (see `muse-codes`' `auth`
module):

```bash
muse login                    # browser device-code flow (plain stdout)
muse auth set --api-key-stdin # API key via stdin — never argv
export META_API_KEY=...       # env var; always wins over saved creds
```

Note: `muse logout` empties the providers map in `auth.json` but keeps the
file — check contents, not existence, when probing credential state.

---

## After updating a CLI

1. Run that crate's drift check locally (or wait for the nightly):
   `scripts/check_claude_schema_drift.py`,
   `scripts/check_codex_schema_drift.py`, `opencode-codes`' drift tests,
   `scripts/check_muse_schema_drift.py`.
2. If drift is reported: re-snapshot, regenerate/extend types, run the
   live integration suite against the new binary, bump the crate's
   version and `TESTED_*` constants, and update the README version lines
   (CI's version-consistency check enforces them).
