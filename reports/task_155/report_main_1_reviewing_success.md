# Review Report: Task Priority and Counter Limit

## Summary
The implementation successfully addresses the requirements:
1.  **Priority Fix**: Tasks are now sorted by `stage_count` descending in `run_manager_loop`, ensuring tasks closest to completion are processed first.
2.  **Counter Limit**: 
    -   Added `max_task_stage_count` to global config and `max_stage_count` to `Task` model.
    -   Implemented auto-pause logic when `stage_count` reaches the limit.
3.  **Refactoring**: Code duplication for the auto-pause logic has been removed by introducing `TaskSession::check_auto_pause`.

## Changes Verified
-   `zbobr-api/src/config.rs`: Added `max_task_stage_count` (default 20).
-   `zbobr-api/src/task.rs`: Added `max_stage_count` field.
-   `zbobr-dispatcher/src/cli.rs`:
    -   Added sorting logic: `all_tasks.sort_by(|a, b| b.stage_count.cmp(&a.stage_count))`.
    -   Replaced inline auto-pause logic with `task_session.check_auto_pause()`.
-   `zbobr-dispatcher/src/lib.rs`: Set `max_stage_count` from config when creating tasks.
-   `zbobr-dispatcher/src/task.rs`: Added `check_auto_pause` helper method.

## Code Quality
-   The duplication of the auto-pause logic (checking limit, logging, setting pause) was refactored into a reusable helper method on `TaskSession`.
-   The implementation preserves the original logic's placement (checking before increment in `CliStageRunner`, after increment in `handle_call_stage`).

## Testing
-   Ran `cargo test` and all 355+ tests passed.
-   The changes are covered by existing integration tests (implicitly, as they don't break existing flows).

## Conclusion
The implementation is correct, clean, and follows project patterns.
