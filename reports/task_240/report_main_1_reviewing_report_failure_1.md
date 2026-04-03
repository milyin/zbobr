## Overall assessment
The branch is close and the changes are well scoped, but the current implementation still does **not** fully satisfy the task requirements or the follow-up instruction in context. The analog choice was reasonable (standalone helper/projection functions in `zbobr-dispatcher`), but the final code is not fully consistent with that plan.

## Findings

### 1. The compact task-list projection/output still uses `description` instead of `title`
- **Files:** `zbobr-dispatcher/src/cli.rs:267-281`, `zbobr/src/commands.rs:349-357`
- `TaskListEntry` currently exposes `description: String` and is populated from `task.description`.
- The plain-text `task list` path also prints `task.description`.
- This directly conflicts with the latest explicit instruction in the task context: **"restore the `title`! The `description` is a misprint"**.
- Because the branch is clean and these lines are still present in `HEAD`, that follow-up was not implemented.

**Suggested fix:** change `TaskListEntry` back to `title`, populate it from `task.title`, and print `task.title` in the compact non-JSON list output.

### 2. The loop still does not use the shared ready-task lookup function requested by the task
- **Files:** `zbobr-dispatcher/src/cli.rs:297-309`, `zbobr-dispatcher/src/cli.rs:1125-1236`
- The task asked to make a **common function for the lookup** and use it both in `task list --select` and in the loop.
- The branch extracted a shared **priority key** function (`task_priority`), but `run_manager_loop()` still performs its own sort-and-scan over `all_tasks` instead of calling `select_ready_task()` or another shared lookup helper.
- That means the actual task-selection logic is still duplicated between the CLI selector and the manager loop, which is precisely the drift the task asked to avoid.

**Suggested fix:** extract a shared helper that represents the actual "next task to process" lookup and use that helper in both places, rather than sharing only the priority scalar.

## Analog consistency
The implementation generally follows the existing helper/projection style in `zbobr-dispatcher`, but it stops short of full consistency with the planned analog:
- the list projection now diverges from the intended field contract after the follow-up correction,
- and the loop/CLI still do not share the same lookup function end-to-end.

## Extraneous changes
I did not find unrelated churn; the modified files are all directly tied to this task.

## Checklist
I did not mark any additional checklist items complete during this review.