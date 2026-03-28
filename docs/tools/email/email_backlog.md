# `pa-mail` Backlog

This backlog tracks deferred follow-up work for `pa-mail`.

The tool is intended for personal use, so some cases can be handled when they first appear in real usage rather than being fully engineered up front.

## Deferred Test Coverage

- Cover `accounts` output when an account requires re-authentication and should report `auth_required`.
- Cover `accounts` output when a stored token is expired and refresh fails, and confirm `token_expired` behavior.
- Cover `accounts` output when local account configuration is malformed and should report `misconfigured`.
- Cover `config account add` when attempting to add an account that already exists.
- Cover behavior when provider credentials are missing from local config.
- Cover behavior when provider credentials are present but invalid.
- Cover recovery flow after token revocation or similar provider-side auth breakage.

## Time Coverage

- Add a stronger regression case for a known local-vs-UTC date crossover fixture once we naturally encounter one or create one intentionally.

## Potential UX Improvements

- Consider `pa-mail get --brief` to return headers plus a cleaned body summary and extracted links or attachments only.
- Improve heavy HTML notification cleanup, especially for Notion and newsletter-style emails.
- Strip tracking redirects, layout and table artifacts, duplicated blocks, quoted sections, and giant footer or legal boilerplate from cleaned bodies.
- Preserve the small high-signal summary in notification emails instead of dumping the entire converted body when most of it is repetitive or low-value.
- Handle traditional quoted email replies more cleanly in the future, especially `>`-prefixed quoted sections and common reply-chain markers.

## Notes

- Prefer adding concrete repro notes or commands here when one of these cases appears in real use.
- When a backlog item is exercised and resolved, move the record to an archive note rather than deleting the history.
