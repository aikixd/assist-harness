# `pa-mail` Spec (V1)

This document is an older contract sketch.

The authoritative V1 spec is [email_spec.md](/home/aikixd/Dev/Personal/assist-harness/docs/tools/email/email_spec.md).

Goal: let the assistant triage one or more inboxes without sending mail or modifying anything.

## Scope
- Read-only only
- Multi-account first
- Token-efficient by default
- Human-readable output first, structured output optional

## Command Name
- Recommended command: `pa-mail`
- Rationale:
  - consistent with the shared `pa-*` family
  - short
  - avoids generic command collisions

## Required Commands
- `pa-mail --help`
- `pa-mail accounts`
- `pa-mail list --since <time> [--until <time>] [--account <id>] [--label <label>] [--limit <n>]`
- `pa-mail get <id> [--account <id>]`

## Time Format
- Recommendation: RFC3339-like local timestamps, e.g. `2026-03-06T14:30`
- If friendlier input is supported, normalize internally to one format.

## Default Output Rules
- Default output is compact text.
- `list` should group results by account.
- `list` should show a short body preview, not full bodies.
- `get` should show the full readable body text by default.
- HTML should be stripped/converted by default.
- Raw HTML should be explicit, e.g. `--html`.
- Structured output should be explicit, e.g. `--json`.

## `accounts`
Purpose: discover available inboxes without guessing names.

Suggested output:
```text
acc: founder@velarium.one
label: Founder inbox

acc: hello@velarium.one
label: Shared inbound
```

## `list`
Purpose: show recent relevant mail in a low-token format.

Suggested output:
```text
acc: founder@velarium.one
new: 3
total: 8

---
id: 18c5f...
date: 2026-03-06T12:14
from: someone@example.com
to: founder@velarium.one
subject: Quick question about docs
labels: inbox, unread
body_preview: Can you share the latest documentation link? ...
---
```

### `list` notes
- Repeat the account header for each account block.
- `new` means unread/new in that account for the query window.
- `total` means total returned for that account after filters.
- `body_preview` should be short and readable.
- If an account has zero results, either omit it or show it clearly as empty; do not mix empty and non-empty states ambiguously.

## `get`
Purpose: inspect one message in enough detail to decide what to do.

Suggested output:
```text
acc: founder@velarium.one
id: 18c5f...
thread_id: 77aa2...
date: 2026-03-06T12:14
from: someone@example.com
to: founder@velarium.one
cc:
subject: Quick question about docs
labels: inbox, unread

body_text:
Can you share the latest documentation link?

links:
- https://example.com/...

attachments:
- spec.pdf | application/pdf | 48213
```

### `get` notes
- `body_text` is the default canonical body form.
- `body_html` is optional and should only appear when explicitly requested.
- Include `thread_id` if the provider exposes one.
- Include links/attachments only when present.

## Why Account Separation Matters
- Different inboxes imply different priorities and workflows.
- Shared mailboxes and personal/founder mail should not blur together.
- Grouping by account makes triage safer and faster.

## Minimum Useful Fields

### `list`
- `id`
- `date`
- `from`
- `to`
- `subject`
- `labels`
- `body_preview`
- `thread_id` (if available)
- `unread`
- `account`

### `get`
- `id`
- `thread_id` (if available)
- `date`
- `from`
- `to`
- `cc` (optional)
- `subject`
- `labels`
- `body_text`
- `body_html` (optional; only on request)
- `links` (optional)
- `attachments` (optional)
- `account`

## JSON Mode
- `--json` should expose the same information as text mode, but structured.
- JSON is for exact field access, debugging, or provider edge cases.
- Text mode remains the default because it is cheaper and usually enough.

## Assistant Use Cases
- Review what arrived since a given time
- Triage by inbox
- Read one message fully
- Decide whether something needs:
  - a reply draft
  - a task
  - a reminder
  - delegation

## Non-Goals (V1)
- Sending email
- Draft creation inside the provider
- Label modification
- Archiving / deleting / marking as read
- Syncing secrets through this repo
