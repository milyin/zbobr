# Fix: deterministic tie-breaker in select_runnable_task

## Problem
`select_runnable_task` ended with `.max_by_key(|t| task_priority(t))`, which resolves ties by iterator order. The `--select` CLI path pre-sorted tasks by ID ascending while the loop pre-sorted by priority descending, so equal-priority tasks could yield different selections.

## Fix
Changed `.max_by_key` to `.max_by` with a comparator that first orders by `task_priority` and then uses `b.id.cmp(&a.id)` as a deterministic tie-breaker (higher ID wins):

```rust
.max_by(|a, b| {
    task_priority(a)
        .cmp(&task_priority(b))
        .then_with(|| b.id.cmp(&a.id))
})
```

This makes both `task list --select` and `run_manager_loop` always pick the same task regardless of how the input slice was sorted before the call.

## File changed
- `zbobr-dispatcher/src/cli.rs` — `select_runnable_task` function (line ~322)

## Verification
`cargo build` passes with no errors.