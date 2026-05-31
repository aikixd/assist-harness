# `pa-tg` Telegram Spec

This document is the authoritative V1 spec draft for `pa-tg`.

Purpose: define a small Telegram Bot API tool that creates one trusted communication channel between the assistant and the tool owner.

This document should be concrete enough to drive:
- CLI design
- local trust-state design
- end-to-end behavior tests

This document is not an implementation plan.

## Goals

- provide a compact assistant-facing Telegram CLI
- support one secure communication path between the assistant and the tool owner
- allow low-risk outbound messages only to locally trusted chats
- ignore unsolicited inbound traffic by default
- keep default output compact and LLM-friendly
- keep all secrets and live state outside the repo

## Repo-Level Position

This repo generally prefers read-only tools.

`pa-tg` is a scoped exception:
- it may send messages
- it may only send to locally trusted Telegram private chats
- trust is established by a local authentication step that requires knowledge of a one-time key

The intent is not to create a general mutation tool.

The intent is to create a narrow communication channel between the assistant and the tool owner.

## Provider Scope

V1 uses:
- Telegram Bot API

V1 does not use:
- TDLib
- MTProto client flows
- user-account login
- webhooks

V1 uses polling through Bot API updates.

## Security Model

Telegram itself is not sufficient to decide who is trusted.

Trust is established only when both of these are true:
- a user sends a one-time key to the bot in a Telegram private chat
- the local operator runs `pa-tg auth` and provides the same key locally

This means:
- random Telegram users may message the bot
- those messages do not create a trusted channel
- the tool must not reveal their contents through normal assistant workflows
- the tool must not send replies to those chats

## Trust Model

V1 trust unit:
- one Telegram private chat

Each trusted peer record stores:
- bot name
- peer alias
- `chat_id`
- `from_user_id`
- username snapshot if available
- display-name snapshot if available
- paired timestamp
- update id used for pairing
- status

Supported statuses:
- `trusted`
- `revoked`

V1 constraints:
- only private chats can be trusted
- group chats, channels, and forum topics are out of scope
- a revoked peer is never selected implicitly

## V1 Command Surface

V1 commands:
- `pa-tg --help`
- `pa-tg config bot add`
- `pa-tg bots`
- `pa-tg auth --alias <alias> [--bot <name>] [--stdin]`
- `pa-tg peers [--bot <name>] [--all] [--json]`
- `pa-tg pending [--bot <name>] [--peer <alias>]`
- `pa-tg recv [--bot <name>] [--peer <alias>] [--limit <n>] [--json]`
- `pa-tg send [--bot <name>] [--peer <alias>] --text <text> [--json]`
- `pa-tg peers revoke <alias> [--bot <name>]`

Rules:
- `send` targets trusted peers only
- `pending` counts trusted pending messages without consuming them
- `recv` returns messages from trusted peers only
- `recv` consumes update stream progress even when it discards untrusted updates
- `auth` may create trust but must not send any Telegram message itself
- `peers` is the discovery path for trusted destinations

## Bot Identity

Bot selection must be explicit in local config.

Recommended V1 model:
- support one or more named bots
- each bot has a local alias such as `main`
- each bot stores a Bot API token outside the repo

If `--bot` is omitted:
- use the only configured bot if exactly one exists
- otherwise fail and ask for `--bot`

## Auth Flow

### Purpose

Bind one Telegram private chat to one local trusted peer record without trusting Telegram identity alone.

### Supported Flow

1. The owner sends a high-entropy one-time key to the bot in a Telegram private chat.
2. The owner runs `pa-tg auth --alias <alias>`.
3. The tool prompts for the key unless `--stdin` is used.
4. The tool fetches pending Bot API updates for the selected bot.
5. The tool searches for exactly one private-chat message whose text exactly matches the provided key.
6. If exactly one eligible message is found, the tool creates or refreshes the trusted peer binding for that chat using the provided alias.
7. The tool advances the update cursor past the processed updates.

### Input Rules

Required V1 input rules:
- `--alias <alias>` is mandatory
- the alias must be non-empty
- the alias must be unique within the selected bot
- the auth key format is a lowercase GUID in canonical `8-4-4-4-12` form
- surrounding whitespace is trimmed before comparison
- after trimming, the key must match exactly
- the key is treated as one-time and consumed on success

Recommended operator UX:
- prompt by default, so the key does not appear in shell history
- support `--stdin` for scripted use

### Match Eligibility

A message is eligible for auth only if:
- it arrived through the selected bot's pending updates
- it is a message update shape supported by V1
- the chat type is `private`
- the message text, after surrounding-whitespace trimming, exactly equals the provided key

### Match Outcome Rules

If zero eligible matches are found:
- fail with a retry-oriented message

Example:
```text
auth key not found in pending updates; resend the key in Telegram and try again
```

If more than one eligible match is found:
- fail
- do not trust any peer

If one eligible match is found:
- trust that chat
- create a new trusted peer record if the chat is new
- refresh the existing trusted peer record if the `chat_id` is already known
- store the operator-provided alias on the trusted peer record

### Auth And `recv` Race

V1 explicitly allows a race between `auth` and `recv`.

If `recv` consumes the pending auth message first:
- the auth attempt may fail
- the operator may resend the key and retry

This is acceptable V1 behavior.

The tool does not need to preserve unauthenticated traffic in local storage to resolve this race.

## `recv`

### Purpose

Read inbound traffic from trusted peers while treating untrusted traffic as nonexistent for normal assistant use.

### Scope

V1 `recv` works from Bot API pending updates.

V1 does not provide:
- a historical backfill API
- a separate inbox for unauthenticated chats
- a review command for discarded messages

### Filtering Rules

`recv` must:
- consume pending updates from Telegram
- extract supported inbound message events
- return only messages whose chat is locally trusted
- discard unsupported or untrusted updates from output

Discarded updates:
- are not shown
- are not stored for later recovery
- still advance the provider-side update cursor

If `--peer <alias>` is provided:
- return only messages from that trusted peer

If `--peer` is omitted:
- return messages from all trusted peers for the selected bot

### Output Rules

Default output is compact text.

Suggested text shape:
```text
peer: owner
chat_id: 123456789
total: 2

---
update_id: 1001
message_id: 55
date: 2026-03-28T10:14
from: aikixd
text: please summarize the latest errors
---
```

`--json` is explicit.

### Limit Rules

`--limit <n>` limits returned trusted messages, not raw Telegram updates scanned.

The tool may need to read more than `n` raw updates in order to find `n` trusted messages.

## `pending`

### Purpose

Count pending inbound traffic from trusted peers without consuming it.

### Usage

V1 usage:
- `pa-tg pending [--bot <name>] [--peer <alias>]`

### Behavior

`pending` must:
- use the same trusted/private-chat filtering rules as `recv`
- count text and unsupported trusted inbound messages
- ignore untrusted chats and non-message updates
- not advance the stored update cursor

This means:
- a later `recv` should still return the same pending trusted messages
- `pending` acts as a non-consuming peek

### Output Rules

Default output is a bare integer.

Examples:
```text
0
3
```

No `--json` mode is required for V1.

## `send`

### Purpose

Send one outbound text message to a trusted peer.

### Usage

V1 usage:
- `pa-tg send [--bot <name>] [--peer <alias>] --text <text> [--json]`

### Selection Rules

If `--peer` is provided:
- it must resolve to a trusted peer for the selected bot

If `--peer` is omitted:
- use the only trusted peer if exactly one exists
- otherwise fail and require `--peer`

### Safety Rules

`send` must refuse:
- unknown peers
- revoked peers
- untrusted chats
- raw `chat_id` targets passed through a generic escape hatch

V1 should not support:
- attachments
- formatting modes
- reply markup
- group destinations

Text-only send is enough for V1.

## `peers`

### Purpose

Discover trusted destinations and their status without guessing aliases or ids.

Default `peers` output should list trusted peers for one bot.

Suggested text shape:
```text
peer: owner
status: trusted
bot: main
chat_id: 123456789
user_id: 555111222
username: aikixd
paired_at: 2026-03-28T10:12
```

`--all` may include revoked peers.

## `bots`

### Purpose

List configured bots and whether they appear usable.

Suggested fields:
- bot alias
- token status
- default peer summary if one exists

## `config bot add`

### Purpose

Add one Bot API token to local configuration.

Recommended interactive parameters:
- bot alias
- bot token

Recommended validation:
- alias must be non-empty
- alias must be unique
- token must be non-empty

V1 may optionally verify the token with `getMe` during setup.

## Time

Canonical displayed time format:
- local RFC3339-like timestamps such as `2026-03-28T10:14`

V1 does not need `--since` or `--until` on `recv`.

Reasoning:
- Bot API polling is naturally cursor-oriented
- the trusted-channel workflow is about consuming new messages, not scanning history

If time filters are added later, they should follow repo-wide timestamp conventions.

## Output Principles

Default output rules:
- default output is compact text
- `--json` is explicit
- output should be summary-first
- `recv` should show text content, not full raw provider payloads
- expected operational errors should print to stdout
- stderr should remain reserved for fatal process-level failures

## Local Storage

Per repo convention, `pa-tg` stores all live state outside the repo.

Recommended Linux layout:
- config: `~/.config/pa/tg/`
- local data: `~/.local/share/pa/tg/`
- cache: `~/.cache/pa/tg/`

Recommended V1 files:
- bot config under `~/.config/pa/tg/bots/`
- peer trust records under `~/.config/pa/tg/peers/`
- update cursor state under `~/.local/share/pa/tg/`

V1 should not store:
- unauthenticated inbound messages
- a recoverable archive of discarded updates

Sensitive local files should use strict permissions.

## Provider Interaction Model

V1 should use the Bot API polling model with update offsets.

Recommended polling rules:
- use `getUpdates`
- advance the offset after processing fetched updates
- treat offset advancement as normal even when updates are discarded

This means the tool intentionally prefers a clean channel over perfect recovery of unsolicited traffic.

## Supported Content

V1 inbound support:
- plain text messages

V1 may ignore:
- media-only messages
- edited messages
- callback queries
- commands that rely on bot-side menus
- stickers and reactions

If unsupported trusted content is received:
- `recv` should emit a compact placeholder
- the placeholder should identify the peer, timestamp, and unsupported content type when known

## Non-Goals

V1 does not include:
- user-account Telegram access
- webhooks
- group chat trust
- attachments
- bot keyboards
- outbound formatting controls
- a recovery path for discarded unauthenticated traffic
- auto-replies
- broad multi-user bot workflows

## Settled V1 Policy

The following policy choices are settled for V1:
- re-auth refreshes the existing trusted record when the same `chat_id` authenticates again
- trust is keyed by `chat_id`
- `auth` requires `--alias <alias>`
- auth keys use lowercase GUID format in canonical `8-4-4-4-12` form
- auth comparison trims surrounding whitespace and then compares for exact equality
- auth fails unless there is exactly one eligible match
- `recv` returns compact placeholders for unsupported trusted content
- `recv` always advances the Telegram update offset after processing fetched updates
- `send` may default to the only trusted peer when exactly one exists
- revocation is soft and preserves a `revoked` peer record
- only private chats may be trusted
