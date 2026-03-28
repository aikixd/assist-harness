# `pa-mail` Tracking Link Scoring

This document defines the initial scoring model for suspected tracking links in cleaned mail bodies.

It is part of the mail tool specification.

## Purpose

Tracking-link detection should be heuristic rather than binary by construction.

Each link receives a suspicion score.

If a link score is greater than `1.0`, it is considered a suspected tracking link.

Suspected tracking links:
- should be omitted from cleaned body text
- should not dominate agent-facing output
- should be counted in message metadata as stripped links

Expected metadata wording:
```text
stripped <n> suspected tracking links
```

## Scoring Model

Base score:
- `0.0`

Tracking threshold:
- `score > 1.0`

The total score is the sum of the matched heuristics below, subject to any per-family caps.

## Heuristics

### 1. Known Redirect Host

Score:
- `+1.25`

Examples of host patterns:
- `click.*`
- `track.*`
- `lnk.*`

Rationale:
- this is a strong enough signal to classify the link by itself

### 2. Redirect-Style Path

Score:
- `+0.75`

Examples:
- `/click`
- `/track`
- `/redirect`
- `/r/`
- `/c/`
- `/out/`

Rationale:
- strong signal, but not always sufficient by itself

### 3. Explicit Redirect Target Parameter

Score:
- `+0.9`

Examples:
- `url=...`
- `u=...`
- `target=...`
- `dest=...`
- `destination=...`
- `redirect=...`
- `redir=...`

Rationale:
- links that embed another destination URL are often tracking or redirect hops

### 4. Known Tracking Parameters

Score:
- `+0.2` per family
- cap: `+0.8`

Examples:
- `utm_*`
- `mc_*`
- `vero_*`
- `hs_*`
- `gclid`
- `fbclid`
- `msclkid`

Rationale:
- useful weak-to-medium signals
- a single tracking parameter should not usually trigger stripping on its own

### 5. Signature or Token Parameters

Score:
- `+0.35` per family
- cap: `+0.7`

Examples:
- `token`
- `sig`
- `signature`
- `hash`
- `hmac`
- `expires`

Rationale:
- common in wrapped or tracked links
- also appears in legitimate signed links, so it should not classify the link alone

### 6. High Query Parameter Count

Score:
- `>= 5 params`: `+0.25`
- `>= 10 params`: `+0.5`

Rationale:
- tracked links often accumulate large parameter sets

### 7. Very Long URL

Score:
- `> 120 chars`: `+0.25`
- `> 240 chars`: `+0.5`
- `> 500 chars`: `+0.8`

Rationale:
- URL length is a weak signal alone, but useful in combination

### 8. Encoded or Blob-Like Path or Query

Score:
- `+0.5`

Examples:
- long base64-like segments
- unusually dense percent-encoding

Rationale:
- redirect wrappers often encode the true destination or campaign payload

### 9. Clean Text vs Href Mismatch

Score:
- `+0.8`

Example:
- visible anchor text suggests a clean destination
- actual href points somewhere much more suspicious or wrapped

Rationale:
- strong signal when both visible text and href are available

### 10. Suspicious Subdomain Prefix

Score:
- `+0.35`

Examples:
- `click.`
- `track.`
- `lnk.`
- `email.`
- `mg.`

Rationale:
- weak on its own, useful in combination

## Output Handling

If a link crosses the tracking threshold:
- omit it from cleaned `body_text`
- do not let it dominate `body_preview`
- count it in stripped-link metadata

Exact interaction with extracted `links` output is `TBD`.

## Notes

- This model is intentionally heuristic.
- The threshold and weights may be revised after real-world testing.
- The goal is to suppress obvious tracking noise without stripping too many legitimate links.
