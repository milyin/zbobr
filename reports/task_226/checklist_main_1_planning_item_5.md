
## What to change

In `zbobr-dispatcher/src/cli.rs`, update all callers that previously used `set_pause(true)` or the `set_task_error_with_log` + `set_pause_with_signal` pattern:

### Error cases (execution failure, stash/push failure, merge conflict recursion, workspace preparation)
Currently these call `set_task_error_with_log(...)` followed separately by `set_pause_with_signal(signal)`. Merge these into a single `pause_with_signal_and_status(signal, STATUS_ICON_ERROR, message)` call. The `set_task_error_with_log` helper can be removed or repurposed.

### Max stage count limit (lines ~362 and ~624)
Replace `set_pause(true)` with `pause_with_status(STATUS_ICON_PAUSE, "stage count limit reached: {current}/{max}")`.

### PauseThenSignal case (line ~1579 — from `on_success = { pause = true }` in pipeline config)
The user says: "If pause is set by pipeline handler, place the last report (brief message and link) to status field."
Before calling the new pause method, read the current task's context to find the last report record (Success/Failure/Comment type). Format its brief + report URL as the status message. Use `STATUS_ICON_PAUSE` icon. Then call `pause_with_signal_and_status(signal, STATUS_ICON_PAUSE, last_report_status)`.

## Why

All `set_pause(true)` calls must now carry a status explanation. The `PauseThenSignal` case needs special handling to surface the last agent report as the pause reason.

## Analog

Follow same pattern as the existing `set_pause_with_signal` consolidation — atomically setting multiple fields in one modify_task call.
