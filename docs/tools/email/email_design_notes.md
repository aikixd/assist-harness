# `pa-mail` Design Notes

This document is for working design ideas that are not yet stable enough to become part of the authoritative spec.

If this document conflicts with [email_spec.md](/home/aikixd/Dev/Personal/assist-harness/docs/tools/email/email_spec.md), the spec wins.

Use this file for:
- exploratory pipeline ideas
- open questions
- tradeoffs
- concrete examples from real usage
- implementation directions that still need discussion

Do not treat this file as a CLI contract.

## Current Topic: Body Cleaning Pipeline

### Why This Exists

The current mail body rendering is usable for some messages, but still too noisy for HTML-heavy notification emails such as Notion updates and newsletter-style mail.

The problem is broader than HTML conversion alone. What we need is a configurable cleanup pipeline that turns provider message bodies into readable, low-noise text while preserving meaningful structure.

### Current Observations

- Some HTML-heavy messages contain too much converted junk.
- Tracking links dominate the visible body in some messages.
- Layout artifacts such as table markers leak into the cleaned body.
- Some messages lose meaningful block boundaries during conversion.
- Aggressive deduplication is risky because content that looks repetitive may actually be meaningful update history.

### Notion Example

The Notion update email we inspected suggests:
- `Quote` is a visible content label, not a reply-chain quote marker
- the content under `Quote` is part of the document update itself
- repeated-looking sections may reflect change history rather than accidental duplication
- preserving line and block boundaries matters more than trying to collapse repeated content

### Settled Constraints

These are not full spec statements yet, but they are strong design constraints for this topic:

- Prefer structure restoration over aggressive semantic deduplication.
- Keep the pipeline generic by default.
- Avoid sender-specific handlers unless generic cleanup still leaves a sender especially poor.
- Classic email reply-chain quotes are a future cleanup concern, but they are different from content labels such as `Quote`.

### Candidate Pipeline Stages

These stages are exploratory and may change:

1. Source selection
- choose the best body source from multipart content
- avoid rendering duplicate plain-text and HTML content when they represent the same body

2. Base conversion
- convert HTML with a maintained library
- preserve paragraphs, lists, and obvious block structure as much as possible

3. Structure normalization
- normalize whitespace
- preserve block boundaries
- restore missing line breaks where conversion flattened meaningful structure
- remove clear layout artifacts when they are presentational rather than semantic

4. Link cleanup
- keep links readable in body text
- trim visible link length by default
- preserve full links separately in extracted link data
- reduce tracking-link noise when possible

5. Boilerplate cleanup
- remove footer, legal, settings, unsubscribe, and product-tail boilerplate when confidence is high

6. Quote cleanup
- later: handle classic reply-chain quotes such as `>`-prefixed text and common reply markers

### Candidate Concrete Rules

These are promising candidates, but not yet authoritative:

- Visible link text in cleaned body output should be trimmed to a readable default length, likely around 100 to 120 characters.
- Preview generation should happen after body cleanup, not before it.
- If the original message visually separates a short block label from its content, the cleaned output should preserve that separation whenever possible.

### Open Questions

- How aggressive should tracking-link cleanup be in generic mode?
- Should cleaned body text keep markdown-style links, plain text links, or a hybrid representation?
- Which layout artifacts are safe to remove generically without harming meaning?
- How should we detect high-confidence boilerplate versus content that merely looks repetitive?
- At what point, if any, do sender-specific cleanup hooks become justified?

### Ideas To Avoid For Now

- Do not rely on aggressive generic deduplication of repeated-looking blocks.
- Do not treat every `Quote` label as an email reply-chain quote.
- Do not solve the problem primarily through sender-specific template parsing.
