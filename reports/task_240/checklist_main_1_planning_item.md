## What

Create two new items in `zbobr-dispatcher/src/cli.rs` (or a nearby module if better organized):

### TaskListEntry struct
- A lightweight struct with fields: `id: u64`, `stage_count: u64`, `state: State`, `title: String`
- Derive `Serialize` for JSON output, plus `Debug`
- Implement `From<&Task>` to convert a full Task into a TaskListEntry

### Ready-task selection function
- Create a function like `select_ready_task(tasks: &[Task]) -> Option<&Task>` that encapsulates the priority selection logic currently inline in `run_manager_loop` (around line 1074-1105 of `zbobr-dispatcher/src/cli.rs`):
  - Filter out tasks where `state.is_done()` is true
  - Filter out tasks where `pause` is true or `state.is_pause()` is true
  - Sort remaining by `stage_count` descending (highest first = closest to completion)
  - Return the first match (highest priority ready task), or None
- Refactor `run_manager_loop` to use this function for its initial task selection, replacing the inline sort + skip logic
- Export both `TaskListEntry` and `select_ready_task` from `zbobr-dispatcher/src/lib.rs`

## Why
- The selection function is needed by both `loop` and the new `--select` flag on `task list`. Extracting it avoids duplication and ensures consistent priority logic.
- `TaskListEntry` provides the compact subset of fields needed for the one-line list display and its JSON serialization.

## Analog
Follow the pattern of the existing `print_task` function in the same file — a standalone utility operating on `&Task` / `&[Task]`.