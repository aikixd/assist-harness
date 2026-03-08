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

## Notes

- Prefer adding concrete repro notes or commands here when one of these cases appears in real use.
- When a backlog item is exercised and resolved, move the record to an archive note rather than deleting the history.
