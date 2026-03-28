# Task 226: Rename ERROR to STATUS - Implementation Complete

## Summary

All planned changes implemented and tested:

### Changes Made

1. **Task data model** (`zbobr-api/src/task.rs`): Renamed `pub error: Option<String>` → `pub status: Option<String>` (no backward compat)

2. **Separator** (`zbobr-task-backend-github/src/separator.rs`): Renamed `ERROR_SEPARATOR` → `STATUS_SEPARATOR`, changed `---ERROR---` → `---STATUS---`. Updated all variable names in parse/serialize/merge functions.

3. **Shared status formatting** (`zbobr-api/src/backend.rs`):
   - Added `QUESTION_PREFIX: char = '❓'` (U+2753) alongside existing `ERROR_PREFIX: char = '❌'`
   - Added `pub fn format_status(icon, ts, message) -> String` helper
   - Renamed `set_error` → `set_status(status: Option<String>)` (takes pre-formatted string)
   - Replaced `set_pause(bool)` and `set_pause_with_signal(signal)` with:
     - `set_pause_with_status(status: String)` — atomic pause + status
     - `set_pause_with_status_and_signal(status, signal)` — atomic pause + status + signal
   - Enforcement: impossible to set pause without an explanation message

4. **RoleSession/TaskSession** (`zbobr-dispatcher/src/task.rs`): Same API changes as backend trait + added `config()` method on RoleSession for timezone access

5. **stop_with_error/question** (`zbobr-dispatcher/src/mcp/traits.rs`):
   - Shared `pause_with_status_impl(tool, icon, message, add_context_record)` 
   - `stop_with_error_impl`: uses ❌ icon, sets STATUS only (no context record)
   - `stop_with_question_impl`: uses ❓ icon, sets STATUS + adds Question context record (so question appears in agent report as well)
   - Question no longer posts a GitHub comment — uses context records instead

6. **cli.rs** (`zbobr-dispatcher/src/cli.rs`):
   - Added `format_error_status(zbobr, message)` helper
   - Added `set_task_status_with_log(...)` for non-pause status updates
   - All `set_pause(true)` → `set_pause_with_status(status)`
   - All `set_pause_with_signal(signal)` → `set_pause_with_status_and_signal(status, signal)`
   - Merged paired `set_task_error_with_log` + `set_pause_with_signal` calls into single atomic calls

7. **Tests and other files**: Updated all field references from `error` to `status`

### Test Results
All workspace tests pass: `cargo test --workspace` — no failures, no warnings.
