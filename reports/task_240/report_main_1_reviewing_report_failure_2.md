## Overall assessment
The branch is close and the changes are well scoped, but there is still one material correctness issue in the selector logic. The chosen analog was reasonable (`zbobr-dispatcher` helper functions such as `print_task`), and the code generally follows that style, but the final implementation does **not** fully achieve the planned/shared selection behavior.

## Findings

### 1. `task list --select` can return tasks that the loop would not run
- **Files:** `zbobr-dispatcher/src/cli.rs:297-309`, `zbobr/src/commands.rs:341-345`, `zbobr-dispatcher/src/workflow.rs:252-315`, `zbobr-dispatcher/src/cli.rs:1324-1327`
- `select_ready_task()` currently defines “ready” as only:
  - not done
  - not paused
  - not in pause state
  - not running
- That predicate is **weaker** than the loop’s actual notion of a runnable task. In the loop, Phase 1 first calls `workflow.resolve_next_action(task)` and only puts tasks with `StateAction::RunStage(...)` into `runstage_candidates`; only then does it call `select_ready_task(&runstage_candidates)`.
- By contrast, `task list --select` calls `select_ready_task(&tasks)` on the full task list. That means it can select tasks that are *not* runnable according to the workflow, for example:
  - `State::Pending(...)` with no signal → `StateAction::Idle` (`workflow.rs:296-312`)
  - `State::Ready` with a non-empty stack but no signal → `StateAction::Idle` (`workflow.rs:281-291`)
  - `State::Empty` / `State::Unknown(_)` are also idle states in the workflow (`workflow.rs:252-253`, `314`)
- So the CLI selector and the manager loop still do not share the same effective “next runnable task” lookup. The loop now reuses the helper for priority ordering among already-runnable candidates, but `--select` still uses a broader predicate and can return a task that the loop would skip.

**Why this matters:** the task explicitly asked for a common lookup used by both `task list --select` and `loop`. As implemented, users can get an ID from `--select` that is not actually the next runnable task the loop would process.

**Suggested fix:** extract a truly shared selector that operates on workflow-resolved runnable tasks, not just state flags. For example, share a helper that filters the input to tasks whose `resolve_next_action()` is `RunStage(...)` (or otherwise expose a `is_runnable_task(...)` predicate based on workflow resolution), then apply the common priority function on that filtered set in both `task list --select` and `run_manager_loop`.

## Analog consistency
The analog choice was appropriate, and the new `TaskListEntry` / helper placement fits the surrounding code. The inconsistency is behavioral: the code now has a shared priority helper and a partially shared selector, but not yet a single shared definition of “the next runnable task,” which was the core requirement.

## Extraneous changes
I did not find unrelated changes; the modified files are all directly tied to this task.

## Checklist
All checklist items in the task context are already marked complete, so there were no unchecked items to update during this review. However, the implementation still needs the fix above before this task can be accepted.