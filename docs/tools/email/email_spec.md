# `pa-mail` Email Spec

This document is the authoritative spec for `pa-mail` V1.

If this document conflicts with [email_contract.md](/home/aikixd/Dev/Personal/assist-harness/docs/tools/email/email_contract.md), this document wins.

## Purpose

Define the V1 behavior of a read-only mail CLI for assistant use.

The spec should be concrete enough to drive:
- CLI design
- account/config design
- scenario-style end-to-end tests

This document is not an implementation plan.

## Goals

- support safe read-only inbox inspection
- work well for LLM-driven use
- stay token-efficient by default
- support multiple accounts
- keep the CLI and domain model provider-agnostic
- leave room for future providers without redesigning the surface area

## Repo-Level Constraints

- tools in this repo are read-only
- `pa-mail` must not send, modify, archive, label, delete, or mark messages as read
- provider access should request read-only scopes wherever the provider supports them

## V1 Scope

V1 supports:
- personal Gmail accounts
- Google Workspace Gmail accounts

V1 does not require:
- IMAP support
- JMAP support
- Proton Mail support

These remain future directions, but they should not distort V1 into a protocol-first design.

## Non-Goals

V1 does not include:
- sending mail
- draft creation inside the provider
- label or folder mutation
- mark-as-read
- archive or delete
- inbox rules
- background sync daemons
- embedded browser auth UX

## Architecture Direction

We should separate:
- user-facing CLI contract
- mail domain model
- provider adapter interface
- shared OAuth and token handling

The shared OAuth logic should live in a common crate under `libs`.

The mail tool should consume shared auth infrastructure rather than implementing provider token flows directly inside the tool crate.

## Adapter Strategy

Adapter preference:
- use a native/provider adapter when it offers better read-only semantics, richer data, or better auth ergonomics
- use protocol adapters only when a provider lacks a suitable native API or effectively hides access behind a protocol

Long-term adapter set:
- `google`
- `imap`
- `jmap`

V1 adapter set:
- `google`

## Provider Resolution

Provider selection must be explicit in account configuration.

Email-domain mapping may exist as a hint, but it is not the source of truth.

Reasoning:
- Google Workspace mailboxes can use custom domains
- email domain alone is not enough to infer provider safely
- adapter selection should not depend on guesswork

Initial provider hint map:
- `gmail.com` -> `google`

## Access Model

Preferred access model:
- OAuth-based delegated access
- no mailbox passwords stored in this repo
- read-only provider scopes where available

V1 assumes OAuth will also be reused by other tools, so the auth model must be shared infrastructure rather than mail-specific logic.

## OAuth UX

The local CLI auth flow should stay simple.

V1 expectations:
- the tool prints the URL the user needs to open
- the user opens that URL manually in the browser
- the user completes the provider flow manually
- the CLI may run a temporary loopback listener on `127.0.0.1` to receive the provider callback
- errors should be printed clearly so the user does not have to guess what failed

V1 does not require:
- an embedded browser
- a local GUI flow
- fancy TUI auth
- a pasted authorization-code flow

For Google V1, the supported auth path should use a loopback redirect listener rather than a pasted-code flow.

## Command Surface

V1 commands:
- `pa-mail help`
- `pa-mail config account add`
- `pa-mail accounts`
- `pa-mail list --since <time> [--until <time>] [--account <email>] [--label <label>] [--limit <n>] [--json]`
- `pa-mail get <id> <email> [--json]`

Rules:
- account identity is the mailbox email address
- message ids are provider message ids scoped to a specific account
- `get` requires both message id and account email
- `get` must not perform implicit cross-account search

## Time Format

Canonical time format:
- RFC3339-like local timestamps, for example `2026-03-06T14:30`

If friendlier inputs are ever supported, they should normalize internally to one canonical representation.

Time semantics:
- CLI time inputs are interpreted in the machine's local timezone
- provider timestamps may arrive in UTC or with explicit offsets, but filtering must behave by comparing normalized instants
- `--since` is inclusive
- `--until` is exclusive

## Query Model

Requests should be handled in two phases:
- build a provider-aware query plan from CLI arguments
- execute the query only if the plan is valid for the selected account/provider

This is important because some filters may be unsupported by a provider and should fail before any fetch happens.

## Filters

V1 filters:
- `--since`
- `--until`
- `--account`
- `--label`
- `--limit`

Semantics:
- `--limit` is per account, not global
- results are ordered newest first within each account
- account groups are ordered by account email
- `--label` is Google-specific in V1

If a filter is unsupported for the selected account/provider, the tool should fail at query-planning time and avoid fetching results.

Example error:
```text
filter --label is not supported by this account's provider
```

## Output Principles

Default output rules:
- default output is compact text
- `--json` is explicit
- text mode should be summary-first
- list mode should show previews, not full bodies
- get mode should show the readable full body by default
- errors that are part of expected operation should print to stdout
- stderr should be reserved for fatal process-level failures

Rendering rules:
- HTML should be converted using a maintained library rather than a custom parser
- raw HTML should not appear by default
- preview generation happens after HTML-to-readable-text rendering
- preview generation happens after whitespace normalization
- `body_preview` is capped at 250 characters

## `accounts` Command

Purpose:
- show which accounts are configured
- show which accounts are usable
- expose enough status detail for an agent to report problems clearly

Text output shape:
- each account starts with `<email> - <provider>`
- if the account is not ready, append or immediately follow with status information

Required surfaced fields:
- account email
- provider
- status

Suggested status values:
- `ready`
- `auth_required`
- `token_expired`
- `misconfigured`

Example:
```text
personal@gmail.com - google
status: ready

me@company.com - google
status: auth_required
```

## `config account add` Command

Purpose:
- add one local account configuration
- complete the provider auth flow
- store token material locally for later read-only access

Syntax:
- `pa-mail config account add`

V1 interaction flow:
1. prompt for account email
2. prompt for provider
3. print local config and token paths
4. start a temporary loopback listener on `127.0.0.1`
5. print the provider authorization URL
6. user opens the URL manually in the browser
7. provider redirects back to the loopback listener
8. CLI exchanges the returned code for token material
9. CLI writes the account config entry and stores the token
10. CLI prints final status

Rules:
- provider selection is explicit
- account config should only be written after successful token exchange
- if the account already exists, the command should fail clearly rather than silently duplicating it
- the listener should be generic shared OAuth infrastructure, not Google-specific infrastructure inside the mail crate

## `list` Command

Purpose:
- show recent matching messages in a compact, low-token format
- preserve account grouping so cross-account triage stays safe

Supported flags:
- `--since <time>`
- `--until <time>`
- `--account <email>`
- `--label <label>`
- `--limit <n>`
- `--json`

Text output conventions:
- `acc:` starts each account block
- `unread:` is used instead of `new:`
- `total:` reports total returned after filters for that account
- `---` separates messages
- `====` separates account groups

Required message fields in text mode:
- `id`
- `date`
- `from`
- `to`
- `subject`
- `labels` when available
- `body_preview`

Optional text fields:
- `thread_id`

Example:
```text
acc: personal@gmail.com
unread: 2
total: 5

---
id: 18c5f...
date: 2026-03-06T12:14
from: someone@example.com
to: personal@gmail.com
subject: Quick question about docs
labels: inbox, unread
body_preview: Can you share the latest documentation link? ...
---

====

acc: me@company.com
unread: 1
total: 1

---
id: 77aa2...
date: 2026-03-06T10:03
from: billing@example.com
to: me@company.com
subject: Invoice available
labels: inbox, unread
body_preview: Your March invoice is ready to review. ...
---
```

Account block rules:
- omit empty accounts or show them unambiguously as empty
- do not mix empty and non-empty presentation styles ambiguously

## `get` Command

Purpose:
- inspect one message in enough detail to decide what to do next

Syntax:
- `pa-mail get <id> <email>`

Text output fields:
- `acc`
- `id`
- `thread_id` when available
- `date`
- `from`
- `to`
- `cc` when present
- `subject`
- `labels` when available
- `body_text`
- `links` when present
- `attachments` when present

Example:
```text
acc: personal@gmail.com
id: 18c5f...
thread_id: 77aa2...
date: 2026-03-06T12:14
from: someone@example.com
to: personal@gmail.com
cc:
subject: Quick question about docs
labels: inbox, unread

body_text:
Can you share the latest documentation link?

links:
- https://example.com/docs

attachments:
- spec.pdf | application/pdf | 48213
```

Lookup rules:
- the account argument is mandatory
- do not search other accounts
- if the message is missing for the specified account, print:

```text
message with id <id> not found
```

## Domain Model

The mail domain model should stay provider-agnostic at its core.

Core fields should cover:
- account
- message id
- thread id when available
- timestamp
- from
- to
- cc when present
- subject
- unread
- body preview
- readable full body
- links
- attachment metadata

Provider-specific metadata should live in an extension bag.

Suggested shape:
- stable core mail model
- `extensions` bag for provider-specific fields

The extension bag should be JSON-like in spirit so it can represent:
- strings
- booleans
- numbers
- lists
- nested objects

Examples of extension data:
- Google labels
- provider categories
- provider raw ids
- protocol-specific flags that do not belong in the shared core

## JSON Mode

`--json` should expose the same conceptual information as text mode, but in structured form.

JSON mode is intended for:
- exact field access
- debugging
- edge cases where text-mode formatting is inconvenient

Text mode remains the default because it is cheaper for agent use.

## Error Handling

Text-mode error principles:
- keep errors readable and concise
- print expected operational errors to stdout
- reserve stderr for fatal process-level failures

Examples of expected operational errors:
- unsupported filters for the selected provider
- missing account argument where required
- message not found
- auth required
- expired token
- misconfigured account

## Account Status Model

`accounts` should expose both configured accounts and their current usability state.

The status model should be good enough for an agent to report the issue without guessing.

Minimum status set:
- `ready`
- `auth_required`
- `token_expired`
- `misconfigured`

Future versions may add more statuses if they remain easy to interpret.

## Read-Only Safety

Read-only is not just a UX promise. It should hold at:
- CLI surface
- query-planning layer
- provider access scope selection
- adapter behavior

V1 should be designed so mutation operations are absent from the interface and not merely hidden.

## Future Providers

Proton Mail is a planned future provider.

The detailed Proton access path is intentionally deferred, but the architecture should preserve room for:
- a provider-specific adapter
- a different auth or transport model from Google
- provider-specific metadata in the extension bag

Future protocol adapters such as IMAP or JMAP may be added later when there is a real testing path and a concrete need.

## Testing Direction

The first tests should be scenario-style end-to-end tests, not unit tests of internal modules.

The initial test suite should cover:
- account discovery with mixed statuses
- listing across multiple accounts
- listing for a single account
- per-account limit behavior
- newest-first ordering within account blocks
- account-group ordering by email
- HTML message rendering into readable text
- preview truncation to 250 characters after rendering
- fetching a message by id and account
- not-found behavior for `get`
- unsupported filter failure before fetch
- JSON output parity with text-mode concepts
- read-only guarantees at the CLI and provider-scope level

## Open Questions

- Do we want a future machine-readable error mode separate from `--json`?
- Should `accounts` always print `status: ready`, or omit the status line when the account is ready and only print status details for non-ready accounts?
