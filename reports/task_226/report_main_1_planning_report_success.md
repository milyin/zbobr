
# Plan: replace ERROR section with STATUS + unified pause mechanism

## Approach chosen

**Key design decisions:**

1. **Rename `Task.error` → `Task.status`** and `---ERROR---` → `---STATUS---` throughout, no backward compat.

2. **Shared format_status_message helper**: a single `format_status_message(icon, message)` function (in `zbobr-api`) produces `"❌ 2026-03-28 … message"` or `"❓ 2026-03-28 … message"`. Both error and question paths use this.

3. **Enforce pause-with-status at the API level**: Remove `set_pause(true)` from `TaskMut` and `RoleSession`. Replace with:
   - `pause_with_status(icon, message)` — sets pause=true + status atomically
   - `pause_with_signal_and_status(signal, icon, message)` — same + signal
   - `clear_pause()` — only way to set pause=false

4. **stop_with_error**: calls `pause_with_status(ERROR_ICON, message)`. No context record.

5. **stop_with_question**: calls `pause_with_status(QUESTION_ICON, message)` AND adds a context record (like `report_*`). No comment posted.

6. **Pipeline-handler pauses** (PauseThenSignal): reads last report record from task context, formats it as status, calls `pause_with_signal_and_status`.

7. **Error pauses in cli.rs**: merge `set_task_error_with_log` + `set_pause_with_signal` into single `pause_with_signal_and_status` call.

## Analog used

- `set_pause_with_signal` (existing) as pattern for atomic multi-field modify_task
- `report_impl` as pattern for context record + report file storage in the question path

## Checklist items created

1. Rename `error` → `status` in Task data model
2. Rename `---ERROR---` separator → `---STATUS---` in GitHub/FS backends
3. Introduce shared status-formatting + enforce pause-with-status API
4. Update `RoleSession` in dispatcher to use new pause-with-status API
5. Refactor `stop_with_error_impl` and `stop_with_question_impl` to use shared mechanism
6. Update `cli.rs` pause callers to use new pause-with-status API
