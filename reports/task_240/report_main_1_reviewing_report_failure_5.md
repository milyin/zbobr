## Overall assessment
The branch is narrowly scoped, the last routing fix for `task show --json` is present, and the helper-based analog from `zbobr-dispatcher/src/cli.rs` is still the right pattern. The new `TaskListEntry`, JSON paths, and backend-aware `task show` wiring are consistent with the surrounding code.

However, there is still one blocking correctness issue in the shared ready-task selection logic.

## Finding

### 1. Shared selector is still not a full shared lookup because tie-breaking depends on caller ordering
**Files:**
- `zbobr-dispatcher/src/cli.rs:289-323`
- `zbobr-dispatcher/src/cli.rs:1146-1156`
- `zbobr/src/commands.rs:324-329`

`select_runnable_task()` centralizes the readiness predicate and primary priority key, but it ends with:

```rust
.max_by_key(|t| task_priority(t))
```

So when multiple runnable tasks have the same `stage_count`, the selected task depends on iterator order.

That order is **not the same** between the two call sites:

1. `task list --select` sorts tasks by ID before calling the selector:
   ```rust
   tasks.sort_by_key(|t| t.id);
   match select_runnable_task(zbobr.workflow(), &tasks) { ... }
   ```
2. The loop sorts `all_tasks` only by descending priority before building `runstage_candidates`:
   ```rust
   all_tasks.sort_by(|a, b| task_priority(b).cmp(&task_priority(a)));
   ```
   and then passes `runstage_candidates` into the same selector.

Because `max_by_key` resolves equal keys by iteration order, `task list --select` and `run_manager_loop()` can still disagree whenever two runnable tasks have equal `stage_count`. That undermines the requirement to use a common lookup in both places so they select the same highest-priority ready task.

### Why this matters
The task explicitly asked for a common function for this lookup and to use it both in `--select` and in the loop. Right now only the readiness predicate and primary key are shared; the final choice is still influenced by different pre-sorting in each caller.

This is a real behavioral bug, not just style:
- scripts using `zbobr task list --select` can choose a different task than the loop would run next,
- and the disagreement appears exactly in the case where centralized selection was supposed to remove drift.

## Suggested fix
Make the shared selector own the **entire ordering**, including a deterministic tie-breaker, instead of relying on caller order. For example:
- define a total ordering inside `select_runnable_task` (or a shared comparator / full priority key), such as `(task_priority(task), reverse_id)` or whatever tie-breaker the project wants,
- and use that same ordering in both the loop and CLI.

A good follow-up cleanup would be to stop pre-sorting the CLI list before `--select` matters, or at least ensure both callers use the same total ordering semantics.

## Analog consistency
The analog choice remains appropriate, and the implementation mostly follows the existing helper-based style. The remaining issue is specifically that the shared selector does not yet fully encapsulate scheduling choice.

## Extraneous changes
I did not find unrelated changes in this branch.

## Checklist
All checklist items in the provided context are already checked, so I did not mark any additional items during this review.