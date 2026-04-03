## Overall assessment
The branch is close and the changes are well scoped. `TaskListEntry.title` is restored correctly, `task show --json` now returns full `Task` data, and I did not find unrelated file churn. The analog choice (standalone helper/projection functions in `zbobr-dispatcher`) is appropriate.

However, there is still one correctness issue in the shared task-selection behavior.

## Finding

### 1. `task list --select` is still not behaviorally equivalent to the loop for resumed tasks
**Files:**
- `zbobr-dispatcher/src/cli.rs:302-313`
- `zbobr-dispatcher/src/cli.rs:889-915`
- `zbobr-dispatcher/src/cli.rs:1168-1170`
- `zbobr-dispatcher/src/cli.rs:1181-1219`
- `zbobr-dispatcher/src/workflow.rs:252-320`

`select_runnable_task()` uses `workflow.resolve_next_action(task)` directly on the raw task snapshot. That is **not** the same path the manager loop uses before selecting a runnable task:

1. In Phase 1, the loop first runs `apply_ready_from_state()` for `READY` tasks with a non-empty stack.
2. That helper mutates the task to `Pending(saved_pipeline)` and restores the saved signal from the stack.
3. Only after that normalization does the loop consider the task for scheduling.

By contrast, `task list --select` calls `select_runnable_task(zbobr.workflow(), &tasks)` on the raw snapshots from `list_tasks()`, so a task in `READY` with a non-empty stack is resolved without the loop’s normalization step.

That can produce a wrong answer because `Workflow::resolve_next_action()` handles `State::Ready` by calling `resolve_signal()`, and `resolve_signal()` derives the pipeline from the current state via `pipeline_from_state(&task.state)`. For `State::Ready`, that yields `None`, so it falls back to the **default pipeline** instead of the saved pipeline that `apply_ready_from_state()` would restore first.

So `task list --select` can still print a task ID that the loop would not actually run in Phase 2 of the current cycle, and it may resolve that task against the wrong pipeline entirely.

### Why this matters
The task explicitly required a common lookup function to be used by both `--select` and the loop. The current code shares a helper name, but not the full semantics, because the loop still depends on pre-selection state normalization that the CLI path does not perform.

### Suggested fix
Refactor the shared selection logic to operate on the same normalized scheduling view in both places. For example:
- extract a shared helper that computes whether a task is runnable **after** applying the same pause/ready normalization rules the loop uses, or
- reuse a shared "prepare task for scheduling" step before `select_runnable_task()` is called from both the loop and `task list --select`.

The important part is that `--select` must not call `resolve_next_action()` on raw `READY`-with-stack tasks while the loop uses a normalized `Pending(...)` form.

## Analog consistency
The implementation generally follows the existing helper-based style, but this issue means the helper is not yet a true single source of truth for runnable-task lookup. The structure is good; the remaining problem is semantic drift between the helper and the loop.

## Extraneous changes
I did not find unrelated modifications in this branch.

## Checklist
No additional checklist items were checked during this review.