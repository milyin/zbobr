## Overall assessment
The branch is close, and the changes are tightly scoped to the task. The general analog choice was reasonable (`print_task`/standalone helper style in `zbobr-dispatcher`), but the implementation does **not** fully satisfy the plan or the task description yet.

## Findings

### 1. Shared ready-task selection was not actually wired into the loop, and the new helper does not enforce "ready"
- **Files:** `zbobr-dispatcher/src/cli.rs:293-297`, `zbobr-dispatcher/src/cli.rs:1095-1124`, `zbobr/src/commands.rs:87-89`, `zbobr/src/commands.rs:341-346`
- `select_ready_task()` currently filters only `!done && !pause && !state.is_pause()`, so it may return `Pending(...)` or `Running(...)` tasks even though the command/help text says `--select` prints the highest-priority **ready** task.
- Separately, `run_manager_loop()` still does its own inline sort/scan and never calls `select_ready_task()`, so the promised "common function" / single source of truth was not implemented.
- This is both a correctness issue (`task list --select` can emit a non-ready task ID) and a consistency issue (CLI selection and loop selection can drift).

**Suggested fix:** either make the helper represent the true shared "next processable task" concept and use it in both places, or make `--select` genuinely select `state.is_ready()` tasks and keep a different shared helper for the loop. Right now it is neither.

### 2. `task show --json` loses fields in the no-ID path
- **File:** `zbobr/src/commands.rs:370-379`
- For `task show --json` with no ID, the code serializes `Vec<TaskListEntry>` instead of full `Task` values.
- That changes `show` from a full-task view into a compact list view in JSON mode, dropping fields such as `description`, `context`, `signal`, `stack`, `status`, `pause`, `confirm`, etc.
- The task request explicitly asked to add JSON output for `task show` showing the task "with all fields". Even if the singular wording primarily targets `show <id>`, the no-ID path should not silently switch to the compact list schema.

**Suggested fix:** serialize `Vec<Task>` in the no-ID `show --json` path.

### 3. `TaskListEntry` and plain-text `task list` output use `title`, not the requested `description`
- **Files:** `zbobr-dispatcher/src/cli.rs:267-280`, `zbobr/src/commands.rs:355-356`
- The task description asked for one line per task with `id, stage count, state, description`, and for a list-entry type containing that subset.
- The implementation introduced `TaskListEntry { id, stage_count, state, title }` and prints `task.title` in the compact list output.
- `Task` already has distinct `title` and `description` fields, so this is not just naming shorthand; it changes which data is exposed.

**Suggested fix:** switch the compact list entry to `description` if that is the intended contract. If the product decision is actually to show `title` because it is shorter, then the task/CLI contract should be updated explicitly rather than silently substituting a different field.

## Analog consistency
The helper-based extraction pattern matches the existing CLI organization, but the implementation does not consistently follow through on the analog chosen in the plan:
- the extracted selector was not reused by the loop,
- `show --json` mixes "show" and "list" semantics,
- and the new list projection diverges from the requested field set.

## Extraneous changes
I did not see unrelated code churn; the changed files are all directly tied to this task.

## Checklist status
I did **not** mark any additional checklist items complete, because the first item (shared selection logic) is not fully implemented as planned, and the JSON/list output behavior still has requirement mismatches.
