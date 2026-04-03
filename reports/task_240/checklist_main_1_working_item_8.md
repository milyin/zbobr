The shared `select_runnable_task` currently ends with `.max_by_key(|t| task_priority(t))`. When stage_count ties exist, the result depends on iterator order, which differs between the `--select` CLI path (sorted by ID) and the loop path (sorted descending by priority). 

Fix: change the selector to use a total ordering that includes a deterministic tie-breaker (e.g., reverse of task ID or task ID). Both call sites should consistently yield the same highest-priority runnable task.

Files to change:
- `zbobr-dispatcher/src/cli.rs` (select_runnable_task, and possibly remove pre-sorting in list --select path)
- `zbobr/src/commands.rs` (if it has a copy or reference to the selector)