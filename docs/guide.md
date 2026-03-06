# Repo Guide

This file is the canonical place for repository conventions.

Current convention:
- See [AGENTS.md](/home/aikixd/Dev/Personal/assist-harness/AGENTS.md) for collaboration rules that govern planning before implementation.

## Local Storage Convention

All `pa-*` tools should store user data outside the repo using a per-tool layout.

Recommended layout on Linux:
- config: `~/.config/pa/<tool>/`
- local data, including tokens: `~/.local/share/pa/<tool>/`
- cache: `~/.cache/pa/<tool>/`

Notes:
- this repo must not store live account data, tokens, or provider secrets
- per-tool storage is preferred over shared cross-tool storage because it is easier to inspect and remove safely
- tools should keep permissions appropriately strict for any sensitive local files
