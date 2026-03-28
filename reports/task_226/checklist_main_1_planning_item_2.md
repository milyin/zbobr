
## Goal

Create a shared mechanism for formatting the status field and enforce at the API level that pausing always includes an explanatory status message.

## New constants in `zbobr-api/src/backend.rs`

Replace `ERROR_PREFIX` (single constant) with two icon constants:
- `STATUS_ICON_ERROR: char = '❌'` (X mark — for errors)
- `STATUS_ICON_QUESTION: char = '❓'` (question mark — for questions)
- Optionally `STATUS_ICON_PAUSE: char = '⏸'` for pipeline-configured pauses

## New helper

Add a `format_status_message(icon: char, message: &str) -> String` function in `zbobr-api/src/backend.rs` (or `context` module) that formats `"{icon} {timestamp} {message}"` using `chrono::Utc::now()`. Export it from `zbobr-api/src/lib.rs`.

This is the analogue of the current timestamp formatting in `set_error`.

## API changes in `TaskMut` trait (`zbobr-api/src/backend.rs`)

Remove:
- `set_pause(bool)` (the signature that allows pause=true without explanation)
- `set_error(Option<String>)`

Add:
- `set_status(status: Option<String>)` — raw setter, used internally
- `clear_pause()` — sets `pause=false` only
- `pause_with_status(icon: char, message: &str)` — atomically sets `pause=true` AND formats+sets `status` in one `modify_task` call
- `pause_with_signal_and_status(signal: Signal, icon: char, message: &str)` — same as above but also sets `signal`

Note: `set_confirm` and other boolean fields are unaffected.

## In `set_state` default impl

Change `task.error = None` → `task.status = None` when transitioning from non-running to running.

## Update `zbobr-api/src/lib.rs` exports

- Remove `ERROR_PREFIX`
- Export `STATUS_ICON_ERROR`, `STATUS_ICON_QUESTION`, `format_status_message`

## Why

The user's requirement: "ensure this coupling on api level, it should be impossible to set pause without explanation". Making `set_pause(true)` impossible at the type level enforces this invariant. The shared `format_status_message` function ensures both error and question paths format identically.

## Analog

`set_error` + `set_pause` → `pause_with_status` follows the same atomic modify_task pattern as `set_pause_with_signal` already in `zbobr-dispatcher/src/task.rs`.
