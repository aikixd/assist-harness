# `pa-mail` Scenario Test Suite

This document defines the first scenario-style end-to-end test suite for `pa-mail`.

It is intended to validate the user-visible behavior of the tool against real accounts, not internal implementation details.

## Purpose

The goals of this suite are:
- validate the core CLI contract
- validate safe multi-account behavior
- validate time filtering semantics in local time
- validate readable output for agent use
- keep the suite practical to run against real personal and company inboxes

This suite is intentionally light on provider-status fault injection because that is expensive to exercise repeatedly with real OAuth access changes.

## Test Environment

Initial real accounts:
- personal Google account
- company Google Workspace account

V1 assumptions:
- both accounts use the `google` adapter
- both accounts are already configured for the tool
- both accounts are expected to be usable for normal test runs

## Time Semantics Under Test

These semantics come from the authoritative spec and must be treated as test-critical:
- CLI time inputs are interpreted in the machine's local timezone
- provider timestamps may be returned in UTC or with explicit offsets
- filtering must compare normalized instants correctly
- `--since` is inclusive
- `--until` is exclusive

This is the most important behavior area to validate early because timezone mistakes can silently hide or surface messages incorrectly.

Practical test guidance:
- for routine live validation, it is enough to take a known message timestamp and query around it with a narrow local-time window such as `timestamp - 1 minute` and `timestamp + 1 minute`
- if the inclusion and exclusion behavior is coherent across those small windows, that is strong practical evidence that time handling is correct
- a dedicated local-vs-UTC crossover fixture remains useful for stronger explicit regression coverage later

## Fixture Guidance

The suite should rely on a very small controlled set of known messages rather than broad assumptions about live inbox contents.

Recommended minimum fixtures:
- 1 plain-text message
- 1 HTML message
- 1 long-body message
- 1 Gmail-labeled message
- at least 2 messages placed near a time-boundary relevant to local-vs-UTC filtering

The fixtures may be split across personal and company inboxes.

## Test Cases

### 1. Accounts Smoke Test

Purpose:
- confirm the tool discovers both configured Google accounts

Command:
```text
pa-mail accounts
```

Expected behavior:
- both personal and company accounts are listed
- each entry starts with `<email> - google`
- each account is shown as usable for the normal happy-path run

Notes:
- this is a smoke test, not a deep status-matrix test

### 2. Multi-Account List Smoke Test

Purpose:
- confirm default account grouping and baseline list formatting

Command:
```text
pa-mail list --since <time>
```

Expected behavior:
- output is grouped by account
- account groups are ordered by account email
- each account block starts with `acc:`
- each account block includes `unread:` and `total:`
- each message is surrounded by `---`
- account groups are separated by `====`

### 3. Single-Account List Filter

Purpose:
- confirm `--account` restricts output to a single mailbox

Command:
```text
pa-mail list --since <time> --account <email>
```

Expected behavior:
- only the requested account appears
- no messages from the other configured account are shown

### 4. `get` Happy Path

Purpose:
- confirm one message can be fetched from a known list result

Flow:
1. Run `pa-mail list` with a time window that includes a known fixture
2. Capture one returned `id` for a known account
3. Run:

```text
pa-mail get <id> <email>
```

Expected behavior:
- the message is returned successfully
- output includes expected message fields such as `id`, `date`, `from`, `to`, `subject`, and `body_text`
- the tool does not attempt to search across other accounts

### 5. `get` Not Found

Purpose:
- confirm missing-message behavior is explicit and cheap

Command:
```text
pa-mail get does-not-exist <email>
```

Expected behavior:
- prints:

```text
message with id does-not-exist not found
```

### 6. Preview Truncation

Purpose:
- confirm previews are generated from rendered text and capped correctly

Fixture:
- a known long-body message

Command:
```text
pa-mail list --since <time> --account <email>
```

Expected behavior:
- `body_preview` is at most 250 characters
- truncation occurs after rendering and whitespace normalization
- output is readable and not raw HTML

### 7. HTML Rendering

Purpose:
- confirm HTML emails are rendered into readable text by default

Fixture:
- a known HTML message

Commands:
```text
pa-mail list --since <time> --account <email>
```

```text
pa-mail get <id> <email>
```

Expected behavior:
- preview text is readable human text
- full body is readable human text
- raw HTML tags are not shown by default

### 8. Gmail Label Filter

Purpose:
- confirm the Google-specific label filter works on supported accounts

Fixture:
- a known Gmail message with a known label

Command:
```text
pa-mail list --since <time> --account <google-email> --label <label>
```

Expected behavior:
- only messages matching the label are returned
- formatting remains consistent with normal list output

### 9. Local-Time Window Filtering

Purpose:
- confirm time filtering is evaluated in local CLI time

Fixture:
- known messages with timestamps around a chosen local query window

Command:
```text
pa-mail list --since <local-time-a> --until <local-time-b> --account <email>
```

Expected behavior:
- messages whose normalized timestamps fall inside `[since, until)` are included
- messages outside that interval are excluded

### 10. Local-vs-UTC Boundary Case

Purpose:
- catch timezone bugs where provider UTC timestamps cross a local-date boundary

Fixture:
- at least one message whose UTC date differs from its local date
- the local/UTC offset should be large enough that a naive implementation would misclassify the message

Command:
```text
pa-mail list --since <local-time-a> --until <local-time-b> --account <email>
```

Expected behavior:
- inclusion/exclusion is correct according to local CLI time, not naive UTC date matching

### 11. Inclusive `since` Boundary

Purpose:
- validate the lower bound explicitly

Fixture:
- a message whose timestamp exactly matches the chosen `--since` instant

Command:
```text
pa-mail list --since <exact-message-time> --until <later-time> --account <email>
```

Expected behavior:
- the message is included

### 12. Exclusive `until` Boundary

Purpose:
- validate the upper bound explicitly

Fixture:
- a message whose timestamp exactly matches the chosen `--until` instant

Command:
```text
pa-mail list --since <earlier-time> --until <exact-message-time> --account <email>
```

Expected behavior:
- the message is excluded

## Lower-Priority Cases For Later

These are useful, but not required for the first real-inbox suite:
- auth-required status loops
- token-expired status loops
- misconfigured account states
- deep JSON snapshot testing
- attachment-heavy scenarios
- future-provider unsupported-filter scenarios

## Operator Notes

Because the suite runs against real inboxes:
- prefer a small stable fixture set over broad mailbox assumptions
- record the exact query windows used for time-boundary checks
- avoid tests that require frequent add-access/remove-access churn
- keep failure messages concrete so a follow-up agent can report issues clearly
