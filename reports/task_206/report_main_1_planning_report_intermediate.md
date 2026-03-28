# Plan: make-error-up-to-date

## Changes

Both changes are in `zbobr-dispatcher/src/task.rs`.

### 1. Format error message in `RoleSession::set_error`

When `error` is `Some(message)`, wrap it with:
- A red unicode ❌ symbol prefix
- A timestamp suffix using `chrono::Utc::now().with_timezone(&self.zbobr.config().fixed_offset())` formatted as `YYYY-MM-DD HH:MM:SS +HHMM` (same format as `format_timestamp` in `stage_title.rs`, same timezone code as `cli.rs:422`)

The formatted string is stored in `task.error`. No changes to parsing or serialization needed.

### 2. Clear error on running state in `TaskSession::set_state`

Inside the `modify_task` closure, when `state.is_running()` is true, set `task.error = None` before updating `task.state`.

## Rationale

- Single file, minimal scope
- Follows existing timestamp conventions (same `fixed_offset()` approach)
- `is_running()` already exists on `State` — no new utilities needed
