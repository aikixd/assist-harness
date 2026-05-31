# Codex tmux Nudge Spec (V1)

Goal: add low-friction proactive nudging to an existing interactive Codex session that lives in `tmux`, without building a full PTY wrapper.

This spec is for a helper/sidecar tool family that runs in the separate tools workspace, not in this repo.

## Core Idea
- Keep the normal interactive `codex` session unchanged.
- Use `tmux` as the terminal/control substrate.
- Add thin helper commands that:
  - register the target Codex pane
  - observe its state
  - compute a lightweight heartbeat
  - deliver either a display-only nudge or a gated input injection
- Use Telegram as the durable fallback/escalation path when in-terminal nudging is unsafe.

## Why This Shape
- The user already keeps Codex open in a persistent `tmux` pane.
- A naive cron-spawned second Codex instance creates split-brain behavior.
- A custom PTY wrapper would provide more control, but is significantly more complex.
- `tmux` already exposes enough metadata for pane targeting and rough activity detection.

## Non-Goals (V1)
- Replacing the normal Codex interactive CLI
- Building a full PTY proxy/wrapper
- Rich bidirectional control of a running Codex session
- Guaranteed-perfect detection of "user is typing right now"
- Anything that silently bypasses the injection safety policy

## Tool Family
- Recommended prefix: `pa-codex`
- Expected commands:
  - `pa-codex register`
  - `pa-codex heartbeat`
  - `pa-codex status`
  - `pa-codex nudge`
  - optional later: `pa-codex inject`

## High-Level Flow
1. The interactive Codex pane is registered under a logical name such as `assistant`.
2. A heartbeat command periodically inspects tmux state for that pane and writes a small JSON state file.
3. Cron or another wake process reads the JSON state file.
4. If the pane is safe to nudge locally, the tool uses a tmux-native nudge.
5. If local nudging is unsafe or the pane cannot be trusted, the system falls back to Telegram.

## Command Spec

### `pa-codex register`
Purpose: record which tmux pane represents a logical Codex session.

Suggested usage:
```bash
pa-codex register assistant
pa-codex register assistant --pane %3
pa-codex register assistant --cwd /home/aikixd/Dev/Personal/assistant
```

Behavior:
- Stores one session record under a logical name, e.g. `assistant`.
- Defaults to the current tmux pane if run from inside tmux.
- Captures enough identifiers to relocate the pane later:
  - `pane_id`
  - `pane_tty`
  - `pane_pid`
  - `pane_current_path`
  - `pane_current_command`
  - `session_name`
  - `window_index`
  - `pane_index`
  - logical `name`
- Should fail clearly if not running in tmux and no `--pane` was provided.

Suggested text output:
```text
name: assistant
pane_id: %3
session: 0
window: 3
pane: 1
path: /home/aikixd/Dev/Personal/assistant
command: codex
status: registered
```

### `pa-codex heartbeat`
Purpose: inspect the registered pane and compute a coarse "safe to nudge" state.

Suggested usage:
```bash
pa-codex heartbeat assistant
pa-codex heartbeat assistant --json
```

Behavior:
- Reads the registered pane metadata.
- Queries tmux for the current pane/window/session fields.
- Optionally captures the last small tail of pane output with `tmux capture-pane`.
- Writes a JSON state file for later consumers.
- Returns a compact status summary in text mode.

Suggested text output:
```text
name: assistant
alive: true
pane_id: %3
command: codex
path: /home/aikixd/Dev/Personal/assistant
window_active: true
pane_active: true
session_attached: true
idle_seconds: 84
output_quiet_seconds: 23
safe_to_inject: false
reason: recent_client_activity
```

### `pa-codex status`
Purpose: show the last known state without recomputing it unless requested.

Suggested usage:
```bash
pa-codex status assistant
pa-codex status assistant --json
pa-codex status assistant --refresh
```

Behavior:
- Reads the stored state file.
- `--refresh` may run a heartbeat first.

### `pa-codex nudge`
Purpose: send a local in-terminal nudge using tmux display primitives, not Codex input injection.

Suggested usage:
```bash
pa-codex nudge assistant --text "Road2 in 20 min"
pa-codex nudge assistant --text "Mail sweep still open" --ttl 10
```

Behavior:
- Uses a tmux-native display mechanism such as `display-message`.
- Does not send text into Codex as if typed.
- This is the preferred v1 local nudge path.

Suggested text output:
```text
name: assistant
pane_id: %3
mode: display
status: sent
```

### Optional Later: `pa-codex inject`
Purpose: inject actual text into the Codex pane using `tmux send-keys`.

Suggested usage:
```bash
pa-codex inject assistant --text "This is a cron-job wake."
```

Behavior:
- Must be gated by the safety policy below.
- Should default to refusal unless explicit safety checks pass.
- Should support `--force` only if the caller opts in very deliberately; avoid this in normal automation.

## Storage
- Config / registrations:
  - `~/.config/pa/codex/`
- State / heartbeat files:
  - `~/.local/share/pa/codex/`
- Cache / transient capture data:
  - `~/.cache/pa/codex/`

Recommended files:
- `~/.config/pa/codex/sessions/<name>.json`
- `~/.local/share/pa/codex/state/<name>.json`

## Registration Schema
Suggested JSON shape:
```json
{
  "name": "assistant",
  "registered_at": "2026-03-28T18:30:00+02:00",
  "pane_id": "%3",
  "pane_tty": "/dev/pts/5",
  "pane_pid": 129325,
  "session_name": "0",
  "window_index": 3,
  "pane_index": 1,
  "pane_current_path": "/home/aikixd/Dev/Personal/assistant",
  "pane_current_command": "codex"
}
```

## Heartbeat State Schema
Suggested JSON shape:
```json
{
  "name": "assistant",
  "checked_at": "2026-03-28T18:31:00+02:00",
  "alive": true,
  "pane_found": true,
  "pane_id": "%3",
  "pane_pid": 129325,
  "pane_tty": "/dev/pts/5",
  "command": "codex",
  "path": "/home/aikixd/Dev/Personal/assistant",
  "pane_active": true,
  "window_active": true,
  "session_attached": true,
  "pane_in_mode": false,
  "client_activity": 1774710820,
  "window_activity": 1774710820,
  "last_seen_client_activity_at": "2026-03-28T18:30:20+02:00",
  "last_output_change_at": "2026-03-28T18:30:45+02:00",
  "tail_hash": "sha256:...",
  "tail_sample": "...optional short sanitized sample...",
  "idle_seconds": 40,
  "output_quiet_seconds": 15,
  "safe_to_display_nudge": true,
  "safe_to_inject": false,
  "reason": "recent_output_change"
}
```

## tmux Fields To Use
Confirmed useful tmux fields:
- `pane_current_command`
- `pane_current_path`
- `pane_pid`
- `pane_tty`
- `pane_id`
- `pane_active`
- `window_active`
- `session_attached`
- `window_active_clients`
- `client_activity`
- `session_activity`
- `window_activity`
- `pane_in_mode`
- `pane_unseen_changes`

## What tmux Can and Cannot Tell Us
tmux is good at:
- locating the right pane
- telling whether it is the current pane/window
- telling whether the tmux session is attached
- exposing coarse activity timestamps

tmux is not good at:
- precisely telling the last user keypress time for one pane
- precisely telling the last Codex output time for one pane
- guaranteeing that injected text will not race with live interaction

Therefore, safety is heuristic, not perfect.

## Heartbeat / Safety Heuristics
The heartbeat logic should compute two coarse signals:

### Recent user activity
Treat the pane as recently user-active if:
- the target pane is in the active window and active pane, and
- `client_activity` changed recently

This is only a proxy for user typing, but is good enough for v1 gating.

### Recent Codex output
Estimate recent output by:
- capturing a small tail from the pane
- hashing it
- storing the last seen hash
- if the hash changes, update `last_output_change_at`

This is a coarse proxy for "Codex screen changed recently."

## Safety Policy

### Safe to display nudge
Display-only nudges are usually safe when:
- pane exists
- command is still `codex`
- pane is not dead

No need for strict idleness, because display nudges do not type into Codex.

### Safe to inject
Input injection should require all of:
- pane exists
- command is still `codex`
- registered path still matches expected path
- pane is not dead
- pane is not in copy mode (`pane_in_mode = 0`)
- tmux session is attached
- no recent client activity for a configured threshold
- no recent output change for a configured threshold
- no outstanding queued injection exists

Recommended conservative thresholds:
- `min_idle_before_inject_seconds = 30`
- `min_output_quiet_before_inject_seconds = 15`

Recommended additional guard:
- if the pane is the current active pane and the window is visible, prefer display-only or Telegram over injection unless the message is very important

## Injection Policy
Default policy:
- `nudge` is allowed
- `inject` is denied unless `safe_to_inject = true`

Recommended v1 automation rule:
- Cron should use `nudge` for local/visible reminders
- Cron should use Telegram for durable/high-priority reminders
- Cron should not use `inject` automatically until real-world behavior is observed and trusted

## Detection / Matching Rules
To locate the correct pane, match in this order:
1. exact registered `pane_id` if it still exists
2. otherwise a fallback match on:
   - `pane_current_command == codex`
   - `pane_current_path == expected path`
3. if multiple matches remain:
   - prefer `window_active = 1`
   - then prefer `pane_active = 1`
   - otherwise fail with ambiguity

The tool must fail safely if it cannot uniquely identify the intended pane.

## Cron Integration
Suggested pattern:
- cron invokes a higher-level wake command every N minutes
- wake command runs `pa-codex heartbeat assistant`
- wake command decides:
  - do nothing
  - local `pa-codex nudge assistant --text ...`
  - Telegram `pa-tg send ...`
  - later maybe `pa-codex inject assistant --text ...`

## Output Rules
- Default output should be compact text
- `--json` should be available for `heartbeat` and `status`
- Errors should be concise and actionable

Examples:
```text
error: registered pane %3 no longer exists
error: multiple codex panes matched path /home/aikixd/Dev/Personal/assistant
error: inject denied; safe_to_inject=false reason=recent_client_activity
```

## Logging
Optional but recommended:
- append one structured log entry for:
  - registration updates
  - heartbeat refreshes only when state changes materially
  - nudges sent
  - inject attempts
  - inject refusals

Keep logs compact; this tool should not become a high-volume logger.

## Nice-to-Haves Later
- explicit queueing of deferred injections
- a small policy engine for "do nothing / nudge / Telegram / inject"
- prompt-pattern detection to improve safe-to-inject confidence
- multiple named Codex sessions across repos
- temporary suppression / snooze windows

## Open Questions
- Should `register` be invoked manually, or via a tiny shell alias/function that launches Codex and registers automatically?
- Should `nudge` use `display-message` only, or also support a status-line message mode?
- How much pane output, if any, should be stored in state for debugging without becoming privacy-noisy?
- When the pane is active and visible, should automation ever inject, or should it always prefer display-only plus Telegram?
