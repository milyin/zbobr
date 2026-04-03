# Plan: Update task list, add JSON output

## Design decisions
- **TaskListEntry** as a lightweight projection of Task (id, stage_count, state, title) — keeps the compact list focused and avoids exposing internal fields like context/stack in the list view
- **Shared selection function** extracted from `run_manager_loop` — single source of truth for "highest priority ready task" logic, used by both `loop` and `--select`
- **Priority = highest stage_count** (descending) — matches existing loop behavior, tasks closest to completion get priority
- **Exit code 1 for --select with no ready task** — enables clean scripting (`if zbobr task list --select; then ...`)

## Key files
- `zbobr-api/src/task.rs` — Task struct (already has Serialize), State enum with helper methods
- `zbobr-dispatcher/src/cli.rs` — print_task function, run_manager_loop with inline selection logic
- `zbobr/src/commands.rs` — TaskSubcommand enum and match arms for List/Show

## Analog
The existing `print_task` utility function pattern in `zbobr-dispatcher/src/cli.rs` — standalone functions operating on Task references.