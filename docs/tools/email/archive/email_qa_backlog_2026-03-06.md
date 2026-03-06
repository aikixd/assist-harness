# `pa-mail` QA Backlog

Archived on 2026-03-06 after the first post-QA fix pass.

## Resolved Items

- [x] Fix `get` not-found behavior so it matches the spec and test suite.
  - Resolution:
    - Gmail message lookup now normalizes both `400` and `404` into:
      - `message with id <id> not found`
  - Verified live with:
    - `cargo run -q -p mail -- get does-not-exist aikixd@gmail.com`

- [x] Improve HTML body rendering in `get` so `body_text` is readable human text.
  - Resolution:
    - replaced the temporary hand-rolled tag stripper with `html-to-markdown-rs`
    - added a cleanup pass for style/script/meta noise and invisible formatting characters
  - Verification note:
    - the output is now markdown-like readable text rather than CSS/layout junk

- [x] Improve HTML-origin preview cleanup in `list`.
  - Resolution:
    - `list` previews now derive from rendered body text instead of Gmail's raw snippet
    - entity leakage and spacer noise were reduced as part of the renderer change
  - Verified live with:
    - `cargo run -q -p mail -- list --since 2026-03-06T00:00 --account aikixd@gmail.com --limit 1`

## Notes

- Time-window behavior looked correct in the first live QA pass and was not changed by this fix set.
- Multi-account grouping/order is still untested because only one Google account was configured during this QA cycle.
