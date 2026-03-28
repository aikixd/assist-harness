# `pa-mail` Boilerplate Scoring

This document defines the initial scoring model for suspected boilerplate blocks in cleaned mail bodies.

It is part of the mail tool specification.

## Purpose

Boilerplate stripping should be conservative.

Each candidate block receives a suspicion score.

If a block score is greater than `1.0`, it may be treated as suspected boilerplate.

The goal is to suppress clear footer and utility noise while preserving real message content.

When uncertain, the content should be preserved.

## Scoring Model

Base score:
- `0.0`

Boilerplate threshold:
- `score > 1.0`

The total score is the sum of the matched heuristics below, subject to any per-family caps.

This model intentionally favors under-stripping.

## Positive Heuristics

### 1. Tail Position

Score:
- `+0.35`

Condition:
- the block appears near the end of the cleaned body

Rationale:
- footer and utility content is more likely to appear at the end

### 2. Known Utility Phrase

Score:
- `+0.7`

Examples:
- `unsubscribe`
- `manage preferences`
- `email settings`
- `notification settings`
- `view in browser`
- `open in app`

Rationale:
- these phrases are common boilerplate signals, but should not usually classify a block alone

### 3. Legal or Policy Phrase

Score:
- `+0.9`

Examples:
- `privacy policy`
- `terms of service`
- `all rights reserved`
- `confidentiality notice`
- `this email and any attachments`

Rationale:
- legal footer language is a strong signal

### 4. Link-Heavy Utility Block

Score:
- `+0.45`

Condition:
- the block is dominated by short utility links or utility CTA text

Rationale:
- footer blocks often consist mostly of settings/help/privacy/unsubscribe links

### 5. Branding Tail

Score:
- `+0.4`

Condition:
- the block looks like a product or brand tail with short descriptive text rather than message content

Rationale:
- many emails end with a small product identity tail that is not useful for triage

### 6. Multi-Line Utility Cluster

Score:
- `+0.6`

Condition:
- consecutive short lines form a cluster of utility text or links

Examples:
- `Help`
- `Settings`
- `Privacy`
- `Unsubscribe`

Rationale:
- grouped utility links are a common footer pattern

### 7. Separator-Tail Pattern

Score:
- `+0.25`

Condition:
- the block appears after a visual divider or clearly footer-like structural break

Rationale:
- footer material often follows a separator, but this is weak on its own

## Negative Heuristics

These reduce the chance of stripping content that only looks boilerplate-like.

### 1. Real Paragraph Prose

Score:
- `-0.45`

Condition:
- the block contains normal prose rather than utility text or footer fragments

### 2. Topic Overlap With Main Message

Score:
- `-0.4`

Condition:
- the block clearly overlaps with the subject matter of the main message

Rationale:
- content blocks should not be stripped just because they appear late

### 3. Middle Position

Score:
- `-0.5`

Condition:
- the block appears in the middle of the message rather than the tail

### 4. Named Entity or Content-Rich Block

Score:
- `-0.35`

Condition:
- the block contains names, dates, or domain-specific message content

Rationale:
- this is a weak safeguard against stripping meaningful update content

## Safety Rule

This model should be applied conservatively:
- a block should only be stripped when the score clearly crosses the threshold
- if a block mixes utility signals with meaningful content, preserve it

## Notes

- This model is intentionally conservative and prefers under-stripping.
- The threshold and weights may be revised after real-world testing.
- Sender-specific cleanup rules, if they ever exist, should remain secondary to generic conservative handling.
