In `zbobr/src/commands.rs`, locate the handler for the `Process` variant (around line ~430). Update it as follows:

1. If both `task` (Some value) and `select: true` are provided, return an error (conflicting arguments).
2. If `select: true` and `task` is None: call the existing `select_runnable_task` utility (already used by the `List` handler). If it returns `None` (no runnable task), exit with code 1 — matching the behavior of `list --select` when no tasks are available. If it returns a task ID, proceed with processing that task.
3. If `select: false`, fall through to the existing `require_task_id` path (no behavior change).

Analog: Follow the same pattern used in the `List` variant handler (lines ~309–336) for fetching all tasks and calling `select_runnable_task`.

After making changes, run `cargo build` and `cargo test` to verify compilation and that existing tests still pass.