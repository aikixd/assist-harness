# assist-harness

`assist-harness` is a personal workspace for building small read-only CLI tools that my personal assistant can use to access external resources safely.

The goal is to give the assistant practical access to things like email, docs, calendars, and similar systems through explicit tool interfaces, without turning the assistant loose on raw credentials or mutation-heavy APIs.

## What This Repo Is For

- personal-use tooling for my own assistant workflows
- read-only access to external systems
- compact CLI UX that works well for LLM-driven use
- shared infrastructure for concerns like OAuth and local secret/token storage

## Layout

```text
docs/   - specs, contracts, test plans, and repo guidance
libs/   - shared Rust crates
tools/  - individual CLI tools
```

Each tool lives in its own crate under `tools/`. Shared code lives under `libs/`.

## Current Status

The first tool is `pa-mail`, a read-only mail CLI with:
- Google OAuth setup
- multi-account Gmail access
- `accounts`, `list`, and `get` commands

Authoritative mail docs live under [docs/tools/email](/home/aikixd/Dev/Personal/assist-harness/docs/tools/email).

## Conventions

- tools in this repo are read-only
- live account data and tokens are stored outside the repo
- local storage conventions are documented in [docs/guide.md](/home/aikixd/Dev/Personal/assist-harness/docs/guide.md)

## Workspace

This repository is a Cargo workspace.
