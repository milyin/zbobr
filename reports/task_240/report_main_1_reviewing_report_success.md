## Overall assessment
The implementation now matches the task requirements and is narrowly scoped to the requested CLI/output changes. I did not find blocking correctness issues in the current branch diff against `origin/main`.

## Requirement coverage
- `task list` now prints one compact line per task in text mode: `id`, `stage_count`, `state`, `title`.
- `task list --json` returns a JSON array of compact `TaskListEntry` objects.
- `task list --select` prints only the selected task ID, exits with code 1 when no runnable task exists, and uses the shared dispatcher selector.
- `task show --json` returns full `Task` JSON both with and without an explicit task ID.
- `task show` without `--json` still preserves the existing human-readable output path.

## Analog and pattern consistency
The planner’s analog choice was appropriate: helper-style utilities in `zbobr-dispatcher/src/cli.rs` are the right place for both the compact `TaskListEntry` projection and the shared task-selection helper. The new code follows the surrounding style and architectural split cleanly:
- shared dispatcher logic stays in `zbobr-dispatcher`
- CLI surface wiring stays in `zbobr/src/commands.rs`
- public access is exposed via `zbobr-dispatcher/src/lib.rs`

## Selector consistency
The latest selector change resolves the earlier drift between `task list --select` and the loop:
- both paths now call `select_runnable_task`
- the selector uses workflow resolution rather than a weaker hand-rolled readiness check
- READY-with-stack tasks are excluded to match the loop’s Phase 1 normalization semantics
- tie handling is deterministic, so caller input order no longer affects which runnable task is chosen

## Code quality notes
- `TaskListEntry` uses the requested lightweight projection instead of overloading full-task output.
- `task show --json` now serializes full `Task` values rather than list entries.
- The no-backend fast path no longer intercepts `task show` without an ID, so backend-backed task retrieval is reachable as intended.
- I did not find unrelated or extraneous changes in the branch.

## Checklist review
All checklist items provided in the task context were already completed, and the current code matches those completed items. There were no remaining unchecked relevant checklist items to mark during this review.

## Result
Review passed.