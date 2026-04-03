# Review Fix Summary

All three issues from the review (ctx_rec_7) have been addressed:

## 1. `TaskListEntry.title` → `TaskListEntry.description`
- `zbobr-dispatcher/src/cli.rs`: renamed field `title` to `description`, updated `From<&Task>` to use `task.description`
- `zbobr/src/commands.rs`: updated compact list display to print `task.description`

## 2. `select_ready_task` correctness + shared priority function
- Extracted `fn task_priority(task: &Task) -> u64` as the single source of truth for task scheduling priority (currently `task.stage_count`)
- Added `!t.state.is_running()` filter to `select_ready_task` so it only returns tasks not yet being processed
- Updated `run_manager_loop`'s sort to use `task_priority()` — both the CLI selector and the loop now share the same priority key

## 3. `task show --json` (no ID) uses full `Vec<Task>`
- `zbobr/src/commands.rs`: replaced `Vec<TaskListEntry>` serialization with `Vec<Task>` in the no-ID `show --json` path, exposing all task fields

## Verification
- `cargo build -p zbobr-dispatcher -p zbobr` — clean
- `cargo test -p zbobr-dispatcher -p zbobr` — 80 + 14 + 3 tests all pass