# `pa-*` Tooling Principles

Purpose: define one stable shape for assistant-facing tools so I can use them predictably without wasting tokens or relearning each CLI.

## Goals
- Keep command usage consistent across tools.
- Keep default output compact and naturally readable.
- Keep secrets out of this repo.
- Keep docs lean but good enough for discovery beyond the basics.
- Keep every tool in this repo read-only.

## Naming
- Use the prefix `pa-` for all assistant-facing tools.
- Recommended names:
  - `pa-mail`
  - `pa-notion`
  - `pa-calendar`
  - `pa-files`
- Avoid generic names like `emails` that can collide with other commands or feel inconsistent.

## Command Shape
- Shared pattern:
  - `pa-<tool> <action> --since <time> --until <time> --scope <id> --limit <n>`
- Not every tool needs every flag, but similar concepts should use similar names.
- Prefer explicit actions such as:
  - `list`
  - `changes`
  - `search`
  - `get <id>`
  - `help`
- Discovery commands should be available where relevant:
  - `accounts`
  - `workspaces`
  - `sources`

## Time
- Tools should accept one canonical time format consistently.
- Recommendation: use RFC3339-like local timestamps such as `2026-03-06T14:30`.
- Friendlier aliases can exist, but they should normalize to one internal format.
- Avoid ambiguous formats like `03/04/26 7pm`.

## Output Modes
- Default output should be compact text, optimized for low token usage.
- Structured output should be explicit, e.g. `--json`.
- Default mode should be "summary first, details on demand".
- Full/raw content should not be dumped in list views unless explicitly requested.

## Content Rules
- Prefer readable text over raw provider payloads.
- If content starts as HTML, default output should convert it to plain text or markdown-like readable text.
- Raw HTML should be opt-in only.
- Long bodies/content should use previews in list/change views; full content belongs in `get <id>`.

## Assistant Minimum Needs
- A discovery path:
  - `help`
  - a way to list available scopes (`accounts`, `workspaces`, etc.)
- A recent activity path:
  - `list` / `changes` with `--since`
- A detail path:
  - `get <id>`
- A filtering path:
  - `--account`, `--workspace`, labels/tags where relevant
- A structured path:
  - `--json` when exact fields matter

## Secrets
- No secrets in this repo.
- Tools should obtain credentials/config from env-based secure inputs in the separate workspace.
- Prefer env vars that point to opaque handles, encrypted files, or external secret stores rather than exposing raw secrets directly when possible.
- Tool docs should describe required env vars without printing secret values.
- OAuth client credentials may be provided via environment variables when needed for local setup flows.

## Local Storage
- Tools should store user data outside the repo using a per-tool layout.
- Recommended Linux layout:
  - config: `~/.config/pa/<tool>/`
  - local data, including tokens: `~/.local/share/pa/<tool>/`
  - cache: `~/.cache/pa/<tool>/`
- Per-tool storage is preferred over shared cross-tool storage so each tool can be inspected or removed independently.
- Sensitive local files should use appropriately strict permissions.

## Read-Only Rule
- All tools in this repo are read-only.
- Tools must not create, modify, delete, send, archive, label, or otherwise mutate provider state.
- If a future need involves mutations, that should be a separate tool or separately approved scope, not folded into these tools by default.

## Docs
- Each tool should have lean docs with:
  - what it does
  - core commands
  - flags
  - time format
  - output modes
  - a couple of examples
- `help` output should be enough for quick rediscovery.
- Longer provider-specific details can live in a separate implementation workspace if needed.

## Stability
- Once a pattern is chosen, avoid random CLI drift between tools.
- If a tool needs to deviate, the reason should be strong and documented.
- Backward-compatible growth is preferred over breaking flag renames.

## Recommended First Pass
- Start each tool with:
  - one discovery command
  - one recent-activity command
  - one detail command
  - optional `--json`
- Expand only after the default flow feels solid in real use.
